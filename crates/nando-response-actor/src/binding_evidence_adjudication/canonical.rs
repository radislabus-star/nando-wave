use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::binding_evidence::{
    BindingBaselineOutcomeV1, BindingEvaluationLabelV1, BindingVersionSpaceReportV1,
};
use crate::binding_evidence_capture_owner::BindingSupportFreezeV1;
use crate::binding_evidence_future_capture::BindingFutureCaptureFreezeV1;
use crate::binding_evidence_preregistration::{
    BindingEvidencePartitionV1, BindingEvidencePreregistrationV1,
    binding_evidence_preregistration_v1,
};
use crate::canonical_json_sha256;

use super::wire::{
    BINDING_PHYSICAL_LABEL_RECEIPT_SCHEMA_V1, BINDING_PHYSICAL_LABEL_SET_SCHEMA_V1,
    BINDING_TRIAL_PARITY_DOMAIN_V1, BINDING_TRIAL_VERIFIER_DOMAIN_V1, BindingAdjudicationErrorV1,
    BindingExternalLabelTrustReceiptV1, BindingObservedRelationV1, BindingPhysicalActorOutcomeV1,
    BindingPhysicalCandidateTrialV1, BindingPhysicalLabelReceiptSetV1,
    BindingPhysicalLabelReceiptV1, CONTROLLED_ROWS_PER_PARTITION_V1,
};

#[derive(Deserialize)]
struct FutureExternalReceiptWireV1 {
    schema: String,
    future_freeze_file_sha256: String,
    trusted_future_receipt_sha256: String,
    expected_labels_joined: bool,
    execution_authority: bool,
}

pub(super) fn load_frozen_evidence(
    support_freeze_bytes: &[u8],
    support_watermark_bytes: &[u8],
    future_freeze_bytes: &[u8],
    future_external_receipt_bytes: &[u8],
) -> Result<(BindingSupportFreezeV1, BindingFutureCaptureFreezeV1), BindingAdjudicationErrorV1> {
    let support = BindingSupportFreezeV1::from_canonical_bytes(support_freeze_bytes)
        .map_err(|_| BindingAdjudicationErrorV1::InvalidFrozenSupport)?;
    if support
        .watermark_canonical_bytes()
        .map_err(|_| BindingAdjudicationErrorV1::InvalidFrozenSupport)?
        != support_watermark_bytes
    {
        return Err(BindingAdjudicationErrorV1::InvalidFrozenSupport);
    }
    let external: FutureExternalReceiptWireV1 =
        serde_json::from_slice(future_external_receipt_bytes)
            .map_err(|_| BindingAdjudicationErrorV1::InvalidExternalFutureReceipt)?;
    if external.schema != "nando.binding-future-external-receipt.v1"
        || external.future_freeze_file_sha256 != sha256_bytes(future_freeze_bytes)
        || external.expected_labels_joined
        || external.execution_authority
        || !is_sha256(&external.trusted_future_receipt_sha256)
    {
        return Err(BindingAdjudicationErrorV1::InvalidExternalFutureReceipt);
    }
    let future = BindingFutureCaptureFreezeV1::from_canonical_bytes(
        future_freeze_bytes,
        &external.trusted_future_receipt_sha256,
        support_freeze_bytes,
        support_watermark_bytes,
    )
    .map_err(|_| BindingAdjudicationErrorV1::InvalidFrozenFuture)?;
    Ok((support, future))
}

pub(super) fn validate_physical_receipt_set(
    set: &BindingPhysicalLabelReceiptSetV1,
) -> Result<(), BindingAdjudicationErrorV1> {
    if set.schema != BINDING_PHYSICAL_LABEL_SET_SCHEMA_V1
        || set.execution_authority
        || set.receipts.len() != CONTROLLED_ROWS_PER_PARTITION_V1 * 2
        || !is_sha256(&set.support_freeze_file_sha256)
        || !is_sha256(&set.future_freeze_file_sha256)
        || !is_sha256(&set.future_external_receipt_file_sha256)
        || !is_sha256(&set.capture_index_sha256)
        || set
            .receipts
            .windows(2)
            .any(|pair| pair[0].row_id_sha256 >= pair[1].row_id_sha256)
        || physical_receipt_set_digest(set)? != set.receipt_sha256
    {
        return Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt);
    }
    let mut evidence_refs = BTreeSet::new();
    for receipt in &set.receipts {
        validate_physical_label_receipt(receipt)?;
        if !evidence_refs.insert(receipt.evidence_ref_sha256.as_str()) {
            return Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt);
        }
    }
    Ok(())
}

