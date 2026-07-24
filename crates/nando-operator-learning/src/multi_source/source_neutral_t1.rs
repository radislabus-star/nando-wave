use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    AtomSource, AtomValueType, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
    MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1, MultiSourceTypeClassV1, RelationAtom,
    RelationFrame, ResponseArgument, ResponseOperation, ResponseProgram, ResponseRenderSegment,
    ResponseValueSelector, SemanticRole, response_program_version_root_sha256,
};

use super::BlindThenRevealJoinedTransitionV1;

pub(super) fn enumerate_source_neutral_t1_candidates(
    joined: &BlindThenRevealJoinedTransitionV1,
    frame: &RelationFrame,
) -> Result<BTreeMap<String, ResponseProgram>, &'static str> {
    let physical =
        crate::synthesis::enumerate_response_program_candidates(std::slice::from_ref(frame));
    if physical.is_empty() {
        return Err("physical_t1_program_missing");
    }
    let structurally_valid = physical
        .into_iter()
        .filter_map(|program| source_neutralize_t1_program(&program, joined, frame))
        .filter(|program| program.validate().is_ok())
        .collect::<Vec<_>>();
    if structurally_valid.is_empty() {
        return Err(source_neutral_t1_blocker(joined, frame));
    }
    let candidates = structurally_valid
        .into_iter()
        .filter(|program| t1_program_is_consistent(program, joined, frame))
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

pub(super) fn t1_program_is_consistent(
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
    if selected_observations(frame).is_some_and(|observations| observations.len() > 1) {
        return multi_role_program_consistency_blocker(program, joined, frame);
    }
    let Some((_, selected_value_root, _, observed_selector)) = selected_observation(frame) else {
        return Some("selected_observation_missing_or_ambiguous");
    };
    let Some(expected_witness) = witness_for_program(program, joined) else {
        return Some("structural_role_missing_or_ambiguous");
    };
    if expected_witness.value_sha256 != selected_value_root {
        return Some("structural_role_value_mismatch");
    }
    let Some(structural_selector) = primary_t1_selector(program) else {
        return Some("primary_selector_missing");
    };
    let mut bound = program.clone();
    if replace_t1_selector(&mut bound, structural_selector, observed_selector).is_none() {
        return Some("selector_rewrite_failed");
    }
    (!crate::synthesis::program_is_consistent(&bound, frame))
        .then_some("physical_transition_mismatch")
}

fn multi_role_program_consistency_blocker(
    program: &ResponseProgram,
    joined: &BlindThenRevealJoinedTransitionV1,
    frame: &RelationFrame,
) -> Option<&'static str> {
    let Some(observations) = selected_observations(frame) else {
        return Some("selected_observation_missing_or_ambiguous");
    };
    let Some(selectors) = program_role_selectors(program) else {
        return Some("primary_selector_missing");
    };
    if selectors.len() != observations.len() {
        return Some("structural_role_count_mismatch");
    }
    let mut bound = program.clone();
    let mut used_observations = BTreeMap::<ResponseValueSelector, ()>::new();
    for selector in selectors {
        let Some(witness) = witness_for_selector(selector, joined) else {
            return Some("structural_role_missing_or_ambiguous");
        };
        let Some(role) = role_for_witness(joined, witness) else {
            return Some("selected_structural_role_missing");
        };
        let matches = observations
            .iter()
            .filter(|observation| {
                observation.value_root == witness.value_sha256
                    && role_type_matches(role.type_class, observation.value_type)
            })
            .collect::<Vec<_>>();
        let [observation] = matches.as_slice() else {
            return Some("structural_role_value_mismatch");
        };
        if used_observations
            .insert(observation.selector.clone(), ())
            .is_some()
        {
            return Some("structural_role_binding_ambiguous");
        }
        if replace_program_selector(&mut bound, selector, observation.selector).is_none() {
            return Some("selector_rewrite_failed");
        }
    }
    if used_observations.len() != observations.len() {
        return Some("structural_role_coverage_incomplete");
    }
    (!crate::synthesis::program_is_consistent(&bound, frame))
        .then_some("physical_transition_mismatch")
}

fn source_neutralize_t1_program(
    program: &ResponseProgram,
    joined: &BlindThenRevealJoinedTransitionV1,
    frame: &RelationFrame,
) -> Option<ResponseProgram> {
    let observations = selected_observations(frame)?;
    if observations.len() > 1 {
        let mut candidate = program.clone();
        for observation in observations {
            let witness = selected_witness(joined, observation.value_root, observation.value_type)?;
            let role = role_for_witness(joined, witness)?;
            let selector =
                structural_selector_for_role(joined, role, witness, observation.value_type)?;
            replace_program_selector(&mut candidate, observation.selector, &selector)?;
        }
        return Some(candidate);
    }
    let (_, selected_value_root, selected_value_type, observed_selector) =
        selected_observation(frame)?;
    let witness = selected_witness(joined, selected_value_root, selected_value_type)?;
    let role = joined
        .topology
        .roles
        .iter()
        .find(|role| role.local_role_id == witness.local_role_id)?;
    if !role_type_matches(role.type_class, selected_value_type) {
        return None;
    }
    let selector = structural_selector_for_role(joined, role, witness, selected_value_type)?;
    let mut candidate = program.clone();
    replace_t1_selector(&mut candidate, observed_selector, &selector)?;
    Some(candidate)
}

