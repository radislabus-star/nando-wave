use crate::binding_evidence_preregistration::UntrustedBindingLabelManifestV1;

use super::canonical::{
    external_trust_receipt_digest, load_frozen_evidence, sha256_bytes, sha256_json,
    validate_b1a_report, validate_preregistration,
};
use super::wire::{
    BINDING_EXTERNAL_LABEL_TRUST_SCHEMA_V1, BindingAdjudicationErrorV1,
    BindingExternalLabelTrustReceiptV1, BindingPhysicalLabelReceiptSetV1,
};

#[allow(clippy::too_many_arguments)]
pub fn seal_binding_external_label_trust_v1(
    preregistration_bytes: &[u8],
    b1a_report_bytes: &[u8],
    support_freeze_bytes: &[u8],
    support_watermark_bytes: &[u8],
    future_freeze_bytes: &[u8],
    future_external_receipt_bytes: &[u8],
    physical_receipts_bytes: &[u8],
    label_manifest_bytes: &[u8],
) -> Result<BindingExternalLabelTrustReceiptV1, BindingAdjudicationErrorV1> {
    validate_preregistration(preregistration_bytes)?;
    validate_b1a_report(b1a_report_bytes)?;
    let physical = BindingPhysicalLabelReceiptSetV1::from_canonical_bytes(physical_receipts_bytes)?;
    let manifest: UntrustedBindingLabelManifestV1 = serde_json::from_slice(label_manifest_bytes)
        .map_err(|_| BindingAdjudicationErrorV1::InvalidLabelManifest)?;
    if manifest
        .canonical_bytes()
        .map_err(|_| BindingAdjudicationErrorV1::InvalidLabelManifest)?
        != label_manifest_bytes
        || manifest.external_manifest_root_sha256 != physical.receipt_sha256
        || manifest.freeze_watermark_root_sha256 != sha256_bytes(support_watermark_bytes)
        || physical.support_freeze_file_sha256 != sha256_bytes(support_freeze_bytes)
        || physical.future_freeze_file_sha256 != sha256_bytes(future_freeze_bytes)
        || physical.future_external_receipt_file_sha256
            != sha256_bytes(future_external_receipt_bytes)
    {
        return Err(BindingAdjudicationErrorV1::InvalidTrustReceipt);
    }
    load_frozen_evidence(
        support_freeze_bytes,
        support_watermark_bytes,
        future_freeze_bytes,
        future_external_receipt_bytes,
    )?;

    let owner_challenge_root_sha256 = sha256_json(&(
        "nando.binding-label-owner-challenge.v1",
        sha256_bytes(preregistration_bytes),
        sha256_bytes(b1a_report_bytes),
        sha256_bytes(support_freeze_bytes),
        sha256_bytes(future_external_receipt_bytes),
    ))?;
    let mut receipt = BindingExternalLabelTrustReceiptV1 {
        schema: BINDING_EXTERNAL_LABEL_TRUST_SCHEMA_V1.to_owned(),
        receipt_sha256: String::new(),
        stop_id: "STOP-B1B-LABEL-TRUST".to_owned(),
        owner_challenge_root_sha256,
        preregistration_file_sha256: sha256_bytes(preregistration_bytes),
        b1a_report_file_sha256: sha256_bytes(b1a_report_bytes),
        support_freeze_file_sha256: sha256_bytes(support_freeze_bytes),
        support_watermark_file_sha256: sha256_bytes(support_watermark_bytes),
        future_freeze_file_sha256: sha256_bytes(future_freeze_bytes),
        future_external_receipt_file_sha256: sha256_bytes(future_external_receipt_bytes),
        physical_receipts_file_sha256: sha256_bytes(physical_receipts_bytes),
        physical_receipts_root_sha256: physical.receipt_sha256.clone(),
        label_manifest_file_sha256: sha256_bytes(label_manifest_bytes),
        external_manifest_root_sha256: manifest.external_manifest_root_sha256,
        expected_labels_joined: true,
        protocol_mode_compiled: false,
        execution_authority: false,
    };
    receipt.receipt_sha256 = external_trust_receipt_digest(&receipt)?;
    Ok(receipt)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_external_trust_inputs(
    trust: &BindingExternalLabelTrustReceiptV1,
    preregistration_bytes: &[u8],
    b1a_report_bytes: &[u8],
    support_freeze_bytes: &[u8],
    support_watermark_bytes: &[u8],
    future_freeze_bytes: &[u8],
    future_external_receipt_bytes: &[u8],
    physical_receipts_bytes: &[u8],
    label_manifest_bytes: &[u8],
) -> Result<(), BindingAdjudicationErrorV1> {
    let owner_challenge_root_sha256 = sha256_json(&(
        "nando.binding-label-owner-challenge.v1",
        sha256_bytes(preregistration_bytes),
        sha256_bytes(b1a_report_bytes),
        sha256_bytes(support_freeze_bytes),
        sha256_bytes(future_external_receipt_bytes),
    ))?;
    if trust.schema != BINDING_EXTERNAL_LABEL_TRUST_SCHEMA_V1
        || trust.stop_id != "STOP-B1B-LABEL-TRUST"
        || trust.owner_challenge_root_sha256 != owner_challenge_root_sha256
        || trust.preregistration_file_sha256 != sha256_bytes(preregistration_bytes)
        || trust.b1a_report_file_sha256 != sha256_bytes(b1a_report_bytes)
        || trust.support_freeze_file_sha256 != sha256_bytes(support_freeze_bytes)
        || trust.support_watermark_file_sha256 != sha256_bytes(support_watermark_bytes)
        || trust.future_freeze_file_sha256 != sha256_bytes(future_freeze_bytes)
        || trust.future_external_receipt_file_sha256 != sha256_bytes(future_external_receipt_bytes)
        || trust.physical_receipts_file_sha256 != sha256_bytes(physical_receipts_bytes)
        || trust.label_manifest_file_sha256 != sha256_bytes(label_manifest_bytes)
        || trust.physical_receipts_root_sha256 != trust.external_manifest_root_sha256
        || !trust.expected_labels_joined
        || trust.protocol_mode_compiled
        || trust.execution_authority
    {
        return Err(BindingAdjudicationErrorV1::InvalidTrustReceipt);
    }
    Ok(())
}
