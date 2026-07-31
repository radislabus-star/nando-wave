use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{RelationFrame, canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::{
    PreActionTopologyAuditRowV1, RepresentationGapClassV1, RequestStructureAuditSnapshotV1,
    TransportBindingFailureV1, TransportBindingLedgerV1, TransportTerminalReceiptV1,
    build_representation_gap_adjudication_report_v1,
};

pub const MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V1: &str =
    "nando.ms3-linked-frame-acquisition-contract.v1";
pub const MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V2: &str =
    "nando.ms3-linked-frame-acquisition-contract.v2";
pub const MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V3: &str =
    "nando.ms3-linked-frame-acquisition-contract.v3";
pub const MS3_LINKED_FRAME_ELIGIBILITY_POLICY_V1: &str =
    "nando.ms3-provider-bound-topology-eligibility.v1";
pub const MS3_LINKED_FRAME_ELIGIBILITY_POLICY_V2: &str =
    "nando.ms3-route-receipt-terminal-settlement.v1";
pub const MS3_LINKED_FRAME_RECEIPT_SCHEMA_V1: &str = "nando.ms3-linked-frame-receipt.v1";
pub const MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V1: &str =
    "nando.ms3-linked-frame-acquisition-report.v1";
pub const MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V2: &str =
    "nando.ms3-linked-frame-acquisition-report.v2";
pub const MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V3: &str =
    "nando.ms3-linked-frame-acquisition-report.v3";
pub const MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V4: &str =
    "nando.ms3-linked-frame-acquisition-report.v4";
pub const MS3_SCIENTIFIC_TOPOLOGY_SETTLEMENT_SCHEMA_V1: &str =
    "nando.ms3-scientific-topology-settlement.v1";
pub const MS3_SCIENTIFIC_DENOMINATOR_RECEIPT_SCHEMA_V1: &str =
    "nando.ms3-scientific-denominator-receipt.v1";
pub const MS3_SCIENTIFIC_DENOMINATOR_ENVELOPE_SCHEMA_V1: &str =
    "nando.ms3-scientific-denominator-envelope.v1";
pub const REPRESENTATION_GAP_CLASSIFIER_VERSION_V1: &str = "nando.representation-gap-classifier.v1";
pub const MS3_LINKED_FRAME_ACQUISITION_FAIL: &str = "MS3_LINKED_FRAME_ACQUISITION_FAIL";
pub const MS3_CENSORED_UNATTRIBUTED_PROBE: &str = "CENSORED_UNATTRIBUTED_PROBE";
pub const MS3_CENSORED_INELIGIBLE_PROBE: &str = "CENSORED_INELIGIBLE_PROBE";
pub const MS3_CENSORED_PRE_ROUTE_RECEIPT_EPOCH: &str = "CENSORED_PRE_ROUTE_RECEIPT_EPOCH";
pub const MS3_RECEIPT_LAG_SLO_SECONDS_V1: u64 = 300;
const MAX_ACQUISITION_TOPOLOGIES: u64 = 4_096;
const MAX_ACQUISITION_SECONDS: u64 = 7 * 24 * 60 * 60;
const MAX_SCIENTIFIC_DENOMINATOR_ENVELOPE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ms3LinkedFrameAcquisitionVerdictV1 {
    Collecting,
    LinkedFrameObserved,
    AcquisitionFail,
    CensoredUnattributedProbe,
    CensoredIneligibleProbe,
    CensoredPreRouteReceiptEpoch,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ms3CandidateSettlementClassV1 {
    SettledEligible,
    TerminalPending,
    RouteFramePending,
    ReceiptStalled,
    StructurallyIneligible,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3LinkedFrameAcquisitionContractV1 {
    pub schema: String,
    pub contract_root_sha256: String,
    pub topology_prefix_root_sha256: String,
    pub topology_watermark_rows: u64,
    pub opened_at_unix: u64,
    pub deadline_unix: u64,
    pub max_new_topology_rows: u64,
    pub classifier_version: String,
    pub authority_ready: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_raw_topology_rows: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_lag_slo_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eligibility_policy: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3LinkedFrameReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub acquisition_contract_root_sha256: String,
    pub topology_commitment_root_sha256: String,
    pub completed_frame_root_sha256: String,
    pub terminal_receipt_root_sha256: String,
    pub transport_binding_root_sha256: String,
    pub session_lineage_sha256: String,
    pub session_id_sha256: String,
    pub turn_intent_id_sha256: String,
    pub request_event_id_sha256: String,
    pub action_event_id_sha256: String,
    pub classifier_version: String,
    pub gap_adjudication_root_sha256: Option<String>,
    pub gap_class: Option<RepresentationGapClassV1>,
    pub phase_update_allowed: bool,
    pub authority_ready: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3LinkedFrameAcquisitionReportV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub acquisition_contract: Ms3LinkedFrameAcquisitionContractV1,
    pub generated_at_unix: u64,
    pub new_topology_rows_seen: u64,
    pub evaluated_topology_rows: u64,
    pub terminal_receipt_rows: u64,
    pub relevant_verified_frame_rows: u64,
    pub linked_frame_rows: u64,
    pub gap_class_counts: BTreeMap<RepresentationGapClassV1, u64>,
    pub no_representation_gap_rows: u64,
    pub receipts: Vec<Ms3LinkedFrameReceiptV1>,
    pub verdict: Ms3LinkedFrameAcquisitionVerdictV1,
    pub blocker: String,
    pub phase_update_allowed: bool,
    pub authority_ready: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub raw_scanned_topology_rows: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub candidate_topology_rows: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub eligible_topology_rows: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub censored_unattributed_rows: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub censored_topology_rows: u64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ineligible_reason_counts: BTreeMap<super::MultiSourceJoinCensoredReasonV1, u64>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub consumed_topology_cursor_rows: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub consumed_capture_sequence: u64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub candidate_settlement_counts: BTreeMap<Ms3CandidateSettlementClassV1, u64>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub route_settlement_pending_rows: u64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub transport_binding_failure_counts: BTreeMap<TransportBindingFailureV1, u64>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub evidence_reuse_excluded_rows: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ms3ScientificDenominatorReconstructionV1 {
    AtomicAtReport,
    AppendOnlyCountEquivalence,
    ReportRootClosure,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3ScientificTopologySettlementV1 {
    pub schema: String,
    pub settlement_root_sha256: String,
    pub acquisition_contract_root_sha256: String,
    pub topology_commitment_root_sha256: String,
    pub terminal_receipt_root_sha256: String,
    pub route_bound_frame_root_sha256: String,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3ScientificDenominatorReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub acquisition_report_root_sha256: String,
    pub acquisition_contract_root_sha256: String,
    pub acquisition_report_schema: String,
    pub topology_watermark_rows: u64,
    pub consumed_topology_cursor_rows: u64,
    pub eligible_topology_rows: u64,
    pub settlements: Vec<Ms3ScientificTopologySettlementV1>,
    pub reconstruction: Ms3ScientificDenominatorReconstructionV1,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3ScientificDenominatorEnvelopeV1 {
    pub schema: String,
    pub envelope_root_sha256: String,
    pub report: Ms3LinkedFrameAcquisitionReportV1,
    pub receipt: Ms3ScientificDenominatorReceiptV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ms3AcquisitionTopologySelectionV1 {
    pub raw_topologies: Vec<PreActionTopologyAuditRowV1>,
    pub candidate_topologies: Vec<PreActionTopologyAuditRowV1>,
    pub eligible_topologies: Vec<PreActionTopologyAuditRowV1>,
    pub ineligible_reason_counts: BTreeMap<super::MultiSourceJoinCensoredReasonV1, u64>,
    pub candidate_settlement_counts: BTreeMap<Ms3CandidateSettlementClassV1, u64>,
}

#[derive(Serialize)]
struct Ms3LinkedFrameAcquisitionReportDigestV2<'a> {
    schema: &'static str,
    acquisition_contract: &'a Ms3LinkedFrameAcquisitionContractV1,
    generated_at_unix: u64,
    new_topology_rows_seen: u64,
    evaluated_topology_rows: u64,
    terminal_receipt_rows: u64,
    relevant_verified_frame_rows: u64,
    linked_frame_rows: u64,
    gap_class_counts: &'a BTreeMap<RepresentationGapClassV1, u64>,
    no_representation_gap_rows: u64,
    receipts: &'a [Ms3LinkedFrameReceiptV1],
    verdict: Ms3LinkedFrameAcquisitionVerdictV1,
    blocker: &'a str,
    phase_update_allowed: bool,
    authority_ready: bool,
    raw_scanned_topology_rows: u64,
    eligible_topology_rows: u64,
    censored_unattributed_rows: u64,
    censored_topology_rows: u64,
    ineligible_reason_counts: &'a BTreeMap<super::MultiSourceJoinCensoredReasonV1, u64>,
    consumed_topology_cursor_rows: u64,
    consumed_capture_sequence: u64,
}

#[derive(Serialize)]
struct Ms3LinkedFrameAcquisitionReportDigestV3<'a> {
    schema: &'static str,
    acquisition_contract: &'a Ms3LinkedFrameAcquisitionContractV1,
    generated_at_unix: u64,
    new_topology_rows_seen: u64,
    evaluated_topology_rows: u64,
    terminal_receipt_rows: u64,
    relevant_verified_frame_rows: u64,
    linked_frame_rows: u64,
    gap_class_counts: &'a BTreeMap<RepresentationGapClassV1, u64>,
    no_representation_gap_rows: u64,
    receipts: &'a [Ms3LinkedFrameReceiptV1],
    verdict: Ms3LinkedFrameAcquisitionVerdictV1,
    blocker: &'a str,
    phase_update_allowed: bool,
    authority_ready: bool,
    raw_scanned_topology_rows: u64,
    candidate_topology_rows: u64,
    eligible_topology_rows: u64,
    censored_unattributed_rows: u64,
    censored_topology_rows: u64,
    ineligible_reason_counts: &'a BTreeMap<super::MultiSourceJoinCensoredReasonV1, u64>,
    consumed_topology_cursor_rows: u64,
    consumed_capture_sequence: u64,
}

#[derive(Serialize)]
struct Ms3LinkedFrameAcquisitionReportDigestV4<'a> {
    schema: &'static str,
    acquisition_contract: &'a Ms3LinkedFrameAcquisitionContractV1,
    generated_at_unix: u64,
    new_topology_rows_seen: u64,
    evaluated_topology_rows: u64,
    terminal_receipt_rows: u64,
    relevant_verified_frame_rows: u64,
    linked_frame_rows: u64,
    gap_class_counts: &'a BTreeMap<RepresentationGapClassV1, u64>,
    no_representation_gap_rows: u64,
    receipts: &'a [Ms3LinkedFrameReceiptV1],
    verdict: Ms3LinkedFrameAcquisitionVerdictV1,
    blocker: &'a str,
    phase_update_allowed: bool,
    authority_ready: bool,
    raw_scanned_topology_rows: u64,
    candidate_topology_rows: u64,
    eligible_topology_rows: u64,
    censored_unattributed_rows: u64,
    censored_topology_rows: u64,
    ineligible_reason_counts: &'a BTreeMap<super::MultiSourceJoinCensoredReasonV1, u64>,
    consumed_topology_cursor_rows: u64,
    consumed_capture_sequence: u64,
    candidate_settlement_counts: &'a BTreeMap<Ms3CandidateSettlementClassV1, u64>,
    route_settlement_pending_rows: u64,
    transport_binding_failure_counts: &'a BTreeMap<TransportBindingFailureV1, u64>,
    evidence_reuse_excluded_rows: u64,
}

impl Ms3LinkedFrameAcquisitionContractV1 {
    pub fn seal(
        topology_prefix_root_sha256: String,
        topology_watermark_rows: u64,
        opened_at_unix: u64,
        max_new_topology_rows: u64,
        max_elapsed_seconds: u64,
    ) -> Result<Self, &'static str> {
        if !valid_nonzero_sha256(&topology_prefix_root_sha256)
            || opened_at_unix == 0
            || !(1..=MAX_ACQUISITION_TOPOLOGIES).contains(&max_new_topology_rows)
            || !(60..=MAX_ACQUISITION_SECONDS).contains(&max_elapsed_seconds)
        {
            return Err("ms3_linked_frame_acquisition_contract_invalid");
        }
        let deadline_unix = opened_at_unix
            .checked_add(max_elapsed_seconds)
            .ok_or("ms3_linked_frame_acquisition_contract_invalid")?;
        let mut contract = Self {
            schema: MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V1.to_owned(),
            contract_root_sha256: String::new(),
            topology_prefix_root_sha256,
            topology_watermark_rows,
            opened_at_unix,
            deadline_unix,
            max_new_topology_rows,
            classifier_version: REPRESENTATION_GAP_CLASSIFIER_VERSION_V1.to_owned(),
            authority_ready: false,
            max_raw_topology_rows: None,
            receipt_lag_slo_seconds: None,
            eligibility_policy: None,
        };
        contract.contract_root_sha256 = contract.expected_root();
        contract
            .validate()
            .then_some(contract)
            .ok_or("ms3_linked_frame_acquisition_contract_invalid")
    }

    pub fn seal_v2(
        topology_prefix_root_sha256: String,
        topology_watermark_rows: u64,
        opened_at_unix: u64,
        max_eligible_topology_rows: u64,
        max_raw_topology_rows: u64,
        max_elapsed_seconds: u64,
        receipt_lag_slo_seconds: u64,
    ) -> Result<Self, &'static str> {
        if !valid_nonzero_sha256(&topology_prefix_root_sha256)
            || opened_at_unix == 0
            || !(1..=MAX_ACQUISITION_TOPOLOGIES).contains(&max_eligible_topology_rows)
            || !(max_eligible_topology_rows..=MAX_ACQUISITION_TOPOLOGIES)
                .contains(&max_raw_topology_rows)
            || !(60..=MAX_ACQUISITION_SECONDS).contains(&max_elapsed_seconds)
            || receipt_lag_slo_seconds == 0
            || receipt_lag_slo_seconds > max_elapsed_seconds
        {
            return Err("ms3_linked_frame_acquisition_contract_invalid");
        }
        let deadline_unix = opened_at_unix
            .checked_add(max_elapsed_seconds)
            .ok_or("ms3_linked_frame_acquisition_contract_invalid")?;
        let mut contract = Self {
            schema: MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V2.to_owned(),
            contract_root_sha256: String::new(),
            topology_prefix_root_sha256,
            topology_watermark_rows,
            opened_at_unix,
            deadline_unix,
            max_new_topology_rows: max_eligible_topology_rows,
            classifier_version: REPRESENTATION_GAP_CLASSIFIER_VERSION_V1.to_owned(),
            authority_ready: false,
            max_raw_topology_rows: Some(max_raw_topology_rows),
            receipt_lag_slo_seconds: Some(receipt_lag_slo_seconds),
            eligibility_policy: None,
        };
        contract.contract_root_sha256 = contract.expected_root();
        contract
            .validate()
            .then_some(contract)
            .ok_or("ms3_linked_frame_acquisition_contract_invalid")
    }

    pub fn seal_v3(
        topology_prefix_root_sha256: String,
        topology_watermark_rows: u64,
        opened_at_unix: u64,
        max_eligible_topology_rows: u64,
        max_raw_topology_rows: u64,
        max_elapsed_seconds: u64,
        receipt_lag_slo_seconds: u64,
    ) -> Result<Self, &'static str> {
        if !valid_nonzero_sha256(&topology_prefix_root_sha256)
            || opened_at_unix == 0
            || !(1..=MAX_ACQUISITION_TOPOLOGIES).contains(&max_eligible_topology_rows)
            || !(max_eligible_topology_rows..=MAX_ACQUISITION_TOPOLOGIES)
                .contains(&max_raw_topology_rows)
            || !(60..=MAX_ACQUISITION_SECONDS).contains(&max_elapsed_seconds)
            || receipt_lag_slo_seconds == 0
            || receipt_lag_slo_seconds > max_elapsed_seconds
        {
            return Err("ms3_linked_frame_acquisition_contract_invalid");
        }
        let deadline_unix = opened_at_unix
            .checked_add(max_elapsed_seconds)
            .ok_or("ms3_linked_frame_acquisition_contract_invalid")?;
        let mut contract = Self {
            schema: MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V3.to_owned(),
            contract_root_sha256: String::new(),
            topology_prefix_root_sha256,
            topology_watermark_rows,
            opened_at_unix,
            deadline_unix,
            max_new_topology_rows: max_eligible_topology_rows,
            classifier_version: REPRESENTATION_GAP_CLASSIFIER_VERSION_V1.to_owned(),
            authority_ready: false,
            max_raw_topology_rows: Some(max_raw_topology_rows),
            receipt_lag_slo_seconds: Some(receipt_lag_slo_seconds),
            eligibility_policy: Some(MS3_LINKED_FRAME_ELIGIBILITY_POLICY_V2.to_owned()),
        };
        contract.contract_root_sha256 = contract.expected_root();
        contract
            .validate()
            .then_some(contract)
            .ok_or("ms3_linked_frame_acquisition_contract_invalid")
    }

    #[must_use]
    pub fn max_elapsed_seconds(&self) -> u64 {
        self.deadline_unix.saturating_sub(self.opened_at_unix)
    }

    #[must_use]
    pub fn max_raw_topology_rows(&self) -> u64 {
        self.max_raw_topology_rows
            .unwrap_or(self.max_new_topology_rows)
    }

    #[must_use]
    pub fn receipt_lag_slo_seconds(&self) -> u64 {
        self.receipt_lag_slo_seconds
            .unwrap_or(MS3_RECEIPT_LAG_SLO_SECONDS_V1)
    }

    #[must_use]
    pub fn expected_root(&self) -> String {
        if self.schema == MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V3 {
            canonical_json_sha256(&(
                MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V3,
                self.topology_prefix_root_sha256.as_str(),
                self.topology_watermark_rows,
                self.opened_at_unix,
                self.deadline_unix,
                self.max_new_topology_rows,
                self.max_raw_topology_rows(),
                self.receipt_lag_slo_seconds(),
                self.classifier_version.as_str(),
                self.eligibility_policy.as_deref(),
                false,
            ))
            .expect("MS3 acquisition contract serializes")
        } else if self.schema == MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V2 {
            canonical_json_sha256(&(
                MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V2,
                self.topology_prefix_root_sha256.as_str(),
                self.topology_watermark_rows,
                self.opened_at_unix,
                self.deadline_unix,
                self.max_new_topology_rows,
                self.max_raw_topology_rows(),
                self.receipt_lag_slo_seconds(),
                self.classifier_version.as_str(),
                false,
            ))
            .expect("MS3 acquisition contract serializes")
        } else {
            canonical_json_sha256(&(
                MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V1,
                self.topology_prefix_root_sha256.as_str(),
                self.topology_watermark_rows,
                self.opened_at_unix,
                self.deadline_unix,
                self.max_new_topology_rows,
                self.classifier_version.as_str(),
                false,
            ))
            .expect("MS3 acquisition contract serializes")
        }
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        matches!(
            self.schema.as_str(),
            MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V1
                | MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V2
                | MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V3
        ) && valid_nonzero_sha256(&self.contract_root_sha256)
            && valid_nonzero_sha256(&self.topology_prefix_root_sha256)
            && self.opened_at_unix > 0
            && self.deadline_unix > self.opened_at_unix
            && self.max_elapsed_seconds() <= MAX_ACQUISITION_SECONDS
            && (1..=MAX_ACQUISITION_TOPOLOGIES).contains(&self.max_new_topology_rows)
            && (self.schema == MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V1
                && self.max_raw_topology_rows.is_none()
                && self.receipt_lag_slo_seconds.is_none()
                && self.eligibility_policy.is_none()
                || self.schema == MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V2
                    && self.max_raw_topology_rows.is_some_and(|rows| {
                        (self.max_new_topology_rows..=MAX_ACQUISITION_TOPOLOGIES).contains(&rows)
                    })
                    && self.receipt_lag_slo_seconds.is_some_and(|seconds| {
                        seconds > 0 && seconds <= self.max_elapsed_seconds()
                    })
                    && self.eligibility_policy.is_none()
                || self.schema == MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V3
                    && self.max_raw_topology_rows.is_some_and(|rows| {
                        (self.max_new_topology_rows..=MAX_ACQUISITION_TOPOLOGIES).contains(&rows)
                    })
                    && self.receipt_lag_slo_seconds.is_some_and(|seconds| {
                        seconds > 0 && seconds <= self.max_elapsed_seconds()
                    })
                    && matches!(
                        self.eligibility_policy.as_deref(),
                        Some(
                            MS3_LINKED_FRAME_ELIGIBILITY_POLICY_V1
                                | MS3_LINKED_FRAME_ELIGIBILITY_POLICY_V2
                        )
                    ))
            && self.classifier_version == REPRESENTATION_GAP_CLASSIFIER_VERSION_V1
            && !self.authority_ready
            && self.contract_root_sha256 == self.expected_root()
    }

    #[must_use]
    pub fn uses_route_settlement_policy(&self) -> bool {
        self.schema == MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V3
            && self.eligibility_policy.as_deref() == Some(MS3_LINKED_FRAME_ELIGIBILITY_POLICY_V2)
    }
}

#[must_use]
pub fn build_ms3_linked_frame_acquisition_report_v1(
    contract: Ms3LinkedFrameAcquisitionContractV1,
    generated_at_unix: u64,
    new_topologies: Vec<PreActionTopologyAuditRowV1>,
    frames: Vec<RelationFrame>,
    terminals: Vec<TransportTerminalReceiptV1>,
) -> Ms3LinkedFrameAcquisitionReportV1 {
    build_ms3_linked_frame_acquisition_report_excluding_used_evidence_v1(
        contract,
        generated_at_unix,
        new_topologies,
        frames,
        terminals,
        &BTreeSet::new(),
    )
}

#[must_use]
pub fn build_ms3_linked_frame_acquisition_report_excluding_used_evidence_v1(
    contract: Ms3LinkedFrameAcquisitionContractV1,
    generated_at_unix: u64,
    new_topologies: Vec<PreActionTopologyAuditRowV1>,
    frames: Vec<RelationFrame>,
    terminals: Vec<TransportTerminalReceiptV1>,
    used_evidence_roots: &BTreeSet<String>,
) -> Ms3LinkedFrameAcquisitionReportV1 {
    build_ms3_linked_frame_acquisition_report_with_route_bound_evidence_v1(
        contract,
        generated_at_unix,
        new_topologies,
        frames,
        terminals,
        used_evidence_roots,
        &BTreeSet::new(),
    )
}

#[must_use]
pub fn build_ms3_linked_frame_acquisition_report_with_route_bound_evidence_v1(
    contract: Ms3LinkedFrameAcquisitionContractV1,
    generated_at_unix: u64,
    new_topologies: Vec<PreActionTopologyAuditRowV1>,
    mut frames: Vec<RelationFrame>,
    mut terminals: Vec<TransportTerminalReceiptV1>,
    used_evidence_roots: &BTreeSet<String>,
    route_bound_frame_roots: &BTreeSet<String>,
) -> Ms3LinkedFrameAcquisitionReportV1 {
    let new_topology_rows_seen = u64::try_from(new_topologies.len()).unwrap_or(u64::MAX);
    let deadline_nanos = contract.deadline_unix.saturating_mul(1_000_000_000);
    terminals
        .retain(|receipt| receipt.validate() && receipt.completed_at_unix_nanos <= deadline_nanos);
    let Ms3AcquisitionTopologySelectionV1 {
        raw_topologies,
        candidate_topologies,
        eligible_topologies,
        ineligible_reason_counts,
        candidate_settlement_counts,
    } = select_ms3_linked_frame_acquisition_topologies_with_route_bound_evidence_v1(
        &contract,
        generated_at_unix,
        new_topologies,
        &terminals,
        &frames,
        route_bound_frame_roots,
    );
    let terminal_request_ids = terminals
        .iter()
        .map(|receipt| receipt.request_event_id_sha256.clone())
        .collect::<BTreeSet<_>>();
    let request_ids = candidate_topologies
        .iter()
        .map(|row| row.structure.request_event_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let intent_ids = candidate_topologies
        .iter()
        .map(|row| row.structure.turn_intent_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    frames.retain(|frame| {
        intent_ids.contains(frame.client_intent_id_sha256.as_str())
            && frame.observed_at_unix_nanos <= deadline_nanos
            && (!contract.uses_route_settlement_policy()
                || canonical_json_sha256(frame)
                    .is_ok_and(|root| route_bound_frame_roots.contains(&root)))
    });
    terminals.retain(|receipt| {
        request_ids.contains(receipt.request_event_id_sha256.as_str())
            && receipt.completed_at_unix_nanos <= deadline_nanos
    });
    let ledger = TransportBindingLedgerV1::build(&candidate_topologies, &frames, &terminals);
    let transport_binding_failure_counts = ledger.failure_counts();
    let gap_report = build_representation_gap_adjudication_report_v1(
        request_snapshot(eligible_topologies.clone()),
        frames.clone(),
        terminals.clone(),
    );
    let gaps_by_binding = gap_report
        .rows
        .iter()
        .map(|row| (row.transport_binding_root_sha256.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut receipts = Vec::new();
    let mut evidence_reuse_excluded_rows = 0_u64;
    for topology in &eligible_topologies {
        for bound in ledger.bound_for_topology(&topology.commit.commitment_root_sha256) {
            if [
                bound.binding.topology_commitment_root_sha256.as_str(),
                bound.binding.completed_frame_root_sha256.as_str(),
                bound.binding.session_lineage_sha256.as_str(),
            ]
            .iter()
            .any(|root| used_evidence_roots.contains(*root))
            {
                evidence_reuse_excluded_rows = evidence_reuse_excluded_rows.saturating_add(1);
                continue;
            }
            let gap = gaps_by_binding.get(bound.binding.binding_root_sha256.as_str());
            receipts.push(Ms3LinkedFrameReceiptV1::seal(
                &contract,
                bound,
                gap.copied(),
            ));
        }
    }
    receipts.sort_by(|left, right| {
        left.topology_commitment_root_sha256
            .cmp(&right.topology_commitment_root_sha256)
            .then_with(|| {
                left.completed_frame_root_sha256
                    .cmp(&right.completed_frame_root_sha256)
            })
    });
    let gap_class_counts = receipts
        .iter()
        .filter_map(|receipt| receipt.gap_class)
        .fold(
            BTreeMap::<RepresentationGapClassV1, u64>::new(),
            |mut counts, class| {
                *counts.entry(class).or_default() += 1;
                counts
            },
        );
    let no_representation_gap_rows = u64::try_from(
        receipts
            .iter()
            .filter(|receipt| receipt.gap_class.is_none())
            .count(),
    )
    .unwrap_or(u64::MAX);
    let linked_frame_rows = u64::try_from(receipts.len()).unwrap_or(u64::MAX);
    let raw_scanned_topology_rows = u64::try_from(raw_topologies.len()).unwrap_or(u64::MAX);
    let candidate_topology_rows = u64::try_from(candidate_topologies.len()).unwrap_or(u64::MAX);
    let eligible_topology_rows = u64::try_from(eligible_topologies.len()).unwrap_or(u64::MAX);
    let evaluated_topology_rows = eligible_topology_rows;
    let terminal_receipt_rows = u64::try_from(terminals.len()).unwrap_or(u64::MAX);
    let censored_unattributed_rows = ineligible_reason_counts
        .get(&super::MultiSourceJoinCensoredReasonV1::ProviderIdentityUnproven)
        .copied()
        .unwrap_or(0);
    let censored_topology_rows = ineligible_reason_counts
        .get(&super::MultiSourceJoinCensoredReasonV1::TopologyCensored)
        .copied()
        .unwrap_or(0);
    let censored_topology_rows = censored_topology_rows.saturating_add(
        ineligible_reason_counts
            .get(&super::MultiSourceJoinCensoredReasonV1::TerminalReceiptUnavailable)
            .copied()
            .unwrap_or(0),
    );
    let ineligible_rows = raw_scanned_topology_rows.saturating_sub(candidate_topology_rows);
    let operationally_censored_rows =
        censored_unattributed_rows.saturating_add(censored_topology_rows);
    let stale_operationally_censored_rows = u64::try_from(
        raw_topologies
            .iter()
            .filter(|topology| {
                match acquisition_topology_eligibility(
                    &contract,
                    topology,
                    &terminal_request_ids,
                    generated_at_unix,
                    contract.receipt_lag_slo_seconds(),
                ) {
                    Err(super::MultiSourceJoinCensoredReasonV1::TerminalReceiptUnavailable) => true,
                    Err(
                        super::MultiSourceJoinCensoredReasonV1::ProviderIdentityUnproven
                        | super::MultiSourceJoinCensoredReasonV1::TopologyCensored,
                    ) => topology.captured_at_unix_ms.is_some_and(|captured_at| {
                        generated_at_unix
                            .saturating_mul(1_000)
                            .saturating_sub(captured_at)
                            >= contract.receipt_lag_slo_seconds().saturating_mul(1_000)
                    }),
                    _ => false,
                }
            })
            .count(),
    )
    .unwrap_or(u64::MAX);
    let all_eligible_terminal = terminal_receipt_rows >= eligible_topology_rows;
    let eligible_budget_exhausted = eligible_topology_rows == contract.max_new_topology_rows;
    let raw_budget_exhausted = raw_scanned_topology_rows == contract.max_raw_topology_rows();
    let operational_ineligible_censor = !eligible_budget_exhausted
        && raw_budget_exhausted
        && operationally_censored_rows > 0
        && operationally_censored_rows == ineligible_rows
        && stale_operationally_censored_rows == operationally_censored_rows
        && all_eligible_terminal;
    let exhausted = if contract.uses_route_settlement_policy() {
        generated_at_unix >= contract.deadline_unix
            || eligible_budget_exhausted
            || raw_budget_exhausted
    } else {
        (generated_at_unix >= contract.deadline_unix || eligible_budget_exhausted)
            && all_eligible_terminal
    };
    let (verdict, blocker) = if linked_frame_rows > 0 {
        (
            Ms3LinkedFrameAcquisitionVerdictV1::LinkedFrameObserved,
            String::new(),
        )
    } else if operational_ineligible_censor {
        if censored_topology_rows > 0 {
            (
                Ms3LinkedFrameAcquisitionVerdictV1::CensoredIneligibleProbe,
                MS3_CENSORED_INELIGIBLE_PROBE.to_owned(),
            )
        } else {
            (
                Ms3LinkedFrameAcquisitionVerdictV1::CensoredUnattributedProbe,
                MS3_CENSORED_UNATTRIBUTED_PROBE.to_owned(),
            )
        }
    } else if exhausted {
        (
            Ms3LinkedFrameAcquisitionVerdictV1::AcquisitionFail,
            MS3_LINKED_FRAME_ACQUISITION_FAIL.to_owned(),
        )
    } else {
        (
            Ms3LinkedFrameAcquisitionVerdictV1::Collecting,
            if contract.uses_route_settlement_policy()
                && candidate_settlement_counts
                    .get(&Ms3CandidateSettlementClassV1::ReceiptStalled)
                    .copied()
                    .unwrap_or(0)
                    > 0
            {
                "route_settlement_stalled".to_owned()
            } else if contract.uses_route_settlement_policy()
                && candidate_topology_rows > eligible_topology_rows
            {
                "route_settlement_pending".to_owned()
            } else if generated_at_unix >= contract.deadline_unix
                && terminal_receipt_rows < eligible_topology_rows
            {
                "terminal_receipt_stalled".to_owned()
            } else {
                "linked_frame_pending".to_owned()
            },
        )
    };
    let consumed_topology_cursor_rows = contract
        .topology_watermark_rows
        .saturating_add(raw_scanned_topology_rows);
    let consumed_capture_sequence = raw_topologies
        .iter()
        .filter_map(|row| row.bridge_sequence)
        .max()
        .unwrap_or(0);
    let serialized_candidate_topology_rows = if contract.uses_route_settlement_policy() {
        candidate_topology_rows
    } else {
        0
    };
    let route_settlement_pending_rows = if contract.uses_route_settlement_policy() {
        candidate_topology_rows.saturating_sub(eligible_topology_rows)
    } else {
        0
    };
    let serialized_transport_binding_failure_counts = if contract.uses_route_settlement_policy() {
        transport_binding_failure_counts
    } else {
        BTreeMap::new()
    };
    let serialized_evidence_reuse_excluded_rows = if contract.uses_route_settlement_policy() {
        evidence_reuse_excluded_rows
    } else {
        0
    };
    let mut report = Ms3LinkedFrameAcquisitionReportV1 {
        schema: if contract.uses_route_settlement_policy() {
            MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V4
        } else {
            MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V2
        }
        .to_owned(),
        report_root_sha256: String::new(),
        acquisition_contract: contract,
        generated_at_unix,
        new_topology_rows_seen,
        evaluated_topology_rows,
        terminal_receipt_rows,
        relevant_verified_frame_rows: u64::try_from(frames.len()).unwrap_or(u64::MAX),
        linked_frame_rows,
        gap_class_counts,
        no_representation_gap_rows,
        receipts,
        verdict,
        blocker,
        phase_update_allowed: false,
        authority_ready: false,
        raw_scanned_topology_rows,
        candidate_topology_rows: serialized_candidate_topology_rows,
        eligible_topology_rows,
        censored_unattributed_rows,
        censored_topology_rows,
        ineligible_reason_counts,
        consumed_topology_cursor_rows,
        consumed_capture_sequence,
        candidate_settlement_counts,
        route_settlement_pending_rows,
        transport_binding_failure_counts: serialized_transport_binding_failure_counts,
        evidence_reuse_excluded_rows: serialized_evidence_reuse_excluded_rows,
    };
    report.report_root_sha256 = report.expected_root();
    report
}

pub fn close_ms3_pre_route_receipt_epoch_v1(
    mut report: Ms3LinkedFrameAcquisitionReportV1,
) -> Result<Ms3LinkedFrameAcquisitionReportV1, &'static str> {
    let pre_route_rows = report
        .ineligible_reason_counts
        .get(&super::MultiSourceJoinCensoredReasonV1::TerminalReceiptUnavailable)
        .copied()
        .unwrap_or(0);
    if !report.validate()
        || report.acquisition_contract.schema != MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V2
        || report.verdict != Ms3LinkedFrameAcquisitionVerdictV1::Collecting
        || report.linked_frame_rows != 0
        || pre_route_rows == 0
        || pre_route_rows > report.censored_topology_rows
    {
        return Err("ms3_pre_route_receipt_epoch_censor_invalid");
    }
    report.schema = MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V3.to_owned();
    report.verdict = Ms3LinkedFrameAcquisitionVerdictV1::CensoredPreRouteReceiptEpoch;
    report.blocker = MS3_CENSORED_PRE_ROUTE_RECEIPT_EPOCH.to_owned();
    report.report_root_sha256 = report.expected_root();
    report
        .validate()
        .then_some(report)
        .ok_or("ms3_pre_route_receipt_epoch_censor_invalid")
}

fn request_snapshot(
    topologies: Vec<PreActionTopologyAuditRowV1>,
) -> RequestStructureAuditSnapshotV1 {
    RequestStructureAuditSnapshotV1 {
        rows: Vec::new(),
        stored_turns: u64::try_from(topologies.len()).unwrap_or(u64::MAX),
        stored_topologies: u64::try_from(topologies.len()).unwrap_or(u64::MAX),
        topologies,
        evictions: 0,
        provider_bound_by_construction: true,
        pre_action_context_persisted: true,
    }
}

#[must_use]
pub fn select_ms3_linked_frame_acquisition_topologies_v1(
    contract: &Ms3LinkedFrameAcquisitionContractV1,
    generated_at_unix: u64,
    new_topologies: Vec<PreActionTopologyAuditRowV1>,
    terminals: &[TransportTerminalReceiptV1],
) -> Ms3AcquisitionTopologySelectionV1 {
    select_ms3_linked_frame_acquisition_topologies_with_route_bound_evidence_v1(
        contract,
        generated_at_unix,
        new_topologies,
        terminals,
        &[],
        &BTreeSet::new(),
    )
}

#[must_use]
pub fn select_ms3_linked_frame_acquisition_topologies_with_route_bound_evidence_v1(
    contract: &Ms3LinkedFrameAcquisitionContractV1,
    generated_at_unix: u64,
    new_topologies: Vec<PreActionTopologyAuditRowV1>,
    terminals: &[TransportTerminalReceiptV1],
    frames: &[RelationFrame],
    route_bound_frame_roots: &BTreeSet<String>,
) -> Ms3AcquisitionTopologySelectionV1 {
    let deadline_nanos = contract.deadline_unix.saturating_mul(1_000_000_000);
    let terminal_request_ids = terminals
        .iter()
        .filter(|receipt| receipt.validate() && receipt.completed_at_unix_nanos <= deadline_nanos)
        .map(|receipt| receipt.request_event_id_sha256.clone())
        .collect::<BTreeSet<_>>();
    let route_bound_frames = frames
        .iter()
        .filter(|frame| {
            frame.observed_at_unix_nanos <= deadline_nanos
                && canonical_json_sha256(*frame)
                    .is_ok_and(|root| route_bound_frame_roots.contains(&root))
        })
        .collect::<Vec<_>>();
    let mut raw_topologies = Vec::new();
    let mut candidate_topologies = Vec::new();
    let mut eligible_topologies = Vec::new();
    let mut ineligible_reason_counts =
        BTreeMap::<super::MultiSourceJoinCensoredReasonV1, u64>::new();
    let mut candidate_settlement_counts = BTreeMap::<Ms3CandidateSettlementClassV1, u64>::new();
    for topology in new_topologies {
        if raw_topologies.len()
            >= usize::try_from(contract.max_raw_topology_rows()).unwrap_or(usize::MAX)
            || eligible_topologies.len()
                >= usize::try_from(contract.max_new_topology_rows).unwrap_or(usize::MAX)
        {
            break;
        }
        if contract.uses_route_settlement_policy() {
            match super::validate_pre_action_topology_join_eligibility_v1(&topology) {
                Ok(()) => {
                    candidate_topologies.push(topology.clone());
                    let terminal_present =
                        terminal_request_ids.contains(&topology.structure.request_event_id_sha256);
                    let route_frame_present = route_bound_frames
                        .iter()
                        .any(|frame| route_bound_frame_matches_topology(&topology, frame));
                    let settlement_class = if terminal_present && route_frame_present {
                        eligible_topologies.push(topology.clone());
                        Ms3CandidateSettlementClassV1::SettledEligible
                    } else if topology.captured_at_unix_ms.is_some_and(|captured_at| {
                        generated_at_unix
                            .saturating_mul(1_000)
                            .saturating_sub(captured_at)
                            >= contract.receipt_lag_slo_seconds().saturating_mul(1_000)
                    }) {
                        Ms3CandidateSettlementClassV1::ReceiptStalled
                    } else if !terminal_present {
                        Ms3CandidateSettlementClassV1::TerminalPending
                    } else {
                        Ms3CandidateSettlementClassV1::RouteFramePending
                    };
                    *candidate_settlement_counts
                        .entry(settlement_class)
                        .or_default() += 1;
                }
                Err(reason) => {
                    *ineligible_reason_counts.entry(reason).or_default() += 1;
                    *candidate_settlement_counts
                        .entry(Ms3CandidateSettlementClassV1::StructurallyIneligible)
                        .or_default() += 1;
                }
            }
        } else {
            match acquisition_topology_eligibility(
                contract,
                &topology,
                &terminal_request_ids,
                generated_at_unix,
                contract.receipt_lag_slo_seconds(),
            ) {
                Ok(()) => {
                    candidate_topologies.push(topology.clone());
                    eligible_topologies.push(topology.clone());
                }
                Err(reason) => {
                    *ineligible_reason_counts.entry(reason).or_default() += 1;
                }
            }
        }
        raw_topologies.push(topology);
    }
    Ms3AcquisitionTopologySelectionV1 {
        raw_topologies,
        candidate_topologies,
        eligible_topologies,
        ineligible_reason_counts,
        candidate_settlement_counts,
    }
}

fn route_bound_frame_matches_topology(
    topology: &PreActionTopologyAuditRowV1,
    frame: &RelationFrame,
) -> bool {
    topology.structure.turn_intent_id_sha256 == frame.client_intent_id_sha256
        && topology.captured_at_unix_ms.is_some_and(|captured_at| {
            captured_at.saturating_mul(1_000_000) <= frame.observed_at_unix_nanos
        })
        && (topology.structure.session_lineage_roots_sha256.is_empty()
            || topology
                .structure
                .session_lineage_roots_sha256
                .contains(&frame.session_id_sha256))
}

pub fn build_ms3_scientific_denominator_receipt_v1(
    report: &Ms3LinkedFrameAcquisitionReportV1,
    topologies: &[PreActionTopologyAuditRowV1],
    frames: &[RelationFrame],
    terminals: &[TransportTerminalReceiptV1],
    route_bound_frame_roots: &BTreeSet<String>,
    reconstruction: Ms3ScientificDenominatorReconstructionV1,
) -> Result<Ms3ScientificDenominatorReceiptV1, &'static str> {
    if !report.validate()
        || !report.is_terminal()
        || !report.acquisition_contract.uses_route_settlement_policy()
    {
        return Err("ms3_scientific_denominator_report_invalid");
    }
    let selection = select_ms3_linked_frame_acquisition_topologies_with_route_bound_evidence_v1(
        &report.acquisition_contract,
        report.generated_at_unix,
        topologies.to_vec(),
        terminals,
        frames,
        route_bound_frame_roots,
    );
    if u64::try_from(selection.raw_topologies.len()).unwrap_or(u64::MAX)
        != report.raw_scanned_topology_rows
        || u64::try_from(selection.candidate_topologies.len()).unwrap_or(u64::MAX)
            != report.candidate_topology_rows
        || u64::try_from(selection.eligible_topologies.len()).unwrap_or(u64::MAX)
            != report.eligible_topology_rows
        || selection.ineligible_reason_counts != report.ineligible_reason_counts
        || (report.schema == MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V4
            && selection.candidate_settlement_counts != report.candidate_settlement_counts)
    {
        return Err("ms3_scientific_denominator_reconstruction_mismatch");
    }

    let deadline_nanos = report
        .acquisition_contract
        .deadline_unix
        .saturating_mul(1_000_000_000);
    let mut terminals_by_request = BTreeMap::<&str, &TransportTerminalReceiptV1>::new();
    for terminal in terminals.iter().filter(|terminal| {
        terminal.validate() && terminal.completed_at_unix_nanos <= deadline_nanos
    }) {
        match terminals_by_request.insert(&terminal.request_event_id_sha256, terminal) {
            Some(existing) if existing != terminal => {
                return Err("ms3_scientific_denominator_terminal_rebound");
            }
            _ => {}
        }
    }
    let mut route_bound_frames = frames
        .iter()
        .filter_map(|frame| {
            let root = canonical_json_sha256(frame).ok()?;
            (frame.observed_at_unix_nanos <= deadline_nanos
                && route_bound_frame_roots.contains(&root))
            .then_some((root, frame))
        })
        .collect::<Vec<_>>();
    route_bound_frames.sort_by(|left, right| left.0.cmp(&right.0));

    let mut settlements = selection
        .eligible_topologies
        .iter()
        .map(|topology| {
            let terminal = terminals_by_request
                .get(topology.structure.request_event_id_sha256.as_str())
                .copied()
                .ok_or("ms3_scientific_denominator_terminal_missing")?;
            let (frame_root, _) = route_bound_frames
                .iter()
                .find(|(_, frame)| route_bound_frame_matches_topology(topology, frame))
                .ok_or("ms3_scientific_denominator_route_frame_missing")?;
            Ms3ScientificTopologySettlementV1::seal(
                report.acquisition_contract.contract_root_sha256.clone(),
                topology.commit.commitment_root_sha256.clone(),
                terminal.receipt_root_sha256.clone(),
                frame_root.clone(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    settlements.sort_by(|left, right| {
        left.topology_commitment_root_sha256
            .cmp(&right.topology_commitment_root_sha256)
    });

    let mut receipt = Ms3ScientificDenominatorReceiptV1 {
        schema: MS3_SCIENTIFIC_DENOMINATOR_RECEIPT_SCHEMA_V1.to_owned(),
        receipt_root_sha256: String::new(),
        acquisition_report_root_sha256: report.report_root_sha256.clone(),
        acquisition_contract_root_sha256: report.acquisition_contract.contract_root_sha256.clone(),
        acquisition_report_schema: report.schema.clone(),
        topology_watermark_rows: report.acquisition_contract.topology_watermark_rows,
        consumed_topology_cursor_rows: report.consumed_topology_cursor_rows,
        eligible_topology_rows: report.eligible_topology_rows,
        settlements,
        reconstruction,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    receipt.receipt_root_sha256 = receipt.expected_root();
    receipt
        .validate_against_report(report)
        .then_some(receipt)
        .ok_or("ms3_scientific_denominator_receipt_invalid")
}

#[must_use]
pub fn validate_ms3_scientific_denominator_evidence_v1(
    receipt: &Ms3ScientificDenominatorReceiptV1,
    report: &Ms3LinkedFrameAcquisitionReportV1,
    topologies: &[PreActionTopologyAuditRowV1],
    frames: &[RelationFrame],
    terminals: &[TransportTerminalReceiptV1],
    route_bound_frame_roots: &BTreeSet<String>,
) -> bool {
    if !receipt.validate_against_report(report) {
        return false;
    }
    let topologies_by_root = topologies
        .iter()
        .map(|topology| (topology.commit.commitment_root_sha256.as_str(), topology))
        .collect::<BTreeMap<_, _>>();
    let terminals_by_root = terminals
        .iter()
        .filter(|terminal| terminal.validate())
        .map(|terminal| (terminal.receipt_root_sha256.as_str(), terminal))
        .collect::<BTreeMap<_, _>>();
    let frames_by_root = frames
        .iter()
        .filter_map(|frame| canonical_json_sha256(frame).ok().map(|root| (root, frame)))
        .collect::<BTreeMap<_, _>>();
    receipt.settlements.iter().all(|settlement| {
        let Some(topology) =
            topologies_by_root.get(settlement.topology_commitment_root_sha256.as_str())
        else {
            return false;
        };
        let Some(terminal) =
            terminals_by_root.get(settlement.terminal_receipt_root_sha256.as_str())
        else {
            return false;
        };
        let Some(frame) = frames_by_root.get(settlement.route_bound_frame_root_sha256.as_str())
        else {
            return false;
        };
        terminal.request_event_id_sha256 == topology.structure.request_event_id_sha256
            && terminal.completed_at_unix_nanos
                <= report
                    .acquisition_contract
                    .deadline_unix
                    .saturating_mul(1_000_000_000)
            && route_bound_frame_roots.contains(&settlement.route_bound_frame_root_sha256)
            && frame.observed_at_unix_nanos
                <= report
                    .acquisition_contract
                    .deadline_unix
                    .saturating_mul(1_000_000_000)
            && route_bound_frame_matches_topology(topology, frame)
    })
}

fn acquisition_topology_eligibility(
    contract: &Ms3LinkedFrameAcquisitionContractV1,
    topology: &PreActionTopologyAuditRowV1,
    terminal_request_ids: &BTreeSet<String>,
    generated_at_unix: u64,
    receipt_lag_slo_seconds: u64,
) -> Result<(), super::MultiSourceJoinCensoredReasonV1> {
    super::validate_pre_action_topology_join_eligibility_v1(topology)?;
    if contract.schema == MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V3
        && !contract.uses_route_settlement_policy()
    {
        return Ok(());
    }
    if !terminal_request_ids.contains(&topology.structure.request_event_id_sha256)
        && topology.captured_at_unix_ms.is_some_and(|captured_at| {
            generated_at_unix
                .saturating_mul(1_000)
                .saturating_sub(captured_at)
                >= receipt_lag_slo_seconds.saturating_mul(1_000)
        })
    {
        return Err(super::MultiSourceJoinCensoredReasonV1::TerminalReceiptUnavailable);
    }
    Ok(())
}

impl Ms3LinkedFrameReceiptV1 {
    fn seal(
        contract: &Ms3LinkedFrameAcquisitionContractV1,
        bound: &super::TransportBoundJoinedTransitionV1,
        gap: Option<&super::RepresentationGapAdjudicationV1>,
    ) -> Self {
        let mut receipt = Self {
            schema: MS3_LINKED_FRAME_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            acquisition_contract_root_sha256: contract.contract_root_sha256.clone(),
            topology_commitment_root_sha256: bound.binding.topology_commitment_root_sha256.clone(),
            completed_frame_root_sha256: bound.binding.completed_frame_root_sha256.clone(),
            terminal_receipt_root_sha256: bound.binding.terminal_receipt_root_sha256.clone(),
            transport_binding_root_sha256: bound.binding.binding_root_sha256.clone(),
            session_lineage_sha256: bound.binding.session_lineage_sha256.clone(),
            session_id_sha256: bound.joined.session_id_sha256.clone(),
            turn_intent_id_sha256: bound.binding.turn_intent_id_sha256.clone(),
            request_event_id_sha256: bound.binding.request_event_id_sha256.clone(),
            action_event_id_sha256: bound.binding.action_event_id_sha256.clone(),
            classifier_version: REPRESENTATION_GAP_CLASSIFIER_VERSION_V1.to_owned(),
            gap_adjudication_root_sha256: gap.map(|row| row.adjudication_root_sha256.clone()),
            gap_class: gap.map(|row| row.gap_class),
            phase_update_allowed: false,
            authority_ready: false,
        };
        receipt.receipt_root_sha256 = receipt.expected_root();
        receipt
    }

    #[must_use]
    pub fn expected_root(&self) -> String {
        canonical_json_sha256(&(
            MS3_LINKED_FRAME_RECEIPT_SCHEMA_V1,
            self.acquisition_contract_root_sha256.as_str(),
            self.topology_commitment_root_sha256.as_str(),
            self.completed_frame_root_sha256.as_str(),
            self.terminal_receipt_root_sha256.as_str(),
            self.transport_binding_root_sha256.as_str(),
            self.session_lineage_sha256.as_str(),
            self.session_id_sha256.as_str(),
            self.turn_intent_id_sha256.as_str(),
            self.request_event_id_sha256.as_str(),
            self.action_event_id_sha256.as_str(),
            self.classifier_version.as_str(),
            self.gap_adjudication_root_sha256.as_deref(),
            self.gap_class,
            false,
            false,
        ))
        .expect("MS3 linked-frame receipt serializes")
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == MS3_LINKED_FRAME_RECEIPT_SCHEMA_V1
            && [
                Some(self.receipt_root_sha256.as_str()),
                Some(self.acquisition_contract_root_sha256.as_str()),
                Some(self.topology_commitment_root_sha256.as_str()),
                Some(self.completed_frame_root_sha256.as_str()),
                Some(self.terminal_receipt_root_sha256.as_str()),
                Some(self.transport_binding_root_sha256.as_str()),
                Some(self.session_lineage_sha256.as_str()),
                Some(self.session_id_sha256.as_str()),
                Some(self.turn_intent_id_sha256.as_str()),
                Some(self.request_event_id_sha256.as_str()),
                Some(self.action_event_id_sha256.as_str()),
                self.gap_adjudication_root_sha256.as_deref(),
            ]
            .into_iter()
            .flatten()
            .all(valid_nonzero_sha256)
            && self.gap_adjudication_root_sha256.is_some() == self.gap_class.is_some()
            && self.classifier_version == REPRESENTATION_GAP_CLASSIFIER_VERSION_V1
            && !self.phase_update_allowed
            && !self.authority_ready
            && self.receipt_root_sha256 == self.expected_root()
    }
}

impl Ms3ScientificTopologySettlementV1 {
    fn seal(
        acquisition_contract_root_sha256: String,
        topology_commitment_root_sha256: String,
        terminal_receipt_root_sha256: String,
        route_bound_frame_root_sha256: String,
    ) -> Result<Self, &'static str> {
        let mut settlement = Self {
            schema: MS3_SCIENTIFIC_TOPOLOGY_SETTLEMENT_SCHEMA_V1.to_owned(),
            settlement_root_sha256: String::new(),
            acquisition_contract_root_sha256,
            topology_commitment_root_sha256,
            terminal_receipt_root_sha256,
            route_bound_frame_root_sha256,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        settlement.settlement_root_sha256 = settlement.expected_root();
        settlement
            .validate()
            .then_some(settlement)
            .ok_or("ms3_scientific_topology_settlement_invalid")
    }

    #[must_use]
    pub fn expected_root(&self) -> String {
        canonical_json_sha256(&(
            MS3_SCIENTIFIC_TOPOLOGY_SETTLEMENT_SCHEMA_V1,
            self.acquisition_contract_root_sha256.as_str(),
            self.topology_commitment_root_sha256.as_str(),
            self.terminal_receipt_root_sha256.as_str(),
            self.route_bound_frame_root_sha256.as_str(),
            false,
            false,
        ))
        .expect("MS3 scientific topology settlement serializes")
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == MS3_SCIENTIFIC_TOPOLOGY_SETTLEMENT_SCHEMA_V1
            && [
                self.settlement_root_sha256.as_str(),
                self.acquisition_contract_root_sha256.as_str(),
                self.topology_commitment_root_sha256.as_str(),
                self.terminal_receipt_root_sha256.as_str(),
                self.route_bound_frame_root_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_nonzero_sha256)
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && self.settlement_root_sha256 == self.expected_root()
    }
}

impl Ms3ScientificDenominatorReceiptV1 {
    #[must_use]
    pub fn expected_root(&self) -> String {
        canonical_json_sha256(&(
            MS3_SCIENTIFIC_DENOMINATOR_RECEIPT_SCHEMA_V1,
            self.acquisition_report_root_sha256.as_str(),
            self.acquisition_contract_root_sha256.as_str(),
            self.acquisition_report_schema.as_str(),
            self.topology_watermark_rows,
            self.consumed_topology_cursor_rows,
            self.eligible_topology_rows,
            &self.settlements,
            self.reconstruction,
            false,
            false,
        ))
        .expect("MS3 scientific denominator receipt serializes")
    }

    #[must_use]
    pub fn validate_against_report(&self, report: &Ms3LinkedFrameAcquisitionReportV1) -> bool {
        let settlement_topology_roots = self
            .settlements
            .iter()
            .map(|settlement| settlement.topology_commitment_root_sha256.as_str())
            .collect::<BTreeSet<_>>();
        self.schema == MS3_SCIENTIFIC_DENOMINATOR_RECEIPT_SCHEMA_V1
            && [
                self.receipt_root_sha256.as_str(),
                self.acquisition_report_root_sha256.as_str(),
                self.acquisition_contract_root_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_nonzero_sha256)
            && report.validate()
            && report.is_terminal()
            && report.acquisition_contract.uses_route_settlement_policy()
            && self.acquisition_report_root_sha256 == report.report_root_sha256
            && self.acquisition_contract_root_sha256
                == report.acquisition_contract.contract_root_sha256
            && self.acquisition_report_schema == report.schema
            && self.topology_watermark_rows == report.acquisition_contract.topology_watermark_rows
            && self.consumed_topology_cursor_rows == report.consumed_topology_cursor_rows
            && self.eligible_topology_rows == report.eligible_topology_rows
            && self.settlements.len()
                == usize::try_from(self.eligible_topology_rows).unwrap_or(usize::MAX)
            && self.settlements.iter().all(|settlement| {
                settlement.validate()
                    && settlement.acquisition_contract_root_sha256
                        == self.acquisition_contract_root_sha256
            })
            && self.settlements.windows(2).all(|pair| {
                pair[0].topology_commitment_root_sha256 < pair[1].topology_commitment_root_sha256
            })
            && report.receipts.iter().all(|receipt| {
                settlement_topology_roots.contains(receipt.topology_commitment_root_sha256.as_str())
            })
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && self.receipt_root_sha256 == self.expected_root()
    }
}

impl Ms3ScientificDenominatorEnvelopeV1 {
    pub fn seal(
        report: Ms3LinkedFrameAcquisitionReportV1,
        receipt: Ms3ScientificDenominatorReceiptV1,
    ) -> Result<Self, &'static str> {
        let mut envelope = Self {
            schema: MS3_SCIENTIFIC_DENOMINATOR_ENVELOPE_SCHEMA_V1.to_owned(),
            envelope_root_sha256: String::new(),
            report,
            receipt,
        };
        envelope.envelope_root_sha256 = envelope.expected_root();
        envelope
            .validate()
            .then_some(envelope)
            .ok_or("ms3_scientific_denominator_envelope_invalid")
    }

    #[must_use]
    pub fn expected_root(&self) -> String {
        canonical_json_sha256(&(
            MS3_SCIENTIFIC_DENOMINATOR_ENVELOPE_SCHEMA_V1,
            self.report.report_root_sha256.as_str(),
            self.receipt.receipt_root_sha256.as_str(),
        ))
        .expect("MS3 scientific denominator envelope serializes")
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == MS3_SCIENTIFIC_DENOMINATOR_ENVELOPE_SCHEMA_V1
            && valid_nonzero_sha256(&self.envelope_root_sha256)
            && self.receipt.validate_against_report(&self.report)
            && self.envelope_root_sha256 == self.expected_root()
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, &'static str> {
        if !self.validate() {
            return Err("ms3_scientific_denominator_envelope_invalid");
        }
        let bytes =
            serde_cbor::to_vec(self).map_err(|_| "ms3_scientific_denominator_envelope_encode")?;
        if bytes.is_empty() || bytes.len() > MAX_SCIENTIFIC_DENOMINATOR_ENVELOPE_BYTES {
            return Err("ms3_scientific_denominator_envelope_budget");
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.is_empty() || bytes.len() > MAX_SCIENTIFIC_DENOMINATOR_ENVELOPE_BYTES {
            return Err("ms3_scientific_denominator_envelope_budget");
        }
        let envelope = serde_cbor::from_slice::<Self>(bytes)
            .map_err(|_| "ms3_scientific_denominator_envelope_decode")?;
        if !envelope.validate() || envelope.canonical_bytes()? != bytes {
            return Err("ms3_scientific_denominator_envelope_invalid");
        }
        Ok(envelope)
    }
}

impl Ms3LinkedFrameAcquisitionReportV1 {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        !matches!(self.verdict, Ms3LinkedFrameAcquisitionVerdictV1::Collecting)
    }

    #[must_use]
    pub fn expected_root(&self) -> String {
        if self.schema == MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V4 {
            canonical_json_sha256(&Ms3LinkedFrameAcquisitionReportDigestV4 {
                schema: MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V4,
                acquisition_contract: &self.acquisition_contract,
                generated_at_unix: self.generated_at_unix,
                new_topology_rows_seen: self.new_topology_rows_seen,
                evaluated_topology_rows: self.evaluated_topology_rows,
                terminal_receipt_rows: self.terminal_receipt_rows,
                relevant_verified_frame_rows: self.relevant_verified_frame_rows,
                linked_frame_rows: self.linked_frame_rows,
                gap_class_counts: &self.gap_class_counts,
                no_representation_gap_rows: self.no_representation_gap_rows,
                receipts: &self.receipts,
                verdict: self.verdict,
                blocker: &self.blocker,
                phase_update_allowed: false,
                authority_ready: false,
                raw_scanned_topology_rows: self.raw_scanned_topology_rows,
                candidate_topology_rows: self.candidate_topology_rows,
                eligible_topology_rows: self.eligible_topology_rows,
                censored_unattributed_rows: self.censored_unattributed_rows,
                censored_topology_rows: self.censored_topology_rows,
                ineligible_reason_counts: &self.ineligible_reason_counts,
                consumed_topology_cursor_rows: self.consumed_topology_cursor_rows,
                consumed_capture_sequence: self.consumed_capture_sequence,
                candidate_settlement_counts: &self.candidate_settlement_counts,
                route_settlement_pending_rows: self.route_settlement_pending_rows,
                transport_binding_failure_counts: &self.transport_binding_failure_counts,
                evidence_reuse_excluded_rows: self.evidence_reuse_excluded_rows,
            })
            .expect("MS3 acquisition report serializes")
        } else if self.schema == MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V3 {
            canonical_json_sha256(&Ms3LinkedFrameAcquisitionReportDigestV3 {
                schema: MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V3,
                acquisition_contract: &self.acquisition_contract,
                generated_at_unix: self.generated_at_unix,
                new_topology_rows_seen: self.new_topology_rows_seen,
                evaluated_topology_rows: self.evaluated_topology_rows,
                terminal_receipt_rows: self.terminal_receipt_rows,
                relevant_verified_frame_rows: self.relevant_verified_frame_rows,
                linked_frame_rows: self.linked_frame_rows,
                gap_class_counts: &self.gap_class_counts,
                no_representation_gap_rows: self.no_representation_gap_rows,
                receipts: &self.receipts,
                verdict: self.verdict,
                blocker: &self.blocker,
                phase_update_allowed: false,
                authority_ready: false,
                raw_scanned_topology_rows: self.raw_scanned_topology_rows,
                candidate_topology_rows: self.candidate_topology_rows,
                eligible_topology_rows: self.eligible_topology_rows,
                censored_unattributed_rows: self.censored_unattributed_rows,
                censored_topology_rows: self.censored_topology_rows,
                ineligible_reason_counts: &self.ineligible_reason_counts,
                consumed_topology_cursor_rows: self.consumed_topology_cursor_rows,
                consumed_capture_sequence: self.consumed_capture_sequence,
            })
            .expect("MS3 acquisition report serializes")
        } else if self.schema == MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V2 {
            canonical_json_sha256(&Ms3LinkedFrameAcquisitionReportDigestV2 {
                schema: MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V2,
                acquisition_contract: &self.acquisition_contract,
                generated_at_unix: self.generated_at_unix,
                new_topology_rows_seen: self.new_topology_rows_seen,
                evaluated_topology_rows: self.evaluated_topology_rows,
                terminal_receipt_rows: self.terminal_receipt_rows,
                relevant_verified_frame_rows: self.relevant_verified_frame_rows,
                linked_frame_rows: self.linked_frame_rows,
                gap_class_counts: &self.gap_class_counts,
                no_representation_gap_rows: self.no_representation_gap_rows,
                receipts: &self.receipts,
                verdict: self.verdict,
                blocker: &self.blocker,
                phase_update_allowed: false,
                authority_ready: false,
                raw_scanned_topology_rows: self.raw_scanned_topology_rows,
                eligible_topology_rows: self.eligible_topology_rows,
                censored_unattributed_rows: self.censored_unattributed_rows,
                censored_topology_rows: self.censored_topology_rows,
                ineligible_reason_counts: &self.ineligible_reason_counts,
                consumed_topology_cursor_rows: self.consumed_topology_cursor_rows,
                consumed_capture_sequence: self.consumed_capture_sequence,
            })
            .expect("MS3 acquisition report serializes")
        } else {
            canonical_json_sha256(&(
                MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V1,
                &self.acquisition_contract,
                self.generated_at_unix,
                self.new_topology_rows_seen,
                self.evaluated_topology_rows,
                self.terminal_receipt_rows,
                self.relevant_verified_frame_rows,
                self.linked_frame_rows,
                &self.gap_class_counts,
                self.no_representation_gap_rows,
                &self.receipts,
                self.verdict,
                self.blocker.as_str(),
                false,
                false,
            ))
            .expect("MS3 acquisition report serializes")
        }
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        let classified = self.gap_class_counts.values().copied().sum::<u64>();
        let common_valid = matches!(
            self.schema.as_str(),
            MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V1
                | MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V2
                | MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V3
                | MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V4
        ) && self.acquisition_contract.validate()
            && self.generated_at_unix >= self.acquisition_contract.opened_at_unix
            && self.evaluated_topology_rows <= self.acquisition_contract.max_new_topology_rows
            && self.evaluated_topology_rows <= self.new_topology_rows_seen
            && self.linked_frame_rows == u64::try_from(self.receipts.len()).unwrap_or(u64::MAX)
            && classified.saturating_add(self.no_representation_gap_rows) == self.linked_frame_rows
            && self.receipts.iter().all(Ms3LinkedFrameReceiptV1::validate)
            && self.receipts.iter().all(|receipt| {
                receipt.acquisition_contract_root_sha256
                    == self.acquisition_contract.contract_root_sha256
            });
        let verdict_valid = match self.verdict {
            Ms3LinkedFrameAcquisitionVerdictV1::Collecting => {
                let bounded_route_window_closed = self.schema
                    == MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V4
                    && (self.generated_at_unix >= self.acquisition_contract.deadline_unix
                        || self.evaluated_topology_rows
                            == self.acquisition_contract.max_new_topology_rows
                        || self.raw_scanned_topology_rows
                            == self.acquisition_contract.max_raw_topology_rows());
                self.linked_frame_rows == 0
                    && !bounded_route_window_closed
                    && !((self.generated_at_unix >= self.acquisition_contract.deadline_unix
                        || self.evaluated_topology_rows
                            == self.acquisition_contract.max_new_topology_rows)
                        && self.terminal_receipt_rows >= self.evaluated_topology_rows
                        && (!self.acquisition_contract.uses_route_settlement_policy()
                            || self.candidate_topology_rows == self.eligible_topology_rows))
                    && matches!(
                        self.blocker.as_str(),
                        "linked_frame_pending"
                            | "terminal_receipt_stalled"
                            | "route_settlement_stalled"
                            | "route_settlement_pending"
                    )
            }
            Ms3LinkedFrameAcquisitionVerdictV1::LinkedFrameObserved => {
                self.linked_frame_rows > 0 && self.blocker.is_empty()
            }
            Ms3LinkedFrameAcquisitionVerdictV1::AcquisitionFail => {
                let bounded_route_window_closed = self.schema
                    == MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V4
                    && (self.generated_at_unix >= self.acquisition_contract.deadline_unix
                        || self.evaluated_topology_rows
                            == self.acquisition_contract.max_new_topology_rows
                        || self.raw_scanned_topology_rows
                            == self.acquisition_contract.max_raw_topology_rows());
                self.linked_frame_rows == 0
                    && (bounded_route_window_closed
                        || ((self.generated_at_unix >= self.acquisition_contract.deadline_unix
                            || self.evaluated_topology_rows
                                == self.acquisition_contract.max_new_topology_rows)
                            && self.terminal_receipt_rows >= self.evaluated_topology_rows
                            && (!self.acquisition_contract.uses_route_settlement_policy()
                                || self.candidate_topology_rows == self.eligible_topology_rows)))
                    && self.blocker == MS3_LINKED_FRAME_ACQUISITION_FAIL
            }
            Ms3LinkedFrameAcquisitionVerdictV1::CensoredUnattributedProbe => {
                self.linked_frame_rows == 0
                    && self.censored_unattributed_rows > 0
                    && self.evaluated_topology_rows
                        < self.acquisition_contract.max_new_topology_rows
                    && self.raw_scanned_topology_rows
                        == self.acquisition_contract.max_raw_topology_rows()
                    && self.terminal_receipt_rows >= self.evaluated_topology_rows
                    && self.censored_topology_rows == 0
                    && self.censored_unattributed_rows
                        == self
                            .raw_scanned_topology_rows
                            .saturating_sub(self.eligible_topology_rows)
                    && self.blocker == MS3_CENSORED_UNATTRIBUTED_PROBE
            }
            Ms3LinkedFrameAcquisitionVerdictV1::CensoredIneligibleProbe => {
                self.linked_frame_rows == 0
                    && self.censored_topology_rows > 0
                    && self.evaluated_topology_rows
                        < self.acquisition_contract.max_new_topology_rows
                    && self.raw_scanned_topology_rows
                        == self.acquisition_contract.max_raw_topology_rows()
                    && self.terminal_receipt_rows >= self.evaluated_topology_rows
                    && self
                        .censored_unattributed_rows
                        .saturating_add(self.censored_topology_rows)
                        == self
                            .raw_scanned_topology_rows
                            .saturating_sub(self.eligible_topology_rows)
                    && self.blocker == MS3_CENSORED_INELIGIBLE_PROBE
            }
            Ms3LinkedFrameAcquisitionVerdictV1::CensoredPreRouteReceiptEpoch => {
                self.linked_frame_rows == 0
                    && self.acquisition_contract.schema
                        == MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V2
                    && self
                        .ineligible_reason_counts
                        .get(&super::MultiSourceJoinCensoredReasonV1::TerminalReceiptUnavailable)
                        .is_some_and(|rows| *rows > 0 && *rows <= self.censored_topology_rows)
                    && self.blocker == MS3_CENSORED_PRE_ROUTE_RECEIPT_EPOCH
            }
        };
        let v4_observability_fields_empty = self.candidate_settlement_counts.is_empty()
            && self.route_settlement_pending_rows == 0
            && self.transport_binding_failure_counts.is_empty()
            && self.evidence_reuse_excluded_rows == 0;
        let schema_valid = if self.schema == MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V1 {
            self.raw_scanned_topology_rows == 0
                && self.candidate_topology_rows == 0
                && self.eligible_topology_rows == 0
                && self.censored_unattributed_rows == 0
                && self.censored_topology_rows == 0
                && self.ineligible_reason_counts.is_empty()
                && self.consumed_topology_cursor_rows == 0
                && self.consumed_capture_sequence == 0
                && v4_observability_fields_empty
                && !matches!(
                    self.verdict,
                    Ms3LinkedFrameAcquisitionVerdictV1::CensoredUnattributedProbe
                        | Ms3LinkedFrameAcquisitionVerdictV1::CensoredIneligibleProbe
                        | Ms3LinkedFrameAcquisitionVerdictV1::CensoredPreRouteReceiptEpoch
                )
        } else if self.schema == MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V2 {
            let ineligible_rows = self
                .raw_scanned_topology_rows
                .saturating_sub(self.eligible_topology_rows);
            self.candidate_topology_rows == 0
                && self.raw_scanned_topology_rows
                    <= self.acquisition_contract.max_raw_topology_rows()
                && self.raw_scanned_topology_rows <= self.new_topology_rows_seen
                && self.eligible_topology_rows == self.evaluated_topology_rows
                && self.eligible_topology_rows <= self.raw_scanned_topology_rows
                && self.terminal_receipt_rows <= self.eligible_topology_rows
                && self.ineligible_reason_counts.values().copied().sum::<u64>() == ineligible_rows
                && self.censored_unattributed_rows
                    == self
                        .ineligible_reason_counts
                        .get(&super::MultiSourceJoinCensoredReasonV1::ProviderIdentityUnproven)
                        .copied()
                        .unwrap_or(0)
                && self.censored_topology_rows
                    == self
                        .ineligible_reason_counts
                        .get(&super::MultiSourceJoinCensoredReasonV1::TopologyCensored)
                        .copied()
                        .unwrap_or(0)
                        .saturating_add(
                            self.ineligible_reason_counts
                                .get(
                                    &super::MultiSourceJoinCensoredReasonV1::TerminalReceiptUnavailable,
                                )
                                .copied()
                                .unwrap_or(0),
                        )
                && self.consumed_topology_cursor_rows
                    == self
                        .acquisition_contract
                        .topology_watermark_rows
                        .saturating_add(self.raw_scanned_topology_rows)
                && (self.raw_scanned_topology_rows == 0 && self.consumed_capture_sequence == 0
                    || self.raw_scanned_topology_rows > 0 && self.consumed_capture_sequence > 0)
                && self.verdict
                    != Ms3LinkedFrameAcquisitionVerdictV1::CensoredPreRouteReceiptEpoch
                && v4_observability_fields_empty
        } else if self.schema == MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V3 {
            let route_ineligible_rows = self
                .raw_scanned_topology_rows
                .saturating_sub(self.candidate_topology_rows);
            let pre_route_ineligible_rows = self
                .raw_scanned_topology_rows
                .saturating_sub(self.eligible_topology_rows);
            let common_cursor_valid = self.consumed_topology_cursor_rows
                == self
                    .acquisition_contract
                    .topology_watermark_rows
                    .saturating_add(self.raw_scanned_topology_rows)
                && (self.raw_scanned_topology_rows == 0 && self.consumed_capture_sequence == 0
                    || self.raw_scanned_topology_rows > 0 && self.consumed_capture_sequence > 0);
            let pre_route_migration_valid = self.verdict
                == Ms3LinkedFrameAcquisitionVerdictV1::CensoredPreRouteReceiptEpoch
                && self.acquisition_contract.schema
                    == MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V2
                && self.candidate_topology_rows == 0
                && self.raw_scanned_topology_rows
                    <= self.acquisition_contract.max_raw_topology_rows()
                && self.raw_scanned_topology_rows <= self.new_topology_rows_seen
                && self.eligible_topology_rows == self.evaluated_topology_rows
                && self.eligible_topology_rows <= self.raw_scanned_topology_rows
                && self.terminal_receipt_rows <= self.eligible_topology_rows
                && self.ineligible_reason_counts.values().copied().sum::<u64>()
                    == pre_route_ineligible_rows
                && self.censored_unattributed_rows
                    == self
                        .ineligible_reason_counts
                        .get(&super::MultiSourceJoinCensoredReasonV1::ProviderIdentityUnproven)
                        .copied()
                        .unwrap_or(0)
                && self.censored_topology_rows
                    == self
                        .ineligible_reason_counts
                        .get(&super::MultiSourceJoinCensoredReasonV1::TopologyCensored)
                        .copied()
                        .unwrap_or(0)
                        .saturating_add(
                            self.ineligible_reason_counts
                                .get(
                                    &super::MultiSourceJoinCensoredReasonV1::TerminalReceiptUnavailable,
                                )
                                .copied()
                                .unwrap_or(0),
                        )
                && common_cursor_valid;
            let route_settlement_valid = self.acquisition_contract.uses_route_settlement_policy()
                && self.raw_scanned_topology_rows
                    <= self.acquisition_contract.max_raw_topology_rows()
                && self.raw_scanned_topology_rows <= self.new_topology_rows_seen
                && self.candidate_topology_rows <= self.raw_scanned_topology_rows
                && self.eligible_topology_rows == self.evaluated_topology_rows
                && self.eligible_topology_rows <= self.candidate_topology_rows
                && self.eligible_topology_rows <= self.terminal_receipt_rows
                && self.terminal_receipt_rows <= self.candidate_topology_rows
                && self.ineligible_reason_counts.values().copied().sum::<u64>()
                    == route_ineligible_rows
                && self.censored_unattributed_rows
                    == self
                        .ineligible_reason_counts
                        .get(&super::MultiSourceJoinCensoredReasonV1::ProviderIdentityUnproven)
                        .copied()
                        .unwrap_or(0)
                && self.censored_topology_rows
                    == self
                        .ineligible_reason_counts
                        .get(&super::MultiSourceJoinCensoredReasonV1::TopologyCensored)
                        .copied()
                        .unwrap_or(0)
                && !self.ineligible_reason_counts.contains_key(
                    &super::MultiSourceJoinCensoredReasonV1::TerminalReceiptUnavailable,
                )
                && common_cursor_valid;
            (pre_route_migration_valid || route_settlement_valid) && v4_observability_fields_empty
        } else {
            let route_ineligible_rows = self
                .raw_scanned_topology_rows
                .saturating_sub(self.candidate_topology_rows);
            let pending_rows = self
                .candidate_topology_rows
                .saturating_sub(self.eligible_topology_rows);
            let pending_class_rows = self
                .candidate_settlement_counts
                .get(&Ms3CandidateSettlementClassV1::TerminalPending)
                .copied()
                .unwrap_or(0)
                .saturating_add(
                    self.candidate_settlement_counts
                        .get(&Ms3CandidateSettlementClassV1::RouteFramePending)
                        .copied()
                        .unwrap_or(0),
                )
                .saturating_add(
                    self.candidate_settlement_counts
                        .get(&Ms3CandidateSettlementClassV1::ReceiptStalled)
                        .copied()
                        .unwrap_or(0),
                );
            let settled_rows = self
                .candidate_settlement_counts
                .get(&Ms3CandidateSettlementClassV1::SettledEligible)
                .copied()
                .unwrap_or(0);
            let structurally_ineligible_rows = self
                .candidate_settlement_counts
                .get(&Ms3CandidateSettlementClassV1::StructurallyIneligible)
                .copied()
                .unwrap_or(0);
            self.acquisition_contract.uses_route_settlement_policy()
                && self.raw_scanned_topology_rows
                    <= self.acquisition_contract.max_raw_topology_rows()
                && self.raw_scanned_topology_rows <= self.new_topology_rows_seen
                && self.candidate_topology_rows <= self.raw_scanned_topology_rows
                && self.eligible_topology_rows == self.evaluated_topology_rows
                && self.eligible_topology_rows <= self.candidate_topology_rows
                && self.eligible_topology_rows <= self.terminal_receipt_rows
                && self.terminal_receipt_rows <= self.candidate_topology_rows
                && self.ineligible_reason_counts.values().copied().sum::<u64>()
                    == route_ineligible_rows
                && self
                    .candidate_settlement_counts
                    .values()
                    .copied()
                    .sum::<u64>()
                    == self.raw_scanned_topology_rows
                && self
                    .candidate_settlement_counts
                    .values()
                    .all(|rows| *rows > 0)
                && settled_rows == self.eligible_topology_rows
                && structurally_ineligible_rows == route_ineligible_rows
                && pending_class_rows == pending_rows
                && self.route_settlement_pending_rows == pending_rows
                && self
                    .transport_binding_failure_counts
                    .values()
                    .copied()
                    .sum::<u64>()
                    <= self.candidate_topology_rows
                && self
                    .transport_binding_failure_counts
                    .values()
                    .all(|rows| *rows > 0)
                && self.evidence_reuse_excluded_rows <= self.eligible_topology_rows
                && self.censored_unattributed_rows
                    == self
                        .ineligible_reason_counts
                        .get(&super::MultiSourceJoinCensoredReasonV1::ProviderIdentityUnproven)
                        .copied()
                        .unwrap_or(0)
                && self.censored_topology_rows
                    == self
                        .ineligible_reason_counts
                        .get(&super::MultiSourceJoinCensoredReasonV1::TopologyCensored)
                        .copied()
                        .unwrap_or(0)
                && !self.ineligible_reason_counts.contains_key(
                    &super::MultiSourceJoinCensoredReasonV1::TerminalReceiptUnavailable,
                )
                && self.consumed_topology_cursor_rows
                    == self
                        .acquisition_contract
                        .topology_watermark_rows
                        .saturating_add(self.raw_scanned_topology_rows)
                && (self.raw_scanned_topology_rows == 0 && self.consumed_capture_sequence == 0
                    || self.raw_scanned_topology_rows > 0 && self.consumed_capture_sequence > 0)
        };
        common_valid
            && verdict_valid
            && schema_valid
            && !self.phase_update_allowed
            && !self.authority_ready
            && self.report_root_sha256 == self.expected_root()
    }
}

const fn is_zero(value: &u64) -> bool {
    *value == 0
}
