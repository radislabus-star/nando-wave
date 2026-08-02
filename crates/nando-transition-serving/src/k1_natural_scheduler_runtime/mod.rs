use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    MultiSourceContainerClassV1, MultiSourceExtractionStatusV1, MultiSourceTypeClassV1,
    RelationFrame, canonical_json_sha256, valid_nonzero_sha256,
};
use nando_operator_learning::multi_source::{
    BlindThenRevealJoinedTransitionV1, CompletedEffectFormV1, FactorizedMultiSourceRowV1,
    K1ConsequenceTypeV1, K1GenerationBudgetV1, K1GenerationTerminalVerdictV1,
    K1GenerationVerdictClassV1, K1IdentificationFreezeV1, K1NaturalCandidateFreezeV1,
    K1NaturalCandidateQueueV1, K1NaturalCohortCatalogV1, K1NaturalEvidenceClassV1,
    K1NaturalEvidenceRowV1, K1ProbeBudgetRemainingV1, K1ProbeClassPredictionV1,
    K1ProbeRoundReceiptV1, K1ProbeRoundStateV1, K1SchedulerEventPayloadV1,
    MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2, MultiSourceJoinLedgerV1, MultiSourceJoinReportV1,
    MultiSourceReasonV1, MultiSourceT1IdentificationStateV1, MultiSourceT1IdentificationV3,
    PassiveT1ProbeContractV1, PreActionShapeClassV1, PreActionTopologyAuditRowV1,
    build_k1_natural_candidate_queue_with_exclusions_v1, build_k1_natural_cohort_catalog_v1,
    factor_multi_source_row_v1, identify_multi_source_t1_operator_with_active_protocols_v1,
};
use serde::{Deserialize, Serialize};

use crate::k1_natural_scheduler::{
    K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V1, K1CandidateFreezeAuthorityRequestV1,
    K1SchedulerProjectionV1, append_candidate_freeze, append_scheduler_payload,
    current_deficit_snapshot, restore_projection,
};
use crate::k1_transfer_lifecycle::K1TransferLifecycleReportV1;
use crate::operator_certification::CertificationAuthorityConfigV1;

mod advance;
mod deadline;
mod evidence;
mod lifecycle;
mod report;
mod service;

pub(crate) use service::{advance_state, report_handler};

use advance::*;
use evidence::*;
use lifecycle::advance;
use report::*;

const K1_RUNTIME_REPORT_SCHEMA_V1: &str = "nando.k1-natural-scheduler-runtime-report.v1";
const K1_SCHEDULER_SCHEMA_V1: &str = "nando.k1-operator-blind-scheduler.v1";
const K1_FIXTURE_EXCLUSION_SCHEMA_V1: &str = "nando.k1-natural-fixture-exclusion.v1";
const K1_SOURCE_NEUTRAL_TOPOLOGY_SCHEMA_V1: &str = "nando.k1-source-neutral-role-graph.v1";
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
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct K1NaturalSchedulerRuntimeReportV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub generated_at_unix: u64,
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
