//! Independent admission boundary for one frozen MS3 natural law.

use std::collections::{BTreeMap, BTreeSet};

use nando_core::wave::{
    PhaseCenterCell, phase_coherence, phase_margin_to_micro, phase_vector_from_atom_ids,
};
use nando_operator_kernel::{RelationFrame, canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::multi_source::{
    FrozenVersionSpaceEnvelopeV1, Ms3FutureApplicabilityV1, Ms3IndependentFutureEnvelopeV1,
    Ms3IndependentFutureVerdictV1, NandoRouteReceiptV1, PreActionTopologyAuditRowV1,
    RequestActionBindingV1, TransportBindingLedgerV1, TransportTerminalReceiptV1,
    bind_independent_fallback_transition_v1, classify_ms3_unique_law_v1,
    pre_action_t1_binding_root, validate_request_action_binding_v1,
};
use nando_operator_learning::{OperatorIdentificationMachineV1, RuntimeParityCase};
use serde::{Deserialize, Serialize};

use crate::authority::build_composite_admission_for_registry;
use crate::{
    Ms4FrozenFutureShadowCandidateV1, Ms4RuntimeEvidenceV1, OnlineAdmissionSnapshot,
    RESPONSE_REGISTRY_SCHEMA_V6, ResponsePackage, ResponsePackageState, ResponseRegistry,
    crystallize_ms4_frozen_future_shadow_v1, response_execution_payload_digest,
};

pub const MS4_EXTERNAL_ADMISSION_CANDIDATE_SCHEMA_V1: &str =
    "nando.ms4-external-admission-candidate.v1";
const MS4_ADAPTIVE_GUARD_PROOF_SCHEMA_V1: &str = "nando.ms4-adaptive-guard-proof.v1";
pub const MS4_IN_SAMPLE_PHASE_ABLATION_SCHEMA_V1: &str = "nando.ms4-in-sample-phase-ablation.v1";
const MS4_EXTERNAL_ADMISSION_MAX_BYTES_V1: usize = 64 * 1024 * 1024;
const MS4_MAX_NEGATIVE_TOPOLOGIES_V1: usize = 64;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms4WaveControlOutcomeV1 {
    pub mode: String,
    pub positive_accepts: u64,
    pub negative_accepts: u64,
    pub correct_classifications: u64,
    pub support_margin_micro: Option<i64>,
    pub future_margin_micro: Option<i64>,
    pub max_negative_margin_micro: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms4InSamplePhaseAblationV1 {
    pub schema: String,
    pub proof_root_sha256: String,
    pub candidate_root_sha256: String,
    pub package_id: String,
    pub execution_payload_sha256: String,
    pub support_topology_root_sha256: String,
    pub future_topology_root_sha256: String,
    pub negative_topology_roots_sha256: Vec<String>,
    pub phase_threshold_micro: i64,
    pub full: Ms4WaveControlOutcomeV1,
    pub no_phase: Ms4WaveControlOutcomeV1,
    pub shuffled_phase: Ms4WaveControlOutcomeV1,
    pub magnitude_only: Ms4WaveControlOutcomeV1,
    pub random_center: Ms4WaveControlOutcomeV1,
    pub strict_all_ablation_pass: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms4AdaptiveGuardProofV1 {
    proof_root_sha256: String,
    support_binding_root_sha256: String,
    future_binding_root_sha256: String,
    negative_topology_roots_sha256: Vec<String>,
    negative_lineages_sha256: Vec<String>,
    negative_blockers: Vec<String>,
    phase_center_atom_ids: Vec<u64>,
    phase_anti_center_atom_ids: Vec<u64>,
    phase_threshold_micro: i64,
    support_margin_micro: i64,
    future_margin_micro: i64,
    max_negative_margin_micro: i64,
    no_anti_negative_accepts: u64,
    phase_ablation_pass: bool,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms4ExternalAdmissionCandidateV1 {
    schema: String,
    candidate_root_sha256: String,
    frozen: FrozenVersionSpaceEnvelopeV1,
    future: Ms3IndependentFutureEnvelopeV1,
    support_topology: PreActionTopologyAuditRowV1,
    support_frame: RelationFrame,
    support_terminal: TransportTerminalReceiptV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    support_transport_binding: Option<RequestActionBindingV1>,
    support_runtime_parity: RuntimeParityCase,
    future_topology: PreActionTopologyAuditRowV1,
    future_frame: RelationFrame,
    future_terminal: TransportTerminalReceiptV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    future_route_receipt: Option<NandoRouteReceiptV1>,
    future_runtime_parity: RuntimeParityCase,
    negative_topologies: Vec<PreActionTopologyAuditRowV1>,
    guard_proof: Ms4AdaptiveGuardProofV1,
    shadow_candidate: Ms4FrozenFutureShadowCandidateV1,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

impl Ms4ExternalAdmissionCandidateV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        frozen: FrozenVersionSpaceEnvelopeV1,
        future: Ms3IndependentFutureEnvelopeV1,
        support_topology: PreActionTopologyAuditRowV1,
        support_frame: RelationFrame,
        support_terminal: TransportTerminalReceiptV1,
        support_transport_binding: Option<RequestActionBindingV1>,
        support_runtime_parity: RuntimeParityCase,
        future_topology: PreActionTopologyAuditRowV1,
        future_frame: RelationFrame,
        future_terminal: TransportTerminalReceiptV1,
        future_route_receipt: Option<NandoRouteReceiptV1>,
        future_runtime_parity: RuntimeParityCase,
        mut negative_topologies: Vec<PreActionTopologyAuditRowV1>,
    ) -> Result<Self, &'static str> {
        negative_topologies.sort_by(|left, right| {
            left.commit
                .commitment_root_sha256
                .cmp(&right.commit.commitment_root_sha256)
        });
        negative_topologies.dedup_by(|left, right| {
            left.commit.commitment_root_sha256 == right.commit.commitment_root_sha256
        });
        let support = runtime_evidence(
            &frozen.contract.frame_root_sha256,
            &frozen.contract.session_lineage_sha256,
            &frozen.contract.topology_root_sha256,
            support_runtime_parity.clone(),
        );
        let future_evidence = runtime_evidence(
            &future.receipt.completed_frame_root_sha256,
            &future.receipt.session_lineage_sha256,
            &future.receipt.topology_root_sha256,
            future_runtime_parity.clone(),
        );
        let shadow_candidate =
            crystallize_ms4_frozen_future_shadow_v1(&frozen, &future, &support, &future_evidence)?;
        let guard_proof = build_guard_proof(
            &frozen,
            &future,
            &support_topology,
            &support_frame,
            &support_terminal,
            support_transport_binding.as_ref(),
            &future_topology,
            &future_frame,
            &future_terminal,
            future_route_receipt.as_ref(),
            &negative_topologies,
        )?;
        let mut candidate = Self {
            schema: MS4_EXTERNAL_ADMISSION_CANDIDATE_SCHEMA_V1.to_owned(),
            candidate_root_sha256: String::new(),
            frozen,
            future,
            support_topology,
            support_frame,
            support_terminal,
            support_transport_binding,
            support_runtime_parity,
            future_topology,
            future_frame,
            future_terminal,
            future_route_receipt,
            future_runtime_parity,
            negative_topologies,
            guard_proof,
            shadow_candidate,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        candidate.candidate_root_sha256 = candidate.expected_root()?;
        candidate.validate()?;
        Ok(candidate)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != MS4_EXTERNAL_ADMISSION_CANDIDATE_SCHEMA_V1
            || !valid_nonzero_sha256(&self.candidate_root_sha256)
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.future.receipt.verdict != Ms3IndependentFutureVerdictV1::Pass
        {
            return Err("ms4_external_candidate_contract_invalid");
        }
        self.frozen
            .validate()
            .map_err(|_| "ms4_external_frozen_invalid")?;
        self.future.validate(&self.frozen)?;
        validate_runtime_parity_binding(&self.support_frame, &self.support_runtime_parity)?;
        validate_runtime_parity_binding(&self.future_frame, &self.future_runtime_parity)?;
        let support = runtime_evidence(
            &self.frozen.contract.frame_root_sha256,
            &self.frozen.contract.session_lineage_sha256,
            &self.frozen.contract.topology_root_sha256,
            self.support_runtime_parity.clone(),
        );
        let future_evidence = runtime_evidence(
            &self.future.receipt.completed_frame_root_sha256,
            &self.future.receipt.session_lineage_sha256,
            &self.future.receipt.topology_root_sha256,
            self.future_runtime_parity.clone(),
        );
        let rebuilt_shadow = crystallize_ms4_frozen_future_shadow_v1(
            &self.frozen,
            &self.future,
            &support,
            &future_evidence,
        )?;
        if rebuilt_shadow != self.shadow_candidate {
            return Err("ms4_external_shadow_resynthesis_mismatch");
        }
        let rebuilt_guard = build_guard_proof(
            &self.frozen,
            &self.future,
            &self.support_topology,
            &self.support_frame,
            &self.support_terminal,
            self.support_transport_binding.as_ref(),
            &self.future_topology,
            &self.future_frame,
            &self.future_terminal,
            self.future_route_receipt.as_ref(),
            &self.negative_topologies,
        )?;
        if rebuilt_guard != self.guard_proof
            || self.candidate_root_sha256 != self.expected_root()?
        {
            return Err("ms4_external_candidate_resynthesis_mismatch");
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, &'static str> {
        self.validate()?;
        let bytes = serde_cbor::to_vec(self).map_err(|_| "ms4_external_candidate_encode_failed")?;
        if bytes.is_empty() || bytes.len() > MS4_EXTERNAL_ADMISSION_MAX_BYTES_V1 {
            return Err("ms4_external_candidate_byte_budget");
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.is_empty() || bytes.len() > MS4_EXTERNAL_ADMISSION_MAX_BYTES_V1 {
            return Err("ms4_external_candidate_byte_budget");
        }
        let candidate: Self =
            serde_cbor::from_slice(bytes).map_err(|_| "ms4_external_candidate_decode_failed")?;
        candidate.validate()?;
        if candidate.canonical_bytes()? != bytes {
            return Err("ms4_external_candidate_noncanonical");
        }
        Ok(candidate)
    }

    #[must_use]
    pub fn candidate_root_sha256(&self) -> &str {
        &self.candidate_root_sha256
    }

    #[must_use]
    pub fn future_envelope_root_sha256(&self) -> &str {
        &self.future.envelope_root_sha256
    }

    #[must_use]
    pub fn canonical_bundle_id_sha256(&self) -> &str {
        self.shadow_candidate.canonical_bundle_id_sha256()
    }

    #[must_use]
    pub fn canonical_program_root_sha256(&self) -> &str {
        self.future.receipt.canonical_program_root_sha256.as_str()
    }

    #[must_use]
    pub fn future_runtime_parity_case(&self) -> &RuntimeParityCase {
        &self.future_runtime_parity
    }

    #[must_use]
    pub fn future_terminal_receipt_root_sha256(&self) -> &str {
        &self.future_terminal.receipt_root_sha256
    }

    #[must_use]
    pub fn future_runtime_receipt_root_sha256(&self) -> &str {
        &self
            .shadow_candidate
            .future_runtime_receipt()
            .receipt_sha256
    }

    #[must_use]
    pub fn future_actor_response_sha256(&self) -> &str {
        &self
            .shadow_candidate
            .future_runtime_receipt()
            .actor_response_sha256
    }

    pub fn admitted_package(&self) -> Result<ResponsePackage, &'static str> {
        self.validate()?;
        admitted_package(&self.shadow_candidate, &self.guard_proof)
    }

    /// Diagnostic only: support, future, and threshold-calibration negatives
    /// are not an independent post-center holdout.
    pub fn in_sample_phase_ablation(&self) -> Result<Ms4InSamplePhaseAblationV1, &'static str> {
        self.validate()?;
        let package = admitted_package(&self.shadow_candidate, &self.guard_proof)?;
        let proof = build_in_sample_phase_ablation(
            &self.candidate_root_sha256,
            &package,
            &self.support_topology,
            &self.future_topology,
            &self.negative_topologies,
        )?;
        proof.validate()?;
        Ok(proof)
    }

    #[must_use]
    pub fn center_training_max_sequence(&self) -> u64 {
        self.negative_topologies
            .iter()
            .chain([&self.support_topology, &self.future_topology])
            .filter_map(|topology| topology.bridge_sequence)
            .max()
            .unwrap_or(0)
    }

    #[must_use]
    pub fn center_training_lineages(&self) -> BTreeSet<String> {
        self.negative_topologies
            .iter()
            .chain([&self.support_topology, &self.future_topology])
            .filter_map(|topology| topology.session_lineage_sha256.clone())
            .collect()
    }

    #[must_use]
    pub fn topology_negative_control_count(&self) -> usize {
        self.negative_topologies.len()
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        if self.support_transport_binding.is_some() || self.future_route_receipt.is_some() {
            return canonical_json_sha256(&(
                MS4_EXTERNAL_ADMISSION_CANDIDATE_SCHEMA_V1,
                self.frozen.envelope_root_sha256.as_str(),
                self.future.envelope_root_sha256.as_str(),
                canonical_json_sha256(&self.support_topology)?,
                canonical_json_sha256(&self.support_frame)?,
                (
                    self.support_terminal.receipt_root_sha256.as_str(),
                    self.support_transport_binding
                        .as_ref()
                        .map(|binding| binding.binding_root_sha256.as_str()),
                ),
                canonical_json_sha256(&self.support_runtime_parity)?,
                canonical_json_sha256(&self.future_topology)?,
                canonical_json_sha256(&self.future_frame)?,
                (
                    self.future_terminal.receipt_root_sha256.as_str(),
                    self.future_route_receipt
                        .as_ref()
                        .map(|receipt| receipt.receipt_root_sha256.as_str()),
                ),
                canonical_json_sha256(&self.future_runtime_parity)?,
                self.negative_topologies
                    .iter()
                    .map(|topology| topology.commit.commitment_root_sha256.as_str())
                    .collect::<Vec<_>>(),
                self.guard_proof.proof_root_sha256.as_str(),
                self.shadow_candidate.candidate_root_sha256(),
                false,
                false,
            ));
        }
        canonical_json_sha256(&(
            MS4_EXTERNAL_ADMISSION_CANDIDATE_SCHEMA_V1,
            self.frozen.envelope_root_sha256.as_str(),
            self.future.envelope_root_sha256.as_str(),
            canonical_json_sha256(&self.support_topology)?,
            canonical_json_sha256(&self.support_frame)?,
            self.support_terminal.receipt_root_sha256.as_str(),
            canonical_json_sha256(&self.support_runtime_parity)?,
            canonical_json_sha256(&self.future_topology)?,
            canonical_json_sha256(&self.future_frame)?,
            self.future_terminal.receipt_root_sha256.as_str(),
            canonical_json_sha256(&self.future_runtime_parity)?,
            self.negative_topologies
                .iter()
                .map(|topology| topology.commit.commitment_root_sha256.as_str())
                .collect::<Vec<_>>(),
            self.guard_proof.proof_root_sha256.as_str(),
            self.shadow_candidate.candidate_root_sha256(),
            false,
            false,
        ))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_ms4_external_admission_snapshot(
    candidates: &[Ms4ExternalAdmissionCandidateV1],
    project_id: &str,
    revision: u64,
    now_unix: u64,
    max_age_seconds: u64,
    gate_build_sha256: &str,
    runtime_build_sha256: &str,
) -> Result<Option<OnlineAdmissionSnapshot>, &'static str> {
    let mut packages = Vec::new();
    let mut receipts = BTreeMap::new();
    for candidate in candidates {
        candidate.validate()?;
        let package = candidate.admitted_package()?;
        let package_id = package.package_id.clone();
        let adaptive_root = package
            .proof
            .adaptive_identification
            .as_ref()
            .ok_or("ms4_external_adaptive_proof_missing")?
            .proof_root_sha256()
            .to_owned();
        let runtime_parity_root = canonical_json_sha256(&(
            "nando.ms4-runtime-parity-receipt-set.v1",
            candidate.shadow_candidate.support_runtime_receipt(),
            candidate.shadow_candidate.future_runtime_receipt(),
        ))?;
        receipts.insert(
            package_id,
            (
                candidate.frozen.contract.support_rows_root_sha256.clone(),
                candidate.guard_proof.proof_root_sha256.clone(),
                runtime_parity_root,
                candidate.future.receipt.receipt_root_sha256.clone(),
                adaptive_root,
            ),
        );
        packages.push(package);
    }
    if packages.is_empty() {
        return Ok(None);
    }
    packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    if packages
        .windows(2)
        .any(|pair| pair[0].package_id == pair[1].package_id)
    {
        return Err("ms4_external_duplicate_package_id");
    }
    let registry = ResponseRegistry {
        schema: RESPONSE_REGISTRY_SCHEMA_V6.to_owned(),
        revision,
        packages,
    };
    let admission = build_composite_admission_for_registry(
        &registry,
        receipts,
        project_id,
        now_unix,
        max_age_seconds,
        gate_build_sha256,
        runtime_build_sha256,
        "ms4_external_admission_receipts_missing",
        "ms4_external_admission_verifier_missing",
    )?;
    Ok(Some(OnlineAdmissionSnapshot {
        registry,
        admission,
    }))
}

#[allow(clippy::too_many_arguments)]
fn build_guard_proof(
    frozen: &FrozenVersionSpaceEnvelopeV1,
    future: &Ms3IndependentFutureEnvelopeV1,
    support_topology: &PreActionTopologyAuditRowV1,
    support_frame: &RelationFrame,
    support_terminal: &TransportTerminalReceiptV1,
    support_transport_binding: Option<&RequestActionBindingV1>,
    future_topology: &PreActionTopologyAuditRowV1,
    future_frame: &RelationFrame,
    future_terminal: &TransportTerminalReceiptV1,
    future_route_receipt: Option<&NandoRouteReceiptV1>,
    negative_topologies: &[PreActionTopologyAuditRowV1],
) -> Result<Ms4AdaptiveGuardProofV1, &'static str> {
    if negative_topologies.is_empty() || negative_topologies.len() > MS4_MAX_NEGATIVE_TOPOLOGIES_V1
    {
        return Err("ms4_external_negative_control_missing");
    }
    let support_binding = if let Some(binding) = support_transport_binding {
        validate_frozen_transport_partition(
            support_topology,
            support_frame,
            support_terminal,
            binding,
            &frozen.contract.topology_root_sha256,
            &frozen.contract.frame_root_sha256,
            &frozen.contract.terminal_root_sha256,
            &frozen.contract.transport_binding_root_sha256,
        )?
    } else {
        validate_transport_partition(
            support_topology,
            support_frame,
            support_terminal,
            &frozen.contract.topology_root_sha256,
            &frozen.contract.frame_root_sha256,
            &frozen.contract.terminal_root_sha256,
            &frozen.contract.transport_binding_root_sha256,
        )?
    };
    let future_binding = match (
        future.receipt.client_route_receipt_root_sha256.as_deref(),
        future.receipt.client_route_status,
        future_route_receipt,
    ) {
        (Some(expected_route_root), Some(418), Some(route_receipt)) => {
            if route_receipt.receipt_root_sha256 != expected_route_root {
                return Err("ms4_external_future_route_receipt_mismatch");
            }
            validate_fallback_transport_partition(
                future_topology,
                future_frame,
                future_terminal,
                route_receipt,
                &future.receipt.topology_root_sha256,
                &future.receipt.completed_frame_root_sha256,
                &future.receipt.terminal_receipt_root_sha256,
                &future.receipt.transport_binding_root_sha256,
            )?
        }
        (None, None, None) => validate_transport_partition(
            future_topology,
            future_frame,
            future_terminal,
            &future.receipt.topology_root_sha256,
            &future.receipt.completed_frame_root_sha256,
            &future.receipt.terminal_receipt_root_sha256,
            &future.receipt.transport_binding_root_sha256,
        )?,
        _ => return Err("ms4_external_future_route_receipt_missing"),
    };
    let machine =
        OperatorIdentificationMachineV1::from_checkpoint_bytes(frozen.machine_checkpoint())
            .map_err(|_| "ms4_external_identification_restore_failed")?;
    let freeze = machine.freeze().ok_or("ms4_external_freeze_missing")?;
    let program = machine
        .candidate_programs()
        .get(freeze.canonical_program_root_sha256())
        .cloned()
        .ok_or("ms4_external_program_missing")?;
    let support_role_binding =
        pre_action_t1_binding_root(&program, &support_topology.structure.topology)
            .map_err(|_| "ms4_external_support_guard_rejected")?;
    let future_role_binding =
        pre_action_t1_binding_root(&program, &future_topology.structure.topology)
            .map_err(|_| "ms4_external_future_guard_rejected")?;
    let mut negative_roots = Vec::with_capacity(negative_topologies.len());
    let mut negative_lineages = Vec::with_capacity(negative_topologies.len());
    let mut negative_blockers = Vec::with_capacity(negative_topologies.len());
    let mut seen_lineages = BTreeSet::new();
    for topology in negative_topologies {
        let sequence = topology
            .bridge_sequence
            .ok_or("ms4_external_negative_sequence_missing")?;
        let lineage = topology
            .session_lineage_sha256
            .as_ref()
            .ok_or("ms4_external_negative_lineage_missing")?;
        if sequence < frozen.contract.future_min_sequence
            || lineage == &frozen.contract.session_lineage_sha256
            || lineage == &future.receipt.session_lineage_sha256
            || !topology.structure.provider_bound_turn_identity
            || !topology.physical_order_proven
        {
            return Err("ms4_external_negative_partition_invalid");
        }
        let blocker = match classify_ms3_unique_law_v1(frozen, topology, 1)? {
            Ms3FutureApplicabilityV1::StructurallyNotApplicable { blocker } => blocker,
            _ => return Err("ms4_external_negative_not_separating"),
        };
        negative_roots.push(topology.commit.commitment_root_sha256.clone());
        negative_lineages.push(lineage.clone());
        negative_blockers.push(blocker.to_owned());
        seen_lineages.insert(lineage.as_str());
    }
    if negative_roots.windows(2).any(|pair| pair[0] >= pair[1]) || seen_lineages.is_empty() {
        return Err("ms4_external_negative_controls_invalid");
    }
    let phase_guard = build_phase_guard(
        &program,
        support_topology,
        future_topology,
        negative_topologies,
    )?;
    let mut proof = Ms4AdaptiveGuardProofV1 {
        proof_root_sha256: String::new(),
        support_binding_root_sha256: canonical_json_sha256(&(
            support_binding.as_str(),
            support_role_binding.as_str(),
        ))?,
        future_binding_root_sha256: canonical_json_sha256(&(
            future_binding.as_str(),
            future_role_binding.as_str(),
        ))?,
        negative_topology_roots_sha256: negative_roots,
        negative_lineages_sha256: negative_lineages,
        negative_blockers,
        phase_center_atom_ids: phase_guard.centers,
        phase_anti_center_atom_ids: phase_guard.anti_centers,
        phase_threshold_micro: phase_guard.threshold_micro,
        support_margin_micro: phase_guard.support_margin_micro,
        future_margin_micro: phase_guard.future_margin_micro,
        max_negative_margin_micro: phase_guard.max_negative_margin_micro,
        no_anti_negative_accepts: phase_guard.no_anti_negative_accepts,
        phase_ablation_pass: true,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    proof.proof_root_sha256 = proof.expected_root()?;
    proof.validate()?;
    Ok(proof)
}

impl Ms4AdaptiveGuardProofV1 {
    fn validate(&self) -> Result<(), &'static str> {
        if !valid_nonzero_sha256(&self.proof_root_sha256)
            || !valid_nonzero_sha256(&self.support_binding_root_sha256)
            || !valid_nonzero_sha256(&self.future_binding_root_sha256)
            || self.negative_topology_roots_sha256.is_empty()
            || self.negative_topology_roots_sha256.len() > MS4_MAX_NEGATIVE_TOPOLOGIES_V1
            || self.negative_topology_roots_sha256.len() != self.negative_lineages_sha256.len()
            || self.negative_topology_roots_sha256.len() != self.negative_blockers.len()
            || self.phase_center_atom_ids.is_empty()
            || self.phase_anti_center_atom_ids.is_empty()
            || self
                .negative_topology_roots_sha256
                .iter()
                .chain(&self.negative_lineages_sha256)
                .any(|root| !valid_nonzero_sha256(root))
            || !self
                .negative_topology_roots_sha256
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || !self
                .phase_center_atom_ids
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || !self
                .phase_anti_center_atom_ids
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || self.phase_threshold_micro <= 0
            || self.support_margin_micro < self.phase_threshold_micro
            || self.future_margin_micro < self.phase_threshold_micro
            || self.max_negative_margin_micro >= self.phase_threshold_micro
            || self.no_anti_negative_accepts == 0
            || !self.phase_ablation_pass
            || self.negative_blockers.iter().any(String::is_empty)
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.proof_root_sha256 != self.expected_root()?
        {
            return Err("ms4_external_guard_proof_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            MS4_ADAPTIVE_GUARD_PROOF_SCHEMA_V1,
            self.support_binding_root_sha256.as_str(),
            self.future_binding_root_sha256.as_str(),
            &self.negative_topology_roots_sha256,
            &self.negative_lineages_sha256,
            &self.negative_blockers,
            &self.phase_center_atom_ids,
            &self.phase_anti_center_atom_ids,
            self.phase_threshold_micro,
            self.support_margin_micro,
            self.future_margin_micro,
            self.max_negative_margin_micro,
            self.no_anti_negative_accepts,
            true,
            false,
            false,
        ))
    }
}

impl Ms4InSamplePhaseAblationV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        let negative_rows = u64::try_from(self.negative_topology_roots_sha256.len())
            .map_err(|_| "ms4_in_sample_phase_ablation_invalid")?;
        let total_rows = negative_rows.saturating_add(2);
        let valid_control = |control: &Ms4WaveControlOutcomeV1, mode: &str| {
            control.mode == mode
                && control.positive_accepts <= 2
                && control.negative_accepts <= negative_rows
                && control.correct_classifications
                    == control
                        .positive_accepts
                        .saturating_add(negative_rows.saturating_sub(control.negative_accepts))
        };
        if self.schema != MS4_IN_SAMPLE_PHASE_ABLATION_SCHEMA_V1
            || !valid_nonzero_sha256(&self.proof_root_sha256)
            || !valid_nonzero_sha256(&self.candidate_root_sha256)
            || self.package_id.is_empty()
            || !valid_nonzero_sha256(&self.execution_payload_sha256)
            || !valid_nonzero_sha256(&self.support_topology_root_sha256)
            || !valid_nonzero_sha256(&self.future_topology_root_sha256)
            || self.negative_topology_roots_sha256.is_empty()
            || self
                .negative_topology_roots_sha256
                .iter()
                .any(|root| !valid_nonzero_sha256(root))
            || !self
                .negative_topology_roots_sha256
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || self.phase_threshold_micro <= 0
            || !valid_control(&self.full, "full")
            || !valid_control(&self.no_phase, "no_phase")
            || !valid_control(&self.shuffled_phase, "shuffled_phase")
            || !valid_control(&self.magnitude_only, "magnitude_only")
            || !valid_control(&self.random_center, "random_center")
            || self.full.positive_accepts != 2
            || self.full.negative_accepts != 0
            || self.full.correct_classifications != total_rows
            || self.full.support_margin_micro.is_none()
            || self.full.future_margin_micro.is_none()
            || self.full.max_negative_margin_micro.is_none()
            || self.no_phase.support_margin_micro.is_some()
            || self.no_phase.future_margin_micro.is_some()
            || self.no_phase.max_negative_margin_micro.is_some()
            || self.shuffled_phase.support_margin_micro.is_none()
            || self.shuffled_phase.future_margin_micro.is_none()
            || self.shuffled_phase.max_negative_margin_micro.is_none()
            || self.magnitude_only.support_margin_micro.is_none()
            || self.magnitude_only.future_margin_micro.is_none()
            || self.magnitude_only.max_negative_margin_micro.is_none()
            || self.random_center.support_margin_micro.is_none()
            || self.random_center.future_margin_micro.is_none()
            || self.random_center.max_negative_margin_micro.is_none()
            || self.no_phase.correct_classifications >= total_rows
            || self.shuffled_phase.correct_classifications >= total_rows
            || self.magnitude_only.correct_classifications >= total_rows
            || self.random_center.correct_classifications >= total_rows
            || self.proof_root_sha256 != self.expected_root()?
        {
            return Err("ms4_in_sample_phase_ablation_invalid");
        }
        if !self.strict_all_ablation_pass {
            return Err("ms4_in_sample_phase_ablation_failed");
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, &'static str> {
        self.validate()?;
        serde_cbor::to_vec(self).map_err(|_| "ms4_in_sample_phase_ablation_encode_failed")
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let proof: Self = serde_cbor::from_slice(bytes)
            .map_err(|_| "ms4_in_sample_phase_ablation_decode_failed")?;
        proof.validate()?;
        if proof.canonical_bytes()? != bytes {
            return Err("ms4_in_sample_phase_ablation_noncanonical");
        }
        Ok(proof)
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            MS4_IN_SAMPLE_PHASE_ABLATION_SCHEMA_V1,
            self.candidate_root_sha256.as_str(),
            self.package_id.as_str(),
            self.execution_payload_sha256.as_str(),
            self.support_topology_root_sha256.as_str(),
            self.future_topology_root_sha256.as_str(),
            &self.negative_topology_roots_sha256,
            self.phase_threshold_micro,
            &self.full,
            &self.no_phase,
            &self.shuffled_phase,
            &self.magnitude_only,
            &self.random_center,
            self.strict_all_ablation_pass,
        ))
    }
}

