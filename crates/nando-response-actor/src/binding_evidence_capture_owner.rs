//! Independent, label-blind support capture owner for B1B-S.
//!
//! The owner can only collect pre-action support observations. It has no
//! expected-label input and no future-opening API. Consuming `freeze` seals the
//! exact capture-index prefix that later B1B-F evidence must extend.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::binding_evidence::FrozenCandidateRelationGraphV1;
use super::binding_evidence_preregistration::{
    BINDING_CAPTURE_WATERMARK_SCHEMA_V1, BindingLabelObservationSourceV1,
    MIN_BINDING_APPLICABILITY_NEGATIVE_ROWS_PER_PARTITION_V1,
    MIN_BINDING_POSITIVE_ROWS_PER_PARTITION_V1, MIN_BINDING_SESSION_LINEAGES_PER_PARTITION_V1,
    UntrustedBindingCaptureWatermarkV1,
};
use crate::capture_provenance::{CaptureCommitmentIndex, CaptureEvidenceReceipt};
use crate::{
    EVIDENCE_LEDGER_SCHEMA_V1, EvidenceIngestOutcome, EvidenceLedgerRecord, canonical_json_sha256,
};

pub const BINDING_SUPPORT_CAPTURE_ROW_SCHEMA_V1: &str = "nando.binding-support-capture-row.v1";
pub const BINDING_SUPPORT_CAPTURE_BATCH_SCHEMA_V1: &str = "nando.binding-support-capture-batch.v1";
pub const BINDING_SUPPORT_FREEZE_SCHEMA_V1: &str = "nando.binding-support-freeze.v1";
pub const BINDING_SUPPORT_FREEZE_REPORT_SCHEMA_V1: &str = "nando.binding-support-freeze-report.v1";
pub const MAX_BINDING_SUPPORT_CAPTURE_ROWS_V1: usize = 16_384;
pub const MIN_BINDING_SUPPORT_CAPTURE_ROWS_V1: usize = MIN_BINDING_POSITIVE_ROWS_PER_PARTITION_V1
    + MIN_BINDING_APPLICABILITY_NEGATIVE_ROWS_PER_PARTITION_V1;

