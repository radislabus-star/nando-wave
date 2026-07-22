//! Independent runtime parity reconstruction for mined packages.

use super::*;

pub(super) fn exact_package_runtime_parity(
    package: &nando_response_actor::ResponsePackage,
    frames: &[RelationFrame],
    registry_revision: u64,
) -> (usize, usize, Vec<Value>) {
    if frames.is_empty() {
        return (0, 0, Vec::new());
    }
    let Some(verifier) = package.verifier.as_ref() else {
        return (frames.len(), frames.len(), Vec::new());
    };
    let actor_program_sha256 = response_actor_program_digest(&package.program).unwrap_or_default();
    let verifier_program_sha256 =
        response_independent_verifier_program_digest(verifier).unwrap_or_default();
    let mut failures = 0_usize;
    let mut receipts = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        let payload = parity_provider_payload(package, frame, index);
        let execution = execute_response(&package.program, "", &payload);
        let independently_verified_output = execution
            .response
            .as_deref()
            .filter(|response| verify_response_independently(verifier, &payload, response).is_ok());
        let passed = relation_frame_routes_to_package(package, frame)
            && execution.status == ResponseExecutionStatus::Executed
            && independently_verified_output.is_some();
        if !passed {
            failures = failures.saturating_add(1);
            continue;
        }
        let Some(evidence_sha256) = canonical_json_sha256(&payload).ok() else {
            failures = failures.saturating_add(1);
            continue;
        };
        let Some(output_sha256) =
            independently_verified_output.and_then(|output| canonical_json_sha256(&output).ok())
        else {
            failures = failures.saturating_add(1);
            continue;
        };
        receipts.push(serde_json::json!({
            "schema": "nando.response-runtime-parity-receipt.v1",
            "package_id": package.package_id,
            "registry_revision": registry_revision,
            "frame_id_sha256": frame.frame_id_sha256,
            "actor_program_sha256": actor_program_sha256,
            "independent_verifier_program_sha256": verifier_program_sha256,
            "evidence_sha256": evidence_sha256,
            "output_sha256": output_sha256,
            "result": "pass",
        }));
    }
    (frames.len(), failures, receipts)
}

