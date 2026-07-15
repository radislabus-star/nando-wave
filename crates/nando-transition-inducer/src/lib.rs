//! Cold-path induction of typed transition packages from observed traces.
//!
//! Wave mechanics ranks and compresses relational hypotheses. The actor DSL
//! executes a selected program, and an independent verifier authorizes its
//! output. This crate is deliberately absent from the serving hot path.

mod a2_fixture;
mod causal_proof;
mod guard;
mod hypothesis;
mod induce;
mod live_execution;
mod live_profile;
mod package;
mod raw_grokking_proof;
mod raw_phase;
mod synthesis;
mod trace;
mod verifier;
mod wave;

pub mod a1_lab;
pub mod a2_lab;

pub use causal_proof::{
    WaveCausalModeReport, WaveCausalProofReport, WaveCausalVerdicts, WaveFormationCheckpoint,
    run_wave_causal_proof,
};

pub use guard::{GuardFailure, GuardProgram};
pub use hypothesis::{OperatorSkeleton, RoleHypothesis};
pub use induce::{InductionError, InductionMetrics, TransitionInducer};
pub use live_execution::{
    LIVE_TRANSITION_MAX_REQUEST_BYTES, LIVE_TRANSITION_REQUEST_SCHEMA,
    LIVE_TRANSITION_RESPONSE_SCHEMA, LiveTransitionExecutor, LiveTransitionRequest,
    LiveTransitionResponse,
};
pub use live_profile::{
    AutonomousPromotionPolicy, LIVE_GROUNDED_TRACE_SCHEMA, LIVE_POLICY_VERSION,
    LIVE_REGISTRY_SCHEMA, LIVE_TRACE_SCHEMA, LiveObservedTransition, LivePackageOrigin,
    LivePackageRecord, LiveProfileRegistry, LiveProfileState, LiveRuntimeProfile,
    LiveTransitionTelemetry, RawPhaseFamilyState, RawPhaseSurfaceState, atomic_write_json,
    import_package, import_package_with_origin, packages_from_value, read_package,
    timestamp_unix_nanos, validate_live_package,
};
pub use package::{
    InducedExecution, InducedExecutionStatus, InducedTransition, InducedTransitionPackage,
};
pub use raw_grokking_proof::{
    RawGrokkingCheckpoint, RawGrokkingModeReport, RawGrokkingVerdicts, RawPhaseGrokkingProofReport,
    run_raw_phase_grokking_proof,
};
pub use raw_phase::{
    RawPhaseConfig, RawPhaseInducer, RawPhaseInductionError, RawPhaseInductionMetrics,
    RawPhaseTrainingMetrics, RawPhaseTransferMetrics, evaluate_leave_one_surface_out,
    evaluate_support_query_transfer, split_forward_adaptation_query, transition_family_key,
    transition_surface_key,
};
pub use synthesis::SynthesisMetrics;
pub use trace::{LayoutShape, SurfaceShape, TransitionTrace};
pub use verifier::{VerifierFailure, VerifierProgram};
pub use wave::{
    PortablePhaseCell, PortablePhaseCenter, PortableRoutingSignature, RelationWaveMemory,
    WaveAblation, WaveContributionMetrics, WaveIdTrainingExample, WaveTrainingExample,
};