#[derive(Clone, Copy)]
enum InSamplePhaseControlV1 {
    Full,
    ShuffledPhase,
    MagnitudeOnly,
    RandomCenter,
}

fn build_in_sample_phase_ablation(
    candidate_root_sha256: &str,
    package: &ResponsePackage,
    support: &PreActionTopologyAuditRowV1,
    future: &PreActionTopologyAuditRowV1,
    negatives: &[PreActionTopologyAuditRowV1],
) -> Result<Ms4InSamplePhaseAblationV1, &'static str> {
    let mut required =
        nando_operator_kernel::response_program_required_routing_atom_ids(&package.program);
    required.sort_unstable();
    required.dedup();
    let support_atoms = topology_runtime_atoms(support, &required);
    let future_atoms = topology_runtime_atoms(future, &required);
    let negative_atoms = negatives
        .iter()
        .map(|topology| topology_runtime_atoms(topology, &required))
        .collect::<Vec<_>>();
    let positive = phase_vector_from_atom_ids(package.phase_centers.iter().copied(), 16);
    let negative = phase_vector_from_atom_ids(package.anti_centers.iter().copied(), 16);
    let full = evaluate_in_sample_phase_control(
        "full",
        InSamplePhaseControlV1::Full,
        &support_atoms,
        &future_atoms,
        &negative_atoms,
        &positive,
        &negative,
        package.wave_margin_micro,
    )?;
    let negative_rows = u64::try_from(negative_atoms.len()).unwrap_or(u64::MAX);
    let no_phase = Ms4WaveControlOutcomeV1 {
        mode: "no_phase".to_owned(),
        positive_accepts: 2,
        negative_accepts: negative_rows,
        correct_classifications: 2,
        support_margin_micro: None,
        future_margin_micro: None,
        max_negative_margin_micro: None,
    };
    let shuffled_phase = evaluate_in_sample_phase_control(
        "shuffled_phase",
        InSamplePhaseControlV1::ShuffledPhase,
        &support_atoms,
        &future_atoms,
        &negative_atoms,
        &positive,
        &negative,
        package.wave_margin_micro,
    )?;
    let magnitude_only = evaluate_in_sample_phase_control(
        "magnitude_only",
        InSamplePhaseControlV1::MagnitudeOnly,
        &support_atoms,
        &future_atoms,
        &negative_atoms,
        &positive,
        &negative,
        package.wave_margin_micro,
    )?;
    let random_center = evaluate_in_sample_phase_control(
        "random_center",
        InSamplePhaseControlV1::RandomCenter,
        &support_atoms,
        &future_atoms,
        &negative_atoms,
        &positive,
        &negative,
        package.wave_margin_micro,
    )?;
    let full_correct = full.correct_classifications;
    let strict_all_ablation_pass = full.positive_accepts == 2
        && full.negative_accepts == 0
        && [&no_phase, &shuffled_phase, &magnitude_only, &random_center]
            .into_iter()
            .all(|control| control.correct_classifications < full_correct);
    let mut negative_roots = negatives
        .iter()
        .map(|topology| topology.commit.commitment_root_sha256.clone())
        .collect::<Vec<_>>();
    negative_roots.sort();
    negative_roots.dedup();
    let mut proof = Ms4InSamplePhaseAblationV1 {
        schema: MS4_IN_SAMPLE_PHASE_ABLATION_SCHEMA_V1.to_owned(),
        proof_root_sha256: String::new(),
        candidate_root_sha256: candidate_root_sha256.to_owned(),
        package_id: package.package_id.clone(),
        execution_payload_sha256: response_execution_payload_digest(package)?,
        support_topology_root_sha256: support.commit.commitment_root_sha256.clone(),
        future_topology_root_sha256: future.commit.commitment_root_sha256.clone(),
        negative_topology_roots_sha256: negative_roots,
        phase_threshold_micro: package.wave_margin_micro,
        full,
        no_phase,
        shuffled_phase,
        magnitude_only,
        random_center,
        strict_all_ablation_pass,
    };
    proof.proof_root_sha256 = proof.expected_root()?;
    Ok(proof)
}

