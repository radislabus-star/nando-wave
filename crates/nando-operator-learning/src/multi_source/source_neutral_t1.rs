use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    AtomSource, AtomValueType, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
    MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1, MultiSourceTypeClassV1,
    PreActionMultiSourceTopologyV1, RelationAtom, RelationFrame, ResponseArgument,
    ResponseOperation, ResponseProgram, ResponseRenderSegment, ResponseValueSelector, SemanticRole,
    canonical_json_sha256, response_program_version_root_sha256,
};

use super::BlindThenRevealJoinedTransitionV1;

const T1_MAX_ROLE_BINDING_HYPOTHESES: usize = 64;
const T1_MAX_PROGRAM_HYPOTHESES: usize = 4_096;

type LocalRoleId = u16;
type StructuralSelector = ResponseValueSelector;
type RoleBindingOptions = Vec<(LocalRoleId, StructuralSelector)>;
type RoleHypothesisMap = BTreeMap<ResponseValueSelector, RoleBindingOptions>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SelectedObservationEvidenceV1 {
    Missing,
    Ambiguous,
    Present,
}

pub(super) fn selected_observation_evidence_v1(
    frame: &RelationFrame,
) -> SelectedObservationEvidenceV1 {
    match admissible_observations(frame) {
        Ok(_) => SelectedObservationEvidenceV1::Present,
        Err("selected_observation_missing") => SelectedObservationEvidenceV1::Missing,
        Err(_) => SelectedObservationEvidenceV1::Ambiguous,
    }
}

pub(super) fn enumerate_source_neutral_t1_candidates(
    joined: &BlindThenRevealJoinedTransitionV1,
    frame: &RelationFrame,
) -> Result<BTreeMap<String, ResponseProgram>, &'static str> {
    let replay_frame = deduplicated_relation_frame(frame);
    let observations = admissible_observations(&replay_frame)?;
    let role_hypotheses = source_neutral_role_hypotheses(joined, &observations)?;
    let physical = crate::synthesis::enumerate_response_program_candidates(std::slice::from_ref(
        &replay_frame,
    ));
    if physical.is_empty() {
        return Err("physical_t1_program_missing");
    }
    let mut structurally_valid = Vec::new();
    for program in physical {
        structurally_valid.extend(source_neutralize_t1_programs(
            &program,
            &observations,
            &role_hypotheses,
        )?);
        if structurally_valid.len() > T1_MAX_PROGRAM_HYPOTHESES {
            return Err("source_neutral_program_budget_exhausted");
        }
    }
    if structurally_valid.is_empty() {
        return Err(source_neutral_t1_blocker(joined, frame));
    }
    let candidates = structurally_valid
        .into_iter()
        .filter(|program| t1_program_is_consistent(program, joined, &replay_frame))
        .filter_map(|program| {
            response_program_version_root_sha256(&program)
                .ok()
                .map(|root| (root, program))
        })
        .collect::<BTreeMap<_, _>>();
    if candidates.is_empty() {
        return Err("source_neutral_self_replay_failed");
    }
    Ok(candidates)
}

pub fn t1_program_is_consistent(
    program: &ResponseProgram,
    joined: &BlindThenRevealJoinedTransitionV1,
    frame: &RelationFrame,
) -> bool {
    t1_program_consistency_blocker(program, joined, frame).is_none()
}

