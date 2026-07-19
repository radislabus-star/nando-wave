use std::collections::BTreeSet;

use nando_core::wave::{
    BlueprintFutureEvidence, CandidateCubeField, CandidateCubeFieldError,
    CandidateOperatorBlueprint, Commitment256, FrozenBlueprintFutureWindow, LocalRelationFragment,
    OPERATOR_PAGE32_BYTES, OPERATOR_PAGE32_COMPOSITION_BYTES, OPERATOR_PAGE32_PHASE_BYTES,
    OPERATOR_PAGE32_RENDERER_BYTES, OPERATOR_ROLE_NONE, OperatorCircuit, OperatorCircuitRelation,
    OperatorGrokkingConfig, OperatorPage32, OperatorPage32Error, OperatorPage32Metadata,
    OperatorRelationCell, PhaseCenterCell, RoleGraph, RuntimeRoleBinder,
    SealedBlueprintWinnerReceipt, SearchCompletion, StructuralRole16, StructuralRoleSignature,
    SurfaceFragmentBundle, TernaryOperatorCube32, TernaryRelationState, TransformOp8,
    TypedProgramAtom, phase_vector_from_atoms,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    AtomValueType, BackwardWave, BackwardWaveError, BackwardWaveUpdate, ResponseExecutionStatus,
    ResponseProgram, ResponseValueSelector, ValueProjectionFormat, VerifiedDeltaReceipt,
    VerifierProgram, execute_response, is_privacy_safe_online_response_program,
    is_source_neutral_response_program, response_actor_program_digest,
    response_independent_verifier_program_digest, source_neutral_verifier_for_program,
    verify_response_independently,
};