pub(super) fn collection_runtime_parity(
    package: &ResponsePackage,
    rows: &[ColdCollectionRow],
    frames: &[RelationFrame],
    registry_revision: u64,
) -> (usize, usize, Vec<Value>) {
    let Some(verifier) = package.verifier.as_ref() else {
        return (frames.len(), frames.len(), Vec::new());
    };
    let by_frame = rows
        .iter()
        .map(|row| (row.frame_id_sha256.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let actor_sha256 = response_actor_program_digest(&package.program).unwrap_or_default();
    let verifier_sha256 =
        response_independent_verifier_program_digest(verifier).unwrap_or_default();
    let mut failures = 0_usize;
    let mut receipts = Vec::new();
    for frame in frames {
        let Some(row) = by_frame.get(frame.frame_id_sha256.as_str()) else {
            failures = failures.saturating_add(1);
            continue;
        };
        let execution = execute_response(&package.program, "", &row.example.provider_payload);
        let passed = relation_frame_routes_to_package(package, frame)
            && execution.status == ResponseExecutionStatus::Executed
            && execution.response.as_deref() == Some(row.example.expected_response.as_str())
            && verify_response_independently(
                verifier,
                &row.example.provider_payload,
                execution.response.as_deref().unwrap_or_default(),
            )
            .is_ok();
        if !passed {
            failures = failures.saturating_add(1);
            continue;
        }
        receipts.push(serde_json::json!({
            "schema": "nando.response-runtime-parity-receipt.v1",
            "package_id": package.package_id,
            "registry_revision": registry_revision,
            "frame_id_sha256": frame.frame_id_sha256,
            "actor_program_sha256": actor_sha256,
            "independent_verifier_program_sha256": verifier_sha256,
            "evidence_sha256": canonical_json_sha256(&row.example.provider_payload).unwrap_or_default(),
            "output_sha256": execution.response.as_ref().and_then(|output| canonical_json_sha256(output).ok()).unwrap_or_default(),
            "result": "pass",
        }));
    }
    (frames.len(), failures, receipts)
}

pub(super) fn parity_provider_payload(
    package: &ResponsePackage,
    frame: &RelationFrame,
    index: usize,
) -> Value {
    let cardinality = |role: &str| {
        frame
            .atoms
            .iter()
            .find_map(|atom| match atom {
                RelationAtom::Cardinality {
                    role: atom_role,
                    count,
                } if atom_role == role => Some(*count as usize),
                _ => None,
            })
            .unwrap_or(0)
    };
    let calls = cardinality("turn_call_count_band").max(1);
    let projection_selector = match &package.program.operation {
        ResponseOperation::ProjectSelectedValue { selector, .. } => Some(selector),
        _ => None,
    };
    let status_selector = match &package.program.operation {
        ResponseOperation::ProjectStatus { selector, .. } => Some(selector),
        _ => None,
    };
    let source_value = frame.atoms.iter().find_map(|atom| match atom {
        RelationAtom::TypedSlot {
            value_type,
            source: AtomSource::Observation,
            ..
        } if projection_selector.is_some()
            || status_selector.is_some()
            || response_program_external_verifier_schema(&package.program)
                == Some(SOURCE_VALUE_EXTERNAL_VERIFIER_SCHEMA) =>
        {
            Some(match value_type {
                AtomValueType::Identifier => Value::String(format!("parity-{index}")),
                AtomValueType::String => Value::String(format!("parity value {index}")),
                AtomValueType::Integer => Value::from(index.saturating_add(100)),
                AtomValueType::Boolean => Value::Bool(index.is_multiple_of(2)),
                AtomValueType::Collection => Value::Null,
            })
        }
        _ => None,
    });
    let custom_tool = response_program_external_verifier_schema(&package.program)
        == Some(nando_response_actor::CUSTOM_TOOL_EXTERNAL_VERIFIER_SCHEMA);
    let outputs = cardinality("turn_output_count_band")
        .max(usize::from(source_value.is_some() || custom_tool));
    let pending = cardinality("turn_pending_count_band").min(outputs);
    let messages = cardinality("turn_message_count_band");
    let shapes = cardinality("turn_call_shape_count_band").max(1);
    let observation_call_shape = frame
        .atoms
        .iter()
        .find_map(|atom| match atom {
            RelationAtom::ObservationCallShape { value } => Some(value.as_str()),
            _ => None,
        })
        .unwrap_or("custom_tool_call");
    let request_content = if matches!(
        projection_selector,
        Some(ResponseValueSelector::RequestLastToken)
    ) {
        source_value.as_ref().map_or_else(
            || "runtime parity".to_owned(),
            |value| {
                format!(
                    "runtime parity {}",
                    value
                        .as_str()
                        .map_or_else(|| value.to_string(), str::to_owned)
                )
            },
        )
    } else if matches!(
        projection_selector,
        Some(ResponseValueSelector::RequestUniqueLiteral)
    ) {
        source_value.as_ref().map_or_else(
            || "runtime parity".to_owned(),
            |value| {
                format!(
                    "runtime parity '{}'",
                    value
                        .as_str()
                        .map_or_else(|| value.to_string(), str::to_owned)
                )
            },
        )
    } else if let Some(ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
        ordinal, ..
    }) = projection_selector.or(status_selector)
    {
        (0..=*ordinal).map(|index| format!("role_{index}")).fold(
            "runtime parity".to_owned(),
            |mut request, role| {
                request.push(' ');
                request.push_str(&role);
                request
            },
        )
    } else if matches!(
        projection_selector.or(status_selector),
        Some(ResponseValueSelector::RequestReferencedJsonField { .. })
    ) {
        "runtime parity selected".to_owned()
    } else {
        "runtime parity".to_owned()
    };
    let mut input = vec![serde_json::json!({
        "type": "message",
        "role": "user",
        "content": request_content,
    })];
    input.extend((0..messages).map(|_| {
        serde_json::json!({
            "type": "message",
            "role": "assistant",
            "content": "progress",
        })
    }));
    input.extend((0..calls).map(|call| {
        if observation_call_shape == "function_call" {
            serde_json::json!({
                "type": "function_call",
                "name": format!("shape-{}", call % shapes),
                "call_id": format!("parity-{index}-{call}"),
                "arguments": "{}",
            })
        } else {
            serde_json::json!({
                "type": "custom_tool_call",
                "name": format!("shape-{}", call % shapes),
                "call_id": format!("parity-{index}-{call}"),
                "input": "run",
            })
        }
    }));
    input.extend((0..outputs).map(|output| {
        let is_last = output + 1 == outputs;
        let is_pending = !custom_tool
            && projection_selector.is_none()
            && source_value.is_none()
            && (is_last || output + 1 < pending);
        let output_text = if is_last {
            source_value.as_ref().map(|value| {
                serde_json::to_string(&serde_json::json!({"value": value})).unwrap_or_default()
            })
        } else {
            None
        };
        let output_value = if let (true, Some(selector), Some(value)) =
            (is_last, projection_selector, source_value.as_ref())
        {
            parity_projection_output(selector, value)
        } else if let (true, Some(selector), Some(value)) =
            (is_last, status_selector, source_value.as_ref())
        {
            Value::String(parity_provider_output(selector, value))
        } else if custom_tool && is_last {
            serde_json::json!([{
                "type": "text",
                "text": format!("SESSION_ID={}", index.saturating_add(100)),
            }])
        } else if is_pending {
            Value::String(format!(
                "Script running with cell ID parity-{index}-{output}\n"
            ))
        } else {
            Value::String(output_text.unwrap_or_else(|| "completed".to_owned()))
        };
        serde_json::json!({
            "type": if observation_call_shape == "function_call" {
                "function_call_output"
            } else {
                "custom_tool_call_output"
            },
            "call_id": format!("parity-{index}-{}", output.min(calls.saturating_sub(1))),
            "output": output_value,
        })
    }));
    serde_json::json!({"input": input})
}

pub(super) fn parity_provider_output(selector: &ResponseValueSelector, value: &Value) -> String {
    match selector {
        ResponseValueSelector::ContinuationHandle { .. } => {
            let value = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            format!("Script running with cell ID {value}")
        }
        ResponseValueSelector::UniqueScalar { .. }
        | ResponseValueSelector::UniqueTurnScalar { .. } => value.to_string(),
        ResponseValueSelector::ContentLinePrefix { prefix, .. } => {
            format!("{prefix}{value}")
        }
        ResponseValueSelector::JsonField { field, .. }
        | ResponseValueSelector::UniqueTurnJsonField { field, .. }
        | ResponseValueSelector::UniqueActiveTurnJsonField { field, .. } => {
            let mut object = serde_json::Map::new();
            object.insert(field.clone(), value.clone());
            serde_json::to_string(&Value::Object(object)).unwrap_or_else(|_| "{}".to_owned())
        }
        ResponseValueSelector::RequestReferencedJsonField { .. } => {
            serde_json::json!({"selected": value}).to_string()
        }
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        } => parity_request_referenced_ordinal_output(*ordinal, *value_type, value),
        ResponseValueSelector::JsonScalarOrdinal {
            ordinal,
            value_type,
        } => parity_scalar_ordinal_output(*ordinal, *value_type, value),
        ResponseValueSelector::TurnOutputScalarOrdinal {
            scalar_ordinal,
            value_type,
            ..
        }
        | ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal,
            value_type,
        } => parity_scalar_ordinal_output(*scalar_ordinal, *value_type, value),
        ResponseValueSelector::LatestTurnOutputScalarFromEnd {
            reverse_ordinal,
            value_type,
        } => parity_scalar_reverse_ordinal_output(*reverse_ordinal, *value_type, value),
        ResponseValueSelector::TurnOutputLine { line_index, .. }
        | ResponseValueSelector::LatestTurnOutputLine { line_index, .. } => {
            let value = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            std::iter::repeat_n("parity line", usize::from(*line_index))
                .chain(std::iter::once(value.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        }
        ResponseValueSelector::CommandOutputBody => {
            let value = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            format!("Script completed\nOutput:\n{value}")
        }
        ResponseValueSelector::RequestLastToken => value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_owned),
        ResponseValueSelector::RequestUniqueLiteral => value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_owned),
    }
}

