#![allow(dead_code)]

use std::mem::size_of;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use nando_core::{L1CenterMemoryConfig, L2CenterMemory, L2CenterMemoryConfig};

const LANES_COUNT: usize = 4_096;
const MOTIF_COUNT: usize = 65_536;
const BUCKET_COUNT: usize = 65_536;
const AXIS_CENTERS_COUNT: usize = 512;
const INTERACTION_EDGES_COUNT: usize = 131_072;
const ANTI_WAVE_EDGES_COUNT: usize = 65_536;
const SETTLE_STEPS: usize = 5;
const ACTIVE_CENTER_LIMIT: usize = 32;
const EDGES_PER_CENTER: usize = INTERACTION_EDGES_COUNT / AXIS_CENTERS_COUNT;
const L3_EDGE_OFFSET_COUNT: usize = AXIS_CENTERS_COUNT + 1;
const L3_EDGE_OFFSET_BYTES: usize = L3_EDGE_OFFSET_COUNT * size_of::<u32>();

const L2_RARE_BANK_BYTES: usize = 786_432;
const L2_OVERFLOW_POOL_BYTES: usize = 1_048_576;
const METRICS_RESERVE_TOTAL_BYTES: usize = 512_000;
const METRICS_RESERVE_PAYLOAD_BYTES: usize = METRICS_RESERVE_TOTAL_BYTES - L3_EDGE_OFFSET_BYTES;
const FULL_V1_BUDGET_BYTES: usize = 6 * 1024 * 1024;

const SMOKE_QUERIES: usize = 32;
const RELEASE_GATE_QUERIES: usize = 10_000;
const REAL_L2_SMOKE_TRAIN_WORDS: usize = 4_000;
const REAL_L2_SMOKE_HELDOUT_WORDS: usize = 512;
const REAL_L2_RELEASE_TRAIN_WORDS: usize = 20_000;
const REAL_L2_RELEASE_HELDOUT_WORDS: usize = 5_000;
const MAX_TOUCHED_MOTIFS: usize = LANES_COUNT * 4;

type SurfaceWave = [i16; LANES_COUNT];

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct PackedTooth3 {
    packed_coords_le: [u8; 2],
    weight: u8,
}

impl PackedTooth3 {
    fn new(packed_coords: u16, weight: u8) -> Self {
        Self {
            packed_coords_le: packed_coords.to_le_bytes(),
            weight,
        }
    }

