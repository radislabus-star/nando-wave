use serde::{Deserialize, Serialize};

use super::*;

const CRYSTALLIZED_PROOF_ENVELOPE_V4_SCHEMA: &str = "nando.crystallized-proof-envelope.v4";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CrystallizedProofEnvelopeV4 {
    schema: String,
    blueprint_sha256: Commitment256,
    candidate_set_sha256: Commitment256,
    support_root_sha256: Commitment256,
    future_evidence_root_sha256: Commitment256,
    future_lineage_root_sha256: Commitment256,
    winner_seal_sha256: Commitment256,
    actor_sha256: String,
    verifier_sha256: String,
    verified_future_lineages: Vec<Commitment256>,
    parity_winner_seal_sha256: Commitment256,
    parity_actor_sha256: Commitment256,
    parity_verifier_sha256: Commitment256,
    binding_receipts_root: Commitment256,
    execution_receipts_root: Commitment256,
    future_evidence_count: u32,
    future_lineage_count: u32,
    wrong_accepts: u32,
    parity_seal_sha256: Commitment256,
}

impl VerifiedCrystallizedOperator {
    pub fn crystallized_bundle_v4(
        &self,
    ) -> Result<nando_operator_persistence::CrystallizedOperatorBundleV4, CrystallizedOperatorError>
    {
        let ir = nando_operator_runtime::canonical_operator_ir_from_runtime_artifact_v1(
            &self.operator.runtime_artifact,
            self.operator.verifier_sha256.clone(),
        )
        .map_err(|_| CrystallizedOperatorError::ProgramCompileFailed)?;
        let law_id = decode_sha256(
            &ir.executable_sha256()
                .map_err(|_| CrystallizedOperatorError::DigestFailure)?,
        )?;
        let verifier = self.routing_verifier()?;
        let verifier_image = nando_operator_kernel::canonical_json_bytes(&verifier)
            .map_err(|_| CrystallizedOperatorError::RestartEncode)?;
        let proof_image =
            nando_operator_kernel::canonical_json_bytes(&CrystallizedProofEnvelopeV4::from(self))
                .map_err(|_| CrystallizedOperatorError::RestartEncode)?;
        nando_operator_persistence::CrystallizedOperatorBundleV4::seal(
            law_id,
            ir.canonical_bytes()
                .map_err(|_| CrystallizedOperatorError::RestartEncode)?
                .into_boxed_slice(),
            self.page().as_bytes().to_vec().into_boxed_slice(),
            verifier_image.into_boxed_slice(),
            proof_image.into_boxed_slice(),
        )
        .map_err(|_| CrystallizedOperatorError::RestartEncode)
    }