fn structural_selector_for_role(
    joined: &BlindThenRevealJoinedTransitionV1,
    role: &MultiSourceRoleNodeV1,
    witness: &MultiSourceRoleWitnessV1,
    value_type: AtomValueType,
) -> Option<ResponseValueSelector> {
    if role_has_relation(
        joined,
        role.local_role_id,
        MultiSourceRelationKindV1::ContinuationHandle,
    ) {
        Some(ResponseValueSelector::ContinuationHandle { value_type })
    } else if let Some(ordinal) = witness.request_reference_ordinal {
        Some(ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        })
    } else if joined.topology.roles.len() == 1 {
        Some(ResponseValueSelector::UniqueScalar { value_type })
    } else if role.temporal_class == MultiSourceTemporalClassV1::Latest {
        Some(ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal: role.value_ordinal,
            value_type,
        })
    } else {
        None
    }
}

fn source_neutral_t1_blocker(
    joined: &BlindThenRevealJoinedTransitionV1,
    frame: &RelationFrame,
) -> &'static str {
    let Some((_, selected_value_root, selected_value_type, _)) = selected_observation(frame) else {
        return "selected_observation_missing_or_ambiguous";
    };
    let Some(witness) = selected_witness(joined, selected_value_root, selected_value_type) else {
        let matching = joined
            .topology
            .role_witnesses
            .iter()
            .filter(|witness| {
                witness.value_sha256 == selected_value_root
                    && role_for_witness(joined, witness)
                        .is_some_and(|role| role_type_matches(role.type_class, selected_value_type))
            })
            .count();
        return if matching == 0 {
            "selected_role_witness_missing"
        } else {
            "selected_role_witness_ambiguous"
        };
    };
    let Some(role) = role_for_witness(joined, witness) else {
        return "selected_structural_role_missing";
    };
    if !role_type_matches(role.type_class, selected_value_type) {
        return "selected_structural_role_type_mismatch";
    }
    if witness.request_reference_ordinal.is_none()
        && joined.topology.roles.len() != 1
        && role.temporal_class != MultiSourceTemporalClassV1::Latest
    {
        return "selected_structural_selector_missing";
    }
    "physical_program_selector_rewrite_failed"
}

fn replace_t1_selector(
    program: &mut ResponseProgram,
    observed: &ResponseValueSelector,
    structural: &ResponseValueSelector,
) -> Option<()> {
    match &mut program.operation {
        ResponseOperation::FunctionCallFromRoles { selector, .. }
        | ResponseOperation::CustomToolCallFromRoles { selector, .. } => {
            if selector != observed {
                return None;
            }
            *selector = structural.clone();
        }
        ResponseOperation::ProjectSelectedValue {
            selector, renderer, ..
        } => {
            if selector != observed {
                return None;
            }
            *selector = structural.clone();
            if let nando_operator_kernel::CollectionOutputRenderer::RenderSequence { segments } =
                renderer
            {
                for segment in segments {
                    if let ResponseRenderSegment::Selected { selector, .. } = segment
                        && selector == observed
                    {
                        *selector = structural.clone();
                    }
                }
            }
        }
        _ => return None,
    }
    if matches!(structural, ResponseValueSelector::ContinuationHandle { .. }) {
        normalize_continuation_argument_roles(program);
    }
    Some(())
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

fn witness_for_program<'a>(
    program: &ResponseProgram,
    joined: &'a BlindThenRevealJoinedTransitionV1,
) -> Option<&'a MultiSourceRoleWitnessV1> {
    witness_for_selector(primary_t1_selector(program)?, joined)
}