#[allow(clippy::too_many_arguments)]
fn evaluate_in_sample_phase_control(
    mode_name: &str,
    mode: InSamplePhaseControlV1,
    support_atoms: &[u64],
    future_atoms: &[u64],
    negative_atoms: &[Vec<u64>],
    positive: &[PhaseCenterCell],
    negative: &[PhaseCenterCell],
    threshold_micro: i64,
) -> Result<Ms4WaveControlOutcomeV1, &'static str> {
    let margin = |atoms: &[u64]| in_sample_phase_control_margin(atoms, positive, negative, mode);
    let support_margin_micro = margin(support_atoms)?;
    let future_margin_micro = margin(future_atoms)?;
    let negative_margins = negative_atoms
        .iter()
        .map(|atoms| margin(atoms))
        .collect::<Result<Vec<_>, _>>()?;
    let max_negative_margin_micro = negative_margins
        .iter()
        .copied()
        .max()
        .ok_or("ms4_in_sample_negative_control_missing")?;
    let positive_accepts = u64::from(support_margin_micro >= threshold_micro)
        .saturating_add(u64::from(future_margin_micro >= threshold_micro));
    let negative_accepts = u64::try_from(
        negative_margins
            .iter()
            .filter(|margin| **margin >= threshold_micro)
            .count(),
    )
    .unwrap_or(u64::MAX);
    let negative_rows = u64::try_from(negative_atoms.len()).unwrap_or(u64::MAX);
    Ok(Ms4WaveControlOutcomeV1 {
        mode: mode_name.to_owned(),
        positive_accepts,
        negative_accepts,
        correct_classifications: positive_accepts
            .saturating_add(negative_rows.saturating_sub(negative_accepts)),
        support_margin_micro: Some(support_margin_micro),
        future_margin_micro: Some(future_margin_micro),
        max_negative_margin_micro: Some(max_negative_margin_micro),
    })
}

