/// The first hot-cell atom. It is sized to fit the T480 L1 data-cache target.
pub const CELL32_BYTES: usize = 32 * 1024;

/// Number of harmonic slots used by Stage 2 for one deterministic tick.
pub const PHASE_SLOTS: usize = 256;

/// The first organism size used for the fair `6 x Cell32` vs `Mono192` control.
pub const STAGE2_ORGAN_CELLS: usize = 6;

/// Active cells copied into the bus on the first deterministic tick.
pub const STAGE2_TOP_K: usize = 3;

/// Number of dominant slots persisted in the Stage 2 snapshot.
pub const SNAPSHOT_TOP_SLOTS: usize = 8;

/// Stable Stage 2 snapshot byte length.
pub const SNAPSHOT_V1_BYTES: usize = 148;

/// The first fair monolith/organ control size.
pub const PLANNED_ORGAN192_BYTES: usize = STAGE2_ORGAN_CELLS * CELL32_BYTES;

/// First L3-sized warm organism plan: 128 x Cell32 = 4 MiB.
pub const PLANNED_ORGAN128_CELLS: usize = 128;
pub const PLANNED_ORGAN128_BYTES: usize = PLANNED_ORGAN128_CELLS * CELL32_BYTES;

const CELL32_HEADER_BYTES: usize = 16;
const CELL32_ARRAYS: usize = 6;
const CELL32_ARRAY_BYTES: usize = CELL32_ARRAYS * PHASE_SLOTS * std::mem::size_of::<f32>();
const CELL32_RESERVED_BYTES: usize = CELL32_BYTES - CELL32_HEADER_BYTES - CELL32_ARRAY_BYTES;

const MONO_PHASE_SLOTS: usize = STAGE2_ORGAN_CELLS * PHASE_SLOTS;
const MONO_HEADER_BYTES: usize = 16;
const MONO_ARRAYS: usize = 3;
const MONO_ARRAY_BYTES: usize = MONO_ARRAYS * MONO_PHASE_SLOTS * std::mem::size_of::<f32>();
const MONO_RESERVED_BYTES: usize = PLANNED_ORGAN192_BYTES - MONO_HEADER_BYTES - MONO_ARRAY_BYTES;

mod bus;
mod cache_plan;
mod carrier;
mod cell;
mod learn;
mod math;
mod organ;
mod snapshot;
mod tick;

pub use bus::WaveBus;
pub use cache_plan::{CacheAwareOrganPlan, CacheProfile, HotWindowPlan, Organ128Plan};
pub use carrier::CarrierWave;
pub use cell::{Cell32, CellRank, Mono192};
pub use learn::{
    Cell32Learner, Cell32PromotionReport, LinkProfile, LinkTissue, LiveByteLearner,
    LiveBytePrediction, LiveByteTrainReport, LiveByteTrainStep,
};
pub use organ::{
    LiveCycle, LivePrediction, LocalUpdateReport, OrganState, Stage2Organ, stage2_organ,
};
pub use snapshot::{SnapshotParseError, SpectrumSnapshot, Stage2Tick, TickTrace};
pub use tick::{
    BytePhaseLut, Stage2TraceTick, encode_byte_phases, run_stage2_tick,
    run_stage2_tick_with_carrier, run_stage2_tick_with_disabled,
    run_stage2_tick_with_organ_carrier, run_stage2_tick_with_organ_lut_carrier,
    run_stage2_trace_tick, run_stage2_trace_with_organ_carrier,
    run_stage2_trace_with_organ_lut_carrier,
};

