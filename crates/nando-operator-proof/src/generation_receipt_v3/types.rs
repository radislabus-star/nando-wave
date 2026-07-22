use serde::{Deserialize, Serialize};

use nando_operator_kernel::GenerationEvidencePartitionV3;

use crate::independent_verifier_v3::IndependentVerifierVerdictV3;

pub const GENERATION_VERIFIER_RECEIPT_SCHEMA_V3: &str =
    "nando.operator-generation-verifier-receipt.v3.f7";
pub const GENERATION_VERIFIER_RECEIPT_MAX_BYTES_V3: usize = 16 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationVerifierReceiptV3 {
    pub(super) schema: String,
    pub(super) generation_id_sha256: String,
    pub(super) partition: GenerationEvidencePartitionV3,
    pub(super) capture_sequence: u64,
    pub(super) support_watermark_next_sequence: u64,
    pub(super) support_freeze_sha256: Option<String>,
    pub(super) lineage_root_sha256: String,
    pub(super) event_root_sha256: String,
    pub(super) f6_receipt_sha256: String,
    pub(super) f6_receipt_bytes_sha256: String,
    pub(super) f6_request_sha256: String,
    pub(super) f6_verdict: IndependentVerifierVerdictV3,
    pub(super) generation_receipt_sha256: String,
    pub(super) raw_payloads_persisted: u8,
    pub(super) execution_authority: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationVerifierReceiptInputV3 {
    pub partition: GenerationEvidencePartitionV3,
    pub capture_sequence: u64,
    pub support_watermark_next_sequence: u64,
    pub support_freeze_sha256: Option<String>,
    pub lineage_root_sha256: String,
    pub event_root_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationVerifierReceiptErrorV3 {
    InvalidGeneration,
    InvalidPartitionBinding,
    InvalidRoot,
    ArtifactSetMismatch,
    InvalidVerifierReceipt,
    InvalidEnvelope,
    BudgetExhausted,
    Serialization,
}

impl GenerationVerifierReceiptV3 {
    #[must_use]
    pub fn generation_id_sha256(&self) -> &str {
        &self.generation_id_sha256
    }

    #[must_use]
    pub const fn partition(&self) -> GenerationEvidencePartitionV3 {
        self.partition
    }

    #[must_use]
    pub const fn capture_sequence(&self) -> u64 {
        self.capture_sequence
    }

    #[must_use]
    pub const fn support_watermark_next_sequence(&self) -> u64 {
        self.support_watermark_next_sequence
    }

    #[must_use]
    pub fn support_freeze_sha256(&self) -> Option<&str> {
        self.support_freeze_sha256.as_deref()
    }

    #[must_use]
    pub fn lineage_root_sha256(&self) -> &str {
        &self.lineage_root_sha256
    }

    #[must_use]
    pub fn event_root_sha256(&self) -> &str {
        &self.event_root_sha256
    }

    #[must_use]
    pub fn f6_receipt_sha256(&self) -> &str {
        &self.f6_receipt_sha256
    }

    #[must_use]
    pub fn f6_request_sha256(&self) -> &str {
        &self.f6_request_sha256
    }

    #[must_use]
    pub const fn f6_verdict(&self) -> IndependentVerifierVerdictV3 {
        self.f6_verdict
    }

    #[must_use]
    pub fn generation_receipt_sha256(&self) -> &str {
        &self.generation_receipt_sha256
    }

    #[must_use]
    pub const fn is_verified_pass(&self) -> bool {
        matches!(self.f6_verdict, IndependentVerifierVerdictV3::Verified)
    }

    #[must_use]
    pub const fn raw_payloads_persisted(&self) -> u8 {
        self.raw_payloads_persisted
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        self.execution_authority
    }
}
