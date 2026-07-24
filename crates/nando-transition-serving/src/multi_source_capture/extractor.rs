use nando_operator_kernel::{
    MULTI_SOURCE_MAX_RELATION_EDGES_V1, MULTI_SOURCE_MAX_ROLE_NODES_V1,
    MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceExtractionStatusV1,
    MultiSourceRelationEdgeV1, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
    MultiSourceTemporalClassV1, MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1,
};
use serde_json::Value;

const FLAG_REQUEST_REFERENCED: u16 = 1;
const MAX_RELEVANT_OUTPUTS_V1: usize = 8;

pub(crate) fn extract_pre_action_multi_source_topology_v1(
    payload: &Value,
    request_text: &str,
) -> PreActionMultiSourceTopologyV1 {
    let outputs = select_relevant_outputs(provider_outputs(payload), request_text);
    let outputs = match outputs {
        Ok(outputs) => outputs,
        Err(reason) => return censored(reason),
    };
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

fn provider_outputs(payload: &Value) -> Vec<Value> {
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
        .map(|value| {
            value
                .as_str()
                .and_then(|text| serde_json::from_str(text).ok())
                .unwrap_or_else(|| value.clone())
        })
        .collect()
}

fn select_relevant_outputs(
    outputs: Vec<Value>,
    request_text: &str,
) -> Result<Vec<Value>, &'static str> {
    let mut metadata = outputs
        .into_iter()
        .enumerate()
        .map(|(ordinal, output)| {
            let scalar_count = scalar_count(&output);
            let request_referenced = output_referenced(&output, request_text);
            (ordinal, output, scalar_count, request_referenced)
        })
        .collect::<Vec<_>>();
    if metadata.is_empty() {
        return Ok(Vec::new());
    }
    if metadata.iter().any(|(_, _, scalar_count, referenced)| {
        *referenced && *scalar_count > MULTI_SOURCE_MAX_ROLE_NODES_V1
    }) {
        return Err("referenced_output_role_budget_exceeded");
    }
    let latest_ordinal = metadata.last().map_or(0, |row| row.0);
    let mut selected = metadata
        .iter()
        .filter(|(ordinal, _, _, referenced)| *referenced || *ordinal == latest_ordinal)
        .map(|row| row.0)
        .collect::<std::collections::BTreeSet<_>>();
    let mut selected_roles = metadata
        .iter()
        .filter(|row| selected.contains(&row.0))
        .map(|row| row.2)
        .sum::<usize>();
    if selected_roles > MULTI_SOURCE_MAX_ROLE_NODES_V1 {
        return Err("relevant_output_role_budget_exceeded");
    }
    for (ordinal, _, scalar_count, _) in metadata.iter().rev() {
        if selected.len() >= MAX_RELEVANT_OUTPUTS_V1 {
            break;
        }
        if selected.contains(ordinal) {
            continue;
        }
        if selected_roles.saturating_add(*scalar_count) > MULTI_SOURCE_MAX_ROLE_NODES_V1 {
            continue;
        }
        selected.insert(*ordinal);
        selected_roles = selected_roles.saturating_add(*scalar_count);
    }
    metadata.retain(|row| selected.contains(&row.0));
    metadata.sort_by_key(|row| row.0);
    Ok(metadata.into_iter().map(|row| row.1).collect())
}

fn scalar_count(value: &Value) -> usize {
    match value {
        Value::Array(values) => values.iter().map(scalar_count).sum(),
        Value::Object(values) => values.values().map(scalar_count).sum(),
        _ => 1,
    }
}

fn output_referenced(value: &Value, request_text: &str) -> bool {
    match value {
        Value::Array(values) => values
            .iter()
            .any(|value| output_referenced(value, request_text)),
        Value::Object(values) => values
            .values()
            .any(|value| output_referenced(value, request_text)),
        Value::String(value) => value.len() >= 2 && request_text.contains(value),
        Value::Number(value) => {
            let rendered = value.to_string();
            rendered.len() >= 2 && request_text.contains(&rendered)
        }
        Value::Bool(value) => request_text.contains(if *value { "true" } else { "false" }),
        Value::Null => false,
    }
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
    use std::time::Instant;

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
        let bytes = serde_json::to_vec(&extract_pre_action_multi_source_topology_v1(
            &left, "combine",
        ))
        .expect("serialize");
        let persisted = String::from_utf8(bytes).expect("utf8");
        for forbidden in ["alpha", "beta", "gamma", "\"ok\"", "\"7\"", "\"9\""] {
            assert!(!persisted.contains(forbidden));
        }
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

    #[test]
    fn long_history_keeps_referenced_and_recent_outputs_inside_budget() {
        let mut input = (0..64)
            .map(|index| {
                serde_json::json!({
                    "type": "function_call_output",
                    "output": {"value": format!("history-value-{index}")}
                })
            })
            .collect::<Vec<_>>();
        input.push(serde_json::json!({
            "type": "function_call_output",
            "output": {"answer": "current-answer"}
        }));
        let topology = extract_pre_action_multi_source_topology_v1(
            &serde_json::json!({"input": input}),
            "use history-value-7 and current-answer",
        );
        assert!(matches!(
            topology.extraction_status,
            MultiSourceExtractionStatusV1::Complete
        ));
        assert!(topology.roles.len() <= MULTI_SOURCE_MAX_ROLE_NODES_V1);
        assert!(topology.grounded_output_count <= MAX_RELEVANT_OUTPUTS_V1 as u16);
        assert!(
            topology
                .roles
                .iter()
                .filter(|role| role.structural_flags & FLAG_REQUEST_REFERENCED != 0)
                .count()
                >= 2
        );
    }

    #[test]
    fn oversized_referenced_output_remains_censored() {
        let values = (100..100 + MULTI_SOURCE_MAX_ROLE_NODES_V1 + 1).collect::<Vec<_>>();
        let request = values
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        let topology = extract_pre_action_multi_source_topology_v1(
            &serde_json::json!({
                "input": [{"type": "function_call_output", "output": values}]
            }),
            &request,
        );
        assert!(matches!(
            topology.extraction_status,
            MultiSourceExtractionStatusV1::Censored { .. }
        ));
    }

    #[test]
    fn bounded_extractor_p99_stays_inside_hot_budget() {
        let payload = json!({"input":[
            {"type":"function_call_output","output":"{\"left\":7,\"right\":9}"},
            {"type":"function_call_output","output":"{\"status\":\"ready\"}"}
        ]});
        let mut micros = Vec::with_capacity(4_096);
        for _ in 0..4_096 {
            let started = Instant::now();
            let topology =
                extract_pre_action_multi_source_topology_v1(&payload, "combine 7 and ready");
            assert!(matches!(
                topology.extraction_status,
                MultiSourceExtractionStatusV1::Complete
            ));
            micros.push(started.elapsed().as_micros());
        }
        micros.sort_unstable();
        let p99 = micros[micros.len() * 99 / 100];
        eprintln!("multi_source_extractor_p99_us={p99}");
        assert!(p99 <= 250, "extractor p99 {p99}us exceeds 250us");
    }
}
