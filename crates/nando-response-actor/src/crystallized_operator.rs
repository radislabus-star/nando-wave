use std::collections::BTreeSet;

use nando_core::wave::{
    BlueprintFutureEvidence, CandidateCubeField, CandidateCubeFieldError,
    CandidateOperatorBlueprint, Commitment256, FrozenBlueprintFutureWindow,
    OPERATOR_PAGE32_COMPOSITION_BYTES, OPERATOR_PAGE32_PHASE_BYTES, OperatorCircuit,
    OperatorGrokkingConfig, OperatorPage32, OperatorPage32Error, OperatorPage32Metadata, RoleGraph,
    SealedBlueprintWinnerReceipt, StructuralRole16, StructuralRoleSignature, TernaryOperatorCube32,
    TransformOp8,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    AtomValueType, BackwardWave, BackwardWaveError, BackwardWaveUpdate, ResponseProgram,
    ResponseValueSelector, ValueProjectionFormat, VerifiedDeltaReceipt, VerifierProgram,
    is_privacy_safe_online_response_program, is_source_neutral_response_program,
    response_actor_program_digest, response_independent_verifier_program_digest,
    source_neutral_verifier_for_program, verify_response_independently_with_request,
};

pub use nando_operator_kernel::{
    TRANSFORM_FLAG_CANONICAL_JSON, TRANSFORM_OPCODE_COUNT_COLLECTION,
    TRANSFORM_OPCODE_FILTER_REQUEST_VALUE, TRANSFORM_OPCODE_PROJECT_STATUS,
    TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR, TRANSFORM_ROLE_NONE, TRANSFORM_STATUS_ZERO_IS_OK,
    TRANSFORM_STATUS_ZERO_IS_PASS, TRANSFORM_STATUS_ZERO_IS_SUCCESS, TRANSFORM_STATUS_ZERO_IS_TRUE,
    TRANSFORM_VALUE_BOOLEAN, TRANSFORM_VALUE_COLLECTION, TRANSFORM_VALUE_IDENTIFIER,
    TRANSFORM_VALUE_INTEGER, TRANSFORM_VALUE_STRING,
};
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

pub use nando_operator_runtime::{BoundRoleEnvironment, RuntimeRoleAnchor, RuntimeSurfaceEvidence};

