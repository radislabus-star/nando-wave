use std::collections::{BTreeMap, BTreeSet};

use nando_core::wave::{phase_coherence, phase_vector_from_atom_ids};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    AtomSource, AtomValueType, CustomToolResultProjection, GuardCandidate,
    PROGRAM_CANDIDATE_SCHEMA, RelationAtom, RelationFrame, ResponseArgument, ResponseOperation,
    ResponseProgram, ResponseProgramCandidate, ResponseValueSelector, RoleHypothesis, SemanticRole,
    VerifierProgram, ground_roles, relation_frame_routing_atom_ids,
    response_program_required_routing_atom_ids,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SynthesizedResponseOperator {
    pub candidate: ResponseProgramCandidate,
    pub verifier: VerifierProgram,
    pub support_frame_ids: Vec<String>,
    pub rejected_ambiguous_frames: usize,
}

pub fn partition_teacher_training_families(
    frames: &[RelationFrame],
) -> BTreeMap<(u64, String), Vec<RelationFrame>> {
    let mut families = BTreeMap::<(u64, String), Vec<RelationFrame>>::new();
    let mut teacher_keys = BTreeSet::<(u64, String)>::new();
    for frame in frames
        .iter()
        .filter(|frame| frame.verifier_label != Some(false))
    {
        let plan_advance = frame
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::ActionPlanAdvance));
        let hypotheses = ground_roles(frame);
        if !plan_advance && (hypotheses.len() != 1 || hypotheses[0].competing_binding_count != 0) {
            continue;
        }
        let Some(teacher_signature) = teacher_program_signature(frame) else {
            continue;
        };
        let digest = Sha256::digest(teacher_signature.as_bytes());
        let teacher_pool_id = u64::from_be_bytes(digest[..8].try_into().unwrap_or([0; 8]));
        let key = (teacher_pool_id, teacher_signature);
        teacher_keys.insert(key.clone());
        families.entry(key).or_default().push(frame.clone());
    }
    for frame in frames
        .iter()
        .filter(|frame| frame.verifier_label == Some(false))
    {
        for key in &teacher_keys {
            families.entry(key.clone()).or_default().push(frame.clone());
        }
    }
    families
}

#[doc(hidden)]
pub fn grounded_program_family_id(
    frame: &RelationFrame,
    hypothesis: &RoleHypothesis,
) -> Option<u64> {
    let mut selectors = frame.atoms.iter().filter_map(|atom| match atom {
        RelationAtom::ObservationSelector { selector, .. } => Some(selector),
        _ => None,
    });
    let selector = selectors.next()?;
    if selectors.next().is_some() {
        return None;
    }
    let mut completion_states = frame.atoms.iter().filter_map(|atom| match atom {
        RelationAtom::CompletionState { value } => Some(value.as_str()),
        _ => None,
    });
    let observed_completion_state = completion_states.next()?;
    if completion_states.next().is_some() {
        return None;
    }
    let completion_state = if crate::contracts::selector_denotes_continuation_handle(selector) {
        "pending"
    } else {
        observed_completion_state
    };
    let role_shape = hypothesis
        .bindings
        .iter()
        .map(|(role, index)| match frame.atoms.get(*index) {
            Some(RelationAtom::TypedSlot {
                value_type, source, ..
            }) => Some((*role, *value_type, *source)),
            _ => None,
        })
        .collect::<Option<Vec<(SemanticRole, AtomValueType, AtomSource)>>>()?;
    let mut client_capabilities = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ClientCapabilityAtom { atom_id }
            | RelationAtom::ReconstructedClientCapabilityAtom { atom_id } => Some(*atom_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    client_capabilities.sort_unstable();
    client_capabilities.dedup();
    let material = (
        "nando.grounded-program-family.v2",
        role_shape,
        selector,
        completion_state,
        client_capabilities,
    );
    let digest = Sha256::digest(serde_json::to_vec(&material).ok()?);
    Some(u64::from_be_bytes(digest[..8].try_into().unwrap_or([0; 8])))
}

/// Stable teacher action identity used only while learning from completed traces.
/// Runtime routing must continue to use `relation_frame_routing_atom_ids`.
pub fn teacher_program_signature(frame: &RelationFrame) -> Option<String> {
    crate::teacher_join::teacher_program_signature(frame)
}

fn is_execution_budget_argument(name: &str) -> bool {
    crate::teacher_join::is_execution_budget_argument(name)
}

fn is_noop_poll_argument(argument: &ResponseArgument) -> bool {
    matches!(
        argument,
        ResponseArgument::Integer { name, .. } if is_execution_budget_argument(name)
    ) || matches!(
        argument,
        ResponseArgument::String { name, value } if name == "chars" && value.is_empty()
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SynthesisError {
    EmptySupport,
    AmbiguousRoles,
    InconsistentRoleFamily,
    MissingPendingState,
    MissingCompletionState,
    MissingUniqueHandle,
    NoConsistentProgram,
    AmbiguousPrograms,
}

impl SynthesisError {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptySupport => "empty_support",
            Self::AmbiguousRoles => "ambiguous_roles",
            Self::InconsistentRoleFamily => "inconsistent_role_family",
            Self::MissingPendingState => "missing_pending_state",
            Self::MissingCompletionState => "missing_completion_state",
            Self::MissingUniqueHandle => "missing_unique_handle",
            Self::NoConsistentProgram => "no_consistent_program",
            Self::AmbiguousPrograms => "ambiguous_programs",
        }
    }
}

pub fn synthesize_response_operator(
    support: &[RelationFrame],
) -> Result<SynthesizedResponseOperator, SynthesisError> {
    if support.is_empty() {
        return Err(SynthesisError::EmptySupport);
    }
    let positive_support = support
        .iter()
        .filter(|frame| frame.verifier_label != Some(false))
        .cloned()
        .collect::<Vec<_>>();
    let counterexamples = support
        .iter()
        .filter(|frame| frame.verifier_label == Some(false))
        .collect::<Vec<_>>();
    if positive_support.is_empty() {
        return Err(SynthesisError::EmptySupport);
    }
    if positive_support.iter().any(|frame| {
        frame
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::ActionPlanAdvance))
    }) {
        return synthesize_plan_advance(&positive_support, &counterexamples);
    }
    let mut role_hypotheses = Vec::with_capacity(positive_support.len());
    let mut required_atom_indices = BTreeSet::new();
    let mut source_role = None;
    for frame in &positive_support {
        let hypotheses = ground_roles(frame);
        if hypotheses.len() != 1 || hypotheses[0].competing_binding_count != 0 {
            return Err(SynthesisError::AmbiguousRoles);
        }
        let hypothesis = &hypotheses[0];
        let frame_source_role =
            grounded_source_role(hypothesis).ok_or(SynthesisError::InconsistentRoleFamily)?;
        if source_role
            .replace(frame_source_role)
            .is_some_and(|role| role != frame_source_role)
        {
            return Err(SynthesisError::InconsistentRoleFamily);
        }
        let completion_index = frame
            .atoms
            .iter()
            .position(|atom| matches!(atom, RelationAtom::CompletionState { .. }))
            .ok_or(SynthesisError::MissingCompletionState)?;
        required_atom_indices.insert(completion_index);
        let source_index = *hypothesis
            .bindings
            .get(&frame_source_role)
            .ok_or(SynthesisError::InconsistentRoleFamily)?;
        let source_slot = match frame.atoms.get(source_index) {
            Some(RelationAtom::TypedSlot { slot_id, .. }) => *slot_id,
            _ => return Err(SynthesisError::InconsistentRoleFamily),
        };
        if !frame.atoms.iter().any(
            |atom| matches!(atom, RelationAtom::UniqueSlot { slot_id } if *slot_id == source_slot),
        ) {
            return Err(SynthesisError::MissingUniqueHandle);
        }
        required_atom_indices.insert(source_index);
        role_hypotheses.push(hypothesis.clone());
    }
    let family_ids = role_hypotheses
        .iter()
        .map(|hypothesis| hypothesis.frame_family_id)
        .collect::<BTreeSet<_>>();
    if family_ids.is_empty() {
        return Err(SynthesisError::InconsistentRoleFamily);
    }
    let family_digest = Sha256::digest(serde_json::to_vec(&family_ids).unwrap_or_default());
    let family_id = u64::from_be_bytes(family_digest[..8].try_into().unwrap_or([0; 8]));
    let role_hypothesis_id_sha256 = digest_json(&family_id);
    let guard = GuardCandidate {
        required_atom_indices: required_atom_indices.into_iter().collect(),
        forbidden_atom_indices: Vec::new(),
        require_unique_selector: true,
        max_evidence_age_ms: 30_000,
    };
    let program_candidates = rank_program_candidates_by_phase(
        &positive_support,
        enumerate_response_program_candidates(&positive_support),
    );
    let mut exact_checks = 0_usize;
    let mut survivors = Vec::new();
    for (rank, program) in program_candidates.into_iter().enumerate() {
        let mut consistent = true;
        for frame in &positive_support {
            exact_checks = exact_checks.saturating_add(1);
            if !program_is_consistent(&program, frame) {
                consistent = false;
                break;
            }
        }
        if consistent {
            for frame in &counterexamples {
                exact_checks = exact_checks.saturating_add(1);
                if program_is_consistent(&program, frame) {
                    consistent = false;
                    break;
                }
            }
        }
        if consistent {
            survivors.push((rank.saturating_add(1), program));
        }
    }
    if survivors.len() > 1 {
        return Err(SynthesisError::AmbiguousPrograms);
    }
    let (phase_rank, program) = survivors.pop().ok_or(SynthesisError::NoConsistentProgram)?;
    let candidate_material = serde_json::to_vec(&(&program, &guard, family_id)).unwrap_or_default();
    let candidate = ResponseProgramCandidate {
        schema: PROGRAM_CANDIDATE_SCHEMA.to_owned(),
        candidate_id_sha256: digest_bytes(&candidate_material),
        role_hypothesis_id_sha256,
        program,
        guard,
        phase_rank: u32::try_from(phase_rank).unwrap_or(u32::MAX),
        exact_checks: u32::try_from(exact_checks).unwrap_or(u32::MAX),
        description_length_bytes: candidate_material.len(),
    };
    Ok(SynthesizedResponseOperator {
        candidate,
        verifier: synthesize_independent_verifier(&positive_support)?,
        support_frame_ids: positive_support
            .iter()
            .map(|frame| frame.frame_id_sha256.clone())
            .collect(),
        rejected_ambiguous_frames: 0,
    })
}

