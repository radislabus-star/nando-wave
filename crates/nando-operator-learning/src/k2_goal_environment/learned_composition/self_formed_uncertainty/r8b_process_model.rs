use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1, valid_composition_path_v1,
};
use super::{
    K2UncertaintyOracleBaselineBatchReceiptV1, K2UncertaintyR8BEvidenceKindV2,
    K2UncertaintyR8BMeasuredReceiptV2, denied_authority_v1, require_denied_authority_v1,
    uncertainty_root_v1,
};

pub const K2_UNCERTAINTY_R8B_EXECUTABLE_MANIFEST_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-executable-manifest.v2";
pub const K2_UNCERTAINTY_R8B_PROCESS_EVENT_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-process-event.v2";
pub const K2_UNCERTAINTY_R8B_PROCESS_LEDGER_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-process-ledger.v2";
pub const K2_UNCERTAINTY_R8B_ROUTE_RECEIPT_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-route-receipt.v2";
pub const K2_UNCERTAINTY_R8B_SUITE_RECEIPT_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-suite-receipt.v2";
pub const K2_UNCERTAINTY_R8B_PACKET_ENTRY_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-packet-entry.v2";
pub const K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-packet-manifest.v2";
pub const K2_UNCERTAINTY_R8B_PACKET_MANIFEST_PATH_V2: &str = "aggregate-manifest.json";
pub const K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_SCHEMA_V2: &str =
    "nando.k2-self-formed-r8b-producer-request.v2";
