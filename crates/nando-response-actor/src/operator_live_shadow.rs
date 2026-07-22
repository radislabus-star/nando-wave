use std::collections::{BTreeMap, BTreeSet};

use nando_core::wave::{
    BlueprintBeamConfig, BlueprintFutureEvaluator, BlueprintFutureEvidence, BlueprintPhaseControl,
    BlueprintSynthesisBlockerCount, BlueprintSynthesisReport, BoundedCircuitBeam,
    BoundedRoleAligner, FrozenOperatorBlueprintSet, LocalRelationFragment, OPERATOR_ROLE_NONE,
    RoleAlignmentConfig, SearchCompletion, StructuralRoleSignature, SurfaceFragmentBundle,
    TernaryRelationState, TypedProgramAtom, phase_vector_from_atoms,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

mod induction;
mod state;

pub use induction::extract_live_scalar_circuit_sample;
#[cfg(test)]
use induction::{
    bounded_ordinal_role_permutations, normalized_scalar_renderer, teacher_field_selector_hint,
};
#[cfg(test)]
use state::{common_support_actor_hypotheses, update_support_actor_hypotheses};

use crate::{
    AtomValueType, CollectionOutputRenderer, CollectionProgramStep, CollectionScalarType,
    CollectionSynthesisExample, CrystallizationParityReceipt, CrystallizedOperator,
    ProjectStatusMapping, ResponseArgument, ResponseOperation, ResponsePackage,
    ResponsePackageOrigin, ResponsePackageProof, ResponsePackageState, ResponseProgram,
    ResponseRenderSegment, ResponseValueSelector, RuntimeRoleAnchor, TRANSFORM_FLAG_CANONICAL_JSON,
    TRANSFORM_OPCODE_COUNT_COLLECTION, TRANSFORM_OPCODE_FILTER_REQUEST_VALUE,
    TRANSFORM_OPCODE_PROJECT_STATUS, TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR,
    TRANSFORM_STATUS_ZERO_IS_OK, TRANSFORM_STATUS_ZERO_IS_PASS, TRANSFORM_STATUS_ZERO_IS_SUCCESS,
    TRANSFORM_STATUS_ZERO_IS_TRUE, TRANSFORM_VALUE_BOOLEAN, TRANSFORM_VALUE_COLLECTION,
    TRANSFORM_VALUE_IDENTIFIER, TRANSFORM_VALUE_INTEGER, TRANSFORM_VALUE_STRING, TeacherTransition,
    ValueProjectionFormat, enumerate_source_neutral_response_programs, execute_response,
    is_privacy_safe_online_response_program, is_source_neutral_response_program,
    response_actor_program_digest, response_independent_verifier_program_digest,
    source_neutral_verifier_for_program, synthesize_response_operator,
};

const LIVE_SCALAR_SUPPORT_ROWS: usize = 32;
const LIVE_SCALAR_FUTURE_ROWS: usize = 32;
const TEACHER_CALL_SELECTOR_BUDGET: usize = 512;
const COMMON_ACTOR_TOPOLOGY_BUDGET: usize = 64;
// Session capture already bounds one active-turn provider envelope to 128 KiB.
// Keep the learner contract identical so bounded multi-output turns are not
// discarded by a second, narrower limit after capture.
const LIVE_SCALAR_MAX_PROVIDER_PAYLOAD_BYTES: usize = 128 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct LiveScalarCircuitSample {
    pub bundle: SurfaceFragmentBundle,
    pub anchors: Box<[RuntimeRoleAnchor]>,
    pub actor_template: ResponseProgram,
    pub actor_hypotheses: Box<[ResponseProgram]>,
    pub request_text: String,
    pub provider_payload: Value,
    pub expected_response: String,
    pub raw_input_sha256: [u8; 32],
    pub extractor_version: u32,
    pub law_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveScalarShadowBlocker {
    TeacherRejected,
    MissingParityCase,
    PayloadTooLarge,
    NoExactSourceNeutralProgram,
    ProgramEnumerationFailed,
    CandidateBudgetExhausted,
    EmptyVersionSpace,
    TeacherSynthesisEmptySupport,
    TeacherSynthesisAmbiguousRoles,
    TeacherSynthesisInconsistentRoleFamily,
    TeacherSynthesisMissingPendingState,
    TeacherSynthesisMissingCompletionState,
    TeacherSynthesisMissingUniqueHandle,
    TeacherSynthesisNoConsistentProgram,
    TeacherSynthesisAmbiguousPrograms,
    TeacherProgramNotCall,
    TeacherProgramParityMismatch,
    TeacherProgramRuntimeAbstain,
    TeacherProgramImmediateToolOutputMissing,
    TeacherProgramSelectorPrefixMissing,
    TeacherProgramSelectorPrefixAmbiguous,
    TeacherProgramSelectorTypeMismatch,
    TeacherProgramOutputTextInvalid,
    TeacherProgramSelectorParseFailed,
    TeacherProgramRoleParseFailed,
    TeacherProgramVerificationFailed,
    TeacherProgramInvalid,
    TeacherProgramWireEnvelopeMismatch,
    TeacherProgramSymbolMismatch,
    TeacherProgramInputMismatch,
    TeacherProgramInputWhitespaceMismatch,
    TeacherProgramInputTokenValueMismatch,
    TeacherProgramInputSingleNumericMismatch,
    TeacherProgramInputMultipleNumericMismatch,
    TeacherProgramInputQuotedLiteralMismatch,
    TeacherProgramInputIdentifierMismatch,
    TeacherProgramInputMixedTokenMismatch,
    TeacherProgramDynamicRoleNumericMismatch,
    TeacherProgramDynamicRoleStringMismatch,
    TeacherProgramStaticIntegerMismatch,
    TeacherProgramStaticStringMismatch,
    TeacherProgramRoleValueUnavailable,
    TeacherProgramRoleValueNotObserved,
    TeacherProgramRoleValueCandidateMismatch,
    TeacherProgramRoleValueInRequestText,
    TeacherProgramRoleValueInPayloadScalar,
    TeacherProgramRoleValueInPayloadText,
    TeacherProgramRoleValueAbsentFromPayload,
    TeacherProgramInputSyntaxShapeMismatch,
    TeacherProgramResponseShapeMismatch,
    ExactStatusProgram,
    ExactCollectionProgram,
    UnsupportedRendererProgram,
    ExactTypedCanonicalizationFailed,
    SelectedTemplateCanonicalizationFailed,
    CanonicalCandidateMissing,
    LawShapeMissing,
    HypothesisEncodingFailed,
    HypothesisBudgetExhausted,
    RoleTypeInferenceFailed,
    ObservedRoleExtractionFailed,
    PayloadSerializationFailed,
    RequestTextInvalid,
    ProviderInputMissing,
    UnsupportedTransformOpcode,
    UnsupportedTransformFlags,
    UnsupportedProgramKind,
    // Legacy aggregate retained for checkpoint compatibility.
    UnsupportedScalarProgram,
    InvalidCommitment,
    InvalidBundle,
    // Kept for checkpoints written before support rows were separated from
    // session-diversity evidence. New generations no longer emit this blocker.
    SupportSessionReused,
    // Legacy checkpoints counted every repeated future session as a blocker.
    // New generations only reject overlap across the support/future boundary.
    FutureSessionReused,
    SupportFutureSessionOverlap,
    FutureCapacityReached,
    HistoricalSupportCapacityReached,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LiveScalarShadowState {
    observations: usize,
    executable: usize,
    duplicate_rows: usize,
    blockers: BTreeMap<LiveScalarShadowBlocker, usize>,
    laws: BTreeMap<String, LiveScalarLawState>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct LiveScalarLawState {
    support: Vec<TeacherTransition>,
    future: Vec<TeacherTransition>,
    // Physical selectors are factored out of the semantic law and intersected
    // once as support arrives. Keeping this compact version set here prevents
    // every report from repeating the full selector search over all 32 rows.
    #[serde(default)]
    support_actor_hypotheses: Vec<ResponseProgram>,
    #[serde(default)]
    support_hypotheses_initialized: bool,
}

struct CompetingBlueprintSet {
    support_bundles: Vec<SurfaceFragmentBundle>,
    synthesis: BlueprintSynthesisReport,
    actors_by_blueprint: BTreeMap<[u8; 32], ResponseProgram>,
    actor_hypothesis_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LiveScalarShadowReport {
    pub observations: usize,
    pub executable: usize,
    pub duplicate_rows: usize,
    pub law_count: usize,
    pub support_rows: usize,
    pub future_rows: usize,
    pub actor_hypotheses: usize,
    pub competing_blueprints: usize,
    pub frozen_laws: usize,
    pub full_phase_winners: usize,
    pub causal_control_passes: usize,
    pub verified_shadow_operators: usize,
    pub shadow_executions: usize,
    pub admission_candidates: usize,
    pub ingest_accounting_complete: bool,
    pub laws: Vec<LiveScalarLawReport>,
    pub blockers: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LiveScalarLawReport {
    pub law_sha256: String,
    pub teacher_action_symbol: String,
    pub operation_kind: String,
    pub support_rows: usize,
    pub future_rows: usize,
    pub distinct_support_sessions: usize,
    pub actor_hypotheses: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LiveScalarAdmissionCandidate {
    pub package: ResponsePackage,
    pub support: Vec<TeacherTransition>,
    pub future: Vec<TeacherTransition>,
    pub support_root_sha256: String,
    pub future_evidence_root_sha256: String,
    pub future_lineage_root_sha256: String,
    pub winner_seal_sha256: String,
    pub executable_parity_seal_sha256: String,
}

#[cfg(test)]
#[path = "operator_live_shadow_tests.rs"]
mod tests;
