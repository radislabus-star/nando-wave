//! Core types for the Nando Wave research runtime.
//!
//! Stage 2 introduces fixed-size wave state packets and one deterministic tick.
//! It intentionally contains no training, text generation, or chatbot logic.

pub mod wave;

pub use wave::{
    BytePhaseLut, CELL32_BYTES, CacheAwareOrganPlan, CacheProfile, CalibrationStats, CarrierWave,
    Cell32, Cell32Learner, Cell32PromotionReport, CellRank, HotWindowPlan, Interference8,
    L1_CENTER_RECORD_BYTES, L1_FOURIER_BINS, L1_RESIDUAL_NGRAM_BYTES, L1_SEQUENCE_REF_BYTES,
    L1_WORD_RECORD_BYTES, L1CenterMemory, L1CenterMemoryConfig, L1CenterMemoryProof,
    L1CenterMemoryVerdict, L1CenterSequence, L1SurfaceCenter, L1WordAssignment, L1WordCenterRecord,
    L2_CENTER_RECORD_BYTES, L2_FOURIER_BINS, L2_RESIDUAL_REF_BYTES, L2_TOKEN_REF_BYTES,
    L2_WORD_RECORD_BYTES, L2CenterMemory, L2CenterMemoryConfig, L2CenterMemoryProof,
    L2CenterMemoryVerdict, L2SequenceCenter, L2TokenSequence, L2WordAssignment, L2WordRecord,
    L3_8MB_SYMBOL_ACTIVE_BYTES, L3_8MB_SYMBOL_CELL8_CELLS, L3_8MB_SYMBOL_WAVE_CLUSTERS,
    L3_FRAME_CENTER_BYTES, L3_FRAME_FEATURE_BYTES, L3FrameCenter, L3FrameSelection,
    L3SemanticExample, L3SemanticGrokkingConfig, L3SemanticGrokkingMemory, L3SemanticGrokkingProof,
    L3SemanticGrokkingVerdict, LinkProfile, LinkTissue, LiveByteLearner, LiveBytePrediction,
    LiveByteTrainReport, LiveByteTrainStep, LiveCycle, LivePrediction, LocalUpdateReport,
    MORPHOLOGY_ATOM_BYTES, Mode8, Mono192, MorphologyAtom, MorphologyExtraction,
    MorphologyGrokkingProof, MorphologyGrokkingVerdict, MorphologyScalingReport,
    MorphologyScalingRow, MorphologyWaveBank, MorphologyWaveConfig, Organ128Plan, OrganState,
    PHASE_SLOTS, PLANNED_ORGAN128_BYTES, PLANNED_ORGAN128_CELLS, PLANNED_ORGAN192_BYTES,
    PackedInterference32, PackedMode32, PackedTransition32, PeakOutcome, ProjectionEntry16,
    SEMANTIC_OPERATOR_BYTES, SEMANTIC_WAVE_BYTES, SEMANTIC_WAVE_DIM, SNAPSHOT_TOP_SLOTS,
    SNAPSHOT_V1_BYTES, STAGE2_ORGAN_CELLS, STAGE2_TOP_K, SURFACE_FOURIER_BINS, SURFACE_WAVE_BYTES,
    SURFACE_WAVE_DIM, SURFACE_WAVE_LM_BIAS_BYTES, SURFACE_WAVE_LM_OUTPUTS,
    SURFACE_WAVE_LM_POSITION_BUCKETS, SURFACE_WAVE_LM_POSITION_BYTES,
    SURFACE_WAVE_LM_POSITION_SCORE_WEIGHT, SURFACE_WAVE_LM_STATE_MAX, SURFACE_WAVE_LM_STATE_MIN,
    SURFACE_WAVE_LM_WEIGHT_BYTES, SURFACE_WAVE_NGRAM, SURFACE_WAVE_TRITS,
    SYMBOL_CELL_DENSE2K_BYTES, SYMBOL_CELL_DENSE2K_INTERFERENCE_SLOTS, SYMBOL_CELL_DENSE2K_MODES,
    SYMBOL_CELL_DENSE2K_TRANSITIONS, SYMBOL_CELL8_BYTES, SYMBOL_CELL8_INTERFERENCE_SLOTS,
    SYMBOL_CELL8_MODES, SYMBOL_CELL8_PROJECTION_LANES, SYMBOL_CELL8_TRANSITIONS,
    SYMBOL_CLIQUE_CLASS_BYTES, SYMBOL_DENSE2K_CELLS_PER_2MB, SYMBOL_L3_DEFAULT_ACTIVE_BYTES,
    SYMBOL_L3_DEFAULT_CELL8_CELLS, SYMBOL_L3_DEFAULT_WAVE_CLUSTERS, SYMBOL_L3_TURBO_ACTIVE_BYTES,
    SYMBOL_L3_TURBO_CELL8_CELLS, SYMBOL_L3_TURBO_WAVE_CLUSTERS, SYMBOL_WAVE_CLUSTER_CELLS,
    SemanticAtom, SemanticAtomExtractor, SemanticCandidate, SemanticEquationForm,
    SemanticEquationPrediction, SemanticExtractedForm, SemanticExtraction,
    SemanticExtractionStatus, SemanticFact, SemanticPrediction, SemanticQuery,
    SemanticRelationOperator, SemanticSchemaKey, SemanticWave4096, SemanticWaveEvalReport,
    SemanticWaveGrokkingProof, SemanticWaveGrokkingVerdict, SemanticWaveMemory, SnapshotParseError,
    SpectrumSnapshot, StablePeakScore, Stage2BusTraceTick, Stage2Organ, Stage2Tick,
    Stage2TraceTick, SurfaceFourierSignature, SurfaceMotif, SurfaceMotifBank, SurfaceMotifRef,
    SurfaceMotifSpec, SurfaceResidualRecord, SurfaceWave4096, SurfaceWaveCenter,
    SurfaceWaveContext4096, SurfaceWaveGeneration, SurfaceWaveGenerationCase,
    SurfaceWaveGenerationProof, SurfaceWaveLane, SurfaceWaveLm, SurfaceWaveLmConfig,
    SurfaceWaveLmEvalReport, SurfaceWaveLmTrainReport, SurfaceWavePatternProof,
    SurfaceWavePatternProofConfig, SurfaceWavePatternVerdict, SurfaceWaveTextScore,
    SurfaceWaveTrit, SurfaceWordGrokkingConfig, SurfaceWordGrokkingProof,
    SurfaceWordGrokkingVerdict, SymbolCell8, SymbolCell8Advice, SymbolCell8Calibration,
    SymbolCell8Header, SymbolCell8Tick, SymbolCell32, SymbolCellDense2K, SymbolCliqueClass,
    SymbolClusterCenter, SymbolClusterTick, SymbolExcitation, SymbolHeader, SymbolL3Center,
    SymbolL3Organism, SymbolL3Tick, SymbolProjection, SymbolWaveCluster, TickTrace, Transition8,
    WaveBus, WavePatternCompileReport, WavePatternCompiler, WavePatternSelection,
    WavePatternTemplate, encode_byte_phases, run_stage2_bus_trace_with_organ_carrier,
    run_stage2_tick, run_stage2_tick_with_carrier, run_stage2_tick_with_disabled,
    run_stage2_tick_with_organ_carrier, run_stage2_tick_with_organ_lut_carrier,
    run_stage2_trace_tick, run_stage2_trace_with_organ_carrier,
    run_stage2_trace_with_organ_lut_carrier, semantic_label_slot, stage2_organ,
    surface_ngram_count, surface_ngram_projection,
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
        assert_eq!(size_of::<SymbolCell8>(), SYMBOL_CELL8_BYTES);
        assert_eq!(SYMBOL_L3_TURBO_CELL8_CELLS, 256);
        assert_eq!(SYMBOL_L3_TURBO_WAVE_CLUSTERS, 16);
        assert_eq!(SYMBOL_L3_TURBO_ACTIVE_BYTES, 2 * 1024 * 1024);
        assert_eq!(SYMBOL_L3_DEFAULT_CELL8_CELLS, 512);
        assert_eq!(SYMBOL_L3_DEFAULT_WAVE_CLUSTERS, 32);
        assert_eq!(SYMBOL_L3_DEFAULT_ACTIVE_BYTES, 4 * 1024 * 1024);
        assert_eq!(L3_8MB_SYMBOL_CELL8_CELLS, 1024);
        assert_eq!(SYMBOL_WAVE_CLUSTER_CELLS, 16);
        assert_eq!(L3_8MB_SYMBOL_WAVE_CLUSTERS, 64);
        assert_eq!(L3_8MB_SYMBOL_ACTIVE_BYTES, 8 * 1024 * 1024);
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