const BINDING_CAUSAL_INTERVENTION_IDS_V1: [&str; 6] = ["I1", "I2", "I3", "I4", "I5", "I6"];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingSupportCaptureRowV1 {
    schema: String,
    row_sha256: String,
    frozen_graph: FrozenCandidateRelationGraphV1,
    capture_receipt: CaptureEvidenceReceipt,
    capture_record: EvidenceLedgerRecord,
    pre_action_wire_root_sha256: String,
    observation_source: BindingLabelObservationSourceV1,
    intervention_id: String,
    session_lineage_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingSupportCaptureBatchV1 {
    pub schema: String,
    pub rows: Vec<BindingSupportCaptureRowV1>,
}

impl BindingSupportCaptureBatchV1 {
    pub fn new(
        rows: Vec<BindingSupportCaptureRowV1>,
    ) -> Result<Self, BindingSupportCaptureErrorV1> {
        if rows.len() < MIN_BINDING_SUPPORT_CAPTURE_ROWS_V1
            || rows.len() > MAX_BINDING_SUPPORT_CAPTURE_ROWS_V1
        {
            return Err(BindingSupportCaptureErrorV1::InvalidRowDenominator);
        }
        Ok(Self {
            schema: BINDING_SUPPORT_CAPTURE_BATCH_SCHEMA_V1.to_owned(),
            rows,
        })
    }

    pub fn validate(&self) -> Result<(), BindingSupportCaptureErrorV1> {
        if self.schema != BINDING_SUPPORT_CAPTURE_BATCH_SCHEMA_V1
            || self.rows.len() < MIN_BINDING_SUPPORT_CAPTURE_ROWS_V1
            || self.rows.len() > MAX_BINDING_SUPPORT_CAPTURE_ROWS_V1
        {
            return Err(BindingSupportCaptureErrorV1::InvalidRowDenominator);
        }
        for row in &self.rows {
            validate_support_capture_row(row)?;
        }
        Ok(())
    }
}

impl BindingSupportCaptureRowV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        frozen_graph: FrozenCandidateRelationGraphV1,
        capture_receipt: CaptureEvidenceReceipt,
        capture_record: EvidenceLedgerRecord,
        intervention_id: impl Into<String>,
    ) -> Result<Self, BindingSupportCaptureErrorV1> {
        let (pre_action_wire_root_sha256, session_lineage_sha256) = match &capture_record.outcome {
            EvidenceIngestOutcome::Normalized { graph } => {
                (graph.graph_sha256.clone(), graph.session_id_sha256.clone())
            }
            _ => return Err(BindingSupportCaptureErrorV1::InvalidObservationSource),
        };
        let mut row = Self {
            schema: BINDING_SUPPORT_CAPTURE_ROW_SCHEMA_V1.to_owned(),
            row_sha256: String::new(),
            frozen_graph,
            capture_receipt,
            capture_record,
            pre_action_wire_root_sha256,
            observation_source: BindingLabelObservationSourceV1::PreActionWire,
            intervention_id: intervention_id.into(),
            session_lineage_sha256,
        };
        row.row_sha256 = support_capture_row_digest(&row)?;
        validate_support_capture_row(&row)?;
        Ok(row)
    }

    #[must_use]
    pub fn row_id_sha256(&self) -> &str {
        &self.frozen_graph.graph.row_id_sha256
    }

    #[must_use]
    pub fn evidence_ref_sha256(&self) -> &str {
        &self.frozen_graph.graph.evidence_ref_sha256
    }

    #[must_use]
    pub fn graph_root_sha256(&self) -> &str {
        &self.frozen_graph.graph_root_sha256
    }

    #[must_use]
    pub fn row_sha256(&self) -> &str {
        &self.row_sha256
    }
}

#[derive(Debug)]
pub struct BindingSupportCaptureOwnerV1 {
    capture_index: CaptureCommitmentIndex,
    rows: Vec<BindingSupportCaptureRowV1>,
}

impl BindingSupportCaptureOwnerV1 {
    pub fn new(
        capture_index: CaptureCommitmentIndex,
    ) -> Result<Self, BindingSupportCaptureErrorV1> {
        capture_index
            .validate()
            .map_err(|_| BindingSupportCaptureErrorV1::InvalidCaptureIndex)?;
        Ok(Self {
            capture_index,
            rows: Vec::new(),
        })
    }

    pub fn capture_support(
        &mut self,
        row: BindingSupportCaptureRowV1,
    ) -> Result<(), BindingSupportCaptureErrorV1> {
        if self.rows.len() >= MAX_BINDING_SUPPORT_CAPTURE_ROWS_V1 {
            return Err(BindingSupportCaptureErrorV1::CaptureBudgetExceeded);
        }
        validate_row_against_index(&row, &self.capture_index)?;
        if self.rows.iter().any(|existing| {
            existing.row_id_sha256() == row.row_id_sha256()
                || existing.evidence_ref_sha256() == row.evidence_ref_sha256()
        }) {
            return Err(BindingSupportCaptureErrorV1::DuplicateRow);
        }
        self.rows.push(row);
        Ok(())
    }

    pub fn capture_batch(
        &mut self,
        batch: BindingSupportCaptureBatchV1,
    ) -> Result<(), BindingSupportCaptureErrorV1> {
        batch.validate()?;
        for row in batch.rows {
            self.capture_support(row)?;
        }
        Ok(())
    }

