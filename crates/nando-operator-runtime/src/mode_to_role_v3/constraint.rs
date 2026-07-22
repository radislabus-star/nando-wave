use std::collections::BTreeMap;

use nando_core::wave::StructuralRoleSignature;
use nando_operator_kernel::{BindingPredicateV1, ProtocolModeV2, StructuralCandidateFeaturesV3};
use serde::Serialize;

use super::ModeToRoleErrorV3;
use super::feature_codec::{
    call_lineage_tag_v3, capability_class_tag_v3, completion_state_tag_v3, digest_words_v3,
    request_relation_tag_v3, source_event_tag_v3, value_type_tag_v3,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(super) enum CompiledConstraintKindV3 {
    SourceEventClass,
    CallLineage,
    CapabilityClass,
    TemporalDistance,
    CompletionState,
    EventCandidateCardinality,
    ValueType,
    RequestRelation,
    TopologyNeighborhood,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(super) struct CompiledConstraintV3 {
    pub kind: CompiledConstraintKindV3,
    pub slot: u8,
    pub expected: u32,
}

pub(super) const DISPATCH_DIMENSIONS_V3: [(CompiledConstraintKindV3, u8); 16] = [
    (CompiledConstraintKindV3::SourceEventClass, 0),
    (CompiledConstraintKindV3::CallLineage, 0),
    (CompiledConstraintKindV3::CapabilityClass, 0),
    (CompiledConstraintKindV3::TemporalDistance, 0),
    (CompiledConstraintKindV3::CompletionState, 0),
    (CompiledConstraintKindV3::EventCandidateCardinality, 0),
    (CompiledConstraintKindV3::ValueType, 0),
    (CompiledConstraintKindV3::RequestRelation, 0),
    (CompiledConstraintKindV3::TopologyNeighborhood, 0),
    (CompiledConstraintKindV3::TopologyNeighborhood, 1),
    (CompiledConstraintKindV3::TopologyNeighborhood, 2),
    (CompiledConstraintKindV3::TopologyNeighborhood, 3),
    (CompiledConstraintKindV3::TopologyNeighborhood, 4),
    (CompiledConstraintKindV3::TopologyNeighborhood, 5),
    (CompiledConstraintKindV3::TopologyNeighborhood, 6),
    (CompiledConstraintKindV3::TopologyNeighborhood, 7),
];

pub(super) fn compile_constraints_v3(
    mode: &ProtocolModeV2,
) -> Result<Vec<CompiledConstraintV3>, ModeToRoleErrorV3> {
    let source_type = mode
        .program
        .source_role_schema
        .roles
        .first()
        .ok_or(ModeToRoleErrorV3::InvalidSelector)?
        .value_type;
    let mut constraints = BTreeMap::new();
    insert(
        &mut constraints,
        CompiledConstraintKindV3::ValueType,
        0,
        u32::from(value_type_tag_v3(source_type)),
    )?;
    for predicate in &mode.program.selector_program.predicates {
        let (kind, expected) = match predicate {
            BindingPredicateV1::SourceEventClass { value } => (
                CompiledConstraintKindV3::SourceEventClass,
                u32::from(source_event_tag_v3(*value)),
            ),
            BindingPredicateV1::CallLineage { value } => (
                CompiledConstraintKindV3::CallLineage,
                u32::from(call_lineage_tag_v3(*value)),
            ),
            BindingPredicateV1::CapabilityClass { value } => (
                CompiledConstraintKindV3::CapabilityClass,
                u32::from(capability_class_tag_v3(*value)),
            ),
            BindingPredicateV1::TemporalDistance { value } => (
                CompiledConstraintKindV3::TemporalDistance,
                u32::from(*value),
            ),
            BindingPredicateV1::CompletionState { value } => (
                CompiledConstraintKindV3::CompletionState,
                u32::from(completion_state_tag_v3(*value)),
            ),
            BindingPredicateV1::EventCandidateCardinality { value } => (
                CompiledConstraintKindV3::EventCandidateCardinality,
                u32::from(*value),
            ),
            BindingPredicateV1::ValueType { value } => (
                CompiledConstraintKindV3::ValueType,
                u32::from(value_type_tag_v3(*value)),
            ),
            BindingPredicateV1::RequestRelation { value } => (
                CompiledConstraintKindV3::RequestRelation,
                u32::from(request_relation_tag_v3(*value)),
            ),
            BindingPredicateV1::TopologyNeighborhood { root_sha256 } => {
                for (slot, word) in digest_words_v3(root_sha256)?.into_iter().enumerate() {
                    insert(
                        &mut constraints,
                        CompiledConstraintKindV3::TopologyNeighborhood,
                        u8::try_from(slot).map_err(|_| ModeToRoleErrorV3::InvalidSelector)?,
                        word,
                    )?;
                }
                continue;
            }
        };
        insert(&mut constraints, kind, 0, expected)?;
    }
    Ok(constraints.into_values().collect())
}

fn insert(
    constraints: &mut BTreeMap<(CompiledConstraintKindV3, u8), CompiledConstraintV3>,
    kind: CompiledConstraintKindV3,
    slot: u8,
    expected: u32,
) -> Result<(), ModeToRoleErrorV3> {
    let constraint = CompiledConstraintV3 {
        kind,
        slot,
        expected,
    };
    match constraints.insert((kind, slot), constraint) {
        Some(previous) if previous.expected != expected => Err(ModeToRoleErrorV3::InvalidSelector),
        _ => Ok(()),
    }
}

impl CompiledConstraintV3 {
    pub(super) const fn plane(&self) -> u8 {
        match self.kind {
            CompiledConstraintKindV3::SourceEventClass => 1,
            CompiledConstraintKindV3::CallLineage => 2,
            CompiledConstraintKindV3::CapabilityClass => 3,
            CompiledConstraintKindV3::TemporalDistance => 4,
            CompiledConstraintKindV3::CompletionState => 5,
            CompiledConstraintKindV3::EventCandidateCardinality => 6,
            CompiledConstraintKindV3::ValueType => 7,
            CompiledConstraintKindV3::RequestRelation => 8,
            CompiledConstraintKindV3::TopologyNeighborhood => 16_u8.saturating_add(self.slot),
        }
    }

    pub(super) fn signature(&self) -> StructuralRoleSignature {
        StructuralRoleSignature::new(
            0x80_u8.saturating_add(self.kind.tag()),
            1,
            self.slot,
            self.expected,
            vec![self.plane()],
        )
    }
}

pub(super) fn observed_constraints_v3(
    features: &StructuralCandidateFeaturesV3,
) -> Result<[CompiledConstraintV3; 16], ModeToRoleErrorV3> {
    let topology = digest_words_v3(&features.topology_neighborhood_root_sha256)?;
    let expected = [
        u32::from(source_event_tag_v3(features.source_event_class)),
        u32::from(call_lineage_tag_v3(features.call_lineage)),
        u32::from(capability_class_tag_v3(features.capability_class)),
        u32::from(features.temporal_distance),
        u32::from(completion_state_tag_v3(features.completion_state)),
        u32::from(features.event_candidate_cardinality),
        u32::from(value_type_tag_v3(features.value_type)),
        u32::from(request_relation_tag_v3(features.request_relation)),
        topology[0],
        topology[1],
        topology[2],
        topology[3],
        topology[4],
        topology[5],
        topology[6],
        topology[7],
    ];
    Ok(std::array::from_fn(|index| CompiledConstraintV3 {
        kind: DISPATCH_DIMENSIONS_V3[index].0,
        slot: DISPATCH_DIMENSIONS_V3[index].1,
        expected: expected[index],
    }))
}

impl CompiledConstraintKindV3 {
    const fn tag(self) -> u8 {
        match self {
            Self::SourceEventClass => 1,
            Self::CallLineage => 2,
            Self::CapabilityClass => 3,
            Self::TemporalDistance => 4,
            Self::CompletionState => 5,
            Self::EventCandidateCardinality => 6,
            Self::ValueType => 7,
            Self::RequestRelation => 8,
            Self::TopologyNeighborhood => 9,
        }
    }
}