pub(super) fn t1_program_consistency_blocker(
    program: &ResponseProgram,
    joined: &BlindThenRevealJoinedTransitionV1,
    frame: &RelationFrame,
) -> Option<&'static str> {
    let replay_frame = deduplicated_relation_frame(frame);
    let Ok(observations) = admissible_observations(&replay_frame) else {
        return Some("selected_observation_missing_or_ambiguous");
    };
    let Some(structural_selectors) = program_role_selectors(program) else {
        return Some("primary_selector_missing");
    };
    let mut frontier = vec![(program.clone(), BTreeSet::<ResponseValueSelector>::new())];
    for structural_selector in structural_selectors {
        let Some(witness) = witness_for_selector(structural_selector, &joined.topology) else {
            return Some("structural_role_missing_or_ambiguous");
        };
        let Some(role) = role_for_witness(&joined.topology, witness) else {
            return Some("selected_structural_role_missing");
        };
        let physical_options = observations
            .iter()
            .filter(|observation| {
                observation.value_root == witness.value_sha256
                    && role_type_matches(role.type_class, observation.value_type)
            })
            .collect::<Vec<_>>();
        if physical_options.is_empty() {
            return Some("structural_role_value_mismatch");
        }
        let mut next = Vec::new();
        for (bound, used) in frontier {
            for observation in &physical_options {
                if used.contains(observation.selector) {
                    continue;
                }
                let mut candidate = bound.clone();
                if replace_program_selector(
                    &mut candidate,
                    structural_selector,
                    observation.selector,
                )
                .is_none()
                {
                    continue;
                }
                let mut next_used = used.clone();
                next_used.insert(observation.selector.clone());
                next.push((candidate, next_used));
                if next.len() > T1_MAX_ROLE_BINDING_HYPOTHESES {
                    return Some("role_binding_budget_exhausted");
                }
            }
        }
        if next.is_empty() {
            return Some("selector_rewrite_failed");
        }
        frontier = next;
    }
    if frontier
        .iter()
        .any(|(bound, _)| crate::synthesis::program_is_consistent(bound, &replay_frame))
    {
        None
    } else {
        Some("physical_transition_mismatch")
    }
}

fn deduplicated_relation_frame(frame: &RelationFrame) -> RelationFrame {
    let mut deduplicated = frame.clone();
    deduplicated.atoms.sort();
    deduplicated.atoms.dedup();
    deduplicated
}

fn source_neutralize_t1_programs(
    program: &ResponseProgram,
    observations: &[SelectedObservation<'_>],
    role_hypotheses: &RoleHypothesisMap,
) -> Result<Vec<ResponseProgram>, &'static str> {
    let Some(physical_selectors) = program_role_selectors(program) else {
        return Ok(Vec::new());
    };
    let mut frontier = vec![(program.clone(), BTreeSet::<u16>::new())];
    for physical_selector in physical_selectors {
        let Some(observation) = observations
            .iter()
            .find(|observation| observation.selector == physical_selector)
        else {
            return Ok(Vec::new());
        };
        let role_options = role_hypotheses
            .get(observation.selector)
            .cloned()
            .unwrap_or_default();
        if role_options.len() > T1_MAX_ROLE_BINDING_HYPOTHESES {
            return Err("role_binding_budget_exhausted");
        }
        let mut next = Vec::new();
        for (candidate, used_roles) in frontier {
            for (role_id, structural_selector) in &role_options {
                if used_roles.contains(role_id) {
                    continue;
                }
                let mut bound = candidate.clone();
                if replace_program_selector(&mut bound, physical_selector, structural_selector)
                    .is_none()
                {
                    continue;
                }
                let mut next_used_roles = used_roles.clone();
                next_used_roles.insert(*role_id);
                if bound.validate().is_ok() {
                    next.push((bound, next_used_roles));
                }
                if next.len() > T1_MAX_PROGRAM_HYPOTHESES {
                    return Err("source_neutral_program_budget_exhausted");
                }
            }
        }
        if next.is_empty() {
            return Ok(Vec::new());
        }
        frontier = next;
    }
    Ok(frontier.into_iter().map(|(program, _)| program).collect())
}

fn source_neutral_role_hypotheses(
    joined: &BlindThenRevealJoinedTransitionV1,
    observations: &[SelectedObservation<'_>],
) -> Result<RoleHypothesisMap, &'static str> {
    let mut hypotheses = BTreeMap::new();
    let mut total = 0usize;
    for observation in observations {
        let options = role_binding_options(joined, observation);
        total = total.saturating_add(options.len());
        if total > T1_MAX_ROLE_BINDING_HYPOTHESES {
            return Err("role_binding_budget_exhausted");
        }
        hypotheses.insert(observation.selector.clone(), options);
    }
    Ok(hypotheses)
}