pub(super) fn validate_physical_label_receipt(
    receipt: &BindingPhysicalLabelReceiptV1,
) -> Result<(), BindingAdjudicationErrorV1> {
    let digests = [
        receipt.receipt_sha256.as_str(),
        receipt.row_id_sha256.as_str(),
        receipt.evidence_ref_sha256.as_str(),
        receipt.frozen_graph_root_sha256.as_str(),
        receipt.capture_receipt_root_sha256.as_str(),
        receipt.capture_record_sha256.as_str(),
        receipt.pre_action_wire_root_sha256.as_str(),
        receipt.session_lineage_sha256.as_str(),
        receipt.observed_relation.relation_root_sha256.as_str(),
        receipt.parity_receipt_root_sha256.as_str(),
        receipt.verifier_root_sha256.as_str(),
    ];
    if receipt.schema != BINDING_PHYSICAL_LABEL_RECEIPT_SCHEMA_V1
        || digests.into_iter().any(|digest| !is_sha256(digest))
        || !matches!(
            receipt.intervention_id.as_str(),
            "I1" | "I2" | "I3" | "I4" | "I5" | "I6"
        )
        || receipt.trials.is_empty()
        || receipt
            .trials
            .iter()
            .any(|trial| !is_sha256(&trial.action_equivalence_sha256) || !trial.verifier_agrees)
        || receipt.parity_receipt_root_sha256
            != sha256_json(&(
                BINDING_TRIAL_PARITY_DOMAIN_V1,
                receipt.row_id_sha256.as_str(),
                &receipt.trials,
            ))?
        || receipt.verifier_root_sha256
            != sha256_json(&(
                BINDING_TRIAL_VERIFIER_DOMAIN_V1,
                receipt.row_id_sha256.as_str(),
                receipt
                    .trials
                    .iter()
                    .map(|trial| {
                        (
                            trial.action_equivalence_sha256.as_str(),
                            trial.actor_outcome,
                            trial.applied_parent_ordinal,
                            trial.verifier_agrees,
                        )
                    })
                    .collect::<Vec<_>>(),
            ))?
        || physical_label_receipt_digest(receipt)? != receipt.receipt_sha256
        || observed_relation_digest(&receipt.observed_relation)?
            != receipt.observed_relation.relation_root_sha256
    {
        return Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt);
    }
    let applied = receipt
        .trials
        .iter()
        .filter(|trial| trial.actor_outcome == BindingPhysicalActorOutcomeV1::Applied)
        .collect::<Vec<_>>();
    match (receipt.label, applied.as_slice()) {
        (BindingEvaluationLabelV1::Positive, [trial])
            if receipt.expected_action_equivalence_sha256.as_deref()
                == Some(trial.action_equivalence_sha256.as_str()) => {}
        (BindingEvaluationLabelV1::ApplicabilityNegative, [])
            if receipt.expected_action_equivalence_sha256.is_none() => {}
        _ => return Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt),
    }
    Ok(())
}

pub(super) fn validate_preregistration(
    bytes: &[u8],
) -> Result<BindingEvidencePreregistrationV1, BindingAdjudicationErrorV1> {
    let report: BindingEvidencePreregistrationV1 = serde_json::from_slice(bytes)
        .map_err(|_| BindingAdjudicationErrorV1::InvalidPreregistration)?;
    if report != binding_evidence_preregistration_v1()
        || report.stop_id != "STOP-B1B0R"
        || report.acquisition_run
        || report.protocol_mode_compiled
        || report.f4_started
        || report.execution_authority
    {
        return Err(BindingAdjudicationErrorV1::InvalidPreregistration);
    }
    Ok(report)
}

pub(super) fn validate_b1a_report(
    bytes: &[u8],
) -> Result<BindingVersionSpaceReportV1, BindingAdjudicationErrorV1> {
    let report: BindingVersionSpaceReportV1 =
        serde_json::from_slice(bytes).map_err(|_| BindingAdjudicationErrorV1::InvalidB1aReport)?;
    if report.ties_total == 0
        || report.tie_budget_exhausted
        || report.ties_total != report.ties.len()
        || report.distinguishing_probes.len() != report.ties.len()
        || report.protocol_mode_compiled
        || report.execution_authority
        || report.distinguishing_probes.iter().any(|probe| {
            probe.required_distinction != "expected_action_class_vs_competing_action_classes"
        })
    {
        return Err(BindingAdjudicationErrorV1::InvalidB1aReport);
    }
    Ok(report)
}

pub(super) fn checked_u16(value: usize) -> Result<u16, BindingAdjudicationErrorV1> {
    u16::try_from(value).map_err(|_| BindingAdjudicationErrorV1::InvalidDenominator)
}

pub(super) fn action_digest(value: &str) -> Result<String, BindingAdjudicationErrorV1> {
    canonical_json_sha256(&value).map_err(|_| BindingAdjudicationErrorV1::Serialization)
}

