use nando_operator_kernel::{
    MULTI_SOURCE_MAX_RELATION_EDGES_V1, MultiSourceContainerClassV1, MultiSourceRelationEdgeV1,
    MultiSourceRelationKindV1, MultiSourceRoleNodeV1, MultiSourceTemporalClassV1,
};

pub(super) fn build(
    roles: &[MultiSourceRoleNodeV1],
    continuation_role_ids: &[u16],
) -> Result<Vec<MultiSourceRelationEdgeV1>, &'static str> {
    let mut relations = Vec::new();
    let mut output_anchors = Vec::new();
    for output_roles in roles.chunk_by(|left, right| left.source_ordinal == right.source_ordinal) {
        let anchor = output_roles
            .iter()
            .find(|role| role.container_class != MultiSourceContainerClassV1::Scalar)
            .unwrap_or(&output_roles[0]);
        output_anchors.push(anchor.local_role_id);
        for role in output_roles {
            if role.local_role_id == anchor.local_role_id {
                continue;
            }
            relations.push(MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::SameOutput,
                source_role_id: anchor.local_role_id,
                target_role_id: role.local_role_id,
            });
            if anchor.container_class != MultiSourceContainerClassV1::Scalar {
                relations.push(MultiSourceRelationEdgeV1 {
                    relation: MultiSourceRelationKindV1::Contains,
                    source_role_id: anchor.local_role_id,
                    target_role_id: role.local_role_id,
                });
            }
        }
    }
    for pair in output_anchors.windows(2) {
        relations.push(MultiSourceRelationEdgeV1 {
            relation: MultiSourceRelationKindV1::Precedes,
            source_role_id: pair[0],
            target_role_id: pair[1],
        });
    }
    for role in roles {
        if role.structural_flags & super::REQUEST_REFERENCED_FLAG_V2 != 0 {
            relations.push(MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::RequestReferencesRole,
                source_role_id: role.local_role_id,
                target_role_id: role.local_role_id,
            });
        }
        if role.temporal_class == MultiSourceTemporalClassV1::Latest {
            relations.push(MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::LatestOutput,
                source_role_id: role.local_role_id,
                target_role_id: role.local_role_id,
            });
        }
    }
    for role_id in continuation_role_ids {
        relations.push(MultiSourceRelationEdgeV1 {
            relation: MultiSourceRelationKindV1::ContinuationHandle,
            source_role_id: *role_id,
            target_role_id: *role_id,
        });
    }
    relations.sort();
    relations.dedup();
    if relations.len() > MULTI_SOURCE_MAX_RELATION_EDGES_V1 {
        return Err("relation_budget_exceeded");
    }
    Ok(relations)
}
