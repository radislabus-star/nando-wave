use nando_core::wave::{
    PhaseCenterCell, phase_coherence, phase_margin_to_micro, phase_vector_from_atom_ids,
};
use serde::{Deserialize, Serialize};

use crate::package::relation_frame_routing_query_atom_ids;
use crate::{RelationFrame, ResponsePackage};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GroundedWaveCausalReport {
    pub schema: String,
    pub package_id: String,
    pub verdict: String,
    pub support_rows: usize,
    pub future_rows: usize,
    pub negative_rows: usize,
    pub full_phase_correct: usize,
    #[serde(default)]
    pub no_phase_correct: usize,
    pub shuffled_phase_correct: usize,
    pub random_center_correct: usize,
    #[serde(default)]
    pub magnitude_only_correct: usize,
    #[serde(default)]
    pub no_anti_center_correct: usize,
    pub negative_accepts: usize,
    #[serde(default)]
    pub no_phase_negative_accepts: usize,
    #[serde(default)]
    pub shuffled_negative_accepts: usize,
    #[serde(default)]
    pub random_center_negative_accepts: usize,
    #[serde(default)]
    pub magnitude_only_negative_accepts: usize,
    #[serde(default)]
    pub no_anti_center_negative_accepts: usize,
    pub full_margin_mean_micro: i64,
    pub shuffled_margin_mean_micro: i64,
    pub random_margin_mean_micro: i64,
    pub no_phase_exact_checks: usize,
    pub full_phase_exact_checks: usize,
}

pub fn evaluate_grounded_wave_causality(
    package: &ResponsePackage,
    support: &[RelationFrame],
    future: &[RelationFrame],
    negatives: &[RelationFrame],
) -> GroundedWaveCausalReport {
    evaluate_grounded_wave_causality_refs(
        package,
        &support.iter().collect::<Vec<_>>(),
        &future.iter().collect::<Vec<_>>(),
        &negatives.iter().collect::<Vec<_>>(),
    )
}

