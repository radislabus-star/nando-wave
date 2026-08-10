use nando_operator_kernel::{
    AtomValueType, CollectionOutputRenderer, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
    MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1, MultiSourceTypeClassV1,
    PreActionMultiSourceTopologyV1, ResponseOperation, ResponseProgram, ResponseRenderSegment,
    ResponseValueSelector, canonical_json_sha256,
};

use super::source_neutral_t1_manifest::{
    PreActionT1ConsumedInputV1, pre_action_t1_input_binding_manifest_v1,
};

pub fn pre_action_t1_consumed_role_ids_v1(
    program: &ResponseProgram,
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<Vec<u16>, &'static str> {
    let mut role_ids = if matches!(
        program.operation,
        ResponseOperation::ComposeCollection { .. }
    ) {
        pre_action_t1_input_binding_manifest_v1(program, topology)?
            .inputs
            .into_iter()
            .map(|input| match input {
                PreActionT1ConsumedInputV1::CollectionSource { local_role_id, .. }
                | PreActionT1ConsumedInputV1::SelectedValue { local_role_id, .. }
                | PreActionT1ConsumedInputV1::ImplicitRequestValue { local_role_id, .. } => {
                    local_role_id
                }
            })
            .collect::<Vec<_>>()
    } else {
        program_role_selectors(program)
            .ok_or("primary_selector_missing")?
            .into_iter()
            .map(|selector| {
                let witness = witness_for_selector(selector, topology)
                    .ok_or("structural_role_missing_or_ambiguous")?;
                role_for_witness(topology, witness)
                    .map(|role| role.local_role_id)
                    .ok_or("selected_structural_role_missing")
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    role_ids.sort_unstable();
    role_ids.dedup();
    if role_ids.is_empty() {
        return Err("consumed_role_set_empty");
    }
    Ok(role_ids)
}

pub fn pre_action_t1_binding_root(
    program: &ResponseProgram,
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<String, &'static str> {
    if matches!(
        program.operation,
        ResponseOperation::ComposeCollection { .. }
    ) {
        return pre_action_t1_input_binding_manifest_v1(program, topology)?.root_sha256();
    }
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

pub(super) fn witness_for_selector<'a>(
    selector: &ResponseValueSelector,
    topology: &'a PreActionMultiSourceTopologyV1,
) -> Option<&'a MultiSourceRoleWitnessV1> {
    match selector {
        ResponseValueSelector::RequestReferencedJsonField { value_type } => {
            unique_matching_witness(topology, |witness| {
                witness_is_request_referenced(witness)
                    && role_for_witness(topology, witness)
                        .is_some_and(|role| role_type_matches(role.type_class, *value_type))
            })
        }
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
        ResponseValueSelector::RequestLastToken | ResponseValueSelector::RequestUniqueLiteral => {
            unique_matching_witness(topology, |witness| {
                witness_is_request_referenced(witness)
                    && role_for_witness(topology, witness).is_some_and(|role| {
                        role_type_matches(role.type_class, AtomValueType::String)
                    })
            })
        }
        _ => None,
    }
}

fn witness_is_request_referenced(witness: &MultiSourceRoleWitnessV1) -> bool {
    witness.request_reference_ordinal.is_some()
        || !witness.request_reference_ordinal_candidates.is_empty()
}

pub(super) fn program_role_selectors(
    program: &ResponseProgram,
) -> Option<Vec<&ResponseValueSelector>> {
    let primary = primary_t1_selector(program)?;
    let mut selectors = vec![primary];
    if let ResponseOperation::ProjectSelectedValue {
        renderer: CollectionOutputRenderer::RenderSequence { segments },
        ..
    }
    | ResponseOperation::ProjectStatus {
        renderer: CollectionOutputRenderer::RenderSequence { segments },
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

pub(super) fn role_has_relation(
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

pub(super) fn role_for_witness<'a>(
    topology: &'a PreActionMultiSourceTopologyV1,
    witness: &MultiSourceRoleWitnessV1,
) -> Option<&'a MultiSourceRoleNodeV1> {
    topology
        .roles
        .iter()
        .find(|role| role.local_role_id == witness.local_role_id)
}

fn primary_t1_selector(program: &ResponseProgram) -> Option<&ResponseValueSelector> {
    match &program.operation {
        ResponseOperation::FunctionCallFromRoles { selector, .. }
        | ResponseOperation::CustomToolCallFromRoles { selector, .. }
        | ResponseOperation::ProjectSelectedValue { selector, .. }
        | ResponseOperation::ProjectStatus { selector, .. } => Some(selector),
        _ => None,
    }
}

pub(super) const fn role_type_matches(role: MultiSourceTypeClassV1, value: AtomValueType) -> bool {
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
