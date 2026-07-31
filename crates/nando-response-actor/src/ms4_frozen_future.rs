//! Fail-closed bridge from a frozen MS3 law and independent future to Bundle V4.
//!
//! This module deliberately stops at a QUARANTINE package. Crystallization
//! proves that the learned program is executable and restart-stable; it does
//! not grant routing authority or manufacture the missing Wave ablation.

use std::fmt::Write as _;

use nando_operator_admission::{
    AdaptiveIdentificationProofInputV1, ResponsePackageOrigin, ResponsePackageProof,
    ResponsePackageState, seal_adaptive_identification_proof_v1,
};
use nando_operator_kernel::{
    canonical_json_sha256, response_program_version_root_sha256, valid_nonzero_sha256,
};
use nando_operator_learning::multi_source::{
    FrozenVersionSpaceEnvelopeV1, Ms3FrozenVersionSpaceStateV1, Ms3IndependentFutureEnvelopeV1,
    Ms3IndependentFutureVerdictV1,
};
use nando_operator_learning::{OperatorIdentificationMachineV1, RuntimeParityCase};
use serde::{Deserialize, Serialize};

use crate::crystallized_operator::{
    DurableProgramCrystallizationProof, VerifiedCrystallizedOperator, decode_sha256,
};
use crate::{
    DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1, DurableRuntimeParityReceipt, ResponseExecutionStatus,
    ResponsePackage, ResponseProgram, VerifiedOperatorRestartBundle, VerifierProgram,
    execute_response, response_actor_program_digest, response_independent_verifier_program_digest,
    response_program_external_verifier_schema, response_program_required_routing_atom_ids,
    sha256_bytes, source_neutral_verifier_for_program, verify_response_independently_with_request,
};

pub const MS4_FROZEN_FUTURE_SHADOW_CANDIDATE_SCHEMA_V1: &str =
    "nando.ms4-frozen-future-shadow-candidate.v1";
