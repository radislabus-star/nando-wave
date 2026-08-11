use nando_core::wave::{
    PhaseCenterCell, phase_coherence, phase_margin_to_micro, phase_vector_from_atom_ids,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::authority::{
    CompositeResponseAdmissionV2, FinalizedRuntimeVerificationReceiptV2,
    IndependentlyVerifiedExecution, ValidatedResponseAuthority, canonical_json_sha256,
    finalize_runtime_receipt, response_registry_digest, valid_nonzero_sha256,
    validate_response_authority,
};
#[cfg(test)]
use crate::contracts::canonical_response_value_selector;
use crate::runtime::immediate_selected_scalar;

use crate::{
    AtomValueType, BoundCrystallizedOperator, RelationAtom, RelationFrame, ResponseArgument,
    ResponseExecutionStatus, ResponseOperation, ResponseProgram, ResponseValueSelector,
    SemanticRole, VerifiedCrystallizedOperator, VerifiedOperatorRestartBundle, VerifierProgram,
    execute_response, verify_response_independently,
};

use nando_operator_admission::OperatorCertificationLedgerV1;
use nando_operator_learning::{
    AvailableActionContractsV1, DecisionAuthoritySnapshotV1, K1ActionContractProjectionV1,
    OpaqueActionExecutionBindingV1,
};

#[cfg(test)]
use nando_operator_kernel::{
    AtomSource, balanced_wave_hidden_atom_ids, ranked_wave_pairs, ranked_wave_triples,
    relation_atom_phase_id, relation_frame_hidden_wave_atom_ids,
};

pub use nando_operator_admission::{
    LEGACY_CONTROL_FUTURE_ROWS, LEGACY_CONTROL_MIN_SESSIONS, LEGACY_CONTROL_MIN_SURFACES,
    LEGACY_CONTROL_SUPPORT_ROWS, LearnedWaveRoute, LearnedWaveSubcenter, ResponsePackageOrigin,
    ResponsePackageProof, ResponsePackageState, ResponseRoutingComparison,
    ResponseRoutingPredicate,
};
pub use nando_operator_kernel::{
    relation_frame_online_routing_atom_ids, relation_frame_phase_atom_ids,
    relation_frame_routing_atom_ids, response_program_required_routing_atom_ids,
};

pub const CONTINUATION_EXTERNAL_VERIFIER_SCHEMA: &str = "continue_handle_external_evidence.v1";
pub const SOURCE_VALUE_EXTERNAL_VERIFIER_SCHEMA: &str = "source_value_external_evidence.v1";
pub const CUSTOM_TOOL_EXTERNAL_VERIFIER_SCHEMA: &str = "custom_tool_external_evidence.v1";
pub const VALUE_PROJECTION_EXTERNAL_VERIFIER_SCHEMA: &str = "value_projection_external_evidence.v1";
pub const STATUS_PROJECTION_EXTERNAL_VERIFIER_SCHEMA: &str =
    "status_projection_external_evidence.v1";
pub const COLLECTION_EXTERNAL_VERIFIER_SCHEMA: &str = "collection_program_external_evidence.v1";
pub const PLAN_ADVANCE_EXTERNAL_VERIFIER_SCHEMA: &str = "plan_advance_external_evidence.v1";

fn response_operation_label(program: &ResponseProgram) -> &'static str {
    match &program.operation {
        ResponseOperation::ProjectSelectedValue { .. } => "project",
        ResponseOperation::ProjectStatus { .. } => "status",
        ResponseOperation::ComposeCollection { .. } => "collection",
        _ => "other",
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponsePackage {
    pub schema: String,
    pub package_id: String,
    pub origin: ResponsePackageOrigin,
    pub state: ResponsePackageState,
    pub program: ResponseProgram,
    #[serde(default)]
    pub verifier: Option<VerifierProgram>,
    #[serde(default)]
    pub routing_predicates: Vec<ResponseRoutingPredicate>,
    #[serde(default)]
    pub required_routing_atom_ids: Vec<u64>,
    pub phase_centers: Vec<u64>,
    pub anti_centers: Vec<u64>,
    pub wave_margin_micro: i64,
    #[serde(default)]
    pub learned_wave_route: Option<LearnedWaveRoute>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crystallized_operator: Option<VerifiedOperatorRestartBundle>,
    pub proof: ResponsePackageProof,
}

impl ResponsePackage {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != "nando.response-package.v1" {
            return Err("unsupported_package_schema");
        }
        if self.package_id.is_empty() {
            return Err("empty_package_id");
        }
        self.program.validate()?;
        if let Some(bundle) = &self.crystallized_operator {
            let operator = bundle
                .restore_verified()
                .map_err(|_| "crystallized_operator_restore_failed")?;
            let actor_sha256 = crate::response_actor_program_digest(&self.program)
                .map_err(|_| "crystallized_actor_digest_failed")?;
            if actor_sha256 != operator.actor_sha256() {
                return Err("crystallized_actor_commitment_mismatch");
            }
            let verifier = self
                .verifier
                .as_ref()
                .ok_or("crystallized_verifier_missing")?;
            let verifier_sha256 = crate::response_independent_verifier_program_digest(verifier)
                .map_err(|_| "crystallized_verifier_digest_failed")?;
            if verifier_sha256 != operator.verifier_sha256()
                || operator.parity_seal().wrong_accepts() != 0
            {
                return Err("crystallized_verifier_commitment_mismatch");
            }
        }
        if let Some(proof) = &self.proof.adaptive_identification {
            proof.validate()?;
            let program_root =
                nando_operator_kernel::response_program_version_root_sha256(&self.program)
                    .map_err(|_| "adaptive_identification_program_digest_failed")?;
            if proof.canonical_program_root_sha256() != program_root {
                return Err("adaptive_identification_program_root_mismatch");
            }
            if self.crystallized_operator.is_none()
                && !response_program_verifier_matches(&self.program, self.verifier.as_ref())
            {
                return Err("adaptive_identification_verifier_not_bound");
            }
        }
        if matches!(
            self.program.operation,
            ResponseOperation::ProjectStatus { .. }
        ) {
            if self.proof.verifier_schema != STATUS_PROJECTION_EXTERNAL_VERIFIER_SCHEMA {
                return Err("status_projection_external_evidence_required");
            }
            if !response_program_verifier_matches(&self.program, self.verifier.as_ref()) {
                return Err("status_projection_actor_verifier_mismatch");
            }
        }
        if matches!(
            self.program.operation,
            ResponseOperation::ComposeCollection { .. }
        ) && (self.proof.verifier_schema != COLLECTION_EXTERNAL_VERIFIER_SCHEMA
            || !response_program_verifier_matches(&self.program, self.verifier.as_ref()))
        {
            return Err("collection_external_evidence_required");
        }
        if matches!(
            self.program.operation,
            ResponseOperation::AdvancePlan { .. }
        ) && (self.proof.verifier_schema != PLAN_ADVANCE_EXTERNAL_VERIFIER_SCHEMA
            || !response_program_verifier_matches(&self.program, self.verifier.as_ref()))
        {
            return Err("plan_advance_external_evidence_required");
        }
        if self
            .routing_predicates
            .iter()
            .any(|predicate| predicate.role.is_empty())
        {
            return Err("empty_routing_predicate_role");
        }
        if self.routing_predicates.iter().any(|predicate| {
            let ordered_unique = predicate
                .allowed_counts
                .windows(2)
                .all(|pair| pair[0] < pair[1]);
            match predicate.comparison {
                ResponseRoutingComparison::OneOf => {
                    predicate.allowed_counts.is_empty() || !ordered_unique
                }
                ResponseRoutingComparison::AtMost | ResponseRoutingComparison::AtLeast => {
                    !predicate.allowed_counts.is_empty()
                }
            }
        }) {
            return Err("invalid_routing_predicate_values");
        }
        if !self
            .required_routing_atom_ids
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        {
            return Err("invalid_required_routing_atoms");
        }
        if self.phase_centers.is_empty() {
            return Err("phase_centers_missing");
        }
        if self.wave_margin_micro <= 0 {
            return Err("wave_margin_missing");
        }
        if let Some(route) = &self.learned_wave_route
            && (route.cells == 0
                || route.cells > 256
                || route.center_delta_micro.len() != usize::from(route.cells) * 2
                || route.threshold_micro <= 0
                || route.subcenters.len() > 7
                || route.subcenters.iter().any(|subcenter| {
                    subcenter.center_delta_micro.len() != usize::from(route.cells) * 2
                        || subcenter.threshold_micro <= 0
                }))
        {
            return Err("learned_wave_route_invalid");
        }
        if let Some(route) = &self.learned_wave_route
            && (route.query_atom_ids.len() > 256
                || !route
                    .query_atom_ids
                    .windows(2)
                    .all(|pair| pair[0] < pair[1]))
        {
            return Err("learned_wave_query_atoms_invalid");
        }
        Ok(())
    }

    #[must_use]
    pub fn eligible_for_admission_candidate(&self) -> bool {
        self.admission_candidate_blocker().is_none()
    }

    #[must_use]
    pub fn admission_candidate_blocker(&self) -> Option<&'static str> {
        let grounded_authority = self.origin == ResponsePackageOrigin::GroundedSynthesis
            && matches!(
                self.program.operation,
                ResponseOperation::UniqueConsensus { .. }
                    | ResponseOperation::AdvancePlan { .. }
                    | ResponseOperation::FunctionCallFromRoles { .. }
                    | ResponseOperation::CustomToolCallFromRoles { .. }
                    | ResponseOperation::ProjectSelectedValue { .. }
                    | ResponseOperation::ProjectStatus { .. }
                    | ResponseOperation::ComposeCollection { .. }
            );
        let verifier_program_bound =
            response_program_verifier_matches(&self.program, self.verifier.as_ref());
        let verifier_bound = response_program_external_verifier_schema(&self.program)
            .is_some_and(|schema| self.proof.verifier_schema == schema);
        let adaptive_identification_bound = self
            .proof
            .adaptive_identification
            .as_ref()
            .is_some_and(|proof| proof.validate().is_ok());
        let semantic_applicability_guard_bound =
            !nando_operator_kernel::response_program_requires_semantic_applicability_guard(
                &self.program,
            ) || (adaptive_identification_bound && !self.anti_centers.is_empty());
        let required_atoms = response_program_required_routing_atom_ids(&self.program);
        let exact_guard_bound = if let Some(bundle) = &self.crystallized_operator {
            // Runtime always takes the crystallized branch first. Legacy phase
            // atoms may remain as frozen learning evidence, but they cannot
            // route around the sealed RoleGraph and RelationProgram.
            bundle.restore_verified().is_ok()
        } else {
            !required_atoms.is_empty()
                && required_atoms
                    .iter()
                    .all(|atom| self.required_routing_atom_ids.binary_search(atom).is_ok())
        };
        nando_operator_admission::package_admission_candidate_blocker(
            nando_operator_admission::PackageAdmissionFacts {
                validation_blocker: self.validate().err(),
                grounded_authority,
                package_active: self.state == ResponsePackageState::Active,
                support_rows: self.proof.support_rows,
                future_rows: self.proof.future_rows,
                distinct_sessions: self.proof.distinct_sessions,
                distinct_surfaces: self.proof.distinct_surfaces,
                wrong_accepts: self.proof.wrong_accepts,
                runtime_parity_failures: self.proof.runtime_parity_failures,
                exact_cache_overlap: self.proof.exact_cache_overlap,
                wave_causal_pass: self.proof.wave_causal_pass,
                verifier_schema_bound: verifier_bound,
                verifier_program_bound,
                exact_guard_bound,
                adaptive_identification_bound,
                semantic_applicability_guard_bound,
            },
        )
    }

    /// Package state and counters are only admission inputs, never execution authority.
    #[must_use]
    pub const fn eligible_for_local_accept(&self) -> bool {
        false
    }
}

#[must_use]
pub fn response_program_external_verifier_schema(
    program: &ResponseProgram,
) -> Option<&'static str> {
    let arguments = match &program.operation {
        ResponseOperation::UniqueConsensus { variants, .. } => {
            let schemas = variants
                .iter()
                .filter_map(|variant| response_program_external_verifier_schema(&variant.program))
                .collect::<BTreeSet<_>>();
            return (schemas.len() == 1)
                .then(|| schemas.first().copied())
                .flatten();
        }
        ResponseOperation::AdvancePlan { .. } => {
            return Some(PLAN_ADVANCE_EXTERNAL_VERIFIER_SCHEMA);
        }
        ResponseOperation::FunctionCallFromRoles { arguments, .. } => arguments,
        ResponseOperation::CustomToolCallFromRoles { .. } => {
            return Some(CUSTOM_TOOL_EXTERNAL_VERIFIER_SCHEMA);
        }
        ResponseOperation::ProjectSelectedValue { .. } => {
            return Some(VALUE_PROJECTION_EXTERNAL_VERIFIER_SCHEMA);
        }
        ResponseOperation::ProjectStatus { .. } => {
            return Some(STATUS_PROJECTION_EXTERNAL_VERIFIER_SCHEMA);
        }
        ResponseOperation::ComposeCollection { .. } => {
            return Some(COLLECTION_EXTERNAL_VERIFIER_SCHEMA);
        }
        _ => return None,
    };
    let roles = arguments
        .iter()
        .filter_map(|argument| match argument {
            ResponseArgument::Role { role, .. } => Some(*role),
            ResponseArgument::Integer { .. }
            | ResponseArgument::String { .. }
            | ResponseArgument::Boolean { .. } => None,
        })
        .collect::<BTreeSet<_>>();
    if roles.len() != 1 {
        return None;
    }
    match roles.first().copied() {
        Some(SemanticRole::ContinuationHandle) => Some(CONTINUATION_EXTERNAL_VERIFIER_SCHEMA),
        Some(SemanticRole::SourceValue) => Some(SOURCE_VALUE_EXTERNAL_VERIFIER_SCHEMA),
        _ => None,
    }
}

