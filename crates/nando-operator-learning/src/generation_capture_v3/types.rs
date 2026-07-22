use nando_operator_kernel::valid_nonzero_sha256;
use serde::{Deserialize, Serialize};

pub const GENERATION_CAPTURE_INDEX_MAX_RECORDS_V3: usize = 16_384;
pub const GENERATION_CAPTURE_INDEX_MAX_BYTES_V3: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationCaptureErrorV3 {
    InvalidCommitment,
    DuplicateCommitment,
    BudgetExhausted,
    InvalidIndex,
    Serialization,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationCaptureCommitmentV3 {
    capture_sequence: u64,
    record_root_sha256: String,
    lineage_root_sha256: String,
    event_root_sha256: String,
    request_root_sha256: String,
}

impl GenerationCaptureCommitmentV3 {
    pub fn new(
        capture_sequence: u64,
        record_root_sha256: String,
        lineage_root_sha256: String,
        event_root_sha256: String,
        request_root_sha256: String,
    ) -> Result<Self, GenerationCaptureErrorV3> {
        let commitment = Self {
            capture_sequence,
            record_root_sha256,
            lineage_root_sha256,
            event_root_sha256,
            request_root_sha256,
        };
        commitment.validate()?;
        Ok(commitment)
    }

    pub(super) fn validate(&self) -> Result<(), GenerationCaptureErrorV3> {
        [
            self.record_root_sha256.as_str(),
            self.lineage_root_sha256.as_str(),
            self.event_root_sha256.as_str(),
            self.request_root_sha256.as_str(),
        ]
        .into_iter()
        .all(valid_nonzero_sha256)
        .then_some(())
        .ok_or(GenerationCaptureErrorV3::InvalidCommitment)
    }

    #[must_use]
    pub const fn capture_sequence(&self) -> u64 {
        self.capture_sequence
    }

    #[must_use]
    pub fn record_root_sha256(&self) -> &str {
        &self.record_root_sha256
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
}