pub(super) fn parity_projection_output(
    selector: &nando_response_actor::ResponseValueSelector,
    value: &Value,
) -> Value {
    match selector {
        nando_response_actor::ResponseValueSelector::ContinuationHandle { .. } => {
            let value = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            Value::String(format!("Script running with cell ID {value}"))
        }
        nando_response_actor::ResponseValueSelector::UniqueScalar { .. }
        | nando_response_actor::ResponseValueSelector::UniqueTurnScalar { .. } => {
            Value::String(serde_json::to_string(value).unwrap_or_else(|_| "null".to_owned()))
        }
        nando_response_actor::ResponseValueSelector::ContentLinePrefix { prefix, .. } => {
            let value = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            Value::String(format!("{prefix}{value}"))
        }
        nando_response_actor::ResponseValueSelector::JsonField { field, .. }
        | nando_response_actor::ResponseValueSelector::UniqueTurnJsonField { field, .. }
        | nando_response_actor::ResponseValueSelector::UniqueActiveTurnJsonField {
            field, ..
        } => {
            let mut object = serde_json::Map::new();
            object.insert(field.clone(), value.clone());
            Value::String(
                serde_json::to_string(&Value::Object(object)).unwrap_or_else(|_| "null".to_owned()),
            )
        }
        nando_response_actor::ResponseValueSelector::RequestReferencedJsonField { .. } => {
            Value::String(serde_json::json!({"selected": value}).to_string())
        }
        nando_response_actor::ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        } => Value::String(parity_request_referenced_ordinal_output(
            *ordinal,
            *value_type,
            value,
        )),
        nando_response_actor::ResponseValueSelector::JsonScalarOrdinal {
            ordinal,
            value_type,
        } => Value::String(parity_scalar_ordinal_output(*ordinal, *value_type, value)),
        nando_response_actor::ResponseValueSelector::TurnOutputScalarOrdinal {
            scalar_ordinal,
            value_type,
            ..
        }
        | nando_response_actor::ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal,
            value_type,
        } => Value::String(parity_scalar_ordinal_output(
            *scalar_ordinal,
            *value_type,
            value,
        )),
        nando_response_actor::ResponseValueSelector::LatestTurnOutputScalarFromEnd {
            reverse_ordinal,
            value_type,
        } => Value::String(parity_scalar_reverse_ordinal_output(
            *reverse_ordinal,
            *value_type,
            value,
        )),
        nando_response_actor::ResponseValueSelector::TurnOutputLine { line_index, .. }
        | nando_response_actor::ResponseValueSelector::LatestTurnOutputLine {
            line_index, ..
        } => {
            let value = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            Value::String(
                std::iter::repeat_n("parity line", usize::from(*line_index))
                    .chain(std::iter::once(value.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        }
        nando_response_actor::ResponseValueSelector::CommandOutputBody => {
            let value = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            Value::String(format!("Script completed\nOutput:\n{value}"))
        }
        nando_response_actor::ResponseValueSelector::RequestLastToken => Value::String(
            value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned),
        ),
        nando_response_actor::ResponseValueSelector::RequestUniqueLiteral => Value::String(
            value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned),
        ),
    }
}

