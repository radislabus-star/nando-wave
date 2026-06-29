#![allow(dead_code)]

use std::collections::HashSet;
use std::mem::size_of;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use nando_core::{L1CenterMemoryConfig, L2CenterMemory, L2CenterMemoryConfig};

const AXIS_CENTERS_COUNT: usize = 512;
const INTERACTION_EDGES_COUNT: usize = 131_072;
const ANTI_WAVE_EDGES_COUNT: usize = 65_536;
const L3_EDGE_OFFSET_COUNT: usize = AXIS_CENTERS_COUNT + 1;
const ACTIVE_CENTER_LIMIT: usize = 32;
const SETTLE_STEPS: usize = 5;
const MOTIF_COUNT: usize = 65_536;

const SMOKE_TRAIN_WORDS: usize = 4_000;
const SMOKE_HELDOUT_WORDS: usize = 512;
const RELEASE_TRAIN_WORDS: usize = 20_000;
const RELEASE_HELDOUT_WORDS: usize = 5_000;

const MAX_INTERACTION_EDGES_PER_CENTER: usize = 256;
const MAX_ANTI_EDGES_PER_CENTER: usize = 128;
const VETO_THRESHOLD: i32 = 8_000;
const VETO_MARGIN: i32 = 2_240;
const GAP_THRESHOLD: i16 = 48;

#[repr(C, align(8))]
#[derive(Clone, Copy, Default)]
struct AlignedEdge {
    center_a_id: u16,
    center_b_id: u16,
    compatibility: i16,
    conflict: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InferenceStatus {
    Success,
    FieldUnsettled,
}

#[derive(Clone, Copy, Debug)]
struct InferenceResult {
    status: InferenceStatus,
    early_veto: bool,
    seed_center_count: usize,
    active_center_peak: usize,
    interaction_edges_visited: usize,
    anti_edges_visited: usize,
}

#[derive(Clone, Debug)]
struct BenchReport {
    queries: usize,
    p50_latency: Duration,
    p99_latency: Duration,
    seed_center_p99: usize,
    active_center_p99: usize,
    interaction_edges_visited_p99: usize,
    anti_edges_visited_p99: usize,
    total_false_accepts: usize,
    trap_rejection_milli: u16,
    trap_early_veto_milli: u16,
    normal_early_veto_milli: u16,
    normal_success_milli: u16,
}

#[derive(Clone, Debug)]
struct L3InductionReport {
    train_examples: usize,
    trap_examples: usize,
    trained_interaction_edges: usize,
    trained_anti_wave_edges: usize,
    center_mass_nonzero: usize,
    manual_edge_simulation: bool,
}

#[derive(Clone, Debug)]
struct CorpusQuery {
    centers: Vec<u16>,
    is_trap: bool,
}

struct CorpusCenterSets {
    train_positive: Vec<Vec<u16>>,
    heldout_positive: Vec<Vec<u16>>,
}

struct TrainedL3Field {
    interaction_offsets: Box<[u32; L3_EDGE_OFFSET_COUNT]>,
    interaction_edges: Box<[AlignedEdge; INTERACTION_EDGES_COUNT]>,
    anti_wave_offsets: Box<[u32; L3_EDGE_OFFSET_COUNT]>,
    anti_wave_edges: Box<[AlignedEdge; ANTI_WAVE_EDGES_COUNT]>,
    interaction_edge_count: usize,
    anti_wave_edge_count: usize,
    center_mass: [u32; AXIS_CENTERS_COUNT],
}

struct L3Scratch {
    center_activation: Box<[i16; AXIS_CENTERS_COUNT]>,
    center_next: Box<[i16; AXIS_CENTERS_COUNT]>,
    active_centers: Box<[u16; ACTIVE_CENTER_LIMIT]>,
    next_active_centers: Box<[u16; ACTIVE_CENTER_LIMIT]>,
    active_marks: Box<[bool; AXIS_CENTERS_COUNT]>,
    next_active_marks: Box<[bool; AXIS_CENTERS_COUNT]>,
    seed_marks: Box<[bool; AXIS_CENTERS_COUNT]>,
    active_count: usize,
    next_active_count: usize,
    seed_center_count: usize,
    active_center_peak: usize,
    interaction_edges_visited: usize,
    anti_edges_visited: usize,
}

impl L3Scratch {
    fn new() -> Self {
        Self {
            center_activation: boxed_array(0),
            center_next: boxed_array(0),
            active_centers: boxed_array(0),
            next_active_centers: boxed_array(0),
            active_marks: boxed_array(false),
            next_active_marks: boxed_array(false),
            seed_marks: boxed_array(false),
            active_count: 0,
            next_active_count: 0,
            seed_center_count: 0,
            active_center_peak: 0,
            interaction_edges_visited: 0,
            anti_edges_visited: 0,
        }
    }

