use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    MultiSourceContainerClassV1, MultiSourceExtractionStatusV1, MultiSourceRelationKindV1,
    MultiSourceTemporalClassV1, PreActionMultiSourceTopologyV1, canonical_json_sha256,
};
use serde::{Deserialize, Serialize};

use super::{BlindThenRevealJoinedTransitionV1, CompletedEffectAtomV1};

pub const MULTI_SOURCE_FACTORIZED_ROW_SCHEMA_V1: &str = "nando.multi-source-factorized-row.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiSourceReasonV1 {
    MultipleGroundedOutputs,
    MultipleOutputParts,
    SingleGroundedOutput,
    NoGroundedOutput,
    Censored,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreActionShapeClassV1 {
    SingleRoleProjection,
    OneOutputManyScalarRoles,
    ManyOutputsLatestRelevantRole,
    CrossOutputDependency,
    CollectionPlusScalarMetadata,
    MultipleCollections,
    Unresolved,
    Censored,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletedEffectFormV1 {
    SingleRoleProjection,
    MultiRoleRendering,
    StatusValueBranch,
    CollectionTransform,
    CrossOutputComposition,
    Unexplained,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FactorizedMultiSourceRowV1 {
    pub schema: String,
    pub join_root_sha256: String,
    pub turn_intent_id_sha256: String,
    pub input_tokens: u64,
    pub accepted: bool,
    pub reason: MultiSourceReasonV1,
    pub pre_action_shape: PreActionShapeClassV1,
    pub completed_effect: CompletedEffectFormV1,
    pub applicability_shape_root_sha256: String,
    pub discovery_work_root_sha256: String,
}

#[derive(Serialize)]
struct ApplicabilityShapeDigest {
    schema: &'static str,
    reason: MultiSourceReasonV1,
    pre_action_shape: PreActionShapeClassV1,
    grounded_output_count: u16,
    output_part_count: u16,
    role_signature: Vec<(String, String, String, String, u8, u16)>,
    relation_signature: Vec<String>,
}

#[must_use]
pub fn factor_multi_source_row_v1(
    joined: &BlindThenRevealJoinedTransitionV1,
) -> FactorizedMultiSourceRowV1 {
    let reason = reason(&joined.topology);
    let pre_action_shape = pre_action_shape(&joined.topology);
    let completed_effect = completed_effect(joined);
    let applicability_shape_root_sha256 = pre_action_applicability_shape_root_v1(&joined.topology)
        .expect("validated source-neutral shape serializes");
    let discovery_work_root_sha256 = canonical_json_sha256(&(
        MULTI_SOURCE_FACTORIZED_ROW_SCHEMA_V1,
        applicability_shape_root_sha256.as_str(),
        completed_effect,
        joined.semantic_action_root_sha256.as_str(),
    ))
    .expect("discovery work shape serializes");
    FactorizedMultiSourceRowV1 {
        schema: MULTI_SOURCE_FACTORIZED_ROW_SCHEMA_V1.to_owned(),
        join_root_sha256: joined.join_root_sha256.clone(),
        turn_intent_id_sha256: joined.turn_intent_id_sha256.clone(),
        input_tokens: joined.input_tokens,
        accepted: joined.accepted,
        reason,
        pre_action_shape,
        completed_effect,
        applicability_shape_root_sha256,
        discovery_work_root_sha256,
    }
}

pub fn pre_action_applicability_shape_root_v1(
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<String, &'static str> {
    topology.validate()?;
    let applicability = ApplicabilityShapeDigest {
        schema: MULTI_SOURCE_FACTORIZED_ROW_SCHEMA_V1,
        reason: reason(topology),
        pre_action_shape: pre_action_shape(topology),
        grounded_output_count: topology.grounded_output_count,
        output_part_count: topology.output_part_count,
        role_signature: topology
            .roles
            .iter()
            .map(|role| {
                (
                    format!("{:?}", role.type_class),
                    format!("{:?}", role.container_class),
                    format!("{:?}", role.cardinality_class),
                    format!("{:?}", role.temporal_class),
                    role.depth_bucket,
                    role.structural_flags,
                )
            })
            .collect(),
        relation_signature: topology
            .relations
            .iter()
            .map(|edge| format!("{:?}", edge.relation))
            .collect(),
    };
    canonical_json_sha256(&applicability)
}

pub fn source_neutral_topology_root_v1(
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<String, &'static str> {
    topology.validate()?;
    let role_index = topology
        .roles
        .iter()
        .enumerate()
        .map(|(index, role)| (role.local_role_id, index))
        .collect::<BTreeMap<_, _>>();
    let roles = topology
        .roles
        .iter()
        .map(|role| {
            (
                role.type_class,
                role.container_class,
                role.cardinality_class,
                role.temporal_class,
                role.depth_bucket,
                role.structural_flags,
            )
        })
        .collect::<Vec<_>>();
    let relations = topology
        .relations
        .iter()
        .map(|edge| {
            Ok((
                edge.relation,
                *role_index
                    .get(&edge.source_role_id)
                    .ok_or("source_neutral_topology_source_role_missing")?,
                *role_index
                    .get(&edge.target_role_id)
                    .ok_or("source_neutral_topology_target_role_missing")?,
            ))
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    canonical_json_sha256(&(
        "nando.k1-source-neutral-role-graph.v1",
        topology.grounded_output_count,
        topology.output_part_count,
        roles,
        relations,
    ))
}

fn reason(topology: &PreActionMultiSourceTopologyV1) -> MultiSourceReasonV1 {
    if matches!(
        topology.extraction_status,
        MultiSourceExtractionStatusV1::Censored { .. }
    ) {
        MultiSourceReasonV1::Censored
    } else if topology.grounded_output_count > 1 {
        MultiSourceReasonV1::MultipleGroundedOutputs
    } else if topology.output_part_count > 1 {
        MultiSourceReasonV1::MultipleOutputParts
    } else if topology.grounded_output_count == 1 {
        MultiSourceReasonV1::SingleGroundedOutput
    } else {
        MultiSourceReasonV1::NoGroundedOutput
    }
}

fn pre_action_shape(topology: &PreActionMultiSourceTopologyV1) -> PreActionShapeClassV1 {
    if matches!(
        topology.extraction_status,
        MultiSourceExtractionStatusV1::Censored { .. }
    ) {
        return PreActionShapeClassV1::Censored;
    }
    let collections = topology
        .roles
        .iter()
        .filter(|role| !matches!(role.container_class, MultiSourceContainerClassV1::Scalar))
        .count();
    let scalars = topology.roles.len().saturating_sub(collections);
    if collections > 1 {
        return PreActionShapeClassV1::MultipleCollections;
    }
    if collections == 1 && scalars > 0 {
        return PreActionShapeClassV1::CollectionPlusScalarMetadata;
    }
    let source_ordinals = topology
        .roles
        .iter()
        .map(|role| role.source_ordinal)
        .collect::<BTreeSet<_>>();
    let cross_output_relation = topology.relations.iter().any(|edge| {
        let source = topology
            .roles
            .iter()
            .find(|role| role.local_role_id == edge.source_role_id);
        let target = topology
            .roles
            .iter()
            .find(|role| role.local_role_id == edge.target_role_id);
        !matches!(edge.relation, MultiSourceRelationKindV1::Precedes)
            && source
                .zip(target)
                .is_some_and(|(left, right)| left.source_ordinal != right.source_ordinal)
    });
    if cross_output_relation {
        return PreActionShapeClassV1::CrossOutputDependency;
    }
    if source_ordinals.len() > 1
        && topology
            .roles
            .iter()
            .any(|role| role.temporal_class == MultiSourceTemporalClassV1::Latest)
    {
        return PreActionShapeClassV1::ManyOutputsLatestRelevantRole;
    }
    match (topology.grounded_output_count, topology.roles.len()) {
        (1, 1) => PreActionShapeClassV1::SingleRoleProjection,
        (1, count) if count > 1 => PreActionShapeClassV1::OneOutputManyScalarRoles,
        _ => PreActionShapeClassV1::Unresolved,
    }
}

fn completed_effect(joined: &BlindThenRevealJoinedTransitionV1) -> CompletedEffectFormV1 {
    let atoms = &joined.effect_atoms;
    if atoms.contains(&CompletedEffectAtomV1::StatusProjection) {
        CompletedEffectFormV1::StatusValueBranch
    } else if atoms.contains(&CompletedEffectAtomV1::ValueProjection)
        && joined
            .topology
            .roles
            .iter()
            .any(|role| !matches!(role.container_class, MultiSourceContainerClassV1::Scalar))
    {
        CompletedEffectFormV1::CollectionTransform
    } else {
        let legacy_role_input = atoms.contains(&CompletedEffectAtomV1::RoleInput);
        let role_input_slots = atoms
            .iter()
            .filter_map(|atom| match atom {
                CompletedEffectAtomV1::RoleInputSlot { slot_id, .. } => Some(*slot_id),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let role_inputs = role_input_slots.len().max(usize::from(legacy_role_input));
        match role_inputs {
            0 => CompletedEffectFormV1::Unexplained,
            1 => CompletedEffectFormV1::SingleRoleProjection,
            _ if joined.topology.grounded_output_count > 1 => {
                CompletedEffectFormV1::CrossOutputComposition
            }
            _ => CompletedEffectFormV1::MultiRoleRendering,
        }
    }
}