pub(super) fn parity_request_referenced_ordinal_output(
    ordinal: u16,
    value_type: AtomValueType,
    value: &Value,
) -> String {
    let filler = match value_type {
        AtomValueType::String | AtomValueType::Identifier => Value::String(String::new()),
        AtomValueType::Integer => Value::from(0),
        AtomValueType::Boolean => Value::Bool(false),
        AtomValueType::Collection => Value::Null,
    };
    let mut object = serde_json::Map::new();
    for index in 0..=ordinal {
        object.insert(
            format!("role_{index}"),
            if index == ordinal {
                value.clone()
            } else {
                filler.clone()
            },
        );
    }
    serde_json::to_string(&Value::Object(object)).unwrap_or_else(|_| "{}".to_owned())
}

pub(super) fn parity_scalar_ordinal_output(
    ordinal: u16,
    value_type: AtomValueType,
    value: &Value,
) -> String {
    let filler = match value_type {
        AtomValueType::String | AtomValueType::Identifier => Value::String(String::new()),
        AtomValueType::Integer => Value::from(0),
        AtomValueType::Boolean => Value::Bool(false),
        AtomValueType::Collection => Value::Null,
    };
    let mut values = vec![filler; usize::from(ordinal)];
    values.push(value.clone());
    serde_json::json!({"values": values}).to_string()
}

