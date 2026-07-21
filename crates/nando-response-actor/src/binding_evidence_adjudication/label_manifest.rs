use std::collections::BTreeMap;

use crate::EvidenceLedgerRecord;
use crate::binding_evidence::FrozenCandidateRelationGraphV1;
use crate::binding_evidence_preregistration::{
    BINDING_LABEL_ENVELOPE_SCHEMA_V1, BindingCaptureReceiptEntryV1, BindingEvidencePartitionV1,
    BindingLabelObservationSourceV1, UntrustedBindingLabelEnvelopeV1,
    UntrustedBindingLabelManifestV1,
};
use crate::capture_provenance::CaptureEvidenceReceipt;

use super::canonical::{load_frozen_evidence, sha256_bytes, validate_physical_receipt_set};
use super::wire::{
    BindingAdjudicationErrorV1, BindingPhysicalLabelReceiptSetV1, BindingPhysicalLabelReceiptV1,
};

pub fn build_binding_label_manifest_v1(
    support_freeze_bytes: &[u8],
    support_watermark_bytes: &[u8],
    future_freeze_bytes: &[u8],
    future_external_receipt_bytes: &[u8],
    physical: &BindingPhysicalLabelReceiptSetV1,
) -> Result<UntrustedBindingLabelManifestV1, BindingAdjudicationErrorV1> {
    validate_physical_receipt_set(physical)?;
    let (support, future) = load_frozen_evidence(
        support_freeze_bytes,
        support_watermark_bytes,
        future_freeze_bytes,
        future_external_receipt_bytes,
    )?;
    if physical.support_freeze_file_sha256 != sha256_bytes(support_freeze_bytes)
        || physical.future_freeze_file_sha256 != sha256_bytes(future_freeze_bytes)
        || physical.future_external_receipt_file_sha256
            != sha256_bytes(future_external_receipt_bytes)
        || physical.capture_index_sha256 != future.capture_index().index_sha256
    {
        return Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt);
    }

    let receipt_by_row = physical
        .receipts
        .iter()
        .map(|receipt| (receipt.row_id_sha256.as_str(), receipt))
        .collect::<BTreeMap<_, _>>();
    let mut envelopes = Vec::with_capacity(physical.receipts.len());
    let mut capture_receipts = Vec::with_capacity(physical.receipts.len());
    for row in support.support_label_rows() {
        append_manifest_row(
            &mut envelopes,
            &mut capture_receipts,
            &receipt_by_row,
            row.frozen_graph(),
            row.capture_receipt(),
            row.capture_record(),
            row.pre_action_wire_root_sha256(),
            row.session_lineage_sha256(),
            BindingEvidencePartitionV1::Support,
            row.intervention_id(),
            &physical.receipt_sha256,
        )?;
    }
    for row in future.future_label_rows() {
        let intervention_id = future
            .protocol()
            .source
            .slots
            .iter()
            .find(|slot| slot.slot_id == row.slot_id())
            .ok_or(BindingAdjudicationErrorV1::InvalidIntervention)?
            .intervention_id
            .as_str();
        append_manifest_row(
            &mut envelopes,
            &mut capture_receipts,
            &receipt_by_row,
            row.frozen_graph(),
            row.capture_receipt(),
            row.capture_record(),
            row.pre_action_wire_root_sha256(),
            row.session_lineage_sha256(),
            BindingEvidencePartitionV1::Future,
            intervention_id,
            &physical.receipt_sha256,
        )?;
    }
    if envelopes.len() != physical.receipts.len() {
        return Err(BindingAdjudicationErrorV1::InvalidDenominator);
    }

    UntrustedBindingLabelManifestV1::new(
        physical.receipt_sha256.clone(),
        sha256_bytes(support_watermark_bytes),
        future.capture_index().clone(),
        capture_receipts,
        envelopes,
    )
    .map_err(|_| BindingAdjudicationErrorV1::InvalidLabelManifest)
}

#[allow(clippy::too_many_arguments)]
fn append_manifest_row(
    envelopes: &mut Vec<UntrustedBindingLabelEnvelopeV1>,
    capture_receipts: &mut Vec<BindingCaptureReceiptEntryV1>,
    receipt_by_row: &BTreeMap<&str, &BindingPhysicalLabelReceiptV1>,
    graph: &FrozenCandidateRelationGraphV1,
    capture_receipt: &CaptureEvidenceReceipt,
    capture_record: &EvidenceLedgerRecord,
    pre_action_wire_root_sha256: &str,
    session_lineage_sha256: &str,
    partition: BindingEvidencePartitionV1,
    intervention_id: &str,
    external_manifest_root_sha256: &str,
) -> Result<(), BindingAdjudicationErrorV1> {
    let physical = receipt_by_row
        .get(graph.graph.row_id_sha256.as_str())
        .ok_or(BindingAdjudicationErrorV1::InvalidPhysicalReceipt)?;
    if physical.frozen_graph_root_sha256 != graph.graph_root_sha256
        || physical.capture_record_sha256 != capture_record.record_sha256
        || physical.capture_receipt_root_sha256 != capture_receipt.records_root_sha256
        || physical.pre_action_wire_root_sha256 != pre_action_wire_root_sha256
        || physical.session_lineage_sha256 != session_lineage_sha256
        || physical.partition != partition
        || physical.intervention_id != intervention_id
    {
        return Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt);
    }
    let mut envelope = UntrustedBindingLabelEnvelopeV1 {
        schema: BINDING_LABEL_ENVELOPE_SCHEMA_V1.to_owned(),
        envelope_sha256: String::new(),
        row_id_sha256: graph.graph.row_id_sha256.clone(),
        evidence_ref_sha256: graph.graph.evidence_ref_sha256.clone(),
        frozen_graph_root_sha256: graph.graph_root_sha256.clone(),
        capture_receipt_root_sha256: capture_receipt.records_root_sha256.clone(),
        capture_sequence: capture_record.sequence,
        capture_record_sha256: capture_record.record_sha256.clone(),
        parity_receipt_root_sha256: physical.parity_receipt_root_sha256.clone(),
        verifier_root_sha256: physical.verifier_root_sha256.clone(),
        external_manifest_root_sha256: external_manifest_root_sha256.to_owned(),
        pre_action_wire_root_sha256: pre_action_wire_root_sha256.to_owned(),
        observed_relation_root_sha256: Some(
            physical.observed_relation.relation_root_sha256.clone(),
        ),
        observation_source: BindingLabelObservationSourceV1::PreActionWire,
        intervention_id: intervention_id.to_owned(),
        session_lineage_sha256: session_lineage_sha256.to_owned(),
        partition,
        captured_post_freeze: partition == BindingEvidencePartitionV1::Future,
        label: physical.label,
        expected_action_equivalence_sha256: physical.expected_action_equivalence_sha256.clone(),
        baseline_outcome: physical.baseline_outcome,
    };
    envelope
        .refresh_integrity_checksum()
        .map_err(|_| BindingAdjudicationErrorV1::InvalidLabelManifest)?;
    capture_receipts.push(BindingCaptureReceiptEntryV1 {
        evidence_ref_sha256: graph.graph.evidence_ref_sha256.clone(),
        receipt: capture_receipt.clone(),
    });
    envelopes.push(envelope);
    Ok(())
}