#[derive(Clone, Debug, PartialEq)]
pub struct BoundCrystallizedOperator {
    runtime: nando_operator_runtime::BoundRuntimeOperator,
    verifier: VerifierProgram,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableParitySeal {
    winner_seal_sha256: Commitment256,
    actor_sha256: Commitment256,
    verifier_sha256: Commitment256,
    binding_receipts_root: Commitment256,
    execution_receipts_root: Commitment256,
    future_evidence_count: u32,
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

#[derive(Clone, Debug, PartialEq)]
pub struct CrystallizedOperator {
    runtime_artifact: nando_operator_runtime::RuntimeOperatorArtifact,
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
    ActorContractMismatch,
    VerifierContractMismatch,
    EmptyFutureWindow,
    DuplicateParityLineage,
    DuplicateParityEvidence,
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
    RuntimeOperandArityMismatch,
    RuntimeOperandTypeMismatch,
    AmbiguousRuntimeAction,
    OperatorVmRejected,
    ProgramCompileFailed,
}

impl From<nando_operator_runtime::RuntimeBindingError> for CrystallizedOperatorError {
    fn from(error: nando_operator_runtime::RuntimeBindingError) -> Self {
        use nando_operator_runtime::RuntimeBindingError;

        match error {
            RuntimeBindingError::InvalidActor => Self::InvalidActor,
            RuntimeBindingError::UnsupportedTransformProgram => Self::UnsupportedTransformProgram,
            RuntimeBindingError::BindingExhausted => Self::RuntimeBindingExhausted,
            RuntimeBindingError::RelationMismatch => Self::RuntimeRelationMismatch,
            RuntimeBindingError::MissingAnchor => Self::MissingRuntimeAnchor,
            RuntimeBindingError::OperandArityMismatch => Self::RuntimeOperandArityMismatch,
            RuntimeBindingError::OperandTypeMismatch => Self::RuntimeOperandTypeMismatch,
            RuntimeBindingError::AmbiguousAction => Self::AmbiguousRuntimeAction,
            RuntimeBindingError::ActorDidNotExecute => Self::ActorDidNotExecute,
            RuntimeBindingError::ActorVmMismatch => Self::ActorVerifierMismatch,
            RuntimeBindingError::VmRejected => Self::OperatorVmRejected,
            RuntimeBindingError::DigestFailure => Self::DigestFailure,
            RuntimeBindingError::ExpectedActionMissing => Self::IndependentVerifierRejected,
        }
    }
}

impl From<nando_operator_runtime::RuntimeArtifactRestartError> for CrystallizedOperatorError {
    fn from(error: nando_operator_runtime::RuntimeArtifactRestartError) -> Self {
        use nando_operator_runtime::RuntimeArtifactRestartError;

        match error {
            RuntimeArtifactRestartError::Encode => Self::RestartEncode,
            RuntimeArtifactRestartError::Decode => Self::RestartDecode,
            RuntimeArtifactRestartError::DigestMismatch => Self::RestartDigestMismatch,
            RuntimeArtifactRestartError::InvalidPage(error) => Self::InvalidPage(error),
        }
    }
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
        Self::crystallize_internal(
            future_window,
            winner_receipt,
            future_evidence,
            receipts,
            None,
        )
    }

    pub fn crystallize_with_actor_template(
        future_window: &FrozenBlueprintFutureWindow,
        winner_receipt: &SealedBlueprintWinnerReceipt,
        future_evidence: &[BlueprintFutureEvidence],
        receipts: &[CrystallizationParityReceipt],
        actor_template: ResponseProgram,
    ) -> Result<VerifiedCrystallizedOperator, CrystallizedOperatorError> {
        Self::crystallize_internal(
            future_window,
            winner_receipt,
            future_evidence,
            receipts,
            Some(actor_template),
        )
    }

    fn crystallize_internal(
        future_window: &FrozenBlueprintFutureWindow,
        winner_receipt: &SealedBlueprintWinnerReceipt,
        future_evidence: &[BlueprintFutureEvidence],
        receipts: &[CrystallizationParityReceipt],
        actor_template: Option<ResponseProgram>,
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
        let actor_was_external = actor_template.is_some();
        let (renderer, actor) = if let Some(actor) = actor_template {
            let renderer = actor_renderer_contract(&actor)?;
            (renderer, actor)
        } else {
            let renderer = infer_future_renderer(blueprint, future_evidence, receipts)?;
            let actor = compile_blueprint_actor(blueprint, &renderer)?;
            (renderer, actor)
        };
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
        } = verify_future_receipts(future_window, future_evidence, blueprint, &actor, receipts)?;
        let actor_sha256 = response_actor_program_digest(&actor)
            .map_err(|_| CrystallizedOperatorError::DigestFailure)?;
        let verifier_sha256 = response_independent_verifier_program_digest(&verifier)
            .map_err(|_| CrystallizedOperatorError::DigestFailure)?;
        // An externally materialized actor is data, never authority. The
        // phase-selected blueprint owns both executable commitments before
        // freeze; crystallization accepts only byte-identical implementations.
        if actor_was_external {
            if !digest_matches_commitment(
                &actor_sha256,
                blueprint.renderer_hypothesis().commitment_sha256(),
            ) {
                return Err(CrystallizedOperatorError::ActorContractMismatch);
            }
            if !digest_matches_commitment(
                &verifier_sha256,
                blueprint.verifier_contract().commitment_sha256(),
            ) {
                return Err(CrystallizedOperatorError::VerifierContractMismatch);
            }
        }
        let uses_typed_actor_renderer = matches!(
            &actor.operation,
            crate::ResponseOperation::FunctionCallFromRoles { .. }
                | crate::ResponseOperation::CustomToolCallFromRoles { .. }
        );
        let page = build_operator_page(
            blueprint,
            frozen.source_generation().saturating_add(1),
            frozen.candidate_set_sha256(),
            frozen.support_lineages_sha256(),
            &verified_future_lineages,
            &renderer,
            uses_typed_actor_renderer,
            &actor_sha256,
            &verifier_sha256,
        )?;

        let parity_seal = build_executable_parity_seal(
            winner_receipt,
            &actor_sha256,
            &verifier_sha256,
            &binding_receipts,
            &execution_receipts,
            verified_future_lineages.len(),
        )?;
        let operator = Self {
            runtime_artifact: nando_operator_runtime::RuntimeOperatorArtifact::new(
                page,
                blueprint.relation_program().clone(),
                blueprint.role_graph().clone(),
                blueprint.transform_program().into(),
                renderer,
                actor,
            ),
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
            .runtime_artifact
            .page()
            .header()
            .map_err(CrystallizedFeedbackError::InvalidPage)?
            .generation;
        let mut field = CandidateCubeField::new(generation, config)
            .map_err(CrystallizedFeedbackError::InvalidField)?;
        field
            .register_circuit(self.runtime_artifact.relation_program().clone())
            .map_err(CrystallizedFeedbackError::InvalidField)?;
        Ok(field)
    }

    fn apply_verified_feedback(
        &self,
        field: &mut CandidateCubeField,
        receipt: &VerifiedDeltaReceipt,
    ) -> Result<BackwardWaveUpdate, CrystallizedFeedbackError> {
        let header = self
            .runtime_artifact
            .page()
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
        self.runtime_artifact.page()
    }

    #[must_use]
    const fn relation_program(&self) -> &OperatorCircuit {
        self.runtime_artifact.relation_program()
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
    pub(crate) fn crystallize_durable_program(
        actor: ResponseProgram,
        verifier: VerifierProgram,
        proof: DurableProgramCrystallizationProof,
    ) -> Result<Self, CrystallizedOperatorError> {
        actor
            .validate()
            .map_err(|_| CrystallizedOperatorError::InvalidActor)?;
        if !is_source_neutral_response_program(&actor)
            && !is_privacy_safe_online_response_program(&actor)
        {
            return Err(CrystallizedOperatorError::NonSourceNeutralActor);
        }
        let rebuilt_verifier = source_neutral_verifier_for_program(&actor)
            .map_err(|_| CrystallizedOperatorError::VerifierBuildFailed)?;
        if rebuilt_verifier != verifier
            || !crate::package::response_program_verifier_matches(&actor, Some(&verifier))
        {
            return Err(CrystallizedOperatorError::ActorVerifierMismatch);
        }
        if proof.support_lineages.is_empty()
            || proof.future_lineages.is_empty()
            || proof.binding_receipts.is_empty()
            || proof.binding_receipts.len() != proof.execution_receipts.len()
        {
            return Err(CrystallizedOperatorError::MissingParityReceipt);
        }
        let support = proof
            .support_lineages
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let future = proof
            .future_lineages
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if support.len() != proof.support_lineages.len()
            || future.len() != proof.future_lineages.len()
            || !support.is_disjoint(&future)
        {
            return Err(CrystallizedOperatorError::DuplicateParityLineage);
        }

        let compiled = nando_operator_runtime::compile_runtime_program(&actor)
            .map_err(|_| CrystallizedOperatorError::ProgramCompileFailed)?;
        let actor_sha256 = response_actor_program_digest(&actor)
            .map_err(|_| CrystallizedOperatorError::DigestFailure)?;
        let verifier_sha256 = response_independent_verifier_program_digest(&verifier)
            .map_err(|_| CrystallizedOperatorError::DigestFailure)?;
        let uses_typed_actor_renderer = matches!(
            actor.operation,
            crate::ResponseOperation::FunctionCallFromRoles { .. }
                | crate::ResponseOperation::CustomToolCallFromRoles { .. }
        );
        let page = build_operator_page_from_parts(
            compiled.role_graph(),
            compiled.relation_program(),
            compiled.transform_program(),
            proof.generation,
            &proof.support_lineages,
            &proof.future_lineages,
            compiled.renderer(),
            uses_typed_actor_renderer,
            &verifier_sha256,
        )?;
        let parity_seal = build_executable_parity_seal_from_commitment(
            &proof.winner_seal_sha256,
            &actor_sha256,
            &verifier_sha256,
            &proof.binding_receipts,
            &proof.execution_receipts,
            proof.future_lineages.len(),
        )?;
        let runtime_artifact = nando_operator_runtime::RuntimeOperatorArtifact::new(
            page,
            compiled.relation_program().clone(),
            compiled.role_graph().clone(),
            compiled.transform_program().to_vec().into_boxed_slice(),
            compiled.renderer().clone(),
            actor,
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
                actor_sha256,
                verifier_sha256,
                verified_future_lineages: proof.future_lineages.into_boxed_slice(),
            },
            parity_seal,
        })
    }