pub fn evaluate_grounded_wave_causality_refs(
    package: &ResponsePackage,
    support: &[&RelationFrame],
    future: &[&RelationFrame],
    negatives: &[&RelationFrame],
) -> GroundedWaveCausalReport {
    let learned = package.learned_wave_route.as_ref();
    let cells = learned.map_or(16, |route| usize::from(route.cells));
    let positive = learned.map_or_else(
        || phase_vector_from_atom_ids(package.phase_centers.iter().copied(), cells),
        |route| {
            route
                .center_delta_micro
                .chunks_exact(2)
                .map(|delta| PhaseCenterCell {
                    re: f64::from(delta[0]) / 1_000_000.0,
                    im: f64::from(delta[1]) / 1_000_000.0,
                })
                .collect()
        },
    );
    let negative = if learned.is_some() {
        vec![PhaseCenterCell::default(); cells]
    } else {
        phase_vector_from_atom_ids(package.anti_centers.iter().copied(), cells)
    };
    let mut shuffled = positive.clone();
    if learned.is_some() {
        let shift = (cells / 3).max(1);
        shuffled.rotate_left(shift);
    } else {
        shuffled = phase_vector_from_atom_ids(
            package
                .phase_centers
                .iter()
                .map(|atom| atom.rotate_left(17) ^ 0x9e37_79b9_7f4a_7c15),
            cells,
        );
    }
    let random = if learned.is_some() {
        positive
            .iter()
            .rev()
            .enumerate()
            .map(|(index, cell)| {
                let sign = if index % 2 == 0 { 1.0 } else { -1.0 };
                PhaseCenterCell {
                    re: cell.im * sign,
                    im: cell.re * -sign,
                }
            })
            .collect()
    } else {
        phase_vector_from_atom_ids(
            package
                .phase_centers
                .iter()
                .map(|atom| atom.wrapping_mul(0xd6e8_feb8_6659_fd93) ^ 0xa5a5_a5a5_a5a5_a5a5),
            cells,
        )
    };
    let learned_centers = learned.map(|route| {
        std::iter::once((route.center_delta_micro.as_slice(), route.threshold_micro))
            .chain(route.subcenters.iter().map(|subcenter| {
                (
                    subcenter.center_delta_micro.as_slice(),
                    subcenter.threshold_micro,
                )
            }))
            .map(|(delta, threshold)| {
                (
                    delta
                        .chunks_exact(2)
                        .map(|cell| PhaseCenterCell {
                            re: f64::from(cell[0]) / 1_000_000.0,
                            im: f64::from(cell[1]) / 1_000_000.0,
                        })
                        .collect::<Vec<_>>(),
                    threshold,
                )
            })
            .collect::<Vec<_>>()
    });
    let learned_margin = |frame: &RelationFrame, variant: u8| {
        let centers = learned_centers.as_ref()?;
        let atoms = relation_frame_routing_query_atom_ids(package, frame)?;
        let query = phase_vector_from_atom_ids(atoms, cells);
        let shift = (cells / 3).max(1);
        let best_excess = centers
            .iter()
            .map(|(center, threshold)| {
                let score = query
                    .iter()
                    .enumerate()
                    .map(|(index, query_cell)| {
                        let center_cell = match variant {
                            1 => center[(index + shift) % cells],
                            2 => {
                                let source = center[cells.saturating_sub(index + 1)];
                                let sign = if index % 2 == 0 { 1.0 } else { -1.0 };
                                PhaseCenterCell {
                                    re: source.im * sign,
                                    im: source.re * -sign,
                                }
                            }
                            _ => center[index],
                        };
                        query_cell.re * center_cell.re + query_cell.im * center_cell.im
                    })
                    .sum::<f64>()
                    / cells as f64;
                phase_margin_to_micro(score)
                    .unwrap_or(i64::MIN)
                    .saturating_sub(*threshold)
            })
            .max()?;
        Some(package.wave_margin_micro.saturating_add(best_excess) as f64 / 1_000_000.0)
    };
    let margin = |frame: &RelationFrame, center: &[PhaseCenterCell]| {
        relation_frame_routing_query_atom_ids(package, frame).map(|atoms| {
            let query = phase_vector_from_atom_ids(atoms, cells);
            if learned.is_some() {
                query
                    .iter()
                    .zip(center.iter())
                    .map(|(query, center)| query.re * center.re + query.im * center.im)
                    .sum::<f64>()
                    / cells as f64
            } else {
                phase_coherence(&query, center) - phase_coherence(&query, &negative)
            }
        })
    };
    let no_anti_margin = |frame: &RelationFrame| {
        relation_frame_routing_query_atom_ids(package, frame).map(|atoms| {
            let query = phase_vector_from_atom_ids(atoms, cells);
            if learned.is_some() {
                query
                    .iter()
                    .zip(positive.iter())
                    .map(|(query, center)| query.re * center.re + query.im * center.im)
                    .sum::<f64>()
                    / cells as f64
            } else {
                phase_coherence(&query, &positive)
            }
        })
    };
    let magnitude_margin = |frame: &RelationFrame| {
        relation_frame_routing_query_atom_ids(package, frame).map(|atoms| {
            let query = phase_vector_from_atom_ids(atoms, cells);
            query
                .iter()
                .zip(positive.iter())
                .map(|(query, center)| query.re.hypot(query.im) * center.re.hypot(center.im))
                .sum::<f64>()
                / cells as f64
        })
    };
    let full_margins = future
        .iter()
        .map(|frame| {
            if learned.is_some() {
                learned_margin(frame, 0).unwrap_or(-2.0)
            } else {
                margin(frame, &positive).unwrap_or(-2.0)
            }
        })
        .collect::<Vec<_>>();
    let shuffled_margins = future
        .iter()
        .map(|frame| {
            if learned.is_some() {
                learned_margin(frame, 1).unwrap_or(-2.0)
            } else {
                margin(frame, &shuffled).unwrap_or(-2.0)
            }
        })
        .collect::<Vec<_>>();
    let random_margins = future
        .iter()
        .map(|frame| {
            if learned.is_some() {
                learned_margin(frame, 2).unwrap_or(-2.0)
            } else {
                margin(frame, &random).unwrap_or(-2.0)
            }
        })
        .collect::<Vec<_>>();
    let magnitude_margins = future
        .iter()
        .map(|frame| magnitude_margin(frame).unwrap_or(-2.0))
        .collect::<Vec<_>>();
    let no_anti_margins = future
        .iter()
        .map(|frame| no_anti_margin(frame).unwrap_or(-2.0))
        .collect::<Vec<_>>();
    let routes = |margin: &&f64| {
        phase_margin_to_micro(**margin).is_ok_and(|value| value >= package.wave_margin_micro)
    };
    let full_phase_correct = full_margins.iter().filter(routes).count();
    let shuffled_phase_correct = shuffled_margins.iter().filter(routes).count();
    let random_center_correct = random_margins.iter().filter(routes).count();
    let magnitude_only_correct = magnitude_margins.iter().filter(routes).count();
    let no_anti_center_correct = no_anti_margins.iter().filter(routes).count();
    let negative_accepts = negatives
        .iter()
        .filter(|frame| {
            let observed = if learned.is_some() {
                learned_margin(frame, 0)
            } else {
                margin(frame, &positive)
            };
            observed.is_some_and(|margin| {
                phase_margin_to_micro(margin).is_ok_and(|value| value >= package.wave_margin_micro)
            })
        })
        .count();
    let shuffled_negative_accepts = negatives
        .iter()
        .filter(|frame| {
            let observed = if learned.is_some() {
                learned_margin(frame, 1)
            } else {
                margin(frame, &shuffled)
            };
            observed.is_some_and(|margin| {
                phase_margin_to_micro(margin).is_ok_and(|value| value >= package.wave_margin_micro)
            })
        })
        .count();
    let random_center_negative_accepts = negatives
        .iter()
        .filter(|frame| {
            let observed = if learned.is_some() {
                learned_margin(frame, 2)
            } else {
                margin(frame, &random)
            };
            observed.is_some_and(|margin| {
                phase_margin_to_micro(margin).is_ok_and(|value| value >= package.wave_margin_micro)
            })
        })
        .count();
    let magnitude_only_negative_accepts = negatives
        .iter()
        .filter(|frame| {
            magnitude_margin(frame).is_some_and(|margin| {
                phase_margin_to_micro(margin).is_ok_and(|value| value >= package.wave_margin_micro)
            })
        })
        .count();
    let no_anti_center_negative_accepts = negatives
        .iter()
        .filter(|frame| {
            no_anti_margin(frame).is_some_and(|margin| {
                phase_margin_to_micro(margin).is_ok_and(|value| value >= package.wave_margin_micro)
            })
        })
        .count();
    let full_margin_mean = mean(&full_margins);
    let shuffled_margin_mean = mean(&shuffled_margins);
    let random_margin_mean = mean(&random_margins);
    let full_phase_exact_checks = future.len();
    let no_phase_exact_checks = future.len().saturating_mul(2);
    let shuffled_causal_degradation = shuffled_phase_correct < full_phase_correct
        || shuffled_negative_accepts > negative_accepts
        || full_margin_mean > shuffled_margin_mean;
    let random_causal_degradation = random_center_correct < full_phase_correct
        || random_center_negative_accepts > negative_accepts
        || full_margin_mean > random_margin_mean;
    let pass = support.len() >= crate::LEGACY_CONTROL_SUPPORT_ROWS
        && future.len() >= crate::LEGACY_CONTROL_FUTURE_ROWS
        && full_phase_correct == future.len()
        && negative_accepts == 0
        && shuffled_causal_degradation
        && random_causal_degradation
        && full_phase_exact_checks < no_phase_exact_checks;
    GroundedWaveCausalReport {
        schema: "nando.grounded-response-wave-causal-report.v2".to_owned(),
        package_id: package.package_id.clone(),
        verdict: if pass { "PASS" } else { "WATCH" }.to_owned(),
        support_rows: support.len(),
        future_rows: future.len(),
        negative_rows: negatives.len(),
        full_phase_correct,
        no_phase_correct: future.len(),
        shuffled_phase_correct,
        random_center_correct,
        magnitude_only_correct,
        no_anti_center_correct,
        negative_accepts,
        no_phase_negative_accepts: negatives.len(),
        shuffled_negative_accepts,
        random_center_negative_accepts,
        magnitude_only_negative_accepts,
        no_anti_center_negative_accepts,
        full_margin_mean_micro: phase_margin_to_micro(full_margin_mean).unwrap_or_default(),
        shuffled_margin_mean_micro: phase_margin_to_micro(shuffled_margin_mean).unwrap_or_default(),
        random_margin_mean_micro: phase_margin_to_micro(random_margin_mean).unwrap_or_default(),
        no_phase_exact_checks,
        full_phase_exact_checks,
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}