fn role_binding_options(
    joined: &BlindThenRevealJoinedTransitionV1,
    observation: &SelectedObservation<'_>,
) -> RoleBindingOptions {
    joined
        .topology
        .role_witnesses
        .iter()
        .filter(|witness| witness.value_sha256 == observation.value_root)
        .flat_map(|witness| {
            let Some(role) = role_for_witness(&joined.topology, witness) else {
                return Vec::new();
            };
            if !role_type_matches(role.type_class, observation.value_type) {
                return Vec::new();
            }
            structural_selectors_for_role(joined, role, witness, observation.value_type)
                .into_iter()
                .map(|selector| (role.local_role_id, selector))
                .collect::<Vec<_>>()
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn structural_selectors_for_role(
    joined: &BlindThenRevealJoinedTransitionV1,
    role: &MultiSourceRoleNodeV1,
    witness: &MultiSourceRoleWitnessV1,
    value_type: AtomValueType,
) -> Vec<ResponseValueSelector> {
    if role_has_relation(
        &joined.topology,
        role.local_role_id,
        MultiSourceRelationKindV1::ContinuationHandle,
    ) {
        vec![ResponseValueSelector::ContinuationHandle { value_type }]
    } else if witness.request_reference_ordinal.is_some()
        || !witness.request_reference_ordinal_candidates.is_empty()
    {
        witness
            .request_reference_ordinal
            .into_iter()
            .chain(witness.request_reference_ordinal_candidates.iter().copied())
            .map(
                |ordinal| ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                    ordinal,
                    value_type,
                },
            )
            .collect()
    } else if joined.topology.roles.len() == 1 {
        vec![ResponseValueSelector::UniqueScalar { value_type }]
    } else if role.temporal_class == MultiSourceTemporalClassV1::Latest {
        vec![ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal: role.value_ordinal,
            value_type,
        }]
    } else {
        Vec::new()
    }
}

fn source_neutral_t1_blocker(
    joined: &BlindThenRevealJoinedTransitionV1,
    frame: &RelationFrame,
) -> &'static str {
    let observations = match admissible_observations(frame) {
        Ok(observations) => observations,
        Err(blocker) => return blocker,
    };
    if observations
        .iter()
        .all(|observation| role_binding_options(joined, observation).is_empty())
    {
        return "selected_role_witness_missing";
    }
    if observations.iter().any(|observation| {
        joined.topology.role_witnesses.iter().any(|witness| {
            witness.value_sha256 == observation.value_root
                && role_for_witness(&joined.topology, witness)
                    .is_some_and(|role| role_type_matches(role.type_class, observation.value_type))
        }) && role_binding_options(joined, observation).is_empty()
    }) {
        return "selected_structural_selector_missing";
    }
    "physical_program_selector_rewrite_failed"
}

fn replace_program_selector(
    program: &mut ResponseProgram,
    observed: &ResponseValueSelector,
    structural: &ResponseValueSelector,
) -> Option<()> {
    let mut replaced = false;
    match &mut program.operation {
        ResponseOperation::FunctionCallFromRoles { selector, .. }
        | ResponseOperation::CustomToolCallFromRoles { selector, .. } => {
            if selector == observed {
                *selector = structural.clone();
                replaced = true;
            }
        }
        ResponseOperation::ProjectSelectedValue {
            selector, renderer, ..
        } => {
            if selector == observed {
                *selector = structural.clone();
                replaced = true;
            }
            if let nando_operator_kernel::CollectionOutputRenderer::RenderSequence { segments } =
                renderer
            {
                for segment in segments {
                    if let ResponseRenderSegment::Selected { selector, .. } = segment
                        && selector == observed
                    {
                        *selector = structural.clone();
                        replaced = true;
                    }
                }
            }
        }
        _ => return None,
    }
    if !replaced {
        return None;
    }
    if matches!(structural, ResponseValueSelector::ContinuationHandle { .. }) {
        normalize_continuation_argument_roles(program);
    }
    Some(())
}

