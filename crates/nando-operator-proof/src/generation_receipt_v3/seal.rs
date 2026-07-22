use nando_operator_kernel::{
    GenerationEvidencePartitionV3, OperatorGenerationManifestV3, canonical_json_sha256,
    sha256_bytes, valid_nonzero_sha256,
};
use serde::Serialize;

use crate::independent_verifier_v3::{IndependentVerifierReceiptV3, IndependentVerifierVerdictV3};

use super::{
    GENERATION_VERIFIER_RECEIPT_SCHEMA_V3, GenerationVerifierReceiptErrorV3,
    GenerationVerifierReceiptInputV3, GenerationVerifierReceiptV3,
};

#[derive(Serialize)]
struct GenerationVerifierReceiptDigestV3<'a> {
    schema: &'a str,
    generation_id_sha256: &'a str,
    partition: GenerationEvidencePartitionV3,
    capture_sequence: u64,
    support_watermark_next_sequence: u64,
    support_freeze_sha256: &'a Option<String>,
    lineage_root_sha256: &'a str,
    event_root_sha256: &'a str,
    f6_receipt_sha256: &'a str,
    f6_receipt_bytes_sha256: &'a str,
    f6_request_sha256: &'a str,
    f6_verdict: IndependentVerifierVerdictV3,
    raw_payloads_persisted: u8,
    execution_authority: bool,
}

pub fn seal_generation_verifier_receipt_v3(
    manifest: &OperatorGenerationManifestV3,
    input: GenerationVerifierReceiptInputV3,
    f6_receipt: &IndependentVerifierReceiptV3,
) -> Result<GenerationVerifierReceiptV3, GenerationVerifierReceiptErrorV3> {
    validate_f6_receipt(manifest, f6_receipt)?;
    validate_partition_binding(
        input.partition,
        input.capture_sequence,
        input.support_watermark_next_sequence,
        input.support_freeze_sha256.as_deref(),
    )?;
    if !valid_nonzero_sha256(&input.lineage_root_sha256)
        || !valid_nonzero_sha256(&input.event_root_sha256)
    {
        return Err(GenerationVerifierReceiptErrorV3::InvalidRoot);
    }
    let f6_receipt_bytes = f6_receipt
        .canonical_bytes()
        .map_err(|_| GenerationVerifierReceiptErrorV3::InvalidVerifierReceipt)?;
    let mut receipt = GenerationVerifierReceiptV3 {
        schema: GENERATION_VERIFIER_RECEIPT_SCHEMA_V3.to_owned(),
        generation_id_sha256: manifest.generation_id_sha256().to_owned(),
        partition: input.partition,
        capture_sequence: input.capture_sequence,
        support_watermark_next_sequence: input.support_watermark_next_sequence,
        support_freeze_sha256: input.support_freeze_sha256,
        lineage_root_sha256: input.lineage_root_sha256,
        event_root_sha256: input.event_root_sha256,
        f6_receipt_sha256: f6_receipt.receipt_sha256().to_owned(),
        f6_receipt_bytes_sha256: sha256_bytes(&f6_receipt_bytes),
        f6_request_sha256: f6_receipt.request_sha256().to_owned(),
        f6_verdict: f6_receipt.verdict(),
        generation_receipt_sha256: String::new(),
        raw_payloads_persisted: 0,
        execution_authority: false,
    };
    receipt.generation_receipt_sha256 = generation_receipt_digest_v3(&receipt)?;
    validate_generation_receipt_v3(&receipt)?;
    Ok(receipt)
}

fn validate_f6_receipt(
    manifest: &OperatorGenerationManifestV3,
    receipt: &IndependentVerifierReceiptV3,
) -> Result<(), GenerationVerifierReceiptErrorV3> {
    let bytes = receipt
        .canonical_bytes()
        .map_err(|_| GenerationVerifierReceiptErrorV3::InvalidVerifierReceipt)?;
    let restored = IndependentVerifierReceiptV3::from_canonical_bytes(&bytes)
        .map_err(|_| GenerationVerifierReceiptErrorV3::InvalidVerifierReceipt)?;
    if &restored != receipt
        || receipt.raw_payloads_persisted() != 0
        || receipt.execution_authority()
    {
        return Err(GenerationVerifierReceiptErrorV3::InvalidVerifierReceipt);
    }
    if receipt.artifact_set_sha256() != manifest.components().artifact_set_sha256 {
        return Err(GenerationVerifierReceiptErrorV3::ArtifactSetMismatch);
    }
    Ok(())
}