pub(super) fn observed_relation_digest(
    relation: &BindingObservedRelationV1,
) -> Result<String, BindingAdjudicationErrorV1> {
    sha256_json(&(
        relation.schema.as_str(),
        &relation.parents,
        &relation.requested_parent_instance_sha256,
        &relation.requested_capability_action_sha256,
        &relation.candidates,
    ))
}

pub(super) fn physical_label_receipt_digest(
    receipt: &BindingPhysicalLabelReceiptV1,
) -> Result<String, BindingAdjudicationErrorV1> {
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        row_id_sha256: &'a str,
        evidence_ref_sha256: &'a str,
        frozen_graph_root_sha256: &'a str,
        capture_receipt_root_sha256: &'a str,
        capture_sequence: u64,
        capture_record_sha256: &'a str,
        pre_action_wire_root_sha256: &'a str,
        session_lineage_sha256: &'a str,
        partition: BindingEvidencePartitionV1,
        intervention_id: &'a str,
        observed_relation: &'a BindingObservedRelationV1,
        trials: &'a [BindingPhysicalCandidateTrialV1],
        parity_receipt_root_sha256: &'a str,
        verifier_root_sha256: &'a str,
        label: BindingEvaluationLabelV1,
        expected_action_equivalence_sha256: &'a Option<String>,
        baseline_outcome: BindingBaselineOutcomeV1,
    }
    sha256_json(&DigestFields {
        schema: &receipt.schema,
        row_id_sha256: &receipt.row_id_sha256,
        evidence_ref_sha256: &receipt.evidence_ref_sha256,
        frozen_graph_root_sha256: &receipt.frozen_graph_root_sha256,
        capture_receipt_root_sha256: &receipt.capture_receipt_root_sha256,
        capture_sequence: receipt.capture_sequence,
        capture_record_sha256: &receipt.capture_record_sha256,
        pre_action_wire_root_sha256: &receipt.pre_action_wire_root_sha256,
        session_lineage_sha256: &receipt.session_lineage_sha256,
        partition: receipt.partition,
        intervention_id: &receipt.intervention_id,
        observed_relation: &receipt.observed_relation,
        trials: &receipt.trials,
        parity_receipt_root_sha256: &receipt.parity_receipt_root_sha256,
        verifier_root_sha256: &receipt.verifier_root_sha256,
        label: receipt.label,
        expected_action_equivalence_sha256: &receipt.expected_action_equivalence_sha256,
        baseline_outcome: receipt.baseline_outcome,
    })
}

pub(super) fn physical_receipt_set_digest(
    set: &BindingPhysicalLabelReceiptSetV1,
) -> Result<String, BindingAdjudicationErrorV1> {
    sha256_json(&(
        set.schema.as_str(),
        set.support_freeze_file_sha256.as_str(),
        set.future_freeze_file_sha256.as_str(),
        set.future_external_receipt_file_sha256.as_str(),
        set.capture_index_sha256.as_str(),
        &set.receipts,
        set.execution_authority,
    ))
}

pub(super) fn external_trust_receipt_digest(
    receipt: &BindingExternalLabelTrustReceiptV1,
) -> Result<String, BindingAdjudicationErrorV1> {
    sha256_json(&(
        receipt.schema.as_str(),
        receipt.stop_id.as_str(),
        receipt.owner_challenge_root_sha256.as_str(),
        receipt.preregistration_file_sha256.as_str(),
        receipt.b1a_report_file_sha256.as_str(),
        receipt.support_freeze_file_sha256.as_str(),
        receipt.support_watermark_file_sha256.as_str(),
        receipt.future_freeze_file_sha256.as_str(),
        receipt.future_external_receipt_file_sha256.as_str(),
        receipt.physical_receipts_file_sha256.as_str(),
        receipt.physical_receipts_root_sha256.as_str(),
        receipt.label_manifest_file_sha256.as_str(),
        receipt.external_manifest_root_sha256.as_str(),
        receipt.expected_labels_joined,
        receipt.protocol_mode_compiled,
        receipt.execution_authority,
    ))
}

pub(super) fn pretty_json_bytes<T: Serialize>(
    value: &T,
) -> Result<Vec<u8>, BindingAdjudicationErrorV1> {
    let mut bytes =
        serde_json::to_vec_pretty(value).map_err(|_| BindingAdjudicationErrorV1::Serialization)?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(super) fn sha256_json<T: Serialize>(value: &T) -> Result<String, BindingAdjudicationErrorV1> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_bytes(&bytes))
        .map_err(|_| BindingAdjudicationErrorV1::Serialization)
}

pub(super) fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
