use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    MultiSourceContainerClassV1, MultiSourceExtractionStatusV1, MultiSourceTypeClassV1,
    RelationFrame, canonical_json_sha256, valid_nonzero_sha256,
};
#[cfg(test)]
use nando_operator_learning::multi_source::MultiSourceJoinLedgerV1;
use nando_operator_learning::multi_source::{
    BlindThenRevealJoinedTransitionV1, CompletedEffectFormV1, FactorizedMultiSourceRowV1,
    FrozenRawPhaseT1ContractV1, K1_CANDIDATE_READINESS_MIN_LINEAGES_V1,
    K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1, K1_MISSING_COMPLETED_FRAME_BLOCKER_V1,
    K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V1, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V2,
    K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V3, K1_NATURAL_EVIDENCE_ROW_SCHEMA_V1,
    K1_NATURAL_EVIDENCE_ROW_SCHEMA_V2, K1ConsequenceTypeV1, K1GenerationBudgetV1,
    K1GenerationTerminalVerdictV1, K1GenerationVerdictClassV1, K1IdentificationFreezeV1,
    K1NaturalCandidateFreezeV1, K1NaturalCandidateQueueV1, K1NaturalCohortCatalogV1,
    K1NaturalEvidenceClassV1, K1NaturalEvidenceRowV1, K1ProbeBudgetRemainingV1,
    K1ProbeClassPredictionV1, K1ProbeRoundReceiptV1, K1ProbeRoundStateV1,
    K1SchedulerEventPayloadV1, MULTI_SOURCE_JOIN_MAX_ROWS_V1,
    MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2, MultiSourceJoinCensoredReasonV1,
    MultiSourceJoinReportV1, MultiSourceReasonV1, MultiSourceT1IdentificationStateV1,
    MultiSourceT1IdentificationV3, NaturalT1ProgramArtifactV1, PassiveT1ProbeContractV1,
    PreActionShapeClassV1, PreActionTopologyAuditRowV1, TransportTerminalReceiptV1,
    build_k1_natural_candidate_queue_with_exclusions_v1, build_k1_natural_cohort_catalog_v1,
    factor_multi_source_row_v1, identify_multi_source_t1_operator_with_frozen_raw_phase_v1,
    join_prepared_multi_source_frame_v1, natural_t1_discovery_basis_root_v1,
    pre_action_applicability_shape_root_v1, pre_action_t1_binding_root,
    prepare_multi_source_join_frame_v1, source_neutral_topology_root_v1,
    stream_multi_source_joins_from_iter, validate_pre_action_topology_join_eligibility_v1,
};
use serde::{Deserialize, Serialize};

use crate::k1_natural_scheduler::{
    K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V1,
    K1_FUTURE_CONTRACT_AUTHORITY_REQUEST_SCHEMA_V1, K1_FUTURE_OUTCOME_AUTHORITY_REQUEST_SCHEMA_V1,
    K1_FUTURE_PREDICTION_AUTHORITY_REQUEST_SCHEMA_V1,
    K1_FUTURE_PREDICTION_CENSOR_AUTHORITY_REQUEST_SCHEMA_V1, K1CandidateFreezeAuthorityRequestV1,
    K1FutureContractAuthorityRequestV1, K1FutureOutcomeAuthorityRequestV1,
    K1FuturePredictionAuthorityRequestV1, K1FuturePredictionCensorAuthorityRequestV1,
    K1SchedulerLaneV1, K1SchedulerProjectionV1, append_candidate_freeze_for,
    append_future_contract, append_future_outcome, append_future_prediction,
    append_future_prediction_censor, append_scheduler_payload_for, candidate_exclusions_for,
    current_deficit_snapshot, duplicate_candidate_exclusions_for, restore_projection_for,
};
use crate::k1_transfer_lifecycle::K1TransferLifecycleReportV1;
use crate::operator_certification::CertificationAuthorityConfigV1;

mod advance;
mod deadline;
mod evidence;
mod law_lab_eligibility;
mod lifecycle;
mod report;
mod service;
mod structural_frontier_census;

pub(crate) use service::{
    K1EvidenceCursorV1, advance_state, law_lab_eligibility_report_handler,
    mechanism_report_handler, report_handler, summary_handler,
};

use advance::*;
use evidence::*;
#[cfg(test)]
use lifecycle::prepare_tick_context_from_join_ledger;
use lifecycle::{
    AdvanceInput, PreparedK1TickContextV1, advance, extend_prepared_tick_context,
    prepare_tick_context_from_bindings,
};
use report::*;

const K1_RUNTIME_REPORT_SCHEMA_V1: &str = "nando.k1-natural-scheduler-runtime-report.v1";
const K1_SCHEDULER_SCHEMA_V2: &str = "nando.k1-operator-blind-scheduler.v2";
const K1_FIXTURE_EXCLUSION_SCHEMA_V1: &str = "nando.k1-natural-fixture-exclusion.v1";
const K1_SEMANTIC_NOVELTY_SCHEMA_V1: &str = "nando.k1-coarse-semantic-novelty.v1";
const K1_SEMANTIC_QUOTIENT_SCHEMA_V1: &str = "nando.k1-semantic-quotient.v1";
const K1_PROBE_POLICY_SCHEMA_V1: &str = "nando.k1-passive-probe-policy.v1";
const K1_PREDICTION_SCHEMA_V1: &str = "nando.multi-source-t1-passive-outcome-partition.v2";
const K1_MAX_SUPPORT_ROWS_V1: usize = 64;
const K1_MAX_PROBE_ROUNDS_V1: u64 = 8;
const K1_MAX_PROBE_COST_UNITS_V1: u64 = 24;
const K1_MAX_GENERATION_SECONDS_V1: u64 = 86_400;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum K1NaturalSchedulerRuntimeStateV1 {
    WaitingForEvidence,
    CandidateFrozen,
    IdentificationFrozen,
    FuturePredictionContractSealed,
    FuturePredictionCensored,
    FutureOutcomeSettled,
    ProbePending,
    ProbeOutcomeApplied,
    ProbeOutcomeCensored,
    AwaitingIndependentFuture,
    AwaitingCertification,
    TerminalPass,
    TerminalAbstain,
    TerminalAcquisitionFail,
    TerminalIndependentFutureNotObserved,
    TerminalProbeExhausted,
    K1VocabularyOpen,
    MechanismWatchComplete,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct K1NaturalSchedulerRuntimeReportV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub generated_at_unix: u64,
    pub lane: K1SchedulerLaneV1,
    pub state: K1NaturalSchedulerRuntimeStateV1,
    pub blocker: String,
    pub projection: K1SchedulerProjectionV1,
    pub join: MultiSourceJoinReportV1,
    pub catalog: K1NaturalCohortCatalogV1,
    pub queue: K1NaturalCandidateQueueV1,
    pub identification: Option<MultiSourceT1IdentificationV3>,
    pub transfer_lifecycle: Option<K1TransferLifecycleReportV1>,
    pub frozen_evidence_rows: u64,
    pub future_eligible_rows: u64,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct RuntimeReportDigestV1<'a> {
    schema: &'static str,
    generated_at_unix: u64,
    lane: K1SchedulerLaneV1,
    state: K1NaturalSchedulerRuntimeStateV1,
    blocker: &'a str,
    projection_root_sha256: &'a str,
    join: &'a MultiSourceJoinReportV1,
    catalog_root_sha256: &'a str,
    queue_root_sha256: &'a str,
    identification_root_sha256: Option<&'a str>,
    transfer_lifecycle_root_sha256: Option<&'a str>,
    frozen_evidence_rows: u64,
    future_eligible_rows: u64,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}
