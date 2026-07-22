use std::collections::{BTreeMap, BTreeSet};

use nando_core::wave::{PhaseCenterCell, phase_margin_to_micro, phase_vector_from_atom_ids};
use nando_operator_kernel::{
    MAX_ADAPTER_WAVE_ANCHOR_ATOMS, MAX_ADAPTER_WAVE_FINGERPRINTS, MAX_ADAPTER_WAVE_SUBCENTERS,
    ResponseAdapterWaveRoute, ResponseAdapterWaveSubcenter,
};

use crate::{
    LearnedWaveRoute, LearnedWaveSubcenter, RelationFrame, relation_frame_online_routing_atom_ids,
};

#[doc(hidden)]
#[must_use]
pub fn learned_wave_route_from_support_medoid(
    support: &[RelationFrame],
    negatives: &[RelationFrame],
    cells: usize,
) -> Option<LearnedWaveRoute> {
    if support.len() < 32 || negatives.is_empty() || cells == 0 || cells > usize::from(u16::MAX) {
        return None;
    }
    let query_atom_ids = learned_wave_feature_vocabulary(support, negatives, 256);
    if query_atom_ids.is_empty() {
        return None;
    }
    let filtered_atoms = |frame: &RelationFrame| {
        let mut atoms = relation_frame_online_routing_atom_ids(frame);
        atoms.retain(|atom| query_atom_ids.binary_search(atom).is_ok());
        atoms
    };
    let support_vectors = support
        .iter()
        .map(|frame| phase_vector_from_atom_ids(filtered_atoms(frame), cells))
        .collect::<Vec<_>>();
    let negative_vectors = negatives
        .iter()
        .map(|frame| phase_vector_from_atom_ids(filtered_atoms(frame), cells))
        .collect::<Vec<_>>();
    let mut negative_center = vec![PhaseCenterCell::default(); cells];
    for vector in &negative_vectors {
        for (center, cell) in negative_center.iter_mut().zip(vector) {
            center.re += cell.re / negative_vectors.len() as f64;
            center.im += cell.im / negative_vectors.len() as f64;
        }
    }
    let score = |vector: &[PhaseCenterCell], center_delta_micro: &[i32]| {
        phase_margin_to_micro(
            vector
                .iter()
                .zip(center_delta_micro.chunks_exact(2))
                .map(|(query, center)| {
                    query.re * f64::from(center[0]) / 1_000_000.0
                        + query.im * f64::from(center[1]) / 1_000_000.0
                })
                .sum::<f64>()
                / cells as f64,
        )
        .unwrap_or(i64::MIN)
    };

    #[derive(Clone)]
    struct MedoidCandidate {
        coverage: BTreeSet<usize>,
        gap_micro: i64,
        threshold_micro: i64,
        frame_id: String,
        center_delta_micro: Vec<i32>,
    }

    let mut candidates = Vec::<MedoidCandidate>::new();
    let mut seen_routes = BTreeSet::<(Vec<i32>, i64)>::new();
    for (index, representative) in support_vectors.iter().enumerate() {
        let delta = representative
            .iter()
            .zip(&negative_center)
            .map(|(positive, negative)| PhaseCenterCell {
                re: positive.re - negative.re,
                im: positive.im - negative.im,
            })
            .collect::<Vec<_>>();
        let center_delta_micro = delta
            .into_iter()
            .flat_map(|cell| {
                [
                    (cell.re * 1_000_000.0).round() as i32,
                    (cell.im * 1_000_000.0).round() as i32,
                ]
            })
            .collect::<Vec<_>>();
        let maximum_negative = negative_vectors
            .iter()
            .map(|vector| score(vector, &center_delta_micro))
            .max()
            .unwrap_or(i64::MIN);
        let Some(threshold_micro) = maximum_negative.checked_add(1).map(|value| value.max(1))
        else {
            continue;
        };
        if negative_vectors
            .iter()
            .any(|vector| score(vector, &center_delta_micro) >= threshold_micro)
        {
            continue;
        }
        let support_margins = support_vectors
            .iter()
            .map(|vector| score(vector, &center_delta_micro))
            .collect::<Vec<_>>();
        let coverage = support_margins
            .iter()
            .enumerate()
            .filter_map(|(support_index, margin)| {
                (*margin >= threshold_micro).then_some(support_index)
            })
            .collect::<BTreeSet<_>>();
        if coverage.is_empty() || !seen_routes.insert((center_delta_micro.clone(), threshold_micro))
        {
            continue;
        }
        let maximum_positive = coverage
            .iter()
            .map(|support_index| support_margins[*support_index])
            .max()
            .unwrap_or(i64::MIN);
        candidates.push(MedoidCandidate {
            coverage,
            gap_micro: maximum_positive.saturating_sub(maximum_negative),
            threshold_micro,
            frame_id: support[index].frame_id_sha256.clone(),
            center_delta_micro,
        });
    }

    let mut uncovered = (0..support.len()).collect::<BTreeSet<_>>();
    let mut selected = Vec::<MedoidCandidate>::new();
    let mut selected_indices = BTreeSet::<usize>::new();
    while !uncovered.is_empty() && selected.len() < 8 {
        let mut best = None::<(usize, usize)>;
        for (candidate_index, candidate) in candidates.iter().enumerate() {
            if selected_indices.contains(&candidate_index) {
                continue;
            }
            let newly_covered = candidate.coverage.intersection(&uncovered).count();
            if newly_covered == 0 {
                continue;
            }
            let replace = best.is_none_or(|(best_index, best_newly_covered)| {
                let current = &candidates[best_index];
                newly_covered > best_newly_covered
                    || (newly_covered == best_newly_covered
                        && (candidate.coverage.len() > current.coverage.len()
                            || (candidate.coverage.len() == current.coverage.len()
                                && (candidate.gap_micro > current.gap_micro
                                    || (candidate.gap_micro == current.gap_micro
                                        && (candidate.threshold_micro
                                            < current.threshold_micro
                                            || (candidate.threshold_micro
                                                == current.threshold_micro
                                                && candidate.frame_id < current.frame_id)))))))
            });
            if replace {
                best = Some((candidate_index, newly_covered));
            }
        }
        let Some((best_index, _)) = best else {
            break;
        };
        let candidate = candidates[best_index].clone();
        for covered in &candidate.coverage {
            uncovered.remove(covered);
        }
        selected_indices.insert(best_index);
        selected.push(candidate);
    }
    if support.len().saturating_sub(uncovered.len()) < 32 {
        return None;
    }

    while selected.len() < 8 && selected_indices.len() < candidates.len() {
        let mut best = None::<(usize, u64)>;
        for (candidate_index, candidate) in candidates.iter().enumerate() {
            if selected_indices.contains(&candidate_index) {
                continue;
            }
            let minimum_center_distance = selected
                .iter()
                .map(|current| {
                    candidate
                        .center_delta_micro
                        .iter()
                        .zip(&current.center_delta_micro)
                        .fold(0_u64, |distance, (left, right)| {
                            distance.saturating_add(u64::from(left.abs_diff(*right)))
                        })
                })
                .min()
                .unwrap_or(u64::MAX);
            let replace = best.is_none_or(|(best_index, best_distance)| {
                let current = &candidates[best_index];
                minimum_center_distance > best_distance
                    || (minimum_center_distance == best_distance
                        && (candidate.coverage.len() > current.coverage.len()
                            || (candidate.coverage.len() == current.coverage.len()
                                && (candidate.gap_micro > current.gap_micro
                                    || (candidate.gap_micro == current.gap_micro
                                        && (candidate.threshold_micro
                                            < current.threshold_micro
                                            || (candidate.threshold_micro
                                                == current.threshold_micro
                                                && candidate.frame_id < current.frame_id)))))))
            });
            if replace {
                best = Some((candidate_index, minimum_center_distance));
            }
        }
        let Some((best_index, _)) = best else {
            break;
        };
        selected_indices.insert(best_index);
        selected.push(candidates[best_index].clone());
    }

    let primary = selected.first()?.clone();
    let subcenters = selected
        .into_iter()
        .skip(1)
        .map(|candidate| LearnedWaveSubcenter {
            center_delta_micro: candidate.center_delta_micro,
            threshold_micro: candidate.threshold_micro,
        })
        .collect();
    Some(LearnedWaveRoute {
        cells: cells as u16,
        center_delta_micro: primary.center_delta_micro,
        threshold_micro: primary.threshold_micro,
        query_atom_ids,
        subcenters,
    })
}

