use nando_operator_kernel::{
    AtomValueType, CollectionOutputRenderer, CollectionProgramStep, CollectionScalarType,
    MultiSourceContainerClassV1, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
    MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1, MultiSourceTypeClassV1,
    PreActionMultiSourceTopologyV1, ResponseOperation, ResponseProgram, ResponseRenderSegment,
    ResponseValueSelector, canonical_json_sha256,
};

pub fn pre_action_t1_binding_root(
    program: &ResponseProgram,
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<String, &'static str> {
    if matches!(
        program.operation,
        ResponseOperation::ComposeCollection { .. }
    ) {
        return pre_action_collection_binding_root(program, topology);
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

fn pre_action_collection_binding_root(
    program: &ResponseProgram,
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<String, &'static str> {
    let ResponseOperation::ComposeCollection {
        steps, renderer, ..
    } = &program.operation
    else {
        return Err("collection_program_required");
    };
    let requested_source_ordinal = steps.iter().find_map(|step| match step {
        CollectionProgramStep::SelectTurnOutput { output_ordinal } => Some(*output_ordinal),
        _ => None,
    });
    let roles = topology
        .roles
        .iter()
        .filter(|role| role.container_class != MultiSourceContainerClassV1::Scalar)
        .filter(|role| {
            requested_source_ordinal.is_none_or(|ordinal| role.source_ordinal == ordinal)
        })
        .collect::<Vec<_>>();
    let [role] = roles.as_slice() else {
        return Err("collection_role_missing_or_ambiguous");
    };
    let witness = unique_matching_witness(topology, |witness| {
        witness.local_role_id == role.local_role_id
    })
    .ok_or("collection_role_witness_missing_or_ambiguous")?;
    let mut implicit_bindings = Vec::new();
    for step in steps {
        let value_type = match step {
            CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { value_type } => {
                collection_scalar_atom_type(*value_type)
            }
            _ => continue,
        };
        let scalar_witness = unique_matching_witness(topology, |candidate| {
            role_for_witness(topology, candidate).is_some_and(|candidate_role| {
                candidate_role.container_class == MultiSourceContainerClassV1::Scalar
                    && role_type_matches(candidate_role.type_class, value_type)
            })
        })
        .ok_or("collection_selector_role_missing_or_ambiguous")?;
        let scalar_role = role_for_witness(topology, scalar_witness)
            .filter(|candidate| candidate.container_class == MultiSourceContainerClassV1::Scalar)
            .ok_or("collection_selector_role_not_scalar")?;
        implicit_bindings.push((
            "request_value",
            value_type,
            scalar_role.local_role_id,
            scalar_role.source_ordinal,
            scalar_witness.value_sha256.as_str(),
        ));
    }
    implicit_bindings.sort();
    let selector_bindings = collection_selector_witnesses(steps, renderer, topology)?
        .into_iter()
        .map(|(selector, selector_witness)| {
            let selector_role = role_for_witness(topology, selector_witness)
                .filter(|candidate| {
                    candidate.container_class == MultiSourceContainerClassV1::Scalar
                })
                .ok_or("collection_selector_role_not_scalar")?;
            Ok((
                selector.clone(),
                selector_role.local_role_id,
                selector_role.source_ordinal,
                selector_witness.value_sha256.as_str(),
            ))
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    canonical_json_sha256(&(
        "nando.ms3-pre-action-t1-collection-binding.v3",
        role.local_role_id,
        role.source_ordinal,
        witness.value_sha256.as_str(),
        implicit_bindings,
        selector_bindings,
    ))
    .map_err(|_| "pre_action_binding_commitment_failed")
}

pub fn pre_action_t1_selector_witnesses_v1(
    program: &ResponseProgram,
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<Vec<(ResponseValueSelector, String)>, &'static str> {
    let ResponseOperation::ComposeCollection {
        steps, renderer, ..
    } = &program.operation
    else {
        return Err("collection_program_required");
    };
    collection_selector_witnesses(steps, renderer, topology).map(|bindings| {
        bindings
            .into_iter()
            .map(|(selector, witness)| (selector.clone(), witness.value_sha256.clone()))
            .collect()
    })
}

fn collection_selector_witnesses<'a>(
    steps: &'a [CollectionProgramStep],
    renderer: &'a CollectionOutputRenderer,
    topology: &'a PreActionMultiSourceTopologyV1,
) -> Result<Vec<(&'a ResponseValueSelector, &'a MultiSourceRoleWitnessV1)>, &'static str> {
    let mut selectors = steps
        .iter()
        .filter_map(|step| match step {
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { selector, .. } => {
                Some(selector)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
        selectors.extend(segments.iter().filter_map(|segment| match segment {
            ResponseRenderSegment::Selected { selector, .. } => Some(selector),
            _ => None,
        }));
    }
    selectors.sort();
    selectors.dedup();
    selectors
        .into_iter()
        .map(|selector| {
            let witness = witness_for_selector(selector, topology)
                .ok_or("collection_selector_role_missing_or_ambiguous")?;
            Ok((selector, witness))
        })
        .collect()
}

const fn collection_scalar_atom_type(value_type: CollectionScalarType) -> AtomValueType {
    match value_type {
        CollectionScalarType::String => AtomValueType::String,
        CollectionScalarType::Integer => AtomValueType::Integer,
        CollectionScalarType::Boolean => AtomValueType::Boolean,
    }
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
