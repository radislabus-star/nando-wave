use nando_operator_kernel::{
    MULTI_SOURCE_MAX_ROLE_NODES_V1, MultiSourceCardinalityClassV1, MultiSourceContainerClassV1,
    MultiSourceExtractionStatusV1, MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1,
    MultiSourceTemporalClassV1, MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1,
};
use nando_operator_runtime::{
    ObservedJsonScalarRole, ObservedScalarRoleClass, canonical_collection_from_provider_output,
    observed_continuation_handle_role, observed_json_scalar_roles,
};
use serde_json::Value;

const MAX_RELEVANT_OUTPUTS_V1: usize = 8;
const MAX_TURN_OUTPUTS_V2: usize = 64;

type ContainerSignature = (
    MultiSourceTypeClassV1,
    MultiSourceContainerClassV1,
    MultiSourceCardinalityClassV1,
);

struct CollectedRole {
    node: MultiSourceRoleNodeV1,
    value_sha256: String,
    request_positions: Vec<u16>,
    request_reference_ordinal: Option<u16>,
    request_reference_ordinal_candidates: Vec<u16>,
    json_path_sha256: [u8; 32],
    continuation_handle: bool,
}

struct SelectedOutput {
    source_ordinal: u16,
    value: Value,
    scalar_roles: Vec<ObservedJsonScalarRole>,
}

struct ProviderOutput {
    ordinal: usize,
    raw: Value,
    structural: Value,
}

struct OutputMetadata {
    ordinal: usize,
    value: Value,
    roles: Vec<ObservedJsonScalarRole>,
    request_referenced: bool,
    container_signature: Option<ContainerSignature>,
}

impl OutputMetadata {
    fn role_cost(&self) -> usize {
        self.roles
            .len()
            .saturating_add(usize::from(self.container_signature.is_some()))
    }

    fn continuation_roots(&self) -> impl Iterator<Item = &str> {
        self.roles
            .iter()
            .filter(|role| role.role_class == ObservedScalarRoleClass::ContinuationHandle)
            .map(|role| role.value_sha256.as_str())
    }
}