#[doc(hidden)]
#[must_use]
pub fn learned_wave_route_accepts_frame(route: &LearnedWaveRoute, frame: &RelationFrame) -> bool {
    let cells = usize::from(route.cells);
    if cells == 0 || route.center_delta_micro.len() != cells.saturating_mul(2) {
        return false;
    }
    let mut atoms = relation_frame_online_routing_atom_ids(frame);
    if !route.query_atom_ids.is_empty() {
        atoms.retain(|atom| route.query_atom_ids.binary_search(atom).is_ok());
    }
    if atoms.is_empty() {
        return false;
    }
    let query = phase_vector_from_atom_ids(atoms, cells);
    let score = |center_delta_micro: &[i32]| {
        if center_delta_micro.len() != cells.saturating_mul(2) {
            return i64::MIN;
        }
        phase_margin_to_micro(
            query
                .iter()
                .zip(center_delta_micro.chunks_exact(2))
                .map(|(cell, center)| {
                    cell.re * f64::from(center[0]) / 1_000_000.0
                        + cell.im * f64::from(center[1]) / 1_000_000.0
                })
                .sum::<f64>()
                / cells as f64,
        )
        .unwrap_or(i64::MIN)
    };
    score(&route.center_delta_micro) >= route.threshold_micro
        || route
            .subcenters
            .iter()
            .any(|subcenter| score(&subcenter.center_delta_micro) >= subcenter.threshold_micro)
}