fn in_sample_phase_control_margin(
    atoms: &[u64],
    positive: &[PhaseCenterCell],
    negative: &[PhaseCenterCell],
    mode: InSamplePhaseControlV1,
) -> Result<i64, &'static str> {
    let query = phase_vector_from_atom_ids(atoms.iter().copied(), positive.len());
    let score = match mode {
        InSamplePhaseControlV1::Full => {
            phase_coherence(&query, positive) - phase_coherence(&query, negative)
        }
        InSamplePhaseControlV1::ShuffledPhase => {
            let mut shuffled_positive = positive.to_vec();
            let mut shuffled_negative = negative.to_vec();
            let shift = (positive.len() / 3).max(1);
            shuffled_positive.rotate_left(shift);
            shuffled_negative.rotate_left(shift);
            phase_coherence(&query, &shuffled_positive)
                - phase_coherence(&query, &shuffled_negative)
        }
        InSamplePhaseControlV1::MagnitudeOnly => {
            magnitude_coherence(&query, positive) - magnitude_coherence(&query, negative)
        }
        InSamplePhaseControlV1::RandomCenter => {
            let random_positive = matched_random_center(positive);
            let random_negative = matched_random_center(negative);
            phase_coherence(&query, &random_positive) - phase_coherence(&query, &random_negative)
        }
    };
    phase_margin_to_micro(score).map_err(|_| "ms4_in_sample_phase_margin_invalid")
}