pub(crate) fn extract_pre_action_multi_source_topology_v2(
    payload: &Value,
    request_text: &str,
) -> PreActionMultiSourceTopologyV1 {
    let outputs = select_relevant_outputs(provider_outputs(payload), request_text);
    let outputs = match outputs {
        Ok(outputs) => outputs,
        Err(reason) => return censored(reason),
    };
    let mut collected = Vec::new();
    for (output_index, output) in outputs.iter().enumerate() {
        let latest = output_index + 1 == outputs.len();
        let container = match canonical_collection_from_provider_output(&output.value) {
            Ok(canonical) => match super::container_role::from_output(&canonical) {
                Ok(container) => container,
                Err(reason) => return censored(reason),
            },
            Err(_) => None,
        };
        if let Some(container) = container {
            collected.push(CollectedRole {
                node: MultiSourceRoleNodeV1 {
                    local_role_id: 0,
                    source_ordinal: output.source_ordinal,
                    value_ordinal: 0,
                    type_class: container.type_class,
                    container_class: container.container_class,
                    cardinality_class: container.cardinality_class,
                    temporal_class: if latest {
                        MultiSourceTemporalClassV1::Latest
                    } else {
                        MultiSourceTemporalClassV1::Historical
                    },
                    depth_bucket: 0,
                    structural_flags: 0,
                },
                value_sha256: container.value_sha256,
                request_positions: Vec::new(),
                request_reference_ordinal: None,
                request_reference_ordinal_candidates: Vec::new(),
                json_path_sha256: [0; 32],
                continuation_handle: false,
            });
        }
        for (value_ordinal, role) in output.scalar_roles.iter().enumerate() {
            collected.push(CollectedRole {
                node: role_node(
                    role,
                    output.source_ordinal,
                    u16::try_from(value_ordinal.saturating_add(1)).unwrap_or(u16::MAX),
                    latest,
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
    let continuation_role_ids = collected
        .iter()
        .filter(|role| role.continuation_handle)
        .map(|role| role.node.local_role_id)
        .collect::<Vec<_>>();
    let relations = match super::relations::build(&roles, &continuation_role_ids) {
        Ok(relations) => relations,
        Err(reason) => return censored(reason),
    };
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

fn provider_outputs(payload: &Value) -> Vec<ProviderOutput> {
    let Some(items) = payload.get("input").and_then(Value::as_array) else {
        return Vec::new();
    };
    let turn_start = items
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    let outputs = items[turn_start..]
        .iter()
        .filter(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })
        .filter_map(|item| item.get("output").or_else(|| item.get("content")))
        .collect::<Vec<_>>();
    let latest_ordinal = outputs.len().saturating_sub(1);
    outputs
        .into_iter()
        .enumerate()
        .filter(|(ordinal, _)| *ordinal < MAX_TURN_OUTPUTS_V2 || *ordinal == latest_ordinal)
        .map(|(ordinal, value)| ProviderOutput {
            ordinal,
            raw: value.clone(),
            structural: crate::session_stream::canonical_embedded_session_output(value)
                .unwrap_or_else(|| {
                    value
                        .as_str()
                        .and_then(|text| serde_json::from_str(text).ok())
                        .unwrap_or_else(|| value.clone())
                }),
        })
        .collect()
}

fn select_relevant_outputs(
    outputs: Vec<ProviderOutput>,
    request_text: &str,
) -> Result<Vec<SelectedOutput>, &'static str> {
    let mut metadata = outputs
        .into_iter()
        .map(|output| {
            let mut roles = observed_json_scalar_roles(request_text, &output.structural)?;
            if let Ok(continuation) = observed_continuation_handle_role(&output.raw) {
                roles.push(continuation);
            }
            let container_signature = canonical_collection_from_provider_output(&output.structural)
                .ok()
                .and_then(|canonical| {
                    super::container_role::from_output(&canonical)
                        .ok()
                        .flatten()
                })
                .map(|container| {
                    (
                        container.type_class,
                        container.container_class,
                        container.cardinality_class,
                    )
                });
            Ok::<_, &'static str>(OutputMetadata {
                ordinal: output.ordinal,
                value: output.structural,
                roles,
                request_referenced: false,
                container_signature,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if metadata.is_empty() {
        return Ok(Vec::new());
    }
    let latest_ordinal = metadata.last().map_or(0, |row| row.ordinal);
    let mut references = std::collections::BTreeMap::<u16, Vec<(usize, usize)>>::new();
    for (output_index, output) in metadata.iter().enumerate() {
        for (role_index, role) in output.roles.iter().enumerate() {
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
            .filter(|(output_index, _)| metadata[*output_index].ordinal == latest_ordinal)
            .collect::<Vec<_>>();
        for (output_index, role_index) in matches {
            if !latest.contains(&(*output_index, *role_index)) {
                metadata[*output_index].roles[*role_index]
                    .request_position_candidates
                    .retain(|candidate| candidate != position);
                let candidates =
                    &metadata[*output_index].roles[*role_index].request_position_candidates;
                metadata[*output_index].roles[*role_index].request_position =
                    (candidates.len() == 1).then(|| candidates[0]);
            }
        }
    }
    for output in &mut metadata {
        output.request_referenced = output
            .roles
            .iter()
            .any(|role| !role.request_position_candidates.is_empty());
    }
    if metadata.iter().any(|output| {
        output.request_referenced && output.role_cost() > MULTI_SOURCE_MAX_ROLE_NODES_V1
    }) {
        return Err("referenced_output_role_budget_exceeded");
    }
    let mut selected = metadata
        .iter()
        .filter(|output| output.request_referenced || output.ordinal == latest_ordinal)
        .map(|output| output.ordinal)
        .collect::<std::collections::BTreeSet<_>>();
    let mut selected_roles = metadata
        .iter()
        .filter(|output| selected.contains(&output.ordinal))
        .map(OutputMetadata::role_cost)
        .sum::<usize>();
    if selected.len() > MAX_RELEVANT_OUTPUTS_V1 || selected_roles > MULTI_SOURCE_MAX_ROLE_NODES_V1 {
        return Err("relevant_output_role_budget_exceeded");
    }
    let mut represented_types = metadata
        .iter()
        .filter(|output| selected.contains(&output.ordinal))
        .flat_map(|output| output.roles.iter().map(|role| role.value_type))
        .collect::<std::collections::BTreeSet<_>>();
    let mut represented_continuations = metadata
        .iter()
        .filter(|output| selected.contains(&output.ordinal))
        .flat_map(OutputMetadata::continuation_roots)
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    let mut represented_containers = metadata
        .iter()
        .filter(|output| selected.contains(&output.ordinal))
        .filter_map(|output| output.container_signature)
        .collect::<std::collections::BTreeSet<_>>();
    for output in metadata.iter().rev() {
        let adds_continuation = output
            .continuation_roots()
            .any(|root| !represented_continuations.contains(root));
        if adds_continuation {
            add_output_to_reservoir(
                output,
                &mut selected,
                &mut selected_roles,
                &mut represented_types,
                &mut represented_continuations,
                &mut represented_containers,
            );
        }
    }
    for output in metadata.iter().rev() {
        let adds_type = output
            .roles
            .iter()
            .any(|role| !represented_types.contains(&role.value_type));
        if adds_type {
            add_output_to_reservoir(
                output,
                &mut selected,
                &mut selected_roles,
                &mut represented_types,
                &mut represented_continuations,
                &mut represented_containers,
            );
        }
    }
    for output in metadata.iter().rev() {
        let adds_container = output
            .container_signature
            .is_some_and(|signature| !represented_containers.contains(&signature));
        if adds_container {
            add_output_to_reservoir(
                output,
                &mut selected,
                &mut selected_roles,
                &mut represented_types,
                &mut represented_continuations,
                &mut represented_containers,
            );
        }
    }
    for output in metadata.iter().rev() {
        add_output_to_reservoir(
            output,
            &mut selected,
            &mut selected_roles,
            &mut represented_types,
            &mut represented_continuations,
            &mut represented_containers,
        );
    }
    metadata.retain(|output| selected.contains(&output.ordinal));
    metadata.sort_by_key(|output| output.ordinal);
    metadata
        .into_iter()
        .map(|output| {
            Ok(SelectedOutput {
                source_ordinal: u16::try_from(output.ordinal)
                    .map_err(|_| "source_ordinal_budget")?,
                value: output.value,
                scalar_roles: output.roles,
            })
        })
        .collect()
}

fn add_output_to_reservoir(
    output: &OutputMetadata,
    selected: &mut std::collections::BTreeSet<usize>,
    selected_roles: &mut usize,
    represented_types: &mut std::collections::BTreeSet<nando_operator_kernel::AtomValueType>,
    represented_continuations: &mut std::collections::BTreeSet<String>,
    represented_containers: &mut std::collections::BTreeSet<ContainerSignature>,
) -> bool {
    if selected.len() >= MAX_RELEVANT_OUTPUTS_V1
        || selected.contains(&output.ordinal)
        || selected_roles.saturating_add(output.role_cost()) > MULTI_SOURCE_MAX_ROLE_NODES_V1
    {
        return false;
    }
    selected.insert(output.ordinal);
    *selected_roles = selected_roles.saturating_add(output.role_cost());
    represented_types.extend(output.roles.iter().map(|role| role.value_type));
    represented_continuations.extend(output.continuation_roots().map(str::to_owned));
    if let Some(signature) = output.container_signature {
        represented_containers.insert(signature);
    }
    true
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
            * super::REQUEST_REFERENCED_FLAG_V2,
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

    use nando_operator_kernel::MultiSourceRelationKindV1;
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
        let left_topology = extract_pre_action_multi_source_topology_v2(&left, "combine");
        let right_topology = extract_pre_action_multi_source_topology_v2(&right, "combine");
        assert_eq!(left_topology.roles, right_topology.roles);
        assert_eq!(left_topology.relations, right_topology.relations);
        let bytes = serde_json::to_vec(&left_topology).expect("serialize");
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
        let topology = extract_pre_action_multi_source_topology_v2(&payload, "continue");

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
    fn embedded_custom_output_exposes_typed_scalar_without_retaining_payload() {
        let payload = json!({"input":[{
            "type":"custom_tool_call_output",
            "output":[
                {"type":"input_text","text":""},
                {"type":"input_text","text":"{\"chunk_id\":\"abc\",\"session_id\":60906,\"output\":\"Compiling\"}"}
            ]
        }]});
        let topology = extract_pre_action_multi_source_topology_v2(&payload, "continue");

        topology.validate().expect("valid topology");
        let number = topology
            .roles
            .iter()
            .find(|role| role.type_class == MultiSourceTypeClassV1::Number)
            .expect("typed integer role");
        let witness = topology
            .role_witnesses
            .iter()
            .find(|witness| witness.local_role_id == number.local_role_id)
            .expect("integer witness");
        assert_eq!(
            witness.value_sha256,
            nando_operator_kernel::canonical_json_sha256(&json!(60906)).expect("hash")
        );
        let encoded = serde_json::to_string(&topology).expect("encode");
        assert!(!encoded.contains("session_id"));
        assert!(!encoded.contains("60906"));
        assert!(!encoded.contains("Compiling"));
    }

    #[test]
    fn semantic_handle_survives_the_bounded_output_reservoir() {
        let mut input = vec![json!({
            "type": "custom_tool_call_output",
            "output": "{\"chunk_id\":\"first\",\"session_id\":60906,\"output\":\"running\",\"wall_time_seconds\":1}"
        })];
        input.extend((0..8).map(|index| {
            json!({
                "type": "custom_tool_call_output",
                "output": format!(
                    "{{\"chunk_id\":\"later-{index}\",\"exit_code\":0,\"output\":\"ordinary output {index}\",\"wall_time_seconds\":1}}"
                )
            })
        }));
        let topology = extract_pre_action_multi_source_topology_v2(
            &json!({"input": input}),
            "continue the active command",
        );

        topology.validate().expect("valid topology");
        assert!(topology.grounded_output_count <= MAX_RELEVANT_OUTPUTS_V1 as u16);
        assert!(topology.roles.len() <= MULTI_SOURCE_MAX_ROLE_NODES_V1);
        let continuation = topology
            .relations
            .iter()
            .find(|edge| edge.relation == MultiSourceRelationKindV1::ContinuationHandle)
            .expect("semantic continuation relation");
        let role = topology
            .roles
            .iter()
            .find(|role| role.local_role_id == continuation.source_role_id)
            .expect("semantic continuation role");
        assert_eq!(role.source_ordinal, 0);
        assert_eq!(role.type_class, MultiSourceTypeClassV1::Number);
        let witness = topology
            .role_witnesses
            .iter()
            .find(|witness| witness.local_role_id == role.local_role_id)
            .expect("semantic continuation witness");
        assert_eq!(
            witness.value_sha256,
            nando_operator_kernel::canonical_json_sha256(&json!(60906)).expect("hash")
        );
    }

    #[test]
    fn production_capture_feeds_collection_binding_and_factorizer() {
        use nando_operator_kernel::{
            CollectionProgramStep, ResponseProgram, ValueProjectionFormat,
        };
        use nando_operator_learning::multi_source::{
            BlindThenRevealJoinedTransitionV1, CompletedEffectAtomV1, CompletedEffectFormV1,
            PreActionShapeClassV1, factor_multi_source_row_v1, pre_action_t1_binding_root,
        };

        let output = json!({"items": [{"status": "active"}, {"status": "idle"}]});
        let topology = extract_pre_action_multi_source_topology_v2(
            &json!({"input":[{"type":"function_call_output","output": output}]}),
            "count items",
        );
        topology.validate().expect("production topology");
        let container = topology
            .roles
            .iter()
            .find(|role| role.container_class != MultiSourceContainerClassV1::Scalar)
            .expect("container role");
        let witness = topology
            .role_witnesses
            .iter()
            .find(|witness| witness.local_role_id == container.local_role_id)
            .expect("container witness");
        assert_eq!(
            witness.value_sha256,
            nando_operator_kernel::canonical_json_sha256(&output).expect("output root")
        );
        assert!(topology.relations.iter().any(|edge| {
            edge.relation == MultiSourceRelationKindV1::Contains
                && edge.source_role_id == container.local_role_id
        }));
        assert!(topology.relations.iter().any(|edge| {
            edge.relation == MultiSourceRelationKindV1::SameOutput
                && edge.source_role_id == container.local_role_id
        }));
        let program = ResponseProgram::compose_collection(
            vec![
                CollectionProgramStep::SelectTurnOutput { output_ordinal: 1 },
                CollectionProgramStep::SelectOnlyArrayField,
                CollectionProgramStep::Count,
            ],
            ValueProjectionFormat::PlainText,
            "completed",
        );
        pre_action_t1_binding_root(&program, &topology).expect("production binding");

        let joined = BlindThenRevealJoinedTransitionV1 {
            schema: "test".to_owned(),
            join_root_sha256: nando_operator_kernel::sha256_bytes(b"join"),
            capture_sequence: 1,
            turn_intent_id_sha256: nando_operator_kernel::sha256_bytes(b"turn"),
            request_event_id_sha256: nando_operator_kernel::sha256_bytes(b"request"),
            action_event_id_sha256: nando_operator_kernel::sha256_bytes(b"action"),
            session_lineage_sha256: nando_operator_kernel::sha256_bytes(b"lineage"),
            session_id_sha256: nando_operator_kernel::sha256_bytes(b"session"),
            topology_commitment_root_sha256: nando_operator_kernel::sha256_bytes(b"topology"),
            extractor_root_sha256: nando_operator_kernel::sha256_bytes(b"extractor-v2"),
            extractor_config_root_sha256: nando_operator_kernel::sha256_bytes(
                b"extractor-config-v2",
            ),
            capture_generation_root_sha256: nando_operator_kernel::sha256_bytes(
                b"capture-generation-v2",
            ),
            pre_action_record_root_sha256: nando_operator_kernel::sha256_bytes(b"pre-action"),
            completed_frame_root_sha256: nando_operator_kernel::sha256_bytes(b"frame"),
            physical_action_root_sha256: nando_operator_kernel::sha256_bytes(b"physical"),
            semantic_action_root_sha256: nando_operator_kernel::sha256_bytes(b"semantic"),
            effect_atoms: vec![CompletedEffectAtomV1::ValueProjection],
            verifier_receipt_root_sha256: nando_operator_kernel::sha256_bytes(b"verifier"),
            input_tokens: 1,
            captured_at_unix_ms: 1,
            completed_at_unix_nanos: 2,
            accepted: true,
            topology,
        };
        let factored = factor_multi_source_row_v1(&joined);
        assert_eq!(
            factored.pre_action_shape,
            PreActionShapeClassV1::CollectionPlusScalarMetadata
        );
        assert_eq!(
            factored.completed_effect,
            CompletedEffectFormV1::CollectionTransform
        );
    }

    #[test]
    fn capture_and_runtime_share_collection_canonicalization() {
        let rows = json!([{"status": "active"}, {"status": "idle"}]);
        let forms = [
            json!({"rows": rows.clone()}),
            rows.clone(),
            Value::String(rows.to_string()),
            json!([{"type":"output_text","text":rows.to_string()}]),
        ];
        for form in forms {
            let canonical = canonical_collection_from_provider_output(&form).expect("canonical");
            let topology = extract_pre_action_multi_source_topology_v2(
                &json!({"input":[{"type":"function_call_output","output":form}]}),
                "count rows",
            );
            let container = topology
                .roles
                .iter()
                .find(|role| role.container_class != MultiSourceContainerClassV1::Scalar)
                .expect("container");
            let witness = topology
                .role_witnesses
                .iter()
                .find(|witness| witness.local_role_id == container.local_role_id)
                .expect("witness");
            assert_eq!(
                witness.value_sha256,
                nando_operator_kernel::canonical_json_sha256(&canonical).expect("canonical root")
            );
        }
    }

    #[test]
    fn runtime_rejected_object_never_becomes_a_collection_role() {
        let topology = extract_pre_action_multi_source_topology_v2(
            &json!({"input":[{"type":"function_call_output","output":{
                "first":[{"value":1}],
                "second":[{"value":2}]
            }}]}),
            "inspect",
        );
        assert!(
            topology
                .roles
                .iter()
                .all(|role| role.container_class == MultiSourceContainerClassV1::Scalar)
        );
    }

    #[test]
    fn source_ordinals_match_the_runtime_active_turn() {
        let topology = extract_pre_action_multi_source_topology_v2(
            &json!({"input":[
                {"type":"function_call_output","output":{"old": 9}},
                {"type":"message","role":"user","content":"use current outputs"},
                {"type":"function_call_output","output":{"first": [1]}},
                {"type":"custom_tool_call_output","output":{"second": [2]}}
            ]}),
            "use current outputs",
        );
        topology.validate().expect("active turn topology");
        assert_eq!(topology.grounded_output_count, 2);
        assert!(topology.roles.iter().all(|role| role.source_ordinal <= 1));
        assert!(topology.roles.iter().any(|role| role.source_ordinal == 0));
        assert!(topology.roles.iter().any(|role| role.source_ordinal == 1));
    }

    #[test]
    fn output_over_budget_is_censored_without_partial_graph() {
        let values = (0..=MULTI_SOURCE_MAX_ROLE_NODES_V1).collect::<Vec<_>>();
        let topology = extract_pre_action_multi_source_topology_v2(
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
        let mut input = (0..80)
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
        let topology = extract_pre_action_multi_source_topology_v2(
            &serde_json::json!({"input": input}),
            "use history_target and answer",
        );
        assert!(matches!(
            topology.extraction_status,
            MultiSourceExtractionStatusV1::Complete
        ));
        assert!(topology.roles.len() <= MULTI_SOURCE_MAX_ROLE_NODES_V1);
        assert!(topology.grounded_output_count <= MAX_RELEVANT_OUTPUTS_V1 as u16);
        assert!(topology.roles.iter().any(|role| role.source_ordinal == 80));
        assert!(
            topology
                .roles
                .iter()
                .filter(|role| {
                    role.structural_flags & crate::multi_source_capture::REQUEST_REFERENCED_FLAG_V2
                        != 0
                })
                .count()
                >= 2
        );
    }

    #[test]
    fn repeated_request_mention_is_preserved_as_typed_ambiguity() {
        let topology = extract_pre_action_multi_source_topology_v2(
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
        let witness = topology
            .role_witnesses
            .iter()
            .find(|witness| {
                witness.request_reference_ordinal.is_some()
                    || !witness.request_reference_ordinal_candidates.is_empty()
            })
            .expect("request-referenced role witness");
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
        let topology = extract_pre_action_multi_source_topology_v2(
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
                    extract_pre_action_multi_source_topology_v2(&payload, "combine 7 and ready");
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
                extract_pre_action_multi_source_topology_v2(&payload, "combine 7 and ready");
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
