use nando_operator_kernel::{canonical_json_sha256, sha256_bytes};
use serde::{Deserialize, Serialize};

use super::{GENERATION_CHECKPOINT_SCHEMA_V3, GenerationCheckpointErrorV3};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GenerationCheckpointWireV3 {
    pub schema: String,
    pub publish_sequence: u64,
    pub generation_id_sha256: String,
    pub generation_bundle_sha256: String,
    #[serde(with = "serde_bytes")]
    pub generation_bundle_bytes: Vec<u8>,
    pub evidence_root_sha256: String,
    #[serde(with = "serde_bytes")]
    pub evidence_ledger_bytes: Vec<u8>,
    pub receipt_set_sha256: String,
    pub receipts: Vec<GenerationCheckpointReceiptWireV3>,
    pub checkpoint_sha256: String,
    pub raw_payloads_persisted: u8,
    pub execution_authority: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GenerationCheckpointReceiptWireV3 {
    pub capture_sequence: u64,
    pub f6_receipt_sha256: String,
    #[serde(with = "serde_bytes")]
    pub f6_receipt_bytes: Vec<u8>,
    pub generation_receipt_sha256: String,
    #[serde(with = "serde_bytes")]
    pub generation_receipt_bytes: Vec<u8>,
}

pub(super) fn receipt_set_digest_v3(
    generation_id_sha256: &str,
    receipts: &[GenerationCheckpointReceiptWireV3],
) -> Result<String, GenerationCheckpointErrorV3> {
    let commitments = receipts
        .iter()
        .map(|receipt| {
            (
                receipt.capture_sequence,
                receipt.f6_receipt_sha256.as_str(),
                sha256_bytes(&receipt.f6_receipt_bytes),
                receipt.generation_receipt_sha256.as_str(),
                sha256_bytes(&receipt.generation_receipt_bytes),
            )
        })
        .collect::<Vec<_>>();
    canonical_json_sha256(&(
        GENERATION_CHECKPOINT_SCHEMA_V3,
        "receipt-set",
        generation_id_sha256,
        commitments,
    ))
    .map_err(|_| GenerationCheckpointErrorV3::Serialization)
}

pub(super) fn checkpoint_digest_v3(
    wire: &GenerationCheckpointWireV3,
) -> Result<String, GenerationCheckpointErrorV3> {
    canonical_json_sha256(&(
        GENERATION_CHECKPOINT_SCHEMA_V3,
        wire.publish_sequence,
        wire.generation_id_sha256.as_str(),
        wire.generation_bundle_sha256.as_str(),
        wire.evidence_root_sha256.as_str(),
        wire.receipt_set_sha256.as_str(),
        0_u8,
        false,
    ))
    .map_err(|_| GenerationCheckpointErrorV3::Serialization)
}
