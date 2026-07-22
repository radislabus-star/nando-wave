use nando_operator_kernel::canonical_json_sha256;
use nando_operator_learning::GenerationCaptureIndexV3;

use super::RestoredGenerationCheckpointV3;

pub const CAPTURE_JOINED_GENERATION_SCHEMA_V3: &str =
    "nando.capture-joined-generation-checkpoint.v3.f7";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationCaptureJoinErrorV3 {
    MissingCaptureCommitment,
    Serialization,
}

pub struct CaptureJoinedGenerationCheckpointV3 {
    checkpoint: RestoredGenerationCheckpointV3,
    capture_index_sha256: String,
    join_sha256: String,
}

pub fn join_generation_checkpoint_to_capture_index_v3(
    checkpoint: RestoredGenerationCheckpointV3,
    capture_index: &GenerationCaptureIndexV3,
) -> Result<CaptureJoinedGenerationCheckpointV3, GenerationCaptureJoinErrorV3> {
    for pair in checkpoint.receipts() {
        let receipt = pair.generation_receipt();
        if !capture_index.contains_exact(
            receipt.capture_sequence(),
            receipt.lineage_root_sha256(),
            receipt.event_root_sha256(),
            receipt.f6_request_sha256(),
        ) {
            return Err(GenerationCaptureJoinErrorV3::MissingCaptureCommitment);
        }
    }
    let capture_index_sha256 = capture_index.index_sha256().to_owned();
    let join_sha256 = canonical_json_sha256(&(
        CAPTURE_JOINED_GENERATION_SCHEMA_V3,
        checkpoint.checkpoint_sha256(),
        capture_index_sha256.as_str(),
        checkpoint.receipt_set_sha256(),
    ))
    .map_err(|_| GenerationCaptureJoinErrorV3::Serialization)?;
    Ok(CaptureJoinedGenerationCheckpointV3 {
        checkpoint,
        capture_index_sha256,
        join_sha256,
    })
}

impl CaptureJoinedGenerationCheckpointV3 {
    #[must_use]
    pub const fn checkpoint(&self) -> &RestoredGenerationCheckpointV3 {
        &self.checkpoint
    }

    #[must_use]
    pub fn capture_index_sha256(&self) -> &str {
        &self.capture_index_sha256
    }

    #[must_use]
    pub fn join_sha256(&self) -> &str {
        &self.join_sha256
    }

    #[must_use]
    pub fn into_checkpoint(self) -> RestoredGenerationCheckpointV3 {
        self.checkpoint
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}
