use nando_core::wave::{
    CandidateCubeField, CandidateCubeFieldError, CircuitSynthesisConfig, CircuitSynthesisError,
    CircuitSynthesizer, CoherentOperatorCandidate, FrozenCircuitSetError,
    FrozenSynthesizedCircuitSet, OperatorCircuit, OperatorCircuitSynthesisReport,
    OperatorConsolidationReport, OperatorGrokkingConfig, OperatorGrokkingConsolidator,
    OperatorPage32, OperatorPage32Error, ProvenOperatorGrokking, VerifiedPartialRelationWave,
};

use crate::{BackwardWave, BackwardWaveError, BackwardWaveUpdate, VerifiedDeltaReceipt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatorGenerationError {
    InvalidActivePage(OperatorPage32Error),
    CandidateField(CandidateCubeFieldError),
    BackwardWave(BackwardWaveError),
    CircuitSynthesis(CircuitSynthesisError),
    FrozenCircuitSet(FrozenCircuitSetError),
    SupportGenerationMismatch,
    SupportReceiptReused,
    CandidateGenerationMismatch,
    CandidateFingerprintMismatch,
    ProofMismatch,
    ShadowPageMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissionReadyOperatorGeneration {
    source_generation: u64,
    source_operator_fingerprint64: u64,
    page: OperatorPage32,
    proof: ProvenOperatorGrokking,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperatorGenerationFirewall {
    active_page: OperatorPage32,
    active_generation: u64,
    active_operator_fingerprint64: u64,
    candidate_field: CandidateCubeField,
    support_receipt_ids: Box<[u64]>,
}

impl OperatorGenerationFirewall {
    pub fn new(
        active_page: OperatorPage32,
        candidate_circuits: Vec<OperatorCircuit>,
        config: OperatorGrokkingConfig,
    ) -> Result<Self, OperatorGenerationError> {
        Self::new_with_support_receipts(active_page, candidate_circuits, config, Box::new([]))
    }

    pub fn from_synthesized_support(
        active_page: OperatorPage32,
        support_waves: &[VerifiedPartialRelationWave],
        synthesis_config: CircuitSynthesisConfig,
        grokking_config: OperatorGrokkingConfig,
    ) -> Result<(Self, OperatorCircuitSynthesisReport), OperatorGenerationError> {
        let header = active_page
            .header()
            .map_err(OperatorGenerationError::InvalidActivePage)?;
        if support_waves
            .iter()
            .any(|wave| wave.generation != header.generation)
        {
            return Err(OperatorGenerationError::SupportGenerationMismatch);
        }
        let report = CircuitSynthesizer::synthesize(support_waves, synthesis_config)
            .map_err(OperatorGenerationError::CircuitSynthesis)?;
        let frozen = FrozenSynthesizedCircuitSet::freeze(header.generation, &report)
            .map_err(OperatorGenerationError::FrozenCircuitSet)?;
        let firewall = Self::new_with_support_receipts(
            active_page,
            frozen.circuits().to_vec(),
            grokking_config,
            frozen.support_receipt_ids().to_vec().into_boxed_slice(),
        )?;
        Ok((firewall, report))
    }

    fn new_with_support_receipts(
        active_page: OperatorPage32,
        candidate_circuits: Vec<OperatorCircuit>,
        config: OperatorGrokkingConfig,
        support_receipt_ids: Box<[u64]>,
    ) -> Result<Self, OperatorGenerationError> {
        let header = active_page
            .header()
            .map_err(OperatorGenerationError::InvalidActivePage)?;
        active_page
            .validate()
            .map_err(OperatorGenerationError::InvalidActivePage)?;
        let mut candidate_field = CandidateCubeField::new(header.generation, config)
            .map_err(OperatorGenerationError::CandidateField)?;
        for circuit in candidate_circuits {
            candidate_field
                .register_circuit(circuit)
                .map_err(OperatorGenerationError::CandidateField)?;
        }
        Ok(Self {
            active_page,
            active_generation: header.generation,
            active_operator_fingerprint64: header.circuit_fingerprint64,
            candidate_field,
            support_receipt_ids,
        })
    }

    pub fn observe_verified_delta(
        &mut self,
        receipt: &VerifiedDeltaReceipt,
    ) -> Result<BackwardWaveUpdate, OperatorGenerationError> {
        if self
            .support_receipt_ids
            .binary_search(&BackwardWave::receipt_id(receipt))
            .is_ok()
        {
            return Err(OperatorGenerationError::SupportReceiptReused);
        }
        BackwardWave::apply(
            &mut self.candidate_field,
            self.active_operator_fingerprint64,
            receipt,
        )
        .map_err(OperatorGenerationError::BackwardWave)
    }

    #[must_use]
    pub fn consolidate(&self) -> OperatorConsolidationReport {
        OperatorGrokkingConsolidator::consolidate(&self.candidate_field)
    }

    pub fn seal_admission_ready_shadow(
        &self,
        candidate: &CoherentOperatorCandidate,
        shadow_page: OperatorPage32,
        proof: ProvenOperatorGrokking,
    ) -> Result<AdmissionReadyOperatorGeneration, OperatorGenerationError> {
        if candidate.source_generation != self.active_generation
            || candidate.candidate_generation != self.active_generation.saturating_add(1)
        {
            return Err(OperatorGenerationError::CandidateGenerationMismatch);
        }
        if candidate.circuit.fingerprint64() != proof.circuit_fingerprint64 {
            return Err(OperatorGenerationError::ProofMismatch);
        }
        if candidate.candidate_generation != proof.generation {
            return Err(OperatorGenerationError::ProofMismatch);
        }
        let header = shadow_page
            .header()
            .map_err(OperatorGenerationError::InvalidActivePage)?;
        shadow_page
            .validate()
            .map_err(OperatorGenerationError::InvalidActivePage)?;
        if header.generation != candidate.candidate_generation {
            return Err(OperatorGenerationError::ShadowPageMismatch);
        }
        if header.circuit_fingerprint64 != candidate.circuit.fingerprint64() {
            return Err(OperatorGenerationError::CandidateFingerprintMismatch);
        }
        Ok(AdmissionReadyOperatorGeneration {
            source_generation: self.active_generation,
            source_operator_fingerprint64: self.active_operator_fingerprint64,
            page: shadow_page,
            proof,
        })
    }

    #[must_use]
    pub fn active_page(&self) -> &OperatorPage32 {
        &self.active_page
    }

    #[must_use]
    pub const fn active_generation(&self) -> u64 {
        self.active_generation
    }

    #[must_use]
    pub const fn active_operator_fingerprint64(&self) -> u64 {
        self.active_operator_fingerprint64
    }

    #[must_use]
    pub fn candidate_field(&self) -> &CandidateCubeField {
        &self.candidate_field
    }

    #[must_use]
    pub fn support_receipt_ids(&self) -> &[u64] {
        &self.support_receipt_ids
    }
}

impl AdmissionReadyOperatorGeneration {
    #[must_use]
    pub const fn source_generation(&self) -> u64 {
        self.source_generation
    }

    #[must_use]
    pub const fn source_operator_fingerprint64(&self) -> u64 {
        self.source_operator_fingerprint64
    }

    #[must_use]
    pub fn page(&self) -> &OperatorPage32 {
        &self.page
    }

    #[must_use]
    pub fn proof(&self) -> &ProvenOperatorGrokking {
        &self.proof
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use nando_core::wave::{
        OPERATOR_PAGE32_COMPOSITION_BYTES, OPERATOR_PAGE32_PHASE_BYTES,
        OPERATOR_PAGE32_RENDERER_BYTES, OperatorCircuitRelation, OperatorPage32Metadata,
        OperatorRelationCell, PhaseCenterCell, StructuralRole16, TernaryOperatorCube32,
        TernaryRelationState,
    };

    use super::*;
    use crate::{
        TYPED_EXECUTION_STAGE_RECEIPT_SCHEMA_V1, TypedExecutionStage, TypedExecutionStageReceipt,
        VerifiedDeltaOutcome, VerifiedDeltaRelation, VerifiedDeltaRelationState,
    };

    fn active_page() -> OperatorPage32 {
        OperatorPage32::build(
            OperatorPage32Metadata {
                generation: 7,
                circuit_fingerprint64: 42,
                verifier_binding_fingerprint64: 11,
                proof_lineage_fingerprint64: 12,
                role_signature_fingerprint64: 13,
                relation_plane_count: 1,
                composition_node_count: 0,
                renderer_instruction_count: 0,
                flags: 0,
            },
            &[0; OPERATOR_PAGE32_PHASE_BYTES],
            &[StructuralRole16::default()],
            &TernaryOperatorCube32::default(),
            &[],
            &[0; OPERATOR_PAGE32_COMPOSITION_BYTES],
            &[0; OPERATOR_PAGE32_RENDERER_BYTES],
        )
        .expect("active page")
    }

    fn phase(angle: f64) -> PhaseCenterCell {
        PhaseCenterCell {
            re: angle.cos(),
            im: angle.sin(),
        }
    }

    fn circuit() -> OperatorCircuit {
        OperatorCircuit::new(
            3,
            vec![
                OperatorCircuitRelation {
                    cell: OperatorRelationCell {
                        plane: 0,
                        source_role: 0,
                        target_role: 1,
                    },
                    state: TernaryRelationState::Supported,
                    phase_anchor: phase(0.0),
                },
                OperatorCircuitRelation {
                    cell: OperatorRelationCell {
                        plane: 0,
                        source_role: 1,
                        target_role: 2,
                    },
                    state: TernaryRelationState::Supported,
                    phase_anchor: phase(FRAC_PI_2),
                },
                OperatorCircuitRelation {
                    cell: OperatorRelationCell {
                        plane: 1,
                        source_role: 0,
                        target_role: 2,
                    },
                    state: TernaryRelationState::Supported,
                    phase_anchor: phase(PI),
                },
            ],
        )
        .expect("connected circuit")
    }

    fn receipt(
        surface: u8,
        plane: u8,
        source_role: u8,
        target_role: u8,
        phase_re_micro: i32,
        phase_im_micro: i32,
    ) -> VerifiedDeltaReceipt {
        let receipts = TypedExecutionStage::ALL
            .into_iter()
            .map(|stage| TypedExecutionStageReceipt {
                schema: TYPED_EXECUTION_STAGE_RECEIPT_SCHEMA_V1.to_owned(),
                stage,
                generation: 7,
                operator_fingerprint64: 42,
                surface_id_sha256: format!("{:064x}", surface),
                session_id_sha256: format!("{:064x}", surface + 10),
                input_relation_sha256: format!("{:064x}", surface + 20),
                predicted_relation_sha256: format!("{:064x}", surface + 30),
                observed_relation_sha256: format!("{:064x}", surface + 30),
                stage_payload_sha256: format!("{:064x}", stage as u8 + 40),
                independently_observed: stage == TypedExecutionStage::IndependentVerifier,
                accepted: true,
            })
            .collect();
        VerifiedDeltaReceipt::from_typed_trace(
            receipts,
            VerifiedDeltaOutcome::Positive,
            vec![VerifiedDeltaRelation {
                plane,
                source_role,
                target_role,
                state: VerifiedDeltaRelationState::Supported,
                phase_re_micro,
                phase_im_micro,
            }],
        )
        .expect("verified delta")
    }

    #[test]
    fn verified_feedback_builds_g_plus_one_without_mutating_active_page() {
        let active = active_page();
        let original_bytes = *active.as_bytes();
        let mut firewall = OperatorGenerationFirewall::new(
            active,
            vec![circuit()],
            OperatorGrokkingConfig::default(),
        )
        .expect("generation firewall");

        for receipt in [
            receipt(1, 0, 0, 1, 1_000_000, 0),
            receipt(2, 0, 1, 2, 0, 1_000_000),
            receipt(3, 1, 0, 2, -1_000_000, 0),
        ] {
            firewall
                .observe_verified_delta(&receipt)
                .expect("verified feedback");
        }

        let report = firewall.consolidate();
        let candidate = report.candidate.expect("g+1 candidate");
        assert_eq!(candidate.source_generation, 7);
        assert_eq!(candidate.candidate_generation, 8);
        assert_eq!(firewall.active_page().as_bytes(), &original_bytes);
        assert_eq!(firewall.active_generation(), 7);
    }

    #[test]
    fn support_receipts_synthesize_their_own_circuit_before_disjoint_future() {
        let support_receipts = [
            receipt(1, 0, 0, 1, 1_000_000, 0),
            receipt(2, 0, 1, 2, 0, 1_000_000),
            receipt(3, 1, 0, 2, -1_000_000, 0),
        ];
        let mut support_field =
            CandidateCubeField::new(7, OperatorGrokkingConfig::default()).expect("support field");
        for receipt in &support_receipts {
            BackwardWave::apply(&mut support_field, 42, receipt).expect("support wave");
        }

        let (mut firewall, synthesis) = OperatorGenerationFirewall::from_synthesized_support(
            active_page(),
            support_field.waves(),
            CircuitSynthesisConfig::default(),
            OperatorGrokkingConfig::default(),
        )
        .expect("autonomous circuit generation");

        assert_eq!(synthesis.emitted_circuits, 1);
        assert_eq!(firewall.candidate_field().circuits().len(), 1);
        assert!(firewall.candidate_field().waves().is_empty());
        assert_eq!(
            firewall.observe_verified_delta(&support_receipts[0]),
            Err(OperatorGenerationError::SupportReceiptReused)
        );

        for receipt in [
            receipt(4, 0, 0, 1, 1_000_000, 0),
            receipt(5, 0, 1, 2, 0, 1_000_000),
            receipt(6, 1, 0, 2, -1_000_000, 0),
        ] {
            firewall
                .observe_verified_delta(&receipt)
                .expect("disjoint future feedback");
        }

        let candidate = firewall
            .consolidate()
            .candidate
            .expect("phase-coherent synthesized candidate");
        assert_eq!(candidate.source_generation, 7);
        assert_eq!(candidate.candidate_generation, 8);
    }
}
