use nando_core::wave::{
    CandidateCubeField, CandidateCubeFieldError, PhaseCenterCell, TernaryRelationState,
    VerifiedPartialRelationWave, VerifiedRelationSample, VerifiedWaveOutcome,
};
use sha2::{Digest, Sha256};

use crate::{
    VERIFIED_DELTA_RECEIPT_SCHEMA_V1, VerifiedDeltaOutcome, VerifiedDeltaReceipt,
    VerifiedDeltaRelationState,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackwardWaveUpdate {
    Applied,
    CensoredIgnored,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackwardWaveError {
    InvalidReceiptSchema,
    OperatorMismatch,
    Field(CandidateCubeFieldError),
    InvalidWave,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BackwardWave;

impl BackwardWave {
    pub fn apply(
        field: &mut CandidateCubeField,
        operator_fingerprint64: u64,
        receipt: &VerifiedDeltaReceipt,
    ) -> Result<BackwardWaveUpdate, BackwardWaveError> {
        if receipt.schema() != VERIFIED_DELTA_RECEIPT_SCHEMA_V1 {
            return Err(BackwardWaveError::InvalidReceiptSchema);
        }
        if receipt.operator_fingerprint64() != operator_fingerprint64 {
            return Err(BackwardWaveError::OperatorMismatch);
        }
        if receipt.outcome() == VerifiedDeltaOutcome::CensoredUnknown {
            return Ok(BackwardWaveUpdate::CensoredIgnored);
        }

        let samples = receipt
            .relations()
            .iter()
            .map(|relation| VerifiedRelationSample {
                cell: nando_core::wave::OperatorRelationCell {
                    plane: relation.plane,
                    source_role: relation.source_role,
                    target_role: relation.target_role,
                },
                state: match relation.state {
                    VerifiedDeltaRelationState::Opposed => TernaryRelationState::Opposed,
                    VerifiedDeltaRelationState::Unresolved => TernaryRelationState::Unresolved,
                    VerifiedDeltaRelationState::Supported => TernaryRelationState::Supported,
                },
                phase: PhaseCenterCell {
                    re: VerifiedDeltaReceipt::phase_component(relation.phase_re_micro),
                    im: VerifiedDeltaReceipt::phase_component(relation.phase_im_micro),
                },
            })
            .collect::<Vec<_>>();
        let wave = VerifiedPartialRelationWave::new(
            digest_id(receipt.receipt_sha256()),
            digest_id(receipt.surface_id_sha256()),
            digest_id(receipt.session_id_sha256()),
            receipt.generation(),
            match receipt.outcome() {
                VerifiedDeltaOutcome::Positive => VerifiedWaveOutcome::Positive,
                VerifiedDeltaOutcome::ApplicabilityNegative => {
                    VerifiedWaveOutcome::ApplicabilityNegative
                }
                VerifiedDeltaOutcome::HardContradiction => VerifiedWaveOutcome::HardContradiction,
                VerifiedDeltaOutcome::CensoredUnknown => VerifiedWaveOutcome::CensoredUnknown,
            },
            samples,
        )
        .map_err(|_| BackwardWaveError::InvalidWave)?;
        field.observe(wave).map_err(BackwardWaveError::Field)?;
        Ok(BackwardWaveUpdate::Applied)
    }
}

fn digest_id(value: &str) -> u64 {
    let digest = Sha256::digest(value.as_bytes());
    u64::from_le_bytes(digest[..8].try_into().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use nando_core::wave::OperatorGrokkingConfig;

    use super::*;
    use crate::{
        TYPED_EXECUTION_STAGE_RECEIPT_SCHEMA_V1, TypedExecutionStage, TypedExecutionStageReceipt,
        VerifiedDeltaRelation,
    };

    fn typed_trace(
        verifier_independent: bool,
        verifier_accepts: bool,
    ) -> Vec<TypedExecutionStageReceipt> {
        TypedExecutionStage::ALL
            .into_iter()
            .map(|stage| TypedExecutionStageReceipt {
                schema: TYPED_EXECUTION_STAGE_RECEIPT_SCHEMA_V1.to_owned(),
                stage,
                generation: 7,
                operator_fingerprint64: 42,
                surface_id_sha256: "1".repeat(64),
                session_id_sha256: "2".repeat(64),
                input_relation_sha256: "3".repeat(64),
                predicted_relation_sha256: "4".repeat(64),
                observed_relation_sha256: "4".repeat(64),
                stage_payload_sha256: format!("{:064x}", stage as u8 + 5),
                independently_observed: stage == TypedExecutionStage::IndependentVerifier
                    && verifier_independent,
                accepted: stage != TypedExecutionStage::IndependentVerifier || verifier_accepts,
            })
            .collect()
    }

    fn relation() -> VerifiedDeltaRelation {
        VerifiedDeltaRelation {
            plane: 0,
            source_role: 0,
            target_role: 1,
            state: VerifiedDeltaRelationState::Supported,
            phase_re_micro: 1_000_000,
            phase_im_micro: 0,
        }
    }

    #[test]
    fn only_complete_independently_verified_trace_reaches_candidate_field() {
        let receipt = VerifiedDeltaReceipt::from_typed_trace(
            typed_trace(true, true),
            VerifiedDeltaOutcome::Positive,
            vec![relation()],
        )
        .expect("independent typed trace");
        let mut field =
            CandidateCubeField::new(7, OperatorGrokkingConfig::default()).expect("candidate field");
        assert_eq!(
            BackwardWave::apply(&mut field, 42, &receipt),
            Ok(BackwardWaveUpdate::Applied)
        );
        assert_eq!(field.waves().len(), 1);
        assert_eq!(field.waves()[0].outcome, VerifiedWaveOutcome::Positive);
    }

    #[test]
    fn non_independent_verifier_cannot_create_verified_delta() {
        let result = VerifiedDeltaReceipt::from_typed_trace(
            typed_trace(false, true),
            VerifiedDeltaOutcome::Positive,
            vec![relation()],
        );
        assert_eq!(
            result,
            Err(crate::VerifiedDeltaError::VerifierNotIndependent)
        );
    }

    #[test]
    fn censored_receipt_does_not_update_wave_field() {
        let receipt = VerifiedDeltaReceipt::from_typed_trace(
            typed_trace(true, true),
            VerifiedDeltaOutcome::CensoredUnknown,
            Vec::new(),
        )
        .expect("valid censored trace");
        let mut field =
            CandidateCubeField::new(7, OperatorGrokkingConfig::default()).expect("candidate field");
        assert_eq!(
            BackwardWave::apply(&mut field, 42, &receipt),
            Ok(BackwardWaveUpdate::CensoredIgnored)
        );
        assert!(field.waves().is_empty());
    }
}
