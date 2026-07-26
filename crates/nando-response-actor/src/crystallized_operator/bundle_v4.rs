use serde::{Deserialize, Serialize};

use super::*;

const CRYSTALLIZED_PROOF_ENVELOPE_V4_SCHEMA: &str = "nando.crystallized-proof-envelope.v4";
const CRYSTALLIZED_ROUTING_IMAGE_V4_SCHEMA: &str = "nando.crystallized-routing-image.v4";
const CRYSTALLIZED_EXECUTION_IMAGE_V4_SCHEMA: &str = "nando.crystallized-execution-image.v4";
const CRYSTALLIZED_EXECUTION_PROGRAM_V4_SCHEMA: &str = "nando.crystallized-execution-program.v4";
const CRYSTALLIZED_EXECUTION_PAGE_BYTES: usize = 4_032;
const CRYSTALLIZED_EXECUTION_MAX_EXTENSION_PAGES: usize = 8;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CrystallizedRoutingImageV4 {
    schema: String,
    roles: Vec<nando_operator_kernel::CanonicalOperatorRoleV1>,
    relations: Vec<nando_operator_kernel::CanonicalOperatorRelationV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CrystallizedExecutionProgramV4 {
    schema: String,
    transforms: Vec<nando_operator_kernel::CanonicalOperatorTransformV1>,
    composition_edges: Vec<nando_operator_kernel::CanonicalOperatorCompositionEdgeV1>,
    renderer: crate::CollectionOutputRenderer,
    actor_template: ResponseProgram,
    verifier_contract_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CrystallizedExecutionExtensionPageV4 {
    #[serde(with = "serde_bytes")]
    bytes: Box<[u8]>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CrystallizedExecutionImageV4 {
    schema: String,
    #[serde(with = "serde_bytes")]
    entry_page: Box<[u8]>,
    extension_pages: Vec<CrystallizedExecutionExtensionPageV4>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CrystallizedProofEnvelopeV4 {
    schema: String,
    compiler_generation: u64,
    compiler_support_lineages: Vec<Commitment256>,
    compiler_uses_typed_actor_renderer: bool,
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
        if self.operator.support_lineages.is_empty() {
            return Err(CrystallizedOperatorError::RestartEncode);
        }
        let ir = nando_operator_runtime::canonical_operator_ir_from_runtime_artifact_v1(
            &self.operator.runtime_artifact,
            self.operator.verifier_sha256.clone(),
        )
        .map_err(|_| CrystallizedOperatorError::ProgramCompileFailed)?;
        let law_id = decode_sha256(
            &ir.executable_sha256()
                .map_err(|_| CrystallizedOperatorError::DigestFailure)?,
        )?;
        let routing_image =
            nando_operator_kernel::canonical_json_bytes(&CrystallizedRoutingImageV4 {
                schema: CRYSTALLIZED_ROUTING_IMAGE_V4_SCHEMA.to_owned(),
                roles: ir.roles().to_vec(),
                relations: ir.relations().to_vec(),
            })
            .map_err(|_| CrystallizedOperatorError::RestartEncode)?;
        let execution_program =
            nando_operator_kernel::canonical_json_bytes(&CrystallizedExecutionProgramV4 {
                schema: CRYSTALLIZED_EXECUTION_PROGRAM_V4_SCHEMA.to_owned(),
                transforms: ir.transforms().to_vec(),
                composition_edges: ir.composition_edges().to_vec(),
                renderer: ir.renderer().clone(),
                actor_template: ir.actor_template().clone(),
                verifier_contract_sha256: ir.verifier_contract_sha256().to_owned(),
            })
            .map_err(|_| CrystallizedOperatorError::RestartEncode)?;
        let extension_pages = execution_program
            .chunks(CRYSTALLIZED_EXECUTION_PAGE_BYTES)
            .map(|bytes| CrystallizedExecutionExtensionPageV4 {
                bytes: bytes.to_vec().into_boxed_slice(),
            })
            .collect::<Vec<_>>();
        if extension_pages.is_empty()
            || extension_pages.len() > CRYSTALLIZED_EXECUTION_MAX_EXTENSION_PAGES
        {
            return Err(CrystallizedOperatorError::RestartEncode);
        }
        let execution_image =
            nando_operator_kernel::canonical_json_bytes(&CrystallizedExecutionImageV4 {
                schema: CRYSTALLIZED_EXECUTION_IMAGE_V4_SCHEMA.to_owned(),
                entry_page: self.page().as_bytes().to_vec().into_boxed_slice(),
                extension_pages,
            })
            .map_err(|_| CrystallizedOperatorError::RestartEncode)?;
        let verifier = self.routing_verifier()?;
        let verifier_image = nando_operator_kernel::canonical_json_bytes(&verifier)
            .map_err(|_| CrystallizedOperatorError::RestartEncode)?;
        let proof_image =
            nando_operator_kernel::canonical_json_bytes(&CrystallizedProofEnvelopeV4::from(self))
                .map_err(|_| CrystallizedOperatorError::RestartEncode)?;
        nando_operator_persistence::CrystallizedOperatorBundleV4::seal(
            law_id,
            routing_image.into_boxed_slice(),
            execution_image.into_boxed_slice(),
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
        let routing: CrystallizedRoutingImageV4 = serde_json::from_slice(bundle.routing_image())
            .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        let canonical_routing = nando_operator_kernel::canonical_json_bytes(&routing)
            .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        if routing.schema != CRYSTALLIZED_ROUTING_IMAGE_V4_SCHEMA
            || canonical_routing.as_slice() != bundle.routing_image()
        {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let execution: CrystallizedExecutionImageV4 =
            serde_json::from_slice(bundle.execution_image())
                .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        let canonical_execution = nando_operator_kernel::canonical_json_bytes(&execution)
            .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        if execution.schema != CRYSTALLIZED_EXECUTION_IMAGE_V4_SCHEMA
            || canonical_execution.as_slice() != bundle.execution_image()
            || execution.entry_page.len() != nando_core::wave::OPERATOR_PAGE32_BYTES
            || execution.extension_pages.is_empty()
            || execution.extension_pages.len() > CRYSTALLIZED_EXECUTION_MAX_EXTENSION_PAGES
            || execution.extension_pages.iter().any(|page| {
                page.bytes.is_empty() || page.bytes.len() > CRYSTALLIZED_EXECUTION_PAGE_BYTES
            })
        {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let mut execution_program_bytes =
            Vec::with_capacity(execution.extension_pages.len() * CRYSTALLIZED_EXECUTION_PAGE_BYTES);
        for page in &execution.extension_pages {
            execution_program_bytes.extend_from_slice(&page.bytes);
        }
        let execution_program: CrystallizedExecutionProgramV4 =
            serde_json::from_slice(&execution_program_bytes)
                .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        let canonical_execution_program =
            nando_operator_kernel::canonical_json_bytes(&execution_program)
                .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        if execution_program.schema != CRYSTALLIZED_EXECUTION_PROGRAM_V4_SCHEMA
            || canonical_execution_program != execution_program_bytes
        {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let ir = nando_operator_kernel::CanonicalOperatorIrV1::new(
            routing.roles,
            routing.relations,
            execution_program.transforms,
            execution_program.composition_edges,
            execution_program.renderer,
            execution_program.actor_template,
            execution_program.verifier_contract_sha256,
        )
        .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        let law_id = decode_sha256(
            &ir.executable_sha256()
                .map_err(|_| CrystallizedOperatorError::RestartDecode)?,
        )?;
        if &law_id != bundle.manifest().law_id() {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let page = OperatorPage32::from_bytes(&execution.entry_page)
            .map_err(CrystallizedOperatorError::InvalidPage)?;
        let verifier: VerifierProgram = serde_json::from_slice(bundle.verifier_image())
            .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        let canonical_verifier = nando_operator_kernel::canonical_json_bytes(&verifier)
            .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        if canonical_verifier.as_slice() != bundle.verifier_image() {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let proof: CrystallizedProofEnvelopeV4 = serde_json::from_slice(bundle.proof_envelope())
            .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        let canonical_proof = nando_operator_kernel::canonical_json_bytes(&proof)
            .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        if canonical_proof.as_slice() != bundle.proof_envelope()
            || proof.schema != CRYSTALLIZED_PROOF_ENVELOPE_V4_SCHEMA
            || proof.compiler_support_lineages.is_empty()
            || proof.wrong_accepts != 0
            || proof.future_lineage_count as usize != proof.verified_future_lineages.len()
            || proof.future_evidence_count < proof.future_lineage_count
        {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let support_lineages = proof
            .compiler_support_lineages
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let future_lineages = proof
            .verified_future_lineages
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if support_lineages.len() != proof.compiler_support_lineages.len()
            || future_lineages.len() != proof.verified_future_lineages.len()
            || !support_lineages.is_disjoint(&future_lineages)
        {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let (rebuilt_page, compiled) = compile_operator_page_from_ir(
            &ir,
            proof.compiler_generation,
            &proof.compiler_support_lineages,
            &proof.verified_future_lineages,
            proof.compiler_uses_typed_actor_renderer,
        )
        .map_err(|_| CrystallizedOperatorError::RestartDigestMismatch)?;
        if rebuilt_page.as_bytes() != page.as_bytes() {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        if source_neutral_verifier_for_program(compiled.actor_template())
            .map_err(|_| CrystallizedOperatorError::VerifierBuildFailed)?
            != verifier
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
                compiler_generation: proof.compiler_generation,
                blueprint_sha256: proof.blueprint_sha256,
                candidate_set_sha256: proof.candidate_set_sha256,
                support_root_sha256: proof.support_root_sha256,
                future_evidence_root_sha256: proof.future_evidence_root_sha256,
                future_lineage_root_sha256: proof.future_lineage_root_sha256,
                winner_seal_sha256: proof.winner_seal_sha256,
                actor_sha256: proof.actor_sha256,
                verifier_sha256: proof.verifier_sha256,
                support_lineages: proof.compiler_support_lineages.into_boxed_slice(),
                verified_future_lineages: proof.verified_future_lineages.into_boxed_slice(),
                uses_typed_actor_renderer: proof.compiler_uses_typed_actor_renderer,
            },
            parity_seal,
        })
    }
}

impl From<&VerifiedCrystallizedOperator> for CrystallizedProofEnvelopeV4 {
    fn from(operator: &VerifiedCrystallizedOperator) -> Self {
        Self {
            schema: CRYSTALLIZED_PROOF_ENVELOPE_V4_SCHEMA.to_owned(),
            compiler_generation: operator.operator.compiler_generation,
            compiler_support_lineages: operator.operator.support_lineages.to_vec(),
            compiler_uses_typed_actor_renderer: operator.operator.uses_typed_actor_renderer,
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

#[cfg(test)]
pub(super) fn reseal_with_alternate_compiler_page_for_test(
    operator: &VerifiedCrystallizedOperator,
    bundle: &nando_operator_persistence::CrystallizedOperatorBundleV4,
) -> nando_operator_persistence::CrystallizedOperatorBundleV4 {
    let ir = nando_operator_runtime::canonical_operator_ir_from_runtime_artifact_v1(
        &operator.operator.runtime_artifact,
        operator.operator.verifier_sha256.clone(),
    )
    .expect("test IR");
    let (alternate_page, _) = compile_operator_page_from_ir(
        &ir,
        operator.operator.compiler_generation.saturating_add(1),
        &operator.operator.support_lineages,
        &operator.operator.verified_future_lineages,
        operator.operator.uses_typed_actor_renderer,
    )
    .expect("alternate compiler page");
    let mut execution: CrystallizedExecutionImageV4 =
        serde_json::from_slice(bundle.execution_image()).expect("execution image");
    execution.entry_page = alternate_page.as_bytes().to_vec().into_boxed_slice();
    let execution_image =
        nando_operator_kernel::canonical_json_bytes(&execution).expect("execution bytes");
    nando_operator_persistence::CrystallizedOperatorBundleV4::seal(
        *bundle.manifest().law_id(),
        bundle.routing_image().to_vec().into_boxed_slice(),
        execution_image.into_boxed_slice(),
        bundle.verifier_image().to_vec().into_boxed_slice(),
        bundle.proof_envelope().to_vec().into_boxed_slice(),
    )
    .expect("resealed test bundle")
}