    fn reset(&mut self) {
        self.center_activation.fill(0);
        self.center_next.fill(0);
        self.active_marks.fill(false);
        self.next_active_marks.fill(false);
        self.seed_marks.fill(false);
        self.active_count = 0;
        self.next_active_count = 0;
        self.seed_center_count = 0;
        self.active_center_peak = 0;
        self.interaction_edges_visited = 0;
        self.anti_edges_visited = 0;
    }

    fn seed_center(&mut self, center_id: usize, delta: i16) {
        if center_id >= AXIS_CENTERS_COUNT {
            return;
        }
        if !self.seed_marks[center_id] {
            self.seed_marks[center_id] = true;
            self.seed_center_count += 1;
        }
        self.center_activation[center_id] = self.center_activation[center_id].saturating_add(delta);
        if !self.active_marks[center_id] && self.active_count < ACTIVE_CENTER_LIMIT {
            self.active_centers[self.active_count] = center_id as u16;
            self.active_count += 1;
            self.active_marks[center_id] = true;
            self.active_center_peak = self.active_center_peak.max(self.active_count);
        }
    }

    fn push_next_active_center(&mut self, center_id: usize) {
        if center_id >= AXIS_CENTERS_COUNT || self.next_active_marks[center_id] {
            return;
        }
        if self.next_active_count < ACTIVE_CENTER_LIMIT {
            self.next_active_centers[self.next_active_count] = center_id as u16;
            self.next_active_count += 1;
            self.next_active_marks[center_id] = true;
            self.active_center_peak = self.active_center_peak.max(self.next_active_count);
        }
    }