fn witness_for_selector<'a>(
    selector: &ResponseValueSelector,
    joined: &'a BlindThenRevealJoinedTransitionV1,
) -> Option<&'a MultiSourceRoleWitnessV1> {
    match selector {
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        } => unique_matching_witness(joined, |witness| {
            witness.request_reference_ordinal == Some(*ordinal)
                && role_for_witness(joined, witness)
                    .is_some_and(|role| role_type_matches(role.type_class, *value_type))
        }),
        ResponseValueSelector::UniqueScalar { value_type } => {
            let mut witnesses = joined.topology.role_witnesses.iter().filter(|witness| {
                role_for_witness(joined, witness)
                    .is_some_and(|role| role_type_matches(role.type_class, *value_type))
            });
            let witness = witnesses.next()?;
            witnesses.next().is_none().then_some(witness)
        }
        ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal,
            value_type,
        } => unique_matching_witness(joined, |witness| {
            role_for_witness(joined, witness).is_some_and(|role| {
                role.temporal_class == MultiSourceTemporalClassV1::Latest
                    && role.value_ordinal == *scalar_ordinal
                    && role_type_matches(role.type_class, *value_type)
            })
        }),
        ResponseValueSelector::ContinuationHandle { value_type } => {
            unique_matching_witness_prefer_latest(joined, |witness| {
                role_for_witness(joined, witness).is_some_and(|role| {
                    role_type_matches(role.type_class, *value_type)
                        && role_has_relation(
                            joined,
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
    joined: &BlindThenRevealJoinedTransitionV1,
    role_id: u16,
    relation: MultiSourceRelationKindV1,
) -> bool {
    joined.topology.relations.iter().any(|edge| {
        edge.relation == relation
            && edge.source_role_id == role_id
            && edge.target_role_id == role_id
    })
}

fn unique_matching_witness(
    joined: &BlindThenRevealJoinedTransitionV1,
    mut predicate: impl FnMut(&MultiSourceRoleWitnessV1) -> bool,
) -> Option<&MultiSourceRoleWitnessV1> {
    let mut witnesses = joined
        .topology
        .role_witnesses
        .iter()
        .filter(|witness| predicate(witness));
    let witness = witnesses.next()?;
    witnesses.next().is_none().then_some(witness)
}

fn unique_matching_witness_prefer_latest(
    joined: &BlindThenRevealJoinedTransitionV1,
    mut predicate: impl FnMut(&MultiSourceRoleWitnessV1) -> bool,
) -> Option<&MultiSourceRoleWitnessV1> {
    let matches = joined
        .topology
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
            role_for_witness(joined, witness)
                .is_some_and(|role| role.temporal_class == MultiSourceTemporalClassV1::Latest)
        })
        .collect::<Vec<_>>();
    let [witness] = latest.as_slice() else {
        return None;
    };
    Some(*witness)
}

fn role_for_witness<'a>(
    joined: &'a BlindThenRevealJoinedTransitionV1,
    witness: &MultiSourceRoleWitnessV1,
) -> Option<&'a MultiSourceRoleNodeV1> {
    joined
        .topology
        .roles
        .iter()
        .find(|role| role.local_role_id == witness.local_role_id)
}

fn selected_witness<'a>(
    joined: &'a BlindThenRevealJoinedTransitionV1,
    value_root: &str,
    value_type: AtomValueType,
) -> Option<&'a MultiSourceRoleWitnessV1> {
    let witnesses = joined
        .topology
        .role_witnesses
        .iter()
        .filter(|witness| {
            witness.value_sha256 == value_root
                && role_for_witness(joined, witness)
                    .is_some_and(|role| role_type_matches(role.type_class, value_type))
        })
        .collect::<Vec<_>>();
    if let [witness] = witnesses.as_slice() {
        return Some(*witness);
    }
    // The completed frame observes the current tool result. A repeated scalar
    // in historical outputs must not erase an otherwise unique latest role.
    let latest = witnesses
        .into_iter()
        .filter(|witness| {
            role_for_witness(joined, witness)
                .is_some_and(|role| role.temporal_class == MultiSourceTemporalClassV1::Latest)
        })
        .collect::<Vec<_>>();
    let [witness] = latest.as_slice() else {
        return None;
    };
    Some(*witness)
}

fn selected_observation(
    frame: &RelationFrame,
) -> Option<(u16, &str, AtomValueType, &ResponseValueSelector)> {
    let mut selectors = frame.atoms.iter().filter_map(|atom| match atom {
        RelationAtom::ObservationSelector { slot_id, selector } => Some((*slot_id, selector)),
        _ => None,
    });
    let (slot_id, selector) = selectors.next()?;
    if selectors.next().is_some() {
        return None;
    }
    let mut slots = frame.atoms.iter().filter_map(|atom| match atom {
        RelationAtom::TypedSlot {
            slot_id: candidate,
            value_type,
            source: AtomSource::Observation,
            value_sha256,
        } if *candidate == slot_id => Some((value_sha256.as_str(), *value_type)),
        _ => None,
    });
    let (value_root, value_type) = slots.next()?;
    slots
        .next()
        .is_none()
        .then_some((slot_id, value_root, value_type, selector))
}

struct SelectedObservation<'a> {
    value_root: &'a str,
    value_type: AtomValueType,
    selector: &'a ResponseValueSelector,
}

fn selected_observations(frame: &RelationFrame) -> Option<Vec<SelectedObservation<'_>>> {
    let observations = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ObservationSelector { slot_id, selector } => {
                let slots = frame
                    .atoms
                    .iter()
                    .filter_map(|candidate| match candidate {
                        RelationAtom::TypedSlot {
                            slot_id: candidate_slot,
                            value_type,
                            source: AtomSource::Observation,
                            value_sha256,
                        } if candidate_slot == slot_id => {
                            Some((value_sha256.as_str(), *value_type))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let [(value_root, value_type)] = slots.as_slice() else {
                    return None;
                };
                Some(SelectedObservation {
                    value_root,
                    value_type: *value_type,
                    selector,
                })
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if observations.is_empty() {
        return None;
    }
    let unique_selectors = observations
        .iter()
        .map(|observation| observation.selector)
        .collect::<BTreeSet<_>>();
    (unique_selectors.len() == observations.len()).then_some(observations)
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
