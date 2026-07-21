//! Frozen B1B-F acquisition protocol and label-blind future capture owner.
//!
//! This module binds future capture to the committed B1B-S prefix. It can
//! freeze pre-action candidate graphs, but it cannot join labels, adjudicate a
//! causal hypothesis, compile a protocol mode, or grant execution authority.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::binding_evidence::{
    BindingEvidenceBudgetV1, FrozenCandidateRelationGraphV1, PreActionBindingContextV1,
    PreActionBindingSurfaceV1,
};
use super::binding_evidence_capture_owner::BindingSupportFreezeV1;
use super::binding_evidence_preregistration::BindingLabelObservationSourceV1;
use crate::capture_provenance::{CaptureCommitmentIndex, CaptureEvidenceReceipt};
use crate::evidence::{CanonicalEventGraph, CanonicalEventNode};
use crate::{
    EVIDENCE_LEDGER_SCHEMA_V1, EvidenceIngestOutcome, EvidenceLedgerRecord, canonical_json_sha256,
    evidence_payload_sha256,
};

pub const BINDING_FUTURE_ACQUISITION_PROTOCOL_SCHEMA_V1: &str =
    "nando.binding-future-acquisition-protocol.v1";
pub const BINDING_FUTURE_CAPTURE_BATCH_SCHEMA_V1: &str = "nando.binding-future-capture-batch.v1";
pub const BINDING_FUTURE_CAPTURE_ROW_SCHEMA_V1: &str = "nando.binding-future-capture-row.v1";
pub const BINDING_FUTURE_CAPTURE_FREEZE_SCHEMA_V1: &str = "nando.binding-future-capture-freeze.v1";
const BINDING_FUTURE_CAPTURE_REPORT_SCHEMA_V1: &str = "nando.binding-future-capture-report.v1";
const PINNED_SUPPORT_COMMIT_ID: &str = "a3f8f05f5b8d8928c1044ed38eb107fb8be27e5b";
const PINNED_SUPPORT_FREEZE_FILE_SHA256: &str =
    "8a856f4abc9f56b3c618c78acb0d38e9a93b3b10a0f0a7cf1dc0248f38bb2a5f";
const PINNED_SUPPORT_FREEZE_RECEIPT_SHA256: &str =
    "c614e71c8856eb8c5787df694be4a79474caf5206a03739b7eec29e3a018241f";
const PINNED_SUPPORT_WATERMARK_FILE_SHA256: &str =
    "4eb5160744dfdf0ffd3717fbe91a918cadab797532b3bfa733172b3926dcbaa5";
const PINNED_SUPPORT_CAPTURE_INDEX_SHA256: &str =
    "6767c53f13fd42f74c55c154610ec69f534a874efee68f2eaf722b46bc5055ea";
const PINNED_SUPPORT_WATERMARK_NEXT_SEQUENCE: u64 = 12;
const FUTURE_ROWS_V1: usize = 12;
const FUTURE_SESSION_SLOTS_V1: usize = 4;
const MIN_DISTINCT_FUTURE_SHAPES_V1: usize = 3;
const MIN_ORDINAL_LAYOUT_TRAP_PAIRS_V1: usize = 1;
const FUTURE_REQUEST_TEXT_V1: &str = "continue active execution";