    fn rotate_active_frontier(&mut self) {
        std::mem::swap(&mut self.active_centers, &mut self.next_active_centers);
        std::mem::swap(&mut self.active_marks, &mut self.next_active_marks);
        self.active_count = self.next_active_count;
        self.next_active_count = 0;
        self.next_active_marks.fill(false);
    }
}

#[test]
fn corpus_l3_layout_keeps_aligned_edge_contract() {
    assert_eq!(size_of::<AlignedEdge>(), 8);
    assert_eq!(
        size_of::<AlignedEdge>() * INTERACTION_EDGES_COUNT,
        1_048_576
    );
    assert_eq!(size_of::<AlignedEdge>() * ANTI_WAVE_EDGES_COUNT, 524_288);
    assert_eq!(size_of::<u32>() * L3_EDGE_OFFSET_COUNT, 2_052);
}

#[test]
fn corpus_l3_induction_smoke_uses_learned_edges() {
    let corpus = corpus_center_sets(SMOKE_TRAIN_WORDS, SMOKE_HELDOUT_WORDS);
    let (field, induction) = induce_l3_field(&corpus.train_positive);
    let queries = heldout_queries(&corpus.heldout_positive);
    let report = run_corpus_l3_bench(&field, &queries);

    eprintln!("corpus-trained L3 induction smoke: {induction:#?}");
    eprintln!("corpus-trained L3 bench smoke: {report:#?}");

    assert!(!induction.manual_edge_simulation);
    assert!(induction.trained_interaction_edges > 0);
    assert!(induction.trained_anti_wave_edges > 0);
    assert_eq!(report.total_false_accepts, 0, "report={report:#?}");
    assert_eq!(report.trap_rejection_milli, 1_000, "report={report:#?}");
    assert!(
        report.trap_early_veto_milli >= 400,
        "early veto should reject a meaningful trap share before settle: {report:#?}"
    );
    assert!(
        report.normal_early_veto_milli <= 300,
        "early veto is over-silencing normal queries: {report:#?}"
    );
    assert!(
        report.active_center_p99 <= ACTIVE_CENTER_LIMIT,
        "active center limit exceeded: {report:#?}"
    );
}

#[test]
#[ignore = "release-only corpus-trained L3 field induction gate; run explicitly with --release"]
fn corpus_l3_release_induction_latency_gate() {
    #[cfg(debug_assertions)]
    {
        panic!(
            "run with: cargo test -p nando-core --release --test wave_full_v1_corpus_l3_bench -- --ignored"
        );
    }

    #[cfg(not(debug_assertions))]
    {
        let corpus = corpus_center_sets(RELEASE_TRAIN_WORDS, RELEASE_HELDOUT_WORDS);
        let (field, induction) = induce_l3_field(&corpus.train_positive);
        let queries = heldout_queries(&corpus.heldout_positive);
        let report = run_corpus_l3_bench(&field, &queries);

        eprintln!("corpus-trained L3 induction release: {induction:#?}");
        eprintln!("corpus-trained L3 bench release: {report:#?}");

        assert!(!induction.manual_edge_simulation);
        assert!(induction.trained_interaction_edges > 8_000);
        assert!(induction.trained_anti_wave_edges > 1_000);
        assert_eq!(report.total_false_accepts, 0, "report={report:#?}");
        assert_eq!(report.trap_rejection_milli, 1_000, "report={report:#?}");
        assert!(
            report.trap_early_veto_milli >= 400,
            "release early veto gate failed: {report:#?}"
        );
        assert!(
            report.normal_early_veto_milli <= 300,
            "release early veto over-silenced normal queries: {report:#?}"
        );
        assert!(
            report.seed_center_p99 <= ACTIVE_CENTER_LIMIT,
            "seed-center gate failed: {report:#?}"
        );
        assert!(
            report.active_center_p99 <= ACTIVE_CENTER_LIMIT,
            "active-center gate failed: {report:#?}"
        );
        assert!(
            report.interaction_edges_visited_p99 < INTERACTION_EDGES_COUNT * SETTLE_STEPS,
            "active interaction traversal did not beat full scan: {report:#?}"
        );
        assert!(
            report.p99_latency <= Duration::from_micros(500),
            "release P99 latency gate failed: {report:#?}"
        );
    }
}

fn induce_l3_field(train_sets: &[Vec<u16>]) -> (TrainedL3Field, L3InductionReport) {
    let mut cooccur = vec![0_u32; AXIS_CENTERS_COUNT * AXIS_CENTERS_COUNT];
    let mut conflict = vec![0_u32; AXIS_CENTERS_COUNT * AXIS_CENTERS_COUNT];
    let mut center_mass = [0_u32; AXIS_CENTERS_COUNT];
    let mut trap_examples = 0;

    for centers in train_sets {
        accumulate_positive(centers, &mut center_mass, &mut cooccur);
    }

    for pair in train_sets.windows(2).step_by(2) {
        let trap = splice_centers(&pair[0], &pair[1]);
        if trap.len() >= 2 {
            trap_examples += 1;
            accumulate_conflict(&trap, &mut conflict);
        }
    }

    let interaction_candidates = build_interaction_candidates(&cooccur, &conflict, &center_mass);
    let anti_candidates = build_anti_candidates(&cooccur, &conflict, &center_mass);
    let (interaction_edges, interaction_offsets, interaction_count) =
        pack_edges::<INTERACTION_EDGES_COUNT>(
            interaction_candidates,
            MAX_INTERACTION_EDGES_PER_CENTER,
        );
    let (anti_wave_edges, anti_wave_offsets, anti_count) =
        pack_edges::<ANTI_WAVE_EDGES_COUNT>(anti_candidates, MAX_ANTI_EDGES_PER_CENTER);
    let center_mass_nonzero = center_mass.iter().filter(|mass| **mass > 0).count();

    let field = TrainedL3Field {
        interaction_offsets,
        interaction_edges,
        anti_wave_offsets,
        anti_wave_edges,
        interaction_edge_count: interaction_count,
        anti_wave_edge_count: anti_count,
        center_mass,
    };
    let report = L3InductionReport {
        train_examples: train_sets.len(),
        trap_examples,
        trained_interaction_edges: interaction_count,
        trained_anti_wave_edges: anti_count,
        center_mass_nonzero,
        manual_edge_simulation: false,
    };

    (field, report)
}

fn accumulate_positive(centers: &[u16], center_mass: &mut [u32], cooccur: &mut [u32]) {
    for &center in centers {
        center_mass[usize::from(center)] += 1;
    }
    for (left_index, &a) in centers.iter().enumerate() {
        for &b in &centers[left_index + 1..] {
            bump_pair(cooccur, a, b, 1);
        }
    }
}

fn accumulate_conflict(centers: &[u16], conflict: &mut [u32]) {
    for (left_index, &a) in centers.iter().enumerate() {
        for &b in &centers[left_index + 1..] {
            bump_pair(conflict, a, b, 2);
        }
    }
}

fn bump_pair(matrix: &mut [u32], a: u16, b: u16, amount: u32) {
    let a = usize::from(a);
    let b = usize::from(b);
    matrix[pair_index(a, b)] = matrix[pair_index(a, b)].saturating_add(amount);
    matrix[pair_index(b, a)] = matrix[pair_index(b, a)].saturating_add(amount);
}

fn build_interaction_candidates(
    cooccur: &[u32],
    conflict: &[u32],
    center_mass: &[u32; AXIS_CENTERS_COUNT],
) -> Vec<AlignedEdge> {
    let mut edges = Vec::new();
    for a in 0..AXIS_CENTERS_COUNT {
        if center_mass[a] == 0 {
            continue;
        }
        for b in 0..AXIS_CENTERS_COUNT {
            if a == b || center_mass[b] == 0 {
                continue;
            }
            let co = cooccur[pair_index(a, b)];
            let cf = conflict[pair_index(a, b)];
            if co == 0 && cf < 2 {
                continue;
            }
            let norm = ((u64::from(center_mass[a]) * u64::from(center_mass[b])) as f64)
                .sqrt()
                .max(1.0);
            let compat = ((f64::from(co) / norm) * 768.0).round() as i32;
            let soft_conflict =
                ((f64::from(cf.min(co.saturating_add(cf))) / norm) * 384.0).round() as i32;
            if compat < 2 && soft_conflict < 2 {
                continue;
            }
            edges.push(AlignedEdge {
                center_a_id: a as u16,
                center_b_id: b as u16,
                compatibility: compat.clamp(0, 512) as i16,
                conflict: soft_conflict.clamp(0, 256) as i16,
            });
        }
    }
    edges
}

fn build_anti_candidates(
    cooccur: &[u32],
    conflict: &[u32],
    center_mass: &[u32; AXIS_CENTERS_COUNT],
) -> Vec<AlignedEdge> {
    let mut edges = Vec::new();
    for a in 0..AXIS_CENTERS_COUNT {
        if center_mass[a] == 0 {
            continue;
        }
        for b in 0..AXIS_CENTERS_COUNT {
            if a == b || center_mass[b] == 0 {
                continue;
            }
            let cf = conflict[pair_index(a, b)];
            if cf < 2 {
                continue;
            }
            let co = cooccur[pair_index(a, b)];
            let conflict_dominates = cf >= co.saturating_mul(2).saturating_add(2);
            if !conflict_dominates {
                continue;
            }
            let norm = ((u64::from(center_mass[a]) * u64::from(center_mass[b])) as f64)
                .sqrt()
                .max(1.0);
            let anti_weight = ((f64::from(cf) / norm) * 2048.0).round() as i32;
            if anti_weight < 24 {
                continue;
            }
            let local_margin = 32 + co.min(16) as i32 * 4;
            edges.push(AlignedEdge {
                center_a_id: a as u16,
                center_b_id: b as u16,
                compatibility: anti_weight.clamp(24, 2048) as i16,
                conflict: local_margin.clamp(32, 256) as i16,
            });
        }
    }
    edges
}

fn pack_edges<const N: usize>(
    mut candidates: Vec<AlignedEdge>,
    max_per_center: usize,
) -> (
    Box<[AlignedEdge; N]>,
    Box<[u32; L3_EDGE_OFFSET_COUNT]>,
    usize,
) {
    candidates.sort_by_key(|edge| {
        (
            edge.center_a_id,
            -(i32::from(edge.compatibility) - i32::from(edge.conflict)),
            edge.center_b_id,
        )
    });

    let mut edges = boxed_array(AlignedEdge::default());
    let mut offsets = boxed_array(0_u32);
    let mut cursor = 0;
    let mut source_index = 0;

    for center_id in 0..AXIS_CENTERS_COUNT {
        offsets[center_id] = cursor as u32;
        let mut added = 0;
        while source_index < candidates.len()
            && usize::from(candidates[source_index].center_a_id) < center_id
        {
            source_index += 1;
        }
        let mut scan = source_index;
        while scan < candidates.len()
            && usize::from(candidates[scan].center_a_id) == center_id
            && added < max_per_center
            && cursor < N
        {
            edges[cursor] = candidates[scan];
            cursor += 1;
            added += 1;
            scan += 1;
        }
        offsets[center_id + 1] = cursor as u32;
    }

    (edges, offsets, cursor)
}

fn run_corpus_l3_bench(field: &TrainedL3Field, queries: &[CorpusQuery]) -> BenchReport {
    let mut scratch = L3Scratch::new();
    let mut latencies = Vec::with_capacity(queries.len());
    let mut seed_counts = Vec::with_capacity(queries.len());
    let mut active_peaks = Vec::with_capacity(queries.len());
    let mut interaction_edges = Vec::with_capacity(queries.len());
    let mut anti_edges = Vec::with_capacity(queries.len());
    let mut false_accepts = 0;
    let mut total_traps = 0;
    let mut total_normals = 0;
    let mut rejected_traps = 0;
    let mut early_veto_traps = 0;
    let mut early_veto_normals = 0;
    let mut successful_normals = 0;

    for query in queries {
        if query.is_trap {
            total_traps += 1;
        } else {
            total_normals += 1;
        }
        let start = Instant::now();
        let result = std::hint::black_box(field.infer(&mut scratch, &query.centers));
        latencies.push(start.elapsed());
        seed_counts.push(result.seed_center_count);
        active_peaks.push(result.active_center_peak);
        interaction_edges.push(result.interaction_edges_visited);
        anti_edges.push(result.anti_edges_visited);

        if query.is_trap {
            if result.status == InferenceStatus::Success {
                false_accepts += 1;
            } else {
                rejected_traps += 1;
            }
            if result.early_veto {
                early_veto_traps += 1;
            }
        } else {
            if result.early_veto {
                early_veto_normals += 1;
            }
            if result.status == InferenceStatus::Success {
                successful_normals += 1;
            }
        }
    }

    latencies.sort_unstable();
    seed_counts.sort_unstable();
    active_peaks.sort_unstable();
    interaction_edges.sort_unstable();
    anti_edges.sort_unstable();
    let trap_rejection_milli = milli_ratio(rejected_traps, total_traps);
    let trap_early_veto_milli = milli_ratio(early_veto_traps, total_traps);
    let normal_early_veto_milli = milli_ratio(early_veto_normals, total_normals);
    let normal_success_milli = milli_ratio(successful_normals, total_normals);

    BenchReport {
        queries: queries.len(),
        p50_latency: latencies[percentile_index(latencies.len(), 50)],
        p99_latency: latencies[percentile_index(latencies.len(), 99)],
        seed_center_p99: seed_counts[percentile_index(seed_counts.len(), 99)],
        active_center_p99: active_peaks[percentile_index(active_peaks.len(), 99)],
        interaction_edges_visited_p99: interaction_edges
            [percentile_index(interaction_edges.len(), 99)],
        anti_edges_visited_p99: anti_edges[percentile_index(anti_edges.len(), 99)],
        total_false_accepts: false_accepts,
        trap_rejection_milli,
        trap_early_veto_milli,
        normal_early_veto_milli,
        normal_success_milli,
    }
}

impl TrainedL3Field {
    fn infer(&self, scratch: &mut L3Scratch, centers: &[u16]) -> InferenceResult {
        scratch.reset();
        for &center in centers {
            let center_id = usize::from(center);
            let mass = self.center_mass[center_id].max(1);
            let delta = (32 + (mass.ilog2() as i16 * 2)).clamp(32, 96);
            scratch.seed_center(center_id, delta);
        }

        let seed_positive_energy = positive_energy(scratch);
        let early_veto = self.evaluate_early_veto(scratch, seed_positive_energy);
        if early_veto {
            return InferenceResult {
                status: InferenceStatus::FieldUnsettled,
                early_veto: true,
                seed_center_count: scratch.seed_center_count,
                active_center_peak: scratch.active_center_peak,
                interaction_edges_visited: scratch.interaction_edges_visited,
                anti_edges_visited: scratch.anti_edges_visited,
            };
        }

        self.settle_l3(scratch);
        let late_veto = self.evaluate_early_veto(scratch, positive_energy(scratch));
        let (_, best, second) = top_two_centers(&scratch.center_activation);
        let gap = best.saturating_sub(second);
        let status = if late_veto || gap < GAP_THRESHOLD {
            InferenceStatus::FieldUnsettled
        } else {
            InferenceStatus::Success
        };

        InferenceResult {
            status,
            early_veto: false,
            seed_center_count: scratch.seed_center_count,
            active_center_peak: scratch.active_center_peak,
            interaction_edges_visited: scratch.interaction_edges_visited,
            anti_edges_visited: scratch.anti_edges_visited,
        }
    }