    pub fn freeze(self) -> Result<BindingSupportFreezeV1, BindingSupportCaptureErrorV1> {
        BindingSupportFreezeV1::seal(self.capture_index, self.rows)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingSupportFreezeV1 {
    schema: String,
    receipt_sha256: String,
    capture_index: CaptureCommitmentIndex,
    watermark: UntrustedBindingCaptureWatermarkV1,
    watermark_bytes_sha256: String,
    support_rows_root_sha256: String,
    support_graphs_root_sha256: String,
    support_lineages_root_sha256: String,
    support_rows: Vec<BindingSupportCaptureRowV1>,
    intervention_rows: BTreeMap<String, usize>,
    support_session_lineages: usize,
    expected_labels_joined: bool,
    h0_adjudicated: bool,
    h1_adjudicated: bool,
    future_opened: bool,
    protocol_mode_compiled: bool,
    execution_authority: bool,
}

impl BindingSupportFreezeV1 {
    fn seal(
        capture_index: CaptureCommitmentIndex,
        mut rows: Vec<BindingSupportCaptureRowV1>,
    ) -> Result<Self, BindingSupportCaptureErrorV1> {
        capture_index
            .validate()
            .map_err(|_| BindingSupportCaptureErrorV1::InvalidCaptureIndex)?;
        if rows.len() < MIN_BINDING_SUPPORT_CAPTURE_ROWS_V1
            || rows.len() > MAX_BINDING_SUPPORT_CAPTURE_ROWS_V1
        {
            return Err(BindingSupportCaptureErrorV1::InvalidRowDenominator);
        }
        rows.sort_by(|left, right| left.row_id_sha256().cmp(right.row_id_sha256()));
        validate_support_rows(&rows, &capture_index)?;

        let intervention_rows = intervention_counts(&rows);
        if BINDING_CAUSAL_INTERVENTION_IDS_V1
            .iter()
            .any(|id| intervention_rows.get(*id).copied().unwrap_or_default() == 0)
        {
            return Err(BindingSupportCaptureErrorV1::MissingInterventionDenominator);
        }
        let support_lineages = rows
            .iter()
            .map(|row| row.session_lineage_sha256.clone())
            .collect::<BTreeSet<_>>();
        if support_lineages.len() < MIN_BINDING_SESSION_LINEAGES_PER_PARTITION_V1 {
            return Err(BindingSupportCaptureErrorV1::MissingSessionLineageDenominator);
        }

        let watermark = UntrustedBindingCaptureWatermarkV1::new(capture_index.clone())
            .map_err(|_| BindingSupportCaptureErrorV1::InvalidWatermark)?;
        let watermark_bytes = watermark
            .canonical_bytes()
            .map_err(|_| BindingSupportCaptureErrorV1::Serialization)?;
        let row_roots = rows
            .iter()
            .map(|row| row.row_sha256.as_str())
            .collect::<Vec<_>>();
        let graph_roots = rows
            .iter()
            .map(|row| row.frozen_graph.graph_root_sha256.as_str())
            .collect::<Vec<_>>();
        let mut freeze = Self {
            schema: BINDING_SUPPORT_FREEZE_SCHEMA_V1.to_owned(),
            receipt_sha256: String::new(),
            capture_index,
            watermark,
            watermark_bytes_sha256: sha256_bytes(&watermark_bytes),
            support_rows_root_sha256: sha256_json(&row_roots)?,
            support_graphs_root_sha256: sha256_json(&graph_roots)?,
            support_lineages_root_sha256: sha256_json(&support_lineages)?,
            support_rows: rows,
            intervention_rows,
            support_session_lineages: support_lineages.len(),
            expected_labels_joined: false,
            h0_adjudicated: false,
            h1_adjudicated: false,
            future_opened: false,
            protocol_mode_compiled: false,
            execution_authority: false,
        };
        freeze.receipt_sha256 = support_freeze_digest(&freeze)?;
        validate_support_freeze(&freeze)?;
        Ok(freeze)
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BindingSupportCaptureErrorV1> {
        serde_json::to_vec(self).map_err(|_| BindingSupportCaptureErrorV1::Serialization)
    }

    pub fn watermark_canonical_bytes(&self) -> Result<Vec<u8>, BindingSupportCaptureErrorV1> {
        self.watermark
            .canonical_bytes()
            .map_err(|_| BindingSupportCaptureErrorV1::Serialization)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, BindingSupportCaptureErrorV1> {
        let freeze: Self = serde_json::from_slice(bytes)
            .map_err(|_| BindingSupportCaptureErrorV1::InvalidFreezeReceipt)?;
        if freeze.canonical_bytes()? != bytes {
            return Err(BindingSupportCaptureErrorV1::InvalidFreezeReceipt);
        }
        validate_support_freeze(&freeze)?;
        Ok(freeze)
    }

    #[must_use]
    pub fn report(&self) -> BindingSupportFreezeReportV1 {
        BindingSupportFreezeReportV1 {
            schema: BINDING_SUPPORT_FREEZE_REPORT_SCHEMA_V1.to_owned(),
            stop_id: "STOP-B1B-S".to_owned(),
            freeze_receipt_sha256: self.receipt_sha256.clone(),
            support_rows: self.support_rows.len(),
            support_rows_root_sha256: self.support_rows_root_sha256.clone(),
            support_graphs_root_sha256: self.support_graphs_root_sha256.clone(),
            support_lineages_root_sha256: self.support_lineages_root_sha256.clone(),
            support_session_lineages: self.support_session_lineages,
            intervention_rows: self.intervention_rows.clone(),
            capture_index_sha256: self.capture_index.index_sha256.clone(),
            watermark_bytes_sha256: self.watermark_bytes_sha256.clone(),
            watermark_next_sequence: self.watermark.next_sequence,
            expected_labels_joined: self.expected_labels_joined,
            support_label_denominators: "PENDING_TRUSTED_RESOLVER".to_owned(),
            h0_status: "UNPROVEN".to_owned(),
            h1_status: "UNPROVEN".to_owned(),
            future_status: "NOT_OPENED".to_owned(),
            acquisition_stage: "SUPPORT_FROZEN".to_owned(),
            f4_status: "BLOCKED".to_owned(),
            execution_authority: self.execution_authority,
        }
    }

    #[must_use]
    pub fn watermark_bytes_sha256(&self) -> &str {
        &self.watermark_bytes_sha256
    }

    #[must_use]
    pub fn watermark_next_sequence(&self) -> u64 {
        self.watermark.next_sequence
    }

    #[must_use]
    pub fn support_rows(&self) -> usize {
        self.support_rows.len()
    }

    #[must_use]
    pub fn receipt_sha256(&self) -> &str {
        &self.receipt_sha256
    }

    #[must_use]
    pub fn capture_index(&self) -> &CaptureCommitmentIndex {
        &self.capture_index
    }

    pub(crate) fn support_capture_records(&self) -> Vec<&EvidenceLedgerRecord> {
        self.support_rows
            .iter()
            .map(|row| &row.capture_record)
            .collect()
    }

    pub(crate) fn support_session_lineage_sha256s(&self) -> BTreeSet<String> {
        self.support_rows
            .iter()
            .map(|row| row.session_lineage_sha256.clone())
            .collect()
    }

    pub(crate) fn support_label_rows(&self) -> &[BindingSupportCaptureRowV1] {
        &self.support_rows
    }
}

impl BindingSupportCaptureRowV1 {
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

    pub(crate) fn intervention_id(&self) -> &str {
        &self.intervention_id
    }

    pub(crate) fn session_lineage_sha256(&self) -> &str {
        &self.session_lineage_sha256
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingSupportFreezeReportV1 {
    pub schema: String,
    pub stop_id: String,
    pub freeze_receipt_sha256: String,
    pub support_rows: usize,
    pub support_rows_root_sha256: String,
    pub support_graphs_root_sha256: String,
    pub support_lineages_root_sha256: String,
    pub support_session_lineages: usize,
    pub intervention_rows: BTreeMap<String, usize>,
    pub capture_index_sha256: String,
    pub watermark_bytes_sha256: String,
    pub watermark_next_sequence: u64,
    pub expected_labels_joined: bool,
    pub support_label_denominators: String,
    pub h0_status: String,
    pub h1_status: String,
    pub future_status: String,
    pub acquisition_stage: String,
    pub f4_status: String,
    pub execution_authority: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingSupportCaptureErrorV1 {
    InvalidDigest,
    InvalidCaptureIndex,
    InvalidCaptureReceipt,
    InvalidCaptureChronology,
    InvalidFrozenGraph,
    InvalidObservationSource,
    InvalidIntervention,
    InvalidWatermark,
    InvalidRowDenominator,
    MissingInterventionDenominator,
    MissingSessionLineageDenominator,
    DuplicateRow,
    DuplicateLineageVote,
    CaptureBudgetExceeded,
    InvalidFreezeReceipt,
    Serialization,
}

fn validate_support_capture_row(
    row: &BindingSupportCaptureRowV1,
) -> Result<(), BindingSupportCaptureErrorV1> {
    if row.schema != BINDING_SUPPORT_CAPTURE_ROW_SCHEMA_V1
        || !is_sha256(&row.row_sha256)
        || !is_sha256(&row.pre_action_wire_root_sha256)
        || !is_sha256(&row.session_lineage_sha256)
        || support_capture_row_digest(row)? != row.row_sha256
    {
        return Err(BindingSupportCaptureErrorV1::InvalidDigest);
    }
    if row.observation_source != BindingLabelObservationSourceV1::PreActionWire {
        return Err(BindingSupportCaptureErrorV1::InvalidObservationSource);
    }
    if !BINDING_CAUSAL_INTERVENTION_IDS_V1.contains(&row.intervention_id.as_str()) {
        return Err(BindingSupportCaptureErrorV1::InvalidIntervention);
    }
    let refrozen = row
        .frozen_graph
        .graph
        .clone()
        .freeze()
        .map_err(|_| BindingSupportCaptureErrorV1::InvalidFrozenGraph)?;
    if refrozen != row.frozen_graph {
        return Err(BindingSupportCaptureErrorV1::InvalidFrozenGraph);
    }
    row.capture_receipt
        .validate()
        .map_err(|_| BindingSupportCaptureErrorV1::InvalidCaptureReceipt)?;
    validate_evidence_record(&row.capture_record)?;
    let EvidenceIngestOutcome::Normalized { graph } = &row.capture_record.outcome else {
        return Err(BindingSupportCaptureErrorV1::InvalidObservationSource);
    };
    if graph.graph_sha256 != row.pre_action_wire_root_sha256
        || graph.session_id_sha256 != row.session_lineage_sha256
    {
        return Err(BindingSupportCaptureErrorV1::InvalidObservationSource);
    }
    if !row.capture_receipt.records.iter().any(|record| {
        record.sequence == row.capture_record.sequence
            && record.record_sha256 == row.capture_record.record_sha256
    }) {
        return Err(BindingSupportCaptureErrorV1::InvalidCaptureReceipt);
    }
    Ok(())
}

fn validate_row_against_index(
    row: &BindingSupportCaptureRowV1,
    capture_index: &CaptureCommitmentIndex,
) -> Result<(), BindingSupportCaptureErrorV1> {
    validate_support_capture_row(row)?;
    capture_index
        .verify_receipt(&row.capture_receipt)
        .map_err(|_| BindingSupportCaptureErrorV1::InvalidCaptureReceipt)?;
    if !capture_index.records.iter().any(|record| {
        record.sequence == row.capture_record.sequence
            && record.record_sha256 == row.capture_record.record_sha256
    }) {
        return Err(BindingSupportCaptureErrorV1::InvalidCaptureChronology);
    }
    Ok(())
}

fn validate_support_rows(
    rows: &[BindingSupportCaptureRowV1],
    capture_index: &CaptureCommitmentIndex,
) -> Result<(), BindingSupportCaptureErrorV1> {
    let mut row_ids = BTreeSet::new();
    let mut evidence_refs = BTreeSet::new();
    let mut lineage_votes = BTreeSet::new();
    for row in rows {
        validate_row_against_index(row, capture_index)?;
        if !row_ids.insert(row.row_id_sha256()) || !evidence_refs.insert(row.evidence_ref_sha256())
        {
            return Err(BindingSupportCaptureErrorV1::DuplicateRow);
        }
        if !lineage_votes.insert((
            row.intervention_id.as_str(),
            row.session_lineage_sha256.as_str(),
        )) {
            return Err(BindingSupportCaptureErrorV1::DuplicateLineageVote);
        }
    }
    Ok(())
}

fn validate_support_freeze(
    freeze: &BindingSupportFreezeV1,
) -> Result<(), BindingSupportCaptureErrorV1> {
    freeze
        .capture_index
        .validate()
        .map_err(|_| BindingSupportCaptureErrorV1::InvalidCaptureIndex)?;
    validate_support_rows(&freeze.support_rows, &freeze.capture_index)?;
    if freeze.support_rows.len() < MIN_BINDING_SUPPORT_CAPTURE_ROWS_V1
        || freeze.support_rows.len() > MAX_BINDING_SUPPORT_CAPTURE_ROWS_V1
        || freeze
            .support_rows
            .windows(2)
            .any(|pair| pair[0].row_id_sha256() >= pair[1].row_id_sha256())
    {
        return Err(BindingSupportCaptureErrorV1::InvalidRowDenominator);
    }
    let expected_interventions = intervention_counts(&freeze.support_rows);
    if BINDING_CAUSAL_INTERVENTION_IDS_V1
        .iter()
        .any(|id| expected_interventions.get(*id).copied().unwrap_or_default() == 0)
    {
        return Err(BindingSupportCaptureErrorV1::MissingInterventionDenominator);
    }
    let support_lineages = freeze
        .support_rows
        .iter()
        .map(|row| row.session_lineage_sha256.clone())
        .collect::<BTreeSet<_>>();
    if support_lineages.len() < MIN_BINDING_SESSION_LINEAGES_PER_PARTITION_V1 {
        return Err(BindingSupportCaptureErrorV1::MissingSessionLineageDenominator);
    }
    let expected_watermark = UntrustedBindingCaptureWatermarkV1::new(freeze.capture_index.clone())
        .map_err(|_| BindingSupportCaptureErrorV1::InvalidWatermark)?;
    let expected_watermark_bytes = expected_watermark
        .canonical_bytes()
        .map_err(|_| BindingSupportCaptureErrorV1::Serialization)?;
    let row_roots = freeze
        .support_rows
        .iter()
        .map(|row| row.row_sha256.as_str())
        .collect::<Vec<_>>();
    let graph_roots = freeze
        .support_rows
        .iter()
        .map(|row| row.frozen_graph.graph_root_sha256.as_str())
        .collect::<Vec<_>>();
    if freeze.schema != BINDING_SUPPORT_FREEZE_SCHEMA_V1
        || freeze.watermark.schema != BINDING_CAPTURE_WATERMARK_SCHEMA_V1
        || freeze.receipt_sha256 != support_freeze_digest(freeze)?
        || freeze.watermark != expected_watermark
        || freeze.watermark_bytes_sha256 != sha256_bytes(&expected_watermark_bytes)
        || freeze.support_rows_root_sha256 != sha256_json(&row_roots)?
        || freeze.support_graphs_root_sha256 != sha256_json(&graph_roots)?
        || freeze.support_lineages_root_sha256 != sha256_json(&support_lineages)?
        || freeze.intervention_rows != expected_interventions
        || freeze.support_session_lineages != support_lineages.len()
        || freeze.expected_labels_joined
        || freeze.h0_adjudicated
        || freeze.h1_adjudicated
        || freeze.future_opened
        || freeze.protocol_mode_compiled
        || freeze.execution_authority
    {
        return Err(BindingSupportCaptureErrorV1::InvalidFreezeReceipt);
    }
    Ok(())
}

fn intervention_counts(rows: &[BindingSupportCaptureRowV1]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for row in rows {
        *counts.entry(row.intervention_id.clone()).or_default() += 1;
    }
    counts
}

fn support_capture_row_digest(
    row: &BindingSupportCaptureRowV1,
) -> Result<String, BindingSupportCaptureErrorV1> {
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        frozen_graph: &'a FrozenCandidateRelationGraphV1,
        capture_receipt: &'a CaptureEvidenceReceipt,
        capture_record: &'a EvidenceLedgerRecord,
        pre_action_wire_root_sha256: &'a str,
        observation_source: BindingLabelObservationSourceV1,
        intervention_id: &'a str,
        session_lineage_sha256: &'a str,
    }
    sha256_json(&DigestFields {
        schema: &row.schema,
        frozen_graph: &row.frozen_graph,
        capture_receipt: &row.capture_receipt,
        capture_record: &row.capture_record,
        pre_action_wire_root_sha256: &row.pre_action_wire_root_sha256,
        observation_source: row.observation_source,
        intervention_id: &row.intervention_id,
        session_lineage_sha256: &row.session_lineage_sha256,
    })
}

fn validate_evidence_record(
    record: &EvidenceLedgerRecord,
) -> Result<(), BindingSupportCaptureErrorV1> {
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
    .map_err(|_| BindingSupportCaptureErrorV1::Serialization)?;
    if record.schema != EVIDENCE_LEDGER_SCHEMA_V1
        || !is_sha256(&record.previous_record_sha256)
        || !is_sha256(&record.record_sha256)
        || record.record_sha256 != expected
    {
        return Err(BindingSupportCaptureErrorV1::InvalidCaptureReceipt);
    }
    Ok(())
}

fn support_freeze_digest(
    freeze: &BindingSupportFreezeV1,
) -> Result<String, BindingSupportCaptureErrorV1> {
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        capture_index: &'a CaptureCommitmentIndex,
        watermark: &'a UntrustedBindingCaptureWatermarkV1,
        watermark_bytes_sha256: &'a str,
        support_rows_root_sha256: &'a str,
        support_graphs_root_sha256: &'a str,
        support_lineages_root_sha256: &'a str,
        support_rows: &'a [BindingSupportCaptureRowV1],
        intervention_rows: &'a BTreeMap<String, usize>,
        support_session_lineages: usize,
        expected_labels_joined: bool,
        h0_adjudicated: bool,
        h1_adjudicated: bool,
        future_opened: bool,
        protocol_mode_compiled: bool,
        execution_authority: bool,
    }
    sha256_json(&DigestFields {
        schema: &freeze.schema,
        capture_index: &freeze.capture_index,
        watermark: &freeze.watermark,
        watermark_bytes_sha256: &freeze.watermark_bytes_sha256,
        support_rows_root_sha256: &freeze.support_rows_root_sha256,
        support_graphs_root_sha256: &freeze.support_graphs_root_sha256,
        support_lineages_root_sha256: &freeze.support_lineages_root_sha256,
        support_rows: &freeze.support_rows,
        intervention_rows: &freeze.intervention_rows,
        support_session_lineages: freeze.support_session_lineages,
        expected_labels_joined: freeze.expected_labels_joined,
        h0_adjudicated: freeze.h0_adjudicated,
        h1_adjudicated: freeze.h1_adjudicated,
        future_opened: freeze.future_opened,
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

fn sha256_json<T: Serialize>(value: &T) -> Result<String, BindingSupportCaptureErrorV1> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_bytes(&bytes))
        .map_err(|_| BindingSupportCaptureErrorV1::Serialization)
}

#[cfg(test)]
#[path = "binding_evidence_capture_owner_tests.rs"]
mod tests;