fn validate_partition_binding(
    partition: GenerationEvidencePartitionV3,
    capture_sequence: u64,
    support_watermark_next_sequence: u64,
    support_freeze_sha256: Option<&str>,
) -> Result<(), GenerationVerifierReceiptErrorV3> {
    if capture_sequence == 0 || support_watermark_next_sequence == 0 {
        return Err(GenerationVerifierReceiptErrorV3::InvalidPartitionBinding);
    }
    let valid = match partition {
        GenerationEvidencePartitionV3::Support => {
            capture_sequence < support_watermark_next_sequence && support_freeze_sha256.is_none()
        }
        GenerationEvidencePartitionV3::Future => {
            capture_sequence >= support_watermark_next_sequence
                && support_freeze_sha256.is_some_and(valid_nonzero_sha256)
        }
    };
    valid
        .then_some(())
        .ok_or(GenerationVerifierReceiptErrorV3::InvalidPartitionBinding)
}

pub(super) fn validate_generation_receipt_v3(
    receipt: &GenerationVerifierReceiptV3,
) -> Result<(), GenerationVerifierReceiptErrorV3> {
    if receipt.schema != GENERATION_VERIFIER_RECEIPT_SCHEMA_V3
        || !valid_nonzero_sha256(&receipt.generation_id_sha256)
        || !valid_nonzero_sha256(&receipt.lineage_root_sha256)
        || !valid_nonzero_sha256(&receipt.event_root_sha256)
        || !valid_nonzero_sha256(&receipt.f6_receipt_sha256)
        || !valid_nonzero_sha256(&receipt.f6_receipt_bytes_sha256)
        || !valid_nonzero_sha256(&receipt.f6_request_sha256)
        || !valid_nonzero_sha256(&receipt.generation_receipt_sha256)
        || receipt.raw_payloads_persisted != 0
        || receipt.execution_authority
    {
        return Err(GenerationVerifierReceiptErrorV3::InvalidEnvelope);
    }
    validate_partition_binding(
        receipt.partition,
        receipt.capture_sequence,
        receipt.support_watermark_next_sequence,
        receipt.support_freeze_sha256.as_deref(),
    )?;
    if generation_receipt_digest_v3(receipt)? != receipt.generation_receipt_sha256 {
        return Err(GenerationVerifierReceiptErrorV3::InvalidEnvelope);
    }
    Ok(())
}

fn generation_receipt_digest_v3(
    receipt: &GenerationVerifierReceiptV3,
) -> Result<String, GenerationVerifierReceiptErrorV3> {
    canonical_json_sha256(&GenerationVerifierReceiptDigestV3 {
        schema: GENERATION_VERIFIER_RECEIPT_SCHEMA_V3,
        generation_id_sha256: &receipt.generation_id_sha256,
        partition: receipt.partition,
        capture_sequence: receipt.capture_sequence,
        support_watermark_next_sequence: receipt.support_watermark_next_sequence,
        support_freeze_sha256: &receipt.support_freeze_sha256,
        lineage_root_sha256: &receipt.lineage_root_sha256,
        event_root_sha256: &receipt.event_root_sha256,
        f6_receipt_sha256: &receipt.f6_receipt_sha256,
        f6_receipt_bytes_sha256: &receipt.f6_receipt_bytes_sha256,
        f6_request_sha256: &receipt.f6_request_sha256,
        f6_verdict: receipt.f6_verdict,
        raw_payloads_persisted: 0,
        execution_authority: false,
    })
    .map_err(|_| GenerationVerifierReceiptErrorV3::Serialization)
}
