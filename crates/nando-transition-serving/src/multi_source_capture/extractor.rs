use nando_operator_kernel::{
    MULTI_SOURCE_MAX_RELATION_EDGES_V1, MULTI_SOURCE_MAX_ROLE_NODES_V1,
    MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceExtractionStatusV1,
    MultiSourceRelationEdgeV1, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
    MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1, MultiSourceTypeClassV1,
    PreActionMultiSourceTopologyV1,
};
use nando_operator_runtime::{
    ObservedJsonScalarRole, ObservedScalarRoleClass, observed_continuation_handle_role,
    observed_json_scalar_roles,
};
use serde_json::Value;

const FLAG_REQUEST_REFERENCED: u16 = 1;
const MAX_RELEVANT_OUTPUTS_V1: usize = 8;

struct CollectedRole {
    node: MultiSourceRoleNodeV1,
    value_sha256: String,
    request_positions: Vec<u16>,
    request_reference_ordinal: Option<u16>,
    request_reference_ordinal_candidates: Vec<u16>,
    json_path_sha256: [u8; 32],
    continuation_handle: bool,
}

pub(crate) fn extract_pre_action_multi_source_topology_v1(
    payload: &Value,
    request_text: &str,
) -> PreActionMultiSourceTopologyV1 {
    let outputs = select_relevant_outputs(provider_outputs(payload), request_text);
    let outputs = match outputs {
        Ok(outputs) => outputs,
        Err(reason) => return censored(reason),
    };
    let mut collected = Vec::new();
    for (source_ordinal, (_, output_roles)) in outputs.iter().enumerate() {
        for (value_ordinal, role) in output_roles.iter().enumerate() {
            collected.push(CollectedRole {
                node: role_node(
                    role,
                    u16::try_from(source_ordinal).unwrap_or(u16::MAX),
                    u16::try_from(value_ordinal).unwrap_or(u16::MAX),
                    source_ordinal + 1 == outputs.len(),
                ),
                value_sha256: role.value_sha256.clone(),
                request_positions: role.request_position_candidates.clone(),
                request_reference_ordinal: None,
                request_reference_ordinal_candidates: Vec::new(),
                json_path_sha256: role.json_path_sha256,
                continuation_handle: role.role_class == ObservedScalarRoleClass::ContinuationHandle,
            });
        }
        if collected.len() > MULTI_SOURCE_MAX_ROLE_NODES_V1 {
            return censored("role_budget_exceeded");
        }
    }
    if let Err(reason) = assign_request_reference_ordinals(&mut collected) {
        return censored(reason);
    }
    collected.sort_by_key(|role| {
        (
            role.node.source_ordinal,
            role.node.depth_bucket,
            role.node.type_class,
            role.node.container_class,
            role.node.cardinality_class,
            role.node.structural_flags,
            role.continuation_handle,
            role.request_reference_ordinal,
            role.request_reference_ordinal_candidates.clone(),
            role.node.value_ordinal,
            role.json_path_sha256,
        )
    });
    for (index, role) in collected.iter_mut().enumerate() {
        role.node.local_role_id = u16::try_from(index).unwrap_or(u16::MAX);
    }
    let roles = collected
        .iter()
        .map(|role| role.node.clone())
        .collect::<Vec<_>>();
    let role_witnesses = collected
        .iter()
        .map(|role| MultiSourceRoleWitnessV1 {
            local_role_id: role.node.local_role_id,
            value_sha256: role.value_sha256.clone(),
            request_reference_ordinal: role.request_reference_ordinal,
            request_reference_ordinal_candidates: role.request_reference_ordinal_candidates.clone(),
        })
        .collect::<Vec<_>>();
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
    for role in &collected {
        if role.continuation_handle {
            relations.push(MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::ContinuationHandle,
                source_role_id: role.node.local_role_id,
                target_role_id: role.node.local_role_id,
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
        role_witnesses,
        relations,
    }
}

fn assign_request_reference_ordinals(collected: &mut [CollectedRole]) -> Result<(), &'static str> {
    let referenced = collected
        .iter()
        .enumerate()
        .filter(|(_, role)| !role.request_positions.is_empty())
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if referenced.len() > 16 {
        return Err("request_role_ordinal_budget");
    }
    for index in &referenced {
        let positions = collected[*index].request_positions.clone();
        let mut ordinals = std::collections::BTreeSet::new();
        for position in &positions {
            let mut forced_before = 0_u16;
            let mut possible_before = 0_u16;
            for other_index in &referenced {
                if other_index == index {
                    continue;
                }
                let other = &collected[*other_index].request_positions;
                if other.iter().all(|candidate| candidate < position) {
                    forced_before = forced_before.saturating_add(1);
                    possible_before = possible_before.saturating_add(1);
                } else if other.iter().any(|candidate| candidate <= position) {
                    possible_before = possible_before.saturating_add(1);
                }
            }
            ordinals.extend(forced_before..=possible_before);
        }
        if ordinals.is_empty() || ordinals.iter().any(|ordinal| *ordinal > 15) {
            return Err("request_role_ordinal_budget");
        }
        let ambiguous = positions.len() > 1 || ordinals.len() > 1;
        if ambiguous {
            collected[*index].request_reference_ordinal_candidates = ordinals.into_iter().collect();
        } else {
            collected[*index].request_reference_ordinal = ordinals.into_iter().next();
        }
    }
    Ok(())
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
) -> Result<Vec<(Value, Vec<ObservedJsonScalarRole>)>, &'static str> {
    let mut metadata = outputs
        .into_iter()
        .enumerate()
        .map(|(ordinal, output)| {
            let mut roles = observed_json_scalar_roles(request_text, &output)?;
            if let Ok(continuation) = observed_continuation_handle_role(&output) {
                roles.push(continuation);
            }
            Ok::<_, &'static str>((ordinal, output, roles))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if metadata.is_empty() {
        return Ok(Vec::new());
    }
    let latest_ordinal = metadata.last().map_or(0, |row| row.0);
    let mut references = std::collections::BTreeMap::<u16, Vec<(usize, usize)>>::new();
    for (output_index, (_, _, roles)) in metadata.iter().enumerate() {
        for (role_index, role) in roles.iter().enumerate() {
            for position in &role.request_position_candidates {
                references
                    .entry(*position)
                    .or_default()
                    .push((output_index, role_index));
            }
        }
    }
    for (position, matches) in &references {
        if matches.len() <= 1 {
            continue;
        }
        let latest = matches
            .iter()
            .copied()
            .filter(|(output_index, _)| metadata[*output_index].0 == latest_ordinal)
            .collect::<Vec<_>>();
        for (output_index, role_index) in matches {
            if !latest.contains(&(*output_index, *role_index)) {
                metadata[*output_index].2[*role_index]
                    .request_position_candidates
                    .retain(|candidate| candidate != position);
                let candidates =
                    &metadata[*output_index].2[*role_index].request_position_candidates;
                metadata[*output_index].2[*role_index].request_position =
                    (candidates.len() == 1).then(|| candidates[0]);
            }
        }
    }
    let mut metadata = metadata
        .into_iter()
        .map(|(ordinal, output, roles)| {
            let request_referenced = roles
                .iter()
                .any(|role| !role.request_position_candidates.is_empty());
            (ordinal, output, roles, request_referenced)
        })
        .collect::<Vec<_>>();
    if metadata.iter().any(|(_, _, roles, referenced)| {
        *referenced && roles.len() > MULTI_SOURCE_MAX_ROLE_NODES_V1
    }) {
        return Err("referenced_output_role_budget_exceeded");
    }
    let mut selected = metadata
        .iter()
        .filter(|(ordinal, _, _, referenced)| *referenced || *ordinal == latest_ordinal)
        .map(|row| row.0)
        .collect::<std::collections::BTreeSet<_>>();
    let mut selected_roles = metadata
        .iter()
        .filter(|row| selected.contains(&row.0))
        .map(|row| row.2.len())
        .sum::<usize>();
    if selected_roles > MULTI_SOURCE_MAX_ROLE_NODES_V1 {
        return Err("relevant_output_role_budget_exceeded");
    }
    for (ordinal, _, roles, _) in metadata.iter().rev() {
        if selected.len() >= MAX_RELEVANT_OUTPUTS_V1 {
            break;
        }
        if selected.contains(ordinal) {
            continue;
        }
        if selected_roles.saturating_add(roles.len()) > MULTI_SOURCE_MAX_ROLE_NODES_V1 {
            continue;
        }
        selected.insert(*ordinal);
        selected_roles = selected_roles.saturating_add(roles.len());
    }
    metadata.retain(|row| selected.contains(&row.0));
    metadata.sort_by_key(|row| row.0);
    Ok(metadata
        .into_iter()
        .map(|(_, output, roles, _)| (output, roles))
        .collect())
}

fn role_node(
    role: &ObservedJsonScalarRole,
    source_ordinal: u16,
    value_ordinal: u16,
    latest: bool,
) -> MultiSourceRoleNodeV1 {
    MultiSourceRoleNodeV1 {
        local_role_id: 0,
        source_ordinal,
        value_ordinal,
        type_class: match role.value_type {
            nando_operator_kernel::AtomValueType::String
            | nando_operator_kernel::AtomValueType::Identifier => MultiSourceTypeClassV1::String,
            nando_operator_kernel::AtomValueType::Integer => MultiSourceTypeClassV1::Number,
            nando_operator_kernel::AtomValueType::Boolean => MultiSourceTypeClassV1::Boolean,
            nando_operator_kernel::AtomValueType::Collection => MultiSourceTypeClassV1::Array,
        },
        container_class: MultiSourceContainerClassV1::Scalar,
        cardinality_class: MultiSourceCardinalityClassV1::One,
        temporal_class: if latest {
            MultiSourceTemporalClassV1::Latest
        } else {
            MultiSourceTemporalClassV1::Historical
        },
        depth_bucket: role.depth_bucket,
        structural_flags: u16::from(!role.request_position_candidates.is_empty())
            * FLAG_REQUEST_REFERENCED,
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
        role_witnesses: Vec::new(),
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
    fn continuation_handle_is_a_pre_action_structural_role() {
        let payload = json!({"input":[{
            "type":"function_call_output",
            "output":"Script running with cell ID abc-123"
        }]});
        let topology = extract_pre_action_multi_source_topology_v1(&payload, "continue");

        topology.validate().expect("valid topology");
        let continuation = topology
            .relations
            .iter()
            .find(|edge| edge.relation == MultiSourceRelationKindV1::ContinuationHandle)
            .expect("continuation relation");
        let witness = topology
            .role_witnesses
            .iter()
            .find(|witness| witness.local_role_id == continuation.source_role_id)
            .expect("continuation witness");
        assert_eq!(
            witness.value_sha256,
            nando_operator_kernel::canonical_json_sha256(&json!("abc-123")).expect("hash")
        );
        let encoded = serde_json::to_string(&topology).expect("encode");
        assert!(!encoded.contains("abc-123"));
        assert!(!encoded.contains("Script running"));
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
                let field = if index == 7 {
                    "history_target"
                } else {
                    "value"
                };
                let output = serde_json::Map::from_iter([(
                    field.to_owned(),
                    serde_json::json!(format!("history-value-{index}")),
                )]);
                serde_json::json!({
                    "type": "function_call_output",
                    "output": output
                })
            })
            .collect::<Vec<_>>();
        input.push(serde_json::json!({
            "type": "function_call_output",
            "output": {"answer": "current-answer"}
        }));
        let topology = extract_pre_action_multi_source_topology_v1(
            &serde_json::json!({"input": input}),
            "use history_target and answer",
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
    fn repeated_request_mention_is_preserved_as_typed_ambiguity() {
        let topology = extract_pre_action_multi_source_topology_v1(
            &json!({"input":[{
                "type":"function_call_output",
                "output":{"session_id":93139}
            }]}),
            "compare session_id with the previous session_id",
        );

        assert!(matches!(
            topology.extraction_status,
            MultiSourceExtractionStatusV1::Complete
        ));
        topology.validate().expect("typed ambiguity is canonical");
        let witness = topology.role_witnesses.first().expect("role witness");
        assert_eq!(witness.request_reference_ordinal, None);
        assert_eq!(witness.request_reference_ordinal_candidates, vec![0]);
        assert!(topology.relations.iter().any(|edge| {
            edge.relation == MultiSourceRelationKindV1::RequestReferencesRole
                && edge.source_role_id == witness.local_role_id
        }));
        let encoded = serde_json::to_string(&topology).expect("encode");
        assert!(!encoded.contains("session_id"));
        assert!(!encoded.contains("93139"));
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
    fn bounded_extractor_throughput_stays_inside_hot_budget() {
        const BATCH_SIZE: usize = 64;
        const BATCHES: usize = 16;
        let payload = json!({"input":[
            {"type":"function_call_output","output":"{\"left\":7,\"right\":9}"},
            {"type":"function_call_output","output":"{\"status\":\"ready\"}"}
        ]});
        let mut best_micros = u128::MAX;
        for _ in 0..BATCHES {
            let started = Instant::now();
            for _ in 0..BATCH_SIZE {
                let topology =
                    extract_pre_action_multi_source_topology_v1(&payload, "combine 7 and ready");
                assert!(matches!(
                    topology.extraction_status,
                    MultiSourceExtractionStatusV1::Complete
                ));
            }
            best_micros = best_micros.min(started.elapsed().as_micros() / BATCH_SIZE as u128);
        }
        // Shared test runners can preempt a whole batch. The isolated test
        // below owns wall-clock p99; this gate catches intrinsic regressions.
        assert!(
            best_micros <= 250,
            "extractor best throughput {best_micros}us exceeds 250us"
        );
    }

    #[test]
    #[ignore = "requires an isolated wall-clock performance runner"]
    fn bounded_extractor_wall_p99_stays_inside_hot_budget() {
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
        eprintln!("multi_source_extractor_wall_p99_us={p99}");
        assert!(p99 <= 250, "extractor wall p99 {p99}us exceeds 250us");
    }
}