fn magnitude_coherence(left: &[PhaseCenterCell], right: &[PhaseCenterCell]) -> f64 {
    let cells = left.len().min(right.len());
    if cells == 0 {
        return 0.0;
    }
    left.iter()
        .zip(right)
        .map(|(left, right)| left.re.hypot(left.im) * right.re.hypot(right.im))
        .sum::<f64>()
        / cells as f64
}

fn matched_random_center(center: &[PhaseCenterCell]) -> Vec<PhaseCenterCell> {
    center
        .iter()
        .rev()
        .enumerate()
        .map(|(index, cell)| {
            let sign = if index % 2 == 0 { 1.0 } else { -1.0 };
            PhaseCenterCell {
                re: cell.im * sign,
                im: cell.re * -sign,
            }
        })
        .collect()
}

fn validate_transport_partition(
    topology: &PreActionTopologyAuditRowV1,
    frame: &RelationFrame,
    terminal: &TransportTerminalReceiptV1,
    expected_topology_root: &str,
    expected_frame_root: &str,
    expected_terminal_root: &str,
    expected_binding_root: &str,
) -> Result<String, &'static str> {
    if topology.commit.commitment_root_sha256 != expected_topology_root
        || canonical_json_sha256(frame)? != expected_frame_root
        || terminal.receipt_root_sha256 != expected_terminal_root
        || !terminal.validate()
    {
        return Err("ms4_external_transport_partition_root_mismatch");
    }
    let ledger = TransportBindingLedgerV1::build(
        std::slice::from_ref(topology),
        std::slice::from_ref(frame),
        std::slice::from_ref(terminal),
    );
    let [bound] = ledger.bound_for_topology(expected_topology_root) else {
        return Err("ms4_external_transport_partition_unbound");
    };
    if bound.binding.binding_root_sha256 != expected_binding_root {
        return Err("ms4_external_transport_binding_mismatch");
    }
    Ok(bound.binding.binding_root_sha256.clone())
}