const MS4_FROZEN_FUTURE_SHADOW_CANDIDATE_MAX_BYTES_V1: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ms4RuntimeEvidenceV1 {
    /// Root of the source frame bound by the MS3 support/future contract.
    pub source_frame_root_sha256: String,
    /// Independent session lineage; instance IDs never enter the operator.
    pub session_lineage_sha256: String,
    /// Pre-action topology root used as the transfer surface identity.
    pub surface_sha256: String,
    pub parity: RuntimeParityCase,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Ms4RuntimeBindingReceiptV1 {
    binding_root_sha256: String,
    partition: String,
    source_frame_root_sha256: String,
    session_lineage_sha256: String,
    surface_sha256: String,
    runtime_evidence_ref_sha256: String,
    runtime_input_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms4FrozenFutureShadowCandidateV1 {
    schema: String,
    candidate_root_sha256: String,
    frozen_envelope_root_sha256: String,
    future_envelope_root_sha256: String,
    canonical_program_root_sha256: String,
    canonical_bundle_id_sha256: String,
    operator_page_sha256: String,
    operator_registry_sha256: String,
    support_runtime_receipt: DurableRuntimeParityReceipt,
    future_runtime_receipt: DurableRuntimeParityReceipt,
    support_binding: Ms4RuntimeBindingReceiptV1,
    future_binding: Ms4RuntimeBindingReceiptV1,
    package: ResponsePackage,
    active_admission_blocker: String,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

impl Ms4FrozenFutureShadowCandidateV1 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, &'static str> {
        self.validate()?;
        let bytes = serde_cbor::to_vec(self).map_err(|_| "ms4_shadow_candidate_encode_failed")?;
        if bytes.is_empty() || bytes.len() > MS4_FROZEN_FUTURE_SHADOW_CANDIDATE_MAX_BYTES_V1 {
            return Err("ms4_shadow_candidate_byte_budget");
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.is_empty() || bytes.len() > MS4_FROZEN_FUTURE_SHADOW_CANDIDATE_MAX_BYTES_V1 {
            return Err("ms4_shadow_candidate_byte_budget");
        }
        let candidate: Self =
            serde_cbor::from_slice(bytes).map_err(|_| "ms4_shadow_candidate_decode_failed")?;
        candidate.validate()?;
        if candidate.canonical_bytes()? != bytes {
            return Err("ms4_shadow_candidate_noncanonical");
        }
        Ok(candidate)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let roots = [
            self.candidate_root_sha256.as_str(),
            self.frozen_envelope_root_sha256.as_str(),
            self.future_envelope_root_sha256.as_str(),
            self.canonical_program_root_sha256.as_str(),
            self.canonical_bundle_id_sha256.as_str(),
            self.operator_page_sha256.as_str(),
            self.operator_registry_sha256.as_str(),
        ];
        if self.schema != MS4_FROZEN_FUTURE_SHADOW_CANDIDATE_SCHEMA_V1
            || !roots.into_iter().all(valid_nonzero_sha256)
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.package.origin != ResponsePackageOrigin::GroundedSynthesis
            || self.package.state != ResponsePackageState::Quarantine
            || self.package.proof.wave_causal_pass
        {
            return Err("ms4_shadow_candidate_contract_invalid");
        }
        self.support_runtime_receipt.validate_sealed()?;
        self.future_runtime_receipt.validate_sealed()?;
        self.support_binding.validate()?;
        self.future_binding.validate()?;
        if self.support_binding.partition != "support"
            || self.future_binding.partition != "future"
            || self.support_binding.runtime_evidence_ref_sha256
                != self.support_runtime_receipt.evidence_ref_sha256
            || self.support_binding.runtime_input_sha256
                != self.support_runtime_receipt.input_sha256
            || self.future_binding.runtime_evidence_ref_sha256
                != self.future_runtime_receipt.evidence_ref_sha256
            || self.future_binding.runtime_input_sha256 != self.future_runtime_receipt.input_sha256
            || self.support_binding.session_lineage_sha256
                == self.future_binding.session_lineage_sha256
            || self.support_binding.surface_sha256 == self.future_binding.surface_sha256
            || self.support_binding.runtime_evidence_ref_sha256
                == self.future_binding.runtime_evidence_ref_sha256
        {
            return Err("ms4_shadow_candidate_runtime_binding_invalid");
        }
        self.package.validate()?;
        if response_program_version_root_sha256(&self.package.program)?
            != self.canonical_program_root_sha256
        {
            return Err("ms4_shadow_candidate_program_root_mismatch");
        }
        let bundle = self
            .package
            .crystallized_operator
            .as_ref()
            .ok_or("ms4_shadow_candidate_bundle_missing")?;
        if !bundle.has_canonical_bundle_v4()
            || bundle_id_hex(bundle)? != self.canonical_bundle_id_sha256
            || sha256_bytes(bundle.page_bytes()) != self.operator_page_sha256
            || sha256_bytes(bundle.registry_cbor()) != self.operator_registry_sha256
        {
            return Err("ms4_shadow_candidate_bundle_mismatch");
        }
        let mut active = self.package.clone();
        active.state = ResponsePackageState::Active;
        let blocker = active
            .admission_candidate_blocker()
            .ok_or("ms4_shadow_candidate_unexpected_authority")?;
        if blocker != self.active_admission_blocker
            || self.candidate_root_sha256 != self.expected_root()?
        {
            return Err("ms4_shadow_candidate_seal_mismatch");
        }
        Ok(())
    }

    pub fn validate_against(
        &self,
        frozen: &FrozenVersionSpaceEnvelopeV1,
        future: &Ms3IndependentFutureEnvelopeV1,
    ) -> Result<(), &'static str> {
        self.validate()?;
        frozen
            .validate()
            .map_err(|_| "ms4_frozen_envelope_invalid")?;
        future.validate(frozen)?;
        let candidate_freeze_root = match &frozen.contract.state {
            Ms3FrozenVersionSpaceStateV1::UniqueLawFrozen {
                candidate_freeze_root_sha256,
                ..
            } => candidate_freeze_root_sha256,
            _ => return Err("ms4_unique_law_not_frozen"),
        };
        if self.frozen_envelope_root_sha256 != frozen.envelope_root_sha256
            || self.future_envelope_root_sha256 != future.envelope_root_sha256
            || frozen.contract.authority_ready
            || frozen.contract.phase_mutation_allowed
            || future.receipt.verdict != Ms3IndependentFutureVerdictV1::Pass
            || !future.receipt.exact_transfer_parity
            || future.receipt.runtime_actor_verifier_parity
            || future.receipt.authority_ready
            || future.receipt.phase_mutation_allowed
            || self.canonical_program_root_sha256 != future.receipt.canonical_program_root_sha256
            || candidate_freeze_root != &future.receipt.candidate_freeze_root_sha256
            || self.support_binding.source_frame_root_sha256 != frozen.contract.frame_root_sha256
            || self.support_binding.session_lineage_sha256 != frozen.contract.session_lineage_sha256
            || self.support_binding.surface_sha256 != frozen.contract.topology_root_sha256
            || self.future_binding.source_frame_root_sha256
                != future.receipt.completed_frame_root_sha256
            || self.future_binding.session_lineage_sha256 != future.receipt.session_lineage_sha256
            || self.future_binding.surface_sha256 != future.receipt.topology_root_sha256
        {
            return Err("ms4_shadow_candidate_source_binding_mismatch");
        }
        Ok(())
    }

    #[must_use]
    pub fn package(&self) -> &ResponsePackage {
        &self.package
    }

    #[must_use]
    pub fn candidate_root_sha256(&self) -> &str {
        &self.candidate_root_sha256
    }

    #[must_use]
    pub fn canonical_bundle_id_sha256(&self) -> &str {
        &self.canonical_bundle_id_sha256
    }

    #[must_use]
    pub fn active_admission_blocker(&self) -> &str {
        &self.active_admission_blocker
    }

    #[must_use]
    pub const fn authority_ready(&self) -> bool {
        false
    }

    #[must_use]
    pub fn support_runtime_receipt(&self) -> &DurableRuntimeParityReceipt {
        &self.support_runtime_receipt
    }

    #[must_use]
    pub fn future_runtime_receipt(&self) -> &DurableRuntimeParityReceipt {
        &self.future_runtime_receipt
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            MS4_FROZEN_FUTURE_SHADOW_CANDIDATE_SCHEMA_V1,
            self.frozen_envelope_root_sha256.as_str(),
            self.future_envelope_root_sha256.as_str(),
            self.canonical_program_root_sha256.as_str(),
            self.canonical_bundle_id_sha256.as_str(),
            self.operator_page_sha256.as_str(),
            self.operator_registry_sha256.as_str(),
            &self.support_runtime_receipt,
            &self.future_runtime_receipt,
            &self.support_binding,
            &self.future_binding,
            &self.package,
            self.active_admission_blocker.as_str(),
            false,
            false,
        ))
    }
}