fn normalize_continuation_argument_roles(program: &mut ResponseProgram) {
    let arguments = match &mut program.operation {
        ResponseOperation::FunctionCallFromRoles { arguments, .. }
        | ResponseOperation::CustomToolCallFromRoles { arguments, .. } => arguments,
        _ => return,
    };
    for argument in arguments {
        if let ResponseArgument::Role { role, .. } = argument
            && *role == SemanticRole::SourceValue
        {
            *role = SemanticRole::ContinuationHandle;
        }
    }
}

pub fn pre_action_t1_binding_root(
    program: &ResponseProgram,
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<String, &'static str> {
    let selectors = program_role_selectors(program).ok_or("primary_selector_missing")?;
    let bindings = selectors
        .into_iter()
        .map(|selector| {
            let witness = witness_for_selector(selector, topology)
                .ok_or("structural_role_missing_or_ambiguous")?;
            let role =
                role_for_witness(topology, witness).ok_or("selected_structural_role_missing")?;
            Ok((
                selector.clone(),
                role.local_role_id,
                witness.value_sha256.clone(),
            ))
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    canonical_json_sha256(&("nando.ms3-pre-action-t1-binding.v1", bindings))
        .map_err(|_| "pre_action_binding_commitment_failed")
}

fn witness_for_selector<'a>(
    selector: &ResponseValueSelector,
    topology: &'a PreActionMultiSourceTopologyV1,
) -> Option<&'a MultiSourceRoleWitnessV1> {
    match selector {
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        } => unique_matching_witness(topology, |witness| {
            (witness.request_reference_ordinal == Some(*ordinal)
                || witness
                    .request_reference_ordinal_candidates
                    .binary_search(ordinal)
                    .is_ok())
                && role_for_witness(topology, witness)
                    .is_some_and(|role| role_type_matches(role.type_class, *value_type))
        }),
        ResponseValueSelector::UniqueScalar { value_type } => {
            let mut witnesses = topology.role_witnesses.iter().filter(|witness| {
                role_for_witness(topology, witness)
                    .is_some_and(|role| role_type_matches(role.type_class, *value_type))
            });
            let witness = witnesses.next()?;
            witnesses.next().is_none().then_some(witness)
        }
        ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal,
            value_type,
        } => unique_matching_witness(topology, |witness| {
            role_for_witness(topology, witness).is_some_and(|role| {
                role.temporal_class == MultiSourceTemporalClassV1::Latest
                    && role.value_ordinal == *scalar_ordinal
                    && role_type_matches(role.type_class, *value_type)
            })
        }),
        ResponseValueSelector::ContinuationHandle { value_type } => {
            unique_matching_witness_prefer_latest(topology, |witness| {
                role_for_witness(topology, witness).is_some_and(|role| {
                    role_type_matches(role.type_class, *value_type)
                        && role_has_relation(
                            topology,
                            role.local_role_id,
                            MultiSourceRelationKindV1::ContinuationHandle,
                        )
                })
            })
        }
        _ => None,
    }
}

fn program_role_selectors(program: &ResponseProgram) -> Option<Vec<&ResponseValueSelector>> {
    let primary = primary_t1_selector(program)?;
    let mut selectors = vec![primary];
    if let ResponseOperation::ProjectSelectedValue {
        renderer: nando_operator_kernel::CollectionOutputRenderer::RenderSequence { segments },
        ..
    } = &program.operation
    {
        for segment in segments {
            if let ResponseRenderSegment::Selected { selector, .. } = segment
                && !selectors.contains(&selector)
            {
                selectors.push(selector);
            }
        }
    }
    Some(selectors)
}

fn role_has_relation(
    topology: &PreActionMultiSourceTopologyV1,
    role_id: u16,
    relation: MultiSourceRelationKindV1,
) -> bool {
    topology.relations.iter().any(|edge| {
        edge.relation == relation
            && edge.source_role_id == role_id
            && edge.target_role_id == role_id
    })
}