#[allow(clippy::too_many_arguments)]
fn validate_frozen_transport_partition(
    topology: &PreActionTopologyAuditRowV1,
    frame: &RelationFrame,
    terminal: &TransportTerminalReceiptV1,
    binding: &RequestActionBindingV1,
    expected_topology_root: &str,
    expected_frame_root: &str,
    expected_terminal_root: &str,
    expected_binding_root: &str,
) -> Result<String, &'static str> {
    if topology.commit.commitment_root_sha256 != expected_topology_root
        || canonical_json_sha256(frame)? != expected_frame_root
        || terminal.receipt_root_sha256 != expected_terminal_root
        || binding.binding_root_sha256 != expected_binding_root
    {
        return Err("ms4_external_frozen_partition_root_mismatch");
    }
    validate_request_action_binding_v1(topology, frame, terminal, binding)
        .map_err(|_| "ms4_external_frozen_transport_binding_mismatch")?;
    Ok(binding.binding_root_sha256.clone())
}

#[allow(clippy::too_many_arguments)]
fn validate_fallback_transport_partition(
    topology: &PreActionTopologyAuditRowV1,
    frame: &RelationFrame,
    terminal: &TransportTerminalReceiptV1,
    route_receipt: &NandoRouteReceiptV1,
    expected_topology_root: &str,
    expected_frame_root: &str,
    expected_terminal_root: &str,
    expected_binding_root: &str,
) -> Result<String, &'static str> {
    if topology.commit.commitment_root_sha256 != expected_topology_root
        || canonical_json_sha256(frame)? != expected_frame_root
        || terminal.receipt_root_sha256 != expected_terminal_root
        || !terminal.validate()
        || !route_receipt.validate()
    {
        return Err("ms4_external_fallback_partition_root_mismatch");
    }
    let bound = bind_independent_fallback_transition_v1(topology, frame, terminal, route_receipt)?;
    if bound.binding.binding_root_sha256 != expected_binding_root {
        return Err("ms4_external_fallback_transport_binding_mismatch");
    }
    Ok(bound.binding.binding_root_sha256)
}

