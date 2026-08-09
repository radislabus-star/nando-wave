use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

use nando_core::wave::{
    BlueprintBeamConfig, BlueprintFutureEvaluator, BlueprintFutureEvidence, BlueprintPhaseControl,
    BlueprintSynthesisBlockerCount, BlueprintSynthesisReport, BoundedCircuitBeam,
    BoundedRoleAligner, FrozenOperatorBlueprintSet, LocalRelationFragment,
    OPERATOR_BLUEPRINT_MAX_BUNDLES, OPERATOR_ROLE_NONE, RoleAlignmentConfig, SearchCompletion,
    StructuralRoleSignature, SurfaceFragmentBundle, TernaryRelationState, TypedProgramAtom,
    phase_vector_from_atoms,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

mod identification;
mod induction;
mod raw_phase;
mod state;
mod transfer_basis;

pub use induction::extract_live_scalar_circuit_sample;
#[cfg(test)]
use induction::{
    bounded_ordinal_role_permutations, normalized_scalar_renderer, teacher_field_selector_hint,
};
pub use state::crystallize_multi_source_t1_candidate_v1;
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

const LIVE_SCALAR_MAX_EVIDENCE_ROWS: usize = 64;
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
    CaptureProvenanceConflict,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LiveScalarShadowState {
    observations: usize,
    executable: usize,
    duplicate_rows: usize,
    blockers: BTreeMap<LiveScalarShadowBlocker, usize>,
    #[serde(default)]
    extraction_blockers_by_action: BTreeMap<String, BTreeMap<LiveScalarShadowBlocker, usize>>,
    laws: BTreeMap<String, LiveScalarLawState>,
    // Derived proof work can be much more expensive than ingest. Cache it per
    // immutable law generation so authority refreshes do not replay every law.
    #[serde(skip)]
    evaluation_cache: RefCell<BTreeMap<String, LiveScalarLawEvaluation>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct LiveScalarLawState {
    support: Vec<TeacherTransition>,
    future: Vec<TeacherTransition>,
    // Physical selectors are factored out of the semantic law and intersected
    // once as support arrives. Keeping this compact version set here prevents
    // every report from repeating the full selector search over bounded support.
    #[serde(default)]
    support_actor_hypotheses: Vec<ResponseProgram>,
    #[serde(default)]
    support_hypotheses_initialized: bool,
}

#[derive(Clone, Debug, Default)]
struct LiveScalarLawEvaluation {
    report: LiveScalarShadowReport,
    candidates: Vec<LiveScalarAdmissionCandidate>,
}

pub(super) struct CompetingBlueprintSet {
    pub(super) support_bundles: Vec<SurfaceFragmentBundle>,
    pub(super) synthesis: BlueprintSynthesisReport,
    pub(super) actors_by_blueprint: BTreeMap<[u8; 32], ResponseProgram>,
    pub(super) actor_hypothesis_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LiveScalarShadowReport {
    pub identification_policy: String,
    pub candidate_freezes: usize,
    pub transfer_proofs: usize,
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
    pub transfer_basis_rows: usize,
    pub monitored_future_rows: usize,
    pub future_applicability_negatives: usize,
    pub future_censored_rows: usize,
    pub shadow_executions: usize,
    pub admission_candidates: usize,
    pub ingest_accounting_complete: bool,
    pub laws: Vec<LiveScalarLawReport>,
    pub blockers: BTreeMap<String, usize>,
    pub extraction_blockers_by_action: BTreeMap<String, BTreeMap<String, usize>>,
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
    #[serde(default)]
    pub evaluation_blockers: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LiveScalarAdmissionCandidate {
    pub package: ResponsePackage,
    pub support: Vec<TeacherTransition>,
    pub future: Vec<TeacherTransition>,
    #[serde(default)]
    pub multi_source_identification:
        Option<nando_operator_learning::multi_source::MultiSourceT1IdentificationV3>,
    #[serde(default)]
    pub freeze_watermark_unix_nanos: u64,
    #[serde(default)]
    pub partition_commitment_sha256: String,
    pub support_root_sha256: String,
    pub future_evidence_root_sha256: String,
    pub future_lineage_root_sha256: String,
    pub winner_seal_sha256: String,
    pub executable_parity_seal_sha256: String,
}

impl LiveScalarAdmissionCandidate {
    pub(crate) fn seal_evidence_partition(&mut self) -> Result<(), &'static str> {
        self.freeze_watermark_unix_nanos = evidence_partition_watermark(self)?;
        self.partition_commitment_sha256 = evidence_partition_commitment(self);
        Ok(())
    }

    pub fn verify_evidence_partition(&self) -> Result<(), &'static str> {
        if self.freeze_watermark_unix_nanos == 0
            || self.freeze_watermark_unix_nanos != evidence_partition_watermark(self)?
            || self.partition_commitment_sha256 != evidence_partition_commitment(self)
        {
            return Err("crystallized_evidence_partition_mismatch");
        }
        Ok(())
    }
}

fn evidence_partition_watermark(
    candidate: &LiveScalarAdmissionCandidate,
) -> Result<u64, &'static str> {
    let support_max = candidate
        .support
        .iter()
        .map(|row| row.before.observed_at_unix_nanos)
        .max()
        .ok_or("crystallized_support_partition_empty")?;
    let future_min = candidate
        .future
        .iter()
        .map(|row| row.before.observed_at_unix_nanos)
        .min()
        .ok_or("crystallized_future_partition_empty")?;
    if future_min <= support_max {
        return Err("crystallized_evidence_partition_reordered");
    }
    Ok(support_max)
}

fn evidence_partition_commitment(candidate: &LiveScalarAdmissionCandidate) -> String {
    let mut hasher = Sha256::new();
    if let Some(identification) = &candidate.multi_source_identification {
        hasher.update(b"nando.live-scalar-evidence-partition.v2");
        hasher.update(identification.report_root_sha256.as_bytes());
    } else {
        hasher.update(b"nando.live-scalar-evidence-partition.v1");
    }
    hasher.update(candidate.freeze_watermark_unix_nanos.to_le_bytes());
    hasher.update(candidate.support_root_sha256.as_bytes());
    hasher.update(candidate.future_evidence_root_sha256.as_bytes());
    hasher.update(candidate.future_lineage_root_sha256.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
#[path = "operator_live_shadow_tests.rs"]
mod tests;