pub const TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR: u8 = 1;
pub const TRANSFORM_VALUE_STRING: u16 = 0;
pub const TRANSFORM_VALUE_INTEGER: u16 = 1;
pub const TRANSFORM_VALUE_BOOLEAN: u16 = 2;
pub const TRANSFORM_VALUE_IDENTIFIER: u16 = 3;
pub const TRANSFORM_FLAG_CANONICAL_JSON: u16 = 1;
pub const TRANSFORM_ROLE_NONE: u8 = OPERATOR_ROLE_NONE;
const CRYSTALLIZED_REGISTRY_SCHEMA_V3: &str = "nando.crystallized-registry.v3";
const CRYSTALLIZED_REGISTRY_MAX_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct CrystallizationParityReceipt {
    pub future_lineage_sha256: Commitment256,
    pub future_surface_sha256: Commitment256,
    pub future_bundle_sha256: Commitment256,
    pub raw_input_sha256: Commitment256,
    pub extractor_version: u32,
    pub anchors: Box<[RuntimeRoleAnchor]>,
    pub request_text: String,
    pub provider_payload: Value,
    pub expected_response: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRoleAnchor {
    pub local_role: u8,
    pub selector: ResponseValueSelector,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSurfaceEvidence {
    pub bundle: SurfaceFragmentBundle,
    pub request_text: String,
    pub provider_payload: Value,
    pub anchors: Box<[RuntimeRoleAnchor]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundRoleEnvironment {
    surface_sha256: Commitment256,
    local_to_canonical: Box<[u8]>,
    mapping_sha256: Commitment256,
    action_equivalence_sha256: Commitment256,
    phase_fit_fixed: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundCrystallizedOperator {
    environment: BoundRoleEnvironment,
    actor: ResponseProgram,
    verifier: VerifierProgram,
    request_text: String,
    provider_payload: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableParitySeal {
    winner_seal_sha256: Commitment256,
    actor_sha256: Commitment256,
    verifier_sha256: Commitment256,
    binding_receipts_root: Commitment256,
    execution_receipts_root: Commitment256,
    future_lineage_count: u32,
    wrong_accepts: u32,
    seal_sha256: Commitment256,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedCrystallizedOperator {
    operator: CrystallizedOperator,
    parity_seal: ExecutableParitySeal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifiedOperatorRestartBundle {
    page_bytes: Box<[u8]>,
    registry_cbor: Box<[u8]>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CrystallizedRegistryV2 {
    schema: String,
    page_sha256: Commitment256,
    roles: Vec<RestartRole>,
    relations: Vec<RestartRelation>,
    renderer: crate::CollectionOutputRenderer,
    blueprint_sha256: Commitment256,
    candidate_set_sha256: Commitment256,
    support_root_sha256: Commitment256,
    future_evidence_root_sha256: Commitment256,
    future_lineage_root_sha256: Commitment256,
    winner_seal_sha256: Commitment256,
    actor_sha256: String,
    verifier_sha256: String,
    verified_future_lineages: Vec<Commitment256>,
    parity_seal: RestartParitySeal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RestartRole {
    type_class: u8,
    cardinality_class: u8,
    temporal_position: u8,
    constraint_mask: u32,
    neighboring_relation_planes: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RestartRelation {
    plane: u8,
    source_role: u8,
    target_role: u8,
    state: i8,
    phase_re_bits: u64,
    phase_im_bits: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RestartParitySeal {
    winner_seal_sha256: Commitment256,
    actor_sha256: Commitment256,
    verifier_sha256: Commitment256,
    binding_receipts_root: Commitment256,
    execution_receipts_root: Commitment256,
    future_lineage_count: u32,
    wrong_accepts: u32,
    seal_sha256: Commitment256,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CrystallizedOperator {
    page: OperatorPage32,
    relation_program: OperatorCircuit,
    role_graph: RoleGraph,
    transform_program: Box<[TransformOp8]>,
    renderer: crate::CollectionOutputRenderer,
    blueprint_sha256: Commitment256,
    candidate_set_sha256: Commitment256,
    support_root_sha256: Commitment256,
    future_evidence_root_sha256: Commitment256,
    future_lineage_root_sha256: Commitment256,
    winner_seal_sha256: Commitment256,
    actor_sha256: String,
    verifier_sha256: String,
    verified_future_lineages: Box<[Commitment256]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrystallizedOperatorError {
    FutureNotFullPhase,
    FutureHasBlocker,
    MissingWinner,
    WinnerNotFrozen,
    EmptyTransformProgram,
    CyclicComposition,
    InvalidActor,
    UnsupportedTransformProgram,
    NonSourceNeutralActor,
    VerifierBuildFailed,
    ActorVerifierMismatch,
    EmptyFutureWindow,
    DuplicateParityLineage,
    UnknownParityLineage,
    MissingParityReceipt,
    ActorDidNotExecute,
    ActorResponseMismatch,
    RendererMismatch,
    IndependentVerifierRejected,
    DigestFailure,
    InvalidDigest,
    InvalidPage(OperatorPage32Error),
    InvalidWinnerSeal,
    FutureEvidenceMismatch,
    RestartEncode,
    RestartDecode,
    RestartDigestMismatch,
    RuntimeBindingExhausted,
    RuntimeRelationMismatch,
    MissingRuntimeAnchor,
    AmbiguousRuntimeAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrystallizedFeedbackError {
    InvalidPage(OperatorPage32Error),
    InvalidField(CandidateCubeFieldError),
    WrongFieldGeneration,
    WrongReceiptGeneration,
    BackwardWave(BackwardWaveError),
}

impl CrystallizedOperator {
    pub fn crystallize(
        future_window: &FrozenBlueprintFutureWindow,
        winner_receipt: &SealedBlueprintWinnerReceipt,
        future_evidence: &[BlueprintFutureEvidence],
        receipts: &[CrystallizationParityReceipt],
    ) -> Result<VerifiedCrystallizedOperator, CrystallizedOperatorError> {
        let frozen = future_window.frozen();
        if !winner_receipt.matches_frozen(frozen) {
            return Err(CrystallizedOperatorError::InvalidWinnerSeal);
        }
        if !winner_receipt.matches_future_evidence(future_evidence) {
            return Err(CrystallizedOperatorError::FutureEvidenceMismatch);
        }
        let winner_sha256 = *winner_receipt.winner_sha256();
        let blueprint = frozen
            .blueprints()
            .iter()
            .find(|candidate| candidate.fingerprint_sha256() == &winner_sha256)
            .ok_or(CrystallizedOperatorError::WinnerNotFrozen)?;
        if blueprint.transform_program().is_empty() {
            return Err(CrystallizedOperatorError::EmptyTransformProgram);
        }
        if !composition_is_acyclic(blueprint) {
            return Err(CrystallizedOperatorError::CyclicComposition);
        }
        let renderer = infer_future_renderer(blueprint, future_evidence, receipts)?;
        let actor = compile_blueprint_actor(blueprint, &renderer)?;
        actor
            .validate()
            .map_err(|_| CrystallizedOperatorError::InvalidActor)?;
        // The computed law remains source-neutral. A learned static response
        // renderer is a separately sealed surface adapter and must satisfy the
        // bounded Rust privacy contract on every future receipt.
        if !is_source_neutral_response_program(&actor)
            && !is_privacy_safe_online_response_program(&actor)
        {
            return Err(CrystallizedOperatorError::NonSourceNeutralActor);
        }
        let verifier = source_neutral_verifier_for_program(&actor)
            .map_err(|_| CrystallizedOperatorError::VerifierBuildFailed)?;
        if !crate::package::response_program_verifier_matches(&actor, Some(&verifier)) {
            return Err(CrystallizedOperatorError::ActorVerifierMismatch);
        }

        let FutureParityProof {
            lineages: verified_future_lineages,
            binding_receipts,
            execution_receipts,
        } = verify_future_receipts(
            future_window,
            future_evidence,
            blueprint,
            &renderer,
            receipts,
        )?;
        let actor_sha256 = response_actor_program_digest(&actor)
            .map_err(|_| CrystallizedOperatorError::DigestFailure)?;
        let verifier_sha256 = response_independent_verifier_program_digest(&verifier)
            .map_err(|_| CrystallizedOperatorError::DigestFailure)?;
        let page = build_operator_page(
            blueprint,
            frozen.source_generation().saturating_add(1),
            frozen.candidate_set_sha256(),
            frozen.support_lineages_sha256(),
            &verified_future_lineages,
            &renderer,
            &actor_sha256,
            &verifier_sha256,
        )?;

        let parity_seal = build_executable_parity_seal(
            winner_receipt,
            &actor_sha256,
            &verifier_sha256,
            &binding_receipts,
            &execution_receipts,
        )?;
        let operator = Self {
            page,
            relation_program: blueprint.relation_program().clone(),
            role_graph: blueprint.role_graph().clone(),
            transform_program: blueprint.transform_program().into(),
            renderer,
            blueprint_sha256: winner_sha256,
            candidate_set_sha256: *frozen.candidate_set_sha256(),
            support_root_sha256: *winner_receipt.support_root_sha256(),
            future_evidence_root_sha256: *winner_receipt.future_evidence_root_sha256(),
            future_lineage_root_sha256: *winner_receipt.future_lineage_root_sha256(),
            winner_seal_sha256: *winner_receipt.seal_sha256(),
            actor_sha256,
            verifier_sha256,
            verified_future_lineages: verified_future_lineages.into_boxed_slice(),
        };
        Ok(VerifiedCrystallizedOperator {
            operator,
            parity_seal,
        })
    }

    fn feedback_field(
        &self,
        config: OperatorGrokkingConfig,
    ) -> Result<CandidateCubeField, CrystallizedFeedbackError> {
        let generation = self
            .page
            .header()
            .map_err(CrystallizedFeedbackError::InvalidPage)?
            .generation;
        let mut field = CandidateCubeField::new(generation, config)
            .map_err(CrystallizedFeedbackError::InvalidField)?;
        field
            .register_circuit(self.relation_program.clone())
            .map_err(CrystallizedFeedbackError::InvalidField)?;
        Ok(field)
    }

    fn apply_verified_feedback(
        &self,
        field: &mut CandidateCubeField,
        receipt: &VerifiedDeltaReceipt,
    ) -> Result<BackwardWaveUpdate, CrystallizedFeedbackError> {
        let header = self
            .page
            .header()
            .map_err(CrystallizedFeedbackError::InvalidPage)?;
        if field.generation() != header.generation {
            return Err(CrystallizedFeedbackError::WrongFieldGeneration);
        }
        if receipt.generation() != header.generation {
            return Err(CrystallizedFeedbackError::WrongReceiptGeneration);
        }
        BackwardWave::apply(field, header.circuit_fingerprint64, receipt)
            .map_err(CrystallizedFeedbackError::BackwardWave)
    }

    #[must_use]
    const fn page(&self) -> &OperatorPage32 {
        &self.page
    }

    #[must_use]
    const fn relation_program(&self) -> &OperatorCircuit {
        &self.relation_program
    }

    #[must_use]
    const fn blueprint_sha256(&self) -> &Commitment256 {
        &self.blueprint_sha256
    }

    #[must_use]
    const fn candidate_set_sha256(&self) -> &Commitment256 {
        &self.candidate_set_sha256
    }

    #[must_use]
    fn actor_sha256(&self) -> &str {
        &self.actor_sha256
    }

    #[must_use]
    fn verifier_sha256(&self) -> &str {
        &self.verifier_sha256
    }

    #[must_use]
    fn verified_future_lineages(&self) -> &[Commitment256] {
        &self.verified_future_lineages
    }
}

impl VerifiedCrystallizedOperator {
    /// Grounds a restored operator directly against pre-action payload. Every
    /// structurally valid selector is evaluated independently; authority is
    /// withheld unless all successful bindings produce one response class.
    pub fn bind_pre_action(
        &self,
        request_text: &str,
        provider_payload: &Value,
    ) -> Result<BoundCrystallizedOperator, CrystallizedOperatorError> {
        let [transform] = self.operator.transform_program.as_ref() else {
            return Err(CrystallizedOperatorError::UnsupportedTransformProgram);
        };
        let expected_type = transform_value_type(transform.parameter)?;
        let payload = serde_json::to_vec(provider_payload)
            .map_err(|_| CrystallizedOperatorError::RuntimeRelationMismatch)?;
        let lineage_sha256 = digest_parts(
            b"nando.runtime-operator-lineage.v1",
            &[request_text.as_bytes(), &payload],
        );
        let surface_sha256 = digest_parts(
            b"nando.observed-runtime-surface.v1",
            &[request_text.as_bytes(), &payload],
        );
        let mut actions = std::collections::BTreeMap::<String, BoundCrystallizedOperator>::new();
        for selector in crate::collection_synthesis::learned_selector_candidates(provider_payload) {
            if selector_value_type(&selector) != Some(expected_type) {
                continue;
            }
            let evidence = observed_scalar_surface(
                request_text,
                provider_payload,
                selector,
                *transform,
                [0, 1, 2],
                lineage_sha256,
                surface_sha256,
            )?;
            let Ok(bound) = bind_operator_components(
                &self.operator.role_graph,
                &self.operator.relation_program,
                &self.operator.transform_program,
                &self.operator.renderer,
                evidence,
            ) else {
                continue;
            };
            let Ok(response) = bound.execute_verified() else {
                continue;
            };
            actions.entry(response).or_insert(bound);
        }
        match actions.len() {
            0 => Err(CrystallizedOperatorError::MissingRuntimeAnchor),
            1 => Ok(actions.into_values().next().expect("one action class")),
            _ => Err(CrystallizedOperatorError::AmbiguousRuntimeAction),
        }
    }

    pub fn bind(
        &self,
        evidence: RuntimeSurfaceEvidence,
    ) -> Result<BoundCrystallizedOperator, CrystallizedOperatorError> {
        bind_operator_components(
            &self.operator.role_graph,
            &self.operator.relation_program,
            &self.operator.transform_program,
            &self.operator.renderer,
            evidence,
        )
    }

    pub fn feedback_field(
        &self,
        config: OperatorGrokkingConfig,
    ) -> Result<CandidateCubeField, CrystallizedFeedbackError> {
        self.operator.feedback_field(config)
    }

    pub fn apply_verified_feedback(
        &self,
        field: &mut CandidateCubeField,
        receipt: &VerifiedDeltaReceipt,
    ) -> Result<BackwardWaveUpdate, CrystallizedFeedbackError> {
        self.operator.apply_verified_feedback(field, receipt)
    }

    #[must_use]
    pub const fn parity_seal(&self) -> &ExecutableParitySeal {
        &self.parity_seal
    }

    #[must_use]
    pub const fn page(&self) -> &OperatorPage32 {
        self.operator.page()
    }

    #[must_use]
    pub const fn relation_program(&self) -> &OperatorCircuit {
        self.operator.relation_program()
    }

    #[must_use]
    pub const fn blueprint_sha256(&self) -> &Commitment256 {
        self.operator.blueprint_sha256()
    }

    #[must_use]
    pub const fn candidate_set_sha256(&self) -> &Commitment256 {
        self.operator.candidate_set_sha256()
    }

    #[must_use]
    pub const fn support_root_sha256(&self) -> &Commitment256 {
        &self.operator.support_root_sha256
    }

    #[must_use]
    pub const fn future_evidence_root_sha256(&self) -> &Commitment256 {
        &self.operator.future_evidence_root_sha256
    }

    #[must_use]
    pub const fn future_lineage_root_sha256(&self) -> &Commitment256 {
        &self.operator.future_lineage_root_sha256
    }

    #[must_use]
    pub const fn winner_seal_sha256(&self) -> &Commitment256 {
        &self.operator.winner_seal_sha256
    }

    pub fn routing_program(&self) -> Result<ResponseProgram, CrystallizedOperatorError> {
        let [transform] = self.operator.transform_program.as_ref() else {
            return Err(CrystallizedOperatorError::UnsupportedTransformProgram);
        };
        validate_scalar_transform(*transform)?;
        Ok(ResponseProgram::project_selected_value(
            ResponseValueSelector::UniqueScalar {
                value_type: transform_value_type(transform.parameter)?,
            },
            transform_format(transform.flags),
            "completed",
        )
        .with_value_renderer(self.operator.renderer.clone()))
    }

    pub fn routing_verifier(&self) -> Result<VerifierProgram, CrystallizedOperatorError> {
        source_neutral_verifier_for_program(&self.routing_program()?)
            .map_err(|_| CrystallizedOperatorError::VerifierBuildFailed)
    }

    #[must_use]
    pub fn actor_sha256(&self) -> &str {
        self.operator.actor_sha256()
    }

    #[must_use]
    pub fn verifier_sha256(&self) -> &str {
        self.operator.verifier_sha256()
    }

    #[must_use]
    pub fn verified_future_lineages(&self) -> &[Commitment256] {
        self.operator.verified_future_lineages()
    }

    pub fn restart_bundle(
        &self,
    ) -> Result<VerifiedOperatorRestartBundle, CrystallizedOperatorError> {
        let registry = CrystallizedRegistryV2 {
            schema: CRYSTALLIZED_REGISTRY_SCHEMA_V3.to_owned(),
            page_sha256: Sha256::digest(self.page().as_bytes()).into(),
            roles: self
                .operator
                .role_graph
                .canonical_roles()
                .iter()
                .map(|role| RestartRole {
                    type_class: role.type_class(),
                    cardinality_class: role.cardinality_class(),
                    temporal_position: role.temporal_position(),
                    constraint_mask: role.constraint_mask(),
                    neighboring_relation_planes: role.neighboring_relation_planes().to_vec(),
                })
                .collect(),
            relations: self
                .operator
                .relation_program
                .relations()
                .iter()
                .map(|relation| RestartRelation {
                    plane: relation.cell.plane,
                    source_role: relation.cell.source_role,
                    target_role: relation.cell.target_role,
                    state: relation.state as i8,
                    phase_re_bits: relation.phase_anchor.re.to_bits(),
                    phase_im_bits: relation.phase_anchor.im.to_bits(),
                })
                .collect(),
            renderer: self.operator.renderer.clone(),
            blueprint_sha256: self.operator.blueprint_sha256,
            candidate_set_sha256: self.operator.candidate_set_sha256,
            support_root_sha256: self.operator.support_root_sha256,
            future_evidence_root_sha256: self.operator.future_evidence_root_sha256,
            future_lineage_root_sha256: self.operator.future_lineage_root_sha256,
            winner_seal_sha256: self.operator.winner_seal_sha256,
            actor_sha256: self.operator.actor_sha256.clone(),
            verifier_sha256: self.operator.verifier_sha256.clone(),
            verified_future_lineages: self.operator.verified_future_lineages.to_vec(),
            parity_seal: RestartParitySeal::from(&self.parity_seal),
        };
        let registry_cbor =
            serde_cbor::to_vec(&registry).map_err(|_| CrystallizedOperatorError::RestartEncode)?;
        if registry_cbor.len() > CRYSTALLIZED_REGISTRY_MAX_BYTES {
            return Err(CrystallizedOperatorError::RestartEncode);
        }
        Ok(VerifiedOperatorRestartBundle {
            page_bytes: self.page().as_bytes().to_vec().into_boxed_slice(),
            registry_cbor: registry_cbor.into_boxed_slice(),
        })
    }

    pub fn restore(
        page_bytes: &[u8],
        registry_cbor: &[u8],
    ) -> Result<Self, CrystallizedOperatorError> {
        if page_bytes.len() != OPERATOR_PAGE32_BYTES
            || registry_cbor.len() > CRYSTALLIZED_REGISTRY_MAX_BYTES
        {
            return Err(CrystallizedOperatorError::RestartDecode);
        }
        let page = OperatorPage32::from_bytes(page_bytes)
            .map_err(CrystallizedOperatorError::InvalidPage)?;
        let registry: CrystallizedRegistryV2 = serde_cbor::from_slice(registry_cbor)
            .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        if registry.schema != CRYSTALLIZED_REGISTRY_SCHEMA_V3
            || registry.page_sha256 != Commitment256::from(Sha256::digest(page_bytes))
        {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let role_graph = RoleGraph::from_canonical_roles(
            registry
                .roles
                .into_iter()
                .map(|role| {
                    nando_core::wave::StructuralRoleSignature::new(
                        role.type_class,
                        role.cardinality_class,
                        role.temporal_position,
                        role.constraint_mask,
                        role.neighboring_relation_planes,
                    )
                })
                .collect(),
        )
        .ok_or(CrystallizedOperatorError::RestartDecode)?;
        let header = page
            .header()
            .map_err(CrystallizedOperatorError::InvalidPage)?;
        let transform_program = (0..usize::from(header.transform_count))
            .map(|index| {
                page.transform(index)
                    .ok_or(CrystallizedOperatorError::RestartDecode)
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_boxed_slice();
        let relations = registry
            .relations
            .into_iter()
            .map(restart_relation)
            .collect::<Result<Vec<_>, _>>()?;
        let observed_roles = relations
            .iter()
            .flat_map(|relation| [relation.cell.source_role, relation.cell.target_role])
            .collect::<BTreeSet<_>>();
        let virtual_roles = transform_program
            .iter()
            .map(|transform| transform.output)
            .filter(|role| !observed_roles.contains(role))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let relation_program = OperatorCircuit::new_with_virtual_roles(
            role_graph.role_count(),
            relations,
            &virtual_roles,
        )
        .map_err(|_| CrystallizedOperatorError::RestartDecode)?;
        if relation_program.fingerprint64() != header.circuit_fingerprint64
            || usize::from(header.role_count) != role_graph.canonical_roles().len()
        {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let parity_seal = ExecutableParitySeal::try_from(registry.parity_seal)?;
        if parity_seal.winner_seal_sha256 != registry.winner_seal_sha256 {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let actor_digest = decode_sha256(&registry.actor_sha256)?;
        let verifier_digest = decode_sha256(&registry.verifier_sha256)?;
        if actor_digest != parity_seal.actor_sha256
            || verifier_digest != parity_seal.verifier_sha256
            || first_u64(&verifier_digest) != header.verifier_binding_fingerprint64
            || parity_seal.future_lineage_count as usize != registry.verified_future_lineages.len()
            || parity_seal.wrong_accepts != 0
        {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        Ok(Self {
            operator: CrystallizedOperator {
                page,
                relation_program,
                role_graph,
                transform_program,
                renderer: registry.renderer,
                blueprint_sha256: registry.blueprint_sha256,
                candidate_set_sha256: registry.candidate_set_sha256,
                support_root_sha256: registry.support_root_sha256,
                future_evidence_root_sha256: registry.future_evidence_root_sha256,
                future_lineage_root_sha256: registry.future_lineage_root_sha256,
                winner_seal_sha256: registry.winner_seal_sha256,
                actor_sha256: registry.actor_sha256,
                verifier_sha256: registry.verifier_sha256,
                verified_future_lineages: registry.verified_future_lineages.into_boxed_slice(),
            },
            parity_seal,
        })
    }
}

pub(crate) fn observed_scalar_surface(
    request_text: &str,
    provider_payload: &Value,
    selector: ResponseValueSelector,
    transform: TransformOp8,
    semantic_to_local: [u8; 3],
    lineage_sha256: Commitment256,
    surface_sha256: Commitment256,
) -> Result<RuntimeSurfaceEvidence, CrystallizedOperatorError> {
    validate_scalar_transform(transform)?;
    let value_type = transform_value_type(transform.parameter)?;
    if selector_value_type(&selector) != Some(value_type) {
        return Err(CrystallizedOperatorError::MissingRuntimeAnchor);
    }
    let context = semantic_to_local[0];
    let source = semantic_to_local[1];
    let output = semantic_to_local[2];
    let mut roles = vec![StructuralRoleSignature::new(0, 0, 0, 0, Vec::new()); 3];
    roles[usize::from(context)] = StructuralRoleSignature::new(5, 1, 0, 1, vec![0]);
    roles[usize::from(source)] =
        StructuralRoleSignature::new(runtime_value_type_tag(value_type), 1, 1, 2, vec![0]);
    roles[usize::from(output)] =
        StructuralRoleSignature::new(runtime_value_type_tag(value_type), 1, 2, 4, Vec::new());
    let phase_atoms = [
        format!("scalar_type:{}", runtime_value_type_tag(value_type)),
        "cardinality:unique".to_owned(),
        format!("format:{}", transform.flags),
    ];
    let phase = phase_vector_from_atoms(phase_atoms.iter().map(String::as_str), 1)[0];
    let bundle = SurfaceFragmentBundle::new(
        lineage_sha256,
        surface_sha256,
        roles,
        vec![LocalRelationFragment {
            plane: 0,
            source_local_role: context,
            target_local_role: source,
            state: TernaryRelationState::Supported,
            phase_anchor: phase,
        }],
        vec![TypedProgramAtom {
            opcode: transform.opcode,
            output_local_role: output,
            source_a_local_role: source,
            source_b_local_role: OPERATOR_ROLE_NONE,
            parameter: transform.parameter,
            flags: transform.flags,
        }],
    )
    .map_err(|_| CrystallizedOperatorError::RuntimeRelationMismatch)?;
    Ok(RuntimeSurfaceEvidence {
        bundle,
        request_text: request_text.to_owned(),
        provider_payload: provider_payload.clone(),
        anchors: vec![RuntimeRoleAnchor {
            local_role: source,
            selector,
        }]
        .into_boxed_slice(),
    })
}

const fn runtime_value_type_tag(value_type: AtomValueType) -> u8 {
    match value_type {
        AtomValueType::String => 1,
        AtomValueType::Integer => 2,
        AtomValueType::Boolean => 3,
        AtomValueType::Identifier => 4,
        AtomValueType::Collection => 5,
    }
}

impl VerifiedOperatorRestartBundle {
    #[must_use]
    pub fn page_bytes(&self) -> &[u8] {
        &self.page_bytes
    }

    #[must_use]
    pub fn registry_cbor(&self) -> &[u8] {
        &self.registry_cbor
    }
}

impl ExecutableParitySeal {
    #[must_use]
    pub const fn winner_seal_sha256(&self) -> &Commitment256 {
        &self.winner_seal_sha256
    }

    #[must_use]
    pub const fn future_lineage_count(&self) -> u32 {
        self.future_lineage_count
    }

    #[must_use]
    pub const fn wrong_accepts(&self) -> u32 {
        self.wrong_accepts
    }

    #[must_use]
    pub const fn seal_sha256(&self) -> &Commitment256 {
        &self.seal_sha256
    }
}

impl From<&ExecutableParitySeal> for RestartParitySeal {
    fn from(seal: &ExecutableParitySeal) -> Self {
        Self {
            winner_seal_sha256: seal.winner_seal_sha256,
            actor_sha256: seal.actor_sha256,
            verifier_sha256: seal.verifier_sha256,
            binding_receipts_root: seal.binding_receipts_root,
            execution_receipts_root: seal.execution_receipts_root,
            future_lineage_count: seal.future_lineage_count,
            wrong_accepts: seal.wrong_accepts,
            seal_sha256: seal.seal_sha256,
        }
    }
}

impl TryFrom<RestartParitySeal> for ExecutableParitySeal {
    type Error = CrystallizedOperatorError;

    fn try_from(seal: RestartParitySeal) -> Result<Self, Self::Error> {
        let expected = executable_parity_seal_digest(
            &seal.winner_seal_sha256,
            &seal.actor_sha256,
            &seal.verifier_sha256,
            &seal.binding_receipts_root,
            &seal.execution_receipts_root,
            seal.future_lineage_count,
            seal.wrong_accepts,
        );
        if expected != seal.seal_sha256 {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        Ok(Self {
            winner_seal_sha256: seal.winner_seal_sha256,
            actor_sha256: seal.actor_sha256,
            verifier_sha256: seal.verifier_sha256,
            binding_receipts_root: seal.binding_receipts_root,
            execution_receipts_root: seal.execution_receipts_root,
            future_lineage_count: seal.future_lineage_count,
            wrong_accepts: seal.wrong_accepts,
            seal_sha256: seal.seal_sha256,
        })
    }
}

impl BoundCrystallizedOperator {
    pub fn execute_verified(&self) -> Result<String, CrystallizedOperatorError> {
        let execution = execute_response(&self.actor, &self.request_text, &self.provider_payload);
        if execution.status != ResponseExecutionStatus::Executed {
            return Err(CrystallizedOperatorError::ActorDidNotExecute);
        }
        let response = execution
            .response
            .ok_or(CrystallizedOperatorError::ActorDidNotExecute)?;
        verify_response_independently(&self.verifier, &self.provider_payload, &response)
            .map_err(|_| CrystallizedOperatorError::IndependentVerifierRejected)?;
        Ok(response)
    }

    #[must_use]
    pub const fn environment(&self) -> &BoundRoleEnvironment {
        &self.environment
    }

    #[must_use]
    pub const fn actor(&self) -> &ResponseProgram {
        &self.actor
    }

    #[must_use]
    pub const fn verifier(&self) -> &VerifierProgram {
        &self.verifier
    }
}

impl BoundRoleEnvironment {
    #[must_use]
    pub const fn surface_sha256(&self) -> &Commitment256 {
        &self.surface_sha256
    }

    #[must_use]
    pub fn local_to_canonical(&self) -> &[u8] {
        &self.local_to_canonical
    }

    #[must_use]
    pub const fn mapping_sha256(&self) -> &Commitment256 {
        &self.mapping_sha256
    }

    #[must_use]
    pub const fn action_equivalence_sha256(&self) -> &Commitment256 {
        &self.action_equivalence_sha256
    }

    #[must_use]
    pub const fn phase_fit_fixed(&self) -> i64 {
        self.phase_fit_fixed
    }
}

fn bind_operator_components(
    role_graph: &RoleGraph,
    relation_program: &OperatorCircuit,
    transform_program: &[TransformOp8],
    renderer: &crate::CollectionOutputRenderer,
    evidence: RuntimeSurfaceEvidence,
) -> Result<BoundCrystallizedOperator, CrystallizedOperatorError> {
    let report = RuntimeRoleBinder::bind(
        role_graph,
        relation_program,
        &evidence.bundle,
        nando_core::wave::OPERATOR_BLUEPRINT_MAX_ALIGNMENTS,
    );
    if !matches!(report.completion(), SearchCompletion::Complete { .. }) {
        return Err(CrystallizedOperatorError::RuntimeBindingExhausted);
    }
    if report.mappings().is_empty() {
        return Err(CrystallizedOperatorError::RuntimeRelationMismatch);
    }
    let [transform] = transform_program else {
        return Err(CrystallizedOperatorError::UnsupportedTransformProgram);
    };
    let mut actions = std::collections::BTreeMap::<Commitment256, Vec<_>>::new();
    for mapping in report.mappings() {
        let source_local_role = mapping
            .local_role_for(transform.source_a)
            .ok_or(CrystallizedOperatorError::MissingRuntimeAnchor)?;
        let selector = evidence
            .anchors
            .iter()
            .find(|anchor| anchor.local_role == source_local_role)
            .map(|anchor| anchor.selector.clone())
            .ok_or(CrystallizedOperatorError::MissingRuntimeAnchor)?;
        let actor = compile_bound_actor(*transform, selector, renderer)?;
        let response = execute_response(&actor, &evidence.request_text, &evidence.provider_payload)
            .response
            .ok_or(CrystallizedOperatorError::ActorDidNotExecute)?;
        let action_sha256 = digest_parts(
            b"nando.bound-action.v1",
            &[response.as_bytes(), &[transform.output]],
        );
        actions
            .entry(action_sha256)
            .or_default()
            .push((mapping, actor));
    }
    if actions.len() != 1 {
        return Err(CrystallizedOperatorError::AmbiguousRuntimeAction);
    }
    let (action_equivalence_sha256, mut equivalent) =
        actions.into_iter().next().expect("one action class");
    equivalent.sort_by(|left, right| {
        left.0
            .local_to_canonical()
            .cmp(right.0.local_to_canonical())
    });
    let (mapping, actor) = equivalent.remove(0);
    let response = execute_response(&actor, &evidence.request_text, &evidence.provider_payload)
        .response
        .ok_or(CrystallizedOperatorError::ActorDidNotExecute)?;
    let verifier = independently_bind_verifier(
        role_graph,
        relation_program,
        transform_program,
        renderer,
        &evidence.request_text,
        &evidence.provider_payload,
        &response,
    )?;
    let mapping_sha256 = digest_parts(
        b"nando.bound-role-mapping.v1",
        &[mapping.local_to_canonical()],
    );
    Ok(BoundCrystallizedOperator {
        environment: BoundRoleEnvironment {
            surface_sha256: *evidence.bundle.surface_sha256(),
            local_to_canonical: mapping.local_to_canonical().into(),
            mapping_sha256,
            action_equivalence_sha256,
            phase_fit_fixed: mapping.phase_fit_fixed(),
        },
        actor,
        verifier,
        request_text: evidence.request_text,
        provider_payload: evidence.provider_payload,
    })
}

fn independently_bind_verifier(
    role_graph: &RoleGraph,
    relation_program: &OperatorCircuit,
    transform_program: &[TransformOp8],
    renderer: &crate::CollectionOutputRenderer,
    request_text: &str,
    provider_payload: &Value,
    actor_response: &str,
) -> Result<VerifierProgram, CrystallizedOperatorError> {
    let [transform] = transform_program else {
        return Err(CrystallizedOperatorError::UnsupportedTransformProgram);
    };
    let expected_type = transform_value_type(transform.parameter)?;
    let payload = serde_json::to_vec(provider_payload)
        .map_err(|_| CrystallizedOperatorError::RuntimeRelationMismatch)?;
    let lineage_sha256 = digest_parts(
        b"nando.verifier-runtime-lineage.v1",
        &[request_text.as_bytes(), &payload],
    );
    let surface_sha256 = digest_parts(
        b"nando.verifier-observed-surface.v1",
        &[request_text.as_bytes(), &payload],
    );
    let mut verified = std::collections::BTreeMap::<String, VerifierProgram>::new();
    for selector in crate::collection_synthesis::learned_selector_candidates(provider_payload) {
        if selector_value_type(&selector) != Some(expected_type) {
            continue;
        }
        let Ok(observed) = observed_scalar_surface(
            request_text,
            provider_payload,
            selector,
            *transform,
            [0, 1, 2],
            lineage_sha256,
            surface_sha256,
        ) else {
            continue;
        };
        let report = RuntimeRoleBinder::bind(
            role_graph,
            relation_program,
            &observed.bundle,
            nando_core::wave::OPERATOR_BLUEPRINT_MAX_ALIGNMENTS,
        );
        if !matches!(report.completion(), SearchCompletion::Complete { .. }) {
            return Err(CrystallizedOperatorError::RuntimeBindingExhausted);
        }
        for mapping in report.mappings() {
            let Some(source_local_role) = mapping.local_role_for(transform.source_a) else {
                continue;
            };
            let Some(selector) = observed
                .anchors
                .iter()
                .find(|anchor| anchor.local_role == source_local_role)
                .map(|anchor| anchor.selector.clone())
            else {
                continue;
            };
            let verifier = compile_bound_verifier(*transform, selector, renderer)?;
            if verify_response_independently(&verifier, provider_payload, actor_response).is_ok() {
                let digest = response_independent_verifier_program_digest(&verifier)
                    .map_err(|_| CrystallizedOperatorError::IndependentVerifierRejected)?;
                verified.entry(digest).or_insert(verifier);
            }
        }
    }
    verified
        .into_values()
        .next()
        .ok_or(CrystallizedOperatorError::IndependentVerifierRejected)
}

fn compile_bound_actor(
    transform: TransformOp8,
    selector: ResponseValueSelector,
    renderer: &crate::CollectionOutputRenderer,
) -> Result<ResponseProgram, CrystallizedOperatorError> {
    validate_scalar_transform(transform)?;
    let value_type = transform_value_type(transform.parameter)?;
    if selector_value_type(&selector) != Some(value_type) {
        return Err(CrystallizedOperatorError::MissingRuntimeAnchor);
    }
    Ok(ResponseProgram::project_selected_value(
        selector,
        transform_format(transform.flags),
        "completed",
    )
    .with_value_renderer(renderer.clone()))
}

fn compile_bound_verifier(
    transform: TransformOp8,
    selector: ResponseValueSelector,
    renderer: &crate::CollectionOutputRenderer,
) -> Result<VerifierProgram, CrystallizedOperatorError> {
    validate_scalar_transform(transform)?;
    let value_type = transform_value_type(transform.parameter)?;
    if selector_value_type(&selector) != Some(value_type) {
        return Err(CrystallizedOperatorError::MissingRuntimeAnchor);
    }
    Ok(VerifierProgram::ProjectSelectedValue {
        selector,
        format: transform_format(transform.flags),
        renderer: renderer.clone(),
        completion_state: "completed".to_owned(),
        require_unique_value: true,
    })
}

fn selector_value_type(selector: &ResponseValueSelector) -> Option<AtomValueType> {
    match selector {
        ResponseValueSelector::UniqueScalar { value_type }
        | ResponseValueSelector::UniqueTurnScalar { value_type }
        | ResponseValueSelector::ContentLinePrefix { value_type, .. }
        | ResponseValueSelector::JsonField { value_type, .. }
        | ResponseValueSelector::JsonScalarOrdinal { value_type, .. }
        | ResponseValueSelector::UniqueTurnJsonField { value_type, .. }
        | ResponseValueSelector::UniqueActiveTurnJsonField { value_type, .. }
        | ResponseValueSelector::RequestReferencedJsonField { value_type }
        | ResponseValueSelector::TurnOutputLine { value_type, .. }
        | ResponseValueSelector::TurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputLine { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarFromEnd { value_type, .. } => {
            Some(*value_type)
        }
        ResponseValueSelector::CommandOutputBody
        | ResponseValueSelector::RequestLastToken
        | ResponseValueSelector::RequestUniqueLiteral => Some(AtomValueType::String),
    }
}

fn digest_parts(domain: &[u8], parts: &[&[u8]]) -> Commitment256 {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hasher.finalize().into()
}

pub(crate) fn compile_blueprint_actor(
    blueprint: &CandidateOperatorBlueprint,
    renderer: &crate::CollectionOutputRenderer,
) -> Result<ResponseProgram, CrystallizedOperatorError> {
    let [transform] = blueprint.transform_program() else {
        return Err(CrystallizedOperatorError::UnsupportedTransformProgram);
    };
    if !blueprint.composition_dag().edges().is_empty() {
        return Err(CrystallizedOperatorError::UnsupportedTransformProgram);
    }
    validate_scalar_transform(*transform)?;
    let value_type = transform_value_type(transform.parameter)?;
    Ok(ResponseProgram::project_selected_value(
        ResponseValueSelector::UniqueScalar { value_type },
        transform_format(transform.flags),
        "completed",
    )
    .with_value_renderer(renderer.clone()))
}

fn validate_scalar_transform(transform: TransformOp8) -> Result<(), CrystallizedOperatorError> {
    if transform.opcode != TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR
        || transform.source_b != TRANSFORM_ROLE_NONE
        || transform.output == transform.source_a
        || transform.flags & !TRANSFORM_FLAG_CANONICAL_JSON != 0
    {
        return Err(CrystallizedOperatorError::UnsupportedTransformProgram);
    }
    transform_value_type(transform.parameter)?;
    Ok(())
}

fn transform_value_type(parameter: u16) -> Result<AtomValueType, CrystallizedOperatorError> {
    Ok(match parameter {
        TRANSFORM_VALUE_STRING => AtomValueType::String,
        TRANSFORM_VALUE_INTEGER => AtomValueType::Integer,
        TRANSFORM_VALUE_BOOLEAN => AtomValueType::Boolean,
        TRANSFORM_VALUE_IDENTIFIER => AtomValueType::Identifier,
        _ => return Err(CrystallizedOperatorError::UnsupportedTransformProgram),
    })
}

fn transform_format(flags: u16) -> ValueProjectionFormat {
    if flags & TRANSFORM_FLAG_CANONICAL_JSON == 0 {
        ValueProjectionFormat::PlainText
    } else {
        ValueProjectionFormat::CanonicalJson
    }
}

fn infer_future_renderer(
    blueprint: &CandidateOperatorBlueprint,
    future_evidence: &[BlueprintFutureEvidence],
    receipts: &[CrystallizationParityReceipt],
) -> Result<crate::CollectionOutputRenderer, CrystallizedOperatorError> {
    if receipts.is_empty() {
        return Err(CrystallizedOperatorError::MissingParityReceipt);
    }
    let evidence_by_lineage = future_evidence
        .iter()
        .map(|evidence| (*evidence.bundle().lineage_sha256(), evidence))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut renderers = BTreeSet::new();
    for receipt in receipts {
        let evidence = evidence_by_lineage
            .get(&receipt.future_lineage_sha256)
            .ok_or(CrystallizedOperatorError::FutureEvidenceMismatch)?;
        let bound = bind_operator_components(
            blueprint.role_graph(),
            blueprint.relation_program(),
            blueprint.transform_program(),
            &crate::CollectionOutputRenderer::Direct,
            RuntimeSurfaceEvidence {
                bundle: evidence.bundle().clone(),
                request_text: receipt.request_text.clone(),
                provider_payload: receipt.provider_payload.clone(),
                anchors: receipt.anchors.clone(),
            },
        )?;
        let computed = bound.execute_verified()?;
        renderers.insert(infer_exact_renderer(&computed, &receipt.expected_response)?);
    }
    if renderers.len() != 1 {
        return Err(CrystallizedOperatorError::RendererMismatch);
    }
    renderers
        .into_iter()
        .next()
        .ok_or(CrystallizedOperatorError::RendererMismatch)
}

fn infer_exact_renderer(
    computed: &str,
    expected: &str,
) -> Result<crate::CollectionOutputRenderer, CrystallizedOperatorError> {
    if computed == expected {
        return Ok(crate::CollectionOutputRenderer::Direct);
    }
    if computed.is_empty() {
        return Err(CrystallizedOperatorError::RendererMismatch);
    }
    let mut matches = expected.match_indices(computed);
    let Some((offset, _)) = matches.next() else {
        return Err(CrystallizedOperatorError::RendererMismatch);
    };
    if matches.next().is_some() {
        return Err(CrystallizedOperatorError::RendererMismatch);
    }
    Ok(crate::CollectionOutputRenderer::RenderTemplate {
        prefix: expected[..offset].to_owned(),
        suffix: expected[offset + computed.len()..].to_owned(),
    })
}

struct FutureParityProof {
    lineages: Vec<Commitment256>,
    binding_receipts: Vec<Commitment256>,
    execution_receipts: Vec<Commitment256>,
}

fn verify_future_receipts(
    future_window: &FrozenBlueprintFutureWindow,
    future_evidence: &[BlueprintFutureEvidence],
    blueprint: &CandidateOperatorBlueprint,
    renderer: &crate::CollectionOutputRenderer,
    receipts: &[CrystallizationParityReceipt],
) -> Result<FutureParityProof, CrystallizedOperatorError> {
    let expected = future_window.future_lineages_sha256();
    if expected.is_empty() {
        return Err(CrystallizedOperatorError::EmptyFutureWindow);
    }
    let evidence_by_lineage = future_evidence
        .iter()
        .map(|evidence| (*evidence.bundle().lineage_sha256(), evidence))
        .collect::<std::collections::BTreeMap<_, _>>();
    if evidence_by_lineage.len() != expected.len() || evidence_by_lineage.keys().ne(expected.iter())
    {
        return Err(CrystallizedOperatorError::FutureEvidenceMismatch);
    }
    let mut seen = BTreeSet::new();
    let mut binding_receipts = Vec::with_capacity(receipts.len());
    let mut execution_receipts = Vec::with_capacity(receipts.len());
    for receipt in receipts {
        if !expected.contains(&receipt.future_lineage_sha256) {
            return Err(CrystallizedOperatorError::UnknownParityLineage);
        }
        if !seen.insert(receipt.future_lineage_sha256) {
            return Err(CrystallizedOperatorError::DuplicateParityLineage);
        }
        let evidence = evidence_by_lineage
            .get(&receipt.future_lineage_sha256)
            .ok_or(CrystallizedOperatorError::FutureEvidenceMismatch)?;
        if receipt.future_surface_sha256 != *evidence.bundle().surface_sha256()
            || receipt.future_bundle_sha256 != *evidence.bundle_sha256()
            || receipt.raw_input_sha256 != *evidence.raw_input_sha256()
            || receipt.extractor_version != evidence.extractor_version()
        {
            return Err(CrystallizedOperatorError::FutureEvidenceMismatch);
        }
        let bound = bind_operator_components(
            blueprint.role_graph(),
            blueprint.relation_program(),
            blueprint.transform_program(),
            renderer,
            RuntimeSurfaceEvidence {
                bundle: evidence.bundle().clone(),
                request_text: receipt.request_text.clone(),
                provider_payload: receipt.provider_payload.clone(),
                anchors: receipt.anchors.clone(),
            },
        )?;
        let response = bound.execute_verified()?;
        if response != receipt.expected_response {
            return Err(CrystallizedOperatorError::ActorResponseMismatch);
        }
        binding_receipts.push(digest_parts(
            b"nando.crystallization-binding-receipt.v1",
            &[
                bound.environment().surface_sha256(),
                bound.environment().mapping_sha256(),
                bound.environment().action_equivalence_sha256(),
            ],
        ));
        let actor_digest = response_actor_program_digest(bound.actor())
            .map_err(|_| CrystallizedOperatorError::DigestFailure)?;
        let verifier_digest = response_independent_verifier_program_digest(bound.verifier())
            .map_err(|_| CrystallizedOperatorError::DigestFailure)?;
        execution_receipts.push(digest_parts(
            b"nando.crystallization-execution-receipt.v1",
            &[
                &receipt.future_lineage_sha256,
                actor_digest.as_bytes(),
                verifier_digest.as_bytes(),
                response.as_bytes(),
            ],
        ));
    }
    if seen != *expected {
        return Err(CrystallizedOperatorError::MissingParityReceipt);
    }
    Ok(FutureParityProof {
        lineages: seen.into_iter().collect(),
        binding_receipts,
        execution_receipts,
    })
}

fn build_executable_parity_seal(
    winner_receipt: &SealedBlueprintWinnerReceipt,
    actor_sha256: &str,
    verifier_sha256: &str,
    binding_receipts: &[Commitment256],
    execution_receipts: &[Commitment256],
) -> Result<ExecutableParitySeal, CrystallizedOperatorError> {
    if binding_receipts.is_empty() || binding_receipts.len() != execution_receipts.len() {
        return Err(CrystallizedOperatorError::MissingParityReceipt);
    }
    let actor_sha256 = decode_sha256(actor_sha256)?;
    let verifier_sha256 = decode_sha256(verifier_sha256)?;
    let binding_receipts_root =
        commitment_root(b"nando.binding-receipts-root.v1", binding_receipts);
    let execution_receipts_root =
        commitment_root(b"nando.execution-receipts-root.v1", execution_receipts);
    let future_lineage_count = u32::try_from(binding_receipts.len())
        .map_err(|_| CrystallizedOperatorError::DigestFailure)?;
    let wrong_accepts = 0_u32;
    let seal_sha256 = executable_parity_seal_digest(
        winner_receipt.seal_sha256(),
        &actor_sha256,
        &verifier_sha256,
        &binding_receipts_root,
        &execution_receipts_root,
        future_lineage_count,
        wrong_accepts,
    );
    Ok(ExecutableParitySeal {
        winner_seal_sha256: *winner_receipt.seal_sha256(),
        actor_sha256,
        verifier_sha256,
        binding_receipts_root,
        execution_receipts_root,
        future_lineage_count,
        wrong_accepts,
        seal_sha256,
    })
}

#[allow(clippy::too_many_arguments)]
fn executable_parity_seal_digest(
    winner_seal_sha256: &Commitment256,
    actor_sha256: &Commitment256,
    verifier_sha256: &Commitment256,
    binding_receipts_root: &Commitment256,
    execution_receipts_root: &Commitment256,
    future_lineage_count: u32,
    wrong_accepts: u32,
) -> Commitment256 {
    digest_parts(
        b"nando.executable-parity-seal.v1",
        &[
            winner_seal_sha256,
            actor_sha256,
            verifier_sha256,
            binding_receipts_root,
            execution_receipts_root,
            &future_lineage_count.to_le_bytes(),
            &wrong_accepts.to_le_bytes(),
        ],
    )
}

fn restart_relation(
    relation: RestartRelation,
) -> Result<OperatorCircuitRelation, CrystallizedOperatorError> {
    let phase_anchor = PhaseCenterCell {
        re: f64::from_bits(relation.phase_re_bits),
        im: f64::from_bits(relation.phase_im_bits),
    };
    if !phase_anchor.re.is_finite()
        || !phase_anchor.im.is_finite()
        || phase_anchor.re.hypot(phase_anchor.im) <= f64::EPSILON
    {
        return Err(CrystallizedOperatorError::RestartDecode);
    }
    let state = match relation.state {
        -1 => TernaryRelationState::Opposed,
        1 => TernaryRelationState::Supported,
        _ => return Err(CrystallizedOperatorError::RestartDecode),
    };
    Ok(OperatorCircuitRelation {
        cell: OperatorRelationCell {
            plane: relation.plane,
            source_role: relation.source_role,
            target_role: relation.target_role,
        },
        state,
        phase_anchor,
    })
}

fn commitment_root(domain: &[u8], commitments: &[Commitment256]) -> Commitment256 {
    let mut commitments = commitments.to_vec();
    commitments.sort_unstable();
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update((commitments.len() as u32).to_le_bytes());
    for commitment in commitments {
        hasher.update(commitment);
    }
    hasher.finalize().into()
}

fn composition_is_acyclic(blueprint: &CandidateOperatorBlueprint) -> bool {
    let node_count = blueprint.transform_program().len();
    let mut indegree = vec![0_usize; node_count];
    let mut outgoing = vec![Vec::new(); node_count];
    for edge in blueprint.composition_dag().edges() {
        let producer = usize::from(edge.producer_step);
        let consumer = usize::from(edge.consumer_step);
        if producer >= node_count || consumer >= node_count || producer == consumer {
            return false;
        }
        outgoing[producer].push(consumer);
        indegree[consumer] = indegree[consumer].saturating_add(1);
    }
    let mut ready = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect::<Vec<_>>();
    let mut visited = 0_usize;
    while let Some(node) = ready.pop() {
        visited = visited.saturating_add(1);
        for &consumer in &outgoing[node] {
            indegree[consumer] = indegree[consumer].saturating_sub(1);
            if indegree[consumer] == 0 {
                ready.push(consumer);
            }
        }
    }
    visited == node_count
}

#[allow(clippy::too_many_arguments)]
fn build_operator_page(
    blueprint: &CandidateOperatorBlueprint,
    generation: u64,
    candidate_set_sha256: &Commitment256,
    support_lineages: &[Commitment256],
    future_lineages: &[Commitment256],
    output_renderer: &crate::CollectionOutputRenderer,
    actor_sha256: &str,
    verifier_sha256: &str,
) -> Result<OperatorPage32, CrystallizedOperatorError> {
    let mut cube = TernaryOperatorCube32::default();
    let mut phase_profile = [0_u8; OPERATOR_PAGE32_PHASE_BYTES];
    let mut plane_count = 0_u8;
    for (index, relation) in blueprint.relation_program().relations().iter().enumerate() {
        cube.set(
            relation.cell.plane,
            relation.cell.source_role,
            relation.cell.target_role,
            relation.state,
        )
        .map_err(CrystallizedOperatorError::InvalidPage)?;
        plane_count = plane_count.max(relation.cell.plane.saturating_add(1));
        let offset = index * 4;
        let re = quantize_phase(relation.phase_anchor.re).to_le_bytes();
        let im = quantize_phase(relation.phase_anchor.im).to_le_bytes();
        phase_profile[offset..offset + 2].copy_from_slice(&re);
        phase_profile[offset + 2..offset + 4].copy_from_slice(&im);
    }

    let roles = blueprint
        .role_graph()
        .canonical_roles()
        .iter()
        .map(|role| {
            let signature = structural_role_commitment(role);
            StructuralRole16 {
                type_class: role.type_class(),
                cardinality_class: role.cardinality_class(),
                temporal_class: role.temporal_position(),
                relation_flags: role
                    .neighboring_relation_planes()
                    .iter()
                    .filter(|plane| **plane < 8)
                    .fold(0_u8, |flags, plane| flags | (1_u8 << plane)),
                constraint_mask: role.constraint_mask(),
                role_signature_hash: u32::from_le_bytes([
                    signature[0],
                    signature[1],
                    signature[2],
                    signature[3],
                ]),
                ..StructuralRole16::default()
            }
        })
        .collect::<Vec<_>>();

    let mut composition = [0_u8; OPERATOR_PAGE32_COMPOSITION_BYTES];
    for (index, edge) in blueprint.composition_dag().edges().iter().enumerate() {
        let offset = index * 2;
        if offset + 1 >= composition.len() {
            return Err(CrystallizedOperatorError::InvalidPage(
                OperatorPage32Error::InvalidCompositionCount,
            ));
        }
        composition[offset] = edge.producer_step;
        composition[offset + 1] = edge.consumer_step;
    }

    let actor_digest = decode_sha256(actor_sha256)?;
    let verifier_digest = decode_sha256(verifier_sha256)?;
    let renderer_digest = renderer_commitment(output_renderer)?;
    let execution_digest = digest_parts(
        b"nando.crystallized-execution-programs.v1",
        &[&actor_digest, &verifier_digest],
    );
    let mut renderer = [0_u8; OPERATOR_PAGE32_RENDERER_BYTES];
    renderer[..32].copy_from_slice(&renderer_digest);
    renderer[32..64].copy_from_slice(blueprint.fingerprint_sha256());
    renderer[64..96].copy_from_slice(candidate_set_sha256);
    renderer[96..128].copy_from_slice(&execution_digest);

    let proof_lineage = lineage_commitment(support_lineages, future_lineages);
    let role_commitment = roles_commitment(&roles);
    OperatorPage32::build(
        OperatorPage32Metadata {
            generation,
            circuit_fingerprint64: blueprint.relation_program().fingerprint64(),
            verifier_binding_fingerprint64: first_u64(&verifier_digest),
            proof_lineage_fingerprint64: first_u64(&proof_lineage),
            role_signature_fingerprint64: first_u64(&role_commitment),
            relation_plane_count: plane_count,
            composition_node_count: blueprint.composition_dag().edges().len() as u8,
            renderer_instruction_count: 4,
            flags: 0,
        },
        &phase_profile,
        &roles,
        &cube,
        blueprint.transform_program(),
        &composition,
        &renderer,
    )
    .map_err(CrystallizedOperatorError::InvalidPage)
}

fn renderer_commitment(
    renderer: &crate::CollectionOutputRenderer,
) -> Result<Commitment256, CrystallizedOperatorError> {
    let bytes =
        serde_cbor::to_vec(renderer).map_err(|_| CrystallizedOperatorError::DigestFailure)?;
    Ok(digest_parts(b"nando.crystallized-renderer.v1", &[&bytes]))
}

fn quantize_phase(value: f64) -> i16 {
    (value.clamp(-1.0, 1.0) * f64::from(i16::MAX)).round() as i16
}

fn structural_role_commitment(role: &nando_core::wave::StructuralRoleSignature) -> Commitment256 {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.crystallized-role.v1");
    hasher.update([
        role.type_class(),
        role.cardinality_class(),
        role.temporal_position(),
    ]);
    hasher.update(role.constraint_mask().to_le_bytes());
    hasher.update(role.neighboring_relation_planes());
    hasher.finalize().into()
}

fn lineage_commitment(support: &[Commitment256], future: &[Commitment256]) -> Commitment256 {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.crystallized-lineage.v1");
    for lineage in support.iter().chain(future) {
        hasher.update(lineage);
    }
    hasher.finalize().into()
}

fn roles_commitment(roles: &[StructuralRole16]) -> Commitment256 {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.crystallized-roles.v1");
    for role in roles {
        hasher.update(role.encode());
    }
    hasher.finalize().into()
}

fn first_u64(digest: &Commitment256) -> u64 {
    u64::from_le_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ])
}

fn decode_sha256(value: &str) -> Result<Commitment256, CrystallizedOperatorError> {
    if value.len() != 64 {
        return Err(CrystallizedOperatorError::InvalidDigest);
    }
    let mut output = [0_u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| CrystallizedOperatorError::InvalidDigest)?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use nando_core::wave::{
        BlueprintBeamConfig, BlueprintFutureEvaluator, BlueprintFutureEvidence,
        BlueprintPhaseControl, BlueprintSynthesisReport, BoundedCircuitBeam, BoundedRoleAligner,
        LocalRelationFragment, OperatorGrokkingConfig, PhaseCenterCell, RoleAlignmentConfig,
        StructuralRoleSignature, SurfaceFragmentBundle, TernaryRelationState, TypedProgramAtom,
    };
    use serde_json::json;

    use crate::{
        TYPED_EXECUTION_STAGE_RECEIPT_SCHEMA_V1, TypedExecutionStage, TypedExecutionStageReceipt,
        VerifiedDeltaOutcome, VerifiedDeltaRelation, VerifiedDeltaRelationState,
    };

    use super::*;
    fn digest(byte: u8) -> Commitment256 {
        [byte; 32]
    }

    fn bundle(lineage: u8, phase: PhaseCenterCell) -> SurfaceFragmentBundle {
        bundle_with_surface(lineage, lineage.saturating_add(20), phase)
    }

    fn bundle_with_surface(
        lineage: u8,
        surface: u8,
        _phase: PhaseCenterCell,
    ) -> SurfaceFragmentBundle {
        let phase_atoms = ["scalar_type:2", "cardinality:unique", "format:0"];
        let phase = phase_vector_from_atoms(phase_atoms, 1)[0];
        SurfaceFragmentBundle::new(
            digest(lineage),
            digest(surface),
            vec![
                StructuralRoleSignature::new(5, 1, 0, 1, vec![0]),
                StructuralRoleSignature::new(2, 1, 1, 2, vec![0]),
                StructuralRoleSignature::new(2, 1, 2, 4, Vec::new()),
            ],
            vec![LocalRelationFragment {
                plane: 0,
                source_local_role: 0,
                target_local_role: 1,
                state: TernaryRelationState::Supported,
                phase_anchor: phase,
            }],
            vec![TypedProgramAtom {
                opcode: TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR,
                output_local_role: 2,
                source_a_local_role: 1,
                source_b_local_role: TRANSFORM_ROLE_NONE,
                parameter: TRANSFORM_VALUE_INTEGER,
                flags: 0,
            }],
        )
        .expect("valid bundle")
    }

    fn frozen_blueprints() -> (
        FrozenBlueprintFutureWindow,
        BlueprintSynthesisReport,
        SurfaceFragmentBundle,
    ) {
        let support = vec![
            bundle(1, PhaseCenterCell { re: 1.0, im: 0.0 }),
            bundle(2, PhaseCenterCell { re: 1.0, im: 0.0 }),
        ];
        let alignments = BoundedRoleAligner::align(&support, RoleAlignmentConfig::default());
        let synthesis =
            BoundedCircuitBeam::synthesize(&support, &alignments, BlueprintBeamConfig::default());
        let frozen = nando_core::wave::FrozenOperatorBlueprintSet::freeze(
            7,
            &support,
            BlueprintBeamConfig::default(),
            &synthesis,
        )
        .expect("complete frozen set");
        let future_bundle = bundle(3, PhaseCenterCell { re: 1.0, im: 0.0 });
        let mut future = frozen.future_window();
        future
            .admit_lineage(&future_bundle)
            .expect("independent future lineage");
        (future, synthesis, future_bundle)
    }

    #[test]
    fn winner_crystallizes_into_page_and_bound_verified_actor() {
        let (future, synthesis, future_bundle) = frozen_blueprints();
        let winner = *synthesis.blueprints[0].fingerprint_sha256();
        let evidence = BlueprintFutureEvidence::new(digest(40), 1, future_bundle.clone())
            .expect("valid future evidence");
        let evidence_set = vec![evidence.clone()];
        let sealed = BlueprintFutureEvaluator::evaluate_and_seal(
            future.frozen(),
            &evidence_set,
            Default::default(),
            BlueprintPhaseControl::Full,
        );
        let winner_receipt = sealed.winner_receipt().expect("winner seal");
        assert_eq!(winner_receipt.winner_sha256(), &winner);
        let payload = json!({
            "input": [{"type":"function_call_output", "output":"{\"total\":7}"}]
        });
        let receipt = CrystallizationParityReceipt {
            future_lineage_sha256: *future_bundle.lineage_sha256(),
            future_surface_sha256: *future_bundle.surface_sha256(),
            future_bundle_sha256: *evidence.bundle_sha256(),
            raw_input_sha256: *evidence.raw_input_sha256(),
            extractor_version: evidence.extractor_version(),
            anchors: vec![RuntimeRoleAnchor {
                local_role: 1,
                selector: ResponseValueSelector::JsonField {
                    field: "total".to_owned(),
                    value_type: AtomValueType::Integer,
                },
            }]
            .into_boxed_slice(),
            request_text: String::new(),
            provider_payload: payload.clone(),
            expected_response: "7".to_owned(),
        };

        let operator = CrystallizedOperator::crystallize(
            &future,
            winner_receipt,
            &evidence_set,
            std::slice::from_ref(&receipt),
        )
        .expect("verified crystallized operator");

        assert_eq!(operator.blueprint_sha256(), &winner);
        assert_eq!(operator.verified_future_lineages(), &[digest(3)]);
        assert_eq!(operator.page().as_bytes().len(), 4_032);
        assert_eq!(operator.parity_seal().future_lineage_count(), 1);
        assert_eq!(operator.parity_seal().wrong_accepts(), 0);
        assert_eq!(
            operator.parity_seal().winner_seal_sha256(),
            winner_receipt.seal_sha256()
        );
        let restart_bundle = operator.restart_bundle().expect("bounded restart bundle");
        assert_eq!(restart_bundle.page_bytes().len(), OPERATOR_PAGE32_BYTES);
        assert!(restart_bundle.registry_cbor().len() < CRYSTALLIZED_REGISTRY_MAX_BYTES);
        let restored = VerifiedCrystallizedOperator::restore(
            restart_bundle.page_bytes(),
            restart_bundle.registry_cbor(),
        )
        .expect("verified operator restart");
        assert_eq!(restored.page().as_bytes(), operator.page().as_bytes());
        assert_eq!(restored.parity_seal(), operator.parity_seal());
        let bound = operator
            .bind(RuntimeSurfaceEvidence {
                bundle: future_bundle.clone(),
                request_text: String::new(),
                provider_payload: payload.clone(),
                anchors: vec![RuntimeRoleAnchor {
                    local_role: 1,
                    selector: ResponseValueSelector::JsonField {
                        field: "total".to_owned(),
                        value_type: AtomValueType::Integer,
                    },
                }]
                .into_boxed_slice(),
            })
            .expect("runtime role binding");
        assert_eq!(bound.execute_verified().as_deref(), Ok("7"));
        let restored_bound = restored
            .bind(RuntimeSurfaceEvidence {
                bundle: future_bundle,
                request_text: String::new(),
                provider_payload: payload.clone(),
                anchors: vec![RuntimeRoleAnchor {
                    local_role: 1,
                    selector: ResponseValueSelector::JsonField {
                        field: "total".to_owned(),
                        value_type: AtomValueType::Integer,
                    },
                }]
                .into_boxed_slice(),
            })
            .expect("restored runtime role binding");
        assert_eq!(restored_bound.execute_verified().as_deref(), Ok("7"));
        assert_eq!(restored_bound.environment(), bound.environment());
        let mut corrupted_registry = restart_bundle.registry_cbor().to_vec();
        let last = corrupted_registry.len() - 1;
        corrupted_registry[last] ^= 0x01;
        assert!(
            VerifiedCrystallizedOperator::restore(
                restart_bundle.page_bytes(),
                &corrupted_registry,
            )
            .is_err()
        );
        let page_before = operator.page().as_bytes().to_vec();
        let mut feedback = operator
            .feedback_field(OperatorGrokkingConfig::default())
            .expect("immutable next-generation accumulator");
        assert_eq!(
            feedback.generation(),
            operator.page().header().expect("header").generation
        );
        assert_eq!(feedback.circuits(), &[operator.relation_program().clone()]);
        let header = operator.page().header().expect("operator page header");
        let relation = operator.relation_program().relations()[0];
        let positive = verified_delta(
            header.generation,
            header.circuit_fingerprint64,
            VerifiedDeltaOutcome::Positive,
            vec![VerifiedDeltaRelation {
                plane: relation.cell.plane,
                source_role: relation.cell.source_role,
                target_role: relation.cell.target_role,
                state: match relation.state {
                    TernaryRelationState::Opposed => VerifiedDeltaRelationState::Opposed,
                    TernaryRelationState::Unresolved => VerifiedDeltaRelationState::Unresolved,
                    TernaryRelationState::Supported => VerifiedDeltaRelationState::Supported,
                },
                phase_re_micro: (relation.phase_anchor.re * 1_000_000.0).round() as i32,
                phase_im_micro: (relation.phase_anchor.im * 1_000_000.0).round() as i32,
            }],
            50,
        );
        assert_eq!(
            operator.apply_verified_feedback(&mut feedback, &positive),
            Ok(BackwardWaveUpdate::Applied)
        );
        assert_eq!(operator.page().as_bytes(), page_before.as_slice());
        let field_before_censored = feedback.clone();
        let censored = verified_delta(
            header.generation,
            header.circuit_fingerprint64,
            VerifiedDeltaOutcome::CensoredUnknown,
            Vec::new(),
            51,
        );
        assert_eq!(
            operator.apply_verified_feedback(&mut feedback, &censored),
            Ok(BackwardWaveUpdate::CensoredIgnored)
        );
        assert_eq!(feedback, field_before_censored);
        assert_eq!(
            CrystallizedOperator::crystallize(&future, winner_receipt, &evidence_set, &[]),
            Err(CrystallizedOperatorError::MissingParityReceipt)
        );
        let tampered_evidence = vec![
            BlueprintFutureEvidence::new(
                digest(41),
                1,
                bundle(3, PhaseCenterCell { re: 1.0, im: 0.0 }),
            )
            .expect("well-formed tampered evidence"),
        ];
        assert_eq!(
            CrystallizedOperator::crystallize(
                &future,
                winner_receipt,
                &tampered_evidence,
                std::slice::from_ref(&receipt),
            ),
            Err(CrystallizedOperatorError::FutureEvidenceMismatch)
        );
        let tampered_surface = vec![
            BlueprintFutureEvidence::new(
                digest(40),
                1,
                bundle_with_surface(3, 99, PhaseCenterCell { re: 1.0, im: 0.0 }),
            )
            .expect("well-formed surface substitution"),
        ];
        assert_eq!(
            CrystallizedOperator::crystallize(
                &future,
                winner_receipt,
                &tampered_surface,
                std::slice::from_ref(&receipt),
            ),
            Err(CrystallizedOperatorError::FutureEvidenceMismatch)
        );
        let mut tampered_receipt = receipt.clone();
        tampered_receipt.future_surface_sha256 = digest(99);
        assert_eq!(
            CrystallizedOperator::crystallize(
                &future,
                winner_receipt,
                &evidence_set,
                std::slice::from_ref(&tampered_receipt),
            ),
            Err(CrystallizedOperatorError::FutureEvidenceMismatch)
        );
    }

    fn verified_delta(
        generation: u64,
        operator_fingerprint64: u64,
        outcome: VerifiedDeltaOutcome,
        relations: Vec<VerifiedDeltaRelation>,
        identity: u8,
    ) -> VerifiedDeltaReceipt {
        let predicted = "4".repeat(64);
        let observed = if outcome == VerifiedDeltaOutcome::Positive {
            predicted.clone()
        } else {
            "5".repeat(64)
        };
        let trace = TypedExecutionStage::ALL
            .into_iter()
            .map(|stage| TypedExecutionStageReceipt {
                schema: TYPED_EXECUTION_STAGE_RECEIPT_SCHEMA_V1.to_owned(),
                stage,
                generation,
                operator_fingerprint64,
                surface_id_sha256: format!("{identity:064x}"),
                session_id_sha256: format!("{:064x}", identity.saturating_add(1)),
                input_relation_sha256: "3".repeat(64),
                predicted_relation_sha256: predicted.clone(),
                observed_relation_sha256: observed.clone(),
                stage_payload_sha256: format!("{:064x}", stage as u8 + 10),
                independently_observed: stage == TypedExecutionStage::IndependentVerifier,
                accepted: true,
            })
            .collect();
        VerifiedDeltaReceipt::from_typed_trace(trace, outcome, relations)
            .expect("valid typed feedback receipt")
    }
}