impl Ms4RuntimeBindingReceiptV1 {
    fn validate(&self) -> Result<(), &'static str> {
        let roots = [
            self.binding_root_sha256.as_str(),
            self.source_frame_root_sha256.as_str(),
            self.session_lineage_sha256.as_str(),
            self.surface_sha256.as_str(),
            self.runtime_evidence_ref_sha256.as_str(),
            self.runtime_input_sha256.as_str(),
        ];
        if !matches!(self.partition.as_str(), "support" | "future")
            || !roots.into_iter().all(valid_nonzero_sha256)
            || self.binding_root_sha256 != self.expected_root()?
        {
            return Err("ms4_runtime_binding_receipt_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            "nando.ms4-runtime-binding-receipt.v1",
            self.partition.as_str(),
            self.source_frame_root_sha256.as_str(),
            self.session_lineage_sha256.as_str(),
            self.surface_sha256.as_str(),
            self.runtime_evidence_ref_sha256.as_str(),
            self.runtime_input_sha256.as_str(),
        ))
    }
}

/// Crystallizes an already-frozen law after one independently verified future.
///
/// The output is intentionally non-authoritative. External admission still
/// requires a real phase/negative applicability proof and performs its own
/// candidate reconstruction.
pub fn crystallize_ms4_frozen_future_shadow_v1(
    frozen: &FrozenVersionSpaceEnvelopeV1,
    future: &Ms3IndependentFutureEnvelopeV1,
    support: &Ms4RuntimeEvidenceV1,
    future_evidence: &Ms4RuntimeEvidenceV1,
) -> Result<Ms4FrozenFutureShadowCandidateV1, &'static str> {
    frozen
        .validate()
        .map_err(|_| "ms4_frozen_envelope_invalid")?;
    future.validate(frozen)?;
    let (semantic_class_root, candidate_freeze_root) = match &frozen.contract.state {
        Ms3FrozenVersionSpaceStateV1::UniqueLawFrozen {
            semantic_class_root_sha256,
            candidate_freeze_root_sha256,
        } => (
            semantic_class_root_sha256.as_str(),
            candidate_freeze_root_sha256.as_str(),
        ),
        _ => return Err("ms4_unique_law_not_frozen"),
    };
    if frozen.contract.authority_ready
        || frozen.contract.phase_mutation_allowed
        || future.receipt.verdict != Ms3IndependentFutureVerdictV1::Pass
        || !future.receipt.exact_transfer_parity
        || future.receipt.runtime_actor_verifier_parity
        || future.receipt.authority_ready
        || future.receipt.phase_mutation_allowed
    {
        return Err("ms4_future_not_eligible_for_runtime_proof");
    }
    validate_runtime_evidence_roots(frozen, future, support, future_evidence)?;

    let machine =
        OperatorIdentificationMachineV1::from_checkpoint_bytes(frozen.machine_checkpoint())
            .map_err(|_| "ms4_identification_machine_restore_failed")?;
    let freeze = machine.freeze().ok_or("ms4_candidate_freeze_missing")?;
    freeze
        .validate()
        .map_err(|_| "ms4_candidate_freeze_invalid")?;
    if freeze.freeze_root_sha256() != candidate_freeze_root
        || freeze.freeze_root_sha256() != future.receipt.candidate_freeze_root_sha256
        || freeze.semantic_class_id().as_str() != semantic_class_root
        || freeze.canonical_program_root_sha256() != future.receipt.canonical_program_root_sha256
    {
        return Err("ms4_candidate_freeze_binding_mismatch");
    }
    let programs = machine.candidate_programs();
    let program = programs
        .get(freeze.canonical_program_root_sha256())
        .cloned()
        .ok_or("ms4_canonical_program_missing")?;
    program.validate()?;
    if response_program_version_root_sha256(&program)? != freeze.canonical_program_root_sha256() {
        return Err("ms4_canonical_program_root_mismatch");
    }
    let verifier = source_neutral_verifier_for_program(&program)?;
    let support_runtime_receipt = seal_runtime_parity_receipt(&program, &verifier, support)?;
    let future_runtime_receipt = seal_runtime_parity_receipt(&program, &verifier, future_evidence)?;
    let support_binding =
        seal_runtime_binding_receipt("support", support, &support_runtime_receipt)?;
    let future_binding =
        seal_runtime_binding_receipt("future", future_evidence, &future_runtime_receipt)?;
    let restart_bundle = crystallize_restart_bundle(
        frozen,
        future,
        freeze.freeze_root_sha256(),
        &program,
        &verifier,
        support,
        future_evidence,
        &future_runtime_receipt,
    )?;
    let package = build_quarantine_package(freeze, future, program, verifier, restart_bundle)?;
    let package_bundle = package
        .crystallized_operator
        .as_ref()
        .ok_or("ms4_crystallized_bundle_missing")?;
    let canonical_bundle_id_sha256 = bundle_id_hex(package_bundle)?;
    let operator_page_sha256 = sha256_bytes(package_bundle.page_bytes());
    let operator_registry_sha256 = sha256_bytes(package_bundle.registry_cbor());
    let mut active = package.clone();
    active.state = ResponsePackageState::Active;
    let active_admission_blocker = active
        .admission_candidate_blocker()
        .ok_or("ms4_shadow_candidate_unexpected_authority")?
        .to_owned();
    let mut candidate = Ms4FrozenFutureShadowCandidateV1 {
        schema: MS4_FROZEN_FUTURE_SHADOW_CANDIDATE_SCHEMA_V1.to_owned(),
        candidate_root_sha256: String::new(),
        frozen_envelope_root_sha256: frozen.envelope_root_sha256.clone(),
        future_envelope_root_sha256: future.envelope_root_sha256.clone(),
        canonical_program_root_sha256: freeze.canonical_program_root_sha256().to_owned(),
        canonical_bundle_id_sha256,
        operator_page_sha256,
        operator_registry_sha256,
        support_runtime_receipt,
        future_runtime_receipt,
        support_binding,
        future_binding,
        package,
        active_admission_blocker,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    candidate.candidate_root_sha256 = candidate.expected_root()?;
    candidate.validate_against(frozen, future)?;
    Ok(candidate)
}

