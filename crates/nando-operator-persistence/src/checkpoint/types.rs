use nando_operator_learning::GenerationEvidenceLedgerV3;
use nando_operator_proof::{
    generation_receipt_v3::GenerationVerifierReceiptV3,
    independent_verifier_v3::IndependentVerifierReceiptV3,
};
use nando_operator_runtime::RestoredOperatorGenerationV3;

pub const GENERATION_CHECKPOINT_SCHEMA_V3: &str = "nando.operator-generation-checkpoint.v3.f7";
pub const GENERATION_CHECKPOINT_MAX_BYTES_V3: usize = 16 * 1024 * 1024;
pub const GENERATION_CHECKPOINT_MAX_RECEIPTS_V3: usize = 4_096;

#[derive(Clone, Copy)]
pub struct GenerationCheckpointReceiptRefV3<'a> {
    pub f6_receipt: &'a IndependentVerifierReceiptV3,
    pub generation_receipt: &'a GenerationVerifierReceiptV3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationCheckpointErrorV3 {
    InvalidPublishSequence,
    InvalidGenerationBundle,
    InvalidEvidenceLedger,
    InvalidVerifierReceipt,
    InvalidGenerationReceipt,
    InvalidReceiptSet,
    GenerationMismatch,
    BudgetExhausted,
    InvalidCheckpoint,
    Serialization,
}

pub struct RestoredGenerationReceiptPairV3 {
    f6_receipt: IndependentVerifierReceiptV3,
    generation_receipt: GenerationVerifierReceiptV3,
}

pub struct RestoredGenerationCheckpointV3 {
    publish_sequence: u64,
    generation: RestoredOperatorGenerationV3,
    ledger: GenerationEvidenceLedgerV3,
    receipts: Box<[RestoredGenerationReceiptPairV3]>,
    evidence_root_sha256: String,
    receipt_set_sha256: String,
    checkpoint_sha256: String,
    canonical_bytes: Box<[u8]>,
}

impl RestoredGenerationReceiptPairV3 {
    #[must_use]
    pub const fn f6_receipt(&self) -> &IndependentVerifierReceiptV3 {
        &self.f6_receipt
    }

    #[must_use]
    pub const fn generation_receipt(&self) -> &GenerationVerifierReceiptV3 {
        &self.generation_receipt
    }
}

impl RestoredGenerationCheckpointV3 {
    #[must_use]
    pub const fn publish_sequence(&self) -> u64 {
        self.publish_sequence
    }

    #[must_use]
    pub const fn generation(&self) -> &RestoredOperatorGenerationV3 {
        &self.generation
    }

    #[must_use]
    pub const fn ledger(&self) -> &GenerationEvidenceLedgerV3 {
        &self.ledger
    }

    #[must_use]
    pub const fn receipts(&self) -> &[RestoredGenerationReceiptPairV3] {
        &self.receipts
    }

    #[must_use]
    pub fn evidence_root_sha256(&self) -> &str {
        &self.evidence_root_sha256
    }

    #[must_use]
    pub fn receipt_set_sha256(&self) -> &str {
        &self.receipt_set_sha256
    }

    #[must_use]
    pub fn checkpoint_sha256(&self) -> &str {
        &self.checkpoint_sha256
    }

    #[must_use]
    pub const fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

pub(super) struct RestoredGenerationCheckpointPartsV3 {
    pub publish_sequence: u64,
    pub generation: RestoredOperatorGenerationV3,
    pub ledger: GenerationEvidenceLedgerV3,
    pub receipts: Vec<RestoredGenerationReceiptPairV3>,
    pub evidence_root_sha256: String,
    pub receipt_set_sha256: String,
    pub checkpoint_sha256: String,
}

impl RestoredGenerationCheckpointV3 {
    pub(super) fn from_parts(
        parts: RestoredGenerationCheckpointPartsV3,
        canonical_bytes: Box<[u8]>,
    ) -> Self {
        Self {
            publish_sequence: parts.publish_sequence,
            generation: parts.generation,
            ledger: parts.ledger,
            receipts: parts.receipts.into_boxed_slice(),
            evidence_root_sha256: parts.evidence_root_sha256,
            receipt_set_sha256: parts.receipt_set_sha256,
            checkpoint_sha256: parts.checkpoint_sha256,
            canonical_bytes,
        }
    }
}

impl RestoredGenerationReceiptPairV3 {
    pub(super) const fn new(
        f6_receipt: IndependentVerifierReceiptV3,
        generation_receipt: GenerationVerifierReceiptV3,
    ) -> Self {
        Self {
            f6_receipt,
            generation_receipt,
        }
    }
}
