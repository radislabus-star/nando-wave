use nando_operator_kernel::GenerationEvidencePartitionV3;
use nando_operator_proof::generation_receipt_v3::GenerationVerifierReceiptV3;

use super::{
    GenerationEvidenceErrorV3, GenerationEvidenceLedgerV3, GenerationEvidenceObservationInputV3,
    GenerationEvidenceRecordV3, GenerationLearningOutcomeV3,
    seal_generation_evidence_observation_v3,
};

impl GenerationEvidenceLedgerV3 {
    pub fn append_generation_verifier_receipt(
        &mut self,
        receipt: &GenerationVerifierReceiptV3,
        outcome: GenerationLearningOutcomeV3,
    ) -> Result<&GenerationEvidenceRecordV3, GenerationEvidenceErrorV3> {
        if receipt.generation_id_sha256() != self.generation_id_sha256() {
            return Err(GenerationEvidenceErrorV3::InvalidGeneration);
        }
        if receipt.is_verified_pass()
            != matches!(outcome, GenerationLearningOutcomeV3::VerifiedPass)
        {
            return Err(GenerationEvidenceErrorV3::VerifierOutcomeMismatch);
        }
        let observation =
            seal_generation_evidence_observation_v3(GenerationEvidenceObservationInputV3 {
                generation_id_sha256: receipt.generation_id_sha256().to_owned(),
                capture_sequence: receipt.capture_sequence(),
                support_watermark_next_sequence: receipt.support_watermark_next_sequence(),
                support_freeze_sha256: receipt.support_freeze_sha256().map(str::to_owned),
                lineage_root_sha256: receipt.lineage_root_sha256().to_owned(),
                event_root_sha256: receipt.event_root_sha256().to_owned(),
                request_root_sha256: receipt.f6_request_sha256().to_owned(),
                verifier_receipt_root_sha256: receipt.generation_receipt_sha256().to_owned(),
                outcome,
            })?;
        match receipt.partition() {
            GenerationEvidencePartitionV3::Support => self.append_support(observation),
            GenerationEvidencePartitionV3::Future => self.append_future(observation),
        }
    }
}