pub const K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2: &str = "stdout";
pub const K2_UNCERTAINTY_R8B_MAX_PRODUCED_RECEIPTS_V2: usize = 19;
pub const K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-producer-request.v3";
pub const K2_UNCERTAINTY_R8B_PROCESS_EVENT_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-process-event.v3";
pub const K2_UNCERTAINTY_R8B_LEDGER_HEADER_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-ledger-header.v3";
pub const K2_UNCERTAINTY_R8B_LEDGER_SEAL_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-ledger-seal.v3";
pub const K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-packet-manifest.v3";
pub const K2_UNCERTAINTY_R8B_DOWNSTREAM_CONTRACT_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-downstream-contract.v3";
pub const K2_UNCERTAINTY_R8B_SCHEDULE_AUTHORITY_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-schedule-authority.v3";
pub const K2_UNCERTAINTY_R8B_SCHEDULE_FORMULA_V3: &str = "nando.k2-self-formed-r8b-m10-schedule.v3";
pub const K2_UNCERTAINTY_R8B_STATIC_PROJECTION_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-static-projection.v3";
pub const K2_UNCERTAINTY_R8B_ORACLE_WRAPPER_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-oracle-wrapper.v3";
pub const K2_UNCERTAINTY_R8B_CONTROL_WRAPPER_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-control-wrapper.v3";
pub const K2_UNCERTAINTY_R8B_RESOURCE_RECEIPT_SCHEMA_V3: &str =
    "nando.k2-self-formed-r8b-resource-receipt.v3";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyR8BInputRoleV3 {
    DevelopmentSeed,
    FixtureTree,
    LinkedManifest,
    SuiteManifest,
    ProcessLedger,
    ExclusiveOutput,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyR8BObjectRoleV3 {
    Evidence,
    DownstreamInvocationContract,
    ResourceReceipt,
    ProcessLedger,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyR8BLaunchKindV3 {
    Direct,
    StraceMediated,
    BwrapPrlimitMediated,
    UserSystemd,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyR8BToolRoleV3 {
    Strace,
    Bwrap,
    Prlimit,
    SystemdRun,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyR8BExpectedOutcomeV3 {
    AuthoritySuccess,
    DiagnosticExpectedFailure,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BExitPredicateV3 {
    #[serde(rename = "c")]
    pub exact_exit_code: i32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyR8BCompletionKindV3 {
    AuthoritySuccess,
    DiagnosticExpectedFailure,
    UnexpectedFailure,
    LaunchFailure,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyR8BValidatorV3 {
    ConcreteReceipt,
    RepresentativeCount,
    DownstreamInvocationContract,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
pub enum K2UncertaintyR8BValidatedFactV3 {
    None,
    RepresentativeCount { count: u64 },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BInputBindingV3 {
    pub role: K2UncertaintyR8BInputRoleV3,
    pub canonical_path: String,
    pub unix_mode: u32,
    pub byte_len: u64,
    pub content_sha256: String,
    pub semantic_root_sha256: String,
}

#[rustfmt::skip]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BToolIdentityV3 {
    #[serde(rename = "r")] pub role: K2UncertaintyR8BToolRoleV3,
    #[serde(rename = "p")] pub canonical_path: String,
    #[serde(rename = "h")] pub sha256: String,
}

// Compact wire keys keep the complete four-output M24 event below the frozen line budget.
#[rustfmt::skip]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BOutputContractV3 {
    #[serde(rename = "p")] pub relative_path: String,
    #[serde(rename = "o")] pub object_role: K2UncertaintyR8BObjectRoleV3,
    #[serde(rename = "k")] pub evidence_kind: Option<K2UncertaintyR8BEvidenceKindV2>,
    #[serde(rename = "s")] pub receipt_schema: String,
    #[serde(rename = "d")] pub required_denominator: Option<u64>,
    #[serde(rename = "r")] pub required_source_roots_sha256: Vec<String>,
    #[serde(rename = "u")] pub producer_role: String,
    #[serde(rename = "x")] pub producer_executable_sha256: String,
    #[serde(rename = "v")] pub validator: K2UncertaintyR8BValidatorV3,
    #[serde(rename = "a")] pub file_attestation: Option<K2UncertaintyR8BFileAttestationV3>,
}

#[rustfmt::skip]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BFileAttestationV3 {
    #[serde(rename = "b")] pub byte_len: u64,
    #[serde(rename = "m")] pub unix_mode: u32,
    #[serde(rename = "c")] pub content_sha256: String,
    #[serde(rename = "h")] pub semantic_root_sha256: String,
}

#[rustfmt::skip]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BInvocationPlanV3 {
    #[serde(rename = "i")] pub invocation_id_sha256: String,
    #[serde(rename = "p")] pub parent_invocation_id_sha256: Option<String>,
    #[serde(rename = "o")] pub request_owner_role: String,
    #[serde(rename = "x")] pub request_owner_executable_sha256: String,
    #[serde(rename = "t")] pub target_role: String,
    #[serde(rename = "h")] pub target_executable_sha256: String,
    #[serde(rename = "l")] pub launch_kind: K2UncertaintyR8BLaunchKindV3,
    #[serde(rename = "c")] pub tool_chain: Vec<K2UncertaintyR8BToolIdentityV3>,
    #[serde(rename = "s")] pub stage: String,
    #[serde(rename = "a")] pub case_id_sha256: Option<String>,
    #[serde(rename = "n")] pub probe_ordinal: Option<u64>,
    #[serde(rename = "e")] pub expected_outcome: K2UncertaintyR8BExpectedOutcomeV3,
    #[serde(rename = "q", default, skip_serializing_if = "Option::is_none")]
    pub expected_exit_predicate: Option<K2UncertaintyR8BExitPredicateV3>,
    #[serde(rename = "v")] pub validator: K2UncertaintyR8BValidatorV3,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BProducerRequestV3 {
    pub schema: String,
    pub route_id_sha256: String,
    pub producer_role: String,
    pub producer_executable_sha256: String,
    pub test_selector: String,
    pub inputs: Vec<K2UncertaintyR8BInputBindingV3>,
    pub outputs: Vec<K2UncertaintyR8BOutputContractV3>,
    pub invocation_plan: Vec<K2UncertaintyR8BInvocationPlanV3>,
    pub schedule_grammar_root_sha256: String,
    pub request_root_sha256: String,
}

#[rustfmt::skip]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BValidatedOutputV3 {
    #[serde(rename = "b")] pub stdout_byte_len: u64,
    #[serde(rename = "h")] pub stdout_sha256: String,
    #[serde(rename = "s")] pub receipt_schema: String,
    #[serde(rename = "r")] pub semantic_root_sha256: String,
    #[serde(rename = "v")] pub validator: K2UncertaintyR8BValidatorV3,
    #[serde(rename = "x")] pub validator_executable_sha256: String,
    #[serde(rename = "f")] pub fact: K2UncertaintyR8BValidatedFactV3,
    #[serde(rename = "o")] pub authority_outputs: Vec<K2UncertaintyR8BOutputContractV3>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BProcessEventV3 {
    pub schema: String,
    pub sequence: u64,
    pub previous_event_root_sha256: String,
    pub route_id_sha256: String,
    pub invocation: K2UncertaintyR8BInvocationPlanV3,
    pub request_root_sha256: String,
    pub stdin_sha256: String,
    pub started_event_root_sha256: Option<String>,
    pub completion: Option<K2UncertaintyR8BCompletionKindV3>,
    pub exit_code: Option<i32>,
    pub stdout_byte_len: Option<u64>,
    pub stdout_sha256: Option<String>,
    pub stderr_byte_len: Option<u64>,
    pub stderr_sha256: Option<String>,
    pub validated_output: Option<K2UncertaintyR8BValidatedOutputV3>,
    pub monotonic_ns: u64,
    pub event_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BDownstreamContractV3 {
    pub schema: String,
    pub route_id_sha256: String,
    pub schedule_grammar_root_sha256: String,
    pub invocations: Vec<K2UncertaintyR8BInvocationPlanV3>,
    pub projection_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BScheduleAuthorityV3 {
    pub schema: String,
    pub formula: String,
    pub schedule_grammar_root_sha256: String,
    pub case_ids_sha256: Vec<String>,
    pub minimum_representatives: u64,
    pub maximum_representatives: u64,
    pub authority_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BStaticProjectionV3 {
    pub schema: String,
    pub route_id_sha256: String,
    pub schedule_grammar_root_sha256: String,
    pub invocations: Vec<K2UncertaintyR8BInvocationPlanV3>,
    pub producer_request_roots_sha256: BTreeMap<String, String>,
    pub projection_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BLedgerHeaderV3 {
    pub schema: String,
    pub route_id_sha256: String,
    pub expected_projection_root_sha256: String,
    pub schedule_authority: K2UncertaintyR8BScheduleAuthorityV3,
    pub header_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BLedgerSealV3 {
    pub schema: String,
    pub route_id_sha256: String,
    pub event_count: u64,
    pub final_event_root_sha256: String,
    pub seal_root_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2UncertaintyR8BLedgerSummaryV3 {
    pub route_id_sha256: String,
    pub expected_projection_root_sha256: String,
    pub schedule_authority: K2UncertaintyR8BScheduleAuthorityV3,
    pub event_count: u64,
    pub final_event_root_sha256: String,
    pub seal_root_sha256: Option<String>,
    pub invocations: Vec<K2UncertaintyR8BInvocationPlanV3>,
    pub request_roots_sha256: BTreeMap<String, String>,
    pub representative_counts: BTreeMap<String, u64>,
    pub authority_outputs: Vec<(String, K2UncertaintyR8BOutputContractV3)>,
    pub open_invocations: u64,
    pub fail_stopped: bool,
    pub m16_event_roots_sha256: BTreeSet<String>,
    pub m16_receipt_roots_sha256: BTreeSet<String>,
    pub m17_event_roots_sha256: BTreeSet<String>,
    pub m17_receipt_roots_sha256: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BPacketDescriptorV3 {
    pub relative_path: String,
    pub object_role: K2UncertaintyR8BObjectRoleV3,
    pub evidence_kind: Option<K2UncertaintyR8BEvidenceKindV2>,
    pub byte_len: u64,
    pub unix_mode: u32,
    pub content_sha256: String,
    pub semantic_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BPacketManifestV3 {
    pub schema: String,
    pub route_id_sha256: String,
    pub c08_projection_root_sha256: String,
    pub resource_receipt_root_sha256: String,
    pub ledger_seal_root_sha256: String,
    pub ledger_event_count: u64,
    pub m16_completion_event_roots_sha256: Vec<String>,
    pub m16_receipt_roots_sha256: Vec<String>,
    pub m17_completion_event_roots_sha256: Vec<String>,
    pub m17_receipt_roots_sha256: Vec<String>,
    pub members: Vec<K2UncertaintyR8BPacketDescriptorV3>,
    pub manifest_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BOracleWrapperV3 {
    pub schema: String,
    pub batch: K2UncertaintyOracleBaselineBatchReceiptV1,
    pub completion_event_roots_sha256: Vec<String>,
    pub receipt_roots_sha256: Vec<String>,
    pub receipt_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BControlWrapperV3 {
    pub schema: String,
    pub census: K2UncertaintyR8BMeasuredReceiptV2,
    pub completion_event_roots_sha256: Vec<String>,
    pub receipt_roots_sha256: Vec<String>,
    pub receipt_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BManagerIdentityV3 {
    pub bus_peer_pid: u32,
    pub bus_unique_name: String,
    pub pidfd_alive: bool,
    pub boot_id: String,
    pub start_ticks: u64,
    pub uid: u32,
    pub command: Vec<String>,
    pub cgroup: String,
    pub user_unit: String,
    pub invocation_id: String,
    pub main_pid: u32,
    pub exec_start: String,
    pub fragment_path: String,
    pub control_group: String,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BPrivilegedProbeV3 {
    pub sudo_sha256: String,
    pub sha256sum_sha256: String,
    pub argv: Vec<String>,
    pub exit_code: i32,
    pub stdout_byte_len: u64,
    pub stdout_sha256: String,
    pub stderr_byte_len: u64,
    pub stderr_sha256: String,
    pub live_image_sha256: String,
    pub started_monotonic_ns: u64,
    pub finished_monotonic_ns: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BUnitResourceObservationV3 {
    pub unit: String,
    pub invocation_id: String,
    pub main_pid: u32,
    pub exec_main_code: String,
    pub exec_main_status: i32,
    pub active_state: String,
    pub sub_state: String,
    pub metrics_frozen_while_loaded: bool,
    pub memory_peak: u64,
    pub memory_swap_peak: u64,
    pub oom_policy: String,
    pub oom_kills: u64,
    pub tasks_current: u64,
    pub route_started_monotonic_ns: u64,
    pub route_finished_monotonic_ns: u64,
    pub stop_target: String,
    pub stop_exit_code: i32,
    pub inactive_after_stop: bool,
    pub descendants_after_stop: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BResourceReceiptV3 {
    pub schema: String,
    pub route_id_sha256: String,
    pub delegated_launch_request_root_sha256: String,
    pub normalized_systemd_run_argv: Vec<String>,
    pub pinned_systemd_sha256: String,
    pub manager_pre: K2UncertaintyR8BManagerIdentityV3,
    pub manager_post: K2UncertaintyR8BManagerIdentityV3,
    pub probe_pre: K2UncertaintyR8BPrivilegedProbeV3,
    pub probe_post: K2UncertaintyR8BPrivilegedProbeV3,
    pub unit: K2UncertaintyR8BUnitResourceObservationV3,
    pub sudo_frontends: u64,
    pub sha256sum_descendants: u64,
    pub external_network_calls: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyR8BProcessEventKindV2 {
    ChildStarted,
    ChildFinished,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BProducedReceiptV2 {
    pub relative_path: String,
    pub byte_len: u64,
    pub unix_mode: u32,
    pub content_sha256: String,
    pub receipt_schema: String,
    pub semantic_root_sha256: String,
}

impl K2UncertaintyR8BProducedReceiptV2 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.content_sha256)?;
        require_composition_root_v1(&self.semantic_root_sha256)?;
        let stdout = self.relative_path == K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2;
        if (!stdout && !valid_composition_path_v1(&self.relative_path))
            || self.byte_len == 0
            || self.receipt_schema.is_empty()
            || (stdout && self.unix_mode != 0)
            || (!stdout && self.unix_mode != 0o400)
        {
            return Err(invalid("self_formed_r8b_produced_receipt_invalid"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BProcessEventV2 {
    pub schema: String,
    pub sequence: u64,
    pub previous_event_root_sha256: Option<String>,
    pub kind: K2UncertaintyR8BProcessEventKindV2,
    pub route_id_sha256: String,
    pub stage_id: String,
    pub case_id_sha256: Option<String>,
    pub probe_ordinal: Option<u64>,
    pub writer_role: String,
    pub writer_executable_sha256: String,
    pub role: String,
    pub executable_sha256: String,
    pub request_root_sha256: String,
    pub stdin_sha256: String,
    pub started_event_root_sha256: Option<String>,
    pub normal_exit: Option<bool>,
    pub exit_code: Option<i32>,
    pub stdout_byte_len: Option<u64>,
    pub stdout_sha256: Option<String>,
    pub produced_receipts: Vec<K2UncertaintyR8BProducedReceiptV2>,
    pub stderr_byte_len: Option<u64>,
    pub stderr_sha256: Option<String>,
    pub started_monotonic_ns: u64,
    pub finished_monotonic_ns: Option<u64>,
    pub event_root_sha256: String,
}

impl K2UncertaintyR8BProcessEventV2 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.schema = K2_UNCERTAINTY_R8B_PROCESS_EVENT_SCHEMA_V2.to_owned();
        self.event_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            Some(&self.route_id_sha256),
            Some(&self.executable_sha256),
            Some(&self.request_root_sha256),
            Some(&self.stdin_sha256),
            Some(&self.writer_executable_sha256),
            self.previous_event_root_sha256.as_ref(),
            self.started_event_root_sha256.as_ref(),
            self.stdout_sha256.as_ref(),
            self.stderr_sha256.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            require_composition_root_v1(root)?;
        }
        let started = self.kind == K2UncertaintyR8BProcessEventKindV2::ChildStarted;
        let finish_fields = [
            self.normal_exit.is_some(),
            self.exit_code.is_some(),
            self.stdout_byte_len.is_some(),
            self.stdout_sha256.is_some(),
            !self.produced_receipts.is_empty(),
            self.stderr_byte_len.is_some(),
            self.stderr_sha256.is_some(),
            self.finished_monotonic_ns.is_some(),
            self.started_event_root_sha256.is_some(),
        ];
        if self.schema != K2_UNCERTAINTY_R8B_PROCESS_EVENT_SCHEMA_V2
            || self.stage_id.is_empty()
            || self.writer_role.is_empty()
            || self.role.is_empty()
            || (self.sequence == 0) != self.previous_event_root_sha256.is_none()
            || (started && finish_fields.iter().any(|present| *present))
            || (!started && finish_fields.iter().any(|present| !*present))
            || self.produced_receipts.len() > K2_UNCERTAINTY_R8B_MAX_PRODUCED_RECEIPTS_V2
            || !valid_produced_receipts_v2(&self.produced_receipts)
            || self
                .finished_monotonic_ns
                .is_some_and(|finished| finished < self.started_monotonic_ns)
            || self.event_root_sha256 != self.expected_root()?
        {
            return Err(invalid("self_formed_r8b_process_event_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let mut value = self.clone();
        value.event_root_sha256.clear();
        uncertainty_root_v1(&value)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BProcessLedgerV2 {
    pub schema: String,
    pub route_id_sha256: String,
    pub events: Vec<K2UncertaintyR8BProcessEventV2>,
    pub ledger_root_sha256: String,
}

impl K2UncertaintyR8BProcessLedgerV2 {
    pub fn seal(
        route_id_sha256: String,
        events: Vec<K2UncertaintyR8BProcessEventV2>,
    ) -> K2CompositionResultV1<Self> {
        Self::seal_prefix(route_id_sha256, events, true)
    }

    pub fn seal_natural_prefix(
        route_id_sha256: String,
        events: Vec<K2UncertaintyR8BProcessEventV2>,
    ) -> K2CompositionResultV1<Self> {
        Self::seal_prefix(route_id_sha256, events, false)
    }

    fn seal_prefix(
        route_id_sha256: String,
        events: Vec<K2UncertaintyR8BProcessEventV2>,
        require_complete: bool,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_R8B_PROCESS_LEDGER_SCHEMA_V2.to_owned(),
            route_id_sha256,
            events,
            ledger_root_sha256: String::new(),
        };
        value.ledger_root_sha256 = value.expected_root()?;
        value.validate_prefix(require_complete)?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.validate_prefix(true)
    }

    pub fn validate_natural_prefix(&self) -> K2CompositionResultV1<()> {
        self.validate_prefix(false)
    }

    fn validate_prefix(&self, require_complete: bool) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.route_id_sha256)?;
        let mut starts = BTreeMap::new();
        let mut finished = BTreeSet::new();
        let mut previous = None;
        for (sequence, event) in self.events.iter().enumerate() {
            event.validate()?;
            if event.sequence != sequence as u64
                || event.previous_event_root_sha256 != previous
                || event.route_id_sha256 != self.route_id_sha256
            {
                return Err(invalid("self_formed_r8b_process_ledger_chain_invalid"));
            }
            match event.kind {
                K2UncertaintyR8BProcessEventKindV2::ChildStarted => {
                    starts.insert(event.event_root_sha256.clone(), event);
                }
                K2UncertaintyR8BProcessEventKindV2::ChildFinished => {
                    let root = event
                        .started_event_root_sha256
                        .as_ref()
                        .ok_or_else(|| invalid("self_formed_r8b_process_start_missing"))?;
                    let start = starts
                        .get(root)
                        .ok_or_else(|| invalid("self_formed_r8b_process_start_foreign"))?;
                    if !finished.insert(root.clone()) || !same_invocation_v2(start, event) {
                        return Err(invalid("self_formed_r8b_process_finish_invalid"));
                    }
                }
            }
            previous = Some(event.event_root_sha256.clone());
        }
        if self.schema != K2_UNCERTAINTY_R8B_PROCESS_LEDGER_SCHEMA_V2
            || (require_complete && starts.len() != finished.len())
            || self.ledger_root_sha256 != self.expected_root()?
        {
            return Err(invalid("self_formed_r8b_process_ledger_incomplete"));
        }
        Ok(())
    }

    pub fn finished_event(&self, root: &str) -> Option<&K2UncertaintyR8BProcessEventV2> {
        self.events.iter().find(|event| {
            event.kind == K2UncertaintyR8BProcessEventKindV2::ChildFinished
                && event.event_root_sha256 == root
        })
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let mut value = self.clone();
        value.ledger_root_sha256.clear();
        uncertainty_root_v1(&value)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BPacketEntryV2 {
    pub schema: String,
    pub kind: K2UncertaintyR8BEvidenceKindV2,
    pub relative_path: String,
    pub byte_len: u64,
    pub unix_mode: u32,
    pub content_sha256: String,
    pub receipt_schema: String,
    pub semantic_root_sha256: String,
    pub producer_role: String,
    pub producer_executable_sha256: String,
    pub producer_event_root_sha256: String,
    pub route_id_sha256: String,
    pub observed: u64,
    pub disposition: String,
    pub entry_root_sha256: String,
}

impl K2UncertaintyR8BPacketEntryV2 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.schema = K2_UNCERTAINTY_R8B_PACKET_ENTRY_SCHEMA_V2.to_owned();
        self.entry_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.content_sha256,
            &self.semantic_root_sha256,
            &self.producer_executable_sha256,
            &self.producer_event_root_sha256,
            &self.route_id_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        if self.schema != K2_UNCERTAINTY_R8B_PACKET_ENTRY_SCHEMA_V2
            || !valid_composition_path_v1(&self.relative_path)
            || self.relative_path == K2_UNCERTAINTY_R8B_PACKET_MANIFEST_PATH_V2
            || self.byte_len == 0
            || self.unix_mode != 0o400
            || self.receipt_schema != self.kind.expected_schema()
            || self.producer_role.is_empty()
            || self.observed == 0
            || self.disposition != "PASS"
            || self.entry_root_sha256 != self.expected_root()?
        {
            return Err(invalid("self_formed_r8b_packet_entry_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let mut value = self.clone();
        value.entry_root_sha256.clear();
        uncertainty_root_v1(&value)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyR8BPacketManifestV2 {
    pub schema: String,
    pub tested_commit_sha256: String,
    pub route_id_sha256: String,
    pub linked_manifest_root_sha256: String,
    pub suite_manifest_root_sha256: String,
    pub process_ledger: K2UncertaintyR8BProcessLedgerV2,
    pub entries: Vec<K2UncertaintyR8BPacketEntryV2>,
    pub false_accepts: u64,
    pub sealed_attempts: u64,
    pub authorization_slots: u64,
    pub child_network_calls: u64,
    pub production_mutations: u64,
    pub publisher_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub manifest_root_sha256: String,
}

impl K2UncertaintyR8BPacketManifestV2 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.schema = K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V2.to_owned();
        self.authority = denied_authority_v1();
        self.entries.sort_by(|left, right| {
            (left.kind, &left.relative_path).cmp(&(right.kind, &right.relative_path))
        });
        self.manifest_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.tested_commit_sha256,
            &self.route_id_sha256,
            &self.linked_manifest_root_sha256,
            &self.suite_manifest_root_sha256,
            &self.publisher_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.process_ledger.validate()?;
        require_denied_authority_v1(&self.authority)?;
        let mut paths = BTreeSet::new();
        let mut observed = BTreeMap::new();
        for entry in &self.entries {
            entry.validate()?;
            if entry.route_id_sha256 != self.route_id_sha256 || !paths.insert(&entry.relative_path)
            {
                return Err(invalid("self_formed_r8b_packet_entry_set_invalid"));
            }
            *observed.entry(entry.kind).or_insert(0_u64) += entry.observed;
        }
        let kinds = observed.keys().copied().collect::<BTreeSet<_>>();
        let expected = K2UncertaintyR8BEvidenceKindV2::ALL
            .into_iter()
            .collect::<BTreeSet<_>>();
        let denominators_match = observed.iter().all(|(kind, count)| {
            kind.required()
                .map_or(*count > 0, |required| *count == required)
        });
        if self.schema != K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V2
            || self.process_ledger.route_id_sha256 != self.route_id_sha256
            || kinds != expected
            || !denominators_match
            || self.false_accepts != 0
            || self.sealed_attempts != 0
            || self.authorization_slots != 0
            || self.child_network_calls != 0
            || self.production_mutations != 0
            || self.manifest_root_sha256 != self.expected_root()?
        {
            return Err(invalid("self_formed_r8b_packet_manifest_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let mut value = self.clone();
        value.manifest_root_sha256.clear();
        uncertainty_root_v1(&value)
    }
}

fn same_invocation_v2(
    start: &K2UncertaintyR8BProcessEventV2,
    finish: &K2UncertaintyR8BProcessEventV2,
) -> bool {
    start.route_id_sha256 == finish.route_id_sha256
        && start.stage_id == finish.stage_id
        && start.case_id_sha256 == finish.case_id_sha256
        && start.probe_ordinal == finish.probe_ordinal
        && start.writer_role == finish.writer_role
        && start.writer_executable_sha256 == finish.writer_executable_sha256
        && start.role == finish.role
        && start.executable_sha256 == finish.executable_sha256
        && start.request_root_sha256 == finish.request_root_sha256
        && start.stdin_sha256 == finish.stdin_sha256
        && start.started_monotonic_ns == finish.started_monotonic_ns
}

fn valid_produced_receipts_v2(receipts: &[K2UncertaintyR8BProducedReceiptV2]) -> bool {
    let mut paths = BTreeSet::new();
    let mut roots = BTreeSet::new();
    receipts.iter().all(|receipt| {
        receipt.validate().is_ok()
            && paths.insert(receipt.relative_path.as_str())
            && roots.insert(receipt.semantic_root_sha256.as_str())
    })
}

fn invalid(reason: &'static str) -> K2CompositionErrorV1 {
    K2CompositionErrorV1::Invalid(reason)
}
