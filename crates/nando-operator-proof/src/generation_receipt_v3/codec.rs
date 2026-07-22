use nando_operator_kernel::{OperatorGenerationManifestV3, canonical_json_bytes, sha256_bytes};

use crate::independent_verifier_v3::IndependentVerifierReceiptV3;

use super::{
    GENERATION_VERIFIER_RECEIPT_MAX_BYTES_V3, GenerationVerifierReceiptErrorV3,
    GenerationVerifierReceiptInputV3, GenerationVerifierReceiptV3,
    seal::validate_generation_receipt_v3, seal_generation_verifier_receipt_v3,
};

impl GenerationVerifierReceiptV3 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, GenerationVerifierReceiptErrorV3> {
        let bytes = canonical_json_bytes(self)
            .map_err(|_| GenerationVerifierReceiptErrorV3::Serialization)?;
        if bytes.len() > GENERATION_VERIFIER_RECEIPT_MAX_BYTES_V3 {
            return Err(GenerationVerifierReceiptErrorV3::BudgetExhausted);
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes(
        bytes: &[u8],
        manifest: &OperatorGenerationManifestV3,
        f6_receipt: &IndependentVerifierReceiptV3,
    ) -> Result<Self, GenerationVerifierReceiptErrorV3> {
        if bytes.len() > GENERATION_VERIFIER_RECEIPT_MAX_BYTES_V3 {
            return Err(GenerationVerifierReceiptErrorV3::BudgetExhausted);
        }
        let decoded: Self = serde_json::from_slice(bytes)
            .map_err(|_| GenerationVerifierReceiptErrorV3::InvalidEnvelope)?;
        validate_generation_receipt_v3(&decoded)?;
        if decoded.generation_id_sha256 != manifest.generation_id_sha256() {
            return Err(GenerationVerifierReceiptErrorV3::InvalidGeneration);
        }
        let expected = seal_generation_verifier_receipt_v3(
            manifest,
            GenerationVerifierReceiptInputV3 {
                partition: decoded.partition,
                capture_sequence: decoded.capture_sequence,
                support_watermark_next_sequence: decoded.support_watermark_next_sequence,
                support_freeze_sha256: decoded.support_freeze_sha256.clone(),
                lineage_root_sha256: decoded.lineage_root_sha256.clone(),
                event_root_sha256: decoded.event_root_sha256.clone(),
            },
            f6_receipt,
        )?;
        if decoded != expected
            || decoded.canonical_bytes()? != bytes
            || decoded.f6_receipt_bytes_sha256
                != sha256_bytes(
                    &f6_receipt
                        .canonical_bytes()
                        .map_err(|_| GenerationVerifierReceiptErrorV3::InvalidVerifierReceipt)?,
                )
        {
            return Err(GenerationVerifierReceiptErrorV3::InvalidEnvelope);
        }
        Ok(decoded)
    }
}