fn validate_runtime_evidence_roots(
    frozen: &FrozenVersionSpaceEnvelopeV1,
    future: &Ms3IndependentFutureEnvelopeV1,
    support: &Ms4RuntimeEvidenceV1,
    future_evidence: &Ms4RuntimeEvidenceV1,
) -> Result<(), &'static str> {
    let roots = [
        support.source_frame_root_sha256.as_str(),
        support.session_lineage_sha256.as_str(),
        support.surface_sha256.as_str(),
        support.parity.evidence_ref_sha256.as_str(),
        future_evidence.source_frame_root_sha256.as_str(),
        future_evidence.session_lineage_sha256.as_str(),
        future_evidence.surface_sha256.as_str(),
        future_evidence.parity.evidence_ref_sha256.as_str(),
    ];
    if !roots.into_iter().all(valid_nonzero_sha256) {
        return Err("ms4_runtime_evidence_root_invalid");
    }
    if support.source_frame_root_sha256 != frozen.contract.frame_root_sha256
        || support.session_lineage_sha256 != frozen.contract.session_lineage_sha256
        || support.surface_sha256 != frozen.contract.topology_root_sha256
        || future_evidence.source_frame_root_sha256 != future.receipt.completed_frame_root_sha256
        || future_evidence.session_lineage_sha256 != future.receipt.session_lineage_sha256
        || future_evidence.surface_sha256 != future.receipt.topology_root_sha256
    {
        return Err("ms4_runtime_evidence_binding_mismatch");
    }
    if support.session_lineage_sha256 == future_evidence.session_lineage_sha256
        || support.surface_sha256 == future_evidence.surface_sha256
        || support.parity.evidence_ref_sha256 == future_evidence.parity.evidence_ref_sha256
    {
        return Err("ms4_runtime_evidence_partition_overlap");
    }
    Ok(())
}

