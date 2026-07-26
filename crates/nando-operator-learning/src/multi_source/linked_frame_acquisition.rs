use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{RelationFrame, canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::{
    PreActionTopologyAuditRowV1, RepresentationGapClassV1, RequestStructureAuditSnapshotV1,
    TransportBindingLedgerV1, TransportTerminalReceiptV1,
    build_representation_gap_adjudication_report_v1,
};

pub const MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V1: &str =
    "nando.ms3-linked-frame-acquisition-contract.v1";
pub const MS3_LINKED_FRAME_RECEIPT_SCHEMA_V1: &str = "nando.ms3-linked-frame-receipt.v1";
pub const MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V1: &str =
    "nando.ms3-linked-frame-acquisition-report.v1";
pub const REPRESENTATION_GAP_CLASSIFIER_VERSION_V1: &str = "nando.representation-gap-classifier.v1";
pub const MS3_LINKED_FRAME_ACQUISITION_FAIL: &str = "MS3_LINKED_FRAME_ACQUISITION_FAIL";
const MAX_ACQUISITION_TOPOLOGIES: u64 = 4_096;
const MAX_ACQUISITION_SECONDS: u64 = 7 * 24 * 60 * 60;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ms3LinkedFrameAcquisitionVerdictV1 {
    Collecting,
    LinkedFrameObserved,
    AcquisitionFail,
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
    pub fn expected_root(&self) -> String {
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

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V1
            && valid_nonzero_sha256(&self.contract_root_sha256)
            && valid_nonzero_sha256(&self.topology_prefix_root_sha256)
            && self.opened_at_unix > 0
            && self.deadline_unix > self.opened_at_unix
            && self.max_elapsed_seconds() <= MAX_ACQUISITION_SECONDS
            && (1..=MAX_ACQUISITION_TOPOLOGIES).contains(&self.max_new_topology_rows)
            && self.classifier_version == REPRESENTATION_GAP_CLASSIFIER_VERSION_V1
            && !self.authority_ready
            && self.contract_root_sha256 == self.expected_root()
    }
}

#[must_use]
pub fn build_ms3_linked_frame_acquisition_report_v1(
    contract: Ms3LinkedFrameAcquisitionContractV1,
    generated_at_unix: u64,
    mut new_topologies: Vec<PreActionTopologyAuditRowV1>,
    mut frames: Vec<RelationFrame>,
    mut terminals: Vec<TransportTerminalReceiptV1>,
) -> Ms3LinkedFrameAcquisitionReportV1 {
    let new_topology_rows_seen = u64::try_from(new_topologies.len()).unwrap_or(u64::MAX);
    new_topologies.truncate(usize::try_from(contract.max_new_topology_rows).unwrap_or(usize::MAX));
    let request_ids = new_topologies
        .iter()
        .map(|row| row.structure.request_event_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let intent_ids = new_topologies
        .iter()
        .map(|row| row.structure.turn_intent_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let deadline_nanos = contract.deadline_unix.saturating_mul(1_000_000_000);
    frames.retain(|frame| {
        intent_ids.contains(frame.client_intent_id_sha256.as_str())
            && frame.observed_at_unix_nanos <= deadline_nanos
    });
    terminals.retain(|receipt| {
        request_ids.contains(receipt.request_event_id_sha256.as_str())
            && receipt.completed_at_unix_nanos <= deadline_nanos
    });
    let ledger = TransportBindingLedgerV1::build(&new_topologies, &frames, &terminals);
    let gap_report = build_representation_gap_adjudication_report_v1(
        request_snapshot(new_topologies.clone()),
        frames.clone(),
        terminals.clone(),
    );
    let gaps_by_binding = gap_report
        .rows
        .iter()
        .map(|row| (row.transport_binding_root_sha256.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut receipts = Vec::new();
    for topology in &new_topologies {
        for bound in ledger.bound_for_topology(&topology.commit.commitment_root_sha256) {
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
    let evaluated_topology_rows = u64::try_from(new_topologies.len()).unwrap_or(u64::MAX);
    let terminal_receipt_rows = u64::try_from(terminals.len()).unwrap_or(u64::MAX);
    let row_budget_exhausted = evaluated_topology_rows == contract.max_new_topology_rows
        && terminal_receipt_rows >= evaluated_topology_rows;
    let exhausted = generated_at_unix >= contract.deadline_unix || row_budget_exhausted;
    let (verdict, blocker) = if linked_frame_rows > 0 {
        (
            Ms3LinkedFrameAcquisitionVerdictV1::LinkedFrameObserved,
            String::new(),
        )
    } else if exhausted {
        (
            Ms3LinkedFrameAcquisitionVerdictV1::AcquisitionFail,
            MS3_LINKED_FRAME_ACQUISITION_FAIL.to_owned(),
        )
    } else {
        (
            Ms3LinkedFrameAcquisitionVerdictV1::Collecting,
            "linked_frame_pending".to_owned(),
        )
    };
    let mut report = Ms3LinkedFrameAcquisitionReportV1 {
        schema: MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V1.to_owned(),
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
    };
    report.report_root_sha256 = report.expected_root();
    report
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

impl Ms3LinkedFrameAcquisitionReportV1 {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        !matches!(self.verdict, Ms3LinkedFrameAcquisitionVerdictV1::Collecting)
    }

    #[must_use]
    pub fn expected_root(&self) -> String {
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

    #[must_use]
    pub fn validate(&self) -> bool {
        let classified = self.gap_class_counts.values().copied().sum::<u64>();
        self.schema == MS3_LINKED_FRAME_ACQUISITION_REPORT_SCHEMA_V1
            && self.acquisition_contract.validate()
            && self.generated_at_unix >= self.acquisition_contract.opened_at_unix
            && self.evaluated_topology_rows <= self.acquisition_contract.max_new_topology_rows
            && self.evaluated_topology_rows <= self.new_topology_rows_seen
            && self.linked_frame_rows == u64::try_from(self.receipts.len()).unwrap_or(u64::MAX)
            && classified.saturating_add(self.no_representation_gap_rows) == self.linked_frame_rows
            && self.receipts.iter().all(Ms3LinkedFrameReceiptV1::validate)
            && self.receipts.iter().all(|receipt| {
                receipt.acquisition_contract_root_sha256
                    == self.acquisition_contract.contract_root_sha256
            })
            && match self.verdict {
                Ms3LinkedFrameAcquisitionVerdictV1::Collecting => {
                    self.linked_frame_rows == 0
                        && self.generated_at_unix < self.acquisition_contract.deadline_unix
                        && (self.evaluated_topology_rows
                            < self.acquisition_contract.max_new_topology_rows
                            || self.terminal_receipt_rows < self.evaluated_topology_rows)
                        && self.blocker == "linked_frame_pending"
                }
                Ms3LinkedFrameAcquisitionVerdictV1::LinkedFrameObserved => {
                    self.linked_frame_rows > 0 && self.blocker.is_empty()
                }
                Ms3LinkedFrameAcquisitionVerdictV1::AcquisitionFail => {
                    self.linked_frame_rows == 0
                        && (self.generated_at_unix >= self.acquisition_contract.deadline_unix
                            || (self.evaluated_topology_rows
                                == self.acquisition_contract.max_new_topology_rows
                                && self.terminal_receipt_rows >= self.evaluated_topology_rows))
                        && self.blocker == MS3_LINKED_FRAME_ACQUISITION_FAIL
                }
            }
            && !self.phase_update_allowed
            && !self.authority_ready
            && self.report_root_sha256 == self.expected_root()
    }
}
