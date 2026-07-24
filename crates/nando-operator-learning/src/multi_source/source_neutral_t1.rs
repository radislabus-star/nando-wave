use std::collections::BTreeMap;

use nando_operator_kernel::{
    AtomSource, AtomValueType, MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1,
    MultiSourceTemporalClassV1, MultiSourceTypeClassV1, RelationAtom, RelationFrame,
    ResponseOperation, ResponseProgram, ResponseRenderSegment, ResponseValueSelector,
    response_program_version_root_sha256,
};

use super::BlindThenRevealJoinedTransitionV1;

pub(super) fn enumerate_source_neutral_t1_candidates(
    joined: &BlindThenRevealJoinedTransitionV1,
    frame: &RelationFrame,
) -> BTreeMap<String, ResponseProgram> {
    crate::synthesis::enumerate_response_program_candidates(std::slice::from_ref(frame))
        .into_iter()
        .filter_map(|program| source_neutralize_t1_program(&program, joined, frame))
        .filter(|program| program.validate().is_ok())
        .filter_map(|program| {
            response_program_version_root_sha256(&program)
                .ok()
                .map(|root| (root, program))
        })
        .collect()
}

pub(super) fn t1_program_is_consistent(
    program: &ResponseProgram,
    joined: &BlindThenRevealJoinedTransitionV1,
    frame: &RelationFrame,
) -> bool {
    let Some((_, selected_value_root, _, observed_selector)) = selected_observation(frame) else {
        return false;
    };
    let Some(expected_witness) = witness_for_program(program, joined) else {
        return false;
    };
    if expected_witness.value_sha256 != selected_value_root
        || joined
            .topology
            .role_witnesses
            .iter()
            .filter(|witness| witness.value_sha256 == selected_value_root)
            .count()
            != 1
    {
        return false;
    }
    let Some(structural_selector) = primary_t1_selector(program) else {
        return false;
    };
    let mut bound = program.clone();
    if replace_t1_selector(&mut bound, structural_selector, observed_selector).is_none() {
        return false;
    }
    crate::synthesis::program_is_consistent(&bound, frame)
}

fn source_neutralize_t1_program(
    program: &ResponseProgram,
    joined: &BlindThenRevealJoinedTransitionV1,
    frame: &RelationFrame,
) -> Option<ResponseProgram> {
    let (_, selected_value_root, selected_value_type, observed_selector) =
        selected_observation(frame)?;
    let witness = unique_selected_witness(joined, selected_value_root)?;
    let role = joined
        .topology
        .roles
        .iter()
        .find(|role| role.local_role_id == witness.local_role_id)?;
    if !role_type_matches(role.type_class, selected_value_type) {
        return None;
    }
    let selector = if let Some(ordinal) = witness.request_reference_ordinal {
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type: selected_value_type,
        }
    } else if joined.topology.roles.len() == 1 {
        ResponseValueSelector::UniqueScalar {
            value_type: selected_value_type,
        }
    } else if role.temporal_class == MultiSourceTemporalClassV1::Latest {
        ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal: role.value_ordinal,
            value_type: selected_value_type,
        }
    } else {
        return None;
    };
    let mut candidate = program.clone();
    replace_t1_selector(&mut candidate, observed_selector, &selector)?;
    Some(candidate)
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
    Some(())
}

fn witness_for_program<'a>(
    program: &ResponseProgram,
    joined: &'a BlindThenRevealJoinedTransitionV1,
) -> Option<&'a MultiSourceRoleWitnessV1> {
    match primary_t1_selector(program)? {
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        } => joined.topology.role_witnesses.iter().find(|witness| {
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
        } => joined.topology.role_witnesses.iter().find(|witness| {
            role_for_witness(joined, witness).is_some_and(|role| {
                role.temporal_class == MultiSourceTemporalClassV1::Latest
                    && role.value_ordinal == *scalar_ordinal
                    && role_type_matches(role.type_class, *value_type)
            })
        }),
        _ => None,
    }
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

fn unique_selected_witness<'a>(
    joined: &'a BlindThenRevealJoinedTransitionV1,
    value_root: &str,
) -> Option<&'a MultiSourceRoleWitnessV1> {
    let mut witnesses = joined
        .topology
        .role_witnesses
        .iter()
        .filter(|witness| witness.value_sha256 == value_root);
    let witness = witnesses.next()?;
    witnesses.next().is_none().then_some(witness)
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