fn validate_runtime_parity_binding(
    frame: &RelationFrame,
    parity: &RuntimeParityCase,
) -> Result<(), &'static str> {
    if parity.evidence_ref_sha256 != frame.frame_id_sha256
        || parity.request_text.is_empty()
        || parity.expected_response.is_empty()
    {
        return Err("ms4_external_runtime_parity_binding_invalid");
    }
    if let Some(receipt) = &parity.capture_receipt {
        receipt.validate()?;
        if receipt
            .transition_binding
            .as_ref()
            .is_some_and(|binding| binding.frame_id_sha256 != frame.frame_id_sha256)
        {
            return Err("ms4_external_runtime_capture_binding_invalid");
        }
    }
    Ok(())
}

fn admitted_package(
    shadow: &Ms4FrozenFutureShadowCandidateV1,
    guard: &Ms4AdaptiveGuardProofV1,
) -> Result<ResponsePackage, &'static str> {
    shadow.validate()?;
    guard.validate()?;
    let mut package = shadow.package().clone();
    package.state = ResponsePackageState::Active;
    package.package_id = format!(
        "ms4-natural-{}-{}",
        shadow.canonical_bundle_id_sha256(),
        guard.proof_root_sha256
    );
    package
        .phase_centers
        .clone_from(&guard.phase_center_atom_ids);
    package
        .anti_centers
        .clone_from(&guard.phase_anti_center_atom_ids);
    package.wave_margin_micro = guard.phase_threshold_micro;
    package.proof.wave_causal_pass = guard.phase_ablation_pass;
    package.validate()?;
    if package.admission_candidate_blocker().is_some() {
        return Err("ms4_external_admitted_package_ineligible");
    }
    Ok(package)
}