fn synthesize_plan_advance(
    positive_support: &[RelationFrame],
    counterexamples: &[&RelationFrame],
) -> Result<SynthesizedResponseOperator, SynthesisError> {
    if positive_support.iter().any(|frame| {
        !frame
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::ActionPlanAdvance))
    }) {
        return Err(SynthesisError::InconsistentRoleFamily);
    }
    let observed_function_names = positive_support
        .iter()
        .filter_map(|frame| {
            frame.atoms.iter().find_map(|atom| match atom {
                RelationAtom::ActionFunction { value } => Some(value.clone()),
                _ => None,
            })
        })
        .collect::<Vec<_>>();
    if observed_function_names.len() != positive_support.len() {
        return Err(SynthesisError::InconsistentRoleFamily);
    }
    let function_names = observed_function_names.into_iter().collect::<BTreeSet<_>>();
    if function_names.len() != 1 || function_names.is_empty() {
        return Err(SynthesisError::InconsistentRoleFamily);
    }
    let function_name = function_names
        .first()
        .cloned()
        .ok_or(SynthesisError::InconsistentRoleFamily)?;
    let program = ResponseProgram::advance_plan(&function_name);
    let mut exact_checks = 0_usize;
    for frame in positive_support {
        exact_checks = exact_checks.saturating_add(1);
        if !program_is_consistent(&program, frame) {
            return Err(SynthesisError::NoConsistentProgram);
        }
    }
    for frame in counterexamples {
        exact_checks = exact_checks.saturating_add(1);
        if program_is_consistent(&program, frame) {
            return Err(SynthesisError::NoConsistentProgram);
        }
    }

    let first = positive_support
        .first()
        .ok_or(SynthesisError::EmptySupport)?;
    let required_atom_indices = first
        .atoms
        .iter()
        .enumerate()
        .filter_map(|(index, atom)| {
            (matches!(atom, RelationAtom::PlanState { .. })
                || matches!(atom, RelationAtom::OutputStatus { value } if value == "success"))
            .then_some(index)
        })
        .collect::<Vec<_>>();
    if required_atom_indices.len() != 2 {
        return Err(SynthesisError::NoConsistentProgram);
    }
    let guard = GuardCandidate {
        required_atom_indices,
        forbidden_atom_indices: Vec::new(),
        require_unique_selector: false,
        max_evidence_age_ms: 30_000,
    };
    let family_material = ("nando.plan-advance-family.v1", &function_name);
    let family_digest = Sha256::digest(serde_json::to_vec(&family_material).unwrap_or_default());
    let family_id = u64::from_be_bytes(family_digest[..8].try_into().unwrap_or([0; 8]));
    let role_hypothesis_id_sha256 = digest_json(&family_material);
    let candidate_material = serde_json::to_vec(&(&program, &guard, family_id)).unwrap_or_default();
    let candidate = ResponseProgramCandidate {
        schema: PROGRAM_CANDIDATE_SCHEMA.to_owned(),
        candidate_id_sha256: digest_bytes(&candidate_material),
        role_hypothesis_id_sha256,
        program: program.clone(),
        guard,
        phase_rank: 1,
        exact_checks: u32::try_from(exact_checks).unwrap_or(u32::MAX),
        description_length_bytes: candidate_material.len(),
    };
    Ok(SynthesizedResponseOperator {
        candidate,
        verifier: compile_independent_verifier(&program)?,
        support_frame_ids: positive_support
            .iter()
            .map(|frame| frame.frame_id_sha256.clone())
            .collect(),
        rejected_ambiguous_frames: 0,
    })
}

fn rank_program_candidates_by_phase(
    support: &[RelationFrame],
    mut candidates: Vec<ResponseProgram>,
) -> Vec<ResponseProgram> {
    let queries = support
        .iter()
        .map(|frame| phase_vector_from_atom_ids(relation_frame_routing_atom_ids(frame), 16))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        let score = |program: &ResponseProgram| {
            let center =
                phase_vector_from_atom_ids(response_program_required_routing_atom_ids(program), 16);
            let coherence = queries
                .iter()
                .map(|query| phase_coherence(query, &center))
                .sum::<f64>();
            let bytes = serde_json::to_vec(program).unwrap_or_default();
            (coherence, bytes.len(), bytes)
        };
        let left_score = score(left);
        let right_score = score(right);
        right_score
            .0
            .total_cmp(&left_score.0)
            .then_with(|| left_score.1.cmp(&right_score.1))
            .then_with(|| left_score.2.cmp(&right_score.2))
    });
    candidates
}