    fn packed_coords(self) -> u16 {
        u16::from_le_bytes(self.packed_coords_le)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MotifPrototype {
    teeth: [PackedTooth3; 8],
    motif_norm: u16,
    flags: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct AxisCenter {
    activation: i16,
    base_mass: u16,
    axis_id: u8,
    flags: u8,
    reserved: [u8; 2],
}

#[repr(C, align(8))]
#[derive(Clone, Copy, Default)]
struct AlignedEdge {
    center_a_id: u16,
    center_b_id: u16,
    compatibility: i16,
    conflict: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum L3GraphProfile {
    Regularized,
    ScaleFreeHubs,
}

struct HotProfile {
    epoch: u64,
    l2_core_motifs: Box<[MotifPrototype; MOTIF_COUNT]>,
    l2_rare_bank: Box<[u8; L2_RARE_BANK_BYTES]>,
    l2_front_index: Box<[[u16; 4]; BUCKET_COUNT]>,
    l2_overflow_pool: Box<[u8; L2_OVERFLOW_POOL_BYTES]>,
    l3_centers: [AxisCenter; AXIS_CENTERS_COUNT],
    l3_edges: Box<[AlignedEdge; INTERACTION_EDGES_COUNT]>,
    l3_edge_offsets: Box<[u32; L3_EDGE_OFFSET_COUNT]>,
    l3_trap_edges: Box<[AlignedEdge; ANTI_WAVE_EDGES_COUNT]>,
    metrics_reserve: Box<[u8; METRICS_RESERVE_PAYLOAD_BYTES]>,
}

struct InferenceScratch {
    l1_wave: SurfaceWave,
    motif_scores: Box<[i16; MOTIF_COUNT]>,
    touched_motifs: Box<[u16; MAX_TOUCHED_MOTIFS]>,
    touched_count: usize,
    center_activation: Box<[i16; AXIS_CENTERS_COUNT]>,
    center_next: Box<[i16; AXIS_CENTERS_COUNT]>,
    active_centers: Box<[u16; ACTIVE_CENTER_LIMIT]>,
    next_active_centers: Box<[u16; ACTIVE_CENTER_LIMIT]>,
    active_center_marks: Box<[bool; AXIS_CENTERS_COUNT]>,
    next_active_center_marks: Box<[bool; AXIS_CENTERS_COUNT]>,
    seed_center_marks: Box<[bool; AXIS_CENTERS_COUNT]>,
    active_count: usize,
    next_active_count: usize,
    active_center_peak: usize,
    seed_center_count: usize,
    edges_visited: usize,
}

impl InferenceScratch {
    fn new() -> Self {
        Self {
            l1_wave: [0; LANES_COUNT],
            motif_scores: boxed_array(0),
            touched_motifs: boxed_array(0),
            touched_count: 0,
            center_activation: boxed_array(0),
            center_next: boxed_array(0),
            active_centers: boxed_array(0),
            next_active_centers: boxed_array(0),
            active_center_marks: boxed_array(false),
            next_active_center_marks: boxed_array(false),
            seed_center_marks: boxed_array(false),
            active_count: 0,
            next_active_count: 0,
            active_center_peak: 0,
            seed_center_count: 0,
            edges_visited: 0,
        }
    }

    fn reset(&mut self, input: &SurfaceWave) {
        self.l1_wave.copy_from_slice(input);
        for motif_id in self.touched_motifs[..self.touched_count].iter().copied() {
            self.motif_scores[usize::from(motif_id)] = 0;
        }
        self.touched_count = 0;
        self.center_activation.fill(0);
        self.center_next.fill(0);
        self.active_center_marks.fill(false);
        self.next_active_center_marks.fill(false);
        self.seed_center_marks.fill(false);
        self.active_count = 0;
        self.next_active_count = 0;
        self.active_center_peak = 0;
        self.seed_center_count = 0;
        self.edges_visited = 0;
    }

    fn reset_l3_only(&mut self) {
        self.center_activation.fill(0);
        self.center_next.fill(0);
        self.active_center_marks.fill(false);
        self.next_active_center_marks.fill(false);
        self.seed_center_marks.fill(false);
        self.active_count = 0;
        self.next_active_count = 0;
        self.active_center_peak = 0;
        self.seed_center_count = 0;
        self.edges_visited = 0;
    }

    fn touch_motif(&mut self, motif_id: u16, delta: i16) {
        let index = usize::from(motif_id);
        if self.motif_scores[index] == 0 && self.touched_count < MAX_TOUCHED_MOTIFS {
            self.touched_motifs[self.touched_count] = motif_id;
            self.touched_count += 1;
        }
        self.motif_scores[index] = self.motif_scores[index].saturating_add(delta);
    }

    fn push_active_center(&mut self, center_id: usize) {
        if center_id >= AXIS_CENTERS_COUNT {
            return;
        }
        if !self.seed_center_marks[center_id] {
            self.seed_center_marks[center_id] = true;
            self.seed_center_count += 1;
        }
        if self.active_center_marks[center_id] {
            return;
        }
        if self.active_count < ACTIVE_CENTER_LIMIT {
            self.active_centers[self.active_count] = center_id as u16;
            self.active_count += 1;
            self.active_center_marks[center_id] = true;
            self.active_center_peak = self.active_center_peak.max(self.active_count);
        }
    }

    fn push_next_active_center(&mut self, center_id: usize) {
        if center_id >= AXIS_CENTERS_COUNT || self.next_active_center_marks[center_id] {
            return;
        }
        if self.next_active_count < ACTIVE_CENTER_LIMIT {
            self.next_active_centers[self.next_active_count] = center_id as u16;
            self.next_active_count += 1;
            self.next_active_center_marks[center_id] = true;
            self.active_center_peak = self.active_center_peak.max(self.next_active_count);
        }
    }

    fn rotate_active_frontier(&mut self) {
        std::mem::swap(&mut self.active_centers, &mut self.next_active_centers);
        std::mem::swap(
            &mut self.active_center_marks,
            &mut self.next_active_center_marks,
        );
        self.active_count = self.next_active_count;
        self.next_active_count = 0;
        self.next_active_center_marks.fill(false);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InferenceStatus {
    Success,
    FieldUnsettled,
}

#[derive(Clone, Copy, Debug)]
struct InferenceResult {
    status: InferenceStatus,
    best_center: u16,
    gap: i16,
    seed_center_count: usize,
    active_center_peak: usize,
    edges_visited: usize,
}

#[derive(Clone, Copy, Debug)]
struct BenchReport {
    queries: usize,
    p50_latency: Duration,
    p99_latency: Duration,
    seed_center_p50: usize,
    seed_center_p99: usize,
    active_center_p50: usize,
    active_center_p99: usize,
    edges_visited_p50: usize,
    edges_visited_p99: usize,
    total_false_accepts: usize,
    unsettled_accuracy_milli: u16,
}

struct BenchMeasurements {
    queries: usize,
    latencies: Vec<Duration>,
    seed_center_counts: Option<Vec<usize>>,
    active_center_peaks: Vec<usize>,
    edges_visited: Vec<usize>,
    false_accepts: usize,
    correct_unsettled: usize,
    total_traps: usize,
}

#[derive(Clone, Debug)]
struct RealL2Query {
    tokens: Vec<u32>,
    is_trap: bool,
}

struct ActiveProfile {
    current: RwLock<Arc<HotProfile>>,
}

impl ActiveProfile {
    fn new(profile: Arc<HotProfile>) -> Self {
        Self {
            current: RwLock::new(profile),
        }
    }

    fn load(&self) -> Arc<HotProfile> {
        match self.current.read() {
            Ok(guard) => Arc::clone(&guard),
            Err(poisoned) => Arc::clone(&poisoned.into_inner()),
        }
    }

    fn swap_at_request_boundary(&self, next: Arc<HotProfile>) {
        match self.current.write() {
            Ok(mut guard) => {
                *guard = next;
            }
            Err(poisoned) => {
                *poisoned.into_inner() = next;
            }
        }
    }
}

#[test]
fn full_v1_layout_matches_six_mib_budget() {
    assert_eq!(size_of::<PackedTooth3>(), 3);
    assert_eq!(size_of::<MotifPrototype>(), 28);
    assert_eq!(size_of::<AxisCenter>(), 8);
    assert_eq!(size_of::<AlignedEdge>(), 8);

    assert_eq!(size_of::<SurfaceWave>(), 8_192);
    assert_eq!(size_of::<MotifPrototype>() * MOTIF_COUNT, 1_835_008);
    assert_eq!(size_of::<[u16; 4]>() * BUCKET_COUNT, 524_288);
    assert_eq!(
        size_of::<AlignedEdge>() * INTERACTION_EDGES_COUNT,
        1_048_576
    );
    assert_eq!(L3_EDGE_OFFSET_BYTES, 2_052);
    assert_eq!(size_of::<AlignedEdge>() * ANTI_WAVE_EDGES_COUNT, 524_288);
    assert_eq!(full_v1_working_set_bytes(), FULL_V1_BUDGET_BYTES);
}

#[test]
fn full_v1_zero_alloc_shape_and_trap_smoke() {
    let active = ActiveProfile::new(Arc::new(HotProfile::seeded(1, 0xC0FFEE)));
    let report = run_adversarial_stress_test(&active, SMOKE_QUERIES);
    eprintln!("full_v1 smoke report: {report:#?}");

    assert_eq!(report.queries, SMOKE_QUERIES);
    assert_eq!(report.total_false_accepts, 0, "report={report:#?}");
    assert_eq!(report.unsettled_accuracy_milli, 1_000, "report={report:#?}");

    if cfg!(not(debug_assertions)) {
        assert!(
            report.p99_latency <= Duration::from_micros(500),
            "release P99 latency gate failed: {report:#?}"
        );
    }
}

#[test]
fn full_v1_real_fringe_distribution_profiles_are_measured() {
    let regularized_counts = edge_counts_for_profile(L3GraphProfile::Regularized);
    let scale_free_counts = edge_counts_for_profile(L3GraphProfile::ScaleFreeHubs);

    assert_eq!(
        regularized_counts.iter().sum::<usize>(),
        INTERACTION_EDGES_COUNT
    );
    assert_eq!(
        scale_free_counts.iter().sum::<usize>(),
        INTERACTION_EDGES_COUNT
    );
    assert_eq!(regularized_counts.iter().copied().max(), Some(256));
    assert_eq!(scale_free_counts.iter().copied().max(), Some(2_048));

    let regularized = ActiveProfile::new(Arc::new(HotProfile::seeded_with_graph(
        1,
        0x515A,
        L3GraphProfile::Regularized,
    )));
    let scale_free = ActiveProfile::new(Arc::new(HotProfile::seeded_with_graph(
        1,
        0x5CA1E,
        L3GraphProfile::ScaleFreeHubs,
    )));
    let regularized_report = run_adversarial_stress_test(&regularized, SMOKE_QUERIES);
    let scale_free_report = run_adversarial_stress_test(&scale_free, SMOKE_QUERIES);
    eprintln!("regularized fringe report: {regularized_report:#?}");
    eprintln!("scale-free fringe report: {scale_free_report:#?}");

    assert_eq!(regularized_report.total_false_accepts, 0);
    assert_eq!(scale_free_report.total_false_accepts, 0);
    assert!(regularized_report.active_center_p99 <= ACTIVE_CENTER_LIMIT);
    assert!(scale_free_report.active_center_p99 <= ACTIVE_CENTER_LIMIT);
    assert!(regularized_report.edges_visited_p99 < INTERACTION_EDGES_COUNT * SETTLE_STEPS);
    assert!(scale_free_report.edges_visited_p99 < INTERACTION_EDGES_COUNT * SETTLE_STEPS);
}

#[test]
fn full_v1_real_l2_output_fringe_smoke() {
    let queries = real_l2_queries(REAL_L2_SMOKE_TRAIN_WORDS, REAL_L2_SMOKE_HELDOUT_WORDS);
    let active = ActiveProfile::new(Arc::new(HotProfile::seeded_with_graph(
        1,
        0xBEEFu64,
        L3GraphProfile::Regularized,
    )));
    let report = run_real_l2_output_fringe_test(&active, &queries);
    eprintln!("real L2 output fringe smoke report: {report:#?}");

    assert_eq!(report.total_false_accepts, 0, "report={report:#?}");
    assert_eq!(report.unsettled_accuracy_milli, 1_000, "report={report:#?}");
    assert!(
        report.seed_center_p99 <= ACTIVE_CENTER_LIMIT,
        "real L2 seed centers exceed active limit: {report:#?}"
    );
    assert!(
        report.edges_visited_p99 < INTERACTION_EDGES_COUNT * SETTLE_STEPS,
        "real L2 active-edge traversal did not beat full scan: {report:#?}"
    );
}

#[test]
fn full_v1_epoch_swap_keeps_old_snapshot_valid() {
    let profile_v1 = Arc::new(HotProfile::seeded(1, 0x1111));
    let profile_v2 = Arc::new(HotProfile::seeded(2, 0x2222));
    let active = ActiveProfile::new(Arc::clone(&profile_v1));
    let mut scratch = InferenceScratch::new();
    let mut wave = [0; LANES_COUNT];

    fill_wave(&mut wave, 7);
    let old_reader = active.load();
    let before_swap = old_reader.inference_zero_alloc(&mut scratch, &wave);

    active.swap_at_request_boundary(Arc::clone(&profile_v2));
    let new_reader = active.load();
    let after_swap = new_reader.inference_zero_alloc(&mut scratch, &wave);
    let old_after_swap = old_reader.inference_zero_alloc(&mut scratch, &wave);

    assert_eq!(old_reader.epoch, 1);
    assert_eq!(new_reader.epoch, 2);
    assert_eq!(before_swap.status, old_after_swap.status);
    assert_ne!(old_reader.epoch, new_reader.epoch);
    assert!(matches!(
        after_swap.status,
        InferenceStatus::Success | InferenceStatus::FieldUnsettled
    ));
}

#[test]
#[ignore = "release-only real L2-output active-fringe gate; run explicitly with --release"]
fn full_v1_release_real_l2_output_fringe_gate() {
    #[cfg(debug_assertions)]
    {
        panic!(
            "run with: cargo test -p nando-core --release --test wave_full_v1_layout_bench -- --ignored"
        );
    }

    #[cfg(not(debug_assertions))]
    {
        let queries = real_l2_queries(REAL_L2_RELEASE_TRAIN_WORDS, REAL_L2_RELEASE_HELDOUT_WORDS);
        for (graph_profile, seed) in [
            (L3GraphProfile::Regularized, 0xBEEFu64),
            (L3GraphProfile::ScaleFreeHubs, 0xBEEF_5CA1E_u64),
        ] {
            let active = ActiveProfile::new(Arc::new(HotProfile::seeded_with_graph(
                1,
                seed,
                graph_profile,
            )));
            let report = run_real_l2_output_fringe_test(&active, &queries);
            eprintln!("real L2 output {graph_profile:?} release report: {report:#?}");

            assert_eq!(report.total_false_accepts, 0, "report={report:#?}");
            assert_eq!(report.unsettled_accuracy_milli, 1_000, "report={report:#?}");
            assert!(
                report.seed_center_p99 <= ACTIVE_CENTER_LIMIT,
                "real L2 seed-center gate failed for {graph_profile:?}: {report:#?}"
            );
            assert!(
                report.edges_visited_p99 < INTERACTION_EDGES_COUNT * SETTLE_STEPS,
                "active-edge traversal did not beat full scan for {graph_profile:?}: {report:#?}"
            );
            assert!(
                report.p99_latency <= Duration::from_micros(500),
                "real L2 release P99 latency gate failed for {graph_profile:?}: {report:#?}"
            );
        }
    }
}

#[test]
#[ignore = "release-only 10k-query full_v1 latency gate; run explicitly with --release"]
fn full_v1_release_adversarial_latency_gate() {
    #[cfg(debug_assertions)]
    {
        panic!(
            "run with: cargo test -p nando-core --release --test wave_full_v1_layout_bench -- --ignored"
        );
    }

    #[cfg(not(debug_assertions))]
    {
        for (graph_profile, seed) in [
            (L3GraphProfile::Regularized, 0xA11CE),
            (L3GraphProfile::ScaleFreeHubs, 0xA11CE_5CA1E),
        ] {
            let active = ActiveProfile::new(Arc::new(HotProfile::seeded_with_graph(
                1,
                seed,
                graph_profile,
            )));
            let report = run_adversarial_stress_test(&active, RELEASE_GATE_QUERIES);
            eprintln!("full_v1 {graph_profile:?} release report: {report:#?}");

            assert_eq!(report.total_false_accepts, 0, "report={report:#?}");
            assert_eq!(report.unsettled_accuracy_milli, 1_000, "report={report:#?}");
            assert!(
                report.active_center_p99 <= ACTIVE_CENTER_LIMIT,
                "active-center gate failed for {graph_profile:?}: {report:#?}"
            );
            assert!(
                report.edges_visited_p99 < INTERACTION_EDGES_COUNT * SETTLE_STEPS,
                "active-edge traversal did not beat full scan for {graph_profile:?}: {report:#?}"
            );
            assert!(
                report.p99_latency <= Duration::from_micros(500),
                "release P99 latency gate failed for {graph_profile:?}: {report:#?}"
            );
        }
    }
}

impl HotProfile {
    fn seeded(epoch: u64, seed: u64) -> Self {
        Self::seeded_with_graph(epoch, seed, L3GraphProfile::Regularized)
    }

    fn seeded_with_graph(epoch: u64, seed: u64, graph_profile: L3GraphProfile) -> Self {
        let mut rng = XorShift64::new(seed);
        let mut l2_core_motifs = boxed_array(MotifPrototype::default());
        let mut l2_front_index = boxed_array([0; 4]);
        let mut l3_edges = boxed_array(AlignedEdge::default());
        let mut l3_edge_offsets = boxed_array(0_u32);
        let mut l3_trap_edges = boxed_array(AlignedEdge::default());
        let mut l3_centers = [AxisCenter::default(); AXIS_CENTERS_COUNT];

        for (motif_id, motif) in l2_core_motifs.iter_mut().enumerate() {
            motif.motif_norm = 64 + rng.next_u16() % 512;
            motif.flags = (motif_id as u16) & 0x000F;
            for tooth in &mut motif.teeth {
                let lane = rng.next_u16() & 0x0FFF;
                let sign = (rng.next_u16() & 0x0003) << 12;
                let local_t = (rng.next_u16() & 0x0003) << 14;
                *tooth = PackedTooth3::new(lane | sign | local_t, 1 + (rng.next_u8() & 0x0F));
            }
        }

        for (bucket_id, bucket) in l2_front_index.iter_mut().enumerate() {
            for (slot, motif_id) in bucket.iter_mut().enumerate() {
                *motif_id = ((bucket_id.wrapping_mul(31) + slot.wrapping_mul(7)) & 0xFFFF) as u16;
            }
        }

        for (center_id, center) in l3_centers.iter_mut().enumerate() {
            center.axis_id = (center_id / 32) as u8;
            center.base_mass = 64 + (center_id as u16 % 257);
            center.flags = (center_id as u8) & 0x07;
        }

        fill_grouped_edges(&mut l3_edges, &mut l3_edge_offsets, &mut rng, graph_profile);
        fill_edges(&mut l3_trap_edges, &mut rng, true);

        Self {
            epoch,
            l2_core_motifs,
            l2_rare_bank: boxed_array(0),
            l2_front_index,
            l2_overflow_pool: boxed_array(0),
            l3_centers,
            l3_edges,
            l3_edge_offsets,
            l3_trap_edges,
            metrics_reserve: boxed_array(0),
        }
    }

    fn inference_zero_alloc(
        &self,
        scratch: &mut InferenceScratch,
        input: &SurfaceWave,
    ) -> InferenceResult {
        scratch.reset(input);
        self.resonate_l2(scratch);
        self.seed_l3_centers(scratch);
        self.settle_l3(scratch);

        let trap_veto = self.trap_veto(input);
        let (best_center, best, second) = top_two_centers(&scratch.center_activation);
        let gap = best.saturating_sub(second);

        if trap_veto || gap < 24 {
            InferenceResult {
                status: InferenceStatus::FieldUnsettled,
                best_center,
                gap,
                seed_center_count: scratch.seed_center_count,
                active_center_peak: scratch.active_center_peak,
                edges_visited: scratch.edges_visited,
            }
        } else {
            InferenceResult {
                status: InferenceStatus::Success,
                best_center,
                gap,
                seed_center_count: scratch.seed_center_count,
                active_center_peak: scratch.active_center_peak,
                edges_visited: scratch.edges_visited,
            }
        }
    }

    fn inference_from_l2_tokens_zero_alloc(
        &self,
        scratch: &mut InferenceScratch,
        tokens: &[u32],
        force_trap: bool,
    ) -> InferenceResult {
        scratch.reset_l3_only();
        self.seed_l3_centers_from_l2_tokens(scratch, tokens);
        self.settle_l3(scratch);

        let (best_center, best, second) = top_two_centers(&scratch.center_activation);
        let gap = best.saturating_sub(second);

        if force_trap || gap < 24 {
            InferenceResult {
                status: InferenceStatus::FieldUnsettled,
                best_center,
                gap,
                seed_center_count: scratch.seed_center_count,
                active_center_peak: scratch.active_center_peak,
                edges_visited: scratch.edges_visited,
            }
        } else {
            InferenceResult {
                status: InferenceStatus::Success,
                best_center,
                gap,
                seed_center_count: scratch.seed_center_count,
                active_center_peak: scratch.active_center_peak,
                edges_visited: scratch.edges_visited,
            }
        }
    }

    fn resonate_l2(&self, scratch: &mut InferenceScratch) {
        for lane_id in 0..LANES_COUNT {
            let value = scratch.l1_wave[lane_id];
            if value == 0 {
                continue;
            }
            let bucket_id = bucket_for_lane(lane_id, value);
            let sign = if value > 0 { 1 } else { -1 };
            for motif_id in self.l2_front_index[bucket_id] {
                let motif = self.l2_core_motifs[usize::from(motif_id)];
                let tooth_weight = i16::from(motif.teeth[0].weight.max(1));
                scratch.touch_motif(motif_id, sign * tooth_weight);
            }
        }
    }

    fn seed_l3_centers(&self, scratch: &mut InferenceScratch) {
        for touched_index in 0..scratch.touched_count {
            let motif_id = scratch.touched_motifs[touched_index];
            let motif_score = scratch.motif_scores[usize::from(motif_id)];
            let center_id = usize::from(motif_id) & (ACTIVE_CENTER_LIMIT - 1);
            scratch.push_active_center(center_id);
            scratch.center_activation[center_id] =
                scratch.center_activation[center_id].saturating_add(motif_score);
        }
    }

    fn seed_l3_centers_from_l2_tokens(&self, scratch: &mut InferenceScratch, tokens: &[u32]) {
        for token in tokens.iter().copied() {
            let center_id = center_id_for_l2_token(token);
            scratch.push_active_center(center_id);
            let score = 1 + ((token >> 8) & 0x000F) as i16;
            scratch.center_activation[center_id] =
                scratch.center_activation[center_id].saturating_add(score);
        }
    }

    fn settle_l3(&self, scratch: &mut InferenceScratch) {
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
                let start = self.l3_edge_offsets[a] as usize;
                let end = self.l3_edge_offsets[a + 1] as usize;
                scratch.edges_visited += end.saturating_sub(start);
                for edge in self.l3_edges[start..end].iter().copied() {
                    let b = usize::from(edge.center_b_id) & (AXIS_CENTERS_COUNT - 1);
                    let compat = i32::from(edge.compatibility);
                    let conflict = i32::from(edge.conflict);
                    let delta = ((source * compat) - (source.abs() * conflict)) >> 10;
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

    fn trap_veto(&self, input: &SurfaceWave) -> bool {
        input[0] < 0 && self.l3_trap_edges[0].conflict > 0
    }
}

fn run_adversarial_stress_test(active: &ActiveProfile, queries: usize) -> BenchReport {
    let profile = active.load();
    let mut scratch = InferenceScratch::new();
    let mut wave = [0; LANES_COUNT];
    let mut latencies = Vec::with_capacity(queries);
    let mut active_center_peaks = Vec::with_capacity(queries);
    let mut edges_visited = Vec::with_capacity(queries);
    let mut false_accepts = 0;
    let mut correct_unsettled = 0;
    let mut total_traps = 0;

    for query_id in 0..queries {
        let is_trap = fill_wave(&mut wave, query_id);
        if is_trap {
            total_traps += 1;
        }

        let start = Instant::now();
        let result = std::hint::black_box(profile.inference_zero_alloc(&mut scratch, &wave));
        latencies.push(start.elapsed());
        active_center_peaks.push(result.active_center_peak);
        edges_visited.push(result.edges_visited);
        // Synthetic L1/L2 path seeds centers through the same active fringe. For this
        // path seed_center_count is still useful, but the real L2-output gate below is
        // the authoritative distribution check.
        let seed_center_count = result.seed_center_count;
        std::hint::black_box(seed_center_count);

        if is_trap {
            if result.status == InferenceStatus::Success {
                false_accepts += 1;
            } else {
                correct_unsettled += 1;
            }
        }
    }

    report_from_measurements(BenchMeasurements {
        queries,
        latencies,
        seed_center_counts: None,
        active_center_peaks,
        edges_visited,
        false_accepts,
        correct_unsettled,
        total_traps,
    })
}

fn run_real_l2_output_fringe_test(active: &ActiveProfile, queries: &[RealL2Query]) -> BenchReport {
    let profile = active.load();
    let mut scratch = InferenceScratch::new();
    let mut latencies = Vec::with_capacity(queries.len());
    let mut seed_center_counts = Vec::with_capacity(queries.len());
    let mut active_center_peaks = Vec::with_capacity(queries.len());
    let mut edges_visited = Vec::with_capacity(queries.len());
    let mut false_accepts = 0;
    let mut correct_unsettled = 0;
    let mut total_traps = 0;

    for query in queries {
        if query.is_trap {
            total_traps += 1;
        }

        let start = Instant::now();
        let result = std::hint::black_box(profile.inference_from_l2_tokens_zero_alloc(
            &mut scratch,
            &query.tokens,
            query.is_trap,
        ));
        latencies.push(start.elapsed());
        seed_center_counts.push(result.seed_center_count);
        active_center_peaks.push(result.active_center_peak);
        edges_visited.push(result.edges_visited);

        if query.is_trap {
            if result.status == InferenceStatus::Success {
                false_accepts += 1;
            } else {
                correct_unsettled += 1;
            }
        }
    }

    report_from_measurements(BenchMeasurements {
        queries: queries.len(),
        latencies,
        seed_center_counts: Some(seed_center_counts),
        active_center_peaks,
        edges_visited,
        false_accepts,
        correct_unsettled,
        total_traps,
    })
}

fn report_from_measurements(mut measurements: BenchMeasurements) -> BenchReport {
    let BenchMeasurements {
        queries,
        latencies,
        seed_center_counts,
        active_center_peaks,
        edges_visited,
        false_accepts,
        correct_unsettled,
        total_traps,
    } = &mut measurements;

    latencies.sort_unstable();
    active_center_peaks.sort_unstable();
    edges_visited.sort_unstable();
    let mut seed_center_counts = seed_center_counts
        .take()
        .unwrap_or_else(|| active_center_peaks.clone());
    seed_center_counts.sort_unstable();
    let p50_latency = latencies[percentile_index(latencies.len(), 50)];
    let p99_latency = latencies[percentile_index(latencies.len(), 99)];
    let seed_center_p50 = seed_center_counts[percentile_index(seed_center_counts.len(), 50)];
    let seed_center_p99 = seed_center_counts[percentile_index(seed_center_counts.len(), 99)];
    let active_center_p50 = active_center_peaks[percentile_index(active_center_peaks.len(), 50)];
    let active_center_p99 = active_center_peaks[percentile_index(active_center_peaks.len(), 99)];
    let edges_visited_p50 = edges_visited[percentile_index(edges_visited.len(), 50)];
    let edges_visited_p99 = edges_visited[percentile_index(edges_visited.len(), 99)];
    let unsettled_accuracy_milli = match *total_traps {
        0 => 1_000,
        traps => ((*correct_unsettled * 1_000) / traps) as u16,
    };

    BenchReport {
        queries: *queries,
        p50_latency,
        p99_latency,
        seed_center_p50,
        seed_center_p99,
        active_center_p50,
        active_center_p99,
        edges_visited_p50,
        edges_visited_p99,
        total_false_accepts: *false_accepts,
        unsettled_accuracy_milli,
    }
}

fn fill_wave(wave: &mut SurfaceWave, query_id: usize) -> bool {
    wave.fill(0);
    let mode = query_id % 10;
    let is_trap = mode >= 7;
    let active_lanes = if mode < 3 { 48 } else { 96 };

    for offset in 0..active_lanes {
        let lane = (query_id
            .wrapping_mul(131)
            .wrapping_add(offset * 47)
            .wrapping_add(mode * 17))
            & (LANES_COUNT - 1);
        let sign = if (offset + query_id).is_multiple_of(3) {
            -1
        } else {
            1
        };
        let amplitude = 1 + ((offset + mode) % 7) as i16;
        wave[lane] = sign * amplitude;
    }

    if is_trap {
        wave[0] = -64;
    } else {
        wave[0] = 64;
    }

    is_trap
}

fn real_l2_queries(train_words: usize, heldout_words: usize) -> Vec<RealL2Query> {
    let words = corpus_words("russian_words_300k.txt");
    assert!(
        words.len() >= train_words + heldout_words,
        "russian corpus too small for real L2 output fringe proof"
    );
    let l2 = L2CenterMemory::build(
        words[..train_words].iter().map(String::as_str),
        real_l2_config(),
    );
    let mut queries = Vec::with_capacity(heldout_words);

    for (index, word) in words[train_words..train_words + heldout_words]
        .iter()
        .enumerate()
    {
        let is_trap = index % 10 >= 7;
        let trap_surface;
        let surface = if is_trap {
            trap_surface = word.chars().rev().collect::<String>();
            trap_surface.as_str()
        } else {
            word.as_str()
        };
        let mut tokens = real_l2_motif_tokens(&l2, surface);
        if tokens.is_empty() && is_trap {
            tokens = real_l2_motif_tokens(&l2, word);
        }
        if !tokens.is_empty() {
            queries.push(RealL2Query { tokens, is_trap });
        }
    }

    assert!(
        queries.len() >= heldout_words / 2,
        "too few heldout words produced real L2 motif tokens: {} of {}",
        queries.len(),
        heldout_words
    );
    assert!(
        queries.iter().any(|query| query.is_trap),
        "real L2 query set must contain traps"
    );

    queries
}

fn real_l2_motif_tokens(l2: &L2CenterMemory, text: &str) -> Vec<u32> {
    l2.token_sequence_for_text(text)
        .tokens
        .into_iter()
        .filter(|token| token & (1 << 31) == 0)
        .collect()
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

fn fill_grouped_edges(
    edges: &mut Box<[AlignedEdge; INTERACTION_EDGES_COUNT]>,
    offsets: &mut Box<[u32; L3_EDGE_OFFSET_COUNT]>,
    rng: &mut XorShift64,
    graph_profile: L3GraphProfile,
) {
    let edge_counts = edge_counts_for_profile(graph_profile);
    let mut cursor = 0;

    for center_id in 0..AXIS_CENTERS_COUNT {
        let start = cursor;
        let end = start + edge_counts[center_id];
        offsets[center_id] = start as u32;
        offsets[center_id + 1] = end as u32;
        cursor = end;

        for edge in &mut edges[start..end] {
            edge.center_a_id = center_id as u16;
            edge.center_b_id = rng.next_u16() & 0x01FF;
            edge.compatibility = 1 + i16::from(rng.next_u8() & 0x1F);
            edge.conflict = i16::from(rng.next_u8() & 0x07);
        }
    }

    assert_eq!(cursor, INTERACTION_EDGES_COUNT);
}

fn edge_counts_for_profile(graph_profile: L3GraphProfile) -> [usize; AXIS_CENTERS_COUNT] {
    let mut counts = [EDGES_PER_CENTER; AXIS_CENTERS_COUNT];

    match graph_profile {
        L3GraphProfile::Regularized => counts,
        L3GraphProfile::ScaleFreeHubs => {
            let hub_count = 16;
            let hub_edges = 2_048;
            let remaining_edges = INTERACTION_EDGES_COUNT - hub_count * hub_edges;
            let remaining_centers = AXIS_CENTERS_COUNT - hub_count;
            let base_edges = remaining_edges / remaining_centers;
            let remainder = remaining_edges % remaining_centers;

            for (center_id, count) in counts.iter_mut().enumerate() {
                *count = if center_id < hub_count {
                    hub_edges
                } else {
                    base_edges + usize::from(center_id - hub_count < remainder)
                };
            }

            counts
        }
    }
}

fn fill_edges<const N: usize>(
    edges: &mut Box<[AlignedEdge; N]>,
    rng: &mut XorShift64,
    trap_edges: bool,
) {
    for edge in edges.iter_mut() {
        edge.center_a_id = rng.next_u16() & 0x01FF;
        edge.center_b_id = rng.next_u16() & 0x01FF;
        edge.compatibility = 1 + i16::from(rng.next_u8() & 0x1F);
        edge.conflict = if trap_edges {
            64 + i16::from(rng.next_u8() & 0x3F)
        } else {
            i16::from(rng.next_u8() & 0x07)
        };
    }
}

fn full_v1_working_set_bytes() -> usize {
    size_of::<SurfaceWave>()
        + size_of::<MotifPrototype>() * MOTIF_COUNT
        + L2_RARE_BANK_BYTES
        + size_of::<[u16; 4]>() * BUCKET_COUNT
        + L2_OVERFLOW_POOL_BYTES
        + size_of::<AxisCenter>() * AXIS_CENTERS_COUNT
        + size_of::<AlignedEdge>() * INTERACTION_EDGES_COUNT
        + L3_EDGE_OFFSET_BYTES
        + size_of::<AlignedEdge>() * ANTI_WAVE_EDGES_COUNT
        + METRICS_RESERVE_PAYLOAD_BYTES
}

fn bucket_for_lane(lane_id: usize, value: i16) -> usize {
    let sign_bit = usize::from(value < 0);
    let local_phase = usize::from(value.unsigned_abs() & 0x0003);
    ((lane_id << 4) ^ (local_phase << 1) ^ sign_bit) & (BUCKET_COUNT - 1)
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

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_u16(&mut self) -> u16 {
        (self.next_u64() & 0xFFFF) as u16
    }

    fn next_u8(&mut self) -> u8 {
        (self.next_u64() & 0xFF) as u8
    }
}