fn unique_matching_witness(
    topology: &PreActionMultiSourceTopologyV1,
    mut predicate: impl FnMut(&MultiSourceRoleWitnessV1) -> bool,
) -> Option<&MultiSourceRoleWitnessV1> {
    let mut witnesses = topology
        .role_witnesses
        .iter()
        .filter(|witness| predicate(witness));
    let witness = witnesses.next()?;
    witnesses.next().is_none().then_some(witness)
}

fn unique_matching_witness_prefer_latest(
    topology: &PreActionMultiSourceTopologyV1,
    mut predicate: impl FnMut(&MultiSourceRoleWitnessV1) -> bool,
) -> Option<&MultiSourceRoleWitnessV1> {
    let matches = topology
        .role_witnesses
        .iter()
        .filter(|witness| predicate(witness))
        .collect::<Vec<_>>();
    if let [witness] = matches.as_slice() {
        return Some(*witness);
    }
    let latest = matches
        .into_iter()
        .filter(|witness| {
            role_for_witness(topology, witness)
                .is_some_and(|role| role.temporal_class == MultiSourceTemporalClassV1::Latest)
        })
        .collect::<Vec<_>>();
    let [witness] = latest.as_slice() else {
        return None;
    };
    Some(*witness)
}

fn role_for_witness<'a>(
    topology: &'a PreActionMultiSourceTopologyV1,
    witness: &MultiSourceRoleWitnessV1,
) -> Option<&'a MultiSourceRoleNodeV1> {
    topology
        .roles
        .iter()
        .find(|role| role.local_role_id == witness.local_role_id)
}

struct SelectedObservation<'a> {
    value_root: &'a str,
    value_type: AtomValueType,
    selector: &'a ResponseValueSelector,
}

fn admissible_observations(
    frame: &RelationFrame,
) -> Result<Vec<SelectedObservation<'_>>, &'static str> {
    let mut by_selector = BTreeMap::<&ResponseValueSelector, (&str, AtomValueType)>::new();
    for atom in &frame.atoms {
        let RelationAtom::ObservationSelector { slot_id, selector } = atom else {
            continue;
        };
        let slots = frame
            .atoms
            .iter()
            .filter_map(|candidate| match candidate {
                RelationAtom::TypedSlot {
                    slot_id: candidate_slot,
                    value_type,
                    source: AtomSource::Observation,
                    value_sha256,
                } if candidate_slot == slot_id => Some((value_sha256.as_str(), *value_type)),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        if slots.len() != 1 {
            return Err("physical_observation_binding_ambiguous");
        }
        let (value_root, value_type) = slots
            .into_iter()
            .next()
            .expect("one observation slot remains");
        if let Some(existing) = by_selector.insert(selector, (value_root, value_type))
            && existing != (value_root, value_type)
        {
            return Err("physical_observation_binding_ambiguous");
        }
    }
    if by_selector.is_empty() {
        return Err("selected_observation_missing");
    }
    Ok(by_selector
        .into_iter()
        .map(|(selector, (value_root, value_type))| SelectedObservation {
            value_root,
            value_type,
            selector,
        })
        .collect())
}

fn primary_t1_selector(program: &ResponseProgram) -> Option<&ResponseValueSelector> {
    match &program.operation {
        ResponseOperation::FunctionCallFromRoles { selector, .. }
        | ResponseOperation::CustomToolCallFromRoles { selector, .. }
        | ResponseOperation::ProjectSelectedValue { selector, .. } => Some(selector),
        _ => None,
    }
}

const fn role_type_matches(role: MultiSourceTypeClassV1, value: AtomValueType) -> bool {
    matches!(
        (role, value),
        (
            MultiSourceTypeClassV1::String,
            AtomValueType::String | AtomValueType::Identifier
        ) | (MultiSourceTypeClassV1::Number, AtomValueType::Integer)
            | (MultiSourceTypeClassV1::Boolean, AtomValueType::Boolean)
            | (MultiSourceTypeClassV1::Array, AtomValueType::Collection)
    )
}