fn runtime_evidence(
    frame_root: &str,
    lineage_root: &str,
    topology_root: &str,
    parity: RuntimeParityCase,
) -> Ms4RuntimeEvidenceV1 {
    Ms4RuntimeEvidenceV1 {
        source_frame_root_sha256: frame_root.to_owned(),
        session_lineage_sha256: lineage_root.to_owned(),
        surface_sha256: topology_root.to_owned(),
        parity,
    }
}

struct PhaseGuardV1 {
    centers: Vec<u64>,
    anti_centers: Vec<u64>,
    threshold_micro: i64,
    support_margin_micro: i64,
    future_margin_micro: i64,
    max_negative_margin_micro: i64,
    no_anti_negative_accepts: u64,
}

fn build_phase_guard(
    program: &nando_operator_kernel::ResponseProgram,
    support: &PreActionTopologyAuditRowV1,
    future: &PreActionTopologyAuditRowV1,
    negatives: &[PreActionTopologyAuditRowV1],
) -> Result<PhaseGuardV1, &'static str> {
    let mut required = nando_operator_kernel::response_program_required_routing_atom_ids(program);
    required.sort_unstable();
    required.dedup();
    let support_atoms = topology_runtime_atoms(support, &required);
    let future_atoms = topology_runtime_atoms(future, &required);
    let mut centers = support_atoms
        .iter()
        .chain(&future_atoms)
        .copied()
        .collect::<Vec<_>>();
    centers.sort_unstable();
    centers.dedup();
    let mut anti_centers = negatives
        .iter()
        .flat_map(|topology| topology_runtime_atoms(topology, &required))
        .filter(|atom| centers.binary_search(atom).is_err())
        .collect::<Vec<_>>();
    anti_centers.sort_unstable();
    anti_centers.dedup();
    if centers.is_empty() || anti_centers.is_empty() {
        return Err("ms4_external_phase_guard_not_identified");
    }
    let margin = |atoms: &[u64], include_anti: bool| {
        let query = phase_vector_from_atom_ids(atoms.iter().copied(), 16);
        let positive = phase_vector_from_atom_ids(centers.iter().copied(), 16);
        let positive_margin = phase_coherence(&query, &positive);
        let margin = if include_anti {
            positive_margin
                - phase_coherence(
                    &query,
                    &phase_vector_from_atom_ids(anti_centers.iter().copied(), 16),
                )
        } else {
            positive_margin
        };
        phase_margin_to_micro(margin).map_err(|_| "ms4_external_phase_margin_invalid")
    };
    let support_margin_micro = margin(&support_atoms, true)?;
    let future_margin_micro = margin(&future_atoms, true)?;
    let negative_atoms = negatives
        .iter()
        .map(|topology| topology_runtime_atoms(topology, &required))
        .collect::<Vec<_>>();
    let negative_margins = negative_atoms
        .iter()
        .map(|atoms| margin(atoms, true))
        .collect::<Result<Vec<_>, _>>()?;
    let max_negative_margin_micro = negative_margins
        .iter()
        .copied()
        .max()
        .ok_or("ms4_external_negative_control_missing")?;
    let threshold_micro = max_negative_margin_micro
        .checked_add(1)
        .ok_or("ms4_external_phase_threshold_invalid")?
        .max(1);
    if support_margin_micro < threshold_micro || future_margin_micro < threshold_micro {
        return Err("ms4_external_phase_positive_separation_failed");
    }
    let no_anti_negative_accepts = negative_atoms
        .iter()
        .map(|atoms| margin(atoms, false))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|margin| *margin >= threshold_micro)
        .count() as u64;
    if no_anti_negative_accepts == 0 {
        return Err("ms4_external_phase_ablation_failed");
    }
    Ok(PhaseGuardV1 {
        centers,
        anti_centers,
        threshold_micro,
        support_margin_micro,
        future_margin_micro,
        max_negative_margin_micro,
        no_anti_negative_accepts,
    })
}

fn topology_runtime_atoms(topology: &PreActionTopologyAuditRowV1, required: &[u64]) -> Vec<u64> {
    let mut atoms = topology
        .structure
        .request_phase_atom_ids
        .iter()
        .chain(&topology.structure.pre_action_context_atom_ids)
        .chain(required)
        .copied()
        .collect::<Vec<_>>();
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}
