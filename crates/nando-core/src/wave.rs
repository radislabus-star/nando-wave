/// The first hot-cell atom. It is sized to fit the T480 L1 data-cache target.
pub const CELL32_BYTES: usize = 32 * 1024;

pub const SYMBOL_CELL32_HEADER_BYTES: usize = 256;
pub const SYMBOL_CELL32_PROJECTION_BYTES: usize = 4_096;
pub const SYMBOL_CELL32_MODE_BANK_BYTES: usize = 16_384;
pub const SYMBOL_CELL32_TRANSITION_BANK_BYTES: usize = 4_096;
pub const SYMBOL_CELL32_INTERFERENCE_BYTES: usize = 4_096;
pub const SYMBOL_CELL32_CALIBRATION_STATS_BYTES: usize = 2_048;
pub const SYMBOL_CELL32_SCRATCH_BYTES: usize = 1_792;
pub const SYMBOL_CELL32_MODES: usize = SYMBOL_CELL32_MODE_BANK_BYTES / 8;
pub const SYMBOL_CELL32_TRANSITIONS: usize = SYMBOL_CELL32_TRANSITION_BANK_BYTES / 8;
pub const SYMBOL_CELL32_INTERFERENCE_SLOTS: usize = SYMBOL_CELL32_INTERFERENCE_BYTES / 8;

pub const SYMBOL_CELL8_BYTES: usize = 8 * 1024;
pub const SYMBOL_CELL8_HEADER_BYTES: usize = 128;
pub const SYMBOL_CELL8_PROJECTION_BYTES: usize = 1_024;
pub const SYMBOL_CELL8_MODE_BANK_BYTES: usize = 4_096;
pub const SYMBOL_CELL8_TRANSITION_BANK_BYTES: usize = 1_024;
pub const SYMBOL_CELL8_INTERFERENCE_BYTES: usize = 1_024;
pub const SYMBOL_CELL8_CALIBRATION_STATS_BYTES: usize = 512;
pub const SYMBOL_CELL8_SCRATCH_BYTES: usize = 384;
pub const SYMBOL_CELL8_MODES: usize = SYMBOL_CELL8_MODE_BANK_BYTES / 8;
pub const SYMBOL_CELL8_TRANSITIONS: usize = SYMBOL_CELL8_TRANSITION_BANK_BYTES / 8;
pub const SYMBOL_CELL8_INTERFERENCE_SLOTS: usize = SYMBOL_CELL8_INTERFERENCE_BYTES / 8;
pub const SYMBOL_CELL8_PROJECTION_LANES: usize = SYMBOL_CELL8_PROJECTION_BYTES / 16;
pub const SYMBOL_CLIQUE_CLASS_BYTES: usize = 4 * 1024;
pub const SYMBOL_CELL_DENSE2K_BYTES: usize = 2 * 1024;
pub const SYMBOL_CELL_DENSE2K_MODES: usize = 256;
pub const SYMBOL_CELL_DENSE2K_TRANSITIONS: usize = 128;
pub const SYMBOL_CELL_DENSE2K_INTERFERENCE_SLOTS: usize = 96;
pub const SYMBOL_DENSE2K_CELLS_PER_2MB: usize = (2 * 1024 * 1024) / SYMBOL_CELL_DENSE2K_BYTES;
pub const SYMBOL_WAVE_CLUSTER_CELLS: usize = 16;
pub const SYMBOL_L3_TURBO_CELL8_CELLS: usize = 256;
pub const SYMBOL_L3_TURBO_WAVE_CLUSTERS: usize =
    SYMBOL_L3_TURBO_CELL8_CELLS / SYMBOL_WAVE_CLUSTER_CELLS;
pub const SYMBOL_L3_TURBO_ACTIVE_BYTES: usize = SYMBOL_L3_TURBO_CELL8_CELLS * SYMBOL_CELL8_BYTES;
pub const SYMBOL_L3_DEFAULT_CELL8_CELLS: usize = 512;
pub const SYMBOL_L3_DEFAULT_WAVE_CLUSTERS: usize =
    SYMBOL_L3_DEFAULT_CELL8_CELLS / SYMBOL_WAVE_CLUSTER_CELLS;