const FUTURE_SLOT_SCHEDULE_V1: [(&str, &str, &str); FUTURE_ROWS_V1] = [
    ("F00", "I1", "S0"),
    ("F01", "I2", "S0"),
    ("F02", "I3", "S0"),
    ("F03", "I4", "S1"),
    ("F04", "I5", "S1"),
    ("F05", "I6", "S1"),
    ("F06", "I1", "S2"),
    ("F07", "I2", "S2"),
    ("F08", "I3", "S2"),
    ("F09", "I4", "S3"),
    ("F10", "I5", "S3"),
    ("F11", "I6", "S3"),
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingFutureCaptureSlotV1 {
    pub slot_id: String,
    pub intervention_id: String,
    pub session_slot: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingFutureChallengeContractV1 {
    pub minimum_distinct_future_shape_roots: usize,
    pub future_shape_must_be_absent_from_support: bool,
    pub future_field_names_must_be_disjoint_from_support: bool,
    pub minimum_ordinal_layout_trap_pairs: usize,
    pub trap_requires_equal_candidate_action_set: bool,
    pub candidate_order_is_authority: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingFutureSourceContractV1 {
    pub source_kind: String,
    pub observation_source: BindingLabelObservationSourceV1,
    pub request_contract: String,
    pub planned_future_rows: usize,
    pub planned_session_slots: usize,
    pub rows_per_intervention: BTreeMap<String, usize>,
    pub slots: Vec<BindingFutureCaptureSlotV1>,
    pub challenge: BindingFutureChallengeContractV1,
    pub raw_payload_persisted: bool,
    pub expected_labels_available: bool,
    pub teacher_or_post_action_available: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingFutureAcquisitionProtocolV1 {
    pub schema: String,
    pub receipt_sha256: String,
    pub stop_id: String,
    pub pinned_support_commit_id: String,
    pub support_freeze_file_sha256: String,
    pub support_freeze_receipt_sha256: String,
    pub support_watermark_file_sha256: String,
    pub support_capture_index_sha256: String,
    pub support_watermark_next_sequence: u64,
    pub source: BindingFutureSourceContractV1,
    pub future_rows_captured: usize,
    pub expected_labels_joined: bool,
    pub h0_adjudicated: bool,
    pub h1_adjudicated: bool,
    pub protocol_mode_compiled: bool,
    pub f4_started: bool,
    pub execution_authority: bool,
}

impl BindingFutureAcquisitionProtocolV1 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BindingFutureCaptureErrorV1> {
        let mut bytes = serde_json::to_vec_pretty(self)
            .map_err(|_| BindingFutureCaptureErrorV1::Serialization)?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, BindingFutureCaptureErrorV1> {
        let protocol: Self = serde_json::from_slice(bytes)
            .map_err(|_| BindingFutureCaptureErrorV1::InvalidProtocol)?;
        if protocol.canonical_bytes()? != bytes {
            return Err(BindingFutureCaptureErrorV1::InvalidProtocol);
        }
        validate_protocol(&protocol)?;
        Ok(protocol)
    }

    pub fn validate(&self) -> Result<(), BindingFutureCaptureErrorV1> {
        validate_protocol(self)
    }
}

#[must_use]
pub fn binding_future_acquisition_protocol_v1() -> BindingFutureAcquisitionProtocolV1 {
    let mut protocol = BindingFutureAcquisitionProtocolV1 {
        schema: BINDING_FUTURE_ACQUISITION_PROTOCOL_SCHEMA_V1.to_owned(),
        receipt_sha256: String::new(),
        stop_id: "STOP-B1B-F0".to_owned(),
        pinned_support_commit_id: PINNED_SUPPORT_COMMIT_ID.to_owned(),
        support_freeze_file_sha256: PINNED_SUPPORT_FREEZE_FILE_SHA256.to_owned(),
        support_freeze_receipt_sha256: PINNED_SUPPORT_FREEZE_RECEIPT_SHA256.to_owned(),
        support_watermark_file_sha256: PINNED_SUPPORT_WATERMARK_FILE_SHA256.to_owned(),
        support_capture_index_sha256: PINNED_SUPPORT_CAPTURE_INDEX_SHA256.to_owned(),
        support_watermark_next_sequence: PINNED_SUPPORT_WATERMARK_NEXT_SEQUENCE,
        source: expected_source_contract(),
        future_rows_captured: 0,
        expected_labels_joined: false,
        h0_adjudicated: false,
        h1_adjudicated: false,
        protocol_mode_compiled: false,
        f4_started: false,
        execution_authority: false,
    };
    protocol.receipt_sha256 = protocol_digest(&protocol).expect("static F0 protocol serializes");
    protocol
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BindingFutureCaptureInputV1 {
    pub slot_id: String,
    pub capture_receipt: CaptureEvidenceReceipt,
    pub capture_record: EvidenceLedgerRecord,
    pub provider_payload: Value,
    pub context: PreActionBindingContextV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BindingFutureCaptureBatchV1 {
    schema: String,
    protocol_receipt_sha256: String,
    rows: Vec<BindingFutureCaptureInputV1>,
}

impl BindingFutureCaptureBatchV1 {
    pub fn new(
        protocol: &BindingFutureAcquisitionProtocolV1,
        rows: Vec<BindingFutureCaptureInputV1>,
    ) -> Result<Self, BindingFutureCaptureErrorV1> {
        let batch = Self {
            schema: BINDING_FUTURE_CAPTURE_BATCH_SCHEMA_V1.to_owned(),
            protocol_receipt_sha256: protocol.receipt_sha256.clone(),
            rows,
        };
        validate_future_batch(&batch, protocol)?;
        Ok(batch)
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BindingFutureCaptureErrorV1> {
        serde_json::to_vec(self).map_err(|_| BindingFutureCaptureErrorV1::Serialization)
    }

    pub fn from_canonical_bytes(
        bytes: &[u8],
        protocol: &BindingFutureAcquisitionProtocolV1,
    ) -> Result<Self, BindingFutureCaptureErrorV1> {
        let batch: Self =
            serde_json::from_slice(bytes).map_err(|_| BindingFutureCaptureErrorV1::InvalidBatch)?;
        if batch.canonical_bytes()? != bytes {
            return Err(BindingFutureCaptureErrorV1::InvalidBatch);
        }
        validate_future_batch(&batch, protocol)?;
        Ok(batch)
    }

    #[must_use]
    pub fn rows(&self) -> &[BindingFutureCaptureInputV1] {
        &self.rows
    }

    #[must_use]
    pub fn into_rows(self) -> Vec<BindingFutureCaptureInputV1> {
        self.rows
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct BindingFutureCaptureRowV1 {
    schema: String,
    row_sha256: String,
    protocol_receipt_sha256: String,
    slot_id: String,
    frozen_graph: FrozenCandidateRelationGraphV1,
    capture_receipt: CaptureEvidenceReceipt,
    capture_record: EvidenceLedgerRecord,
    pre_action_wire_root_sha256: String,
    session_lineage_sha256: String,
    wire_shape_root_sha256: String,
    field_name_root_sha256: String,
    candidate_action_set_root_sha256: String,
}

#[derive(Debug)]
pub struct BindingFutureCaptureOwnerV1 {
    protocol: BindingFutureAcquisitionProtocolV1,
    support_freeze: BindingSupportFreezeV1,
    capture_index: CaptureCommitmentIndex,
    support_shape_roots: BTreeSet<String>,
    support_field_names: BTreeSet<String>,
    support_session_lineages: BTreeSet<String>,
    rows: Vec<BindingFutureCaptureRowV1>,
}

impl BindingFutureCaptureOwnerV1 {
    pub fn new(
        protocol: BindingFutureAcquisitionProtocolV1,
        support_freeze_bytes: &[u8],
        support_watermark_bytes: &[u8],
        capture_index: CaptureCommitmentIndex,
    ) -> Result<Self, BindingFutureCaptureErrorV1> {
        validate_protocol(&protocol)?;
        let support_freeze =
            load_pinned_support(&protocol, support_freeze_bytes, support_watermark_bytes)?;
        capture_index
            .validate()
            .map_err(|_| BindingFutureCaptureErrorV1::InvalidCaptureIndex)?;
        validate_exact_prefix_extension(support_freeze.capture_index(), &capture_index)?;

        let support_records = support_freeze.support_capture_records();
        let support_shape_roots = support_records
            .iter()
            .map(|record| normalized_graph(record).and_then(wire_shape_root))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let support_field_names = support_records
            .iter()
            .map(|record| normalized_graph(record).map(field_name_hashes))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        let support_session_lineages = support_freeze.support_session_lineage_sha256s();

        Ok(Self {
            protocol,
            support_freeze,
            capture_index,
            support_shape_roots,
            support_field_names,
            support_session_lineages,
            rows: Vec::new(),
        })
    }

    pub fn capture_future(
        &mut self,
        input: BindingFutureCaptureInputV1,
    ) -> Result<(), BindingFutureCaptureErrorV1> {
        let slot = protocol_slot(&self.protocol, &input.slot_id)?;
        if self.rows.iter().any(|row| row.slot_id == input.slot_id) {
            return Err(BindingFutureCaptureErrorV1::DuplicateSlot);
        }
        validate_evidence_record(&input.capture_record)?;
        self.capture_index
            .verify_receipt(&input.capture_receipt)
            .map_err(|_| BindingFutureCaptureErrorV1::InvalidCaptureReceipt)?;
        if !input.capture_receipt.records.iter().any(|record| {
            record.sequence == input.capture_record.sequence
                && record.record_sha256 == input.capture_record.record_sha256
        }) {
            return Err(BindingFutureCaptureErrorV1::InvalidCaptureReceipt);
        }
        if input.capture_record.sequence < self.protocol.support_watermark_next_sequence {
            return Err(BindingFutureCaptureErrorV1::HistoricalFutureRow);
        }
        let graph = normalized_graph(&input.capture_record)?;
        let pre_action_wire_root_sha256 = graph.graph_sha256.clone();
        let session_lineage_sha256 = graph.session_id_sha256.clone();
        let payload_bytes = serde_json::to_vec(&input.provider_payload)
            .map_err(|_| BindingFutureCaptureErrorV1::Serialization)?;
        if evidence_payload_sha256(&payload_bytes) != graph.payload_sha256 {
            return Err(BindingFutureCaptureErrorV1::PayloadGraphMismatch);
        }
        if self
            .support_session_lineages
            .contains(&graph.session_id_sha256)
        {
            return Err(BindingFutureCaptureErrorV1::SupportFutureLineageOverlap);
        }

        let row_id_sha256 = sha256_json(&(
            self.protocol.receipt_sha256.as_str(),
            slot.slot_id.as_str(),
            input.capture_record.record_sha256.as_str(),
            "row",
        ))?;
        let evidence_ref_sha256 = sha256_json(&(
            self.protocol.receipt_sha256.as_str(),
            slot.slot_id.as_str(),
            input.capture_record.record_sha256.as_str(),
            "evidence",
        ))?;
        let frozen_graph = PreActionBindingSurfaceV1::capture(
            row_id_sha256,
            evidence_ref_sha256,
            FUTURE_REQUEST_TEXT_V1,
            &input.provider_payload,
            input.context,
            BindingEvidenceBudgetV1::default(),
        )
        .map_err(|_| BindingFutureCaptureErrorV1::InvalidFrozenGraph)?
        .candidate_relation_graph(BindingEvidenceBudgetV1::default())
        .map_err(|_| BindingFutureCaptureErrorV1::InvalidFrozenGraph)?
        .freeze()
        .map_err(|_| BindingFutureCaptureErrorV1::InvalidFrozenGraph)?;
        if frozen_graph.graph.extraction_budget_exhausted
            || frozen_graph.graph.relation_budget_exhausted
        {
            return Err(BindingFutureCaptureErrorV1::CaptureBudgetExceeded);
        }

        let wire_shape_root_sha256 = wire_shape_root(graph)?;
        let field_name_root_sha256 = sha256_json(&field_name_hashes(graph))?;
        let candidate_action_set = frozen_graph
            .graph
            .nodes
            .iter()
            .map(|node| node.action_equivalence_sha256.clone())
            .collect::<BTreeSet<_>>();
        let candidate_action_set_root_sha256 = sha256_json(&candidate_action_set)?;
        let mut row = BindingFutureCaptureRowV1 {
            schema: BINDING_FUTURE_CAPTURE_ROW_SCHEMA_V1.to_owned(),
            row_sha256: String::new(),
            protocol_receipt_sha256: self.protocol.receipt_sha256.clone(),
            slot_id: input.slot_id,
            frozen_graph,
            capture_receipt: input.capture_receipt,
            capture_record: input.capture_record,
            pre_action_wire_root_sha256,
            session_lineage_sha256,
            wire_shape_root_sha256,
            field_name_root_sha256,
            candidate_action_set_root_sha256,
        };
        row.row_sha256 = future_row_digest(&row)?;
        validate_future_row(&row, &self.protocol, &self.capture_index)?;
        self.rows.push(row);
        Ok(())
    }

    pub fn freeze(self) -> Result<BindingFutureCaptureFreezeV1, BindingFutureCaptureErrorV1> {
        BindingFutureCaptureFreezeV1::seal(self)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingFutureCaptureFreezeV1 {
    schema: String,
    receipt_sha256: String,
    protocol: BindingFutureAcquisitionProtocolV1,
    support_freeze_receipt_sha256: String,
    support_watermark_file_sha256: String,
    capture_index: CaptureCommitmentIndex,
    future_rows_root_sha256: String,
    future_graphs_root_sha256: String,
    future_lineages_root_sha256: String,
    future_rows: Vec<BindingFutureCaptureRowV1>,
    future_session_lineages: usize,
    distinct_future_shape_roots: usize,
    ordinal_layout_trap_pairs: usize,
    expected_labels_joined: bool,
    h0_adjudicated: bool,
    h1_adjudicated: bool,
    protocol_mode_compiled: bool,
    execution_authority: bool,
}

impl BindingFutureCaptureFreezeV1 {
    fn seal(mut owner: BindingFutureCaptureOwnerV1) -> Result<Self, BindingFutureCaptureErrorV1> {
        let support_freeze = owner.support_freeze.clone();
        owner
            .rows
            .sort_by(|left, right| left.slot_id.cmp(&right.slot_id));
        validate_complete_future_rows(&owner)?;

        let lineages = owner
            .rows
            .iter()
            .map(|row| row.session_lineage_sha256.clone())
            .collect::<BTreeSet<_>>();
        let shape_roots = owner
            .rows
            .iter()
            .map(|row| row.wire_shape_root_sha256.clone())
            .collect::<BTreeSet<_>>();
        let trap_pairs = ordinal_layout_trap_pairs(&owner.rows, &owner.protocol);
        let row_roots = owner
            .rows
            .iter()
            .map(|row| row.row_sha256.as_str())
            .collect::<Vec<_>>();
        let graph_roots = owner
            .rows
            .iter()
            .map(|row| row.frozen_graph.graph_root_sha256.as_str())
            .collect::<Vec<_>>();
        let mut freeze = Self {
            schema: BINDING_FUTURE_CAPTURE_FREEZE_SCHEMA_V1.to_owned(),
            receipt_sha256: String::new(),
            protocol: owner.protocol,
            support_freeze_receipt_sha256: owner.support_freeze.receipt_sha256().to_owned(),
            support_watermark_file_sha256: PINNED_SUPPORT_WATERMARK_FILE_SHA256.to_owned(),
            capture_index: owner.capture_index,
            future_rows_root_sha256: sha256_json(&row_roots)?,
            future_graphs_root_sha256: sha256_json(&graph_roots)?,
            future_lineages_root_sha256: sha256_json(&lineages)?,
            future_rows: owner.rows,
            future_session_lineages: lineages.len(),
            distinct_future_shape_roots: shape_roots.len(),
            ordinal_layout_trap_pairs: trap_pairs,
            expected_labels_joined: false,
            h0_adjudicated: false,
            h1_adjudicated: false,
            protocol_mode_compiled: false,
            execution_authority: false,
        };
        freeze.receipt_sha256 = future_freeze_digest(&freeze)?;
        validate_future_freeze(&freeze, &support_freeze)?;
        Ok(freeze)
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BindingFutureCaptureErrorV1> {
        serde_json::to_vec(self).map_err(|_| BindingFutureCaptureErrorV1::Serialization)
    }

    /// Restores only against a receipt persisted outside the serialized bundle.
    pub fn from_canonical_bytes(
        bytes: &[u8],
        trusted_future_receipt_sha256: &str,
        support_freeze_bytes: &[u8],
        support_watermark_bytes: &[u8],
    ) -> Result<Self, BindingFutureCaptureErrorV1> {
        let freeze: Self = serde_json::from_slice(bytes)
            .map_err(|_| BindingFutureCaptureErrorV1::InvalidFreeze)?;
        if freeze.canonical_bytes()? != bytes
            || !is_sha256(trusted_future_receipt_sha256)
            || freeze.receipt_sha256 != trusted_future_receipt_sha256
        {
            return Err(BindingFutureCaptureErrorV1::InvalidFreeze);
        }
        let support_freeze = load_pinned_support(
            &freeze.protocol,
            support_freeze_bytes,
            support_watermark_bytes,
        )?;
        validate_future_freeze(&freeze, &support_freeze)?;
        Ok(freeze)
    }

    #[must_use]
    pub fn receipt_sha256(&self) -> &str {
        &self.receipt_sha256
    }

    #[must_use]
    pub fn report(&self) -> BindingFutureCaptureReportV1 {
        BindingFutureCaptureReportV1 {
            schema: BINDING_FUTURE_CAPTURE_REPORT_SCHEMA_V1.to_owned(),
            stop_id: "STOP-B1B-F".to_owned(),
            protocol_receipt_sha256: self.protocol.receipt_sha256.clone(),
            support_freeze_receipt_sha256: self.support_freeze_receipt_sha256.clone(),
            capture_index_sha256: self.capture_index.index_sha256.clone(),
            future_rows: self.future_rows.len(),
            future_session_lineages: self.future_session_lineages,
            distinct_future_shape_roots: self.distinct_future_shape_roots,
            ordinal_layout_trap_pairs: self.ordinal_layout_trap_pairs,
            expected_labels_joined: self.expected_labels_joined,
            h0_status: "UNPROVEN".to_owned(),
            h1_status: "UNPROVEN".to_owned(),
            adjudication_status: "NOT_STARTED".to_owned(),
            f4_status: "BLOCKED".to_owned(),
            execution_authority: self.execution_authority,
        }
    }

    pub(crate) fn capture_index(&self) -> &CaptureCommitmentIndex {
        &self.capture_index
    }

    pub(crate) fn future_label_rows(&self) -> &[BindingFutureCaptureRowV1] {
        &self.future_rows
    }

    pub(crate) fn protocol(&self) -> &BindingFutureAcquisitionProtocolV1 {
        &self.protocol
    }
}

impl BindingFutureCaptureRowV1 {
    pub(crate) fn slot_id(&self) -> &str {
        &self.slot_id
    }

    pub(crate) fn frozen_graph(&self) -> &FrozenCandidateRelationGraphV1 {
        &self.frozen_graph
    }

    pub(crate) fn capture_receipt(&self) -> &CaptureEvidenceReceipt {
        &self.capture_receipt
    }

    pub(crate) fn capture_record(&self) -> &EvidenceLedgerRecord {
        &self.capture_record
    }

    pub(crate) fn pre_action_wire_root_sha256(&self) -> &str {
        &self.pre_action_wire_root_sha256
    }

    pub(crate) fn session_lineage_sha256(&self) -> &str {
        &self.session_lineage_sha256
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingFutureCaptureReportV1 {
    pub schema: String,
    pub stop_id: String,
    pub protocol_receipt_sha256: String,
    pub support_freeze_receipt_sha256: String,
    pub capture_index_sha256: String,
    pub future_rows: usize,
    pub future_session_lineages: usize,
    pub distinct_future_shape_roots: usize,
    pub ordinal_layout_trap_pairs: usize,
    pub expected_labels_joined: bool,
    pub h0_status: String,
    pub h1_status: String,
    pub adjudication_status: String,
    pub f4_status: String,
    pub execution_authority: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingFutureCaptureErrorV1 {
    InvalidProtocol,
    InvalidBatch,
    InvalidPinnedSupport,
    InvalidCaptureIndex,
    CaptureIndexNotExtension,
    InvalidCaptureChronology,
    InvalidCaptureReceipt,
    InvalidEvidenceRecord,
    HistoricalFutureRow,
    SupportFutureLineageOverlap,
    PayloadGraphMismatch,
    InvalidFrozenGraph,
    CaptureBudgetExceeded,
    UnknownSlot,
    DuplicateSlot,
    MissingSlot,
    InvalidSessionPartition,
    MissingInterventionDenominator,
    MissingShapeChallenge,
    MissingOrdinalLayoutTrap,
    InvalidRow,
    InvalidFreeze,
    Serialization,
}

fn expected_source_contract() -> BindingFutureSourceContractV1 {
    let slots = FUTURE_SLOT_SCHEDULE_V1
        .iter()
        .map(
            |(slot_id, intervention_id, session_slot)| BindingFutureCaptureSlotV1 {
                slot_id: (*slot_id).to_owned(),
                intervention_id: (*intervention_id).to_owned(),
                session_slot: (*session_slot).to_owned(),
            },
        )
        .collect::<Vec<_>>();
    let mut rows_per_intervention = BTreeMap::new();
    for slot in &slots {
        *rows_per_intervention
            .entry(slot.intervention_id.clone())
            .or_default() += 1;
    }
    BindingFutureSourceContractV1 {
        source_kind: "controlled_label_blind_post_freeze_interventions_v1".to_owned(),
        observation_source: BindingLabelObservationSourceV1::PreActionWire,
        request_contract: "fixed_continue_active_execution".to_owned(),
        planned_future_rows: FUTURE_ROWS_V1,
        planned_session_slots: FUTURE_SESSION_SLOTS_V1,
        rows_per_intervention,
        slots,
        challenge: BindingFutureChallengeContractV1 {
            minimum_distinct_future_shape_roots: MIN_DISTINCT_FUTURE_SHAPES_V1,
            future_shape_must_be_absent_from_support: true,
            future_field_names_must_be_disjoint_from_support: true,
            minimum_ordinal_layout_trap_pairs: MIN_ORDINAL_LAYOUT_TRAP_PAIRS_V1,
            trap_requires_equal_candidate_action_set: true,
            candidate_order_is_authority: false,
        },
        raw_payload_persisted: false,
        expected_labels_available: false,
        teacher_or_post_action_available: false,
    }
}

fn validate_protocol(
    protocol: &BindingFutureAcquisitionProtocolV1,
) -> Result<(), BindingFutureCaptureErrorV1> {
    if protocol.schema != BINDING_FUTURE_ACQUISITION_PROTOCOL_SCHEMA_V1
        || protocol.stop_id != "STOP-B1B-F0"
        || protocol.pinned_support_commit_id != PINNED_SUPPORT_COMMIT_ID
        || protocol.support_freeze_file_sha256 != PINNED_SUPPORT_FREEZE_FILE_SHA256
        || protocol.support_freeze_receipt_sha256 != PINNED_SUPPORT_FREEZE_RECEIPT_SHA256
        || protocol.support_watermark_file_sha256 != PINNED_SUPPORT_WATERMARK_FILE_SHA256
        || protocol.support_capture_index_sha256 != PINNED_SUPPORT_CAPTURE_INDEX_SHA256
        || protocol.support_watermark_next_sequence != PINNED_SUPPORT_WATERMARK_NEXT_SEQUENCE
        || protocol.source != expected_source_contract()
        || protocol.future_rows_captured != 0
        || protocol.expected_labels_joined
        || protocol.h0_adjudicated
        || protocol.h1_adjudicated
        || protocol.protocol_mode_compiled
        || protocol.f4_started
        || protocol.execution_authority
        || protocol.receipt_sha256 != protocol_digest(protocol)?
    {
        return Err(BindingFutureCaptureErrorV1::InvalidProtocol);
    }
    Ok(())
}

fn load_pinned_support(
    protocol: &BindingFutureAcquisitionProtocolV1,
    support_freeze_bytes: &[u8],
    support_watermark_bytes: &[u8],
) -> Result<BindingSupportFreezeV1, BindingFutureCaptureErrorV1> {
    if sha256_bytes(support_freeze_bytes) != protocol.support_freeze_file_sha256
        || sha256_bytes(support_watermark_bytes) != protocol.support_watermark_file_sha256
    {
        return Err(BindingFutureCaptureErrorV1::InvalidPinnedSupport);
    }
    let support_freeze = BindingSupportFreezeV1::from_canonical_bytes(support_freeze_bytes)
        .map_err(|_| BindingFutureCaptureErrorV1::InvalidPinnedSupport)?;
    if support_freeze.receipt_sha256() != protocol.support_freeze_receipt_sha256
        || support_freeze.capture_index().index_sha256 != protocol.support_capture_index_sha256
        || support_freeze.watermark_next_sequence() != protocol.support_watermark_next_sequence
        || support_freeze
            .watermark_canonical_bytes()
            .map_err(|_| BindingFutureCaptureErrorV1::InvalidPinnedSupport)?
            != support_watermark_bytes
    {
        return Err(BindingFutureCaptureErrorV1::InvalidPinnedSupport);
    }
    Ok(support_freeze)
}

fn protocol_slot<'a>(
    protocol: &'a BindingFutureAcquisitionProtocolV1,
    slot_id: &str,
) -> Result<&'a BindingFutureCaptureSlotV1, BindingFutureCaptureErrorV1> {
    protocol
        .source
        .slots
        .iter()
        .find(|slot| slot.slot_id == slot_id)
        .ok_or(BindingFutureCaptureErrorV1::UnknownSlot)
}

fn validate_future_batch(
    batch: &BindingFutureCaptureBatchV1,
    protocol: &BindingFutureAcquisitionProtocolV1,
) -> Result<(), BindingFutureCaptureErrorV1> {
    validate_protocol(protocol)?;
    let row_slots = batch
        .rows
        .iter()
        .map(|row| row.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    let protocol_slots = protocol
        .source
        .slots
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    if batch.schema != BINDING_FUTURE_CAPTURE_BATCH_SCHEMA_V1
        || batch.protocol_receipt_sha256 != protocol.receipt_sha256
        || batch.rows.len() != FUTURE_ROWS_V1
        || row_slots.len() != batch.rows.len()
        || row_slots != protocol_slots
    {
        return Err(BindingFutureCaptureErrorV1::InvalidBatch);
    }
    Ok(())
}

fn validate_exact_prefix_extension(
    support: &CaptureCommitmentIndex,
    current: &CaptureCommitmentIndex,
) -> Result<(), BindingFutureCaptureErrorV1> {
    if current.records.len() != support.records.len() + FUTURE_ROWS_V1
        || current.records.get(..support.records.len()) != Some(support.records.as_slice())
    {
        return Err(BindingFutureCaptureErrorV1::CaptureIndexNotExtension);
    }
    for (offset, record) in current.records[support.records.len()..].iter().enumerate() {
        if record.sequence != PINNED_SUPPORT_WATERMARK_NEXT_SEQUENCE + offset as u64 {
            return Err(BindingFutureCaptureErrorV1::InvalidCaptureChronology);
        }
    }
    Ok(())
}

fn validate_complete_future_rows(
    owner: &BindingFutureCaptureOwnerV1,
) -> Result<(), BindingFutureCaptureErrorV1> {
    if owner.rows.len() != FUTURE_ROWS_V1 {
        return Err(BindingFutureCaptureErrorV1::MissingSlot);
    }
    let row_slots = owner
        .rows
        .iter()
        .map(|row| row.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    let protocol_slots = owner
        .protocol
        .source
        .slots
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    if row_slots != protocol_slots {
        return Err(BindingFutureCaptureErrorV1::MissingSlot);
    }

    let mut intervention_counts = BTreeMap::new();
    let mut session_slot_lineages: BTreeMap<&str, &str> = BTreeMap::new();
    let mut lineage_session_slots: BTreeMap<&str, &str> = BTreeMap::new();
    let mut records = owner.rows.iter().collect::<Vec<_>>();
    records.sort_by_key(|row| row.capture_record.sequence);
    let support_tail = owner
        .support_freeze
        .capture_index()
        .records
        .last()
        .ok_or(BindingFutureCaptureErrorV1::InvalidPinnedSupport)?;
    let mut previous_record_sha256 = support_tail.record_sha256.as_str();

    for row in records {
        validate_future_row(row, &owner.protocol, &owner.capture_index)?;
        if row.capture_record.previous_record_sha256 != previous_record_sha256 {
            return Err(BindingFutureCaptureErrorV1::InvalidCaptureChronology);
        }
        previous_record_sha256 = &row.capture_record.record_sha256;
        let slot = protocol_slot(&owner.protocol, &row.slot_id)?;
        *intervention_counts
            .entry(slot.intervention_id.clone())
            .or_default() += 1;
        if session_slot_lineages
            .insert(&slot.session_slot, &row.session_lineage_sha256)
            .is_some_and(|existing| existing != row.session_lineage_sha256)
        {
            return Err(BindingFutureCaptureErrorV1::InvalidSessionPartition);
        }
        if lineage_session_slots
            .insert(&row.session_lineage_sha256, &slot.session_slot)
            .is_some_and(|existing| existing != slot.session_slot)
        {
            return Err(BindingFutureCaptureErrorV1::InvalidSessionPartition);
        }
        let graph = normalized_graph(&row.capture_record)?;
        if field_name_hashes(graph)
            .iter()
            .any(|name| owner.support_field_names.contains(name))
        {
            return Err(BindingFutureCaptureErrorV1::MissingShapeChallenge);
        }
        if owner
            .support_shape_roots
            .contains(&row.wire_shape_root_sha256)
        {
            return Err(BindingFutureCaptureErrorV1::MissingShapeChallenge);
        }
    }
    if intervention_counts != owner.protocol.source.rows_per_intervention {
        return Err(BindingFutureCaptureErrorV1::MissingInterventionDenominator);
    }
    if session_slot_lineages.len() != FUTURE_SESSION_SLOTS_V1
        || lineage_session_slots.len() != FUTURE_SESSION_SLOTS_V1
    {
        return Err(BindingFutureCaptureErrorV1::InvalidSessionPartition);
    }
    let shape_roots = owner
        .rows
        .iter()
        .map(|row| row.wire_shape_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    if shape_roots.len()
        < owner
            .protocol
            .source
            .challenge
            .minimum_distinct_future_shape_roots
    {
        return Err(BindingFutureCaptureErrorV1::MissingShapeChallenge);
    }
    if ordinal_layout_trap_pairs(&owner.rows, &owner.protocol)
        < owner
            .protocol
            .source
            .challenge
            .minimum_ordinal_layout_trap_pairs
    {
        return Err(BindingFutureCaptureErrorV1::MissingOrdinalLayoutTrap);
    }
    Ok(())
}

fn validate_future_row(
    row: &BindingFutureCaptureRowV1,
    protocol: &BindingFutureAcquisitionProtocolV1,
    capture_index: &CaptureCommitmentIndex,
) -> Result<(), BindingFutureCaptureErrorV1> {
    if row.schema != BINDING_FUTURE_CAPTURE_ROW_SCHEMA_V1
        || row.protocol_receipt_sha256 != protocol.receipt_sha256
        || row.capture_record.sequence < protocol.support_watermark_next_sequence
        || row.row_sha256 != future_row_digest(row)?
        || !is_sha256(&row.pre_action_wire_root_sha256)
        || !is_sha256(&row.session_lineage_sha256)
        || !is_sha256(&row.wire_shape_root_sha256)
        || !is_sha256(&row.field_name_root_sha256)
        || !is_sha256(&row.candidate_action_set_root_sha256)
    {
        return Err(BindingFutureCaptureErrorV1::InvalidRow);
    }
    protocol_slot(protocol, &row.slot_id)?;
    validate_evidence_record(&row.capture_record)?;
    capture_index
        .verify_receipt(&row.capture_receipt)
        .map_err(|_| BindingFutureCaptureErrorV1::InvalidCaptureReceipt)?;
    if !row.capture_receipt.records.iter().any(|record| {
        record.sequence == row.capture_record.sequence
            && record.record_sha256 == row.capture_record.record_sha256
    }) {
        return Err(BindingFutureCaptureErrorV1::InvalidCaptureReceipt);
    }
    let graph = normalized_graph(&row.capture_record)?;
    let expected_row_id_sha256 = sha256_json(&(
        protocol.receipt_sha256.as_str(),
        row.slot_id.as_str(),
        row.capture_record.record_sha256.as_str(),
        "row",
    ))?;
    let expected_evidence_ref_sha256 = sha256_json(&(
        protocol.receipt_sha256.as_str(),
        row.slot_id.as_str(),
        row.capture_record.record_sha256.as_str(),
        "evidence",
    ))?;
    let refrozen = row
        .frozen_graph
        .graph
        .clone()
        .freeze()
        .map_err(|_| BindingFutureCaptureErrorV1::InvalidFrozenGraph)?;
    let candidates = row
        .frozen_graph
        .graph
        .nodes
        .iter()
        .map(|node| node.action_equivalence_sha256.clone())
        .collect::<BTreeSet<_>>();
    if refrozen != row.frozen_graph
        || row.frozen_graph.graph.row_id_sha256 != expected_row_id_sha256
        || row.frozen_graph.graph.evidence_ref_sha256 != expected_evidence_ref_sha256
        || graph.graph_sha256 != row.pre_action_wire_root_sha256
        || graph.session_id_sha256 != row.session_lineage_sha256
        || wire_shape_root(graph)? != row.wire_shape_root_sha256
        || sha256_json(&field_name_hashes(graph))? != row.field_name_root_sha256
        || sha256_json(&candidates)? != row.candidate_action_set_root_sha256
    {
        return Err(BindingFutureCaptureErrorV1::InvalidRow);
    }
    Ok(())
}

fn validate_future_freeze(
    freeze: &BindingFutureCaptureFreezeV1,
    support_freeze: &BindingSupportFreezeV1,
) -> Result<(), BindingFutureCaptureErrorV1> {
    validate_protocol(&freeze.protocol)?;
    freeze
        .capture_index
        .validate()
        .map_err(|_| BindingFutureCaptureErrorV1::InvalidCaptureIndex)?;
    validate_exact_prefix_extension(support_freeze.capture_index(), &freeze.capture_index)?;

    let owner = BindingFutureCaptureOwnerV1 {
        protocol: freeze.protocol.clone(),
        support_freeze: support_freeze.clone(),
        capture_index: freeze.capture_index.clone(),
        support_shape_roots: support_freeze
            .support_capture_records()
            .iter()
            .map(|record| normalized_graph(record).and_then(wire_shape_root))
            .collect::<Result<_, _>>()?,
        support_field_names: support_freeze
            .support_capture_records()
            .iter()
            .map(|record| normalized_graph(record).map(field_name_hashes))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect(),
        support_session_lineages: support_freeze.support_session_lineage_sha256s(),
        rows: freeze.future_rows.clone(),
    };
    validate_complete_future_rows(&owner)?;

    let row_roots = freeze
        .future_rows
        .iter()
        .map(|row| row.row_sha256.as_str())
        .collect::<Vec<_>>();
    let graph_roots = freeze
        .future_rows
        .iter()
        .map(|row| row.frozen_graph.graph_root_sha256.as_str())
        .collect::<Vec<_>>();
    let lineages = freeze
        .future_rows
        .iter()
        .map(|row| row.session_lineage_sha256.clone())
        .collect::<BTreeSet<_>>();
    let shape_roots = freeze
        .future_rows
        .iter()
        .map(|row| row.wire_shape_root_sha256.clone())
        .collect::<BTreeSet<_>>();
    for row in &freeze.future_rows {
        validate_future_row(row, &freeze.protocol, &freeze.capture_index)?;
    }
    if freeze.schema != BINDING_FUTURE_CAPTURE_FREEZE_SCHEMA_V1
        || freeze
            .future_rows
            .windows(2)
            .any(|rows| rows[0].slot_id >= rows[1].slot_id)
        || freeze.receipt_sha256 != future_freeze_digest(freeze)?
        || freeze.support_freeze_receipt_sha256 != PINNED_SUPPORT_FREEZE_RECEIPT_SHA256
        || freeze.support_watermark_file_sha256 != PINNED_SUPPORT_WATERMARK_FILE_SHA256
        || freeze.future_rows.len() != FUTURE_ROWS_V1
        || freeze.future_rows_root_sha256 != sha256_json(&row_roots)?
        || freeze.future_graphs_root_sha256 != sha256_json(&graph_roots)?
        || freeze.future_lineages_root_sha256 != sha256_json(&lineages)?
        || freeze.future_session_lineages != lineages.len()
        || freeze.distinct_future_shape_roots != shape_roots.len()
        || freeze.ordinal_layout_trap_pairs
            != ordinal_layout_trap_pairs(&freeze.future_rows, &freeze.protocol)
        || freeze.expected_labels_joined
        || freeze.h0_adjudicated
        || freeze.h1_adjudicated
        || freeze.protocol_mode_compiled
        || freeze.execution_authority
    {
        return Err(BindingFutureCaptureErrorV1::InvalidFreeze);
    }
    Ok(())
}

fn ordinal_layout_trap_pairs(
    rows: &[BindingFutureCaptureRowV1],
    protocol: &BindingFutureAcquisitionProtocolV1,
) -> usize {
    let mut total = 0;
    for (index, left) in rows.iter().enumerate() {
        let Ok(left_slot) = protocol_slot(protocol, &left.slot_id) else {
            continue;
        };
        for right in rows.iter().skip(index + 1) {
            let Ok(right_slot) = protocol_slot(protocol, &right.slot_id) else {
                continue;
            };
            if left_slot.intervention_id == right_slot.intervention_id
                && left.candidate_action_set_root_sha256 == right.candidate_action_set_root_sha256
                && left.wire_shape_root_sha256 != right.wire_shape_root_sha256
            {
                total += 1;
            }
        }
    }
    total
}

fn normalized_graph(
    record: &EvidenceLedgerRecord,
) -> Result<&CanonicalEventGraph, BindingFutureCaptureErrorV1> {
    match &record.outcome {
        EvidenceIngestOutcome::Normalized { graph } => Ok(graph),
        _ => Err(BindingFutureCaptureErrorV1::InvalidEvidenceRecord),
    }
}

fn wire_shape_root(graph: &CanonicalEventGraph) -> Result<String, BindingFutureCaptureErrorV1> {
    #[derive(Serialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum ShapeNode<'a> {
        Null { path: &'a str },
        Boolean { path: &'a str },
        Number { path: &'a str },
        String { path: &'a str },
        ParsedJson { path: &'a str, source_path: &'a str },
        Array { path: &'a str, len: usize },
        Object { path: &'a str, len: usize },
        ObjectField { path: &'a str, ordinal: usize },
    }
    let nodes = graph
        .nodes
        .iter()
        .map(|node| match node {
            CanonicalEventNode::Null { path } => ShapeNode::Null { path },
            CanonicalEventNode::Boolean { path, .. } => ShapeNode::Boolean { path },
            CanonicalEventNode::Number { path, .. } => ShapeNode::Number { path },
            CanonicalEventNode::String { path, .. } => ShapeNode::String { path },
            CanonicalEventNode::ParsedJson { path, source_path } => {
                ShapeNode::ParsedJson { path, source_path }
            }
            CanonicalEventNode::Array { path, len } => ShapeNode::Array { path, len: *len },
            CanonicalEventNode::Object { path, len } => ShapeNode::Object { path, len: *len },
            CanonicalEventNode::ObjectField { path, ordinal, .. } => ShapeNode::ObjectField {
                path,
                ordinal: *ordinal,
            },
        })
        .collect::<Vec<_>>();
    sha256_json(&nodes)
}

fn field_name_hashes(graph: &CanonicalEventGraph) -> BTreeSet<String> {
    graph
        .nodes
        .iter()
        .filter_map(|node| match node {
            CanonicalEventNode::ObjectField { name_sha256, .. } => Some(name_sha256.clone()),
            _ => None,
        })
        .collect()
}

fn validate_evidence_record(
    record: &EvidenceLedgerRecord,
) -> Result<(), BindingFutureCaptureErrorV1> {
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        sequence: u64,
        previous_record_sha256: &'a str,
        outcome: &'a EvidenceIngestOutcome,
    }
    let expected = canonical_json_sha256(&DigestFields {
        schema: &record.schema,
        sequence: record.sequence,
        previous_record_sha256: &record.previous_record_sha256,
        outcome: &record.outcome,
    })
    .map_err(|_| BindingFutureCaptureErrorV1::Serialization)?;
    if record.schema != EVIDENCE_LEDGER_SCHEMA_V1
        || !is_sha256(&record.previous_record_sha256)
        || !is_sha256(&record.record_sha256)
        || record.record_sha256 != expected
    {
        return Err(BindingFutureCaptureErrorV1::InvalidEvidenceRecord);
    }
    Ok(())
}

fn protocol_digest(
    protocol: &BindingFutureAcquisitionProtocolV1,
) -> Result<String, BindingFutureCaptureErrorV1> {
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        stop_id: &'a str,
        pinned_support_commit_id: &'a str,
        support_freeze_file_sha256: &'a str,
        support_freeze_receipt_sha256: &'a str,
        support_watermark_file_sha256: &'a str,
        support_capture_index_sha256: &'a str,
        support_watermark_next_sequence: u64,
        source: &'a BindingFutureSourceContractV1,
        future_rows_captured: usize,
        expected_labels_joined: bool,
        h0_adjudicated: bool,
        h1_adjudicated: bool,
        protocol_mode_compiled: bool,
        f4_started: bool,
        execution_authority: bool,
    }
    sha256_json(&DigestFields {
        schema: &protocol.schema,
        stop_id: &protocol.stop_id,
        pinned_support_commit_id: &protocol.pinned_support_commit_id,
        support_freeze_file_sha256: &protocol.support_freeze_file_sha256,
        support_freeze_receipt_sha256: &protocol.support_freeze_receipt_sha256,
        support_watermark_file_sha256: &protocol.support_watermark_file_sha256,
        support_capture_index_sha256: &protocol.support_capture_index_sha256,
        support_watermark_next_sequence: protocol.support_watermark_next_sequence,
        source: &protocol.source,
        future_rows_captured: protocol.future_rows_captured,
        expected_labels_joined: protocol.expected_labels_joined,
        h0_adjudicated: protocol.h0_adjudicated,
        h1_adjudicated: protocol.h1_adjudicated,
        protocol_mode_compiled: protocol.protocol_mode_compiled,
        f4_started: protocol.f4_started,
        execution_authority: protocol.execution_authority,
    })
}

fn future_row_digest(
    row: &BindingFutureCaptureRowV1,
) -> Result<String, BindingFutureCaptureErrorV1> {
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        protocol_receipt_sha256: &'a str,
        slot_id: &'a str,
        frozen_graph: &'a FrozenCandidateRelationGraphV1,
        capture_receipt: &'a CaptureEvidenceReceipt,
        capture_record: &'a EvidenceLedgerRecord,
        pre_action_wire_root_sha256: &'a str,
        session_lineage_sha256: &'a str,
        wire_shape_root_sha256: &'a str,
        field_name_root_sha256: &'a str,
        candidate_action_set_root_sha256: &'a str,
    }
    sha256_json(&DigestFields {
        schema: &row.schema,
        protocol_receipt_sha256: &row.protocol_receipt_sha256,
        slot_id: &row.slot_id,
        frozen_graph: &row.frozen_graph,
        capture_receipt: &row.capture_receipt,
        capture_record: &row.capture_record,
        pre_action_wire_root_sha256: &row.pre_action_wire_root_sha256,
        session_lineage_sha256: &row.session_lineage_sha256,
        wire_shape_root_sha256: &row.wire_shape_root_sha256,
        field_name_root_sha256: &row.field_name_root_sha256,
        candidate_action_set_root_sha256: &row.candidate_action_set_root_sha256,
    })
}

fn future_freeze_digest(
    freeze: &BindingFutureCaptureFreezeV1,
) -> Result<String, BindingFutureCaptureErrorV1> {
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        protocol: &'a BindingFutureAcquisitionProtocolV1,
        support_freeze_receipt_sha256: &'a str,
        support_watermark_file_sha256: &'a str,
        capture_index: &'a CaptureCommitmentIndex,
        future_rows_root_sha256: &'a str,
        future_graphs_root_sha256: &'a str,
        future_lineages_root_sha256: &'a str,
        future_rows: &'a [BindingFutureCaptureRowV1],
        future_session_lineages: usize,
        distinct_future_shape_roots: usize,
        ordinal_layout_trap_pairs: usize,
        expected_labels_joined: bool,
        h0_adjudicated: bool,
        h1_adjudicated: bool,
        protocol_mode_compiled: bool,
        execution_authority: bool,
    }
    sha256_json(&DigestFields {
        schema: &freeze.schema,
        protocol: &freeze.protocol,
        support_freeze_receipt_sha256: &freeze.support_freeze_receipt_sha256,
        support_watermark_file_sha256: &freeze.support_watermark_file_sha256,
        capture_index: &freeze.capture_index,
        future_rows_root_sha256: &freeze.future_rows_root_sha256,
        future_graphs_root_sha256: &freeze.future_graphs_root_sha256,
        future_lineages_root_sha256: &freeze.future_lineages_root_sha256,
        future_rows: &freeze.future_rows,
        future_session_lineages: freeze.future_session_lineages,
        distinct_future_shape_roots: freeze.distinct_future_shape_roots,
        ordinal_layout_trap_pairs: freeze.ordinal_layout_trap_pairs,
        expected_labels_joined: freeze.expected_labels_joined,
        h0_adjudicated: freeze.h0_adjudicated,
        h1_adjudicated: freeze.h1_adjudicated,
        protocol_mode_compiled: freeze.protocol_mode_compiled,
        execution_authority: freeze.execution_authority,
    })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256_json<T: Serialize>(value: &T) -> Result<String, BindingFutureCaptureErrorV1> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_bytes(&bytes))
        .map_err(|_| BindingFutureCaptureErrorV1::Serialization)
}

#[cfg(test)]
#[path = "binding_evidence_future_capture_tests.rs"]
mod tests;