pub(super) fn parity_scalar_reverse_ordinal_output(
    reverse_ordinal: u16,
    value_type: AtomValueType,
    value: &Value,
) -> String {
    let filler = match value_type {
        AtomValueType::String | AtomValueType::Identifier => Value::String(String::new()),
        AtomValueType::Integer => Value::from(0),
        AtomValueType::Boolean => Value::Bool(false),
        AtomValueType::Collection => Value::Null,
    };
    let mut values = vec![value.clone()];
    values.extend(std::iter::repeat_n(filler, usize::from(reverse_ordinal)));
    serde_json::to_string(&values).unwrap_or_default()
}

pub(super) fn exact_package_hard_negative_accepts(
    package: &nando_response_actor::ResponsePackage,
) -> usize {
    let continuation_outputs = vec![
        Value::String("completed successfully".to_owned()),
        Value::String(
            "Script running with cell ID first\nScript running with cell ID second\n".to_owned(),
        ),
        Value::String("Script running with cell ID !!!\n".to_owned()),
        Value::String(String::new()),
    ];
    let source_value_outputs = vec![
        Value::String("{}".to_owned()),
        Value::String("[]".to_owned()),
        Value::String("{\"left\":1,\"right\":2}".to_owned()),
        Value::String("null".to_owned()),
        Value::String("1.5".to_owned()),
        Value::String("ambiguous\nmultiline".to_owned()),
    ];
    let status_outputs = vec![
        Value::String("{\"other\":0}".to_owned()),
        Value::String("{\"exit_code\":1000001}".to_owned()),
        Value::String("{\"exit_code\":-1}".to_owned()),
        Value::String("{\"exit_code\":true}".to_owned()),
        Value::String("{\"exit_code\":0}\n{\"exit_code\":1}".to_owned()),
        serde_json::json!([{"type":"unknown_text","text":"{\"exit_code\":0}"}]),
    ];
    let custom_tool_outputs = vec![
        serde_json::json!([]),
        serde_json::json!([{"type":"text","text":"completed"}]),
        serde_json::json!([
            {"type":"text","text":"SESSION_ID=1"},
            {"type":"text","text":"SESSION_ID=2"}
        ]),
        serde_json::json!([{"type":"text","text":"SESSION_ID=invalid integer"}]),
    ];
    let collection_outputs = vec![
        Value::String("{}".to_owned()),
        Value::String("{\"left\":[],\"right\":[]}".to_owned()),
        Value::String("{\"rows\":[]}".to_owned()),
        Value::String("{\"rows\":[{\"left\":\"keep\",\"right\":\"keep\",\"value\":1}]}".to_owned()),
        Value::String("{\"rows\":[{\"kind\":\"keep\"},{\"other\":\"keep\"}]}".to_owned()),
    ];
    let schema = response_program_external_verifier_schema(&package.program);
    let outputs = if matches!(
        schema,
        Some(SOURCE_VALUE_EXTERNAL_VERIFIER_SCHEMA | VALUE_PROJECTION_EXTERNAL_VERIFIER_SCHEMA)
    ) {
        source_value_outputs
    } else if schema == Some(COLLECTION_EXTERNAL_VERIFIER_SCHEMA) {
        collection_outputs
    } else if schema == Some("status_projection_external_evidence.v1") {
        status_outputs
    } else if schema == Some(nando_response_actor::CUSTOM_TOOL_EXTERNAL_VERIFIER_SCHEMA) {
        custom_tool_outputs
    } else {
        continuation_outputs
    };
    outputs
        .into_iter()
        .filter(|output| {
            let payload = serde_json::json!({
                "input": [{
                    "type": "function_call_output",
                    "output": output,
                }]
            });
            let execution = execute_response(&package.program, "", &payload);
            execution.response.as_deref().is_some_and(|response| {
                package.verifier.as_ref().is_some_and(|verifier| {
                    verify_response_independently(verifier, &payload, response).is_ok()
                })
            })
        })
        .count()
}