pub const SYMBOL_L3_DEFAULT_ACTIVE_BYTES: usize =
    SYMBOL_L3_DEFAULT_CELL8_CELLS * SYMBOL_CELL8_BYTES;
pub const L3_8MB_SYMBOL_CELL8_CELLS: usize = (8 * 1024 * 1024) / SYMBOL_CELL8_BYTES;
pub const L3_8MB_SYMBOL_WAVE_CLUSTERS: usize =
    L3_8MB_SYMBOL_CELL8_CELLS / SYMBOL_WAVE_CLUSTER_CELLS;
pub const L3_8MB_SYMBOL_ACTIVE_BYTES: usize = L3_8MB_SYMBOL_CELL8_CELLS * SYMBOL_CELL8_BYTES;

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
mod l1_center_memory;
mod l2_center_memory;
mod l3_self_induced_grokking;
mod l3_semantic_grokking;
mod learn;
mod math;
mod morphology_wave;
mod operator_blueprint;
mod operator_circuit;
mod operator_circuit_synthesis;
mod operator_grokking;
mod operator_grokking_proof;
mod operator_page;
mod organ;
mod phase_center_runtime;
mod semantic_extract;
mod semantic_wave;
mod snapshot;
mod surface_lm;
mod surface_motif;
mod surface_pattern;
mod surface_wave;
mod surface_word;
mod symbol_cell;
mod symbol_cluster;
mod symbol_l3;
mod tick;
mod wave_pattern_compiler;
mod wavepredictor_hebbian;
mod wavepredictor_trainer;

