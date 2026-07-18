use nando_core::wave::{CoherentOperatorCandidate, OperatorPage32, OperatorPage32Error};

use crate::{
    AdmissionReadyOperatorGeneration, ResponseProgram, VerifierProgram,
    response_actor_program_digest, response_independent_verifier_program_digest,
    response_program_external_verifier_schema,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ShadowTransferableOperatorV2 {
    page: OperatorPage32,
    actor: ResponseProgram,
    verifier: VerifierProgram,
    actor_sha256: String,
    verifier_sha256: String,
    verifier_schema: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProvenTransferableOperatorV2 {
    shadow: ShadowTransferableOperatorV2,
    proof_generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferableOperatorV2Error {
    InvalidPage(OperatorPage32Error),
    GenerationMismatch,
    CircuitFingerprintMismatch,
    RoleCountMismatch,
    InvalidActor,
    MissingExternalVerifierSchema,
    ActorVerifierMismatch,
    DigestFailure,
    AdmissionPageMismatch,
    AdmissionProofMismatch,
}

impl ShadowTransferableOperatorV2 {
    pub fn new(
        candidate: &CoherentOperatorCandidate,
        page: OperatorPage32,
        actor: ResponseProgram,
        verifier: VerifierProgram,
    ) -> Result<Self, TransferableOperatorV2Error> {
        page.validate()
            .map_err(TransferableOperatorV2Error::InvalidPage)?;
        let header = page
            .header()
            .map_err(TransferableOperatorV2Error::InvalidPage)?;
        if header.generation != candidate.candidate_generation {
            return Err(TransferableOperatorV2Error::GenerationMismatch);
        }
        if header.circuit_fingerprint64 != candidate.circuit.fingerprint64() {
            return Err(TransferableOperatorV2Error::CircuitFingerprintMismatch);
        }
        if header.role_count != candidate.circuit.role_count() {
            return Err(TransferableOperatorV2Error::RoleCountMismatch);
        }
        actor
            .validate()
            .map_err(|_| TransferableOperatorV2Error::InvalidActor)?;
        let verifier_schema = response_program_external_verifier_schema(&actor)
            .ok_or(TransferableOperatorV2Error::MissingExternalVerifierSchema)?;
        if !crate::package::response_program_verifier_matches(&actor, Some(&verifier)) {
            return Err(TransferableOperatorV2Error::ActorVerifierMismatch);
        }
        let actor_sha256 = response_actor_program_digest(&actor)
            .map_err(|_| TransferableOperatorV2Error::DigestFailure)?;
        let verifier_sha256 = response_independent_verifier_program_digest(&verifier)
            .map_err(|_| TransferableOperatorV2Error::DigestFailure)?;
        Ok(Self {
            page,
            actor,
            verifier,
            actor_sha256,
            verifier_sha256,
            verifier_schema: verifier_schema.to_owned(),
        })
    }

    pub fn bind_proof(
        self,
        admission_ready: &AdmissionReadyOperatorGeneration,
    ) -> Result<ProvenTransferableOperatorV2, TransferableOperatorV2Error> {
        if self.page.as_bytes() != admission_ready.page().as_bytes() {
            return Err(TransferableOperatorV2Error::AdmissionPageMismatch);
        }
        let header = self
            .page
            .header()
            .map_err(TransferableOperatorV2Error::InvalidPage)?;
        if header.generation != admission_ready.proof().generation
            || header.circuit_fingerprint64 != admission_ready.proof().circuit_fingerprint64
        {
            return Err(TransferableOperatorV2Error::AdmissionProofMismatch);
        }
        Ok(ProvenTransferableOperatorV2 {
            shadow: self,
            proof_generation: header.generation,
        })
    }

    #[must_use]
    pub fn page(&self) -> &OperatorPage32 {
        &self.page
    }

    #[must_use]
    pub fn actor(&self) -> &ResponseProgram {
        &self.actor
    }

    #[must_use]
    pub fn verifier(&self) -> &VerifierProgram {
        &self.verifier
    }

    #[must_use]
    pub fn actor_sha256(&self) -> &str {
        &self.actor_sha256
    }

    #[must_use]
    pub fn verifier_sha256(&self) -> &str {
        &self.verifier_sha256
    }

    #[must_use]
    pub fn verifier_schema(&self) -> &str {
        &self.verifier_schema
    }
}

impl ProvenTransferableOperatorV2 {
    #[must_use]
    pub fn shadow(&self) -> &ShadowTransferableOperatorV2 {
        &self.shadow
    }

    #[must_use]
    pub const fn proof_generation(&self) -> u64 {
        self.proof_generation
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::FRAC_PI_2;

    use nando_core::wave::{
        OPERATOR_PAGE32_COMPOSITION_BYTES, OPERATOR_PAGE32_PHASE_BYTES,
        OPERATOR_PAGE32_RENDERER_BYTES, OperatorCircuit, OperatorCircuitRelation,
        OperatorPage32Metadata, OperatorRelationCell, PhaseCenterCell, StructuralRole16,
        TernaryOperatorCube32, TernaryRelationState,
    };

    use super::*;
    use crate::{
        AtomValueType, CollectionOutputRenderer, ResponseValueSelector, ValueProjectionFormat,
    };

    fn candidate() -> CoherentOperatorCandidate {
        let relation = |plane, source_role, target_role, angle: f64| OperatorCircuitRelation {
            cell: OperatorRelationCell {
                plane,
                source_role,
                target_role,
            },
            state: TernaryRelationState::Supported,
            phase_anchor: PhaseCenterCell {
                re: angle.cos(),
                im: angle.sin(),
            },
        };
        let circuit = OperatorCircuit::new(
            3,
            vec![
                relation(0, 0, 1, 0.0),
                relation(0, 1, 2, FRAC_PI_2),
                relation(1, 0, 2, 2.0 * FRAC_PI_2),
            ],
        )
        .expect("connected circuit");
        CoherentOperatorCandidate {
            source_generation: 7,
            candidate_generation: 8,
            circuit,
            coherence: 1.0,
            margin_over_runner_up: 0.5,
            independent_surfaces: 3,
            independent_sessions: 3,
            receipt_ids: vec![1, 2, 3].into_boxed_slice(),
        }
    }

    fn page(candidate: &CoherentOperatorCandidate) -> OperatorPage32 {
        OperatorPage32::build(
            OperatorPage32Metadata {
                generation: candidate.candidate_generation,
                circuit_fingerprint64: candidate.circuit.fingerprint64(),
                verifier_binding_fingerprint64: 11,
                proof_lineage_fingerprint64: 12,
                role_signature_fingerprint64: 13,
                relation_plane_count: 2,
                composition_node_count: 0,
                renderer_instruction_count: 0,
                flags: 0,
            },
            &[0; OPERATOR_PAGE32_PHASE_BYTES],
            &[
                StructuralRole16::default(),
                StructuralRole16::default(),
                StructuralRole16::default(),
            ],
            &TernaryOperatorCube32::default(),
            &[],
            &[0; OPERATOR_PAGE32_COMPOSITION_BYTES],
            &[0; OPERATOR_PAGE32_RENDERER_BYTES],
        )
        .expect("candidate page")
    }

    fn actor_and_verifier() -> (ResponseProgram, VerifierProgram) {
        let selector = ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Integer,
        };
        let actor = ResponseProgram::project_selected_value(
            selector.clone(),
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let verifier = VerifierProgram::ProjectSelectedValue {
            selector,
            format: ValueProjectionFormat::PlainText,
            renderer: CollectionOutputRenderer::Direct,
            completion_state: "completed".to_owned(),
            require_unique_value: true,
        };
        (actor, verifier)
    }

    #[test]
    fn coherent_cube_actor_and_independent_verifier_bind_as_one_shadow_operator() {
        let candidate = candidate();
        let (actor, verifier) = actor_and_verifier();
        let operator =
            ShadowTransferableOperatorV2::new(&candidate, page(&candidate), actor, verifier)
                .expect("bound transferable operator");

        assert_eq!(
            operator.page().header().expect("header").generation,
            candidate.candidate_generation
        );
        assert_eq!(operator.actor_sha256().len(), 64);
        assert_eq!(operator.verifier_sha256().len(), 64);
        assert!(!operator.verifier_schema().is_empty());
    }

    #[test]
    fn mismatched_verifier_cannot_bind_to_the_operator_page() {
        let candidate = candidate();
        let (actor, _) = actor_and_verifier();
        let wrong = VerifierProgram::ContinueHandle {
            require_observation_action_equality: true,
            require_pending_state: true,
            require_unique_handle: true,
        };
        assert_eq!(
            ShadowTransferableOperatorV2::new(&candidate, page(&candidate), actor, wrong),
            Err(TransferableOperatorV2Error::ActorVerifierMismatch)
        );
    }
}