fn seal_runtime_binding_receipt(
    partition: &str,
    evidence: &Ms4RuntimeEvidenceV1,
    runtime_receipt: &DurableRuntimeParityReceipt,
) -> Result<Ms4RuntimeBindingReceiptV1, &'static str> {
    let mut receipt = Ms4RuntimeBindingReceiptV1 {
        binding_root_sha256: String::new(),
        partition: partition.to_owned(),
        source_frame_root_sha256: evidence.source_frame_root_sha256.clone(),
        session_lineage_sha256: evidence.session_lineage_sha256.clone(),
        surface_sha256: evidence.surface_sha256.clone(),
        runtime_evidence_ref_sha256: runtime_receipt.evidence_ref_sha256.clone(),
        runtime_input_sha256: runtime_receipt.input_sha256.clone(),
    };
    receipt.binding_root_sha256 = receipt.expected_root()?;
    receipt.validate()?;
    Ok(receipt)
}

fn seal_runtime_parity_receipt(
    program: &ResponseProgram,
    verifier: &VerifierProgram,
    evidence: &Ms4RuntimeEvidenceV1,
) -> Result<DurableRuntimeParityReceipt, &'static str> {
    if !valid_nonzero_sha256(&evidence.parity.evidence_ref_sha256) {
        return Err("ms4_runtime_parity_evidence_ref_invalid");
    }
    let execution = execute_response(
        program,
        &evidence.parity.request_text,
        &evidence.parity.provider_payload,
    );
    if execution.status != ResponseExecutionStatus::Executed {
        return Err("ms4_runtime_actor_abstained");
    }
    let actor_response = execution
        .response
        .as_deref()
        .ok_or("ms4_runtime_actor_response_missing")?;
    if actor_response != evidence.parity.expected_response {
        return Err("ms4_runtime_teacher_parity_mismatch");
    }
    verify_response_independently_with_request(
        verifier,
        &evidence.parity.request_text,
        &evidence.parity.provider_payload,
        actor_response,
    )
    .map_err(|error| error.0)?;
    let mut receipt = DurableRuntimeParityReceipt {
        schema: DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1.to_owned(),
        receipt_sha256: String::new(),
        evidence_ref_sha256: evidence.parity.evidence_ref_sha256.clone(),
        program_sha256: response_actor_program_digest(program)?,
        verifier_sha256: response_independent_verifier_program_digest(verifier)?,
        input_sha256: canonical_json_sha256(&(
            evidence.parity.request_text.as_str(),
            &evidence.parity.provider_payload,
        ))?,
        teacher_response_sha256: sha256_bytes(evidence.parity.expected_response.as_bytes()),
        actor_response_sha256: sha256_bytes(actor_response.as_bytes()),
        actor_executed: true,
        teacher_authority_match: true,
        independent_verifier_pass: true,
        exact_teacher_match: true,
    };
    receipt.seal_digest()?;
    receipt.validate_sealed()?;
    Ok(receipt)
}