fn synthesize_independent_verifier(
    support: &[RelationFrame],
) -> Result<VerifierProgram, SynthesisError> {
    if support.iter().any(|frame| {
        frame
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::ActionStatusProjection { .. }))
    }) {
        let program = enumerate_status_projection_candidates(support)
            .into_iter()
            .find(|program| {
                support
                    .iter()
                    .all(|frame| program_is_consistent(program, frame))
            })
            .ok_or(SynthesisError::NoConsistentProgram)?;
        let ResponseOperation::ProjectStatus {
            selector,
            mapping,
            renderer,
            completion_state,
        } = program.operation
        else {
            return Err(SynthesisError::NoConsistentProgram);
        };
        return Ok(VerifierProgram::ProjectStatus {
            selector,
            mapping,
            renderer,
            completion_state,
            require_unique_value: true,
        });
    }
    if support.iter().any(|frame| {
        frame
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::ActionValueProjection { .. }))
    }) {
        let program = enumerate_value_projection_candidates(support)
            .into_iter()
            .find(|program| {
                support
                    .iter()
                    .all(|frame| program_is_consistent(program, frame))
            })
            .ok_or(SynthesisError::NoConsistentProgram)?;
        let ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            renderer,
            completion_state,
        } = program.operation
        else {
            return Err(SynthesisError::NoConsistentProgram);
        };
        return Ok(VerifierProgram::ProjectSelectedValue {
            selector,
            format,
            renderer,
            completion_state,
            require_unique_value: true,
        });
    }
    if support.iter().any(|frame| {
        frame
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::ActionCustomTool { .. }))
    }) {
        let source_role = support
            .first()
            .and_then(|frame| ground_roles(frame).into_iter().next())
            .and_then(|hypothesis| grounded_source_role(&hypothesis))
            .ok_or(SynthesisError::InconsistentRoleFamily)?;
        let program = enumerate_custom_tool_candidates(support, source_role)
            .into_iter()
            .find(|program| {
                support
                    .iter()
                    .all(|frame| program_is_consistent(program, frame))
            })
            .ok_or(SynthesisError::NoConsistentProgram)?;
        let ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name,
            inner_tool_name,
            selector,
            arguments,
            projection,
        } = program.operation
        else {
            return Err(SynthesisError::NoConsistentProgram);
        };
        return Ok(VerifierProgram::CustomToolCallFromRoles {
            custom_tool_name,
            inner_tool_name,
            selector,
            arguments,
            projection,
            require_pending_state: true,
            require_unique_handle: true,
        });
    }
    let functions = support
        .iter()
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::ActionFunction { value } => Some(value.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let function_name = functions
        .into_iter()
        .next()
        .filter(|_| {
            support.iter().all(|frame| {
                frame
                    .atoms
                    .iter()
                    .filter(|atom| matches!(atom, RelationAtom::ActionFunction { .. }))
                    .count()
                    == 1
            })
        })
        .ok_or(SynthesisError::NoConsistentProgram)?;
    let selector = consistent_observation_selector(support)?;
    let mut role_arguments = BTreeMap::new();
    let mut role_argument_types = BTreeMap::new();
    let mut integer_values: BTreeMap<String, BTreeMap<u64, usize>> = BTreeMap::new();
    let mut string_values: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut boolean_values: BTreeMap<String, BTreeMap<bool, usize>> = BTreeMap::new();
    for frame in support {
        let hypothesis = ground_roles(frame)
            .into_iter()
            .next()
            .ok_or(SynthesisError::AmbiguousRoles)?;
        let source_role =
            grounded_source_role(&hypothesis).ok_or(SynthesisError::InconsistentRoleFamily)?;
        let target_index = *hypothesis
            .bindings
            .get(&SemanticRole::TargetValue)
            .ok_or(SynthesisError::InconsistentRoleFamily)?;
        let target_slot = match frame.atoms.get(target_index) {
            Some(RelationAtom::TypedSlot { slot_id, .. }) => *slot_id,
            _ => return Err(SynthesisError::InconsistentRoleFamily),
        };
        for atom in &frame.atoms {
            match atom {
                RelationAtom::ActionRoleArgument {
                    name,
                    slot_id,
                    value_type,
                } if *slot_id == target_slot => {
                    role_arguments.insert(name.clone(), source_role);
                    if let Some(value_type) = value_type {
                        role_argument_types.insert(name.clone(), *value_type);
                    }
                }
                RelationAtom::ActionIntegerArgument { name, value } => {
                    *integer_values
                        .entry(name.clone())
                        .or_default()
                        .entry(*value)
                        .or_default() += 1;
                }
                RelationAtom::ActionStringArgument { name, value } => {
                    *string_values
                        .entry(name.clone())
                        .or_default()
                        .entry(value.clone())
                        .or_default() += 1;
                }
                RelationAtom::ActionBooleanArgument { name, value } => {
                    *boolean_values
                        .entry(name.clone())
                        .or_default()
                        .entry(*value)
                        .or_default() += 1;
                }
                _ => {}
            }
        }
    }
    let integer_arguments = integer_values
        .into_iter()
        .filter_map(|(name, counts)| {
            counts
                .into_iter()
                .max_by_key(|(value, count)| (*count, std::cmp::Reverse(*value)))
                .map(|(value, _)| (name, value))
        })
        .collect();
    let string_arguments = most_frequent_literals(string_values);
    let boolean_arguments = most_frequent_literals(boolean_values);
    if role_arguments.is_empty() {
        return Err(SynthesisError::NoConsistentProgram);
    }
    let pending = role_arguments
        .values()
        .any(|role| *role == SemanticRole::ContinuationHandle);
    Ok(VerifierProgram::FunctionCallFromRoles {
        function_name,
        selector,
        role_arguments,
        role_argument_types,
        integer_arguments,
        string_arguments,
        boolean_arguments,
        require_pending_state: pending,
        require_unique_handle: pending,
    })
}

#[doc(hidden)]
pub fn compile_independent_verifier(
    program: &ResponseProgram,
) -> Result<VerifierProgram, SynthesisError> {
    match &program.operation {
        ResponseOperation::UniqueConsensus {
            variants,
            adapter_wave,
        } => Ok(VerifierProgram::UniqueConsensus {
            variants: variants
                .iter()
                .map(|variant| {
                    Ok(crate::VerifierConsensusVariant {
                        verifier: compile_independent_verifier(&variant.program)?,
                        allowed_layout_sha256: variant.allowed_layout_sha256.clone(),
                        required_request_atom_ids: variant.required_request_atom_ids.clone(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            adapter_wave: adapter_wave.clone(),
        }),
        ResponseOperation::AdvancePlan { function_name } => Ok(VerifierProgram::AdvancePlan {
            function_name: function_name.clone(),
            require_explicit_tool_success: true,
            require_canonical_plan: true,
        }),
        ResponseOperation::FunctionCallFromRoles {
            function_name,
            selector,
            arguments,
        } => {
            let mut role_arguments = BTreeMap::new();
            let mut role_argument_types = BTreeMap::new();
            let mut integer_arguments = BTreeMap::new();
            let mut string_arguments = BTreeMap::new();
            let mut boolean_arguments = BTreeMap::new();
            for argument in arguments {
                match argument {
                    ResponseArgument::Role {
                        name,
                        role,
                        value_type,
                    } => {
                        role_arguments.insert(name.clone(), *role);
                        if let Some(value_type) = value_type {
                            role_argument_types.insert(name.clone(), *value_type);
                        }
                    }
                    ResponseArgument::Integer { name, value } => {
                        integer_arguments.insert(name.clone(), *value);
                    }
                    ResponseArgument::String { name, value } => {
                        string_arguments.insert(name.clone(), value.clone());
                    }
                    ResponseArgument::Boolean { name, value } => {
                        boolean_arguments.insert(name.clone(), *value);
                    }
                }
            }
            if role_arguments.is_empty() {
                return Err(SynthesisError::NoConsistentProgram);
            }
            let pending = role_arguments
                .values()
                .any(|role| *role == SemanticRole::ContinuationHandle);
            Ok(VerifierProgram::FunctionCallFromRoles {
                function_name: function_name.clone(),
                selector: selector.clone(),
                role_arguments,
                role_argument_types,
                integer_arguments,
                string_arguments,
                boolean_arguments,
                require_pending_state: pending,
                require_unique_handle: pending,
            })
        }
        ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name,
            inner_tool_name,
            selector,
            arguments,
            projection,
        } => Ok(VerifierProgram::CustomToolCallFromRoles {
            custom_tool_name: custom_tool_name.clone(),
            inner_tool_name: inner_tool_name.clone(),
            selector: selector.clone(),
            arguments: arguments.clone(),
            projection: projection.clone(),
            require_pending_state: true,
            require_unique_handle: true,
        }),
        ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            renderer,
            completion_state,
        } => Ok(VerifierProgram::ProjectSelectedValue {
            selector: selector.clone(),
            format: *format,
            renderer: renderer.clone(),
            completion_state: completion_state.clone(),
            require_unique_value: true,
        }),
        ResponseOperation::ProjectStatus {
            selector,
            mapping,
            renderer,
            completion_state,
        } => Ok(VerifierProgram::ProjectStatus {
            selector: selector.clone(),
            mapping: *mapping,
            renderer: renderer.clone(),
            completion_state: completion_state.clone(),
            require_unique_value: true,
        }),
        ResponseOperation::ComposeCollection {
            steps,
            format,
            renderer,
            completion_state,
            max_items,
        } => Ok(VerifierProgram::ComposeCollection {
            steps: steps.clone(),
            format: *format,
            renderer: renderer.clone(),
            completion_state: completion_state.clone(),
            max_items: *max_items,
        }),
        ResponseOperation::CopyAfterPrefix { .. }
        | ResponseOperation::TestResultSummary { .. }
        | ResponseOperation::WaitOnYieldedCell { .. }
        | ResponseOperation::WaitOnAnyYieldedCell { .. }
        | ResponseOperation::WaitOnYieldedSurfaces { .. } => {
            Err(SynthesisError::NoConsistentProgram)
        }
    }
}

#[doc(hidden)]
pub fn enumerate_response_program_candidates(support: &[RelationFrame]) -> Vec<ResponseProgram> {
    let plan_rows = support
        .iter()
        .filter(|frame| {
            frame
                .atoms
                .iter()
                .any(|atom| matches!(atom, RelationAtom::ActionPlanAdvance))
        })
        .count();
    if plan_rows > 0 {
        if plan_rows != support.len() {
            return Vec::new();
        }
        return support
            .iter()
            .flat_map(|frame| frame.atoms.iter())
            .filter_map(|atom| match atom {
                RelationAtom::ActionFunction { value } => Some(value.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(ResponseProgram::advance_plan)
            .collect();
    }
    let Some(source_role) = support
        .first()
        .and_then(|frame| ground_roles(frame).into_iter().next())
        .and_then(|hypothesis| grounded_source_role(&hypothesis))
    else {
        return Vec::new();
    };
    if support.iter().any(|frame| {
        frame
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::ActionStatusProjection { .. }))
    }) {
        return enumerate_status_projection_candidates(support);
    }
    if support.iter().any(|frame| {
        frame
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::ActionValueProjection { .. }))
    }) {
        return enumerate_value_projection_candidates(support);
    }
    if support.iter().any(|frame| {
        frame
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::ActionCustomTool { .. }))
    }) {
        return enumerate_custom_tool_candidates(support, source_role);
    }
    let functions = support
        .iter()
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::ActionFunction { value } => Some(value.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let role_arguments = support
        .iter()
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::ActionRoleArgument {
                name, value_type, ..
            } => Some((name.clone(), *value_type)),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let selectors = candidate_observation_selectors(support);
    let mut integer_value_counts: BTreeMap<String, BTreeMap<u64, usize>> = BTreeMap::new();
    let mut string_value_counts: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut boolean_value_counts: BTreeMap<String, BTreeMap<bool, usize>> = BTreeMap::new();
    for (name, value) in support
        .iter()
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::ActionIntegerArgument { name, value } => Some((name, *value)),
            _ => None,
        })
    {
        *integer_value_counts
            .entry(name.clone())
            .or_default()
            .entry(value)
            .or_default() += 1;
    }
    for atom in support.iter().flat_map(|frame| frame.atoms.iter()) {
        match atom {
            RelationAtom::ActionStringArgument { name, value } => {
                *string_value_counts
                    .entry(name.clone())
                    .or_default()
                    .entry(value.clone())
                    .or_default() += 1;
            }
            RelationAtom::ActionBooleanArgument { name, value } => {
                *boolean_value_counts
                    .entry(name.clone())
                    .or_default()
                    .entry(*value)
                    .or_default() += 1;
            }
            _ => {}
        }
    }
    let integer_arguments = integer_value_counts
        .into_iter()
        .filter_map(|(name, counts)| {
            counts
                .into_iter()
                .max_by_key(|(value, count)| (*count, std::cmp::Reverse(*value)))
                .map(|(value, _)| (name, value))
        })
        .collect::<Vec<_>>();
    let string_arguments = most_frequent_literals(string_value_counts);
    let boolean_arguments = most_frequent_literals(boolean_value_counts);
    let mut candidates = Vec::new();
    for function_name in functions {
        for selector in &selectors {
            for (role_name, value_type) in &role_arguments {
                let mut arguments = vec![ResponseArgument::Role {
                    name: role_name.clone(),
                    role: source_role,
                    value_type: *value_type,
                }];
                arguments.extend(integer_arguments.iter().map(|(name, value)| {
                    ResponseArgument::Integer {
                        name: name.clone(),
                        value: *value,
                    }
                }));
                arguments.extend(string_arguments.iter().map(|(name, value)| {
                    ResponseArgument::String {
                        name: name.clone(),
                        value: value.clone(),
                    }
                }));
                arguments.extend(boolean_arguments.iter().map(|(name, value)| {
                    ResponseArgument::Boolean {
                        name: name.clone(),
                        value: *value,
                    }
                }));
                candidates.push(ResponseProgram::function_call_from_roles(
                    function_name.clone(),
                    selector.clone(),
                    arguments,
                ));
            }
        }
    }
    candidates
}