    /// Grounds a restored operator directly against pre-action payload. Every
    /// structurally valid selector is evaluated independently; authority is
    /// withheld unless all successful bindings produce one response class.
    pub fn bind_pre_action(
        &self,
        request_text: &str,
        provider_payload: &Value,
    ) -> Result<BoundCrystallizedOperator, CrystallizedOperatorError> {
        let runtime = nando_operator_runtime::bind_pre_action_with_validator(
            self.operator.runtime_artifact.spec(),
            request_text,
            provider_payload,
            |bound, response| {
                independently_bind_verifier(
                    self.operator.runtime_artifact.role_graph(),
                    self.operator.runtime_artifact.relation_program(),
                    self.operator.runtime_artifact.transform_program(),
                    bound.actor(),
                    bound.request_text(),
                    bound.provider_payload(),
                    response,
                )
                .map(|_| ())
            },
        )
        .map_err(|error| match error {
            nando_operator_runtime::ValidatedRuntimeBindingError::Runtime(error) => error.into(),
            nando_operator_runtime::ValidatedRuntimeBindingError::Validation(error) => error,
        })?;
        let response = runtime.execute_unverified()?;
        let verifier = independently_bind_verifier(
            self.operator.runtime_artifact.role_graph(),
            self.operator.runtime_artifact.relation_program(),
            self.operator.runtime_artifact.transform_program(),
            runtime.actor(),
            runtime.request_text(),
            runtime.provider_payload(),
            &response,
        )?;
        Ok(BoundCrystallizedOperator { runtime, verifier })
    }

    pub fn bind(
        &self,
        evidence: RuntimeSurfaceEvidence,
    ) -> Result<BoundCrystallizedOperator, CrystallizedOperatorError> {
        bind_operator_components(
            self.operator.runtime_artifact.role_graph(),
            self.operator.runtime_artifact.relation_program(),
            self.operator.runtime_artifact.transform_program(),
            self.operator.runtime_artifact.actor_template(),
            evidence,
        )
        .map(|bound| bound.with_vm_page(self.operator.page().clone()))
    }