#[allow(clippy::too_many_arguments)]
fn crystallize_restart_bundle(
    frozen: &FrozenVersionSpaceEnvelopeV1,
    future: &Ms3IndependentFutureEnvelopeV1,
    candidate_freeze_root_sha256: &str,
    program: &ResponseProgram,
    verifier: &VerifierProgram,
    support: &Ms4RuntimeEvidenceV1,
    future_evidence: &Ms4RuntimeEvidenceV1,
    future_runtime_receipt: &DurableRuntimeParityReceipt,
) -> Result<VerifiedOperatorRestartBundle, &'static str> {
    let future_lineage_root_sha256 = canonical_json_sha256(&(
        "nando.ms4-future-lineages.v1",
        future_evidence.session_lineage_sha256.as_str(),
    ))?;
    let binding_receipt_sha256 = canonical_json_sha256(&(
        "nando.ms4-pre-action-binding.v1",
        future.receipt.prediction_root_sha256.as_str(),
        future.receipt.applicability_event_root_sha256.as_str(),
        future.receipt.topology_root_sha256.as_str(),
        future.receipt.transport_binding_root_sha256.as_str(),
        future_runtime_receipt.input_sha256.as_str(),
    ))?;
    let winner_seal_sha256 = canonical_json_sha256(&(
        "nando.ms4-frozen-future-winner.v1",
        frozen.envelope_root_sha256.as_str(),
        future.envelope_root_sha256.as_str(),
        future.receipt.canonical_program_root_sha256.as_str(),
        future_runtime_receipt.receipt_sha256.as_str(),
    ))?;
    let proof = DurableProgramCrystallizationProof {
        generation: frozen.contract.future_min_sequence,
        blueprint_sha256: decode_sha256(&future.receipt.canonical_program_root_sha256)
            .map_err(|_| "ms4_blueprint_root_invalid")?,
        candidate_set_sha256: decode_sha256(candidate_freeze_root_sha256)
            .map_err(|_| "ms4_candidate_set_root_invalid")?,
        support_root_sha256: decode_sha256(&frozen.contract.support_rows_root_sha256)
            .map_err(|_| "ms4_support_root_invalid")?,
        future_evidence_root_sha256: decode_sha256(&future.receipt.receipt_root_sha256)
            .map_err(|_| "ms4_future_evidence_root_invalid")?,
        future_lineage_root_sha256: decode_sha256(&future_lineage_root_sha256)
            .map_err(|_| "ms4_future_lineage_root_invalid")?,
        winner_seal_sha256: decode_sha256(&winner_seal_sha256)
            .map_err(|_| "ms4_winner_seal_invalid")?,
        support_lineages: vec![
            decode_sha256(&support.session_lineage_sha256)
                .map_err(|_| "ms4_support_lineage_invalid")?,
        ],
        future_lineages: vec![
            decode_sha256(&future_evidence.session_lineage_sha256)
                .map_err(|_| "ms4_future_lineage_invalid")?,
        ],
        binding_receipts: vec![
            decode_sha256(&binding_receipt_sha256).map_err(|_| "ms4_binding_receipt_invalid")?,
        ],
        execution_receipts: vec![
            decode_sha256(&future_runtime_receipt.receipt_sha256)
                .map_err(|_| "ms4_execution_receipt_invalid")?,
        ],
    };
    let operator = VerifiedCrystallizedOperator::crystallize_durable_program(
        program.clone(),
        verifier.clone(),
        proof,
    )
    .map_err(|_| "ms4_operator_crystallization_failed")?;
    let bundle = operator
        .restart_bundle()
        .map_err(|_| "ms4_restart_bundle_failed")?;
    let restored = bundle
        .restore_verified()
        .map_err(|_| "ms4_restart_restore_failed")?;
    if !bundle.has_canonical_bundle_v4()
        || !operator.execution_equivalent(&restored)
        || operator.actor_sha256() != restored.actor_sha256()
        || operator.verifier_sha256() != restored.verifier_sha256()
    {
        return Err("ms4_restart_parity_mismatch");
    }
    Ok(bundle)
}