fn learned_wave_feature_vocabulary(
    support: &[RelationFrame],
    negatives: &[RelationFrame],
    limit: usize,
) -> Vec<u64> {
    let mut counts = BTreeMap::<u64, (usize, usize)>::new();
    for frame in support {
        for atom in relation_frame_online_routing_atom_ids(frame) {
            counts.entry(atom).or_default().0 += 1;
        }
    }
    for frame in negatives {
        for atom in relation_frame_online_routing_atom_ids(frame) {
            counts.entry(atom).or_default().1 += 1;
        }
    }
    let support_rows = support.len().max(1);
    let negative_rows = negatives.len().max(1);
    let mut ranked = counts
        .into_iter()
        .filter(|(_, (positive, negative))| *positive >= 2 || *negative >= 2)
        .map(|(atom, (positive, negative))| {
            let separation = positive
                .saturating_mul(negative_rows)
                .abs_diff(negative.saturating_mul(support_rows));
            (atom, separation, positive.saturating_add(negative))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| left.0.cmp(&right.0))
    });
    ranked.truncate(limit);
    let mut atoms = ranked
        .into_iter()
        .map(|(atom, _, _)| atom)
        .collect::<Vec<_>>();
    atoms.sort_unstable();
    atoms
}

#[doc(hidden)]
#[must_use]
pub fn fit_adapter_wave_route(
    positive_atoms: &[Vec<u64>],
    negative_atoms: &[Vec<u64>],
    cells: usize,
) -> Option<ResponseAdapterWaveRoute> {
    if cells == 0 || positive_atoms.is_empty() || negative_atoms.is_empty() {
        return None;
    }
    let anchor_atom_ids = fit_adapter_wave_anchor_atoms(positive_atoms, negative_atoms);
    let positive_fingerprint_ids = positive_atoms
        .iter()
        .map(|atoms| adapter_wave_atom_fingerprint(atoms))
        .collect::<BTreeSet<_>>();
    let negative_fingerprint_ids = negative_atoms
        .iter()
        .map(|atoms| adapter_wave_atom_fingerprint(atoms))
        .collect::<BTreeSet<_>>();
    if positive_fingerprint_ids
        .iter()
        .any(|fingerprint| negative_fingerprint_ids.contains(fingerprint))
        || positive_fingerprint_ids.len() > MAX_ADAPTER_WAVE_FINGERPRINTS
    {
        return None;
    }
    let positives = positive_atoms
        .iter()
        .map(|atoms| phase_vector_from_atom_ids(atoms.iter().copied(), cells))
        .collect::<Vec<_>>();
    let negatives = negative_atoms
        .iter()
        .map(|atoms| phase_vector_from_atom_ids(atoms.iter().copied(), cells))
        .collect::<Vec<_>>();
    let mut negative_center = vec![PhaseCenterCell::default(); cells];
    for vector in &negatives {
        for (center, cell) in negative_center.iter_mut().zip(vector) {
            center.re += cell.re / negatives.len() as f64;
            center.im += cell.im / negatives.len() as f64;
        }
    }
    let score = |vector: &[PhaseCenterCell], center: &[i32]| {
        phase_margin_to_micro(
            vector
                .iter()
                .zip(center.chunks_exact(2))
                .map(|(query, center)| {
                    query.re * f64::from(center[0]) / 1_000_000.0
                        + query.im * f64::from(center[1]) / 1_000_000.0
                })
                .sum::<f64>()
                / cells as f64,
        )
        .unwrap_or(i64::MIN)
    };
    let mut candidates = Vec::<(BTreeSet<usize>, Vec<i32>, i64, i64)>::new();
    for representative in &positives {
        let center = representative
            .iter()
            .zip(&negative_center)
            .flat_map(|(positive, negative)| {
                [
                    ((positive.re - negative.re) * 1_000_000.0).round() as i32,
                    ((positive.im - negative.im) * 1_000_000.0).round() as i32,
                ]
            })
            .collect::<Vec<_>>();
        let maximum_negative = negatives
            .iter()
            .map(|vector| score(vector, &center))
            .max()?;
        let threshold = maximum_negative.checked_add(1)?.max(1);
        let coverage = positives
            .iter()
            .enumerate()
            .filter_map(|(index, vector)| (score(vector, &center) >= threshold).then_some(index))
            .collect::<BTreeSet<_>>();
        if coverage.is_empty() {
            continue;
        }
        let gap = coverage
            .iter()
            .map(|index| score(&positives[*index], &center))
            .min()
            .unwrap_or(i64::MIN)
            .saturating_sub(maximum_negative);
        candidates.push((coverage, center, threshold, gap));
    }
    let mut uncovered = (0..positives.len()).collect::<BTreeSet<_>>();
    let mut selected = Vec::<usize>::new();
    while !uncovered.is_empty() && selected.len() < MAX_ADAPTER_WAVE_SUBCENTERS {
        let next = candidates
            .iter()
            .enumerate()
            .filter(|(index, _)| !selected.contains(index))
            .map(|(index, candidate)| {
                (
                    candidate.0.intersection(&uncovered).count(),
                    candidate.3,
                    index,
                )
            })
            .filter(|(gain, _, _)| *gain > 0)
            .max_by(|left, right| left.cmp(right))?;
        selected.push(next.2);
        for covered in &candidates[next.2].0 {
            uncovered.remove(covered);
        }
    }
    if !uncovered.is_empty() {
        return None;
    }
    let primary = candidates.get(*selected.first()?)?;
    Some(ResponseAdapterWaveRoute {
        cells: u16::try_from(cells).ok()?,
        center_delta_micro: primary.1.clone(),
        threshold_micro: primary.2,
        anchor_atom_ids,
        positive_fingerprint_ids: positive_fingerprint_ids.into_iter().collect(),
        subcenters: selected
            .iter()
            .skip(1)
            .filter_map(|index| candidates.get(*index))
            .map(|candidate| ResponseAdapterWaveSubcenter {
                center_delta_micro: candidate.1.clone(),
                threshold_micro: candidate.2,
            })
            .collect(),
    })
}

