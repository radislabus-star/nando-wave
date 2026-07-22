use nando_operator_kernel::sha256_bytes;
use serde::Deserialize;

use super::ExternalGenerationAdmissionErrorV3;

const F8_RESOURCE_RECEIPT_SCHEMA_V1: &str = "nando.stop-f8-0-resource-truth.v1";

#[derive(Deserialize)]
struct ResourceReceiptWireV1 {
    schema: String,
    authority: bool,
    production_policy: ResourceProductionPolicyWireV1,
    proof: ResourceProofWireV1,
}

#[derive(Deserialize)]
struct ResourceProductionPolicyWireV1 {
    resource_observations: u32,
    observations_within_target: u32,
    max_peak_rss_delta_bytes: u64,
    target_bytes: u64,
    verdict: String,
}

#[derive(Deserialize)]
struct ResourceProofWireV1 {
    raw_payload_bytes_persisted: u64,
    local_accepts: u64,
    execution_authority: bool,
}

pub(super) struct ValidatedResourceReceiptV3 {
    pub receipt_sha256: String,
}

pub(super) fn validate_resource_receipt_v3(
    bytes: &[u8],
) -> Result<ValidatedResourceReceiptV3, ExternalGenerationAdmissionErrorV3> {
    if bytes.is_empty() {
        return Err(ExternalGenerationAdmissionErrorV3::MissingInput);
    }
    let wire: ResourceReceiptWireV1 = serde_json::from_slice(bytes)
        .map_err(|_| ExternalGenerationAdmissionErrorV3::InvalidResourceReceipt)?;
    if wire.schema != F8_RESOURCE_RECEIPT_SCHEMA_V1 {
        return Err(ExternalGenerationAdmissionErrorV3::UnknownSchema);
    }
    if wire.authority
        || wire.production_policy.verdict != "PASS"
        || wire.production_policy.resource_observations == 0
        || wire.production_policy.observations_within_target
            != wire.production_policy.resource_observations
        || wire.production_policy.max_peak_rss_delta_bytes > wire.production_policy.target_bytes
        || wire.proof.raw_payload_bytes_persisted != 0
        || wire.proof.local_accepts != 0
        || wire.proof.execution_authority
    {
        return Err(ExternalGenerationAdmissionErrorV3::InvalidResourceReceipt);
    }
    Ok(ValidatedResourceReceiptV3 {
        receipt_sha256: sha256_bytes(bytes),
    })
}