fn build_quarantine_package(
    freeze: &nando_operator_learning::CandidateFreezeReceiptV1,
    future: &Ms3IndependentFutureEnvelopeV1,
    program: ResponseProgram,
    verifier: VerifierProgram,
    restart_bundle: VerifiedOperatorRestartBundle,
) -> Result<ResponsePackage, &'static str> {
    let adaptive_identification =
        seal_adaptive_identification_proof_v1(AdaptiveIdentificationProofInputV1 {
            candidate_freeze_root_sha256: freeze.freeze_root_sha256().to_owned(),
            semantic_class_id_sha256: freeze.semantic_class_id().as_str().to_owned(),
            canonical_program_root_sha256: freeze.canonical_program_root_sha256().to_owned(),
            applicability_scope_root_sha256: freeze.applicability_scope_root_sha256().to_owned(),
            transfer_proof_root_sha256: future.receipt.receipt_root_sha256.clone(),
        })?;
    let verifier_schema = response_program_external_verifier_schema(&program)
        .ok_or("ms4_external_verifier_schema_missing")?
        .to_owned();
    let mut required_routing_atom_ids = response_program_required_routing_atom_ids(&program);
    required_routing_atom_ids.sort_unstable();
    required_routing_atom_ids.dedup();
    let bundle_id = bundle_id_hex(&restart_bundle)?;
    let restored = restart_bundle
        .restore_verified()
        .map_err(|_| "ms4_package_restart_restore_failed")?;
    let package = ResponsePackage {
        schema: "nando.response-package.v1".to_owned(),
        package_id: format!("ms4-shadow-{bundle_id}"),
        origin: ResponsePackageOrigin::GroundedSynthesis,
        state: ResponsePackageState::Quarantine,
        program,
        verifier: Some(verifier),
        routing_predicates: Vec::new(),
        required_routing_atom_ids,
        // This fingerprint only makes the compatibility schema complete. The
        // package remains blocked until an independently measured Wave proof.
        phase_centers: vec![restored.relation_program().fingerprint64()],
        anti_centers: Vec::new(),
        wave_margin_micro: 1,
        learned_wave_route: None,
        crystallized_operator: Some(restart_bundle),
        proof: ResponsePackageProof {
            support_rows: 1,
            future_rows: 1,
            distinct_sessions: 2,
            distinct_surfaces: 2,
            wrong_accepts: 0,
            runtime_parity_failures: 0,
            exact_cache_overlap: 0,
            wave_causal_pass: false,
            verifier_schema,
            adaptive_identification: Some(adaptive_identification),
        },
    };
    package.validate()?;
    Ok(package)
}