fn fit_adapter_wave_anchor_atoms(positives: &[Vec<u64>], negatives: &[Vec<u64>]) -> Vec<u64> {
    let negative_atoms = negatives
        .iter()
        .flat_map(|atoms| atoms.iter().copied())
        .collect::<BTreeSet<_>>();
    let mut coverage = BTreeMap::<u64, BTreeSet<usize>>::new();
    for (index, atoms) in positives.iter().enumerate() {
        for atom in atoms {
            if !negative_atoms.contains(atom) {
                coverage.entry(*atom).or_default().insert(index);
            }
        }
    }
    let mut uncovered = (0..positives.len()).collect::<BTreeSet<_>>();
    let mut selected = Vec::<u64>::new();
    while !uncovered.is_empty() && selected.len() < MAX_ADAPTER_WAVE_ANCHOR_ATOMS {
        let Some((atom, covered)) = coverage
            .iter()
            .filter(|(atom, _)| !selected.contains(*atom))
            .map(|(atom, indices)| {
                (
                    *atom,
                    indices
                        .intersection(&uncovered)
                        .copied()
                        .collect::<Vec<_>>(),
                )
            })
            .filter(|(_, covered)| !covered.is_empty())
            .max_by(|left, right| {
                left.1
                    .len()
                    .cmp(&right.1.len())
                    .then_with(|| right.0.cmp(&left.0))
            })
        else {
            break;
        };
        selected.push(atom);
        for index in covered {
            uncovered.remove(&index);
        }
    }
    selected.sort_unstable();
    selected
}

#[doc(hidden)]
#[must_use]
pub fn adapter_wave_atom_fingerprint(atoms: &[u64]) -> u64 {
    let mut fingerprint = 0xcbf2_9ce4_8422_2325_u64;
    for byte in atoms.iter().flat_map(|atom| atom.to_le_bytes()) {
        fingerprint = (fingerprint ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
    }
    fingerprint
}
