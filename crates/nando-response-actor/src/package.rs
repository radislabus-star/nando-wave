use nando_core::wave::{phase_coherence, phase_margin_to_micro, phase_vector_from_atom_ids};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::authority::{
    CompositeResponseAdmissionV2, FinalizedRuntimeVerificationReceiptV2,
    IndependentlyVerifiedExecution, ValidatedResponseAuthority, canonical_json_sha256,
    finalize_runtime_receipt, validate_response_authority,
};
use crate::contracts::canonical_response_value_selector;
use crate::runtime::immediate_selected_scalar;

use crate::{
    AtomSource, AtomValueType, RelationAtom, RelationFrame, ResponseArgument,
    ResponseExecutionStatus, ResponseOperation, ResponseProgram, ResponseValueSelector,
    SemanticRole, VerifiedCrystallizedOperator, VerifiedOperatorRestartBundle, VerifierProgram,
    execute_response, verify_response_independently,
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponsePackageOrigin {
    GroundedSynthesis,
    LegacyTemplate,
    // Compatibility value for registries produced before authority was split.
    RawPhaseInduction,
    ImportedFixture,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponsePackageState {
    Quarantine,
    Active,
    Revoked,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseRoutingComparison {
    AtMost,
    AtLeast,
    OneOf,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ResponseRoutingPredicate {
    pub role: String,
    pub comparison: ResponseRoutingComparison,
    pub threshold: u32,
    #[serde(default)]
    pub allowed_counts: Vec<u32>,
}

impl ResponseRoutingPredicate {
    #[must_use]
    pub fn matches_count(&self, count: u32) -> bool {
        match self.comparison {
            ResponseRoutingComparison::AtMost => count <= self.threshold,
            ResponseRoutingComparison::AtLeast => count >= self.threshold,
            ResponseRoutingComparison::OneOf => self.allowed_counts.binary_search(&count).is_ok(),
        }
    }

    #[must_use]
    pub fn phase_atom_id(&self) -> u64 {
        let material = match self.comparison {
            ResponseRoutingComparison::AtMost => {
                format!("cardinality_at_most:{}:{}", self.role, self.threshold)
            }
            ResponseRoutingComparison::AtLeast => {
                format!("cardinality_at_least:{}:{}", self.role, self.threshold)
            }
            ResponseRoutingComparison::OneOf => format!(
                "cardinality_one_of:{}:{}",
                self.role,
                self.allowed_counts
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        };
        stable_atom_id(&material)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponsePackageProof {
    pub support_rows: usize,
    pub future_rows: usize,
    pub distinct_sessions: usize,
    pub distinct_surfaces: usize,
    pub wrong_accepts: usize,
    pub runtime_parity_failures: usize,
    pub exact_cache_overlap: usize,
    pub wave_causal_pass: bool,
    pub verifier_schema: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LearnedWaveSubcenter {
    pub center_delta_micro: Vec<i32>,
    pub threshold_micro: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LearnedWaveRoute {
    pub cells: u16,
    pub center_delta_micro: Vec<i32>,
    pub threshold_micro: i64,
    #[serde(default)]
    pub query_atom_ids: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subcenters: Vec<LearnedWaveSubcenter>,
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
            let operator =
                VerifiedCrystallizedOperator::restore(bundle.page_bytes(), bundle.registry_cbor())
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
        let required_atoms = response_program_required_routing_atom_ids(&self.program);
        let exact_guard_bound = !required_atoms.is_empty()
            && required_atoms
                .iter()
                .all(|atom| self.required_routing_atom_ids.binary_search(atom).is_ok());
        if let Err(blocker) = self.validate() {
            Some(blocker)
        } else if !grounded_authority {
            Some("grounded_authority_missing")
        } else if self.state != ResponsePackageState::Active {
            Some("package_not_active")
        } else if self.proof.support_rows < 32 {
            Some("support_rows_below_32")
        } else if self.proof.future_rows < 32 {
            Some("future_rows_below_32")
        } else if self.proof.distinct_sessions < 3 {
            Some("future_sessions_below_3")
        } else if self.proof.distinct_surfaces < 2 {
            Some("surfaces_below_2")
        } else if self.proof.wrong_accepts != 0 {
            Some("wrong_accepts_nonzero")
        } else if self.proof.runtime_parity_failures != 0 {
            Some("runtime_parity_failures_nonzero")
        } else if self.proof.exact_cache_overlap != 0 {
            Some("exact_cache_overlap_nonzero")
        } else if !self.proof.wave_causal_pass {
            Some("wave_causal_proof_missing")
        } else if !verifier_bound {
            Some("verifier_schema_not_bound")
        } else if !verifier_program_bound {
            Some("verifier_program_not_bound")
        } else if !exact_guard_bound {
            Some("exact_guard_not_bound")
        } else {
            None
        }
    }

    /// Package state and counters are only admission inputs, never execution authority.
    #[must_use]
    pub const fn eligible_for_local_accept(&self) -> bool {
        false
    }
}

#[must_use]
pub fn response_program_required_routing_atom_ids(program: &ResponseProgram) -> Vec<u64> {
    let mut atoms = match &program.operation {
        ResponseOperation::UniqueConsensus { variants, .. } => {
            let mut variants = variants.iter();
            let Some(first) = variants.next() else {
                return Vec::new();
            };
            let mut common = response_program_required_routing_atom_ids(&first.program)
                .into_iter()
                .collect::<BTreeSet<_>>();
            for variant in variants {
                let atoms = response_program_required_routing_atom_ids(&variant.program)
                    .into_iter()
                    .collect::<BTreeSet<_>>();
                common.retain(|atom| atoms.contains(atom));
            }
            common.into_iter().collect()
        }
        ResponseOperation::AdvancePlan { function_name } => vec![
            stable_atom_id("relation:plan_state"),
            stable_atom_id("status:success"),
            stable_atom_id(&format!("client_capability:function:{function_name}")),
        ],
        ResponseOperation::FunctionCallFromRoles {
            selector,
            arguments,
            ..
        } => {
            let completion = if arguments.iter().any(|argument| {
                matches!(
                    argument,
                    ResponseArgument::Role {
                        role: SemanticRole::ContinuationHandle,
                        ..
                    }
                )
            }) {
                "pending"
            } else {
                "completed"
            };
            vec![
                stable_atom_id(&format!("completion:{completion}")),
                stable_atom_id("relation:unique_slot"),
                selector_phase_atom_id(selector),
            ]
        }
        ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name, ..
        } => vec![
            stable_atom_id("completion:pending"),
            stable_atom_id("relation:unique_slot"),
            stable_atom_id(&format!("client_capability:custom:{custom_tool_name}")),
        ],
        ResponseOperation::ProjectSelectedValue {
            selector,
            completion_state,
            ..
        } => vec![
            stable_atom_id(&format!("completion:{completion_state}")),
            stable_atom_id("relation:unique_slot"),
            selector_phase_atom_id(selector),
        ],
        ResponseOperation::ProjectStatus {
            selector,
            completion_state,
            ..
        } => vec![
            stable_atom_id(&format!("completion:{completion_state}")),
            stable_atom_id("relation:unique_slot"),
            selector_phase_atom_id(selector),
        ],
        ResponseOperation::ComposeCollection {
            completion_state, ..
        } => vec![
            stable_atom_id(&format!("completion:{completion_state}")),
            stable_atom_id("observation:json_collection"),
        ],
        _ => Vec::new(),
    };
    atoms.sort_unstable();
    atoms.dedup();
    atoms
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

#[derive(Clone, Debug)]
pub struct ResponseExecutor {
    schema: String,
    revision: u64,
    packages: Vec<ResponsePackage>,
    crystallized_operators: BTreeMap<String, VerifiedCrystallizedOperator>,
    authority: Option<ValidatedResponseAuthority>,
}

impl ResponseExecutor {
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| format!("response_registry_read:{error}"))?;
        let registry: ResponseRegistry = serde_json::from_slice(&bytes)
            .map_err(|error| format!("response_registry_parse:{error}"))?;
        Self::from_registry(registry).map_err(str::to_owned)
    }

    pub fn from_registry(registry: ResponseRegistry) -> Result<Self, &'static str> {
        registry.validate()?;
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
        let authority = validate_response_authority(
            &registry,
            &admission,
            expected_project_id,
            expected_gate_build_sha256,
            expected_runtime_build_sha256,
            now_unix,
            max_age_seconds,
        )?;
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
        if self.authority.is_none() {
            return rejected("execution_authority_missing");
        }
        self.execute_inner(request_text, provider_payload, true)
    }

    #[must_use]
    pub fn execute_shadow(
        &self,
        request_text: &str,
        provider_payload: &Value,
    ) -> RoutedResponseExecution {
        self.execute_inner(request_text, provider_payload, false)
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

    fn execute_inner(
        &self,
        request_text: &str,
        provider_payload: &Value,
        require_authority: bool,
    ) -> RoutedResponseExecution {
        let context_counts = response_pre_action_context_counts(provider_payload);
        let mut predicate_matches = 0_usize;
        let mut grounded_matches = 0_usize;
        let mut guard_matches = 0_usize;
        let mut best_margin = i64::MIN;
        let mut best_threshold = 0_i64;
        let mut ranked = [None; 8];
        for package in &self.packages {
            if !routing_predicates_match_counts(&package.routing_predicates, &context_counts) {
                continue;
            }
            predicate_matches = predicate_matches.saturating_add(1);
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
                insert_top_response_candidate(&mut ranked, (margin, package));
            }
        }
        let Some((top_margin, package)) = ranked[0] else {
            return rejected(format!(
                "no_phase_routed_profile:packages={}:predicates={predicate_matches}:grounded={grounded_matches}:guard={guard_matches}:best_margin={best_margin}:best_threshold={best_threshold}",
                self.packages.len()
            ));
        };
        if ranked[1].is_some_and(|(margin, _)| margin == top_margin) {
            return rejected("ambiguous_phase_route");
        }
        let (execution_status, execution_reason, execution_response, independently_verified) =
            if let Some(operator) = self.crystallized_operators.get(&package.package_id) {
                let bound = match operator.bind_pre_action(request_text, provider_payload) {
                    Ok(bound) => bound,
                    Err(error) => {
                        return rejected(format!("crystallized_role_binding:{error:?}"));
                    }
                };
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
            phase_candidates: ranked.len(),
            exact_actor_checks: 1,
            phase_margin_micro: Some(top_margin),
            verified,
        }
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
        let operator =
            VerifiedCrystallizedOperator::restore(bundle.page_bytes(), bundle.registry_cbor())
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
    atom.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

pub fn request_phase_atom_ids(text: &str) -> Vec<u64> {
    let all_tokens = text
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|token| !token.is_empty() && token.len() <= 32)
        .map(str::to_lowercase)
        .take(256)
        .collect::<Vec<_>>();
    let tokens = if all_tokens.len() <= 64 {
        all_tokens
    } else {
        all_tokens[..32]
            .iter()
            .chain(&all_tokens[all_tokens.len().saturating_sub(32)..])
            .cloned()
            .collect()
    };
    let mut atoms = tokens
        .iter()
        .map(|token| stable_atom_id(&format!("request_token:{token}")))
        .collect::<Vec<_>>();
    atoms.extend(
        tokens
            .windows(2)
            .map(|pair| stable_atom_id(&format!("request_bigram:{}:{}", pair[0], pair[1]))),
    );
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

/// Pre-action tool protocol capabilities advertised by the client. Only the
/// provider request's declarations are inspected; historical calls and model
/// outputs are deliberately excluded.
#[must_use]
pub fn provider_tool_capability_atom_ids(provider_payload: &Value) -> Vec<u64> {
    let mut declarations = Vec::new();
    if let Some(tools) = provider_payload.get("tools").and_then(Value::as_array) {
        declarations.extend(tools.iter());
    }
    if let Some(input) = provider_payload.get("input").and_then(Value::as_array) {
        for item in input
            .iter()
            .filter(|item| item.get("type").and_then(Value::as_str) == Some("additional_tools"))
        {
            for key in ["tools", "additional_tools", "definitions", "items"] {
                if let Some(tools) = item.get(key).and_then(Value::as_array) {
                    declarations.extend(tools.iter());
                }
            }
        }
    }
    let mut atoms = declarations
        .into_iter()
        .filter_map(|declaration| {
            let raw_kind = declaration
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("function");
            let kind = match raw_kind {
                "custom" | "custom_tool" => "custom",
                "function" => "function",
                other => other,
            };
            let name = declaration
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| {
                    declaration
                        .pointer("/function/name")
                        .and_then(Value::as_str)
                })?;
            valid_capability_symbol(kind)
                .then_some(())
                .filter(|_| valid_capability_symbol(name))
                .map(|()| stable_atom_id(&format!("client_capability:{kind}:{name}")))
        })
        .collect::<Vec<_>>();
    atoms.sort_unstable();
    atoms.dedup();
    atoms.truncate(64);
    atoms
}

fn valid_capability_symbol(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/' | b':')
        })
}

pub fn relation_frame_phase_atom_ids(frame: &RelationFrame) -> Vec<u64> {
    let mut ids = frame
        .atoms
        .iter()
        .map(relation_atom_phase_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn relation_atom_phase_id(atom: &RelationAtom) -> u64 {
    match atom {
        RelationAtom::TypedSlot {
            value_type, source, ..
        } => stable_atom_id_parts(&[
            "slot:",
            value_type_name(*value_type),
            ":",
            source_name(*source),
        ]),
        RelationAtom::SlotEquality { .. } => stable_atom_id("relation:slot_equality"),
        RelationAtom::UniqueSlot { .. } => stable_atom_id("relation:unique_slot"),
        RelationAtom::ObservationSelector { selector, .. } => selector_phase_atom_id(selector),
        RelationAtom::ObservationCallShape { value } => {
            stable_atom_id_parts(&["observation_call_shape:", value])
        }
        RelationAtom::ActionFunction { value } => {
            stable_atom_id_parts(&["action_function:", value])
        }
        RelationAtom::ActionCustomTool { value } => {
            stable_atom_id_parts(&["action_custom_tool:", value])
        }
        RelationAtom::ActionInnerTool { value } => {
            stable_atom_id_parts(&["action_inner_tool:", value])
        }
        RelationAtom::ActionRoleArgument { name, .. } => {
            stable_atom_id_parts(&["action_role_argument:", name])
        }
        RelationAtom::ActionIntegerArgument { name, .. } => {
            stable_atom_id_parts(&["action_integer_argument:", name])
        }
        RelationAtom::ActionStringArgument { name, .. } => {
            stable_atom_id_parts(&["action_string_argument:", name])
        }
        RelationAtom::ActionBooleanArgument { name, .. } => {
            stable_atom_id_parts(&["action_boolean_argument:", name])
        }
        RelationAtom::PlanState { .. } => stable_atom_id("relation:plan_state"),
        RelationAtom::ActionPlanAdvance => stable_atom_id("action_plan_advance"),
        RelationAtom::ActionResultProjection {
            output_field,
            continuation_field,
            continuation_prefix,
        } => stable_atom_id_parts(&[
            "action_result_projection:",
            output_field,
            ":",
            continuation_field,
            ":",
            continuation_prefix,
        ]),
        RelationAtom::ActionOutputProjection { output_field } => {
            stable_atom_id_parts(&["action_output_projection:", output_field])
        }
        RelationAtom::ActionJsonResultProjection => stable_atom_id("action_json_result_projection"),
        RelationAtom::ActionValueProjection { format, renderer } => stable_atom_id(&format!(
            "action_value_projection:{}:{}",
            match format {
                crate::ValueProjectionFormat::PlainText => "plain_text",
                crate::ValueProjectionFormat::CanonicalJson => "canonical_json",
            },
            serde_json::to_string(renderer).unwrap_or_default(),
        )),
        RelationAtom::ActionStatusProjection { mapping } => match mapping {
            crate::ProjectStatusMapping::ZeroIsSuccess => {
                stable_atom_id("action_status_projection:ZeroIsSuccess")
            }
            crate::ProjectStatusMapping::ZeroIsPass => {
                stable_atom_id("action_status_projection:ZeroIsPass")
            }
            crate::ProjectStatusMapping::ZeroIsOk => {
                stable_atom_id("action_status_projection:ZeroIsOk")
            }
            crate::ProjectStatusMapping::ZeroIsTrue => {
                stable_atom_id("action_status_projection:ZeroIsTrue")
            }
        },
        RelationAtom::CollectionShape { .. } => stable_atom_id("observation:json_collection"),
        RelationAtom::RequestPhaseAtom { atom_id } => *atom_id,
        RelationAtom::ClientCapabilityAtom { atom_id } => *atom_id,
        RelationAtom::ReconstructedClientCapabilityAtom { atom_id } => *atom_id,
        RelationAtom::ToolKind { .. } => stable_atom_id("relation:tool_kind"),
        RelationAtom::OutputStatus { value } => stable_atom_id_parts(&["status:", value]),
        RelationAtom::TypedEquality { .. } => stable_atom_id("relation:typed_equality"),
        RelationAtom::Cardinality { role, count } => {
            let count = count.to_string();
            stable_atom_id_parts(&["cardinality:", role, ":", &count])
        }
        RelationAtom::TemporalEdge { .. } => stable_atom_id("relation:temporal_edge"),
        RelationAtom::ResponseShape { value } => stable_atom_id_parts(&["shape:", value]),
        RelationAtom::CompletionState { value } => stable_atom_id_parts(&["completion:", value]),
    }
}

pub fn relation_frame_routing_atom_ids(frame: &RelationFrame) -> Vec<u64> {
    let mut ids = frame
        .atoms
        .iter()
        .filter(|atom| {
            !matches!(
                atom,
                RelationAtom::TypedSlot {
                    source: AtomSource::Action | AtomSource::Outcome,
                    ..
                } | RelationAtom::SlotEquality { .. }
                    | RelationAtom::ActionFunction { .. }
                    | RelationAtom::ActionCustomTool { .. }
                    | RelationAtom::ActionInnerTool { .. }
                    | RelationAtom::ActionRoleArgument { .. }
                    | RelationAtom::ActionIntegerArgument { .. }
                    | RelationAtom::ActionStringArgument { .. }
                    | RelationAtom::ActionBooleanArgument { .. }
                    | RelationAtom::ActionResultProjection { .. }
                    | RelationAtom::ActionOutputProjection { .. }
                    | RelationAtom::ActionJsonResultProjection
                    | RelationAtom::ActionValueProjection { .. }
                    | RelationAtom::ActionStatusProjection { .. }
                    | RelationAtom::ReconstructedClientCapabilityAtom { .. }
                    | RelationAtom::ResponseShape { .. }
            )
        })
        .map(relation_atom_phase_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// Online-learning namespace may preserve an observed protocol symbol while
/// the stable serving/package ABI continues to emit its legacy generic atom.
#[must_use]
pub fn relation_frame_online_routing_atom_ids(frame: &RelationFrame) -> Vec<u64> {
    let mut ids = relation_frame_routing_atom_ids(frame);
    let continuation_pending = frame.atoms.iter().any(|atom| {
        matches!(
            atom,
            RelationAtom::ObservationSelector {
                selector: ResponseValueSelector::ContentLinePrefix { prefix, .. },
                ..
            } if prefix == "Script running with cell ID "
                || prefix == "Process running with session ID "
        )
    });
    if continuation_pending {
        ids.retain(|atom| *atom != stable_atom_id("completion:completed"));
        ids.push(stable_atom_id("completion:pending"));
    }
    ids.extend(frame.atoms.iter().filter_map(|atom| match atom {
        RelationAtom::ToolKind { value } => Some(stable_atom_id_parts(&["tool_kind:", value])),
        _ => None,
    }));
    ids.extend(relation_frame_hidden_wave_atom_ids(frame));
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[derive(Clone, Copy)]
enum WaveAtomLayer {
    Request,
    State,
    Tool,
}

pub(crate) fn relation_frame_hidden_wave_atom_ids(frame: &RelationFrame) -> Vec<u64> {
    const BASIS_LIMIT: usize = 6;
    const LEGACY_HIDDEN_LIMIT: usize = 12;
    const BALANCED_HIDDEN_LIMIT: usize = 12;

    let pre_action_slots = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::TypedSlot {
                slot_id,
                source: AtomSource::Request | AtomSource::Observation,
                ..
            } => Some(*slot_id),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut request = Vec::new();
    let mut state = Vec::new();
    let mut tool = Vec::new();
    let mut ranked_request = Vec::new();
    let mut ranked_state = Vec::new();
    let mut ranked_tool = Vec::new();
    for atom in &frame.atoms {
        let layer = match atom {
            RelationAtom::TypedSlot {
                source: AtomSource::Request,
                ..
            }
            | RelationAtom::RequestPhaseAtom { .. }
            | RelationAtom::ClientCapabilityAtom { .. }
            | RelationAtom::ReconstructedClientCapabilityAtom { .. } => {
                Some(WaveAtomLayer::Request)
            }
            RelationAtom::TypedSlot {
                source: AtomSource::Observation,
                ..
            }
            | RelationAtom::ObservationCallShape { .. }
            | RelationAtom::CollectionShape { .. }
            | RelationAtom::ToolKind { .. }
            | RelationAtom::OutputStatus { .. } => Some(WaveAtomLayer::Tool),
            RelationAtom::ObservationSelector { slot_id, .. }
                if pre_action_slots.contains(slot_id) =>
            {
                Some(WaveAtomLayer::Tool)
            }
            RelationAtom::SlotEquality {
                left_slot,
                right_slot,
            } if pre_action_slots.contains(left_slot) && pre_action_slots.contains(right_slot) => {
                Some(WaveAtomLayer::State)
            }
            RelationAtom::UniqueSlot { slot_id } if pre_action_slots.contains(slot_id) => {
                Some(WaveAtomLayer::State)
            }
            RelationAtom::TypedEquality { .. }
            | RelationAtom::Cardinality { .. }
            | RelationAtom::TemporalEdge { .. }
            | RelationAtom::CompletionState { .. } => Some(WaveAtomLayer::State),
            _ => None,
        };
        let Some(layer) = layer else {
            continue;
        };
        let id = relation_atom_phase_id(atom);
        let ranked = (wave_hidden_source_priority(atom), id);
        match layer {
            WaveAtomLayer::Request => {
                request.push(id);
                ranked_request.push(ranked);
            }
            WaveAtomLayer::State => {
                state.push(id);
                ranked_state.push(ranked);
            }
            WaveAtomLayer::Tool => {
                tool.push(id);
                ranked_tool.push(ranked);
            }
        }
    }
    for basis in [&mut request, &mut state, &mut tool] {
        basis.sort_unstable();
        basis.dedup();
        basis.truncate(BASIS_LIMIT);
    }
    for basis in [&mut ranked_request, &mut ranked_state, &mut ranked_tool] {
        basis.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        basis.dedup_by_key(|entry| entry.1);
        basis.truncate(BASIS_LIMIT);
    }

    // Keep the original hash-selected atoms so already compiled packages keep
    // exactly the routing vocabulary they were trained with.
    let mut hidden = Vec::with_capacity(LEGACY_HIDDEN_LIMIT + BALANCED_HIDDEN_LIMIT);
    extend_wave_pairs(&mut hidden, 1, &request, &state);
    extend_wave_pairs(&mut hidden, 2, &state, &tool);
    extend_wave_pairs(&mut hidden, 3, &request, &tool);
    for request_id in &request {
        for state_id in &state {
            for tool_id in &tool {
                hidden.push(wave_hidden_atom_id(4, &[*request_id, *state_id, *tool_id]));
            }
        }
    }
    hidden.sort_unstable();
    hidden.dedup();
    hidden.truncate(LEGACY_HIDDEN_LIMIT);

    let balanced = balanced_wave_hidden_atom_ids(
        &ranked_request,
        &ranked_state,
        &ranked_tool,
        BALANCED_HIDDEN_LIMIT,
    );
    hidden.extend(balanced);
    hidden.sort_unstable();
    hidden.dedup();
    hidden
}

fn balanced_wave_hidden_atom_ids(
    request: &[(u16, u64)],
    state: &[(u16, u64)],
    tool: &[(u16, u64)],
    limit: usize,
) -> Vec<u64> {
    let mut groups = [
        ranked_wave_pairs(1, request, state),
        ranked_wave_pairs(2, state, tool),
        ranked_wave_pairs(3, request, tool),
        ranked_wave_triples(request, state, tool),
    ];
    for group in &mut groups {
        group.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        group.dedup_by_key(|entry| entry.1);
    }
    let mut output = Vec::with_capacity(limit);
    let mut seen = BTreeSet::new();
    for rank in 0..groups.iter().map(Vec::len).max().unwrap_or(0) {
        for group in &groups {
            let Some((_, atom_id)) = group.get(rank) else {
                continue;
            };
            if seen.insert(*atom_id) {
                output.push(*atom_id);
                if output.len() == limit {
                    return output;
                }
            }
        }
    }
    output
}

fn ranked_wave_pairs(kind: u64, left: &[(u16, u64)], right: &[(u16, u64)]) -> Vec<(u32, u64)> {
    left.iter()
        .flat_map(|(left_priority, left_id)| {
            right.iter().map(move |(right_priority, right_id)| {
                (
                    u32::from(*left_priority).saturating_add(u32::from(*right_priority)),
                    wave_hidden_atom_id(kind, &[*left_id, *right_id]),
                )
            })
        })
        .collect()
}

fn ranked_wave_triples(
    request: &[(u16, u64)],
    state: &[(u16, u64)],
    tool: &[(u16, u64)],
) -> Vec<(u32, u64)> {
    let mut output = Vec::new();
    for (request_priority, request_id) in request {
        for (state_priority, state_id) in state {
            for (tool_priority, tool_id) in tool {
                output.push((
                    u32::from(*request_priority)
                        .saturating_add(u32::from(*state_priority))
                        .saturating_add(u32::from(*tool_priority)),
                    wave_hidden_atom_id(4, &[*request_id, *state_id, *tool_id]),
                ));
            }
        }
    }
    output
}

fn wave_hidden_source_priority(atom: &RelationAtom) -> u16 {
    match atom {
        RelationAtom::CompletionState { .. } | RelationAtom::OutputStatus { .. } => 1_000,
        RelationAtom::ObservationSelector { .. } | RelationAtom::Cardinality { .. } => 900,
        RelationAtom::TypedSlot { .. }
        | RelationAtom::SlotEquality { .. }
        | RelationAtom::UniqueSlot { .. } => 800,
        RelationAtom::ToolKind { .. }
        | RelationAtom::ObservationCallShape { .. }
        | RelationAtom::TypedEquality { .. }
        | RelationAtom::TemporalEdge { .. } => 700,
        RelationAtom::CollectionShape { .. } | RelationAtom::RequestPhaseAtom { .. } => 600,
        RelationAtom::ClientCapabilityAtom { .. }
        | RelationAtom::ReconstructedClientCapabilityAtom { .. } => 500,
        _ => 0,
    }
}

fn extend_wave_pairs(out: &mut Vec<u64>, kind: u64, left: &[u64], right: &[u64]) {
    for left_id in left {
        for right_id in right {
            out.push(wave_hidden_atom_id(kind, &[*left_id, *right_id]));
        }
    }
}

fn wave_hidden_atom_id(kind: u64, parts: &[u64]) -> u64 {
    let mut hash = 0x6e61_6e64_6f77_6176_u64;
    for byte in kind
        .to_le_bytes()
        .into_iter()
        .chain(parts.iter().flat_map(|part| part.to_le_bytes()))
    {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
    }
    hash
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

fn insert_top_response_candidate<'a>(
    ranked: &mut [Option<(i64, &'a ResponsePackage)>; 8],
    candidate: (i64, &'a ResponsePackage),
) {
    let position = ranked.iter().position(|current| {
        current.is_none_or(|(margin, package)| {
            candidate.0 > margin
                || (candidate.0 == margin && candidate.1.package_id < package.package_id)
        })
    });
    let Some(position) = position else {
        return;
    };
    for index in (position + 1..ranked.len()).rev() {
        ranked[index] = ranked[index - 1];
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
    let Ok(scalar) = immediate_selected_scalar(provider_payload, selector) else {
        return Vec::new();
    };
    let completion = if arguments.iter().any(|argument| {
        matches!(
            argument,
            ResponseArgument::Role {
                role: SemanticRole::ContinuationHandle,
                ..
            }
        )
    }) {
        "pending"
    } else {
        "completed"
    };
    let mut atoms = vec![
        stable_atom_id("relation:tool_kind"),
        stable_atom_id(&format!("completion:{completion}")),
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
    if let Some(tool_kind) = immediate_observation_tool_kind(provider_payload) {
        atoms.push(stable_atom_id(&format!("tool_kind:{tool_kind}")));
    }
    atoms
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
    let input = provider_payload.get("input")?.as_array()?;
    let (output_index, call_id) = input.iter().enumerate().rev().find_map(|(index, item)| {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        ) {
            return None;
        }
        item.get("call_id")
            .and_then(Value::as_str)
            .map(|call_id| (index, call_id))
    })?;
    input[..output_index].iter().rev().find_map(|item| {
        let item_type = item.get("type").and_then(Value::as_str)?;
        if !matches!(item_type, "function_call" | "custom_tool_call")
            || item.get("call_id").and_then(Value::as_str) != Some(call_id)
        {
            return None;
        }
        item.get("name").and_then(Value::as_str)?;
        Some(item_type.to_owned())
    })
}

fn immediate_observation_tool_kind(provider_payload: &Value) -> Option<String> {
    let input = provider_payload.get("input")?.as_array()?;
    let (output_index, call_id) = input.iter().enumerate().rev().find_map(|(index, item)| {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        ) {
            return None;
        }
        item.get("call_id")
            .and_then(Value::as_str)
            .map(|call_id| (index, call_id))
    })?;
    input[..output_index].iter().rev().find_map(|item| {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call" | "custom_tool_call")
        ) || item.get("call_id").and_then(Value::as_str) != Some(call_id)
        {
            return None;
        }
        item.get("name").and_then(Value::as_str).map(str::to_owned)
    })
}

fn response_phase_atom_ids_for_custom_tool_call_payload(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
) -> Vec<u64> {
    let Ok(scalar) = immediate_selected_scalar(provider_payload, selector) else {
        return Vec::new();
    };
    let mut atoms = vec![
        stable_atom_id("relation:tool_kind"),
        stable_atom_id("completion:pending"),
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
    if let Some(tool_kind) = immediate_observation_tool_kind(provider_payload) {
        atoms.push(stable_atom_id(&format!("tool_kind:{tool_kind}")));
    }
    atoms
}

pub(crate) fn response_program_grounded_routing_atom_ids(
    program: &ResponseProgram,
    provider_payload: &Value,
) -> Vec<u64> {
    let mut atoms = match &program.operation {
        ResponseOperation::FunctionCallFromRoles {
            selector,
            arguments,
            ..
        } => response_phase_atom_ids_for_grounded_function_call_payload(
            provider_payload,
            selector,
            arguments,
        ),
        ResponseOperation::CustomToolCallFromRoles { selector, .. } => {
            response_phase_atom_ids_for_custom_tool_call_payload(provider_payload, selector)
        }
        _ => return Vec::new(),
    };
    if atoms.is_empty() {
        return atoms;
    }
    atoms.extend(response_pre_action_context_atom_ids(provider_payload));
    atoms.sort_unstable();
    atoms.dedup();
    atoms
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
    let canonical = canonical_response_value_selector(selector);
    stable_atom_id_parts(&["selector:", &canonical])
}

fn stable_atom_id_parts(parts: &[&str]) -> u64 {
    let mut value = 0xcbf2_9ce4_8422_2325_u64;
    for byte in parts.iter().flat_map(|part| part.bytes()) {
        value = (value ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
    }
    value
}

pub(crate) fn response_pre_action_context_atom_ids(provider_payload: &Value) -> Vec<u64> {
    let mut atoms = response_pre_action_context_counts(provider_payload)
        .into_iter()
        .filter(|(role, _)| role != "active_pending_handle_count_band")
        .map(|(role, count)| stable_atom_id(&format!("cardinality:{role}:{count}")))
        .collect::<Vec<_>>();
    atoms.extend(provider_tool_capability_atom_ids(provider_payload));
    atoms.extend(response_pre_action_tool_atom_ids(provider_payload));
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

fn response_pre_action_tool_atom_ids(provider_payload: &Value) -> Vec<u64> {
    let Some(input) = provider_payload.get("input").and_then(Value::as_array) else {
        return Vec::new();
    };
    let start = input
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    input[start..]
        .iter()
        .filter_map(|item| {
            let item_type = item.get("type").and_then(Value::as_str)?;
            matches!(item_type, "function_call" | "custom_tool_call")
                .then(|| item.get("name").and_then(Value::as_str))
                .flatten()
                .map(|name| stable_atom_id(&format!("tool_kind:{item_type}:{name}")))
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn response_pre_action_context_counts(provider_payload: &Value) -> BTreeMap<String, u32> {
    let Some(input) = provider_payload.get("input").and_then(Value::as_array) else {
        return BTreeMap::new();
    };
    let start = input
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    let mut calls = 0_usize;
    let mut outputs = 0_usize;
    let mut pending_outputs = 0_usize;
    let mut messages = 0_usize;
    let mut call_shapes = BTreeSet::new();
    let mut active_pending_handles = BTreeSet::new();
    let mut wait_calls = BTreeMap::<String, String>::new();
    for item in &input[start..] {
        let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");
        match item_type {
            "custom_tool_call" | "function_call" => {
                calls = calls.saturating_add(1);
                let name = item
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unnamed");
                call_shapes.insert(format!("{item_type}:{name}"));
                if item_type == "function_call"
                    && name == "wait"
                    && let (Some(call_id), Some(arguments)) = (
                        item.get("call_id").and_then(Value::as_str),
                        item.get("arguments").and_then(Value::as_str),
                    )
                    && let Some(cell_id) =
                        serde_json::from_str::<Value>(arguments)
                            .ok()
                            .and_then(|value| {
                                value
                                    .get("cell_id")
                                    .and_then(Value::as_str)
                                    .map(str::to_owned)
                            })
                {
                    wait_calls.insert(call_id.to_owned(), cell_id);
                }
            }
            "custom_tool_call_output" | "function_call_output" => {
                outputs = outputs.saturating_add(1);
                let output = item.get("output").unwrap_or(&Value::Null);
                if let Some(handle) = item
                    .get("call_id")
                    .and_then(Value::as_str)
                    .and_then(|call_id| wait_calls.remove(call_id))
                {
                    active_pending_handles.remove(&handle);
                }
                if value_contains_pending_cell(output) {
                    pending_outputs = pending_outputs.saturating_add(1);
                    if let Some(handle) = pending_cell_handle(output) {
                        active_pending_handles.insert(handle);
                    }
                }
            }
            "message" if item.get("role").and_then(Value::as_str) == Some("assistant") => {
                messages = messages.saturating_add(1);
            }
            _ => {}
        }
    }
    [
        ("turn_call_count_band", count_band(calls)),
        ("turn_output_count_band", count_band(outputs)),
        ("turn_pending_count_band", count_band(pending_outputs)),
        ("turn_message_count_band", count_band(messages)),
        ("turn_call_shape_count_band", count_band(call_shapes.len())),
        (
            "active_pending_handle_count_band",
            count_band(active_pending_handles.len()),
        ),
    ]
    .into_iter()
    .map(|(role, count)| (role.to_owned(), count as u32))
    .collect()
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

fn value_contains_pending_cell(value: &Value) -> bool {
    match value {
        Value::String(text) => text.starts_with("Script running with cell ID "),
        Value::Array(items) => items.iter().any(value_contains_pending_cell),
        Value::Object(object) => object.get("text").is_some_and(value_contains_pending_cell),
        _ => false,
    }
}

fn pending_cell_handle(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => text
            .strip_prefix("Script running with cell ID ")
            .and_then(|rest| rest.split_whitespace().next())
            .filter(|handle| !handle.is_empty())
            .map(str::to_owned),
        Value::Array(items) => items.iter().find_map(pending_cell_handle),
        Value::Object(object) => object.get("text").and_then(pending_cell_handle),
        _ => None,
    }
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

const fn source_name(source: AtomSource) -> &'static str {
    match source {
        AtomSource::Request => "request",
        AtomSource::Observation => "observation",
        AtomSource::Action => "action",
        AtomSource::Outcome => "outcome",
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
            },
        }
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
