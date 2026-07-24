use nando_operator_kernel::{
    MULTI_SOURCE_MAX_RELATION_EDGES_V1, MULTI_SOURCE_MAX_ROLE_NODES_V1,
    MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceExtractionStatusV1,
    MultiSourceRelationEdgeV1, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
    MultiSourceTemporalClassV1, MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1,
};
use serde_json::Value;

const FLAG_REQUEST_REFERENCED: u16 = 1;

pub(crate) fn extract_pre_action_multi_source_topology_v1(
    payload: &Value,
    request_text: &str,
) -> PreActionMultiSourceTopologyV1 {
    let outputs = provider_outputs(payload);
    let mut roles = Vec::new();
    for (source_ordinal, output) in outputs.iter().enumerate() {
        collect_roles(
            output,
            u16::try_from(source_ordinal).unwrap_or(u16::MAX),
            source_ordinal + 1 == outputs.len(),
            request_text,
            0,
            &mut roles,
        );
        if roles.len() > MULTI_SOURCE_MAX_ROLE_NODES_V1 {
            return censored("role_budget_exceeded");
        }
    }
    roles.sort_by_key(|role| {
        (
            role.source_ordinal,
            role.depth_bucket,
            role.type_class,
            role.container_class,
            role.cardinality_class,
            role.structural_flags,
            role.value_ordinal,
        )
    });
    for (index, role) in roles.iter_mut().enumerate() {
        role.local_role_id = u16::try_from(index).unwrap_or(u16::MAX);
    }
    let mut relations = Vec::new();
    for pair in roles.windows(2) {
        relations.push(MultiSourceRelationEdgeV1 {
            relation: MultiSourceRelationKindV1::Precedes,
            source_role_id: pair[0].local_role_id,
            target_role_id: pair[1].local_role_id,
        });
    }
    for role in &roles {
        if role.structural_flags & FLAG_REQUEST_REFERENCED != 0 {
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
    relations.sort();
    relations.dedup();
    if relations.len() > MULTI_SOURCE_MAX_RELATION_EDGES_V1 {
        return censored("relation_budget_exceeded");
    }
    PreActionMultiSourceTopologyV1 {
        extraction_status: MultiSourceExtractionStatusV1::Complete,
        grounded_output_count: u16::try_from(outputs.len()).unwrap_or(u16::MAX),
        output_part_count: u16::try_from(roles.len()).unwrap_or(u16::MAX),
        roles,
        relations,
    }
}

fn provider_outputs(payload: &Value) -> Vec<&Value> {
    payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output" | "tool_result")
            )
        })
        .filter_map(|item| item.get("output").or_else(|| item.get("content")))
        .collect()
}

fn collect_roles(
    value: &Value,
    source_ordinal: u16,
    latest: bool,
    request_text: &str,
    depth: u8,
    roles: &mut Vec<MultiSourceRoleNodeV1>,
) {
    match value {
        Value::Array(values) => {
            for value in values {
                collect_roles(
                    value,
                    source_ordinal,
                    latest,
                    request_text,
                    depth.saturating_add(1),
                    roles,
                );
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_roles(
                    value,
                    source_ordinal,
                    latest,
                    request_text,
                    depth.saturating_add(1),
                    roles,
                );
            }
        }
        scalar => {
            let rendered = match scalar {
                Value::String(value) => value.as_str().to_owned(),
                _ => scalar.to_string(),
            };
            roles.push(MultiSourceRoleNodeV1 {
                local_role_id: 0,
                source_ordinal,
                value_ordinal: u16::try_from(roles.len()).unwrap_or(u16::MAX),
                type_class: value_type(scalar),
                container_class: MultiSourceContainerClassV1::Scalar,
                cardinality_class: MultiSourceCardinalityClassV1::One,
                temporal_class: if latest {
                    MultiSourceTemporalClassV1::Latest
                } else {
                    MultiSourceTemporalClassV1::Historical
                },
                depth_bucket: depth.min(7),
                structural_flags: u16::from(
                    !rendered.is_empty() && request_text.contains(&rendered),
                ) * FLAG_REQUEST_REFERENCED,
            });
        }
    }
}

const fn value_type(value: &Value) -> MultiSourceTypeClassV1 {
    match value {
        Value::Null => MultiSourceTypeClassV1::Null,
        Value::Bool(_) => MultiSourceTypeClassV1::Boolean,
        Value::Number(_) => MultiSourceTypeClassV1::Number,
        Value::String(_) => MultiSourceTypeClassV1::String,
        Value::Array(_) => MultiSourceTypeClassV1::Array,
        Value::Object(_) => MultiSourceTypeClassV1::Object,
    }
}

fn censored(reason: &str) -> PreActionMultiSourceTopologyV1 {
    PreActionMultiSourceTopologyV1 {
        extraction_status: MultiSourceExtractionStatusV1::Censored {
            reason: reason.to_owned(),
        },
        grounded_output_count: 0,
        output_part_count: 0,
        roles: Vec::new(),
        relations: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn renamed_and_reordered_fields_keep_source_neutral_topology() {
        let left = json!({"input":[
            {"type":"function_call_output","output":{"alpha":7,"beta":"ok"}},
            {"type":"function_call_output","output":{"gamma":9}}
        ]});
        let right = json!({"input":[
            {"type":"function_call_output","output":{"renamed_b":"ok","renamed_a":7}},
            {"type":"function_call_output","output":{"renamed_c":9}}
        ]});
        assert_eq!(
            extract_pre_action_multi_source_topology_v1(&left, "combine"),
            extract_pre_action_multi_source_topology_v1(&right, "combine")
        );
    }

    #[test]
    fn output_over_budget_is_censored_without_partial_graph() {
        let values = (0..=MULTI_SOURCE_MAX_ROLE_NODES_V1).collect::<Vec<_>>();
        let topology = extract_pre_action_multi_source_topology_v1(
            &json!({"input":[{"type":"function_call_output","output":values}]}),
            "",
        );
        assert!(matches!(
            topology.extraction_status,
            MultiSourceExtractionStatusV1::Censored { .. }
        ));
        assert!(topology.roles.is_empty());
    }
}
