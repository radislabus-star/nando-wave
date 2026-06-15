//! Core types for the Nando Wave research runtime.
//!
//! Stage 2 introduces fixed-size wave state packets and one deterministic tick.
//! It intentionally contains no training, text generation, or chatbot logic.

pub mod wave;

pub use wave::{
    BytePhaseLut, CELL32_BYTES, CacheAwareOrganPlan, CacheProfile, CarrierWave, Cell32,
    Cell32Learner, Cell32PromotionReport, CellRank, HotWindowPlan, LinkProfile, LinkTissue,
    LiveByteLearner, LiveBytePrediction, LiveByteTrainReport, LiveByteTrainStep, LiveCycle,
    LivePrediction, LocalUpdateReport, Mono192, Organ128Plan, OrganState, PHASE_SLOTS,
    PLANNED_ORGAN128_BYTES, PLANNED_ORGAN128_CELLS, PLANNED_ORGAN192_BYTES, SNAPSHOT_TOP_SLOTS,
    SNAPSHOT_V1_BYTES, STAGE2_ORGAN_CELLS, STAGE2_TOP_K, SnapshotParseError, SpectrumSnapshot,
    Stage2Organ, Stage2Tick, Stage2TraceTick, TickTrace, WaveBus, encode_byte_phases,
    run_stage2_tick, run_stage2_tick_with_carrier, run_stage2_tick_with_disabled,
    run_stage2_tick_with_organ_carrier, run_stage2_tick_with_organ_lut_carrier,
    run_stage2_trace_tick, run_stage2_trace_with_organ_carrier,
    run_stage2_trace_with_organ_lut_carrier, stage2_organ,
};

/// Current implementation stage from `docs/DETAILED_ROADMAP.md`.
pub const CURRENT_STAGE: &str = "stage-2-fixed-wave-tick";

/// Human-readable scope boundary for the current stage.
pub const CURRENT_SCOPE: &str = "fixed wave structures, optimized tick, eval-gated Cell32 learner, LinkTissue, and topology compare";

/// High-level project metadata used by the CLI and future reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectStatus {
    pub name: &'static str,
    pub stage: &'static str,
    pub scope: &'static str,
    pub rust_first: bool,
    pub cell32_bytes: usize,
    pub planned_organ128_bytes: usize,
    pub planned_organ192_bytes: usize,
}

/// Return the current project status without touching any runtime state.
#[must_use]
pub const fn project_status() -> ProjectStatus {
    ProjectStatus {
        name: "Nando Wave",
        stage: CURRENT_STAGE,
        scope: CURRENT_SCOPE,
        rust_first: true,
        cell32_bytes: CELL32_BYTES,
        planned_organ128_bytes: PLANNED_ORGAN128_BYTES,
        planned_organ192_bytes: PLANNED_ORGAN192_BYTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn planned_sizes_match_the_roadmap() {
        assert_eq!(CELL32_BYTES, 32_768);
        assert_eq!(PLANNED_ORGAN128_CELLS, 128);
        assert_eq!(PLANNED_ORGAN128_BYTES, 4 * 1024 * 1024);
        assert_eq!(PLANNED_ORGAN192_BYTES, 196_608);
        assert_eq!(size_of::<Cell32>(), CELL32_BYTES);
        assert_eq!(
            size_of::<[Cell32; STAGE2_ORGAN_CELLS]>(),
            PLANNED_ORGAN192_BYTES
        );
        assert_eq!(size_of::<Mono192>(), PLANNED_ORGAN192_BYTES);
    }

    #[test]
    fn status_is_rust_first() {
        let status = project_status();
        assert_eq!(status.name, "Nando Wave");
        assert_eq!(status.stage, CURRENT_STAGE);
        assert_eq!(status.scope, CURRENT_SCOPE);
        assert!(status.rust_first);
    }
}