fn enumerate_status_projection_candidates(support: &[RelationFrame]) -> Vec<ResponseProgram> {
    if support.is_empty() {
        return Vec::new();
    }
    let selectors = candidate_observation_selectors(support);
    let mappings = support
        .iter()
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::ActionStatusProjection { mapping } => Some(*mapping),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let completion_states = support
        .iter()
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::CompletionState { value } => Some(value.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let (Some(mapping), Some(completion_state)) = (
        mappings.into_iter().next().filter(|_| {
            support.iter().all(|frame| {
                frame
                    .atoms
                    .iter()
                    .filter(|atom| matches!(atom, RelationAtom::ActionStatusProjection { .. }))
                    .count()
                    == 1
            })
        }),
        completion_states.into_iter().next().filter(|_| {
            support.iter().all(|frame| {
                frame
                    .atoms
                    .iter()
                    .filter(|atom| matches!(atom, RelationAtom::CompletionState { .. }))
                    .count()
                    == 1
            })
        }),
    ) else {
        return Vec::new();
    };
    selectors
        .into_iter()
        .filter(|selector| selector_value_type(selector) == crate::AtomValueType::Integer)
        .map(|selector| {
            ResponseProgram::project_status(selector, mapping, completion_state.clone())
        })
        .filter(|program| {
            support
                .iter()
                .all(|frame| program_is_consistent(program, frame))
        })
        .collect()
}

fn enumerate_value_projection_candidates(support: &[RelationFrame]) -> Vec<ResponseProgram> {
    if support.is_empty()
        || support.iter().any(|frame| {
            frame
                .atoms
                .iter()
                .filter(|atom| matches!(atom, RelationAtom::ObservationSelector { .. }))
                .count()
                != 1
                || frame
                    .atoms
                    .iter()
                    .filter(|atom| matches!(atom, RelationAtom::ActionValueProjection { .. }))
                    .count()
                    != 1
                || frame
                    .atoms
                    .iter()
                    .filter(|atom| matches!(atom, RelationAtom::CompletionState { .. }))
                    .count()
                    != 1
        })
    {
        return Vec::new();
    }
    let selectors = candidate_observation_selectors(support);
    let projections = support
        .iter()
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::ActionValueProjection { format, renderer } => {
                Some((*format, renderer.clone()))
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let completion_states = support
        .iter()
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::CompletionState { value } => Some(value.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if selectors.is_empty() || projections.len() != 1 || completion_states.len() != 1 {
        return Vec::new();
    }
    let Some((format, renderer)) = projections.into_iter().next() else {
        return Vec::new();
    };
    let Some(completion_state) = completion_states.into_iter().next() else {
        return Vec::new();
    };
    if !matches!(completion_state.as_str(), "pending" | "completed") {
        return Vec::new();
    }
    selectors
        .into_iter()
        .filter(|selector| selector_value_type(selector) != crate::AtomValueType::Collection)
        .map(|selector| {
            ResponseProgram::project_selected_value(selector, format, completion_state.clone())
                .with_value_renderer(renderer.clone())
        })
        .filter(|program| {
            support
                .iter()
                .all(|frame| program_is_consistent(program, frame))
        })
        .collect()
}

fn enumerate_custom_tool_candidates(
    support: &[RelationFrame],
    source_role: SemanticRole,
) -> Vec<ResponseProgram> {
    if support.is_empty() {
        return Vec::new();
    }
    let selectors = candidate_observation_selectors(support);
    let custom_tool_names = support
        .iter()
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::ActionCustomTool { value } => Some(value.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let inner_tool_names = support
        .iter()
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::ActionInnerTool { value } => Some(value.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let projections = support
        .iter()
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::ActionOutputProjection { output_field } => {
                Some(CustomToolResultProjection::OutputField {
                    output_field: output_field.clone(),
                })
            }
            RelationAtom::ActionResultProjection {
                output_field,
                continuation_field,
                continuation_prefix,
            } => Some(CustomToolResultProjection::OutputAndContinuation {
                output_field: output_field.clone(),
                continuation_field: continuation_field.clone(),
                continuation_prefix: continuation_prefix.clone(),
            }),
            RelationAtom::ActionJsonResultProjection => {
                Some(CustomToolResultProjection::JsonStringifyResult)
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if selectors.is_empty()
        || custom_tool_names.len() != 1
        || inner_tool_names.len() != 1
        || projections.len() != 1
    {
        return Vec::new();
    }
    let mut role_names = BTreeSet::new();
    let mut integer_counts: BTreeMap<String, BTreeMap<u64, usize>> = BTreeMap::new();
    let mut string_counts: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut boolean_counts: BTreeMap<String, BTreeMap<bool, usize>> = BTreeMap::new();
    for frame in support {
        let Some(hypothesis) = ground_roles(frame).into_iter().next() else {
            return Vec::new();
        };
        let Some(target_slot) = hypothesis
            .bindings
            .get(&SemanticRole::TargetValue)
            .and_then(|index| frame.atoms.get(*index))
            .and_then(|atom| match atom {
                RelationAtom::TypedSlot { slot_id, .. } => Some(*slot_id),
                _ => None,
            })
        else {
            return Vec::new();
        };
        for atom in &frame.atoms {
            match atom {
                RelationAtom::ActionRoleArgument {
                    name,
                    slot_id,
                    value_type,
                } if *slot_id == target_slot => {
                    role_names.insert((name.clone(), *value_type));
                }
                RelationAtom::ActionIntegerArgument { name, value } => {
                    *integer_counts
                        .entry(name.clone())
                        .or_default()
                        .entry(*value)
                        .or_default() += 1;
                }
                RelationAtom::ActionStringArgument { name, value } => {
                    *string_counts
                        .entry(name.clone())
                        .or_default()
                        .entry(value.clone())
                        .or_default() += 1;
                }
                RelationAtom::ActionBooleanArgument { name, value } => {
                    *boolean_counts
                        .entry(name.clone())
                        .or_default()
                        .entry(*value)
                        .or_default() += 1;
                }
                _ => {}
            }
        }
    }
    if role_names.len() != 1 {
        return Vec::new();
    }
    let (
        Some((role_name, role_value_type)),
        Some(custom_tool_name),
        Some(inner_tool_name),
        Some(projection),
    ) = (
        role_names.into_iter().next(),
        custom_tool_names.into_iter().next(),
        inner_tool_names.into_iter().next(),
        projections.into_iter().next(),
    )
    else {
        return Vec::new();
    };
    let mut arguments = vec![ResponseArgument::Role {
        name: role_name,
        role: source_role,
        value_type: role_value_type,
    }];
    arguments.extend(integer_counts.into_iter().filter_map(|(name, counts)| {
        counts
            .into_iter()
            .max_by_key(|(value, count)| (*count, std::cmp::Reverse(*value)))
            .map(|(value, _)| ResponseArgument::Integer { name, value })
    }));
    arguments.extend(string_counts.into_iter().filter_map(|(name, counts)| {
        counts
            .into_iter()
            .max_by_key(|(value, count)| (*count, std::cmp::Reverse(value.clone())))
            .map(|(value, _)| ResponseArgument::String { name, value })
    }));
    arguments.extend(boolean_counts.into_iter().filter_map(|(name, counts)| {
        counts
            .into_iter()
            .max_by_key(|(value, count)| (*count, std::cmp::Reverse(*value)))
            .map(|(value, _)| ResponseArgument::Boolean { name, value })
    }));
    arguments
        .sort_by(|left, right| response_argument_name(left).cmp(response_argument_name(right)));
    selectors
        .into_iter()
        .map(|selector| {
            ResponseProgram::custom_tool_call_from_roles(
                custom_tool_name.clone(),
                inner_tool_name.clone(),
                selector,
                arguments.clone(),
                projection.clone(),
            )
        })
        .collect()
}

fn response_argument_name(argument: &ResponseArgument) -> &str {
    match argument {
        ResponseArgument::Role { name, .. }
        | ResponseArgument::Integer { name, .. }
        | ResponseArgument::String { name, .. }
        | ResponseArgument::Boolean { name, .. } => name,
    }
}

fn candidate_observation_selectors(
    support: &[RelationFrame],
) -> BTreeSet<crate::ResponseValueSelector> {
    let mut selectors = support
        .iter()
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::ObservationSelector { selector, .. } => Some(selector.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let value_types = selectors
        .iter()
        .map(selector_value_type)
        .filter(|value_type| *value_type != crate::AtomValueType::Collection)
        .collect::<BTreeSet<_>>();
    for value_type in value_types {
        let precise_selector_spans_support = selectors.iter().any(|selector| {
            !matches!(selector, crate::ResponseValueSelector::UniqueScalar { .. })
                && selector_value_type(selector) == value_type
                && support
                    .iter()
                    .all(|frame| selector_matches_frame(frame, selector))
        });
        if precise_selector_spans_support {
            continue;
        }
        let generic = crate::ResponseValueSelector::UniqueScalar { value_type };
        if support
            .iter()
            .all(|frame| selector_matches_frame(frame, &generic))
        {
            selectors.insert(generic);
        }
    }
    selectors
}

fn grounded_continuation_value_type(frame: &RelationFrame) -> Option<crate::AtomValueType> {
    ground_roles(frame).into_iter().find_map(|hypothesis| {
        let atom_index = hypothesis.bindings.get(&SemanticRole::ContinuationHandle)?;
        match frame.atoms.get(*atom_index)? {
            RelationAtom::TypedSlot {
                value_type,
                source: crate::AtomSource::Observation,
                ..
            } => Some(*value_type),
            _ => None,
        }
    })
}

/// Generates a role-level candidate from a typed physical winner. This function
/// grants no authority: its only production caller immediately replays the
/// candidate on the complete receipt-backed parity support before it can enter
/// a semantic cohort. Physical CEGIS never receives the broader selector.
#[doc(hidden)]
pub fn canonicalize_continuation_role_program(
    program: &ResponseProgram,
    support: &[RelationFrame],
) -> Option<ResponseProgram> {
    if support.is_empty() {
        return None;
    }
    let mut canonical = program.clone();
    let selector = match &mut canonical.operation {
        ResponseOperation::FunctionCallFromRoles { selector, .. }
        | ResponseOperation::CustomToolCallFromRoles { selector, .. } => selector,
        _ => return None,
    };
    let value_type = match selector_value_type(selector) {
        crate::AtomValueType::Identifier => crate::AtomValueType::Identifier,
        crate::AtomValueType::String => crate::AtomValueType::String,
        // Legacy extractors typed digit-only execution handles as integers.
        // Handles are identities, not arithmetic values, so the canonical role
        // preserves their text form and lets exact parity decide authority.
        crate::AtomValueType::Integer => crate::AtomValueType::Identifier,
        crate::AtomValueType::Boolean | crate::AtomValueType::Collection => return None,
    };
    *selector = crate::ResponseValueSelector::ContinuationHandle { value_type };
    canonical.validate().ok()?;
    Some(canonical)
}

fn consistent_observation_selector(
    support: &[RelationFrame],
) -> Result<crate::ResponseValueSelector, SynthesisError> {
    candidate_observation_selectors(support)
        .into_iter()
        .find(|selector| {
            support
                .iter()
                .all(|frame| selector_matches_frame(frame, selector))
        })
        .ok_or(SynthesisError::NoConsistentProgram)
}

#[doc(hidden)]
pub fn program_is_consistent(program: &ResponseProgram, frame: &RelationFrame) -> bool {
    match &program.operation {
        ResponseOperation::UniqueConsensus { variants, .. } => variants
            .iter()
            .any(|variant| program_is_consistent(&variant.program, frame)),
        ResponseOperation::AdvancePlan { function_name } => frame.atoms.iter().any(
            |atom| matches!(atom, RelationAtom::ActionFunction { value } if value == function_name),
        ) && frame
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::ActionPlanAdvance)),
        ResponseOperation::ComposeCollection { .. } => false,
        ResponseOperation::FunctionCallFromRoles {
            function_name,
            selector,
            arguments,
        } => {
            let hypotheses = ground_roles(frame);
            let Some(target_index) = hypotheses
                .first()
                .and_then(|hypothesis| hypothesis.bindings.get(&SemanticRole::TargetValue))
            else {
                return false;
            };
            let Some(RelationAtom::TypedSlot { slot_id, .. }) = frame.atoms.get(*target_index)
            else {
                return false;
            };
            let observed_function = frame.atoms.iter().find_map(|atom| match atom {
                RelationAtom::ActionFunction { value } => Some(value.as_str()),
                _ => None,
            });
            let observed_selector = frame.atoms.iter().find_map(|atom| match atom {
                RelationAtom::ObservationSelector {
                    selector: observed, ..
                } => Some(observed),
                _ => None,
            });
            let observed_role_names = frame
                .atoms
                .iter()
                .filter_map(|atom| match atom {
                    RelationAtom::ActionRoleArgument {
                        name,
                        slot_id: argument_slot,
                        ..
                    } if argument_slot == slot_id => Some(name.as_str()),
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            let observed_role_types = frame
                .atoms
                .iter()
                .filter_map(|atom| match atom {
                    RelationAtom::ActionRoleArgument {
                        name,
                        slot_id: argument_slot,
                        value_type,
                    } if argument_slot == slot_id => Some((name.as_str(), *value_type)),
                    _ => None,
                })
                .collect::<BTreeMap<_, _>>();
            let observed_integer_names = frame
                .atoms
                .iter()
                .filter_map(|atom| match atom {
                    RelationAtom::ActionIntegerArgument { name, .. }
                        if !is_execution_budget_argument(name) =>
                    {
                        Some(name.as_str())
                    }
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            let observed_integer_values = frame
                .atoms
                .iter()
                .filter_map(|atom| match atom {
                    RelationAtom::ActionIntegerArgument { name, value }
                        if !is_execution_budget_argument(name) =>
                    {
                        Some((name.as_str(), *value))
                    }
                    _ => None,
                })
                .collect::<BTreeMap<_, _>>();
            let observed_string_values = frame
                .atoms
                .iter()
                .filter_map(|atom| match atom {
                    RelationAtom::ActionStringArgument { name, value } => {
                        Some((name.as_str(), value.as_str()))
                    }
                    _ => None,
                })
                .collect::<BTreeMap<_, _>>();
            let observed_boolean_values = frame
                .atoms
                .iter()
                .filter_map(|atom| match atom {
                    RelationAtom::ActionBooleanArgument { name, value } => {
                        Some((name.as_str(), *value))
                    }
                    _ => None,
                })
                .collect::<BTreeMap<_, _>>();
            let candidate_role_names = arguments
                .iter()
                .filter_map(|argument| match argument {
                    ResponseArgument::Role { name, .. } => Some(name.as_str()),
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            let candidate_role_types = arguments
                .iter()
                .filter_map(|argument| match argument {
                    ResponseArgument::Role {
                        name, value_type, ..
                    } => Some((name.as_str(), *value_type)),
                    _ => None,
                })
                .collect::<BTreeMap<_, _>>();
            let candidate_integer_names = arguments
                .iter()
                .filter_map(|argument| match argument {
                    ResponseArgument::Integer { name, .. }
                        if !is_execution_budget_argument(name) =>
                    {
                        Some(name.as_str())
                    }
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            let candidate_integer_values = arguments
                .iter()
                .filter_map(|argument| match argument {
                    ResponseArgument::Integer { name, value }
                        if !is_execution_budget_argument(name) =>
                    {
                        Some((name.as_str(), *value))
                    }
                    _ => None,
                })
                .collect::<BTreeMap<_, _>>();
            let candidate_string_values = arguments
                .iter()
                .filter_map(|argument| match argument {
                    ResponseArgument::String { name, value } => {
                        Some((name.as_str(), value.as_str()))
                    }
                    _ => None,
                })
                .collect::<BTreeMap<_, _>>();
            let candidate_boolean_values = arguments
                .iter()
                .filter_map(|argument| match argument {
                    ResponseArgument::Boolean { name, value } => Some((name.as_str(), *value)),
                    _ => None,
                })
                .collect::<BTreeMap<_, _>>();
            observed_function == Some(function_name.as_str())
                && observed_selector.is_some()
                && selector_matches_frame(frame, selector)
                && observed_role_names == candidate_role_names
                && observed_role_types == candidate_role_types
                && observed_integer_names == candidate_integer_names
                && observed_integer_values == candidate_integer_values
                && observed_string_values == candidate_string_values
                && observed_boolean_values == candidate_boolean_values
        }
        ResponseOperation::CustomToolCallFromRoles { .. } => ground_roles(frame)
            .into_iter()
            .next()
            .and_then(|hypothesis| grounded_source_role(&hypothesis))
            .is_some_and(|source_role| {
                enumerate_custom_tool_candidates(std::slice::from_ref(frame), source_role)
                    .iter()
                    .any(|candidate| custom_program_shape_matches(candidate, program))
            }),
        ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            renderer,
            completion_state,
        } => value_projection_frame_matches(frame, selector, *format, renderer, completion_state),
        ResponseOperation::ProjectStatus {
            selector,
            mapping,
            completion_state,
            ..
        } => status_projection_frame_matches(frame, selector, *mapping, completion_state),
        ResponseOperation::CopyAfterPrefix { .. }
        | ResponseOperation::TestResultSummary { .. }
        | ResponseOperation::WaitOnYieldedCell { .. }
        | ResponseOperation::WaitOnAnyYieldedCell { .. }
        | ResponseOperation::WaitOnYieldedSurfaces { .. } => false,
    }
}

/// Tests only whether the pre-action representation would let the actor run.
/// It deliberately ignores the completed teacher action, which is exactly
/// what a competing-action negative must exercise.
#[doc(hidden)]
pub fn program_runtime_applicable(program: &ResponseProgram, completed: &RelationFrame) -> bool {
    let runtime = crate::RuntimeFrame::from_completed(completed).as_routing_relation_frame();
    match &program.operation {
        ResponseOperation::UniqueConsensus { variants, .. } => variants
            .iter()
            .any(|variant| program_runtime_applicable(&variant.program, completed)),
        ResponseOperation::AdvancePlan { .. } => runtime
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::PlanState { .. }))
            && runtime.atoms.iter().any(
                |atom| matches!(atom, RelationAtom::OutputStatus { value } if value == "success"),
            ),
        ResponseOperation::FunctionCallFromRoles {
            selector,
            arguments,
            ..
        }
        | ResponseOperation::CustomToolCallFromRoles {
            selector,
            arguments,
            ..
        } => {
            arguments.iter().all(|argument| match argument {
                ResponseArgument::Role { role, .. } => matches!(
                    role,
                    SemanticRole::ContinuationHandle
                        | SemanticRole::SourceValue
                        | SemanticRole::StatusOrResult
                ),
                ResponseArgument::Integer { .. }
                | ResponseArgument::String { .. }
                | ResponseArgument::Boolean { .. } => true,
            }) && unique_selector_matches_frame(&runtime, selector)
        }
        ResponseOperation::ProjectSelectedValue {
            selector,
            completion_state,
            ..
        }
        | ResponseOperation::ProjectStatus {
            selector,
            completion_state,
            ..
        } => {
            unique_selector_matches_frame(&runtime, selector)
                && completion_state_matches_frame(&runtime, completion_state)
        }
        ResponseOperation::ComposeCollection {
            completion_state, ..
        } => {
            let mut selectors = runtime.atoms.iter().filter_map(|atom| match atom {
                RelationAtom::ObservationSelector { slot_id, selector }
                    if selector_value_type(selector) == crate::AtomValueType::Collection =>
                {
                    Some(*slot_id)
                }
                _ => None,
            });
            let Some(slot_id) = selectors.next() else {
                return false;
            };
            selectors.next().is_none()
                && runtime.atoms.iter().any(
                    |atom| matches!(atom, RelationAtom::UniqueSlot { slot_id: unique } if *unique == slot_id),
                )
                && completion_state_matches_frame(&runtime, completion_state)
        }
        ResponseOperation::CopyAfterPrefix { .. }
        | ResponseOperation::TestResultSummary { .. }
        | ResponseOperation::WaitOnYieldedCell { .. }
        | ResponseOperation::WaitOnAnyYieldedCell { .. }
        | ResponseOperation::WaitOnYieldedSurfaces { .. } => false,
    }
}

fn unique_selector_matches_frame(frame: &RelationFrame, selector: &ResponseValueSelector) -> bool {
    let mut observed = frame.atoms.iter().filter_map(|atom| match atom {
        RelationAtom::ObservationSelector {
            slot_id,
            selector: observed,
        } if observed == selector || selector_matches_frame(frame, selector) => Some(*slot_id),
        _ => None,
    });
    let Some(slot_id) = observed.next() else {
        return false;
    };
    observed.next().is_none()
        && frame
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::UniqueSlot { slot_id: unique } if *unique == slot_id))
}

fn completion_state_matches_frame(frame: &RelationFrame, completion_state: &str) -> bool {
    frame.atoms.iter().any(
        |atom| matches!(atom, RelationAtom::CompletionState { value } if value == completion_state),
    )
}

fn most_frequent_literals<K: Ord, V: Ord>(
    values: BTreeMap<K, BTreeMap<V, usize>>,
) -> BTreeMap<K, V> {
    values
        .into_iter()
        .filter_map(|(name, counts)| {
            counts
                .into_iter()
                .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
                .map(|(value, _)| (name, value))
        })
        .collect()
}

fn status_projection_frame_matches(
    frame: &RelationFrame,
    selector: &crate::ResponseValueSelector,
    mapping: crate::ProjectStatusMapping,
    completion_state: &str,
) -> bool {
    if selector_value_type(selector) != crate::AtomValueType::Integer
        || !matches!(completion_state, "pending" | "completed")
        || frame.atoms.iter().any(|atom| {
            matches!(
                atom,
                RelationAtom::ActionFunction { .. }
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
            )
        })
    {
        return false;
    }
    let mut selectors = frame.atoms.iter().filter_map(|atom| match atom {
        RelationAtom::ObservationSelector {
            slot_id,
            selector: observed,
        } => Some((*slot_id, observed)),
        _ => None,
    });
    let Some((slot_id, _observed_selector)) = selectors.next() else {
        return false;
    };
    if selectors.next().is_some()
        || !selector_matches_frame(frame, selector)
        || frame
            .atoms
            .iter()
            .filter(|atom| {
                matches!(atom, RelationAtom::ActionStatusProjection { mapping: observed } if *observed == mapping)
            })
            .count()
            != 1
        || frame
            .atoms
            .iter()
            .filter(|atom| {
                matches!(atom, RelationAtom::CompletionState { value } if value == completion_state)
            })
            .count()
            != 1
        || frame
            .atoms
            .iter()
            .filter(|atom| {
                matches!(atom, RelationAtom::UniqueSlot { slot_id: candidate } if *candidate == slot_id)
            })
            .count()
            != 1
    {
        return false;
    }
    frame
        .atoms
        .iter()
        .filter(|atom| {
            matches!(
                atom,
                RelationAtom::TypedSlot {
                    slot_id: candidate,
                    value_type: crate::AtomValueType::Integer,
                    source: crate::AtomSource::Observation,
                    ..
                } if *candidate == slot_id
            )
        })
        .count()
        == 1
}

const fn selector_value_type(selector: &crate::ResponseValueSelector) -> crate::AtomValueType {
    match selector {
        crate::ResponseValueSelector::ContinuationHandle { value_type }
        | crate::ResponseValueSelector::UniqueScalar { value_type }
        | crate::ResponseValueSelector::UniqueTurnScalar { value_type }
        | crate::ResponseValueSelector::ContentLinePrefix { value_type, .. }
        | crate::ResponseValueSelector::JsonField { value_type, .. }
        | crate::ResponseValueSelector::JsonScalarOrdinal { value_type, .. }
        | crate::ResponseValueSelector::UniqueTurnJsonField { value_type, .. }
        | crate::ResponseValueSelector::UniqueActiveTurnJsonField { value_type, .. }
        | crate::ResponseValueSelector::RequestReferencedJsonField { value_type }
        | crate::ResponseValueSelector::RequestReferencedJsonFieldOrdinal { value_type, .. }
        | crate::ResponseValueSelector::TurnOutputLine { value_type, .. }
        | crate::ResponseValueSelector::TurnOutputScalarOrdinal { value_type, .. }
        | crate::ResponseValueSelector::LatestTurnOutputLine { value_type, .. }
        | crate::ResponseValueSelector::LatestTurnOutputScalarOrdinal { value_type, .. }
        | crate::ResponseValueSelector::LatestTurnOutputScalarFromEnd { value_type, .. } => {
            *value_type
        }
        crate::ResponseValueSelector::CommandOutputBody
        | crate::ResponseValueSelector::RequestLastToken
        | crate::ResponseValueSelector::RequestUniqueLiteral => crate::AtomValueType::String,
    }
}

fn selector_matches_frame(frame: &RelationFrame, candidate: &crate::ResponseValueSelector) -> bool {
    let mut observed = frame.atoms.iter().filter_map(|atom| match atom {
        RelationAtom::ObservationSelector { slot_id, selector } => Some((*slot_id, selector)),
        _ => None,
    });
    let Some((slot_id, observed_selector)) = observed.next() else {
        return false;
    };
    if observed.next().is_some() {
        return false;
    }
    if observed_selector == candidate {
        return true;
    }
    if matches!(
        candidate,
        crate::ResponseValueSelector::ContinuationHandle { .. }
    ) {
        return grounded_continuation_value_type(frame) == Some(selector_value_type(candidate));
    }
    let crate::ResponseValueSelector::UniqueScalar { value_type } = candidate else {
        return false;
    };
    if selector_value_type(observed_selector) != *value_type
        || *value_type == crate::AtomValueType::Collection
        || !frame
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::UniqueSlot { slot_id: unique } if *unique == slot_id))
    {
        return false;
    }
    frame
        .atoms
        .iter()
        .filter(|atom| {
            matches!(
                atom,
                RelationAtom::TypedSlot {
                    value_type: observed_type,
                    source: crate::AtomSource::Observation,
                    ..
                } if observed_type == value_type
            )
        })
        .count()
        == 1
}

fn value_projection_frame_matches(
    frame: &RelationFrame,
    selector: &crate::ResponseValueSelector,
    format: crate::ValueProjectionFormat,
    renderer: &crate::CollectionOutputRenderer,
    completion_state: &str,
) -> bool {
    if !matches!(completion_state, "pending" | "completed")
        || frame
            .atoms
            .iter()
            .filter(|atom| matches!(atom, RelationAtom::ActionValueProjection { .. }))
            .count()
            != 1
        || frame
            .atoms
            .iter()
            .filter(|atom| matches!(atom, RelationAtom::ObservationSelector { .. }))
            .count()
            != 1
        || frame.atoms.iter().any(|atom| {
            matches!(
                atom,
                RelationAtom::ActionFunction { .. }
                    | RelationAtom::ActionCustomTool { .. }
                    | RelationAtom::ActionRoleArgument { .. }
            )
        })
    {
        return false;
    }
    let Some(RelationAtom::CompletionState { value }) = frame
        .atoms
        .iter()
        .find(|atom| matches!(atom, RelationAtom::CompletionState { .. }))
    else {
        return false;
    };
    if value != completion_state
        || !frame.atoms.iter().any(|atom| {
            matches!(atom, RelationAtom::ActionValueProjection { format: observed, renderer: observed_renderer } if *observed == format && observed_renderer == renderer)
        })
        || !selector_matches_frame(frame, selector)
    {
        return false;
    }
    let hypotheses = ground_roles(frame);
    let Some(hypothesis) = hypotheses.first().filter(|_| hypotheses.len() == 1) else {
        return false;
    };
    let Some(source_role) = grounded_source_role(hypothesis) else {
        return false;
    };
    let (Some(source_index), Some(target_index)) = (
        hypothesis.bindings.get(&source_role),
        hypothesis.bindings.get(&SemanticRole::TargetValue),
    ) else {
        return false;
    };
    let (
        Some(RelationAtom::TypedSlot {
            slot_id: source_slot,
            value_type: source_type,
            value_sha256: source_hash,
            ..
        }),
        Some(RelationAtom::TypedSlot {
            slot_id: target_slot,
            value_type: target_type,
            value_sha256: target_hash,
            ..
        }),
    ) = (
        frame.atoms.get(*source_index),
        frame.atoms.get(*target_index),
    )
    else {
        return false;
    };
    source_type == target_type
        && source_hash == target_hash
        && frame.atoms.iter().any(|atom| {
            matches!(atom, RelationAtom::SlotEquality { left_slot, right_slot }
                if left_slot == source_slot && right_slot == target_slot)
        })
        && frame.atoms.iter().any(
            |atom| matches!(atom, RelationAtom::UniqueSlot { slot_id } if slot_id == source_slot),
        )
}

fn custom_program_shape_matches(left: &ResponseProgram, right: &ResponseProgram) -> bool {
    custom_program_shape_mismatch_reasons(left, right).is_empty()
}

fn custom_program_shape_mismatch_reasons(
    left: &ResponseProgram,
    right: &ResponseProgram,
) -> BTreeSet<&'static str> {
    let mut reasons = BTreeSet::new();
    let (
        ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name: left_custom,
            inner_tool_name: left_inner,
            selector: left_selector,
            arguments: left_arguments,
            projection: left_projection,
        },
        ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name: right_custom,
            inner_tool_name: right_inner,
            selector: right_selector,
            arguments: right_arguments,
            projection: right_projection,
        },
    ) = (&left.operation, &right.operation)
    else {
        reasons.insert("operation_kind");
        return reasons;
    };
    let left_arguments = left_arguments
        .iter()
        .filter(|argument| !is_noop_poll_argument(argument))
        .collect::<Vec<_>>();
    let right_arguments = right_arguments
        .iter()
        .filter(|argument| !is_noop_poll_argument(argument))
        .collect::<Vec<_>>();
    if left_custom != right_custom {
        reasons.insert("custom_tool");
    }
    if left_inner != right_inner {
        reasons.insert("inner_tool");
    }
    if !custom_selector_family_matches(left_selector, right_selector) {
        reasons.insert("selector_family");
    }
    if left_projection != right_projection {
        reasons.insert("projection");
    }
    if left_arguments.len() != right_arguments.len() {
        reasons.insert("argument_count");
    }
    for (left, right) in left_arguments.into_iter().zip(right_arguments) {
        match (left, right) {
            (
                ResponseArgument::Role {
                    name: left_name,
                    role: left_role,
                    value_type: left_value_type,
                },
                ResponseArgument::Role {
                    name: right_name,
                    role: right_role,
                    value_type: right_value_type,
                },
            ) => {
                if left_name != right_name {
                    reasons.insert("argument_name");
                }
                if left_role != right_role {
                    reasons.insert("argument_role");
                }
                if left_value_type != right_value_type {
                    reasons.insert("argument_type");
                }
            }
            (
                ResponseArgument::Integer {
                    name: left_name, ..
                },
                ResponseArgument::Integer {
                    name: right_name, ..
                },
            )
            | (
                ResponseArgument::String {
                    name: left_name, ..
                },
                ResponseArgument::String {
                    name: right_name, ..
                },
            )
            | (
                ResponseArgument::Boolean {
                    name: left_name, ..
                },
                ResponseArgument::Boolean {
                    name: right_name, ..
                },
            ) => {
                if left_name != right_name {
                    reasons.insert("argument_name");
                }
            }
            _ => {
                reasons.insert("argument_kind");
            }
        }
    }
    reasons
}

#[doc(hidden)]
pub fn nearest_custom_program_mismatch_reasons(
    programs: &[ResponseProgram],
    frame: &RelationFrame,
) -> Vec<String> {
    let candidates = ground_roles(frame)
        .into_iter()
        .next()
        .and_then(|hypothesis| grounded_source_role(&hypothesis))
        .map_or_else(Vec::new, |source_role| {
            enumerate_custom_tool_candidates(std::slice::from_ref(frame), source_role)
        });
    if candidates.is_empty() {
        return vec!["no_custom_candidate".to_owned()];
    }
    candidates
        .iter()
        .flat_map(|candidate| {
            programs
                .iter()
                .map(move |program| custom_program_shape_mismatch_reasons(candidate, program))
        })
        .min_by(|left, right| {
            left.len()
                .cmp(&right.len())
                .then_with(|| left.iter().cmp(right.iter()))
        })
        .map(|reasons| reasons.into_iter().map(str::to_owned).collect())
        .unwrap_or_else(|| vec!["no_survivor".to_owned()])
}

fn custom_selector_family_matches(
    left: &crate::ResponseValueSelector,
    right: &crate::ResponseValueSelector,
) -> bool {
    if left == right {
        return true;
    }
    matches!(
        (left, right),
        (
            crate::ResponseValueSelector::JsonField {
                field: left_field,
                value_type: left_type,
            }
                | crate::ResponseValueSelector::UniqueActiveTurnJsonField {
                    field: left_field,
                    value_type: left_type,
                },
            crate::ResponseValueSelector::JsonField {
                field: right_field,
                value_type: right_type,
            }
                | crate::ResponseValueSelector::UniqueActiveTurnJsonField {
                    field: right_field,
                    value_type: right_type,
                },
        ) if left_field == right_field && left_type == right_type
    )
}

pub fn verify_operator_structure(
    frame: &RelationFrame,
    operator: &SynthesizedResponseOperator,
) -> bool {
    if let VerifierProgram::ProjectStatus {
        selector,
        mapping,
        renderer,
        completion_state,
        require_unique_value,
    } = &operator.verifier
    {
        let ResponseOperation::ProjectStatus {
            selector: program_selector,
            mapping: program_mapping,
            renderer: program_renderer,
            completion_state: program_completion,
        } = &operator.candidate.program.operation
        else {
            return false;
        };
        return *require_unique_value
            && selector == program_selector
            && mapping == program_mapping
            && renderer == program_renderer
            && completion_state == program_completion
            && status_projection_frame_matches(frame, selector, *mapping, completion_state);
    }
    if let VerifierProgram::ProjectSelectedValue {
        selector,
        format,
        renderer,
        completion_state,
        require_unique_value,
    } = &operator.verifier
    {
        let ResponseOperation::ProjectSelectedValue {
            selector: program_selector,
            format: program_format,
            renderer: program_renderer,
            completion_state: program_completion,
        } = &operator.candidate.program.operation
        else {
            return false;
        };
        return *require_unique_value
            && selector == program_selector
            && format == program_format
            && renderer == program_renderer
            && completion_state == program_completion
            && value_projection_frame_matches(
                frame,
                selector,
                *format,
                renderer,
                completion_state,
            );
    }
    let (require_observation_action_equality, require_pending_state, require_unique_handle) =
        match &operator.verifier {
            VerifierProgram::UniqueConsensus { .. } => return false,
            VerifierProgram::AdvancePlan {
                function_name,
                require_explicit_tool_success,
                require_canonical_plan,
            } => {
                return *require_explicit_tool_success
                    && *require_canonical_plan
                    && matches!(
                        &operator.candidate.program.operation,
                        ResponseOperation::AdvancePlan { function_name: program_name }
                            if program_name == function_name
                    )
                    && frame
                        .atoms
                        .iter()
                        .any(|atom| matches!(atom, RelationAtom::ActionPlanAdvance));
            }
            VerifierProgram::FunctionCallFromRoles {
                require_pending_state,
                require_unique_handle,
                ..
            } => (true, *require_pending_state, *require_unique_handle),
            VerifierProgram::CustomToolCallFromRoles {
                require_pending_state,
                require_unique_handle,
                ..
            } => (true, *require_pending_state, *require_unique_handle),
            VerifierProgram::ProjectSelectedValue { .. } => return false,
            VerifierProgram::ContinueHandle {
                require_observation_action_equality,
                require_pending_state,
                require_unique_handle,
            } => (
                *require_observation_action_equality,
                *require_pending_state,
                *require_unique_handle,
            ),
            VerifierProgram::ProjectStatus { .. } => return false,
            VerifierProgram::ComposeCollection { .. } => return false,
        };
    let hypotheses = ground_roles(frame);
    if hypotheses.len() != 1 || hypotheses[0].competing_binding_count != 0 {
        return false;
    }
    let Some(source_role) = grounded_source_role(&hypotheses[0]) else {
        return false;
    };
    let source_index = match hypotheses[0].bindings.get(&source_role) {
        Some(index) => *index,
        None => return false,
    };
    let source_slot = match frame.atoms.get(source_index) {
        Some(RelationAtom::TypedSlot { slot_id, .. }) => *slot_id,
        _ => return false,
    };
    (!require_pending_state
        || frame.atoms.iter().any(|atom| {
            matches!(atom, RelationAtom::CompletionState { value } if value == "pending")
        }))
        && (!require_unique_handle
            || frame.atoms.iter().any(
                |atom| matches!(atom, RelationAtom::UniqueSlot { slot_id } if *slot_id == source_slot),
            ))
        && (!require_observation_action_equality
            || frame.atoms.iter().any(|atom| {
                matches!(atom, RelationAtom::SlotEquality { left_slot, .. } if *left_slot == source_slot)
            }))
}

fn grounded_source_role(hypothesis: &crate::RoleHypothesis) -> Option<SemanticRole> {
    [
        SemanticRole::ContinuationHandle,
        SemanticRole::SourceValue,
        SemanticRole::StatusOrResult,
    ]
    .into_iter()
    .find(|role| hypothesis.bindings.contains_key(role))
}

fn digest_json<T: Serialize>(value: &T) -> String {
    digest_bytes(&serde_json::to_vec(value).unwrap_or_default())
}

fn digest_bytes(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

#[cfg(test)]
#[path = "synthesis_tests.rs"]
mod tests;