    fn evaluate_early_veto(&self, scratch: &mut L3Scratch, positive_energy: i32) -> bool {
        let mut anti_energy = 0_i32;
        for index in 0..scratch.active_count {
            let a = usize::from(scratch.active_centers[index]);
            let start = self.anti_wave_offsets[a] as usize;
            let end = self.anti_wave_offsets[a + 1] as usize;
            scratch.anti_edges_visited += end.saturating_sub(start);
            for edge in self.anti_wave_edges[start..end].iter().copied() {
                let b = usize::from(edge.center_b_id);
                if b >= AXIS_CENTERS_COUNT || !scratch.active_marks[b] {
                    continue;
                }
                let act_a = i32::from(scratch.center_activation[a].abs());
                let act_b = i32::from(scratch.center_activation[b].abs());
                let link_energy = i32::from(edge.compatibility) * act_a.min(act_b);
                if link_energy >= i32::from(edge.conflict) {
                    anti_energy += link_energy;
                }
            }
        }

        anti_energy >= VETO_THRESHOLD && anti_energy - positive_energy >= VETO_MARGIN
    }

    fn settle_l3(&self, scratch: &mut L3Scratch) {
        for _ in 0..SETTLE_STEPS {
            scratch
                .center_next
                .copy_from_slice(&scratch.center_activation[..]);
            for active_index in 0..scratch.active_count {
                let a = usize::from(scratch.active_centers[active_index]);
                scratch.push_next_active_center(a);
                let source = i32::from(scratch.center_activation[a]);
                if source == 0 {
                    continue;
                }
                let start = self.interaction_offsets[a] as usize;
                let end = self.interaction_offsets[a + 1] as usize;
                scratch.interaction_edges_visited += end.saturating_sub(start);
                for edge in self.interaction_edges[start..end].iter().copied() {
                    let b = usize::from(edge.center_b_id);
                    if b >= AXIS_CENTERS_COUNT {
                        continue;
                    }
                    let compat = i32::from(edge.compatibility);
                    let conflict = i32::from(edge.conflict);
                    let delta = ((source * compat) - (source.abs() * conflict)) >> 11;
                    if delta != 0 {
                        scratch.center_next[b] =
                            saturating_i16_add_i32(scratch.center_next[b], delta);
                        scratch.push_next_active_center(b);
                    }
                }
            }
            scratch
                .center_activation
                .copy_from_slice(&scratch.center_next[..]);
            scratch.rotate_active_frontier();
        }
    }
}

fn positive_energy(scratch: &L3Scratch) -> i32 {
    scratch
        .center_activation
        .iter()
        .fold(0_i32, |total, activation| {
            total + i32::from(activation.abs())
        })
}

fn corpus_center_sets(train_words: usize, heldout_words: usize) -> CorpusCenterSets {
    let words = corpus_words("russian_words_300k.txt");
    assert!(
        words.len() >= train_words + heldout_words,
        "russian corpus too small for corpus-trained L3 proof"
    );
    let l2 = L2CenterMemory::build(
        words[..train_words].iter().map(String::as_str),
        real_l2_config(),
    );
    let train_positive = center_sets_for_words(&l2, &words[..train_words]);
    let heldout_positive =
        center_sets_for_words(&l2, &words[train_words..train_words + heldout_words]);

    assert!(
        train_positive.len() >= train_words / 2,
        "too few train words produced L2 center sets: {} of {}",
        train_positive.len(),
        train_words
    );
    assert!(
        heldout_positive.len() >= heldout_words / 2,
        "too few heldout words produced L2 center sets: {} of {}",
        heldout_positive.len(),
        heldout_words
    );

    CorpusCenterSets {
        train_positive,
        heldout_positive,
    }
}

fn center_sets_for_words(l2: &L2CenterMemory, words: &[String]) -> Vec<Vec<u16>> {
    words
        .iter()
        .filter_map(|word| {
            let centers = centers_for_text(l2, word);
            if centers.len() >= 2 {
                Some(centers)
            } else {
                None
            }
        })
        .collect()
}

fn heldout_queries(heldout: &[Vec<u16>]) -> Vec<CorpusQuery> {
    let mut queries = Vec::with_capacity(heldout.len());
    for index in 0..heldout.len() {
        if index % 10 >= 7 && index + 1 < heldout.len() {
            let centers = splice_centers(&heldout[index], &heldout[index + 1]);
            if centers.len() >= 2 {
                queries.push(CorpusQuery {
                    centers,
                    is_trap: true,
                });
            }
        } else {
            queries.push(CorpusQuery {
                centers: heldout[index].clone(),
                is_trap: false,
            });
        }
    }
    assert!(queries.iter().any(|query| query.is_trap));
    queries
}

fn splice_centers(left: &[u16], right: &[u16]) -> Vec<u16> {
    let left_take = left.len().div_ceil(2).min(ACTIVE_CENTER_LIMIT / 2);
    let right_take = right.len().div_ceil(2).min(ACTIVE_CENTER_LIMIT - left_take);
    let mut seen = HashSet::new();
    let mut centers = Vec::with_capacity(left_take + right_take);
    for &center in left.iter().take(left_take) {
        if seen.insert(center) {
            centers.push(center);
        }
    }
    for &center in right.iter().rev().take(right_take) {
        if seen.insert(center) {
            centers.push(center);
        }
    }
    centers
}

fn centers_for_text(l2: &L2CenterMemory, text: &str) -> Vec<u16> {
    let mut seen = HashSet::new();
    let mut centers = Vec::new();
    for token in l2
        .token_sequence_for_text(text)
        .tokens
        .into_iter()
        .filter(|token| token & (1 << 31) == 0)
    {
        let center = center_id_for_l2_token(token) as u16;
        if seen.insert(center) {
            centers.push(center);
        }
        if centers.len() >= ACTIVE_CENTER_LIMIT {
            break;
        }
    }
    centers
}

fn real_l2_config() -> L2CenterMemoryConfig {
    L2CenterMemoryConfig {
        l1_config: L1CenterMemoryConfig {
            min_center_support: 2,
            min_heldout_ngram_coverage: 0.70,
            min_average_reconstruction_similarity: 0.68,
            min_average_fourier_similarity: 0.64,
            min_fourier_ablation_drop: 0.03,
            min_real_vs_corrupt_coverage_gap: 0.12,
            max_model_to_naive_ratio: 0.12,
            max_corrupt_eval_words: 1_024,
            max_fourier_eval_words: 512,
            ..L1CenterMemoryConfig::default()
        },
        motif_len: 3,
        min_motif_support: 4,
        max_motifs: MOTIF_COUNT,
        min_heldout_ref_coverage: 0.60,
        min_heldout_word_coverage: 0.50,
        min_average_sequence_similarity: 0.65,
        min_average_fourier_similarity: 0.65,
        min_fourier_ablation_drop: 0.20,
        min_real_vs_corrupt_coverage_gap: 0.30,
        max_model_to_naive_ratio: 0.90,
        max_corrupt_eval_words: 1_024,
        max_fourier_eval_words: 512,
        ..L2CenterMemoryConfig::default()
    }
}

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/corpus")
        .join(name)
}