pub use bus::WaveBus;
pub use cache_plan::{CacheAwareOrganPlan, CacheProfile, HotWindowPlan, Organ128Plan};
pub use carrier::CarrierWave;
pub use cell::{Cell32, CellRank, Mono192};
pub use l1_center_memory::{
    L1_CENTER_RECORD_BYTES, L1_FOURIER_BINS, L1_RESIDUAL_NGRAM_BYTES, L1_SEQUENCE_REF_BYTES,
    L1_WORD_RECORD_BYTES, L1CenterMemory, L1CenterMemoryConfig, L1CenterMemoryProof,
    L1CenterMemoryVerdict, L1CenterSequence, L1SurfaceCenter, L1WordAssignment, L1WordCenterRecord,
};
pub use l2_center_memory::{
    L2_CENTER_RECORD_BYTES, L2_FOURIER_BINS, L2_RESIDUAL_REF_BYTES, L2_TOKEN_REF_BYTES,
    L2_WORD_RECORD_BYTES, L2CenterMemory, L2CenterMemoryConfig, L2CenterMemoryProof,
    L2CenterMemoryVerdict, L2SequenceCenter, L2TokenSequence, L2WordAssignment, L2WordRecord,
};
pub use l3_self_induced_grokking::{
    L3SelfInducedGrokkingConfig, L3SelfInducedGrokkingProof, L3SelfInducedGrokkingVerdict,
};
pub use l3_semantic_grokking::{
    L3_FRAME_CENTER_BYTES, L3_FRAME_FEATURE_BYTES, L3FrameCenter, L3FrameSelection,
    L3SemanticExample, L3SemanticGrokkingConfig, L3SemanticGrokkingMemory, L3SemanticGrokkingProof,
    L3SemanticGrokkingVerdict,
};
pub use learn::{
    Cell32Learner, Cell32PromotionReport, LinkProfile, LinkTissue, LiveByteLearner,
    LiveBytePrediction, LiveByteTrainReport, LiveByteTrainStep,
};
pub use morphology_wave::{
    MORPHOLOGY_ATOM_BYTES, MorphologyAtom, MorphologyExtraction, MorphologyGrokkingProof,
    MorphologyGrokkingVerdict, MorphologyScalingReport, MorphologyScalingRow, MorphologyWaveBank,
    MorphologyWaveConfig,
};
pub use operator_blueprint::{
    BlueprintBeamConfig, BlueprintFutureBlocker, BlueprintFutureConfig, BlueprintFutureEvaluator,
    BlueprintFutureEvidence, BlueprintFutureEvidenceError, BlueprintFutureReport,
    BlueprintFutureScore, BlueprintPhaseControl, BlueprintSynthesisBlocker,
    BlueprintSynthesisBlockerCount, BlueprintSynthesisReport, BoundedCircuitBeam,
    BoundedRoleAligner, CandidateOperatorBlueprint, Commitment256, CompositionDag, CompositionEdge,
    FrozenBlueprintError, FrozenBlueprintFutureWindow, FrozenOperatorBlueprintSet,
    LocalRelationFragment, OPERATOR_BLUEPRINT_CANONICALIZER_VERSION,
    OPERATOR_BLUEPRINT_MAX_ALIGNMENTS, OPERATOR_BLUEPRINT_MAX_BEAM_DEPTH,
    OPERATOR_BLUEPRINT_MAX_BUNDLES, OPERATOR_BLUEPRINT_MAX_EXPANSIONS,
    OPERATOR_BLUEPRINT_MAX_PROGRAM_ATOMS, OPERATOR_BLUEPRINT_MAX_RELATIONS,
    OPERATOR_BLUEPRINT_MAX_ROLES, OPERATOR_ROLE_COLOR_ROUNDS, OPERATOR_ROLE_NONE, RendererContract,
    RoleAlignmentBlocker, RoleAlignmentConfig, RoleAlignmentHypothesis, RoleAlignmentReport,
    RoleBinding, RoleGraph, RuntimeRoleBinder, RuntimeRoleBindingReport, RuntimeRoleMapping,
    SealedBlueprintEvaluation, SealedBlueprintWinnerReceipt, SearchCompletion, SearchStage,
    StructuralRoleCanonicalizer, StructuralRoleSignature, SurfaceFragmentBundle,
    SurfaceFragmentBundleError, TypedProgramAtom, VerifierContract,
};
pub use operator_circuit::{
    OPERATOR_CIRCUIT_MAX_RELATIONS, OPERATOR_CIRCUIT_MAX_ROLES, OPERATOR_WAVE_MAX_SAMPLES,
    OperatorCircuit, OperatorCircuitError, OperatorCircuitRelation, OperatorRelationCell,
    TernaryRelationState, VerifiedPartialRelationWave, VerifiedRelationSample, VerifiedWaveOutcome,
};
pub use operator_circuit_synthesis::{
    CIRCUIT_SYNTHESIS_MAX_CIRCUITS, CIRCUIT_SYNTHESIS_MAX_FRAGMENTS, CircuitSynthesisBlocker,
    CircuitSynthesisBlockerCount, CircuitSynthesisConfig, CircuitSynthesisError,
    CircuitSynthesizer, FrozenCircuitSetError, FrozenFutureCircuitField,
    FrozenSynthesizedCircuitSet, OperatorCircuitSynthesisReport, RelationFragment,
    RelationFragmentGenerator, RelationFragmentReport,
};
pub use operator_grokking::{
    CandidateCubeField, CandidateCubeFieldError, CoherentOperatorCandidate, OperatorCircuitScore,
    OperatorCircuitStage, OperatorConsolidationReport, OperatorGrokkingConfig,
    OperatorGrokkingConsolidator,
};
pub use operator_grokking_proof::{
    OperatorGrokkingAblation, OperatorGrokkingAblationReceipt, OperatorGrokkingProofStage,
    OperatorGrokkingProofTracker, ProvenOperatorGrokking,
};
pub use operator_page::{
    OPERATOR_PAGE32_BYTES, OPERATOR_PAGE32_COMPOSITION_BYTES, OPERATOR_PAGE32_CUBE_BYTES,
    OPERATOR_PAGE32_HEADER_BYTES, OPERATOR_PAGE32_MAGIC, OPERATOR_PAGE32_MAX_PLANES,
    OPERATOR_PAGE32_MAX_ROLES, OPERATOR_PAGE32_MAX_TRANSFORMS, OPERATOR_PAGE32_PHASE_BYTES,
    OPERATOR_PAGE32_RENDERER_BYTES, OPERATOR_PAGE32_ROLES_BYTES, OPERATOR_PAGE32_SCHEMA_VERSION,
    OPERATOR_PAGE32_TRANSFORM_BYTES, OperatorPage32, OperatorPage32Error, OperatorPage32Header,
    OperatorPage32Metadata, StructuralRole16, TernaryOperatorCube32, TransformOp8,
};
pub use organ::{
    LiveCycle, LivePrediction, LocalUpdateReport, OrganState, Stage2Organ, stage2_organ,
};
pub use phase_center_runtime::{
    PHASE_CENTER_DEFAULT_OFFLOAD_MARGIN_THRESHOLD_MICRO,
    PHASE_CENTER_HOT_RUNTIME_PACKAGE_FINGERPRINT_PERSONAL,
    PHASE_CENTER_HOT_RUNTIME_PACKAGE_HEADER_BYTES, PHASE_CENTER_HOT_RUNTIME_PACKAGE_MAGIC,
    PHASE_CENTER_RUNTIME_PACKAGE_FINGERPRINT_PERSONAL, PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES,
    PHASE_CENTER_RUNTIME_PACKAGE_MAGIC, PhaseCenterAtomEncoder, PhaseCenterCell,
    PhaseCenterCompiler, PhaseCenterEvalTask, PhaseCenterFlatRecord, PhaseCenterFlatRuntime,
    PhaseCenterHotCandidateDecision, PhaseCenterHotDecision, PhaseCenterHotEvidenceRequest,
    PhaseCenterHotPackagePolicyDefaults, PhaseCenterHotProfile, PhaseCenterHotRequest,
    PhaseCenterHotRequestEvidence, PhaseCenterHotRoutePlan, PhaseCenterHotRouteTable,
    PhaseCenterHotRowPreparer, PhaseCenterHotRuntime, PhaseCenterHotRuntimePackage,
    PhaseCenterHotRuntimePackageInfo, PhaseCenterHotScratch, PhaseCenterHotShadowEval,
    PhaseCenterHotWorker, PhaseCenterLiveOperatorAtomEvent, PhaseCenterLiveOperatorStore,
    PhaseCenterLiveOperatorStoreConfig, PhaseCenterLiveRouteStats, PhaseCenterLocalAcceptBlocker,
    PhaseCenterLocalAcceptDecision, PhaseCenterLocalAcceptEvidence, PhaseCenterOffloadAction,
    PhaseCenterOffloadDecision, PhaseCenterOffloadPolicy, PhaseCenterOffloadRuntime,
    PhaseCenterOffloadSummary, PhaseCenterOnlineBucket, PhaseCenterOnlineCandidatePackage,
    PhaseCenterOnlineDecision, PhaseCenterOnlineEvent, PhaseCenterOnlineMiner,
    PhaseCenterOnlineMinerConfig, PhaseCenterOnlineSummary, PhaseCenterOperatorAdmission,
    PhaseCenterOperatorAdmissionBlocker, PhaseCenterOperatorAdmissionDecision,
    PhaseCenterOperatorMemory, PhaseCenterOperatorMemoryConfig, PhaseCenterOperatorProfile,
    PhaseCenterOperatorRoute, PhaseCenterPreparedHotDenominator,
    PhaseCenterPreparedHotEvidenceRequest, PhaseCenterPreparedHotEvidenceRow,
    PhaseCenterPreparedHotRequest, PhaseCenterPromotionBlocker, PhaseCenterPromotionDecision,
    PhaseCenterPromotionEvidence, PhaseCenterRuntimeBudgetSnapshot, PhaseCenterRuntimeError,
    PhaseCenterRuntimePackageInfo, PhaseCenterSavingsBlocker, PhaseCenterSavingsDenominator,
    PhaseCenterSavingsEvidence, PhaseCenterSavingsReport, PhaseCenterThresholdPolicyEvidence,
    PhaseCenterVerifierBinding, add_phase_vector, phase_center_from_sum, phase_circular_unit,
    phase_coherence, phase_margin_from_centers, phase_margin_to_micro, phase_vector_from_atom_ids,
    phase_vector_from_atoms, stable_phase_atom_id_cell, stable_phase_cell,
};
pub use semantic_extract::{
    SemanticAtomExtractor, SemanticEquationForm, SemanticExtractedForm, SemanticExtraction,
    SemanticExtractionStatus, semantic_label_slot,
};
pub use semantic_wave::{
    SEMANTIC_OPERATOR_BYTES, SEMANTIC_WAVE_BYTES, SEMANTIC_WAVE_DIM, SemanticAtom,
    SemanticCandidate, SemanticEquationPrediction, SemanticFact, SemanticPrediction, SemanticQuery,
    SemanticRelationOperator, SemanticSchemaKey, SemanticWave4096, SemanticWaveEvalReport,
    SemanticWaveGrokkingProof, SemanticWaveGrokkingVerdict, SemanticWaveMemory,
};
pub use snapshot::{SnapshotParseError, SpectrumSnapshot, Stage2Tick, TickTrace};
pub use surface_lm::{
    SURFACE_WAVE_LM_BIAS_BYTES, SURFACE_WAVE_LM_OUTPUTS, SURFACE_WAVE_LM_POSITION_BUCKETS,
    SURFACE_WAVE_LM_POSITION_BYTES, SURFACE_WAVE_LM_POSITION_SCORE_WEIGHT,
    SURFACE_WAVE_LM_STATE_MAX, SURFACE_WAVE_LM_STATE_MIN, SURFACE_WAVE_LM_WEIGHT_BYTES,
    SurfaceWaveContext4096, SurfaceWaveGeneration, SurfaceWaveLm, SurfaceWaveLmConfig,
    SurfaceWaveLmEvalReport, SurfaceWaveLmTrainReport, SurfaceWaveTextScore,
};
pub use surface_motif::{
    SURFACE_MOTIF_RECORD_BYTES, SURFACE_MOTIF_REF_BYTES, SurfaceMotif, SurfaceMotifBank,
    SurfaceMotifRef, SurfaceMotifSpec, SurfaceResidualRecord,
};
pub use surface_pattern::{
    SurfaceWaveGenerationCase, SurfaceWaveGenerationProof, SurfaceWavePatternProof,
    SurfaceWavePatternProofConfig, SurfaceWavePatternVerdict,
};
pub use surface_wave::{
    SURFACE_WAVE_BYTES, SURFACE_WAVE_DIM, SURFACE_WAVE_NGRAM, SURFACE_WAVE_TRITS, SurfaceAtom,
    SurfaceWave4096, SurfaceWaveLane, SurfaceWaveTrit, surface_atom_projection, surface_atoms,
    surface_ngram_count, surface_ngram_projection,
};
pub use surface_word::{
    SurfaceWordGrokkingConfig, SurfaceWordGrokkingProof, SurfaceWordGrokkingVerdict,
};
pub use symbol_cell::{
    CalibrationStats, Interference8, Mode8, PackedInterference32, PackedMode32, PackedTransition32,
    PeakOutcome, ProjectionEntry16, StablePeakScore, SymbolCell8, SymbolCell8Advice,
    SymbolCell8Calibration, SymbolCell8Header, SymbolCell8Tick, SymbolCell32, SymbolCellDense2K,
    SymbolCliqueClass, SymbolExcitation, SymbolHeader, SymbolProjection, Transition8,
};
pub use symbol_cluster::{SymbolClusterCenter, SymbolClusterTick, SymbolWaveCluster};
pub use symbol_l3::{SymbolL3Center, SymbolL3Organism, SymbolL3Tick};
pub use tick::{
    BytePhaseLut, Stage2BusTraceTick, Stage2TraceTick, encode_byte_phases,
    run_stage2_bus_trace_with_organ_carrier, run_stage2_tick, run_stage2_tick_with_carrier,
    run_stage2_tick_with_disabled, run_stage2_tick_with_organ_carrier,
    run_stage2_tick_with_organ_lut_carrier, run_stage2_trace_tick,
    run_stage2_trace_with_organ_carrier, run_stage2_trace_with_organ_lut_carrier,
};
pub use wave_pattern_compiler::{
    SURFACE_FOURIER_BINS, SurfaceFourierSignature, SurfaceWaveCenter, WavePatternCompileReport,
    WavePatternCompiler, WavePatternSelection, WavePatternTemplate,
};
pub use wavepredictor_hebbian::{
    WavePredictorActiveCenter, WavePredictorCenterId, WavePredictorConvergenceError,
    WavePredictorHebbianConfig, WavePredictorHebbianEdge, WavePredictorHebbianField,
    WavePredictorHebbianUpdateReport,
};
pub use wavepredictor_trainer::{
    WAVEPREDICTOR_STATE_DELTA_CAP, WAVEPREDICTOR_TARGET_AXIS_CAP, WavePredictorAxisTarget,
    WavePredictorCompositionalTrainTask, WavePredictorEpochReport, WavePredictorMarginSchedule,
    WavePredictorStateDeltaTarget, WavePredictorStateDeltaTrainTask, WavePredictorStateImpulse,
    WavePredictorTrainTask, WavePredictorTrainer, WavePredictorTrainerConfig,
    WavePredictorTrainerReport,
};