pub(crate) fn response_program_verifier_matches(
    program: &ResponseProgram,
    verifier: Option<&VerifierProgram>,
) -> bool {
    match (&program.operation, verifier) {
        (
            ResponseOperation::UniqueConsensus { variants, .. },
            Some(VerifierProgram::UniqueConsensus {
                variants: verifier_variants,
                ..
            }),
        ) => {
            variants.len() == verifier_variants.len()
                && variants
                    .iter()
                    .zip(verifier_variants)
                    .all(|(variant, verifier)| {
                        variant.allowed_layout_sha256 == verifier.allowed_layout_sha256
                            && variant.required_request_atom_ids
                                == verifier.required_request_atom_ids
                            && response_program_verifier_matches(
                                &variant.program,
                                Some(&verifier.verifier),
                            )
                    })
        }
        (
            ResponseOperation::AdvancePlan { function_name },
            Some(VerifierProgram::AdvancePlan {
                function_name: verifier_function,
                require_explicit_tool_success,
                require_canonical_plan,
            }),
        ) => {
            function_name == verifier_function
                && *require_explicit_tool_success
                && *require_canonical_plan
        }
        (
            ResponseOperation::ComposeCollection {
                steps,
                format,
                renderer,
                completion_state,
                max_items,
            },
            Some(VerifierProgram::ComposeCollection {
                steps: verifier_steps,
                format: verifier_format,
                renderer: verifier_renderer,
                completion_state: verifier_completion,
                max_items: verifier_max_items,
            }),
        ) => {
            steps == verifier_steps
                && format == verifier_format
                && renderer == verifier_renderer
                && completion_state == verifier_completion
                && max_items == verifier_max_items
        }
        (
            ResponseOperation::FunctionCallFromRoles {
                function_name,
                selector,
                arguments,
            },
            Some(VerifierProgram::FunctionCallFromRoles {
                function_name: verifier_function,
                selector: verifier_selector,
                role_arguments,
                role_argument_types,
                integer_arguments,
                string_arguments,
                boolean_arguments,
                require_pending_state,
                require_unique_handle,
            }),
        ) => {
            let mut expected_roles = BTreeMap::new();
            let mut expected_role_types = BTreeMap::new();
            let mut expected_integers = BTreeMap::new();
            let mut expected_strings = BTreeMap::new();
            let mut expected_booleans = BTreeMap::new();
            for argument in arguments {
                match argument {
                    ResponseArgument::Role {
                        name,
                        role,
                        value_type,
                    } => {
                        expected_roles.insert(name.clone(), *role);
                        if let Some(value_type) = value_type {
                            expected_role_types.insert(name.clone(), *value_type);
                        }
                    }
                    ResponseArgument::Integer { name, value } => {
                        expected_integers.insert(name.clone(), *value);
                    }
                    ResponseArgument::String { name, value } => {
                        expected_strings.insert(name.clone(), value.clone());
                    }
                    ResponseArgument::Boolean { name, value } => {
                        expected_booleans.insert(name.clone(), *value);
                    }
                }
            }
            let pending = expected_roles
                .values()
                .any(|role| *role == SemanticRole::ContinuationHandle);
            function_name == verifier_function
                && selector == verifier_selector
                && &expected_roles == role_arguments
                && &expected_role_types == role_argument_types
                && &expected_integers == integer_arguments
                && &expected_strings == string_arguments
                && &expected_booleans == boolean_arguments
                && *require_pending_state == pending
                && *require_unique_handle == pending
        }
        (
            ResponseOperation::CustomToolCallFromRoles {
                custom_tool_name,
                inner_tool_name,
                selector,
                arguments,
                projection,
            },
            Some(VerifierProgram::CustomToolCallFromRoles {
                custom_tool_name: verifier_custom_tool,
                inner_tool_name: verifier_inner_tool,
                selector: verifier_selector,
                arguments: verifier_arguments,
                projection: verifier_projection,
                require_pending_state,
                require_unique_handle,
            }),
        ) => {
            custom_tool_name == verifier_custom_tool
                && inner_tool_name == verifier_inner_tool
                && selector == verifier_selector
                && arguments == verifier_arguments
                && projection == verifier_projection
                && *require_pending_state
                && *require_unique_handle
        }
        (
            ResponseOperation::ProjectSelectedValue {
                selector,
                format,
                renderer,
                completion_state,
            },
            Some(VerifierProgram::ProjectSelectedValue {
                selector: verifier_selector,
                format: verifier_format,
                renderer: verifier_renderer,
                completion_state: verifier_completion,
                require_unique_value,
            }),
        ) => {
            selector == verifier_selector
                && format == verifier_format
                && renderer == verifier_renderer
                && completion_state == verifier_completion
                && *require_unique_value
        }
        (
            ResponseOperation::ProjectStatus {
                selector,
                mapping,
                renderer,
                completion_state,
            },
            Some(VerifierProgram::ProjectStatus {
                selector: verifier_selector,
                mapping: verifier_mapping,
                renderer: verifier_renderer,
                completion_state: verifier_completion,
                require_unique_value,
            }),
        ) => {
            selector == verifier_selector
                && mapping == verifier_mapping
                && renderer == verifier_renderer
                && completion_state == verifier_completion
                && *require_unique_value
        }
        _ => false,
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponseRegistry {
    pub schema: String,
    pub revision: u64,
    pub packages: Vec<ResponsePackage>,
}

impl ResponseRegistry {
    pub fn validate(&self) -> Result<(), &'static str> {
        match self.schema.as_str() {
            "nando.response-registry.v4" => {
                if self.packages.iter().any(|package| {
                    matches!(
                        package.program.operation,
                        ResponseOperation::ProjectSelectedValue { .. }
                    )
                }) {
                    return Err("registry_v4_value_projection_unsupported");
                }
                if self.packages.iter().any(|package| {
                    matches!(
                        package.program.operation,
                        ResponseOperation::ProjectStatus { .. }
                    )
                }) {
                    return Err("registry_v4_status_projection_unsupported");
                }
            }
            "nando.response-registry.v5" | "nando.response-registry.v6" => {}
            _ => return Err("unsupported_registry_schema"),
        }
        let mut package_ids = BTreeSet::new();
        for package in &self.packages {
            if !package_ids.insert(package.package_id.as_str()) {
                return Err("duplicate_package_id");
            }
            package.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutedResponseExecution {
    pub status: ResponseExecutionStatus,
    pub reason: String,
    pub response: Option<String>,
    pub package_id: Option<String>,
    pub verification_receipt_id: Option<String>,
    pub verifier_schema: Option<String>,
    pub phase_candidates: usize,
    pub exact_actor_checks: usize,
    pub phase_margin_micro: Option<i64>,
    verified: Option<IndependentlyVerifiedExecution>,
}

pub const RESPONSE_PRE_ACTION_EVALUATOR_SCHEMA_V1: &str = "nando.response-pre-action-evaluator.v1";
const MAX_K1_ACTION_INDEX_ENTRIES_V1: usize = 2_048;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreparedK1EvidenceCensorV1 {
    CaptureDisabled,
    AuthoritySnapshotMismatch,
    NoApplicableK1Action,
    ActionProjectionIncomplete,
    CapacityExhausted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreparedK1EvidenceV1 {
    Ready {
        authority_snapshot_root_sha256: String,
        available_actions: AvailableActionContractsV1,
        opaque_execution_binding_set_root_sha256: String,
    },
    Censored(PreparedK1EvidenceCensorV1),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K1ActionIndexEntryV1 {
    package_id: String,
    projection: K1ActionContractProjectionV1,
    execution_binding: OpaqueActionExecutionBindingV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K1ActionIndexEntryMaterialV1 {
    pub package_id: String,
    pub projection: K1ActionContractProjectionV1,
    pub execution_binding: OpaqueActionExecutionBindingV1,
}

impl K1ActionIndexEntryV1 {
    fn new(
        package_id: String,
        projection: K1ActionContractProjectionV1,
        execution_binding: OpaqueActionExecutionBindingV1,
    ) -> Result<Self, &'static str> {
        projection.validate()?;
        execution_binding.validate()?;
        if package_id.is_empty()
            || execution_binding.action_contract_root_sha256
                != projection.action_contract_root_sha256
        {
            return Err("k1_action_index_entry_invalid");
        }
        Ok(Self {
            package_id,
            projection,
            execution_binding,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K1ActionIndexV1 {
    authority_snapshot: DecisionAuthoritySnapshotV1,
    abstain_contract_root_sha256: String,
    entries: BTreeMap<String, K1ActionIndexEntryV1>,
}

impl K1ActionIndexV1 {
    fn new(
        authority_snapshot: DecisionAuthoritySnapshotV1,
        abstain_contract_root_sha256: String,
        entries: Vec<K1ActionIndexEntryV1>,
    ) -> Result<Self, &'static str> {
        authority_snapshot.validate()?;
        if !valid_nonzero_sha256(&abstain_contract_root_sha256)
            || entries.len() > MAX_K1_ACTION_INDEX_ENTRIES_V1
        {
            return Err("k1_action_index_invalid");
        }
        let mut by_package = BTreeMap::new();
        for entry in entries {
            if entry.execution_binding.response_registry_revision
                != authority_snapshot.response_registry_revision
                || entry.execution_binding.response_registry_root_sha256
                    != authority_snapshot.response_registry_root_sha256
                || entry.execution_binding.certification_ledger_revision
                    != authority_snapshot.certification_ledger_revision
                || entry.execution_binding.certification_ledger_root_sha256
                    != authority_snapshot.certification_ledger_root_sha256
            {
                return Err("k1_action_index_binding_snapshot_mismatch");
            }
            if by_package.insert(entry.package_id.clone(), entry).is_some() {
                return Err("k1_action_index_duplicate_package");
            }
        }
        Ok(Self {
            authority_snapshot,
            abstain_contract_root_sha256,
            entries: by_package,
        })
    }

    fn entry(&self, package_id: &str) -> Option<&K1ActionIndexEntryV1> {
        self.entries.get(package_id)
    }
}

struct PreparedResponseCandidate<'a> {
    margin: i64,
    package: &'a ResponsePackage,
    crystallized_binding: Option<BoundCrystallizedOperator>,
}

enum PreparedResponsePlan<'a> {
    Rejected(RoutedResponseExecution),
    Selected(PreparedResponseCandidate<'a>),
}

pub struct PreparedResponseEvaluation<'a> {
    executor_snapshot_root_sha256: &'a str,
    request_identity_root_sha256: Option<String>,
    provider_identity_root_sha256: Option<String>,
    request_text: &'a str,
    provider_payload: &'a Value,
    require_authority: bool,
    k1_evidence: PreparedK1EvidenceV1,
    plan: PreparedResponsePlan<'a>,
}

impl PreparedResponseEvaluation<'_> {
    #[must_use]
    pub const fn k1_evidence(&self) -> &PreparedK1EvidenceV1 {
        &self.k1_evidence
    }

    #[must_use]
    pub fn request_identity_root_sha256(&self) -> Option<&str> {
        self.request_identity_root_sha256.as_deref()
    }

    #[must_use]
    pub fn provider_identity_root_sha256(&self) -> Option<&str> {
        self.provider_identity_root_sha256.as_deref()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponsePhaseControlV1 {
    Full,
    NoPhase,
    ShuffledPhase,
    MagnitudeOnly,
    RandomCenter,
}

impl ResponsePhaseControlV1 {
    pub const ALL: [Self; 5] = [
        Self::Full,
        Self::NoPhase,
        Self::ShuffledPhase,
        Self::MagnitudeOnly,
        Self::RandomCenter,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::NoPhase => "no_phase",
            Self::ShuffledPhase => "shuffled_phase",
            Self::MagnitudeOnly => "magnitude_only",
            Self::RandomCenter => "random_center",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResponseExecutor {
    schema: String,
    revision: u64,
    registry_root_sha256: String,
    snapshot_root_sha256: String,
    packages: Vec<ResponsePackage>,
    crystallized_operators: BTreeMap<String, VerifiedCrystallizedOperator>,
    authority: Option<ValidatedResponseAuthority>,
}

impl ResponseExecutor {
    /// Builds the capture-only K1 index from the same immutable authority
    /// material that produced this executor. The certification ledger must
    /// already have passed anchored-ledger validation in the off-path owner.
    pub fn build_k1_action_index_v1(
        &self,
        authority_snapshot: DecisionAuthoritySnapshotV1,
        abstain_contract_root_sha256: String,
        certification_ledger: &OperatorCertificationLedgerV1,
        entry_materials: Vec<K1ActionIndexEntryMaterialV1>,
    ) -> Result<K1ActionIndexV1, &'static str> {
        certification_ledger.validate()?;
        let vocabulary_gate = certification_ledger.k1_vocabulary_gate()?;
        let Some(runtime_authority) = &self.authority else {
            return Err("k1_action_index_execution_authority_missing");
        };
        if authority_snapshot.response_registry_schema != self.schema
            || authority_snapshot.response_registry_revision != self.revision
            || authority_snapshot.response_registry_root_sha256 != self.registry_root_sha256
            || authority_snapshot.external_admission_authority_root_sha256
                != runtime_authority.admission_sha256
            || authority_snapshot.certification_ledger_revision != certification_ledger.revision
            || authority_snapshot.certification_ledger_root_sha256
                != certification_ledger.ledger_root_sha256
            || authority_snapshot.k1_vocabulary_gate_root_sha256 != vocabulary_gate.gate_root_sha256
            || authority_snapshot.runtime_contract_root_sha256
                != crate::response_runtime_contract_sha256()
        {
            return Err("k1_action_index_authority_snapshot_mismatch");
        }

        let latest_certification = certification_ledger
            .latest_entries()
            .into_iter()
            .map(|entry| (entry.package_id.as_str(), entry))
            .collect::<BTreeMap<_, _>>();
        let entries = entry_materials
            .into_iter()
            .map(|material| {
                K1ActionIndexEntryV1::new(
                    material.package_id,
                    material.projection,
                    material.execution_binding,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        for entry in &entries {
            let package = self
                .packages
                .iter()
                .find(|package| package.package_id == entry.package_id)
                .ok_or("k1_action_index_package_missing")?;
            let admitted = runtime_authority
                .packages
                .get(&entry.package_id)
                .ok_or("k1_action_index_admission_binding_missing")?;
            let certification = latest_certification
                .get(entry.package_id.as_str())
                .ok_or("k1_action_index_certification_entry_missing")?;
            let execution_payload_root_sha256 = crate::response_execution_payload_digest(package)?;
            if admitted.execution_payload_sha256 != execution_payload_root_sha256
                || entry.execution_binding.execution_payload_root_sha256
                    != execution_payload_root_sha256
            {
                return Err("k1_action_index_execution_payload_mismatch");
            }
            if entry
                .execution_binding
                .external_admission_package_binding_root_sha256
                != admitted_package_binding_root_v1(admitted)?
            {
                return Err("k1_action_index_admission_package_binding_mismatch");
            }
            if !certification.k1_unit_eligible
                || certification.entry_root_sha256
                    != entry.execution_binding.certification_entry_root_sha256
                || certification.semantic_law_id_sha256 != entry.projection.semantic_law_id_sha256
                || certification.role_topology_id_sha256 != entry.projection.role_topology_id_sha256
            {
                return Err("k1_action_index_certification_projection_mismatch");
            }
        }
        K1ActionIndexV1::new(authority_snapshot, abstain_contract_root_sha256, entries)
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| format!("response_registry_read:{error}"))?;
        let registry: ResponseRegistry = serde_json::from_slice(&bytes)
            .map_err(|error| format!("response_registry_parse:{error}"))?;
        Self::from_registry(registry).map_err(str::to_owned)
    }

    pub fn from_registry(registry: ResponseRegistry) -> Result<Self, &'static str> {
        registry.validate()?;
        let registry_root_sha256 = response_registry_digest(&registry)?;
        let snapshot_root_sha256 = canonical_json_sha256(&(
            "nando.response-executor-snapshot.v1",
            registry_root_sha256.as_str(),
            Option::<&str>::None,
        ))?;
        let packages = registry
            .packages
            .into_iter()
            .filter(|package| {
                package.eligible_for_admission_candidate()
                    || package.origin == ResponsePackageOrigin::LegacyTemplate
            })
            .collect::<Vec<_>>();
        let crystallized_operators = restore_crystallized_operators(&packages)?;
        Ok(Self {
            schema: registry.schema,
            revision: registry.revision,
            registry_root_sha256,
            snapshot_root_sha256,
            packages,
            crystallized_operators,
            authority: None,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_registry_with_admission(
        registry: ResponseRegistry,
        admission: CompositeResponseAdmissionV2,
        expected_project_id: &str,
        expected_gate_build_sha256: &str,
        expected_runtime_build_sha256: &str,
        now_unix: u64,
        max_age_seconds: u64,
    ) -> Result<Self, &'static str> {
        let registry_root_sha256 = response_registry_digest(&registry)?;
        let authority = validate_response_authority(
            &registry,
            &admission,
            expected_project_id,
            expected_gate_build_sha256,
            expected_runtime_build_sha256,
            now_unix,
            max_age_seconds,
        )?;
        let snapshot_root_sha256 = canonical_json_sha256(&(
            "nando.response-executor-snapshot.v1",
            registry_root_sha256.as_str(),
            Some(authority.admission_sha256.as_str()),
        ))?;
        let admitted_ids = authority.packages.keys().collect::<BTreeSet<_>>();
        let packages = registry
            .packages
            .into_iter()
            .filter(|package| admitted_ids.contains(&package.package_id))
            .collect::<Vec<_>>();
        let crystallized_operators = restore_crystallized_operators(&packages)?;
        Ok(Self {
            schema: registry.schema,
            revision: registry.revision,
            registry_root_sha256,
            snapshot_root_sha256,
            packages,
            crystallized_operators,
            authority: Some(authority),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_authorized_json(
        registry_json: &[u8],
        admission_json: &[u8],
        expected_project_id: &str,
        expected_gate_build_sha256: &str,
        expected_runtime_build_sha256: &str,
        now_unix: u64,
        max_age_seconds: u64,
    ) -> Result<Self, String> {
        let registry = serde_json::from_slice(registry_json)
            .map_err(|error| format!("response_registry_parse:{error}"))?;
        let admission = serde_json::from_slice(admission_json)
            .map_err(|error| format!("response_admission_parse:{error}"))?;
        Self::from_registry_with_admission(
            registry,
            admission,
            expected_project_id,
            expected_gate_build_sha256,
            expected_runtime_build_sha256,
            now_unix,
            max_age_seconds,
        )
        .map_err(str::to_owned)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_revalidated_authorized_json(
        registry_json: &[u8],
        admission_json: &[u8],
        expected_project_id: &str,
        expected_gate_build_sha256: &str,
        expected_runtime_build_sha256: &str,
        now_unix: u64,
        max_age_seconds: u64,
    ) -> Result<Self, String> {
        let registry = serde_json::from_slice(registry_json)
            .map_err(|error| format!("response_registry_parse:{error}"))?;
        let mut admission: CompositeResponseAdmissionV2 = serde_json::from_slice(admission_json)
            .map_err(|error| format!("response_admission_parse:{error}"))?;
        admission.generated_at_unix = now_unix;
        admission.expires_at_unix = now_unix.saturating_add(max_age_seconds);
        Self::from_registry_with_admission(
            registry,
            admission,
            expected_project_id,
            expected_gate_build_sha256,
            expected_runtime_build_sha256,
            now_unix,
            max_age_seconds,
        )
        .map_err(str::to_owned)
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn active_package_count(&self) -> usize {
        if self.authority.is_some() {
            self.packages.len()
        } else {
            0
        }
    }

    #[must_use]
    pub fn active_program_labels(&self) -> BTreeMap<String, usize> {
        if self.authority.is_none() {
            return BTreeMap::new();
        }
        let mut labels = BTreeMap::new();
        for package in &self.packages {
            let label = match &package.program.operation {
                ResponseOperation::UniqueConsensus { variants, .. } => format!(
                    "consensus:{}",
                    variants
                        .first()
                        .map(|variant| response_operation_label(&variant.program))
                        .unwrap_or("empty")
                ),
                ResponseOperation::AdvancePlan { function_name } => {
                    format!("plan_advance:{function_name}")
                }
                ResponseOperation::FunctionCallFromRoles { function_name, .. } => {
                    format!("function:{function_name}")
                }
                ResponseOperation::CustomToolCallFromRoles {
                    custom_tool_name,
                    inner_tool_name,
                    ..
                } => format!("custom_tool:{custom_tool_name}/{inner_tool_name}"),
                ResponseOperation::ProjectSelectedValue { .. } => "project".to_owned(),
                ResponseOperation::ProjectStatus { .. } => "status".to_owned(),
                ResponseOperation::ComposeCollection { .. } => "collection".to_owned(),
                ResponseOperation::CopyAfterPrefix { .. }
                | ResponseOperation::TestResultSummary { .. }
                | ResponseOperation::WaitOnYieldedCell { .. }
                | ResponseOperation::WaitOnAnyYieldedCell { .. }
                | ResponseOperation::WaitOnYieldedSurfaces { .. } => "legacy".to_owned(),
            };
            *labels.entry(label).or_default() += 1;
        }
        labels
    }

    #[must_use]
    pub fn diagnostic_package_count(&self) -> usize {
        self.packages.len()
    }

    #[must_use]
    pub fn registry_schema(&self) -> &str {
        &self.schema
    }

    #[must_use]
    pub fn admission_sha256(&self) -> Option<&str> {
        self.authority
            .as_ref()
            .map(|authority| authority.admission_sha256.as_str())
    }

    #[must_use]
    pub fn execute(&self, request_text: &str, provider_payload: &Value) -> RoutedResponseExecution {
        let prepared = self.prepare_inner(request_text, provider_payload, true, None);
        self.execute_prepared(prepared)
    }

    #[must_use]
    pub fn execute_shadow(
        &self,
        request_text: &str,
        provider_payload: &Value,
    ) -> RoutedResponseExecution {
        let prepared = self.prepare_inner(request_text, provider_payload, false, None);
        self.execute_prepared(prepared)
    }

    #[must_use]
    pub fn evaluate_pre_action<'a>(
        &'a self,
        request_text: &'a str,
        provider_payload: &'a Value,
        k1_index: &K1ActionIndexV1,
    ) -> PreparedResponseEvaluation<'a> {
        self.prepare_inner(request_text, provider_payload, true, Some(k1_index))
    }

    /// Proof-only package-scoped execution. It never carries authority and
    /// cannot produce a runtime receipt or a local accept.
    #[must_use]
    pub fn execute_package_control_shadow(
        &self,
        package_id: &str,
        control: ResponsePhaseControlV1,
        request_text: &str,
        provider_payload: &Value,
    ) -> RoutedResponseExecution {
        let Some(package) = self
            .packages
            .iter()
            .find(|package| package.package_id == package_id)
        else {
            return rejected(format!(
                "proof_package_missing:package={package_id}:control={}",
                control.label()
            ));
        };
        let Some(operator) = self.crystallized_operators.get(package_id) else {
            return rejected(format!(
                "proof_crystallized_operator_missing:package={package_id}:control={}",
                control.label()
            ));
        };
        let context_counts = response_pre_action_context_counts(provider_payload);
        if !routing_predicates_match_counts(&package.routing_predicates, &context_counts) {
            return rejected(format!(
                "proof_routing_predicate_rejected:package={package_id}:control={}",
                control.label()
            ));
        }
        let bound = match operator.bind_pre_action(request_text, provider_payload) {
            Ok(bound) => bound,
            Err(error) => {
                return rejected(format!(
                    "proof_crystallized_role_binding:{error:?}:package={package_id}:control={}",
                    control.label()
                ));
            }
        };
        let structural_margin = operator.runtime_route_margin(&bound);
        let margin = match control {
            // Remove the learned phase-center guard, while preserving the
            // crystallized RoleGraph/RelationProgram route margin.
            ResponsePhaseControlV1::NoPhase => structural_margin,
            ResponsePhaseControlV1::Full => {
                if nando_operator_kernel::response_program_requires_semantic_applicability_guard(
                    &package.program,
                ) {
                    let Some(applicability_margin) =
                        runtime_applicability_margin_micro(package, request_text, provider_payload)
                    else {
                        return rejected(format!(
                            "proof_applicability_guard_rejected:package={package_id}:control={}",
                            control.label()
                        ));
                    };
                    structural_margin.min(applicability_margin)
                } else {
                    structural_margin
                }
            }
            ResponsePhaseControlV1::ShuffledPhase
            | ResponsePhaseControlV1::MagnitudeOnly
            | ResponsePhaseControlV1::RandomCenter => {
                let Some(applicability_margin) = controlled_applicability_margin_micro(
                    package,
                    request_text,
                    provider_payload,
                    control,
                ) else {
                    return rejected(format!(
                        "proof_control_applicability_rejected:package={package_id}:control={}",
                        control.label()
                    ));
                };
                structural_margin.min(applicability_margin)
            }
        };
        if margin < package.wave_margin_micro {
            return rejected(format!(
                "proof_phase_rejected:package={package_id}:control={}:margin={margin}:threshold={}",
                control.label(),
                package.wave_margin_micro
            ));
        }
        let response = match bound.execute_verified() {
            Ok(response) => response,
            Err(error) => {
                return rejected(format!(
                    "proof_crystallized_actor_verifier:{error:?}:package={package_id}:control={}",
                    control.label()
                ));
            }
        };
        RoutedResponseExecution {
            status: ResponseExecutionStatus::Executed,
            reason: format!("proof_crystallized_operator_verified:{}", control.label()),
            response: Some(response),
            package_id: Some(package.package_id.clone()),
            verification_receipt_id: None,
            verifier_schema: Some(package.proof.verifier_schema.clone()),
            phase_candidates: 1,
            exact_actor_checks: 1,
            phase_margin_micro: (control != ResponsePhaseControlV1::NoPhase).then_some(margin),
            verified: None,
        }
    }

    pub fn finalize_runtime_receipt(
        &self,
        execution: &RoutedResponseExecution,
        request_sha256: &str,
        projector_schema: &str,
        projector_program_sha256: &str,
        projected_output: &Value,
    ) -> Result<FinalizedRuntimeVerificationReceiptV2, &'static str> {
        if execution.status != ResponseExecutionStatus::Executed {
            return Err("runtime_receipt_execution_not_verified");
        }
        let verified = execution
            .verified
            .as_ref()
            .ok_or("runtime_receipt_independent_verifier_missing")?;
        finalize_runtime_receipt(
            verified,
            request_sha256,
            projector_schema,
            projector_program_sha256,
            projected_output,
        )
    }

    fn k1_index_matches_authority(&self, index: &K1ActionIndexV1) -> bool {
        let Some(authority) = &self.authority else {
            return false;
        };
        index.authority_snapshot.response_registry_schema == self.schema
            && index.authority_snapshot.response_registry_revision == self.revision
            && index.authority_snapshot.response_registry_root_sha256 == self.registry_root_sha256
            && index
                .authority_snapshot
                .external_admission_authority_root_sha256
                == authority.admission_sha256
    }

    fn prepare_inner<'a>(
        &'a self,
        request_text: &'a str,
        provider_payload: &'a Value,
        require_authority: bool,
        k1_index: Option<&K1ActionIndexV1>,
    ) -> PreparedResponseEvaluation<'a> {
        let (request_identity_root_sha256, provider_identity_root_sha256) = match k1_index {
            Some(_) => (
                Some(crate::sha256_bytes(request_text.as_bytes())),
                Some(match serde_json::to_vec(provider_payload) {
                    Ok(bytes) => crate::sha256_bytes(&bytes),
                    Err(_) => crate::sha256_bytes(b"provider_identity_serialization_failed"),
                }),
            ),
            None => (None, None),
        };
        let k1_authority_matches =
            k1_index.is_none_or(|index| self.k1_index_matches_authority(index));
        let effective_k1_index = if k1_authority_matches { k1_index } else { None };
        if require_authority && self.authority.is_none() {
            return PreparedResponseEvaluation {
                executor_snapshot_root_sha256: &self.snapshot_root_sha256,
                request_identity_root_sha256,
                provider_identity_root_sha256,
                request_text,
                provider_payload,
                require_authority,
                k1_evidence: PreparedK1EvidenceV1::Censored(if k1_index.is_some() {
                    PreparedK1EvidenceCensorV1::AuthoritySnapshotMismatch
                } else {
                    PreparedK1EvidenceCensorV1::NoApplicableK1Action
                }),
                plan: PreparedResponsePlan::Rejected(rejected("execution_authority_missing")),
            };
        }
        let context_counts = response_pre_action_context_counts(provider_payload);
        let mut predicate_matches = 0_usize;
        let mut grounded_matches = 0_usize;
        let mut guard_matches = 0_usize;
        let mut best_margin = i64::MIN;
        let mut best_threshold = 0_i64;
        let mut ranked: [Option<PreparedResponseCandidate<'a>>; 8] = std::array::from_fn(|_| None);
        let mut k1_action_roots = BTreeSet::new();
        let mut k1_binding_roots = BTreeSet::new();
        let mut k1_capacity_exhausted = false;
        for package in &self.packages {
            if !routing_predicates_match_counts(&package.routing_predicates, &context_counts) {
                continue;
            }
            predicate_matches = predicate_matches.saturating_add(1);
            if let Some(operator) = self.crystallized_operators.get(&package.package_id) {
                let Ok(bound) = operator.bind_pre_action(request_text, provider_payload) else {
                    continue;
                };
                grounded_matches = grounded_matches.saturating_add(1);
                let structural_margin = operator.runtime_route_margin(&bound);
                let margin =
                    if nando_operator_kernel::response_program_requires_semantic_applicability_guard(
                        &package.program,
                    ) {
                        let Some(applicability_margin) = runtime_applicability_margin_micro(
                            package,
                            request_text,
                            provider_payload,
                        ) else {
                            continue;
                        };
                        structural_margin.min(applicability_margin)
                    } else {
                        structural_margin
                    };
                guard_matches = guard_matches.saturating_add(1);
                if margin > best_margin {
                    best_margin = margin;
                    best_threshold = package.wave_margin_micro;
                }
                if margin >= package.wave_margin_micro {
                    record_applicable_k1_action(
                        package,
                        effective_k1_index,
                        &mut k1_action_roots,
                        &mut k1_binding_roots,
                        &mut k1_capacity_exhausted,
                    );
                    insert_top_response_candidate(
                        &mut ranked,
                        PreparedResponseCandidate {
                            margin,
                            package,
                            crystallized_binding: Some(bound),
                        },
                    );
                }
                continue;
            }
            let grounded_atoms = match &package.program.operation {
                ResponseOperation::AdvancePlan { function_name } => {
                    Some(response_phase_atom_ids_for_advance_plan_payload(
                        provider_payload,
                        function_name,
                    ))
                }
                ResponseOperation::FunctionCallFromRoles {
                    selector,
                    arguments,
                    ..
                } => Some(response_phase_atom_ids_for_grounded_function_call_payload(
                    provider_payload,
                    selector,
                    arguments,
                )),
                ResponseOperation::CustomToolCallFromRoles { selector, .. } => {
                    Some(response_phase_atom_ids_for_custom_tool_call_payload(
                        provider_payload,
                        selector,
                    ))
                }
                ResponseOperation::ProjectSelectedValue {
                    selector,
                    completion_state,
                    ..
                } => {
                    let mut atoms = response_phase_atom_ids_for_value_projection_payload(
                        provider_payload,
                        selector,
                        completion_state,
                    );
                    atoms.extend(request_phase_atom_ids(request_text));
                    Some(atoms)
                }
                ResponseOperation::ProjectStatus {
                    selector,
                    completion_state,
                    ..
                } => {
                    let mut atoms = response_phase_atom_ids_for_value_projection_payload(
                        provider_payload,
                        selector,
                        completion_state,
                    );
                    atoms.extend(request_phase_atom_ids(request_text));
                    Some(atoms)
                }
                _ => None,
            };
            let query_atoms = if let Some(mut grounded_atoms) = grounded_atoms {
                if grounded_atoms.is_empty() {
                    continue;
                }
                grounded_matches = grounded_matches.saturating_add(1);
                if package.routing_predicates.is_empty() {
                    grounded_atoms.extend(response_pre_action_context_atom_ids(provider_payload));
                    grounded_atoms.sort_unstable();
                    grounded_atoms.dedup();
                    grounded_atoms
                } else {
                    grounded_atoms.extend(
                        package
                            .routing_predicates
                            .iter()
                            .map(ResponseRoutingPredicate::phase_atom_id),
                    );
                    grounded_atoms.sort_unstable();
                    grounded_atoms.dedup();
                    grounded_atoms
                }
            } else {
                response_phase_atom_ids(request_text, provider_payload)
            };
            if query_atoms.is_empty() {
                continue;
            }
            if !package
                .required_routing_atom_ids
                .iter()
                .all(|atom| query_atoms.binary_search(atom).is_ok())
            {
                continue;
            }
            if matches!(
                package.program.operation,
                ResponseOperation::ProjectSelectedValue { .. }
                    | ResponseOperation::ProjectStatus { .. }
            ) && package
                .anti_centers
                .iter()
                .any(|atom| query_atoms.binary_search(atom).is_ok())
            {
                continue;
            }
            guard_matches = guard_matches.saturating_add(1);
            let Some(margin) = package_phase_margin_micro(package, query_atoms) else {
                continue;
            };
            if margin > best_margin {
                best_margin = margin;
                best_threshold = package.wave_margin_micro;
            }
            if margin >= package.wave_margin_micro {
                record_applicable_k1_action(
                    package,
                    effective_k1_index,
                    &mut k1_action_roots,
                    &mut k1_binding_roots,
                    &mut k1_capacity_exhausted,
                );
                insert_top_response_candidate(
                    &mut ranked,
                    PreparedResponseCandidate {
                        margin,
                        package,
                        crystallized_binding: None,
                    },
                );
            }
        }
        let k1_evidence = finalize_prepared_k1_evidence(
            k1_index,
            k1_authority_matches,
            k1_action_roots,
            k1_binding_roots,
            k1_capacity_exhausted,
        );
        let Some(top_margin) = ranked[0].as_ref().map(|candidate| candidate.margin) else {
            return PreparedResponseEvaluation {
                executor_snapshot_root_sha256: &self.snapshot_root_sha256,
                request_identity_root_sha256,
                provider_identity_root_sha256,
                request_text,
                provider_payload,
                require_authority,
                k1_evidence,
                plan: PreparedResponsePlan::Rejected(rejected(format!(
                    "no_phase_routed_profile:packages={}:predicates={predicate_matches}:grounded={grounded_matches}:guard={guard_matches}:best_margin={best_margin}:best_threshold={best_threshold}",
                    self.packages.len()
                ))),
            };
        };
        if ranked[1]
            .as_ref()
            .is_some_and(|candidate| candidate.margin == top_margin)
        {
            return PreparedResponseEvaluation {
                executor_snapshot_root_sha256: &self.snapshot_root_sha256,
                request_identity_root_sha256,
                provider_identity_root_sha256,
                request_text,
                provider_payload,
                require_authority,
                k1_evidence,
                plan: PreparedResponsePlan::Rejected(rejected("ambiguous_phase_route")),
            };
        }
        let Some(candidate) = ranked[0].take() else {
            return PreparedResponseEvaluation {
                executor_snapshot_root_sha256: &self.snapshot_root_sha256,
                request_identity_root_sha256,
                provider_identity_root_sha256,
                request_text,
                provider_payload,
                require_authority,
                k1_evidence,
                plan: PreparedResponsePlan::Rejected(rejected("prepared_candidate_missing")),
            };
        };
        PreparedResponseEvaluation {
            executor_snapshot_root_sha256: &self.snapshot_root_sha256,
            request_identity_root_sha256,
            provider_identity_root_sha256,
            request_text,
            provider_payload,
            require_authority,
            k1_evidence,
            plan: PreparedResponsePlan::Selected(candidate),
        }
    }

    #[must_use]
    pub fn execute_prepared(
        &self,
        prepared: PreparedResponseEvaluation<'_>,
    ) -> RoutedResponseExecution {
        if prepared.executor_snapshot_root_sha256 != self.snapshot_root_sha256 {
            return rejected("prepared_executor_snapshot_mismatch");
        }
        let PreparedResponseEvaluation {
            request_text,
            provider_payload,
            require_authority,
            plan,
            ..
        } = prepared;
        let candidate = match plan {
            PreparedResponsePlan::Rejected(execution) => return execution,
            PreparedResponsePlan::Selected(candidate) => candidate,
        };
        let PreparedResponseCandidate {
            margin: top_margin,
            package,
            crystallized_binding,
        } = candidate;
        let (execution_status, execution_reason, execution_response, independently_verified) =
            if let Some(bound) = crystallized_binding {
                match bound.execute_verified() {
                    Ok(response) => (
                        ResponseExecutionStatus::Executed,
                        "crystallized_operator_verified".to_owned(),
                        Some(response),
                        true,
                    ),
                    Err(error) => {
                        return rejected(format!("crystallized_actor_verifier:{error:?}"));
                    }
                }
            } else {
                let execution = execute_response(&package.program, request_text, provider_payload);
                if execution.status != ResponseExecutionStatus::Executed {
                    return rejected(format!("phase_routed_actor_abstain:{}", execution.reason));
                }
                let Some(response) = execution.response.as_deref() else {
                    return rejected("actor_output_missing");
                };
                let independently_verified = if let Some(verifier) = &package.verifier {
                    if let Err(error) =
                        verify_response_independently(verifier, provider_payload, response)
                    {
                        return rejected(format!("independent_verifier_failed:{error}"));
                    }
                    true
                } else {
                    false
                };
                (
                    execution.status,
                    execution.reason,
                    execution.response,
                    independently_verified,
                )
            };
        let Some(response) = execution_response.as_deref() else {
            return rejected("actor_output_missing");
        };
        let verified = if require_authority {
            if !independently_verified {
                return rejected("independent_verifier_missing");
            }
            let Some(authority) = self
                .authority
                .as_ref()
                .and_then(|authority| authority.packages.get(&package.package_id))
                .cloned()
            else {
                return rejected("package_admission_binding_missing");
            };
            let provider_evidence_sha256 = match canonical_json_sha256(provider_payload) {
                Ok(digest) => digest,
                Err(error) => return rejected(format!("provider_evidence_digest:{error}")),
            };
            let actor_output_sha256 = match canonical_json_sha256(&response) {
                Ok(digest) => digest,
                Err(error) => return rejected(format!("actor_output_digest:{error}")),
            };
            Some(IndependentlyVerifiedExecution {
                package_id: package.package_id.clone(),
                authority,
                provider_evidence_sha256,
                actor_output_sha256,
            })
        } else {
            None
        };
        RoutedResponseExecution {
            status: execution_status,
            reason: execution_reason,
            response: execution_response,
            package_id: Some(package.package_id.clone()),
            verification_receipt_id: None,
            verifier_schema: Some(package.proof.verifier_schema.clone()),
            phase_candidates: 8,
            exact_actor_checks: 1,
            phase_margin_micro: Some(top_margin),
            verified,
        }
    }
}

fn record_applicable_k1_action(
    package: &ResponsePackage,
    k1_index: Option<&K1ActionIndexV1>,
    action_roots: &mut BTreeSet<String>,
    binding_roots: &mut BTreeSet<String>,
    capacity_exhausted: &mut bool,
) {
    let Some(entry) = k1_index.and_then(|index| index.entry(&package.package_id)) else {
        return;
    };
    let action_root = &entry.projection.action_contract_root_sha256;
    if !action_roots.contains(action_root) && action_roots.len() >= 256 {
        *capacity_exhausted = true;
        return;
    }
    action_roots.insert(action_root.clone());
    if binding_roots.len() >= MAX_K1_ACTION_INDEX_ENTRIES_V1
        && !binding_roots.contains(&entry.execution_binding.binding_root_sha256)
    {
        *capacity_exhausted = true;
        return;
    }
    binding_roots.insert(entry.execution_binding.binding_root_sha256.clone());
}

fn finalize_prepared_k1_evidence(
    k1_index: Option<&K1ActionIndexV1>,
    authority_matches: bool,
    action_roots: BTreeSet<String>,
    binding_roots: BTreeSet<String>,
    capacity_exhausted: bool,
) -> PreparedK1EvidenceV1 {
    let Some(index) = k1_index else {
        return PreparedK1EvidenceV1::Censored(PreparedK1EvidenceCensorV1::CaptureDisabled);
    };
    if !authority_matches {
        return PreparedK1EvidenceV1::Censored(
            PreparedK1EvidenceCensorV1::AuthoritySnapshotMismatch,
        );
    }
    if capacity_exhausted {
        return PreparedK1EvidenceV1::Censored(PreparedK1EvidenceCensorV1::CapacityExhausted);
    }
    if action_roots.is_empty() {
        return PreparedK1EvidenceV1::Censored(PreparedK1EvidenceCensorV1::NoApplicableK1Action);
    }
    let available_actions = match AvailableActionContractsV1::seal(
        action_roots.into_iter().collect(),
        index.abstain_contract_root_sha256.clone(),
    ) {
        Ok(available_actions) => available_actions,
        Err(_) => {
            return PreparedK1EvidenceV1::Censored(
                PreparedK1EvidenceCensorV1::ActionProjectionIncomplete,
            );
        }
    };
    let binding_roots = binding_roots.into_iter().collect::<Vec<_>>();
    let opaque_execution_binding_set_root_sha256 = match canonical_json_sha256(&(
        "nando.opaque-action-execution-binding-set.v1",
        &binding_roots,
    )) {
        Ok(root) => root,
        Err(_) => {
            return PreparedK1EvidenceV1::Censored(
                PreparedK1EvidenceCensorV1::ActionProjectionIncomplete,
            );
        }
    };
    PreparedK1EvidenceV1::Ready {
        authority_snapshot_root_sha256: index.authority_snapshot.snapshot_root_sha256.clone(),
        available_actions,
        opaque_execution_binding_set_root_sha256,
    }
}

fn restore_crystallized_operators(
    packages: &[ResponsePackage],
) -> Result<BTreeMap<String, VerifiedCrystallizedOperator>, &'static str> {
    let mut operators = BTreeMap::new();
    for package in packages {
        let Some(bundle) = &package.crystallized_operator else {
            continue;
        };
        let operator = bundle
            .restore_verified()
            .map_err(|_| "crystallized_operator_restore_failed")?;
        if operators
            .insert(package.package_id.clone(), operator)
            .is_some()
        {
            return Err("duplicate_crystallized_operator_package");
        }
    }
    Ok(operators)
}

pub(crate) fn response_phase_atom_ids(request_text: &str, provider_payload: &Value) -> Vec<u64> {
    let mut atoms = response_phase_atom_ids_without_capabilities(request_text, provider_payload);
    atoms.extend(provider_tool_capability_atom_ids(provider_payload));
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

fn response_phase_atom_ids_without_capabilities(
    request_text: &str,
    provider_payload: &Value,
) -> Vec<u64> {
    if provider_payload
        .get("input")
        .and_then(Value::as_array)
        .and_then(|items| items.last())
        .filter(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })
        .and_then(|item| item.get("output"))
        .and_then(Value::as_str)
        .is_some_and(|output| output.starts_with("Script running with cell ID "))
    {
        return if crate::runtime::immediate_yielded_build_or_test(provider_payload) {
            response_phase_atom_ids_for_wait()
        } else if let Some(surface) = crate::runtime::immediate_yielded_surface(provider_payload) {
            response_phase_atom_ids_for_wait_surface(surface)
        } else {
            response_phase_atom_ids_for_any_wait()
        };
    }
    if provider_payload
        .get("input")
        .and_then(Value::as_array)
        .and_then(|items| items.last())
        .filter(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })
        .and_then(|item| item.get("output"))
        .and_then(Value::as_str)
        .and_then(|output| serde_json::from_str::<Value>(output).ok())
        .is_some_and(|value| {
            value
                .as_object()
                .is_some_and(|object| object.values().filter(|value| value.is_array()).count() == 1)
        })
    {
        let mut atoms = vec![
            stable_atom_id("completion:completed"),
            stable_atom_id("observation:json_collection"),
        ];
        atoms.extend(request_phase_atom_ids(request_text));
        atoms.sort_unstable();
        return atoms;
    }
    let request = request_text.trim_start();
    let Some(colon) = request.find(':') else {
        return Vec::new();
    };
    let prefix = request[..=colon].trim().to_ascii_lowercase();
    response_phase_atom_ids_for_prefix(&prefix)
}

pub(crate) fn response_phase_atom_ids_for_any_wait() -> Vec<u64> {
    [
        "response:function_call".to_owned(),
        "role:continue_async_execution".to_owned(),
        "evidence:running_cell_id".to_owned(),
        "scope:any_yielded_cell".to_owned(),
        "function:wait".to_owned(),
    ]
    .iter()
    .map(|atom| stable_atom_id(atom))
    .collect()
}

pub(crate) fn response_phase_atom_ids_for_wait_surface(surface: &str) -> Vec<u64> {
    [
        "response:function_call".to_owned(),
        "role:continue_async_execution".to_owned(),
        "evidence:running_cell_id".to_owned(),
        format!("surface:{surface}"),
        "function:wait".to_owned(),
    ]
    .iter()
    .map(|atom| stable_atom_id(atom))
    .collect()
}

pub(crate) fn response_phase_atom_ids_for_wait() -> Vec<u64> {
    [
        "response:function_call".to_owned(),
        "role:continue_async_execution".to_owned(),
        "evidence:running_cell_id".to_owned(),
        "function:wait".to_owned(),
    ]
    .iter()
    .map(|atom| stable_atom_id(atom))
    .collect()
}

pub(crate) fn response_phase_atom_ids_for_prefix(prefix: &str) -> Vec<u64> {
    let normalized = prefix.trim().to_ascii_lowercase();
    let first = normalized.split_whitespace().next().unwrap_or("");
    let role = match first {
        "reply" | "respond" | "return" | "output" | "answer" | "say" | "write" => {
            "role:emit_literal"
        }
        _ => "role:unknown",
    };
    [
        "response:copy_after_prefix".to_owned(),
        "shape:single_line".to_owned(),
        "shape:colon_delimited".to_owned(),
        role.to_owned(),
        format!("prefix:{normalized}"),
    ]
    .iter()
    .map(|atom| stable_atom_id(atom))
    .collect()
}

pub(crate) fn stable_atom_id(atom: &str) -> u64 {
    nando_operator_runtime::stable_atom_id(atom)
}

pub fn request_phase_atom_ids(text: &str) -> Vec<u64> {
    nando_operator_runtime::request_phase_atom_ids(text)
}

/// Pre-action tool protocol capabilities advertised by the client. Only the
/// provider request's declarations are inspected; historical calls and model
/// outputs are deliberately excluded.
#[must_use]
pub fn provider_tool_capability_atom_ids(provider_payload: &Value) -> Vec<u64> {
    nando_operator_runtime::provider_tool_capability_atom_ids(provider_payload)
}

#[must_use]
pub fn relation_frame_required_observable_atom_ids(frame: &RelationFrame) -> Vec<u64> {
    let mut ids = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ObservationCallShape { value } => {
                Some(stable_atom_id(&format!("observation_call_shape:{value}")))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    ids
}

pub fn relation_frame_phase_margin_micro(
    package: &ResponsePackage,
    frame: &RelationFrame,
) -> Option<i64> {
    let query_atoms = relation_frame_routing_query_atom_ids(package, frame)?;
    package_phase_margin_micro(package, query_atoms)
}

fn package_phase_margin_micro(package: &ResponsePackage, query_atoms: Vec<u64>) -> Option<i64> {
    if let Some(route) = &package.learned_wave_route {
        let cells = usize::from(route.cells);
        let query = phase_vector_from_atom_ids(query_atoms, cells);
        let score = |center_delta_micro: &[i32]| {
            let mut score = 0.0_f64;
            for (cell, delta) in query.iter().zip(center_delta_micro.chunks_exact(2)) {
                score += cell.re * f64::from(delta[0]) / 1_000_000.0;
                score += cell.im * f64::from(delta[1]) / 1_000_000.0;
            }
            phase_margin_to_micro(score / cells as f64).ok()
        };
        let primary = score(&route.center_delta_micro)?;
        if route.subcenters.is_empty() {
            return Some(primary);
        }
        let best_excess = route.subcenters.iter().fold(
            primary.saturating_sub(route.threshold_micro),
            |best, subcenter| {
                score(&subcenter.center_delta_micro)
                    .map(|margin| margin.saturating_sub(subcenter.threshold_micro))
                    .map_or(best, |excess| best.max(excess))
            },
        );
        return Some(package.wave_margin_micro.saturating_add(best_excess));
    }
    let query = phase_vector_from_atom_ids(query_atoms, 16);
    let positive = phase_vector_from_atom_ids(package.phase_centers.iter().copied(), 16);
    let negative = phase_vector_from_atom_ids(package.anti_centers.iter().copied(), 16);
    let margin = phase_coherence(&query, &positive)
        - if package.anti_centers.is_empty() {
            0.0
        } else {
            phase_coherence(&query, &negative)
        };
    phase_margin_to_micro(margin).ok()
}

fn runtime_applicability_margin_micro(
    package: &ResponsePackage,
    request_text: &str,
    provider_payload: &Value,
) -> Option<i64> {
    let query_atoms =
        runtime_applicability_query_atom_ids(package, request_text, provider_payload)?;
    package_phase_margin_micro(package, query_atoms)
}

fn runtime_applicability_query_atom_ids(
    package: &ResponsePackage,
    request_text: &str,
    provider_payload: &Value,
) -> Option<Vec<u64>> {
    if package.anti_centers.is_empty() {
        return None;
    }

    let mut observed_atoms = request_phase_atom_ids(request_text);
    observed_atoms.extend(response_pre_action_context_atom_ids(provider_payload));
    observed_atoms.extend(
        package
            .routing_predicates
            .iter()
            .map(ResponseRoutingPredicate::phase_atom_id),
    );
    observed_atoms.sort_unstable();
    observed_atoms.dedup();

    let mut vocabulary = package
        .learned_wave_route
        .as_ref()
        .filter(|route| !route.query_atom_ids.is_empty())
        .map_or_else(
            || {
                package
                    .phase_centers
                    .iter()
                    .chain(&package.anti_centers)
                    .copied()
                    .collect::<Vec<_>>()
            },
            |route| route.query_atom_ids.clone(),
        );
    vocabulary.sort_unstable();
    vocabulary.dedup();
    if !observed_atoms
        .iter()
        .any(|atom| vocabulary.binary_search(atom).is_ok())
    {
        return None;
    }

    let mut query_atoms = observed_atoms;
    query_atoms.extend(response_program_required_routing_atom_ids(&package.program));
    query_atoms.retain(|atom| vocabulary.binary_search(atom).is_ok());
    query_atoms.sort_unstable();
    query_atoms.dedup();
    if query_atoms.is_empty()
        || !package
            .required_routing_atom_ids
            .iter()
            .all(|atom| query_atoms.binary_search(atom).is_ok())
    {
        return None;
    }
    Some(query_atoms)
}

fn controlled_applicability_margin_micro(
    package: &ResponsePackage,
    request_text: &str,
    provider_payload: &Value,
    control: ResponsePhaseControlV1,
) -> Option<i64> {
    if matches!(
        control,
        ResponsePhaseControlV1::Full | ResponsePhaseControlV1::NoPhase
    ) {
        return None;
    }
    let atoms = runtime_applicability_query_atom_ids(package, request_text, provider_payload)?;
    if package.learned_wave_route.is_some() {
        let mut controlled = package.clone();
        let route = controlled.learned_wave_route.as_mut()?;
        transform_center_delta(&mut route.center_delta_micro, control);
        for subcenter in &mut route.subcenters {
            transform_center_delta(&mut subcenter.center_delta_micro, control);
        }
        return package_phase_margin_micro(&controlled, atoms);
    }
    let positive = phase_vector_from_atom_ids(package.phase_centers.iter().copied(), 16);
    let negative = phase_vector_from_atom_ids(package.anti_centers.iter().copied(), 16);
    let query = phase_vector_from_atom_ids(atoms, positive.len());
    let score = match control {
        ResponsePhaseControlV1::ShuffledPhase => {
            let mut shuffled_positive = positive.clone();
            let mut shuffled_negative = negative.clone();
            let shift = (positive.len() / 3).max(1);
            shuffled_positive.rotate_left(shift);
            shuffled_negative.rotate_left(shift);
            phase_coherence(&query, &shuffled_positive)
                - phase_coherence(&query, &shuffled_negative)
        }
        ResponsePhaseControlV1::MagnitudeOnly => {
            magnitude_coherence(&query, &positive) - magnitude_coherence(&query, &negative)
        }
        ResponsePhaseControlV1::RandomCenter => {
            let random_positive = matched_random_center(&positive);
            let random_negative = matched_random_center(&negative);
            phase_coherence(&query, &random_positive) - phase_coherence(&query, &random_negative)
        }
        ResponsePhaseControlV1::Full | ResponsePhaseControlV1::NoPhase => return None,
    };
    phase_margin_to_micro(score).ok()
}

fn transform_center_delta(values: &mut Vec<i32>, control: ResponsePhaseControlV1) {
    match control {
        ResponsePhaseControlV1::ShuffledPhase => {
            let cells = values.len() / 2;
            values.rotate_left(2 * (cells / 3).max(1).min(cells));
        }
        ResponsePhaseControlV1::MagnitudeOnly => {
            for value in values.chunks_exact_mut(2) {
                let magnitude = f64::from(value[0])
                    .hypot(f64::from(value[1]))
                    .round()
                    .clamp(f64::from(i32::MIN), f64::from(i32::MAX))
                    as i32;
                value[0] = magnitude;
                value[1] = 0;
            }
        }
        ResponsePhaseControlV1::RandomCenter => {
            let mut cells = values
                .chunks_exact(2)
                .map(|value| [value[0], value[1]])
                .collect::<Vec<_>>();
            cells.reverse();
            for (index, cell) in cells.iter_mut().enumerate() {
                let sign = if index % 2 == 0 { 1 } else { -1 };
                *cell = [cell[1].saturating_mul(sign), cell[0].saturating_mul(-sign)];
            }
            values.clear();
            values.extend(cells.into_iter().flatten());
        }
        ResponsePhaseControlV1::Full | ResponsePhaseControlV1::NoPhase => {}
    }
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

fn insert_top_response_candidate<'a>(
    ranked: &mut [Option<PreparedResponseCandidate<'a>>; 8],
    candidate: PreparedResponseCandidate<'a>,
) {
    let position = ranked.iter().position(|current| {
        current.as_ref().is_none_or(|current| {
            candidate.margin > current.margin
                || (candidate.margin == current.margin
                    && candidate.package.package_id < current.package.package_id)
        })
    });
    let Some(position) = position else {
        return;
    };
    for index in (position + 1..ranked.len()).rev() {
        ranked[index] = ranked[index - 1].take();
    }
    ranked[position] = Some(candidate);
}

pub(crate) fn relation_frame_routing_query_atom_ids(
    package: &ResponsePackage,
    frame: &RelationFrame,
) -> Option<Vec<u64>> {
    let counts = relation_frame_cardinality_counts(frame);
    if !routing_predicates_match_counts(&package.routing_predicates, &counts) {
        return None;
    }
    let mut atoms = if package.routing_predicates.is_empty() {
        if package.learned_wave_route.is_some() {
            relation_frame_online_routing_atom_ids(frame)
        } else {
            relation_frame_routing_atom_ids(frame)
        }
    } else {
        let filtered = RelationFrame {
            atoms: frame
                .atoms
                .iter()
                .filter(|atom| !matches!(atom, RelationAtom::Cardinality { .. }))
                .cloned()
                .collect(),
            ..frame.clone()
        };
        if package.learned_wave_route.is_some() {
            relation_frame_online_routing_atom_ids(&filtered)
        } else {
            relation_frame_routing_atom_ids(&filtered)
        }
    };
    atoms.extend(
        package
            .routing_predicates
            .iter()
            .map(ResponseRoutingPredicate::phase_atom_id),
    );
    atoms.sort_unstable();
    atoms.dedup();
    if !package
        .required_routing_atom_ids
        .iter()
        .all(|atom| atoms.binary_search(atom).is_ok())
    {
        return None;
    }
    if let Some(route) = &package.learned_wave_route
        && !route.query_atom_ids.is_empty()
    {
        atoms.retain(|atom| route.query_atom_ids.binary_search(atom).is_ok());
        if atoms.is_empty() {
            return None;
        }
    }
    // A non-empty query_atom_ids list is the immutable training vocabulary.
    // Unknown future atoms must not perturb the frozen phase vector.
    Some(atoms)
}

#[must_use]
pub(crate) fn relation_frame_matches_package_guard(
    package: &ResponsePackage,
    frame: &RelationFrame,
) -> bool {
    relation_frame_routing_query_atom_ids(package, frame).is_some()
}

#[must_use]
pub fn relation_frame_routes_to_package(package: &ResponsePackage, frame: &RelationFrame) -> bool {
    relation_frame_phase_margin_micro(package, frame)
        .is_some_and(|margin| margin >= package.wave_margin_micro)
}

#[cfg(test)]
pub(crate) fn response_phase_atom_ids_for_grounded_function_call() -> Vec<u64> {
    let mut atoms = [
        "relation:tool_kind",
        "completion:pending",
        "slot:identifier:observation",
        "relation:unique_slot",
    ]
    .into_iter()
    .map(stable_atom_id)
    .collect::<Vec<_>>();
    atoms.push(selector_phase_atom_id(
        &ResponseValueSelector::ContentLinePrefix {
            prefix: "Script running with cell ID ".to_owned(),
            value_type: AtomValueType::Identifier,
        },
    ));
    atoms
}

fn response_phase_atom_ids_for_grounded_function_call_payload(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
    arguments: &[ResponseArgument],
) -> Vec<u64> {
    nando_operator_runtime::response_phase_atom_ids_for_grounded_function_call_payload(
        provider_payload,
        selector,
        arguments,
    )
}

fn response_phase_atom_ids_for_advance_plan_payload(
    provider_payload: &Value,
    function_name: &str,
) -> Vec<u64> {
    let Some((step_count, completed_count, _active_index)) =
        crate::runtime::advance_plan_runtime_state(provider_payload, function_name)
    else {
        return Vec::new();
    };
    let mut atoms = vec![
        stable_atom_id("relation:tool_kind"),
        stable_atom_id("completion:completed"),
        stable_atom_id("status:success"),
        stable_atom_id("relation:plan_state"),
        stable_atom_id(&format!(
            "cardinality:plan_step_count_band:{}",
            count_band(usize::from(step_count))
        )),
        stable_atom_id(&format!(
            "cardinality:plan_completed_count_band:{}",
            count_band(usize::from(completed_count))
        )),
    ];
    if let Some(shape) = immediate_observation_call_shape(provider_payload) {
        atoms.push(stable_atom_id(&format!("observation_call_shape:{shape}")));
    }
    atoms
}

fn immediate_observation_call_shape(provider_payload: &Value) -> Option<String> {
    nando_operator_runtime::immediate_observation_call_shape(provider_payload)
}

fn response_phase_atom_ids_for_custom_tool_call_payload(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
) -> Vec<u64> {
    nando_operator_runtime::response_phase_atom_ids_for_custom_tool_call_payload(
        provider_payload,
        selector,
    )
}

fn response_phase_atom_ids_for_value_projection_payload(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
    completion_state: &str,
) -> Vec<u64> {
    let Ok(scalar) = immediate_selected_scalar(provider_payload, selector) else {
        return Vec::new();
    };
    let mut atoms = vec![
        stable_atom_id("relation:tool_kind"),
        stable_atom_id(&format!("completion:{completion_state}")),
        stable_atom_id(&format!(
            "slot:{}:observation",
            value_type_name(scalar.value_type)
        )),
        stable_atom_id("relation:unique_slot"),
        selector_phase_atom_id(selector),
    ];
    if let Some(shape) = immediate_observation_call_shape(provider_payload) {
        atoms.push(stable_atom_id(&format!("observation_call_shape:{shape}")));
    }
    atoms
}

fn selector_phase_atom_id(selector: &ResponseValueSelector) -> u64 {
    nando_operator_runtime::selector_phase_atom_id(selector)
}

#[cfg(test)]
fn stable_atom_id_parts(parts: &[&str]) -> u64 {
    nando_operator_runtime::stable_atom_id_parts(parts)
}

pub(crate) fn response_pre_action_context_atom_ids(provider_payload: &Value) -> Vec<u64> {
    nando_operator_runtime::response_pre_action_context_atom_ids(provider_payload)
}

fn admitted_package_binding_root_v1(
    admitted: &nando_operator_admission::AuthorizedResponsePackage,
) -> Result<String, &'static str> {
    #[derive(Serialize)]
    struct BindingMaterial<'a> {
        schema: &'static str,
        admission_sha256: &'a str,
        registry_sha256: &'a str,
        registry_revision: u64,
        package_sha256: &'a str,
        execution_payload_sha256: &'a str,
        actor_program_sha256: &'a str,
        independent_verifier_program_sha256: &'a str,
        verifier_schema: &'a str,
        gate_build_sha256: &'a str,
        runtime_build_sha256: &'a str,
        support_manifest_sha256: &'a str,
        exact_causal_proof_sha256: &'a str,
        runtime_parity_receipt_set_sha256: &'a str,
        future_verifier_receipt_set_sha256: &'a str,
        semantic_alias_proof_sha256: &'a str,
        proof_receipts_sha256: &'a str,
    }

    canonical_json_sha256(&BindingMaterial {
        schema: "nando.response-admission-package-binding-projection.v1",
        admission_sha256: &admitted.admission_sha256,
        registry_sha256: &admitted.registry_sha256,
        registry_revision: admitted.registry_revision,
        package_sha256: &admitted.package_sha256,
        execution_payload_sha256: &admitted.execution_payload_sha256,
        actor_program_sha256: &admitted.actor_program_sha256,
        independent_verifier_program_sha256: &admitted.independent_verifier_program_sha256,
        verifier_schema: &admitted.verifier_schema,
        gate_build_sha256: &admitted.gate_build_sha256,
        runtime_build_sha256: &admitted.runtime_build_sha256,
        support_manifest_sha256: &admitted.support_manifest_sha256,
        exact_causal_proof_sha256: &admitted.exact_causal_proof_sha256,
        runtime_parity_receipt_set_sha256: &admitted.runtime_parity_receipt_set_sha256,
        future_verifier_receipt_set_sha256: &admitted.future_verifier_receipt_set_sha256,
        semantic_alias_proof_sha256: &admitted.semantic_alias_proof_sha256,
        proof_receipts_sha256: &admitted.proof_receipts_sha256,
    })
}

fn response_pre_action_context_counts(provider_payload: &Value) -> BTreeMap<String, u32> {
    nando_operator_runtime::response_pre_action_context_counts(provider_payload)
}

fn relation_frame_cardinality_counts(frame: &RelationFrame) -> BTreeMap<String, u32> {
    frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::Cardinality { role, count } => Some((role.clone(), *count)),
            _ => None,
        })
        .collect()
}

fn routing_predicates_match_counts(
    predicates: &[ResponseRoutingPredicate],
    counts: &BTreeMap<String, u32>,
) -> bool {
    predicates.iter().all(|predicate| {
        counts
            .get(&predicate.role)
            .is_some_and(|count| predicate.matches_count(*count))
    })
}

const fn count_band(value: usize) -> usize {
    if value == 0 {
        0
    } else {
        1_usize << (usize::BITS - 1 - value.leading_zeros())
    }
}

const fn value_type_name(value: AtomValueType) -> &'static str {
    match value {
        AtomValueType::String => "string",
        AtomValueType::Integer => "integer",
        AtomValueType::Boolean => "boolean",
        AtomValueType::Identifier => "identifier",
        AtomValueType::Collection => "collection",
    }
}

fn rejected(reason: impl Into<String>) -> RoutedResponseExecution {
    RoutedResponseExecution {
        status: ResponseExecutionStatus::Abstain,
        reason: reason.into(),
        response: None,
        package_id: None,
        verification_receipt_id: None,
        verifier_schema: None,
        phase_candidates: 0,
        exact_actor_checks: 0,
        phase_margin_micro: None,
        verified: None,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::{
        ProjectStatusMapping, RELATION_FRAME_SCHEMA, SOURCE_NEUTRAL_EXTRACTOR_VERSION,
        ValueProjectionFormat,
    };

    #[test]
    fn plan_runtime_payload_reconstructs_every_required_routing_atom() {
        let payload = serde_json::json!({
            "tools":[{"type":"function","name":"update_plan"}],
            "input":[
                {
                    "type":"function_call",
                    "name":"update_plan",
                    "call_id":"plan-1",
                    "arguments":{
                        "plan":[
                            {"step":"Inspect","status":"in_progress"},
                            {"step":"Apply","status":"pending"}
                        ]
                    }
                },
                {
                    "type":"function_call",
                    "name":"exec_command",
                    "call_id":"exec-1"
                },
                {
                    "type":"function_call_output",
                    "call_id":"exec-1",
                    "output":{"exit_code":0}
                }
            ]
        });
        let program = ResponseProgram::advance_plan("update_plan");
        let mut query = response_phase_atom_ids_for_advance_plan_payload(&payload, "update_plan");
        query.extend(response_pre_action_context_atom_ids(&payload));
        query.sort_unstable();
        query.dedup();
        let required = response_program_required_routing_atom_ids(&program);
        assert!(
            required
                .iter()
                .all(|atom| query.binary_search(atom).is_ok())
        );

        let mut failed = payload;
        failed["input"][2]["output"]["exit_code"] = Value::from(1);
        assert!(
            response_phase_atom_ids_for_advance_plan_payload(&failed, "update_plan").is_empty()
        );
    }

    fn active_projection_package() -> ResponsePackage {
        let selector = ResponseValueSelector::JsonField {
            field: "status".to_owned(),
            value_type: AtomValueType::Identifier,
        };
        let program = ResponseProgram::project_selected_value(
            selector.clone(),
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let required_routing_atom_ids = response_program_required_routing_atom_ids(&program);
        ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: "projection-package".to_owned(),
            origin: ResponsePackageOrigin::GroundedSynthesis,
            state: ResponsePackageState::Active,
            program,
            verifier: Some(VerifierProgram::ProjectSelectedValue {
                selector,
                format: ValueProjectionFormat::PlainText,
                renderer: crate::CollectionOutputRenderer::Direct,
                completion_state: "completed".to_owned(),
                require_unique_value: true,
            }),
            routing_predicates: Vec::new(),
            phase_centers: required_routing_atom_ids.clone(),
            required_routing_atom_ids,
            anti_centers: Vec::new(),
            wave_margin_micro: 1,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: ResponsePackageProof {
                support_rows: 32,
                future_rows: 32,
                distinct_sessions: 3,
                distinct_surfaces: 2,
                wrong_accepts: 0,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass: true,
                verifier_schema: VALUE_PROJECTION_EXTERNAL_VERIFIER_SCHEMA.to_owned(),
                adaptive_identification: None,
            },
        }
    }

    fn active_status_package() -> ResponsePackage {
        let selector = ResponseValueSelector::JsonField {
            field: "opaque_code".to_owned(),
            value_type: AtomValueType::Integer,
        };
        let program = ResponseProgram::project_status(
            selector.clone(),
            ProjectStatusMapping::ZeroIsSuccess,
            "completed",
        );
        let required_routing_atom_ids = response_program_required_routing_atom_ids(&program);
        ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: "status-package".to_owned(),
            origin: ResponsePackageOrigin::GroundedSynthesis,
            state: ResponsePackageState::Active,
            program,
            verifier: Some(VerifierProgram::ProjectStatus {
                selector,
                mapping: ProjectStatusMapping::ZeroIsSuccess,
                renderer: crate::CollectionOutputRenderer::Direct,
                completion_state: "completed".to_owned(),
                require_unique_value: true,
            }),
            routing_predicates: Vec::new(),
            phase_centers: required_routing_atom_ids.clone(),
            required_routing_atom_ids,
            anti_centers: Vec::new(),
            wave_margin_micro: 1,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: ResponsePackageProof {
                support_rows: 32,
                future_rows: 32,
                distinct_sessions: 3,
                distinct_surfaces: 2,
                wrong_accepts: 0,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass: true,
                verifier_schema: STATUS_PROJECTION_EXTERNAL_VERIFIER_SCHEMA.to_owned(),
                adaptive_identification: None,
            },
        }
    }

    fn digest_root(label: &str) -> String {
        canonical_json_sha256(&("nando.s1c-test-root.v1", label)).expect("root")
    }

    fn k1_projection(label: &str) -> K1ActionContractProjectionV1 {
        K1ActionContractProjectionV1::seal(
            digest_root(&format!("{label}:law")),
            digest_root(&format!("{label}:topology")),
            digest_root(&format!("{label}:semantic")),
            digest_root(&format!("{label}:effect")),
            digest_root(&format!("{label}:applicability")),
            digest_root(&format!("{label}:verifier")),
            digest_root(&format!("{label}:callees")),
            nando_operator_learning::multi_source::K1ConsequenceTypeV1::Scalar,
        )
        .expect("projection")
    }

    fn k1_index_entry(
        package_id: String,
        label: &str,
        snapshot: &DecisionAuthoritySnapshotV1,
    ) -> K1ActionIndexEntryV1 {
        let projection = k1_projection(label);
        let binding = OpaqueActionExecutionBindingV1::seal(
            projection.action_contract_root_sha256.clone(),
            digest_root(&format!("{label}:execution")),
            digest_root(&format!("{label}:admission-binding")),
            digest_root(&format!("{label}:certification-entry")),
            snapshot.response_registry_root_sha256.clone(),
            snapshot.response_registry_revision,
            snapshot.certification_ledger_root_sha256.clone(),
            snapshot.certification_ledger_revision,
        )
        .expect("binding");
        K1ActionIndexEntryV1::new(package_id, projection, binding).expect("entry")
    }

    fn k1_certification_ledger(
        package_id: &str,
        projection: &K1ActionContractProjectionV1,
    ) -> nando_operator_admission::OperatorCertificationLedgerV1 {
        use nando_operator_admission::{
            ExecutionCertificateStatusV1, ExecutionCertificateV1, LawCertificateStatusV1,
            LawCertificateV1, MechanismCertificateStatusV1, MechanismCertificateV1,
            OperatorCertificationEntryV1, OperatorMechanismClassV1,
        };

        let bundle_id = digest_root(&format!("{package_id}:bundle"));
        let execution = ExecutionCertificateV1::seal(
            &bundle_id,
            package_id,
            ExecutionCertificateStatusV1::Pass,
            vec![digest_root(&format!("{package_id}:execution-evidence"))],
            "",
        )
        .expect("execution certificate");
        let law = LawCertificateV1::seal(
            &bundle_id,
            package_id,
            LawCertificateStatusV1::Pass,
            vec![digest_root(&format!("{package_id}:law-evidence"))],
            Some(digest_root(&format!("{package_id}:cleanup"))),
            "",
        )
        .expect("law certificate");
        let mechanism = MechanismCertificateV1::seal(
            &bundle_id,
            package_id,
            MechanismCertificateStatusV1::Collecting,
            OperatorMechanismClassV1::Unresolved,
            vec![digest_root(&format!("{package_id}:mechanism-evidence"))],
            "mechanism_collecting",
        )
        .expect("mechanism certificate");
        let entry = OperatorCertificationEntryV1::seal(
            &bundle_id,
            package_id,
            &projection.semantic_law_id_sha256,
            &projection.role_topology_id_sha256,
            execution,
            law,
            mechanism,
            0,
        )
        .expect("certification entry");
        let mut ledger =
            nando_operator_admission::OperatorCertificationLedgerV1::empty().expect("empty ledger");
        assert!(ledger.append(entry).expect("append certification"));
        ledger
    }

    fn projection_payload() -> Value {
        serde_json::json!({
            "input": [{
                "type": "function_call_output",
                "call_id": "call-1",
                "output": "{\"status\":\"ready\"}"
            }]
        })
    }

    fn authorized_executor(
        revision: u64,
        packages: Vec<ResponsePackage>,
    ) -> (ResponseExecutor, DecisionAuthoritySnapshotV1) {
        let registry = ResponseRegistry {
            schema: "nando.response-registry.v6".to_owned(),
            revision,
            packages,
        };
        let registry_root = response_registry_digest(&registry).expect("registry root");
        let receipt_digests = registry
            .packages
            .iter()
            .map(|package| {
                (
                    package.package_id.clone(),
                    (
                        digest_root(&format!("{}:support", package.package_id)),
                        digest_root(&format!("{}:causal", package.package_id)),
                        digest_root(&format!("{}:parity", package.package_id)),
                        digest_root(&format!("{}:future", package.package_id)),
                        digest_root(&format!("{}:alias", package.package_id)),
                    ),
                )
            })
            .collect();
        let gate_build = digest_root("gate-build");
        let runtime_build = digest_root("runtime-build");
        let admission = crate::authority::build_composite_admission_for_registry(
            &registry,
            receipt_digests,
            "s1c-test-project",
            100,
            30,
            &gate_build,
            &runtime_build,
            "missing receipts",
            "missing verifier",
        )
        .expect("admission");
        let admission_root = canonical_json_sha256(&admission).expect("admission root");
        let snapshot = DecisionAuthoritySnapshotV1::seal(
            registry.schema.clone(),
            registry.revision,
            registry_root,
            admission_root,
            1,
            digest_root("certification-ledger"),
            digest_root("k1-gate"),
            crate::response_runtime_contract_sha256(),
        )
        .expect("snapshot");
        let executor = ResponseExecutor::from_registry_with_admission(
            registry,
            admission,
            "s1c-test-project",
            &gate_build,
            &runtime_build,
            100,
            30,
        )
        .expect("executor");
        (executor, snapshot)
    }

    #[test]
    fn capture_disabled_prepared_route_skips_request_identity_materialization() {
        let (executor, _) = authorized_executor(100, vec![active_projection_package()]);
        let payload = projection_payload();
        let prepared = executor.prepare_inner("compatibility request", &payload, true, None);

        assert_eq!(
            prepared.k1_evidence(),
            &PreparedK1EvidenceV1::Censored(PreparedK1EvidenceCensorV1::CaptureDisabled)
        );
        assert_eq!(prepared.request_identity_root_sha256(), None);
        assert_eq!(prepared.provider_identity_root_sha256(), None);
        assert_eq!(
            executor.execute_prepared(prepared).status,
            ResponseExecutionStatus::Executed
        );
    }

    #[test]
    #[ignore = "isolated remote release S1C compatibility latency gate"]
    fn capture_disabled_compatibility_latency_stays_within_hot_budget() {
        const SAMPLES: usize = 4_096;
        let (executor, _) = authorized_executor(99, vec![active_projection_package()]);
        let matched = projection_payload();
        let unmatched = serde_json::json!({});

        for _ in 0..128 {
            assert_eq!(
                executor.execute("compatibility request", &matched).status,
                ResponseExecutionStatus::Executed
            );
            assert_eq!(
                executor.execute("unmatched request", &unmatched).status,
                ResponseExecutionStatus::Abstain
            );
        }
        let matched_samples = execution_latency_samples(
            &executor,
            "compatibility request",
            &matched,
            ResponseExecutionStatus::Executed,
            SAMPLES,
        );
        let unmatched_samples = execution_latency_samples(
            &executor,
            "unmatched request",
            &unmatched,
            ResponseExecutionStatus::Abstain,
            SAMPLES,
        );
        let matched_p99 = percentile_ns(&matched_samples, 99);
        let unmatched_p99 = percentile_ns(&unmatched_samples, 99);
        let hard_max = matched_samples
            .iter()
            .chain(&unmatched_samples)
            .copied()
            .max()
            .unwrap_or(u128::MAX);
        println!(
            "S1C_HOT_LATENCY matched_p99_ns={matched_p99} no_goal_p99_ns={unmatched_p99} hard_max_ns={hard_max} samples={SAMPLES}"
        );
        assert!(matched_p99 <= 1_000_000, "matched p99 exceeded 1 ms");
        assert!(unmatched_p99 <= 250_000, "no-goal p99 exceeded 250 us");
        assert!(hard_max <= 2_000_000, "hard ceiling exceeded 2 ms");
    }

    #[test]
    #[ignore = "isolated remote release S1C 60-second idle CPU gate"]
    fn capture_disabled_executor_has_no_sustained_idle_cpu_work() {
        let (executor, _) = authorized_executor(100, vec![active_projection_package()]);
        assert_eq!(
            executor
                .execute("compatibility request", &projection_payload())
                .status,
            ResponseExecutionStatus::Executed
        );
        let ticks_per_second = std::process::Command::new("getconf")
            .arg("CLK_TCK")
            .output()
            .expect("getconf CLK_TCK");
        assert!(ticks_per_second.status.success());
        let ticks_per_second = String::from_utf8(ticks_per_second.stdout)
            .expect("CLK_TCK utf8")
            .trim()
            .parse::<u64>()
            .expect("CLK_TCK integer");
        let before = process_cpu_ticks();
        std::thread::sleep(std::time::Duration::from_secs(60));
        let elapsed_ticks = process_cpu_ticks().saturating_sub(before);
        let cpu_percent_of_one_core =
            (elapsed_ticks as f64) * 100.0 / ((ticks_per_second as f64) * 60.0);
        println!(
            "S1C_IDLE_CPU elapsed_ticks={elapsed_ticks} ticks_per_second={ticks_per_second} percent_of_one_core={cpu_percent_of_one_core:.6}"
        );
        assert!(
            cpu_percent_of_one_core <= 0.25,
            "idle CPU exceeded 0.25% of one core"
        );
    }

    fn process_cpu_ticks() -> u64 {
        let stat = std::fs::read_to_string("/proc/self/stat").expect("process stat");
        let after_name = stat.rsplit_once(") ").expect("process stat comm").1;
        let fields = after_name.split_ascii_whitespace().collect::<Vec<_>>();
        let user_ticks = fields[11].parse::<u64>().expect("process user ticks");
        let system_ticks = fields[12].parse::<u64>().expect("process system ticks");
        user_ticks.saturating_add(system_ticks)
    }

    fn execution_latency_samples(
        executor: &ResponseExecutor,
        request_text: &str,
        provider_payload: &Value,
        expected: ResponseExecutionStatus,
        count: usize,
    ) -> Vec<u128> {
        (0..count)
            .map(|_| {
                let started = Instant::now();
                let execution = executor.execute(request_text, provider_payload);
                let elapsed = started.elapsed().as_nanos();
                assert_eq!(execution.status, expected);
                elapsed
            })
            .collect()
    }

    fn percentile_ns(samples: &[u128], percentile: usize) -> u128 {
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();
        let index = sorted
            .len()
            .saturating_mul(percentile)
            .div_ceil(100)
            .saturating_sub(1)
            .min(sorted.len().saturating_sub(1));
        sorted[index]
    }

    #[test]
    fn prepared_route_preserves_the_frozen_serving_execution() {
        let (executor, authority_snapshot) =
            authorized_executor(101, vec![active_projection_package()]);
        let payload = projection_payload();
        let compatibility = executor.execute("", &payload);
        assert_eq!(compatibility.status, ResponseExecutionStatus::Executed);

        let entry = k1_index_entry(
            "projection-package".to_owned(),
            "projection",
            &authority_snapshot,
        );
        let index = K1ActionIndexV1::new(authority_snapshot, digest_root("abstain"), vec![entry])
            .expect("index");
        let prepared = executor.evaluate_pre_action("", &payload, &index);
        assert!(prepared.request_identity_root_sha256().is_some());
        assert!(prepared.provider_identity_root_sha256().is_some());
        let evidence = prepared.k1_evidence().clone();
        let direct = executor.execute_prepared(prepared);
        assert_eq!(direct, compatibility);
        let PreparedK1EvidenceV1::Ready {
            authority_snapshot_root_sha256,
            available_actions,
            ..
        } = evidence
        else {
            panic!("K1 evidence was not prepared")
        };
        assert_eq!(
            authority_snapshot_root_sha256,
            index.authority_snapshot.snapshot_root_sha256
        );
        assert_eq!(available_actions.action_contract_roots_sha256.len(), 1);
        assert_eq!(
            available_actions.action_contract_roots_sha256[0],
            k1_projection("projection").action_contract_root_sha256
        );
    }

    #[test]
    fn authority_builder_rejects_forged_execution_payload_binding() {
        let package_id = "projection-package";
        let (executor, _) = authorized_executor(151, vec![active_projection_package()]);
        let projection = k1_projection("projection");
        let ledger = k1_certification_ledger(package_id, &projection);
        let gate = ledger.k1_vocabulary_gate().expect("K1 gate");
        let runtime_authority = executor.authority.as_ref().expect("runtime authority");
        let snapshot = DecisionAuthoritySnapshotV1::seal(
            executor.schema.clone(),
            executor.revision,
            executor.registry_root_sha256.clone(),
            runtime_authority.admission_sha256.clone(),
            ledger.revision,
            ledger.ledger_root_sha256.clone(),
            gate.gate_root_sha256,
            crate::response_runtime_contract_sha256(),
        )
        .expect("authority snapshot");
        let certification = ledger
            .latest_entries()
            .into_iter()
            .find(|entry| entry.package_id == package_id)
            .expect("certification entry");
        let admitted = runtime_authority
            .packages
            .get(package_id)
            .expect("admitted package");
        let forged_binding = OpaqueActionExecutionBindingV1::seal(
            projection.action_contract_root_sha256.clone(),
            digest_root("forged-execution-payload"),
            admitted_package_binding_root_v1(admitted).expect("admission binding root"),
            certification.entry_root_sha256.clone(),
            snapshot.response_registry_root_sha256.clone(),
            snapshot.response_registry_revision,
            snapshot.certification_ledger_root_sha256.clone(),
            snapshot.certification_ledger_revision,
        )
        .expect("forged binding");
        let entry = K1ActionIndexEntryMaterialV1 {
            package_id: package_id.to_owned(),
            projection,
            execution_binding: forged_binding,
        };

        assert_eq!(
            executor.build_k1_action_index_v1(
                snapshot,
                digest_root("abstain"),
                &ledger,
                vec![entry],
            ),
            Err("k1_action_index_execution_payload_mismatch")
        );
    }

    #[test]
    fn prepared_state_rejects_a_different_executor_snapshot() {
        let left = ResponseExecutor::from_registry(ResponseRegistry {
            schema: "nando.response-registry.v6".to_owned(),
            revision: 201,
            packages: vec![active_projection_package()],
        })
        .expect("left");
        let right = ResponseExecutor::from_registry(ResponseRegistry {
            schema: "nando.response-registry.v6".to_owned(),
            revision: 202,
            packages: vec![active_projection_package()],
        })
        .expect("right");
        let payload = projection_payload();
        let prepared = left.prepare_inner("", &payload, false, None);
        assert_eq!(
            right.execute_prepared(prepared).reason,
            "prepared_executor_snapshot_mismatch"
        );
    }

    #[test]
    fn torn_k1_authority_snapshot_censors_evidence_without_changing_serving() {
        let (executor, _) = authorized_executor(251, vec![active_projection_package()]);
        let mismatched = DecisionAuthoritySnapshotV1::seal(
            "nando.response-registry.v6".to_owned(),
            999,
            digest_root("wrong-registry"),
            digest_root("wrong-admission"),
            2,
            digest_root("wrong-certification"),
            digest_root("wrong-k1-gate"),
            crate::response_runtime_contract_sha256(),
        )
        .expect("mismatched snapshot");
        let entry = k1_index_entry(
            "projection-package".to_owned(),
            "projection-mismatch",
            &mismatched,
        );
        let index = K1ActionIndexV1::new(mismatched, digest_root("abstain-mismatch"), vec![entry])
            .expect("index");
        let payload = projection_payload();
        let compatibility = executor.execute("", &payload);
        let prepared = executor.evaluate_pre_action("", &payload, &index);
        assert_eq!(
            prepared.k1_evidence(),
            &PreparedK1EvidenceV1::Censored(PreparedK1EvidenceCensorV1::AuthoritySnapshotMismatch)
        );
        assert_eq!(executor.execute_prepared(prepared), compatibility);
    }

    #[test]
    fn k1_action_capacity_censors_evidence_but_not_serving() {
        let mut packages = Vec::new();
        for index in 0..257 {
            let package_id = format!("projection-package-{index:03}");
            let mut package = active_projection_package();
            package.package_id = package_id.clone();
            packages.push(package);
        }
        let (executor, authority_snapshot) = authorized_executor(301, packages);
        let entries = (0..257)
            .map(|index| {
                k1_index_entry(
                    format!("projection-package-{index:03}"),
                    &format!("projection-{index:03}"),
                    &authority_snapshot,
                )
            })
            .collect();
        let index =
            K1ActionIndexV1::new(authority_snapshot, digest_root("abstain-capacity"), entries)
                .expect("index");
        let payload = projection_payload();
        let compatibility = executor.execute("", &payload);
        let prepared = executor.evaluate_pre_action("", &payload, &index);
        assert_eq!(
            prepared.k1_evidence(),
            &PreparedK1EvidenceV1::Censored(PreparedK1EvidenceCensorV1::CapacityExhausted)
        );
        assert_eq!(executor.execute_prepared(prepared), compatibility);
    }

    fn adaptive_request_last_token_package() -> ResponsePackage {
        let selector = ResponseValueSelector::RequestLastToken;
        let program = ResponseProgram::project_selected_value(
            selector.clone(),
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let program_root = nando_operator_kernel::response_program_version_root_sha256(&program)
            .expect("program root");
        let proof = nando_operator_admission::seal_adaptive_identification_proof_v1(
            nando_operator_admission::AdaptiveIdentificationProofInputV1 {
                candidate_freeze_root_sha256: "11".repeat(32),
                semantic_class_id_sha256: "22".repeat(32),
                canonical_program_root_sha256: program_root,
                applicability_scope_root_sha256: "33".repeat(32),
                transfer_proof_root_sha256: "44".repeat(32),
            },
        )
        .expect("adaptive proof");
        let required_routing_atom_ids = response_program_required_routing_atom_ids(&program);
        ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: "request-last-token-package".to_owned(),
            origin: ResponsePackageOrigin::GroundedSynthesis,
            state: ResponsePackageState::Active,
            program,
            verifier: Some(VerifierProgram::ProjectSelectedValue {
                selector,
                format: ValueProjectionFormat::PlainText,
                renderer: crate::CollectionOutputRenderer::Direct,
                completion_state: "completed".to_owned(),
                require_unique_value: true,
            }),
            routing_predicates: Vec::new(),
            phase_centers: required_routing_atom_ids.clone(),
            required_routing_atom_ids,
            anti_centers: Vec::new(),
            wave_margin_micro: 1,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: ResponsePackageProof {
                support_rows: 1,
                future_rows: 1,
                distinct_sessions: 2,
                distinct_surfaces: 2,
                wrong_accepts: 0,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass: true,
                verifier_schema: VALUE_PROJECTION_EXTERNAL_VERIFIER_SCHEMA.to_owned(),
                adaptive_identification: Some(proof),
            },
        }
    }

    #[test]
    fn request_last_token_cannot_gain_authority_without_applicability_negatives() {
        let mut package = adaptive_request_last_token_package();
        assert_eq!(
            package.admission_candidate_blocker(),
            Some("semantic_applicability_guard_missing")
        );
        assert_eq!(
            runtime_applicability_margin_micro(
                &package,
                "Explain why this answer is broken",
                &serde_json::json!({"input":[]}),
            ),
            None
        );

        package.anti_centers = vec![stable_atom_id("intent:ordinary_question")];
        assert_eq!(package.admission_candidate_blocker(), None);
    }

    #[test]
    fn registry_drops_request_last_token_without_applicability_authority() {
        let executor = ResponseExecutor::from_registry(ResponseRegistry {
            schema: "nando.response-registry.v6".to_owned(),
            revision: 1,
            packages: vec![adaptive_request_last_token_package()],
        })
        .expect("valid quarantine registry");
        let execution = executor.execute_shadow(
            "Explain why this answer is broken",
            &serde_json::json!({
                "input": [{
                    "role": "user",
                    "content": "Explain why this answer is broken"
                }]
            }),
        );
        assert_eq!(execution.status, ResponseExecutionStatus::Abstain);
        assert!(
            execution
                .reason
                .starts_with("no_phase_routed_profile:packages=0")
        );
    }

    #[test]
    fn value_projection_package_requires_an_exact_independent_verifier_binding() {
        let package = active_projection_package();
        assert!(package.eligible_for_admission_candidate());

        let mut mutated = package;
        if let Some(VerifierProgram::ProjectSelectedValue {
            completion_state, ..
        }) = &mut mutated.verifier
        {
            *completion_state = "pending".to_owned();
        }
        assert!(!mutated.eligible_for_admission_candidate());
    }

    #[test]
    fn terminal_projection_without_applicability_negatives_is_quarantined() {
        let selector = ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal: 0,
            value_type: AtomValueType::String,
        };
        let mut package = adaptive_request_last_token_package();
        package.program = ResponseProgram::project_selected_value(
            selector.clone(),
            ValueProjectionFormat::PlainText,
            "completed",
        );
        package.verifier = Some(VerifierProgram::ProjectSelectedValue {
            selector,
            format: ValueProjectionFormat::PlainText,
            renderer: crate::CollectionOutputRenderer::Direct,
            completion_state: "completed".to_owned(),
            require_unique_value: true,
        });
        let canonical_program_root_sha256 =
            nando_operator_kernel::response_program_version_root_sha256(&package.program)
                .expect("program root");
        package.proof.adaptive_identification = Some(
            nando_operator_admission::seal_adaptive_identification_proof_v1(
                nando_operator_admission::AdaptiveIdentificationProofInputV1 {
                    candidate_freeze_root_sha256: "11".repeat(32),
                    semantic_class_id_sha256: "22".repeat(32),
                    canonical_program_root_sha256,
                    applicability_scope_root_sha256: "33".repeat(32),
                    transfer_proof_root_sha256: "44".repeat(32),
                },
            )
            .expect("adaptive proof"),
        );
        assert_eq!(
            package.admission_candidate_blocker(),
            Some("semantic_applicability_guard_missing")
        );
    }

    #[test]
    fn status_package_requires_external_evidence_and_exact_verifier_binding() {
        let package = active_status_package();
        assert_eq!(package.validate(), Ok(()));
        assert!(package.eligible_for_admission_candidate());

        let mut wrong_schema = package.clone();
        wrong_schema.proof.verifier_schema = VALUE_PROJECTION_EXTERNAL_VERIFIER_SCHEMA.to_owned();
        assert_eq!(
            wrong_schema.validate(),
            Err("status_projection_external_evidence_required")
        );

        let mut mismatched = package;
        if let Some(VerifierProgram::ProjectStatus { selector, .. }) = &mut mismatched.verifier {
            *selector = ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::Integer,
            };
        }
        assert_eq!(
            mismatched.validate(),
            Err("status_projection_actor_verifier_mismatch")
        );
        assert!(!mismatched.eligible_for_admission_candidate());
    }

    fn current_v4_wait_package() -> ResponsePackage {
        ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: "current-v4-wait".to_owned(),
            origin: ResponsePackageOrigin::LegacyTemplate,
            state: ResponsePackageState::Quarantine,
            program: ResponseProgram::wait_on_any_yielded_cell(),
            verifier: None,
            routing_predicates: Vec::new(),
            required_routing_atom_ids: Vec::new(),
            phase_centers: vec![1],
            anti_centers: Vec::new(),
            wave_margin_micro: 1,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: ResponsePackageProof {
                support_rows: 32,
                future_rows: 0,
                distinct_sessions: 1,
                distinct_surfaces: 1,
                wrong_accepts: 0,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass: false,
                verifier_schema: "response_actor_independent_verifier.v1".to_owned(),
                adaptive_identification: None,
            },
        }
    }

    #[test]
    fn response_registry_dual_reads_v4_but_never_allows_v5_operations_to_hide_in_it() {
        let v4 = ResponseRegistry {
            schema: "nando.response-registry.v4".to_owned(),
            revision: 39,
            packages: vec![current_v4_wait_package()],
        };
        assert_eq!(v4.validate(), Ok(()));
        assert!(ResponseExecutor::from_registry(v4).is_ok());

        assert_eq!(
            ResponseRegistry {
                schema: "nando.response-registry.v4".to_owned(),
                revision: 1,
                packages: vec![active_projection_package()],
            }
            .validate(),
            Err("registry_v4_value_projection_unsupported")
        );
        assert_eq!(
            ResponseRegistry {
                schema: "nando.response-registry.v5".to_owned(),
                revision: 1,
                packages: vec![active_projection_package()],
            }
            .validate(),
            Ok(())
        );
        assert_eq!(
            ResponseRegistry {
                schema: "nando.response-registry.v4".to_owned(),
                revision: 1,
                packages: vec![active_status_package()],
            }
            .validate(),
            Err("registry_v4_status_projection_unsupported")
        );
        assert_eq!(
            ResponseRegistry {
                schema: "nando.response-registry.v5".to_owned(),
                revision: 1,
                packages: vec![active_status_package()],
            }
            .validate(),
            Ok(())
        );
        assert_eq!(
            ResponseRegistry {
                schema: "nando.response-registry.v3".to_owned(),
                revision: 1,
                packages: vec![current_v4_wait_package()],
            }
            .validate(),
            Err("unsupported_registry_schema")
        );
    }

    #[test]
    fn action_value_projection_never_becomes_a_runtime_routing_atom() {
        let frame = RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: "1".repeat(64),
            event_id_sha256: "2".repeat(64),
            client_intent_id_sha256: "3".repeat(64),
            session_id_sha256: "4".repeat(64),
            observed_at_unix_nanos: 1,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            estimated_input_tokens: 55,
            atoms: vec![
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
                RelationAtom::ActionValueProjection {
                    format: ValueProjectionFormat::PlainText,
                    renderer: crate::CollectionOutputRenderer::Direct,
                },
            ],
            evidence_ref_sha256: "5".repeat(64),
        };
        let mut without_target = frame.clone();
        without_target
            .atoms
            .retain(|atom| !matches!(atom, RelationAtom::ActionValueProjection { .. }));
        assert_eq!(
            relation_frame_routing_atom_ids(&frame),
            relation_frame_routing_atom_ids(&without_target)
        );
    }

    #[test]
    fn balanced_hidden_wave_keeps_every_cross_layer_kind() {
        let request = (0_u64..6)
            .map(|id| (600, id.saturating_add(10)))
            .collect::<Vec<_>>();
        let state = (0_u64..6)
            .map(|id| (900, id.saturating_add(20)))
            .collect::<Vec<_>>();
        let tool = (0_u64..6)
            .map(|id| (1_000, id.saturating_add(30)))
            .collect::<Vec<_>>();
        let balanced = balanced_wave_hidden_atom_ids(&request, &state, &tool, 12);
        assert_eq!(balanced.len(), 12);

        let expected_groups = [
            ranked_wave_pairs(1, &request, &state),
            ranked_wave_pairs(2, &state, &tool),
            ranked_wave_pairs(3, &request, &tool),
            ranked_wave_triples(&request, &state, &tool),
        ];
        for expected in expected_groups {
            let expected = expected
                .into_iter()
                .map(|(_, atom_id)| atom_id)
                .collect::<BTreeSet<_>>();
            assert_eq!(
                balanced
                    .iter()
                    .filter(|atom_id| expected.contains(atom_id))
                    .count(),
                3
            );
        }
    }

    #[test]
    fn hidden_wave_atoms_ignore_teacher_action_and_outcome() {
        let mut frame = RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: "1".repeat(64),
            event_id_sha256: "2".repeat(64),
            client_intent_id_sha256: "3".repeat(64),
            session_id_sha256: "4".repeat(64),
            observed_at_unix_nanos: 1,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            estimated_input_tokens: 55,
            atoms: vec![
                RelationAtom::RequestPhaseAtom { atom_id: 11 },
                RelationAtom::Cardinality {
                    role: "records".to_owned(),
                    count: 3,
                },
                RelationAtom::ToolKind {
                    value: "exec".to_owned(),
                },
                RelationAtom::CompletionState {
                    value: "pending".to_owned(),
                },
            ],
            evidence_ref_sha256: "5".repeat(64),
        };
        let before = relation_frame_hidden_wave_atom_ids(&frame);
        frame.atoms.extend([
            RelationAtom::ActionFunction {
                value: "wait".to_owned(),
            },
            RelationAtom::ResponseShape {
                value: "function_call".to_owned(),
            },
        ]);
        assert_eq!(before, relation_frame_hidden_wave_atom_ids(&frame));
    }

    #[test]
    fn learned_wave_route_filters_unknown_future_atoms_after_exact_guard() {
        let mut package = active_projection_package();
        let required = stable_atom_id("tool_kind:exec");
        let completion = stable_atom_id("completion:completed");
        package.required_routing_atom_ids = vec![required];
        let mut feature_vocabulary = vec![required, completion];
        feature_vocabulary.sort_unstable();
        package.learned_wave_route = Some(LearnedWaveRoute {
            cells: 16,
            center_delta_micro: vec![0; 32],
            threshold_micro: 1,
            query_atom_ids: feature_vocabulary.clone(),
            subcenters: Vec::new(),
        });
        let frame = RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: "1".repeat(64),
            event_id_sha256: "2".repeat(64),
            client_intent_id_sha256: "3".repeat(64),
            session_id_sha256: "4".repeat(64),
            observed_at_unix_nanos: 1,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            estimated_input_tokens: 55,
            atoms: vec![
                RelationAtom::ToolKind {
                    value: "exec".to_owned(),
                },
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
                RelationAtom::RequestPhaseAtom { atom_id: 999_999 },
            ],
            evidence_ref_sha256: "5".repeat(64),
        };

        let query = relation_frame_routing_query_atom_ids(&package, &frame).expect("guard");
        assert_eq!(query, feature_vocabulary);
    }

    #[test]
    fn learned_wave_subcenter_routes_when_primary_center_abstains() {
        let mut package = active_projection_package();
        package.required_routing_atom_ids.clear();
        let frame = RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: "1".repeat(64),
            event_id_sha256: "2".repeat(64),
            client_intent_id_sha256: "3".repeat(64),
            session_id_sha256: "4".repeat(64),
            observed_at_unix_nanos: 1,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            estimated_input_tokens: 55,
            atoms: vec![
                RelationAtom::ToolKind {
                    value: "exec".to_owned(),
                },
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
            ],
            evidence_ref_sha256: "5".repeat(64),
        };
        let query = phase_vector_from_atom_ids(relation_frame_online_routing_atom_ids(&frame), 16);
        let positive = query
            .iter()
            .flat_map(|cell| {
                [
                    (cell.re * 1_000_000.0).round() as i32,
                    (cell.im * 1_000_000.0).round() as i32,
                ]
            })
            .collect::<Vec<_>>();
        let negative = positive
            .iter()
            .map(|value| value.saturating_neg())
            .collect();
        package.wave_margin_micro = 1;
        package.learned_wave_route = Some(LearnedWaveRoute {
            cells: 16,
            center_delta_micro: negative,
            threshold_micro: 1,
            query_atom_ids: Vec::new(),
            subcenters: vec![LearnedWaveSubcenter {
                center_delta_micro: positive,
                threshold_micro: 1,
            }],
        });

        assert!(relation_frame_routes_to_package(&package, &frame));
        package
            .learned_wave_route
            .as_mut()
            .expect("route")
            .subcenters
            .clear();
        assert!(!relation_frame_routes_to_package(&package, &frame));
    }

    #[test]
    fn request_phase_encoder_matches_capture_fnv_contract() {
        assert!(
            request_phase_atom_ids("Please count these rows").contains(&15_291_052_347_829_727_369)
        );
        assert!(
            request_phase_atom_ids("Пожалуйста, посчитай строки")
                .contains(&13_856_698_933_407_100_379)
        );
    }

    #[test]
    fn request_phase_encoder_retains_tail_intent_with_bounded_size() {
        let text = (0..100)
            .map(|index| format!("token{index}"))
            .chain(["FINAL_INTENT".to_owned()])
            .collect::<Vec<_>>()
            .join(" ");
        let atoms = request_phase_atom_ids(&text);
        assert!(atoms.contains(&stable_atom_id("request_token:final_intent")));
        assert!(atoms.len() <= 127);
    }

    #[test]
    fn capability_encoder_reads_only_pre_action_declarations() {
        let top_level = serde_json::json!({
            "tools": [{"type":"function","name":"wait"}],
            "input": [{"type":"function_call","name":"must_not_be_a_capability"}]
        });
        assert_eq!(
            provider_tool_capability_atom_ids(&top_level),
            vec![stable_atom_id("client_capability:function:wait")]
        );

        let additional = serde_json::json!({
            "input": [{
                "type":"additional_tools",
                "tools":[{"type":"custom","name":"exec"}]
            }]
        });
        assert_eq!(
            provider_tool_capability_atom_ids(&additional),
            vec![stable_atom_id("client_capability:custom:exec")]
        );
    }

    #[test]
    fn allocation_free_phase_hashing_preserves_existing_atom_ids() {
        let cases = [
            (
                RelationAtom::TypedSlot {
                    slot_id: 7,
                    value_type: AtomValueType::Identifier,
                    source: AtomSource::Observation,
                    value_sha256: "a".repeat(64),
                },
                "slot:identifier:observation".to_owned(),
            ),
            (
                RelationAtom::ObservationSelector {
                    slot_id: 3,
                    selector: ResponseValueSelector::TurnOutputLine {
                        output_ordinal: 2,
                        line_index: 5,
                        value_type: AtomValueType::String,
                    },
                },
                format!(
                    "selector:{}",
                    canonical_response_value_selector(&ResponseValueSelector::TurnOutputLine {
                        output_ordinal: 2,
                        line_index: 5,
                        value_type: AtomValueType::String,
                    })
                ),
            ),
            (
                RelationAtom::Cardinality {
                    role: "turn_output_count_band".to_owned(),
                    count: 4,
                },
                "cardinality:turn_output_count_band:4".to_owned(),
            ),
            (
                RelationAtom::ActionFunction {
                    value: "functions.wait".to_owned(),
                },
                "action_function:functions.wait".to_owned(),
            ),
            (
                RelationAtom::ActionResultProjection {
                    output_field: "output".to_owned(),
                    continuation_field: "cell_id".to_owned(),
                    continuation_prefix: "cell-".to_owned(),
                },
                "action_result_projection:output:cell_id:cell-".to_owned(),
            ),
            (
                RelationAtom::OutputStatus {
                    value: "completed".to_owned(),
                },
                "status:completed".to_owned(),
            ),
            (
                RelationAtom::ResponseShape {
                    value: "multi_claim".to_owned(),
                },
                "shape:multi_claim".to_owned(),
            ),
            (
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
                "completion:completed".to_owned(),
            ),
        ];

        for (atom, previous_canonical_form) in cases {
            assert_eq!(
                relation_atom_phase_id(&atom),
                stable_atom_id(&previous_canonical_form),
                "phase atom ID changed for {atom:?}"
            );
        }

        let parts = ["action_role_argument:", "continuation_handle"];
        assert_eq!(
            stable_atom_id_parts(&parts),
            stable_atom_id(&parts.concat())
        );
    }
}
