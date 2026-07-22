use serde_json::json;

use super::*;

#[test]
fn continuation_role_ignores_unrelated_scalars() {
    let payload = json!({
        "input": [{
            "type": "function_call_output",
            "output": "progress 17\nScript running with cell ID handle-42\nremaining 99"
        }]
    });
    let program = ResponseProgram::custom_tool_call_from_roles(
        "exec",
        "write_stdin",
        ResponseValueSelector::ContinuationHandle {
            value_type: AtomValueType::Identifier,
        },
        vec![ResponseArgument::Role {
            name: "session_id".to_owned(),
            role: SemanticRole::ContinuationHandle,
            value_type: Some(AtomValueType::Identifier),
        }],
        CustomToolResultProjection::JsonStringifyResult,
    );

    let execution = execute_response_unverified(&program, "", &payload);
    assert_eq!(execution.status, ResponseExecutionStatus::Executed);
    let response: Value =
        serde_json::from_str(execution.response.as_deref().expect("custom tool response"))
            .expect("response JSON");
    let source = response["input"].as_str().expect("custom tool source");
    assert!(source.contains("\"session_id\":\"handle-42\""));
    assert!(!source.contains("\"session_id\":17"));
    assert!(!source.contains("\"session_id\":99"));
}

#[test]
fn observation_layout_ignores_provider_history() {
    let short = json!({
        "input": [{
            "type": "function_call_output",
            "output": "Process running with session ID handle-42"
        }]
    });
    let long = json!({
        "input": [
            {"type": "message", "content": "unrelated history"},
            {"type": "function_call", "name": "query", "arguments": "{}"},
            {
                "type": "function_call_output",
                "output": "Process running with session ID handle-99"
            }
        ]
    });

    assert_eq!(
        actor_structural_layout_sha256(&short).expect("short observation layout"),
        actor_structural_layout_sha256(&long).expect("long observation layout")
    );
}

#[test]
fn reverse_ordinal_reaches_continuation_after_large_scalar_prefix() {
    let mut values = (0_u64..100).map(Value::from).collect::<Vec<_>>();
    values.push(Value::from(9_999_u64));
    let payload = json!({
        "input": [{
            "type": "custom_tool_call_output",
            "output": serde_json::to_string(&values).expect("bounded output")
        }]
    });

    let selected = latest_turn_output_scalar_from_end(&payload, 0, AtomValueType::Integer)
        .expect("tail scalar beyond legacy 64-value budget");
    assert_eq!(selected.value, Value::from(9_999_u64));
}

#[test]
fn teacher_value_compiles_to_name_free_structural_ordinals() {
    let mut values = (0_u64..100).map(Value::from).collect::<Vec<_>>();
    values.push(Value::from(9_999_u64));
    let payload = json!({
        "input": [{
            "type": "custom_tool_call_output",
            "output": serde_json::to_string(&values).expect("bounded output")
        }]
    });

    let selectors = structural_output_selectors_for_teacher_value(
        &payload,
        &Value::from(9_999_u64),
        AtomValueType::Integer,
    )
    .expect("teacher value aligns to observed ordinals");
    assert!(selectors.iter().all(|selector| matches!(
        selector,
        ResponseValueSelector::TurnOutputScalarOrdinal { .. }
            | ResponseValueSelector::LatestTurnOutputScalarOrdinal { .. }
            | ResponseValueSelector::LatestTurnOutputScalarFromEnd { .. }
    )));
    assert!(selectors.iter().any(|selector| matches!(
        selector,
        ResponseValueSelector::LatestTurnOutputScalarFromEnd {
            reverse_ordinal: 0,
            ..
        }
    )));
}

#[test]
fn teacher_value_can_bind_an_earlier_turn_output() {
    let payload = json!({
        "input": [
            {
                "type": "function_call_output",
                "output": "{\"candidate\":9999}"
            },
            {
                "type": "function_call_output",
                "output": "{\"noise\":7}"
            }
        ]
    });

    let selectors = structural_output_selectors_for_teacher_value(
        &payload,
        &Value::from(9_999_u64),
        AtomValueType::Integer,
    )
    .expect("teacher value aligns across the active turn");
    assert!(selectors.iter().any(|selector| matches!(
        selector,
        ResponseValueSelector::TurnOutputScalarOrdinal {
            output_ordinal: 1,
            scalar_ordinal: 0,
            ..
        }
    )));
    assert!(!selectors.iter().any(|selector| matches!(
        selector,
        ResponseValueSelector::LatestTurnOutputScalarOrdinal { .. }
            | ResponseValueSelector::LatestTurnOutputScalarFromEnd { .. }
    )));
}

#[test]
fn field_hint_compiles_to_name_free_structural_ordinals() {
    let noise = (0_u64..100).collect::<Vec<_>>();
    let payload = json!({
        "input": [{
            "type": "custom_tool_call_output",
            "output": serde_json::to_string(&json!({
                "first": {"session_id": 111},
                "noise": noise,
                "second": {"session_id": 999}
            }))
            .expect("bounded output")
        }]
    });
    let selectors =
        structural_output_selectors_for_field_hint(&payload, "session_id", AtomValueType::Integer)
            .expect("field hint ordinals");
    assert!(!selectors.is_empty());
    assert!(selectors.iter().all(|selector| matches!(
        selector,
        ResponseValueSelector::LatestTurnOutputScalarOrdinal { .. }
            | ResponseValueSelector::LatestTurnOutputScalarFromEnd { .. }
    )));
    let values = selectors
        .iter()
        .filter_map(|selector| immediate_selected_scalar(&payload, selector).ok())
        .map(|selected| selected.value.to_string())
        .collect::<BTreeSet<_>>();
    assert!(values.contains("111"));
    assert!(values.contains("999"));
}
