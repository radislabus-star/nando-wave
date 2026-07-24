use std::collections::BTreeSet;

use nando_operator_kernel::{
    MultiSourceContainerClassV1, MultiSourceExtractionStatusV1, MultiSourceRelationKindV1,
    MultiSourceTemporalClassV1, canonical_json_sha256,
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
    let reason = reason(joined);
    let pre_action_shape = pre_action_shape(joined);
    let completed_effect = completed_effect(joined);
    let applicability = ApplicabilityShapeDigest {
        schema: MULTI_SOURCE_FACTORIZED_ROW_SCHEMA_V1,
        reason,
        pre_action_shape,
        grounded_output_count: joined.topology.grounded_output_count,
        output_part_count: joined.topology.output_part_count,
        role_signature: joined
            .topology
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
        relation_signature: joined
            .topology
            .relations
            .iter()
            .map(|edge| format!("{:?}", edge.relation))
            .collect(),
    };
    let applicability_shape_root_sha256 =
        canonical_json_sha256(&applicability).expect("source-neutral shape serializes");
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

fn reason(joined: &BlindThenRevealJoinedTransitionV1) -> MultiSourceReasonV1 {
    if matches!(
        joined.topology.extraction_status,
        MultiSourceExtractionStatusV1::Censored { .. }
    ) {
        MultiSourceReasonV1::Censored
    } else if joined.topology.grounded_output_count > 1 {
        MultiSourceReasonV1::MultipleGroundedOutputs
    } else if joined.topology.output_part_count > 1 {
        MultiSourceReasonV1::MultipleOutputParts
    } else if joined.topology.grounded_output_count == 1 {
        MultiSourceReasonV1::SingleGroundedOutput
    } else {
        MultiSourceReasonV1::NoGroundedOutput
    }
}

fn pre_action_shape(joined: &BlindThenRevealJoinedTransitionV1) -> PreActionShapeClassV1 {
    if matches!(
        joined.topology.extraction_status,
        MultiSourceExtractionStatusV1::Censored { .. }
    ) {
        return PreActionShapeClassV1::Censored;
    }
    let collections = joined
        .topology
        .roles
        .iter()
        .filter(|role| !matches!(role.container_class, MultiSourceContainerClassV1::Scalar))
        .count();
    let scalars = joined.topology.roles.len().saturating_sub(collections);
    if collections > 1 {
        return PreActionShapeClassV1::MultipleCollections;
    }
    if collections == 1 && scalars > 0 {
        return PreActionShapeClassV1::CollectionPlusScalarMetadata;
    }
    let source_ordinals = joined
        .topology
        .roles
        .iter()
        .map(|role| role.source_ordinal)
        .collect::<BTreeSet<_>>();
    let cross_output_relation = joined.topology.relations.iter().any(|edge| {
        let source = joined
            .topology
            .roles
            .iter()
            .find(|role| role.local_role_id == edge.source_role_id);
        let target = joined
            .topology
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
        && joined
            .topology
            .roles
            .iter()
            .any(|role| role.temporal_class == MultiSourceTemporalClassV1::Latest)
    {
        return PreActionShapeClassV1::ManyOutputsLatestRelevantRole;
    }
    match (
        joined.topology.grounded_output_count,
        joined.topology.roles.len(),
    ) {
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
        let role_inputs = atoms
            .iter()
            .filter(|atom| {
                matches!(
                    atom,
                    CompletedEffectAtomV1::RoleInput | CompletedEffectAtomV1::RoleInputSlot { .. }
                )
            })
            .count();
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
