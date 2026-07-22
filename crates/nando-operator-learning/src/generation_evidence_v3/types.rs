use serde::{Deserialize, Serialize};

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};

pub use nando_operator_kernel::GenerationEvidencePartitionV3;

pub const GENERATION_EVIDENCE_LEDGER_SCHEMA_V3: &str =
    "nando.operator-generation-evidence-ledger.v3.f7";
pub const GENERATION_EVIDENCE_MAX_ROWS_PER_PARTITION_V3: usize = 2_048;
pub const GENERATION_EVIDENCE_MAX_BYTES_V3: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationCensoredReasonV3 {
    Timeout,
    EnvironmentUnavailable,
    MissingPayload,
    BudgetExhausted,
    VerifierUnavailable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationLearningOutcomeV3 {
    VerifiedPass,
    ApplicabilityNegative,
    HardContradiction,
    Censored(GenerationCensoredReasonV3),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationSemanticUpdateV3 {
    PositiveReinforcement,
    ApplicabilityCounterWave,
    StructuralRevision,
}

impl GenerationLearningOutcomeV3 {
    #[must_use]
    pub const fn semantic_update(self) -> Option<GenerationSemanticUpdateV3> {
        match self {
            Self::VerifiedPass => Some(GenerationSemanticUpdateV3::PositiveReinforcement),
            Self::ApplicabilityNegative => {
                Some(GenerationSemanticUpdateV3::ApplicabilityCounterWave)
            }
            Self::HardContradiction => Some(GenerationSemanticUpdateV3::StructuralRevision),
            Self::Censored(_) => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationEvidenceObservationV3 {
    pub(super) generation_id_sha256: String,
    pub(super) capture_sequence: u64,
    pub(super) support_watermark_next_sequence: u64,
    pub(super) support_freeze_sha256: Option<String>,
    pub(super) lineage_root_sha256: String,
    pub(super) event_root_sha256: String,
    pub(super) request_root_sha256: String,
    pub(super) verifier_receipt_root_sha256: String,
    pub(super) outcome: GenerationLearningOutcomeV3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationEvidenceObservationInputV3 {
    pub generation_id_sha256: String,
    pub capture_sequence: u64,
    pub support_watermark_next_sequence: u64,
    pub support_freeze_sha256: Option<String>,
    pub lineage_root_sha256: String,
    pub event_root_sha256: String,
    pub request_root_sha256: String,
    pub verifier_receipt_root_sha256: String,
    pub outcome: GenerationLearningOutcomeV3,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationEvidenceRecordV3 {
    pub(super) partition: GenerationEvidencePartitionV3,
    pub(super) ordinal: u32,
    pub(super) previous_record_sha256: String,
    pub(super) observation: GenerationEvidenceObservationV3,
    pub(super) record_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationSupportFreezeV3 {
    pub(super) next_capture_sequence: u64,
    pub(super) watermark_root_sha256: String,
    pub(super) support_partition_sha256: String,
    pub(super) support_lineages: u32,
    pub(super) freeze_sha256: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GenerationEvidenceAccountingV3 {
    pub support_rows: usize,
    pub support_lineages: usize,
    pub future_rows: usize,
    pub future_lineages: usize,
    pub positive_rows: usize,
    pub applicability_negative_rows: usize,
    pub hard_contradiction_rows: usize,
    pub censored_rows: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationEvidenceErrorV3 {
    InvalidGeneration,
    InvalidRoot,
    InvalidSequence,
    InvalidPartitionBinding,
    VerifierOutcomeMismatch,
    SupportClosed,
    SupportNotFrozen,
    EmptySupport,
    BeforeWatermark,
    CrossPartitionLineage,
    DuplicateEvent,
    DuplicateRequest,
    DuplicateReceipt,
    PartitionBudgetExhausted,
    LedgerBudgetExhausted,
    InvalidRecord,
    InvalidFreeze,
    InvalidLedger,
    Serialization,
}

pub fn seal_generation_evidence_observation_v3(
    input: GenerationEvidenceObservationInputV3,
) -> Result<GenerationEvidenceObservationV3, GenerationEvidenceErrorV3> {
    let observation = GenerationEvidenceObservationV3 {
        generation_id_sha256: input.generation_id_sha256,
        capture_sequence: input.capture_sequence,
        support_watermark_next_sequence: input.support_watermark_next_sequence,
        support_freeze_sha256: input.support_freeze_sha256,
        lineage_root_sha256: input.lineage_root_sha256,
        event_root_sha256: input.event_root_sha256,
        request_root_sha256: input.request_root_sha256,
        verifier_receipt_root_sha256: input.verifier_receipt_root_sha256,
        outcome: input.outcome,
    };
    observation.validate()?;
    Ok(observation)
}

impl GenerationEvidenceObservationV3 {
    pub(super) fn validate(&self) -> Result<(), GenerationEvidenceErrorV3> {
        if !valid_nonzero_sha256(&self.generation_id_sha256) {
            return Err(GenerationEvidenceErrorV3::InvalidGeneration);
        }
        if self.capture_sequence == 0 || self.support_watermark_next_sequence == 0 {
            return Err(GenerationEvidenceErrorV3::InvalidSequence);
        }
        let roots = [
            self.lineage_root_sha256.as_str(),
            self.event_root_sha256.as_str(),
            self.request_root_sha256.as_str(),
            self.verifier_receipt_root_sha256.as_str(),
        ];
        roots
            .iter()
            .all(|root| valid_nonzero_sha256(root))
            .then_some(())
            .ok_or(GenerationEvidenceErrorV3::InvalidRoot)?;
        if self
            .support_freeze_sha256
            .as_deref()
            .is_some_and(|root| !valid_nonzero_sha256(root))
        {
            return Err(GenerationEvidenceErrorV3::InvalidRoot);
        }
        Ok(())
    }

    #[must_use]
    pub fn generation_id_sha256(&self) -> &str {
        &self.generation_id_sha256
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
    pub fn request_root_sha256(&self) -> &str {
        &self.request_root_sha256
    }

    #[must_use]
    pub fn verifier_receipt_root_sha256(&self) -> &str {
        &self.verifier_receipt_root_sha256
    }

    #[must_use]
    pub const fn outcome(&self) -> GenerationLearningOutcomeV3 {
        self.outcome
    }
}

impl GenerationEvidenceRecordV3 {
    pub(super) fn digest(
        generation_id_sha256: &str,
        partition: GenerationEvidencePartitionV3,
        ordinal: u32,
        previous_record_sha256: &str,
        observation: &GenerationEvidenceObservationV3,
    ) -> Result<String, GenerationEvidenceErrorV3> {
        canonical_json_sha256(&(
            GENERATION_EVIDENCE_LEDGER_SCHEMA_V3,
            generation_id_sha256,
            partition,
            ordinal,
            previous_record_sha256,
            observation,
        ))
        .map_err(|_| GenerationEvidenceErrorV3::Serialization)
    }

    #[must_use]
    pub const fn partition(&self) -> GenerationEvidencePartitionV3 {
        self.partition
    }

    #[must_use]
    pub const fn observation(&self) -> &GenerationEvidenceObservationV3 {
        &self.observation
    }

    #[must_use]
    pub fn record_sha256(&self) -> &str {
        &self.record_sha256
    }
}

impl GenerationSupportFreezeV3 {
    #[must_use]
    pub const fn next_capture_sequence(&self) -> u64 {
        self.next_capture_sequence
    }

    #[must_use]
    pub fn freeze_sha256(&self) -> &str {
        &self.freeze_sha256
    }
}