fn bundle_id_hex(bundle: &VerifiedOperatorRestartBundle) -> Result<String, &'static str> {
    let id = bundle
        .canonical_bundle_id()
        .ok_or("ms4_canonical_bundle_id_missing")?;
    let mut output = String::with_capacity(64);
    for byte in id {
        write!(&mut output, "{byte:02x}").map_err(|_| "ms4_bundle_id_encode_failed")?;
    }
    valid_nonzero_sha256(&output)
        .then_some(output)
        .ok_or("ms4_canonical_bundle_id_invalid")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        AtomValueType, ResponseArgument, ResponseValueSelector, SemanticRole, ValueProjectionFormat,
    };

    fn projection_program() -> ResponseProgram {
        ResponseProgram::project_selected_value(
            ResponseValueSelector::RequestReferencedJsonField {
                value_type: AtomValueType::Integer,
            },
            ValueProjectionFormat::PlainText,
            "completed",
        )
    }

    fn evidence(expected_response: &str) -> Ms4RuntimeEvidenceV1 {
        Ms4RuntimeEvidenceV1 {
            source_frame_root_sha256: "1".repeat(64),
            session_lineage_sha256: "2".repeat(64),
            surface_sha256: "3".repeat(64),
            parity: RuntimeParityCase {
                evidence_ref_sha256: "4".repeat(64),
                capture_receipt: None,
                request_text: "Return alpha".to_owned(),
                provider_payload: json!({
                    "input": [
                        {"type":"message", "role":"user", "content":"Return alpha"},
                        {"type":"function_call_output", "output":"{\"alpha\":7,\"beta\":8}"}
                    ]
                }),
                expected_response: expected_response.to_owned(),
            },
        }
    }

    #[test]
    fn runtime_receipt_requires_exact_actor_and_independent_verifier_parity() {
        let program = projection_program();
        let verifier = source_neutral_verifier_for_program(&program).expect("verifier");
        let receipt =
            seal_runtime_parity_receipt(&program, &verifier, &evidence("7")).expect("receipt");

        assert!(receipt.validate_sealed().is_ok());
        assert!(receipt.exact_teacher_match);
        assert_eq!(
            receipt.teacher_response_sha256,
            receipt.actor_response_sha256
        );
        assert_eq!(
            seal_runtime_parity_receipt(&program, &verifier, &evidence("8")),
            Err("ms4_runtime_teacher_parity_mismatch")
        );
    }

    #[test]
    fn runtime_receipt_accepts_protocol_continuation_selector() {
        let program = ResponseProgram::function_call_from_roles(
            "wait",
            ResponseValueSelector::ContinuationHandle {
                value_type: AtomValueType::Identifier,
            },
            vec![ResponseArgument::Role {
                name: "cell_id".to_owned(),
                role: SemanticRole::ContinuationHandle,
                value_type: Some(AtomValueType::String),
            }],
        );
        let verifier = source_neutral_verifier_for_program(&program).expect("verifier");
        let evidence = Ms4RuntimeEvidenceV1 {
            source_frame_root_sha256: "1".repeat(64),
            session_lineage_sha256: "2".repeat(64),
            surface_sha256: "3".repeat(64),
            parity: RuntimeParityCase {
                evidence_ref_sha256: "4".repeat(64),
                capture_receipt: None,
                request_text: "continue".to_owned(),
                provider_payload: json!({
                    "input": [{
                        "type": "function_call_output",
                        "output": "Script running with cell ID cell-17"
                    }]
                }),
                expected_response: serde_json::to_string(&json!({
                    "name": "wait",
                    "arguments": {"cell_id": "cell-17"}
                }))
                .expect("expected response"),
            },
        };

        let receipt =
            seal_runtime_parity_receipt(&program, &verifier, &evidence).expect("runtime receipt");
        assert!(receipt.validate_sealed().is_ok());
        assert!(receipt.independent_verifier_pass);
    }

    #[test]
    fn runtime_binding_receipt_rejects_post_seal_root_substitution() {
        let program = projection_program();
        let verifier = source_neutral_verifier_for_program(&program).expect("verifier");
        let evidence = evidence("7");
        let runtime =
            seal_runtime_parity_receipt(&program, &verifier, &evidence).expect("runtime receipt");
        let mut binding =
            seal_runtime_binding_receipt("support", &evidence, &runtime).expect("binding receipt");

        assert!(binding.validate().is_ok());
        binding.surface_sha256 = "5".repeat(64);
        assert_eq!(
            binding.validate(),
            Err("ms4_runtime_binding_receipt_invalid")
        );
    }

    #[test]
    fn v4_crystallization_restarts_and_executes_without_learner_state() {
        let program = projection_program();
        let verifier = source_neutral_verifier_for_program(&program).expect("verifier");
        let proof = DurableProgramCrystallizationProof {
            generation: 1,
            blueprint_sha256: [1; 32],
            candidate_set_sha256: [2; 32],
            support_root_sha256: [3; 32],
            future_evidence_root_sha256: [4; 32],
            future_lineage_root_sha256: [5; 32],
            winner_seal_sha256: [6; 32],
            support_lineages: vec![[7; 32]],
            future_lineages: vec![[8; 32]],
            binding_receipts: vec![[9; 32]],
            execution_receipts: vec![[10; 32]],
        };
        let operator =
            VerifiedCrystallizedOperator::crystallize_durable_program(program, verifier, proof)
                .expect("operator");
        let bundle = operator.restart_bundle().expect("Bundle V4");
        assert!(bundle.has_canonical_bundle_v4());

        let restored = bundle.restore_verified().expect("restart");
        let input = evidence("7").parity;
        let bound = restored
            .bind_pre_action(&input.request_text, &input.provider_payload)
            .expect("role binding");
        assert_eq!(bound.execute_verified().expect("verified execution"), "7");
    }
}