pub(crate) use math::{
    circular_phase_delta, insert_top_index, insert_top_slot, normalized_entropy, unit_noise,
};
pub(crate) use tick::{run_stage2_tick_with_organ_lut_state, run_stage2_tick_with_state};

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn fixed_packets_have_expected_sizes() {
        assert_eq!(size_of::<Cell32>(), CELL32_BYTES);
        assert_eq!(PLANNED_ORGAN128_BYTES, 4 * 1024 * 1024);
        assert_eq!(
            size_of::<[Cell32; STAGE2_ORGAN_CELLS]>(),
            PLANNED_ORGAN192_BYTES
        );
        assert_eq!(size_of::<Mono192>(), PLANNED_ORGAN192_BYTES);
    }

    #[test]
    fn stage2_tick_is_deterministic() {
        let first = run_stage2_tick(7, 42);
        let second = run_stage2_tick(7, 42);
        assert_eq!(first, second);
    }

    #[test]
    fn precomputed_organ_tick_matches_seed_tick() {
        let seed = 7;
        let input = 42;
        let carrier = CarrierWave::from_seed(seed, input);
        let organ = Stage2Organ::new(seed);

        let seed_tick = run_stage2_tick_with_carrier(seed, input, carrier, None);
        let organ_tick = run_stage2_tick_with_organ_carrier(&organ, input, carrier, None);

        assert_eq!(organ_tick, seed_tick);
    }

    #[test]
    fn trace_only_tick_matches_full_tick_trace() {
        let seed = 11;
        let input = b'N';
        let carrier = CarrierWave::from_seed(seed, input);
        let organ = Stage2Organ::new(seed);

        let full = run_stage2_tick_with_organ_carrier(&organ, input, carrier, None);
        let trace_only = run_stage2_trace_with_organ_carrier(&organ, input, carrier, None);

        assert_eq!(trace_only.carrier, full.carrier);
        assert_eq!(trace_only.trace, full.trace);
    }

    #[test]
    fn byte_phase_lut_matches_direct_encoding() {
        let lut = BytePhaseLut::new();
        assert_eq!(lut.phases(0), &encode_byte_phases(0));
        assert_eq!(lut.phases(42), &encode_byte_phases(42));
        assert_eq!(lut.phases(u8::MAX), &encode_byte_phases(u8::MAX));
    }

    #[test]
    fn lut_trace_tick_matches_direct_trace_tick() {
        let seed = 11;
        let input = b'N';
        let carrier = CarrierWave::from_seed(seed, input);
        let organ = Stage2Organ::new(seed);
        let lut = BytePhaseLut::new();

        let direct = run_stage2_trace_with_organ_carrier(&organ, input, carrier, None);
        let lut_tick = run_stage2_trace_with_organ_lut_carrier(&organ, &lut, input, carrier, None);

        assert_eq!(lut_tick, direct);
    }

    #[test]
    fn stage2_tick_produces_snapshot() {
        let tick = run_stage2_tick(11, b'N');
        assert_eq!(tick.trace.cells_scanned, STAGE2_ORGAN_CELLS);
        assert_eq!(tick.trace.active_count, STAGE2_TOP_K);
        assert_eq!(tick.snapshot.version, 1);
        assert_eq!(tick.snapshot.input_byte, b'N');
        assert!(tick.trace.coherence >= 0.0);
        assert!(tick.trace.coherence <= 1.0);
        assert!(tick.trace.spectral_entropy >= 0.0);
        assert!(tick.trace.spectral_entropy <= 1.0);
    }

    #[test]
    fn organ_state_settle_ticks_update_runtime_coupling() {
        let mut state = OrganState::new(13, b'l');
        let first = state.settle_tick(b'e', None);
        let second = state.settle_tick(b't', None);

        assert_eq!(state.tick_index, 2);
        assert_eq!(first.trace.cells_scanned, STAGE2_ORGAN_CELLS);
        assert_eq!(second.trace.active_count, STAGE2_TOP_K);
        assert!(state.previous_coherence >= 0.0);
        assert!(state.previous_coherence <= 1.0);
        assert!(state.cell_coupling.iter().any(|value| *value > 0.0));
        assert_eq!(second.snapshot.to_bytes().len(), SNAPSHOT_V1_BYTES);
    }

    #[test]
    fn live_cycle_applies_local_feedback() {
        let seed = 13;
        let organ = Stage2Organ::new(seed);
        let lut = BytePhaseLut::new();
        let mut state = OrganState::new(seed, b'l');

        let cycle = state.live_cycle(&organ, &lut, b'l', b'e');

        assert_eq!(state.tick_index, 1);
        assert_eq!(cycle.update.target_byte, b'e');
        assert!(cycle.prediction.confidence >= 0.0);
        assert!(cycle.prediction.confidence <= 1.0);
        assert!(cycle.update.reward >= -1.0);
        assert!(cycle.update.reward <= 1.0);
        assert_eq!(cycle.tick.snapshot.to_bytes().len(), SNAPSHOT_V1_BYTES);
        assert!(state.cell_coupling.iter().any(|value| value.abs() > 0.0));
    }

    #[test]
    fn live_byte_learner_updates_trainable_state() {
        let seed = 13;
        let organ = Stage2Organ::new(seed);
        let lut = BytePhaseLut::new();
        let mut state = OrganState::new(seed, b'a');
        let mut learner = LiveByteLearner::default();
        let mut steps = Vec::new();

        for pair in "a файл a файл".as_bytes().windows(2) {
            let cycle = state.live_cycle(&organ, &lut, pair[0], pair[1]);
            steps.push(learner.update(&cycle.tick.trace, pair[1]));
        }

        let report = LiveByteTrainReport::from_steps(&steps, &learner);
        assert_eq!(report.cases, "a файл a файл".as_bytes().len() - 1);
        assert!(report.bias_abs_mean > 0.0);
        assert!(report.class_weight_abs_mean > 0.0);
        assert!(report.mode_weight_abs_mean > 0.0);
        assert!(report.transition_weight_abs_mean > 0.0);
        assert!(report.weight_abs_mean > 0.0);
        assert!(report.context_weight_abs_mean > 0.0);
        assert!((0.0..=1.0).contains(&report.mean_confidence));
    }

    #[test]
    fn cell32_learner_keeps_fixed_cells_stable() {
        let seed = 13;
        let organ = Stage2Organ::new(seed);
        let original_cells = organ.cells().clone();
        let lut = BytePhaseLut::new();
        let mut state = OrganState::new(seed, b'a');
        let mut learner = Cell32Learner::new(3, 0.08);

        for pair in "abababab".as_bytes().windows(2) {
            let cycle = state.live_cycle(&organ, &lut, pair[0], pair[1]);
            let step = learner.update(&cycle.tick.trace, pair[1]);
            assert!((0.0..=1.0).contains(&step.prediction.confidence));
        }

        assert!(learner.state_abs_mean() > 0.0);
        assert_eq!(organ.cells()[0].age_ticks, original_cells[0].age_ticks);
        assert_eq!(
            organ.cells()[0].last_resonance,
            original_cells[0].last_resonance
        );
    }

    #[test]
    fn cell32_promotion_requires_holdout_gain() {
        let accepted = Cell32PromotionReport::new(10, 8, 0.25, 0.50, 0.25);
        let rejected_no_gain = Cell32PromotionReport::new(10, 8, 0.50, 0.25, 0.25);
        let rejected_oos = Cell32PromotionReport::new(10, 8, 0.25, 0.50, 0.75);

        assert!(accepted.accepted);
        assert!(!rejected_no_gain.accepted);
        assert!(!rejected_oos.accepted);
    }

    #[test]
    fn snapshot_roundtrip_is_stable() {
        let tick = run_stage2_tick(7, 42);
        let bytes = tick.snapshot.to_bytes();
        assert_eq!(bytes.len(), SNAPSHOT_V1_BYTES);
        assert_eq!(&bytes[0..4], b"NWV1");
        let parsed = SpectrumSnapshot::from_bytes(&bytes).expect("snapshot parses");
        assert_eq!(parsed, tick.snapshot);
    }

    #[test]
    fn snapshot_rejects_bad_magic() {
        let mut bytes = run_stage2_tick(7, 42).snapshot.to_bytes();
        bytes[0] = b'X';
        let error = SpectrumSnapshot::from_bytes(&bytes).expect_err("bad magic should fail");
        assert_eq!(error, SnapshotParseError::BadMagic);
    }
}