fn corpus_words(name: &str) -> Vec<String> {
    std::fs::read_to_string(corpus_path(name))
        .unwrap_or_else(|error| panic!("{name} corpus file must be readable: {error}"))
        .lines()
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

fn pair_index(a: usize, b: usize) -> usize {
    a * AXIS_CENTERS_COUNT + b
}

fn center_id_for_l2_token(token: u32) -> usize {
    (splitmix64(u64::from(token) ^ 0xF311_0001_4C32) as usize) & (AXIS_CENTERS_COUNT - 1)
}

fn top_two_centers(centers: &[i16; AXIS_CENTERS_COUNT]) -> (u16, i16, i16) {
    let mut best_id = 0;
    let mut best = i16::MIN;
    let mut second = i16::MIN;

    for (center_id, score) in centers.iter().copied().enumerate() {
        if score > best {
            second = best;
            best = score;
            best_id = center_id as u16;
        } else if score > second {
            second = score;
        }
    }

    (best_id, best, second)
}

fn saturating_i16_add_i32(value: i16, delta: i32) -> i16 {
    let next = i32::from(value) + delta;
    next.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

fn percentile_index(len: usize, percentile: usize) -> usize {
    if len <= 1 {
        0
    } else {
        ((len - 1) * percentile) / 100
    }
}

fn milli_ratio(numerator: usize, denominator: usize) -> u16 {
    numerator
        .checked_mul(1_000)
        .and_then(|scaled| scaled.checked_div(denominator))
        .unwrap_or(1_000) as u16
}

fn boxed_array<T: Clone, const N: usize>(value: T) -> Box<[T; N]> {
    let boxed_slice = vec![value; N].into_boxed_slice();
    match boxed_slice.try_into() {
        Ok(array) => array,
        Err(_) => panic!("boxed slice length did not match requested array length"),
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}