pub(crate) use math::{
    circular_phase_delta, insert_top_index, insert_top_slot, normalized_entropy, unit_noise,
};
pub(crate) use tick::{
    run_stage2_bus_trace_with_organ_state, run_stage2_tick_with_organ_lut_state,
    run_stage2_tick_with_state,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn fixed_packets_have_expected_sizes() {
        assert_eq!(size_of::<Cell32>(), CELL32_BYTES);
        assert_eq!(size_of::<SymbolCell32>(), CELL32_BYTES);
        assert_eq!(size_of::<SymbolCell8>(), SYMBOL_CELL8_BYTES);
        assert_eq!(size_of::<SymbolCliqueClass>(), SYMBOL_CLIQUE_CLASS_BYTES);
        assert_eq!(size_of::<SymbolCellDense2K>(), SYMBOL_CELL_DENSE2K_BYTES);
        assert_eq!(SYMBOL_DENSE2K_CELLS_PER_2MB, 1_024);
        assert_eq!(SYMBOL_CELL32_MODES, 2_048);
        assert_eq!(SYMBOL_CELL8_MODES, 512);
        assert_eq!(SYMBOL_L3_TURBO_CELL8_CELLS, 256);
        assert_eq!(SYMBOL_L3_TURBO_WAVE_CLUSTERS, 16);
        assert_eq!(SYMBOL_L3_TURBO_ACTIVE_BYTES, 2 * 1024 * 1024);
        assert_eq!(SYMBOL_L3_DEFAULT_CELL8_CELLS, 512);
        assert_eq!(SYMBOL_L3_DEFAULT_WAVE_CLUSTERS, 32);
        assert_eq!(SYMBOL_L3_DEFAULT_ACTIVE_BYTES, 4 * 1024 * 1024);
        assert_eq!(L3_8MB_SYMBOL_CELL8_CELLS, 1_024);
        assert_eq!(SYMBOL_WAVE_CLUSTER_CELLS, 16);
        assert_eq!(L3_8MB_SYMBOL_WAVE_CLUSTERS, 64);
        assert_eq!(L3_8MB_SYMBOL_ACTIVE_BYTES, 8 * 1024 * 1024);
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
        assert_eq!(report.cases, "a файл a файл".len() - 1);
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