    pub fn restore_crystallized_bundle_v4(
        bundle: &nando_operator_persistence::CrystallizedOperatorBundleV4,
        expected_bundle_id: &nando_operator_persistence::ContentIdV4,
    ) -> Result<Self, CrystallizedOperatorError> {
        bundle
            .validate()
            .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        if bundle.manifest().bundle_id() != expected_bundle_id {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let ir = nando_operator_kernel::CanonicalOperatorIrV1::from_canonical_bytes(
            bundle.routing_image(),
        )
        .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        let law_id = decode_sha256(
            &ir.executable_sha256()
                .map_err(|_| CrystallizedOperatorError::RestartDecode)?,
        )?;
        if &law_id != bundle.manifest().law_id() {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let compiled = nando_operator_runtime::compile_canonical_operator_ir_v1(&ir)
            .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        let page = OperatorPage32::from_bytes(bundle.execution_image())
            .map_err(CrystallizedOperatorError::InvalidPage)?;
        let verifier: VerifierProgram = serde_json::from_slice(bundle.verifier_image())
            .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        let canonical_verifier = nando_operator_kernel::canonical_json_bytes(&verifier)
            .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        if canonical_verifier.as_slice() != bundle.verifier_image()
            || source_neutral_verifier_for_program(compiled.actor_template())
                .map_err(|_| CrystallizedOperatorError::VerifierBuildFailed)?
                != verifier
        {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let proof: CrystallizedProofEnvelopeV4 = serde_json::from_slice(bundle.proof_envelope())
            .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        let canonical_proof = nando_operator_kernel::canonical_json_bytes(&proof)
            .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        if canonical_proof.as_slice() != bundle.proof_envelope()
            || proof.schema != CRYSTALLIZED_PROOF_ENVELOPE_V4_SCHEMA
            || proof.wrong_accepts != 0
            || proof.future_lineage_count as usize != proof.verified_future_lineages.len()
            || proof.future_evidence_count < proof.future_lineage_count
        {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let actor_sha256 = response_actor_program_digest(compiled.actor_template())
            .map_err(|_| CrystallizedOperatorError::RestartDigestMismatch)?;
        let verifier_sha256 = response_independent_verifier_program_digest(&verifier)
            .map_err(|_| CrystallizedOperatorError::RestartDigestMismatch)?;
        if ir.verifier_contract_sha256() != verifier_sha256 {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let header = page
            .header()
            .map_err(CrystallizedOperatorError::InvalidPage)?;
        let verifier_digest = decode_sha256(&verifier_sha256)?;
        let parity_data = nando_operator_runtime::RuntimeRestartParitySealData {
            winner_seal_sha256: proof.parity_winner_seal_sha256,
            actor_sha256: proof.parity_actor_sha256,
            verifier_sha256: proof.parity_verifier_sha256,
            binding_receipts_root: proof.binding_receipts_root,
            execution_receipts_root: proof.execution_receipts_root,
            future_evidence_count: proof.future_evidence_count,
            future_lineage_count: proof.future_lineage_count,
            wrong_accepts: proof.wrong_accepts,
            seal_sha256: proof.parity_seal_sha256,
        };
        let parity_seal = ExecutableParitySeal::try_from(&parity_data)?;
        if actor_sha256 != proof.actor_sha256
            || verifier_sha256 != proof.verifier_sha256
            || decode_sha256(&actor_sha256)? != proof.parity_actor_sha256
            || verifier_digest != proof.parity_verifier_sha256
            || first_u64(&verifier_digest) != header.verifier_binding_fingerprint64
            || compiled.relation_program().fingerprint64() != header.circuit_fingerprint64
            || usize::from(header.role_count) != compiled.role_graph().canonical_roles().len()
            || usize::from(header.transform_count) != compiled.transform_program().len()
            || proof.winner_seal_sha256 != proof.parity_winner_seal_sha256
        {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let runtime_artifact = nando_operator_runtime::RuntimeOperatorArtifact::new(
            page,
            compiled.relation_program().clone(),
            compiled.role_graph().clone(),
            compiled.transform_program().into(),
            compiled.renderer().clone(),
            compiled.actor_template().clone(),
        );
        Ok(Self {
            operator: CrystallizedOperator {
                runtime_artifact,
                blueprint_sha256: proof.blueprint_sha256,
                candidate_set_sha256: proof.candidate_set_sha256,
                support_root_sha256: proof.support_root_sha256,
                future_evidence_root_sha256: proof.future_evidence_root_sha256,
                future_lineage_root_sha256: proof.future_lineage_root_sha256,
                winner_seal_sha256: proof.winner_seal_sha256,
                actor_sha256: proof.actor_sha256,
                verifier_sha256: proof.verifier_sha256,
                verified_future_lineages: proof.verified_future_lineages.into_boxed_slice(),
            },
            parity_seal,
        })
    }
}

impl From<&VerifiedCrystallizedOperator> for CrystallizedProofEnvelopeV4 {
    fn from(operator: &VerifiedCrystallizedOperator) -> Self {
        Self {
            schema: CRYSTALLIZED_PROOF_ENVELOPE_V4_SCHEMA.to_owned(),
            blueprint_sha256: operator.operator.blueprint_sha256,
            candidate_set_sha256: operator.operator.candidate_set_sha256,
            support_root_sha256: operator.operator.support_root_sha256,
            future_evidence_root_sha256: operator.operator.future_evidence_root_sha256,
            future_lineage_root_sha256: operator.operator.future_lineage_root_sha256,
            winner_seal_sha256: operator.operator.winner_seal_sha256,
            actor_sha256: operator.operator.actor_sha256.clone(),
            verifier_sha256: operator.operator.verifier_sha256.clone(),
            verified_future_lineages: operator.operator.verified_future_lineages.to_vec(),
            parity_winner_seal_sha256: operator.parity_seal.winner_seal_sha256,
            parity_actor_sha256: operator.parity_seal.actor_sha256,
            parity_verifier_sha256: operator.parity_seal.verifier_sha256,
            binding_receipts_root: operator.parity_seal.binding_receipts_root,
            execution_receipts_root: operator.parity_seal.execution_receipts_root,
            future_evidence_count: operator.parity_seal.future_evidence_count,
            future_lineage_count: operator.parity_seal.future_lineage_count,
            wrong_accepts: operator.parity_seal.wrong_accepts,
            parity_seal_sha256: operator.parity_seal.seal_sha256,
        }
    }
}
