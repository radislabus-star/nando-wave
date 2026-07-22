use nando_operator_kernel::{
    BindingPredicateV1, BoundProtocolActionV3, BoundProtocolArgumentInputV3, BoundProtocolValueV3,
    CanonicalStructuralRoleV3, ProtocolCapabilityArgumentV3, ProtocolCapabilityKindV3,
    ProtocolModeV2, RuntimeCapabilityKindV3, StructuralCandidateFeaturesV3,
};

use super::super::capability::IndependentCapabilityV3;

pub(super) fn role_matches_mode_v3(
    role: &CanonicalStructuralRoleV3,
    mode: &ProtocolModeV2,
) -> bool {
    features_match_mode_v3(&role.features, mode)
}

pub(crate) fn features_match_mode_v3(
    features: &StructuralCandidateFeaturesV3,
    mode: &ProtocolModeV2,
) -> bool {
    let Some(source_role) = mode.program.source_role_schema.roles.first() else {
        return false;
    };
    source_role.value_type == features.value_type
        && mode
            .program
            .selector_program
            .predicates
            .iter()
            .all(|predicate| predicate_matches_v3(predicate, features))
        && mode
            .program
            .temporal_cardinality_contract
            .completion_states
            .contains(&features.completion_state)
        && mode
            .program
            .temporal_cardinality_contract
            .temporal_distances
            .contains(&features.temporal_distance)
        && mode
            .program
            .temporal_cardinality_contract
            .event_candidate_cardinalities
            .contains(&features.event_candidate_cardinality)
}

fn predicate_matches_v3(
    predicate: &BindingPredicateV1,
    features: &StructuralCandidateFeaturesV3,
) -> bool {
    match predicate {
        BindingPredicateV1::SourceEventClass { value } => features.source_event_class == *value,
        BindingPredicateV1::CallLineage { value } => features.call_lineage == *value,
        BindingPredicateV1::CapabilityClass { value } => features.capability_class == *value,
        BindingPredicateV1::TemporalDistance { value } => features.temporal_distance == *value,
        BindingPredicateV1::CompletionState { value } => features.completion_state == *value,
        BindingPredicateV1::EventCandidateCardinality { value } => {
            features.event_candidate_cardinality == *value
        }
        BindingPredicateV1::ValueType { value } => features.value_type == *value,
        BindingPredicateV1::RequestRelation { value } => features.request_relation == *value,
        BindingPredicateV1::TopologyNeighborhood { root_sha256 } => {
            features.topology_neighborhood_root_sha256 == *root_sha256
        }
    }
}

pub(super) fn capability_matches_mode_v3(
    capability: &IndependentCapabilityV3,
    expected_kind: ProtocolCapabilityKindV3,
    expected_arguments: &[ProtocolCapabilityArgumentV3],
) -> bool {
    let runtime_kind = match expected_kind {
        ProtocolCapabilityKindV3::Function => RuntimeCapabilityKindV3::Function,
        ProtocolCapabilityKindV3::CustomTool => RuntimeCapabilityKindV3::Custom,
    };
    capability.kind == runtime_kind
        && capability.arguments.len() == expected_arguments.len()
        && capability
            .arguments
            .iter()
            .zip(expected_arguments)
            .all(|(actual, expected)| {
                actual.required
                    && actual.ordinal == expected.argument_ordinal()
                    && actual.value_type == expected.value_type()
            })
}

pub(super) fn independent_arguments_v3(
    mode: &ProtocolModeV2,
    capability: &IndependentCapabilityV3,
    value: &BoundProtocolValueV3,
) -> Option<Vec<BoundProtocolArgumentInputV3>> {
    let source_role = mode.program.source_role_schema.roles.first()?;
    mode.program
        .argument_role_schema
        .roles
        .iter()
        .map(|expected| {
            if expected.source_role_id != source_role.role_id {
                return None;
            }
            let physical = capability
                .arguments
                .get(usize::from(expected.argument_ordinal))?;
            (physical.ordinal == expected.argument_ordinal
                && physical.value_type == source_role.value_type
                && value.value_type() == source_role.value_type)
                .then(|| BoundProtocolArgumentInputV3 {
                    argument_ordinal: expected.argument_ordinal,
                    source_role_id: source_role.role_id,
                    physical_name: physical.physical_name.clone(),
                    value: value.clone(),
                })
        })
        .collect()
}

pub(super) fn actor_matches_v3(
    actor: &BoundProtocolActionV3,
    expected: &BoundProtocolActionV3,
) -> bool {
    actor.artifact_root_sha256() == expected.artifact_root_sha256()
        && actor.mode_id_sha256() == expected.mode_id_sha256()
        && actor.executable_mode_root_sha256() == expected.executable_mode_root_sha256()
        && actor.payload_root_sha256() == expected.payload_root_sha256()
        && actor.effect_law_id_sha256() == expected.effect_law_id_sha256()
        && actor.action_class_root_sha256() == expected.action_class_root_sha256()
        && actor.request_view_sha256() == expected.request_view_sha256()
        && actor.capability_id() == expected.capability_id()
        && actor.capability_kind() == expected.capability_kind()
        && actor.physical_symbol() == expected.physical_symbol()
        && actor.arguments() == expected.arguments()
        && actor.semantic_action_sha256() == expected.semantic_action_sha256()
        && actor.physical_action_sha256() == expected.physical_action_sha256()
}