    #[must_use]
    pub fn runtime_route_margin(&self, bound: &BoundCrystallizedOperator) -> i64 {
        let relation_count = self
            .operator
            .runtime_artifact
            .relation_program()
            .relations()
            .len()
            .max(1) as i64;
        bound
            .environment()
            .phase_fit_fixed()
            .saturating_div(relation_count)
            .saturating_add(nando_core::wave::OPERATOR_BLUEPRINT_SCORE_SCALE)
            .saturating_add(1)
            .max(1)
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
        Ok(self.operator.runtime_artifact.actor_template().clone())
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
    pub fn execution_equivalent(&self, other: &Self) -> bool {
        self.operator
            .runtime_artifact
            .execution_equivalent(&other.operator.runtime_artifact)
            && self.operator.actor_sha256 == other.operator.actor_sha256
            && self.operator.verifier_sha256 == other.operator.verifier_sha256
    }

    #[must_use]
    pub fn verified_future_lineages(&self) -> &[Commitment256] {
        self.operator.verified_future_lineages()
    }

    pub fn restart_bundle(
        &self,
    ) -> Result<VerifiedOperatorRestartBundle, CrystallizedOperatorError> {
        let metadata = nando_operator_runtime::RuntimeArtifactRestartMetadata {
            blueprint_sha256: self.operator.blueprint_sha256,
            candidate_set_sha256: self.operator.candidate_set_sha256,
            support_root_sha256: self.operator.support_root_sha256,
            future_evidence_root_sha256: self.operator.future_evidence_root_sha256,
            future_lineage_root_sha256: self.operator.future_lineage_root_sha256,
            winner_seal_sha256: self.operator.winner_seal_sha256,
            actor_sha256: self.operator.actor_sha256.clone(),
            verifier_sha256: self.operator.verifier_sha256.clone(),
            verified_future_lineages: self.operator.verified_future_lineages.to_vec(),
            parity_seal: nando_operator_runtime::RuntimeRestartParitySealData::from(
                &self.parity_seal,
            ),
        };
        let registry_cbor = nando_operator_runtime::encode_runtime_artifact_registry(
            &self.operator.runtime_artifact,
            &metadata,
        )?;
        Ok(VerifiedOperatorRestartBundle {
            page_bytes: self.page().as_bytes().to_vec().into_boxed_slice(),
            registry_cbor,
        })
    }

    pub fn restore(
        page_bytes: &[u8],
        registry_cbor: &[u8],
    ) -> Result<Self, CrystallizedOperatorError> {
        let decoded =
            nando_operator_runtime::decode_runtime_artifact_registry(page_bytes, registry_cbor)?;
        let (runtime_artifact, metadata) = decoded.finalize(scalar_actor_from_transform_program)?;
        let parity_seal = ExecutableParitySeal::try_from(&metadata.parity_seal)?;
        if parity_seal.winner_seal_sha256 != metadata.winner_seal_sha256 {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        let actor_digest = decode_sha256(&metadata.actor_sha256)?;
        let verifier_digest = decode_sha256(&metadata.verifier_sha256)?;
        let restored_actor_sha256 =
            response_actor_program_digest(runtime_artifact.actor_template())
                .map_err(|_| CrystallizedOperatorError::RestartDigestMismatch)?;
        let header = runtime_artifact
            .page()
            .header()
            .map_err(CrystallizedOperatorError::InvalidPage)?;
        if actor_digest != parity_seal.actor_sha256
            || restored_actor_sha256 != metadata.actor_sha256
            || verifier_digest != parity_seal.verifier_sha256
            || first_u64(&verifier_digest) != header.verifier_binding_fingerprint64
            || parity_seal.future_lineage_count as usize != metadata.verified_future_lineages.len()
            || parity_seal.future_evidence_count < parity_seal.future_lineage_count
            || parity_seal.wrong_accepts != 0
        {
            return Err(CrystallizedOperatorError::RestartDigestMismatch);
        }
        Ok(Self {
            operator: CrystallizedOperator {
                runtime_artifact,
                blueprint_sha256: metadata.blueprint_sha256,
                candidate_set_sha256: metadata.candidate_set_sha256,
                support_root_sha256: metadata.support_root_sha256,
                future_evidence_root_sha256: metadata.future_evidence_root_sha256,
                future_lineage_root_sha256: metadata.future_lineage_root_sha256,
                winner_seal_sha256: metadata.winner_seal_sha256,
                actor_sha256: metadata.actor_sha256,
                verifier_sha256: metadata.verifier_sha256,
                verified_future_lineages: metadata.verified_future_lineages.into_boxed_slice(),
            },
            parity_seal,
        })
    }
}

fn bind_raw_pre_action_components(
    role_graph: &RoleGraph,
    relation_program: &OperatorCircuit,
    transform_program: &[TransformOp8],
    actor_template: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
) -> Result<BoundCrystallizedOperator, CrystallizedOperatorError> {
    let runtime = nando_operator_runtime::bind_pre_action_with_validator(
        nando_operator_runtime::RuntimeOperatorSpec::new(
            role_graph,
            relation_program,
            transform_program,
            actor_template,
            None,
        ),
        request_text,
        provider_payload,
        |bound, response| {
            independently_bind_verifier(
                role_graph,
                relation_program,
                transform_program,
                bound.actor(),
                bound.request_text(),
                bound.provider_payload(),
                response,
            )
            .map(|_| ())
        },
    )
    .map_err(|error| match error {
        nando_operator_runtime::ValidatedRuntimeBindingError::Runtime(error) => error.into(),
        nando_operator_runtime::ValidatedRuntimeBindingError::Validation(error) => error,
    })?;
    let response = runtime.execute_unverified()?;
    let verifier = independently_bind_verifier(
        role_graph,
        relation_program,
        transform_program,
        runtime.actor(),
        runtime.request_text(),
        runtime.provider_payload(),
        &response,
    )?;
    Ok(BoundCrystallizedOperator { runtime, verifier })
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
    pub const fn future_evidence_count(&self) -> u32 {
        self.future_evidence_count
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

impl From<&ExecutableParitySeal> for nando_operator_runtime::RuntimeRestartParitySealData {
    fn from(seal: &ExecutableParitySeal) -> Self {
        Self {
            winner_seal_sha256: seal.winner_seal_sha256,
            actor_sha256: seal.actor_sha256,
            verifier_sha256: seal.verifier_sha256,
            binding_receipts_root: seal.binding_receipts_root,
            execution_receipts_root: seal.execution_receipts_root,
            future_evidence_count: seal.future_evidence_count,
            future_lineage_count: seal.future_lineage_count,
            wrong_accepts: seal.wrong_accepts,
            seal_sha256: seal.seal_sha256,
        }
    }
}

impl TryFrom<&nando_operator_runtime::RuntimeRestartParitySealData> for ExecutableParitySeal {
    type Error = CrystallizedOperatorError;

    fn try_from(
        seal: &nando_operator_runtime::RuntimeRestartParitySealData,
    ) -> Result<Self, Self::Error> {
        let expected = executable_parity_seal_digest(
            &seal.winner_seal_sha256,
            &seal.actor_sha256,
            &seal.verifier_sha256,
            &seal.binding_receipts_root,
            &seal.execution_receipts_root,
            seal.future_evidence_count,
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
            future_evidence_count: seal.future_evidence_count,
            future_lineage_count: seal.future_lineage_count,
            wrong_accepts: seal.wrong_accepts,
            seal_sha256: seal.seal_sha256,
        })
    }
}

impl BoundCrystallizedOperator {
    pub fn execute_verified(&self) -> Result<String, CrystallizedOperatorError> {
        let response = self.runtime.execute_unverified()?;
        verify_response_independently_with_request(
            &self.verifier,
            self.runtime.request_text(),
            self.runtime.provider_payload(),
            &response,
        )
        .map_err(|_| CrystallizedOperatorError::IndependentVerifierRejected)?;
        Ok(response)
    }

    fn with_vm_page(mut self, page: OperatorPage32) -> Self {
        self.runtime = self.runtime.with_vm_page(page);
        self
    }

    #[must_use]
    pub const fn environment(&self) -> &BoundRoleEnvironment {
        self.runtime.environment()
    }

    #[must_use]
    pub const fn actor(&self) -> &ResponseProgram {
        self.runtime.actor()
    }

    #[must_use]
    pub const fn verifier(&self) -> &VerifierProgram {
        &self.verifier
    }
}

fn bind_operator_components(
    role_graph: &RoleGraph,
    relation_program: &OperatorCircuit,
    transform_program: &[TransformOp8],
    actor_template: &ResponseProgram,
    evidence: RuntimeSurfaceEvidence,
) -> Result<BoundCrystallizedOperator, CrystallizedOperatorError> {
    let runtime = nando_operator_runtime::bind_operator_components(
        role_graph,
        relation_program,
        transform_program,
        actor_template,
        evidence,
    )?;
    let response = runtime.execute_unverified()?;
    let verifier = independently_bind_verifier(
        role_graph,
        relation_program,
        transform_program,
        runtime.actor(),
        runtime.request_text(),
        runtime.provider_payload(),
        &response,
    )?;
    Ok(BoundCrystallizedOperator { runtime, verifier })
}

fn independently_bind_verifier(
    role_graph: &RoleGraph,
    relation_program: &OperatorCircuit,
    transform_program: &[TransformOp8],
    actor_template: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    actor_response: &str,
) -> Result<VerifierProgram, CrystallizedOperatorError> {
    let candidates = nando_operator_runtime::independently_rebound_actor_candidates(
        role_graph,
        relation_program,
        transform_program,
        actor_template,
        request_text,
        provider_payload,
        actor_response,
    )?;
    let independently_bound_actor = candidates
        .first()
        .map(nando_operator_runtime::ReboundActorCandidate::actor)
        .ok_or(CrystallizedOperatorError::IndependentVerifierRejected)?;
    let verifier = source_neutral_verifier_for_program(independently_bound_actor)
        .map_err(|_| CrystallizedOperatorError::VerifierBuildFailed)?;
    verify_response_independently_with_request(
        &verifier,
        request_text,
        provider_payload,
        actor_response,
    )
    .map_err(|_| CrystallizedOperatorError::IndependentVerifierRejected)?;
    Ok(verifier)
}

fn actor_renderer_contract(
    program: &ResponseProgram,
) -> Result<crate::CollectionOutputRenderer, CrystallizedOperatorError> {
    match &program.operation {
        crate::ResponseOperation::ProjectSelectedValue { renderer, .. }
        | crate::ResponseOperation::ProjectStatus { renderer, .. }
        | crate::ResponseOperation::ComposeCollection { renderer, .. } => Ok(renderer.clone()),
        crate::ResponseOperation::FunctionCallFromRoles { .. }
        | crate::ResponseOperation::CustomToolCallFromRoles { .. } => {
            Ok(crate::CollectionOutputRenderer::Direct)
        }
        crate::ResponseOperation::UniqueConsensus { variants, .. } => {
            let mut renderer = None;
            for variant in variants {
                let candidate = actor_renderer_contract(&variant.program)?;
                if renderer.as_ref().is_some_and(|known| known != &candidate) {
                    return Err(CrystallizedOperatorError::RendererMismatch);
                }
                renderer = Some(candidate);
            }
            renderer.ok_or(CrystallizedOperatorError::RendererMismatch)
        }
        _ => Err(CrystallizedOperatorError::UnsupportedTransformProgram),
    }
}

fn scalar_actor_from_transform_program(
    transform_program: &[TransformOp8],
    renderer: &crate::CollectionOutputRenderer,
) -> Result<ResponseProgram, CrystallizedOperatorError> {
    let [transform] = transform_program else {
        return Err(CrystallizedOperatorError::UnsupportedTransformProgram);
    };
    validate_typed_transform(*transform)?;
    actor_from_transform(*transform, renderer)
}

fn ordered_role_transforms(
    transform_program: &[TransformOp8],
) -> Result<Vec<TransformOp8>, CrystallizedOperatorError> {
    nando_operator_runtime::ordered_role_transforms(transform_program).map_err(Into::into)
}

pub(crate) fn runtime_role_signature_for_selector(
    selector: &ResponseValueSelector,
    plane: u8,
) -> StructuralRoleSignature {
    nando_operator_runtime::runtime_role_signature_for_selector(selector, plane)
}

pub(crate) fn runtime_multi_role_signature_for_selector(
    selector: &ResponseValueSelector,
    plane: u8,
) -> StructuralRoleSignature {
    nando_operator_runtime::runtime_multi_role_signature_for_selector(selector, plane)
}

fn digest_parts(domain: &[u8], parts: &[&[u8]]) -> Commitment256 {
    nando_operator_runtime::digest_parts(domain, parts)
}

pub fn crystallization_raw_input_sha256(
    request_text: &str,
    provider_payload: &Value,
) -> Result<Commitment256, CrystallizedOperatorError> {
    let payload = serde_json::to_vec(provider_payload)
        .map_err(|_| CrystallizedOperatorError::RuntimeRelationMismatch)?;
    Ok(digest_parts(
        b"nando.live-scalar-raw-input.v1",
        &[request_text.as_bytes(), &payload],
    ))
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
    validate_typed_transform(*transform)?;
    actor_from_transform(*transform, renderer)
}

fn validate_typed_transform(transform: TransformOp8) -> Result<(), CrystallizedOperatorError> {
    nando_operator_runtime::validate_typed_transform(transform).map_err(Into::into)
}

fn actor_from_transform(
    transform: TransformOp8,
    renderer: &crate::CollectionOutputRenderer,
) -> Result<ResponseProgram, CrystallizedOperatorError> {
    let value_type = transform_value_type(transform.parameter & 0x00ff)?;
    match transform.opcode {
        TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR => Ok(ResponseProgram::project_selected_value(
            ResponseValueSelector::UniqueScalar { value_type },
            transform_format(transform.flags),
            "completed",
        )
        .with_value_renderer(renderer.clone())),
        TRANSFORM_OPCODE_COUNT_COLLECTION => Ok(ResponseProgram::compose_collection(
            vec![
                crate::CollectionProgramStep::SelectOnlyArrayField,
                crate::CollectionProgramStep::Count,
            ],
            ValueProjectionFormat::PlainText,
            "completed",
        )
        .with_collection_renderer(renderer.clone())),
        TRANSFORM_OPCODE_PROJECT_STATUS => Ok(ResponseProgram::project_status(
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::Integer,
            },
            transform_status_mapping(transform.flags)?,
            "completed",
        )
        .with_status_renderer(renderer.clone())),
        TRANSFORM_OPCODE_FILTER_REQUEST_VALUE => Ok(ResponseProgram::compose_collection(
            vec![
                crate::CollectionProgramStep::SelectOnlyArrayField,
                crate::CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                    selector: ResponseValueSelector::RequestLastToken,
                    value_type: atom_collection_type(value_type)?,
                },
            ],
            ValueProjectionFormat::CanonicalJson,
            "completed",
        )
        .with_collection_renderer(renderer.clone())),
        _ => Err(CrystallizedOperatorError::UnsupportedTransformProgram),
    }
}

fn atom_collection_type(
    value_type: AtomValueType,
) -> Result<crate::CollectionScalarType, CrystallizedOperatorError> {
    Ok(match value_type {
        AtomValueType::String => crate::CollectionScalarType::String,
        AtomValueType::Integer => crate::CollectionScalarType::Integer,
        AtomValueType::Boolean => crate::CollectionScalarType::Boolean,
        _ => return Err(CrystallizedOperatorError::UnsupportedTransformProgram),
    })
}

fn transform_status_mapping(
    flags: u16,
) -> Result<crate::ProjectStatusMapping, CrystallizedOperatorError> {
    Ok(match flags {
        TRANSFORM_STATUS_ZERO_IS_SUCCESS => crate::ProjectStatusMapping::ZeroIsSuccess,
        TRANSFORM_STATUS_ZERO_IS_PASS => crate::ProjectStatusMapping::ZeroIsPass,
        TRANSFORM_STATUS_ZERO_IS_OK => crate::ProjectStatusMapping::ZeroIsOk,
        TRANSFORM_STATUS_ZERO_IS_TRUE => crate::ProjectStatusMapping::ZeroIsTrue,
        _ => return Err(CrystallizedOperatorError::UnsupportedTransformProgram),
    })
}

fn transform_value_type(parameter: u16) -> Result<AtomValueType, CrystallizedOperatorError> {
    nando_operator_runtime::transform_value_type(parameter).map_err(Into::into)
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
    let direct_actor = scalar_actor_from_transform_program(
        blueprint.transform_program(),
        &crate::CollectionOutputRenderer::Direct,
    )?;
    for receipt in receipts {
        let evidence = evidence_by_lineage
            .get(&receipt.future_lineage_sha256)
            .ok_or(CrystallizedOperatorError::FutureEvidenceMismatch)?;
        let provider_payload =
            crate::runtime::provider_payload_view(&receipt.request_text, &receipt.provider_payload)
                .map_err(|_| CrystallizedOperatorError::MissingRuntimeAnchor)?
                .into_owned();
        let bound = bind_operator_components(
            blueprint.role_graph(),
            blueprint.relation_program(),
            blueprint.transform_program(),
            &direct_actor,
            RuntimeSurfaceEvidence {
                bundle: evidence.bundle().clone(),
                request_text: receipt.request_text.clone(),
                provider_payload,
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

pub(crate) struct DurableProgramCrystallizationProof {
    pub generation: u64,
    pub blueprint_sha256: Commitment256,
    pub candidate_set_sha256: Commitment256,
    pub support_root_sha256: Commitment256,
    pub future_evidence_root_sha256: Commitment256,
    pub future_lineage_root_sha256: Commitment256,
    pub winner_seal_sha256: Commitment256,
    pub support_lineages: Vec<Commitment256>,
    pub future_lineages: Vec<Commitment256>,
    pub binding_receipts: Vec<Commitment256>,
    pub execution_receipts: Vec<Commitment256>,
}

fn verify_future_receipts(
    future_window: &FrozenBlueprintFutureWindow,
    future_evidence: &[BlueprintFutureEvidence],
    blueprint: &CandidateOperatorBlueprint,
    actor_template: &ResponseProgram,
    receipts: &[CrystallizationParityReceipt],
) -> Result<FutureParityProof, CrystallizedOperatorError> {
    let expected_lineages = future_window.future_lineages_sha256();
    let expected_surfaces = future_window.future_surfaces_sha256();
    if expected_lineages.is_empty() || expected_surfaces.is_empty() {
        return Err(CrystallizedOperatorError::EmptyFutureWindow);
    }
    let evidence_by_surface = future_evidence
        .iter()
        .map(|evidence| {
            (
                (
                    *evidence.bundle().lineage_sha256(),
                    *evidence.bundle().surface_sha256(),
                ),
                evidence,
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    if evidence_by_surface.len() != expected_surfaces.len()
        || evidence_by_surface
            .keys()
            .map(|(_, surface)| surface)
            .copied()
            .collect::<BTreeSet<_>>()
            != *expected_surfaces
        || evidence_by_surface
            .keys()
            .map(|(lineage, _)| lineage)
            .copied()
            .collect::<BTreeSet<_>>()
            != *expected_lineages
    {
        return Err(CrystallizedOperatorError::FutureEvidenceMismatch);
    }
    let mut seen_evidence = BTreeSet::new();
    let mut seen_lineages = BTreeSet::new();
    let mut binding_receipts = Vec::with_capacity(receipts.len());
    let mut execution_receipts = Vec::with_capacity(receipts.len());
    for receipt in receipts {
        if !expected_lineages.contains(&receipt.future_lineage_sha256) {
            return Err(CrystallizedOperatorError::UnknownParityLineage);
        }
        if !expected_surfaces.contains(&receipt.future_surface_sha256) {
            return Err(CrystallizedOperatorError::FutureEvidenceMismatch);
        }
        let evidence_key = (receipt.future_lineage_sha256, receipt.future_surface_sha256);
        if !seen_evidence.insert(evidence_key) {
            return Err(CrystallizedOperatorError::DuplicateParityEvidence);
        }
        seen_lineages.insert(receipt.future_lineage_sha256);
        let evidence = evidence_by_surface
            .get(&evidence_key)
            .ok_or(CrystallizedOperatorError::FutureEvidenceMismatch)?;
        if receipt.future_surface_sha256 != *evidence.bundle().surface_sha256()
            || receipt.future_bundle_sha256 != *evidence.bundle_sha256()
            || receipt.raw_input_sha256 != *evidence.raw_input_sha256()
            || receipt.extractor_version != evidence.extractor_version()
            || crystallization_raw_input_sha256(&receipt.request_text, &receipt.provider_payload)?
                != receipt.raw_input_sha256
        {
            return Err(CrystallizedOperatorError::FutureEvidenceMismatch);
        }
        // Future evidence teaches and selects the law. Authority is earned by
        // re-extracting a fresh pre-action surface from the committed raw
        // request/payload; caller-provided anchors never cross this boundary.
        let bound = bind_raw_pre_action_components(
            blueprint.role_graph(),
            blueprint.relation_program(),
            blueprint.transform_program(),
            actor_template,
            &receipt.request_text,
            &receipt.provider_payload,
        )?;
        let response = bound.execute_verified()?;
        // Execution budgets control polling cost, not the learned action. The
        // shared normalizer removes only those bounded no-op arguments while
        // preserving the tool, bound roles, and every semantic argument.
        if response != receipt.expected_response
            && !crate::online_admission::responses_match_after_execution_budget_normalization(
                &response,
                &receipt.expected_response,
            )
        {
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
    if seen_evidence.len() != expected_surfaces.len() || seen_lineages != *expected_lineages {
        return Err(CrystallizedOperatorError::MissingParityReceipt);
    }
    Ok(FutureParityProof {
        lineages: seen_lineages.into_iter().collect(),
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
    future_lineage_count: usize,
) -> Result<ExecutableParitySeal, CrystallizedOperatorError> {
    build_executable_parity_seal_from_commitment(
        winner_receipt.seal_sha256(),
        actor_sha256,
        verifier_sha256,
        binding_receipts,
        execution_receipts,
        future_lineage_count,
    )
}

fn build_executable_parity_seal_from_commitment(
    winner_seal_sha256: &Commitment256,
    actor_sha256: &str,
    verifier_sha256: &str,
    binding_receipts: &[Commitment256],
    execution_receipts: &[Commitment256],
    future_lineage_count: usize,
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
    let future_evidence_count = u32::try_from(binding_receipts.len())
        .map_err(|_| CrystallizedOperatorError::DigestFailure)?;
    let future_lineage_count = u32::try_from(future_lineage_count)
        .map_err(|_| CrystallizedOperatorError::DigestFailure)?;
    let wrong_accepts = 0_u32;
    let seal_sha256 = executable_parity_seal_digest(
        winner_seal_sha256,
        &actor_sha256,
        &verifier_sha256,
        &binding_receipts_root,
        &execution_receipts_root,
        future_evidence_count,
        future_lineage_count,
        wrong_accepts,
    );
    Ok(ExecutableParitySeal {
        winner_seal_sha256: *winner_seal_sha256,
        actor_sha256,
        verifier_sha256,
        binding_receipts_root,
        execution_receipts_root,
        future_evidence_count,
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
    future_evidence_count: u32,
    future_lineage_count: u32,
    wrong_accepts: u32,
) -> Commitment256 {
    digest_parts(
        b"nando.executable-parity-seal.v2",
        &[
            winner_seal_sha256,
            actor_sha256,
            verifier_sha256,
            binding_receipts_root,
            execution_receipts_root,
            &future_evidence_count.to_le_bytes(),
            &future_lineage_count.to_le_bytes(),
            &wrong_accepts.to_le_bytes(),
        ],
    )
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
    _candidate_set_sha256: &Commitment256,
    support_lineages: &[Commitment256],
    future_lineages: &[Commitment256],
    output_renderer: &crate::CollectionOutputRenderer,
    uses_typed_actor_renderer: bool,
    _actor_sha256: &str,
    verifier_sha256: &str,
) -> Result<OperatorPage32, CrystallizedOperatorError> {
    build_operator_page_from_parts(
        blueprint.role_graph(),
        blueprint.relation_program(),
        blueprint.transform_program(),
        generation,
        support_lineages,
        future_lineages,
        output_renderer,
        uses_typed_actor_renderer,
        verifier_sha256,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_operator_page_from_parts(
    role_graph: &RoleGraph,
    relation_program: &OperatorCircuit,
    transform_program: &[TransformOp8],
    generation: u64,
    support_lineages: &[Commitment256],
    future_lineages: &[Commitment256],
    output_renderer: &crate::CollectionOutputRenderer,
    uses_typed_actor_renderer: bool,
    verifier_sha256: &str,
) -> Result<OperatorPage32, CrystallizedOperatorError> {
    // Compile every page section from one topological transform order. The
    // renderer stores transform indexes, so reordering only at VM decode time
    // would make an intermediate value look like the final sink after restart.
    let ordered_transforms = ordered_role_transforms(transform_program)?;
    let mut cube = TernaryOperatorCube32::default();
    let mut phase_profile = [0_u8; OPERATOR_PAGE32_PHASE_BYTES];
    let mut plane_count = 0_u8;
    for (index, relation) in relation_program.relations().iter().enumerate() {
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

    let roles = role_graph
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
    let mut composition_count = 0_u8;
    for (producer, produced) in ordered_transforms.iter().enumerate() {
        for (consumer, consumed) in ordered_transforms.iter().enumerate() {
            if producer != consumer
                && (produced.output == consumed.source_a || produced.output == consumed.source_b)
            {
                let offset = usize::from(composition_count) * 2;
                if offset + 1 >= composition.len() {
                    return Err(CrystallizedOperatorError::InvalidPage(
                        OperatorPage32Error::InvalidCompositionCount,
                    ));
                }
                composition[offset] = producer as u8;
                composition[offset + 1] = consumer as u8;
                composition_count = composition_count.saturating_add(1);
            }
        }
    }

    let verifier_digest = decode_sha256(verifier_sha256)?;
    let (renderer, renderer_instruction_count) = if uses_typed_actor_renderer {
        crate::operator_vm::encode_typed_actor_renderer_program(&ordered_transforms)?
    } else {
        crate::operator_vm::encode_renderer_program(output_renderer, &ordered_transforms)?
    };

    let proof_lineage = lineage_commitment(support_lineages, future_lineages);
    let role_commitment = roles_commitment(&roles);
    OperatorPage32::build(
        OperatorPage32Metadata {
            generation,
            circuit_fingerprint64: relation_program.fingerprint64(),
            verifier_binding_fingerprint64: first_u64(&verifier_digest),
            proof_lineage_fingerprint64: first_u64(&proof_lineage),
            role_signature_fingerprint64: first_u64(&role_commitment),
            relation_plane_count: plane_count,
            composition_node_count: composition_count,
            renderer_instruction_count,
            flags: 0,
        },
        &phase_profile,
        &roles,
        &cube,
        &ordered_transforms,
        &composition,
        &renderer,
    )
    .map_err(CrystallizedOperatorError::InvalidPage)
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

fn digest_matches_commitment(digest: &str, expected: &Commitment256) -> bool {
    decode_sha256(digest).is_ok_and(|actual| &actual == expected)
}

pub(crate) fn decode_sha256(value: &str) -> Result<Commitment256, CrystallizedOperatorError> {
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
        FrozenOperatorBlueprintSet, LocalRelationFragment, OPERATOR_PAGE32_BYTES,
        OperatorGrokkingConfig, PhaseCenterCell, RoleAlignmentConfig, StructuralRoleSignature,
        SurfaceFragmentBundle, TernaryRelationState, TypedProgramAtom, phase_vector_from_atoms,
    };
    use serde_json::json;

    use crate::{
        TYPED_EXECUTION_STAGE_RECEIPT_SCHEMA_V1, TypedExecutionStage, TypedExecutionStageReceipt,
        VerifiedDeltaOutcome, VerifiedDeltaRelation, VerifiedDeltaRelationState,
        runtime::observed_request_ordinal_roles,
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
        let phase_atoms = ["scalar_type:2", "cardinality:unique"];
        let phase = phase_vector_from_atoms(phase_atoms, 1)[0];
        SurfaceFragmentBundle::new(
            digest(lineage),
            digest(surface),
            vec![
                StructuralRoleSignature::new(5, 1, 0, 1, vec![0]),
                runtime_role_signature_for_selector(
                    &ResponseValueSelector::UniqueScalar {
                        value_type: AtomValueType::Integer,
                    },
                    0,
                ),
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
    fn pre_action_rich_surface_contains_observation_not_operator_program() {
        let request = "Return total and failed";
        let payload = json!({
            "input": [{
                "type": "function_call_output",
                "output": "{\"total\":100,\"failed\":3}"
            }]
        });
        let observed = observed_request_ordinal_roles(request, &payload)
            .expect("two raw request-referenced roles");
        let evidence = nando_operator_runtime::observed_multi_role_runtime_surface(
            request,
            &payload,
            &observed,
            digest(90),
            digest(91),
        )
        .expect("raw structural surface");

        assert_eq!(evidence.bundle.roles().len(), 3, "context + two sources");
        assert_eq!(evidence.bundle.relations().len(), 2);
        assert!(
            evidence.bundle.program_atoms().is_empty(),
            "pre-action observation must not reflect the sealed TransformProgram"
        );
        assert_eq!(evidence.anchors.len(), 2);
        assert!(
            evidence
                .anchors
                .iter()
                .all(|anchor| anchor.json_path_sha256.is_some())
        );
    }

    #[test]
    fn winner_crystallizes_into_page_and_bound_verified_actor() {
        let (future, synthesis, future_bundle) = frozen_blueprints();
        let winner = *synthesis.blueprints[0].fingerprint_sha256();
        let request = "Return total";
        let payload = json!({
            "input": [{"type":"function_call_output", "output":"{\"total\":7}"}]
        });
        let raw_input_sha256 =
            crystallization_raw_input_sha256(request, &payload).expect("raw input commitment");
        let evidence = BlueprintFutureEvidence::new(raw_input_sha256, 1, future_bundle.clone())
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
        let receipt = CrystallizationParityReceipt {
            future_lineage_sha256: *future_bundle.lineage_sha256(),
            future_surface_sha256: *future_bundle.surface_sha256(),
            future_bundle_sha256: *evidence.bundle_sha256(),
            raw_input_sha256,
            extractor_version: evidence.extractor_version(),
            anchors: vec![RuntimeRoleAnchor {
                local_role: 1,
                selector: ResponseValueSelector::JsonField {
                    field: "total".to_owned(),
                    value_type: AtomValueType::Integer,
                },
                json_path_sha256: None,
            }]
            .into_boxed_slice(),
            request_text: request.to_owned(),
            provider_payload: payload.clone(),
            expected_response: "7".to_owned(),
        };

        let mut tampered_receipt = receipt.clone();
        tampered_receipt.request_text = "Return another scalar".to_owned();
        assert!(matches!(
            CrystallizedOperator::crystallize(
                &future,
                winner_receipt,
                &evidence_set,
                std::slice::from_ref(&tampered_receipt),
            ),
            Err(CrystallizedOperatorError::FutureEvidenceMismatch)
        ));

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
        assert_eq!(operator.parity_seal().future_evidence_count(), 1);
        assert_eq!(operator.parity_seal().future_lineage_count(), 1);
        assert_eq!(operator.parity_seal().wrong_accepts(), 0);
        assert_eq!(
            operator.parity_seal().winner_seal_sha256(),
            winner_receipt.seal_sha256()
        );
        let restart_bundle = operator.restart_bundle().expect("bounded restart bundle");
        assert_eq!(
            format!("{:x}", Sha256::digest(restart_bundle.page_bytes())),
            "982f2960d14552eab32702757f1a877c118989bbebe4a0a8ea5efab8f7d662a5"
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(restart_bundle.registry_cbor())),
            "48683443fb7974b8acf453074fc3b93df295d49e91ab01d203e1fe1eae39ee44"
        );
        assert_eq!(restart_bundle.page_bytes().len(), OPERATOR_PAGE32_BYTES);
        assert!(
            restart_bundle.registry_cbor().len()
                < nando_operator_runtime::CRYSTALLIZED_REGISTRY_MAX_BYTES
        );
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
                request_text: request.to_owned(),
                provider_payload: payload.clone(),
                anchors: vec![RuntimeRoleAnchor {
                    local_role: 1,
                    selector: ResponseValueSelector::JsonField {
                        field: "total".to_owned(),
                        value_type: AtomValueType::Integer,
                    },
                    json_path_sha256: None,
                }]
                .into_boxed_slice(),
            })
            .expect("runtime role binding");
        assert_eq!(bound.execute_verified().as_deref(), Ok("7"));
        let restored_bound = restored
            .bind(RuntimeSurfaceEvidence {
                bundle: future_bundle,
                request_text: request.to_owned(),
                provider_payload: payload.clone(),
                anchors: vec![RuntimeRoleAnchor {
                    local_role: 1,
                    selector: ResponseValueSelector::JsonField {
                        field: "total".to_owned(),
                        value_type: AtomValueType::Integer,
                    },
                    json_path_sha256: None,
                }]
                .into_boxed_slice(),
            })
            .expect("restored runtime role binding");
        assert_eq!(restored_bound.execute_verified().as_deref(), Ok("7"));
        assert_eq!(restored_bound.environment(), bound.environment());
        let runtime_surface = nando_operator_runtime::observed_scalar_runtime_surface(
            request,
            &payload,
            ResponseValueSelector::JsonField {
                field: "total".to_owned(),
                value_type: AtomValueType::Integer,
            },
            digest(70),
            digest(71),
        )
        .expect("pre-action structural extraction");
        assert_eq!(runtime_surface.bundle.roles().len(), 2);
        assert_eq!(runtime_surface.bundle.relations().len(), 1);
        assert!(runtime_surface.bundle.program_atoms().is_empty());
        let automatically_bound = operator
            .bind_pre_action(request, &payload)
            .expect("operator binds from pre-action payload");
        assert_eq!(automatically_bound.execute_verified().as_deref(), Ok("7"));
        let payload_with_unrelated_history = json!({
            "input": [
                {
                    "type": "function_call_output",
                    "call_id": "old",
                    "output": "{\"other\":99}"
                },
                {
                    "type": "function_call_output",
                    "call_id": "current",
                    "output": "{\"total\":7}"
                }
            ]
        });
        let history_bound = operator
            .bind_pre_action(request, &payload_with_unrelated_history)
            .expect("sealed selector ignores unrelated earlier outputs");
        assert_eq!(history_bound.execute_verified().as_deref(), Ok("7"));
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

    #[test]
    fn sealed_blueprint_rejects_external_actor_substitution() {
        let support = vec![
            bundle(1, PhaseCenterCell { re: 1.0, im: 0.0 }),
            bundle(2, PhaseCenterCell { re: 1.0, im: 0.0 }),
        ];
        let alignments = BoundedRoleAligner::align(&support, RoleAlignmentConfig::default());
        let mut synthesis =
            BoundedCircuitBeam::synthesize(&support, &alignments, BlueprintBeamConfig::default());
        let committed_actor = ResponseProgram::project_selected_value(
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::Integer,
            },
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let committed_verifier =
            source_neutral_verifier_for_program(&committed_actor).expect("committed verifier");
        let actor_sha256 =
            decode_sha256(&response_actor_program_digest(&committed_actor).expect("actor digest"))
                .expect("actor commitment");
        let verifier_sha256 = decode_sha256(
            &response_independent_verifier_program_digest(&committed_verifier)
                .expect("verifier digest"),
        )
        .expect("verifier commitment");
        synthesis.blueprints = synthesis
            .blueprints
            .iter()
            .cloned()
            .map(|blueprint| blueprint.bind_executable_contracts(actor_sha256, verifier_sha256))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let frozen = FrozenOperatorBlueprintSet::freeze(
            7,
            &support,
            BlueprintBeamConfig::default(),
            &synthesis,
        )
        .expect("frozen committed blueprints");
        let future_bundle = bundle(3, PhaseCenterCell { re: 1.0, im: 0.0 });
        let mut future = frozen.future_window();
        future
            .admit_lineage(&future_bundle)
            .expect("independent future lineage");
        let request = "Return total";
        let payload = json!({
            "input": [{"type":"function_call_output", "output":"{\"total\":7}"}]
        });
        let raw_input_sha256 =
            crystallization_raw_input_sha256(request, &payload).expect("raw input commitment");
        let evidence = BlueprintFutureEvidence::new(raw_input_sha256, 1, future_bundle.clone())
            .expect("future evidence");
        let evidence_set = vec![evidence.clone()];
        let sealed = BlueprintFutureEvaluator::evaluate_and_seal(
            &frozen,
            &evidence_set,
            Default::default(),
            BlueprintPhaseControl::Full,
        );
        let winner = sealed.winner_receipt().expect("sealed winner");
        let receipt = CrystallizationParityReceipt {
            future_lineage_sha256: *future_bundle.lineage_sha256(),
            future_surface_sha256: *future_bundle.surface_sha256(),
            future_bundle_sha256: *evidence.bundle_sha256(),
            raw_input_sha256,
            extractor_version: 1,
            anchors: Vec::new().into_boxed_slice(),
            request_text: request.to_owned(),
            provider_payload: payload,
            expected_response: "7".to_owned(),
        };
        let substituted_actor = ResponseProgram::project_selected_value(
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::Integer,
            },
            ValueProjectionFormat::CanonicalJson,
            "completed",
        );

        assert!(matches!(
            CrystallizedOperator::crystallize_with_actor_template(
                &future,
                winner,
                &evidence_set,
                std::slice::from_ref(&receipt),
                substituted_actor,
            ),
            Err(CrystallizedOperatorError::ActorContractMismatch)
        ));
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
