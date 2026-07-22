use nando_operator_kernel::{
    RuntimeContextExtractionVerdictV3, RuntimeProjectionV3, sha256_bytes,
    validate_extraction_receipt_v3,
};
use serde_json::{Value, json};

use super::*;

fn ready<'a>(
    request_text: &str,
    payload: &'a Value,
    budget: RuntimeContextBudgetV3,
) -> CanonicalRuntimeRequestV3<'a> {
    let request_sha256 = sha256_bytes(
        serde_json::to_vec(payload)
            .expect("request fixture serializes")
            .as_slice(),
    );
    let outcome = extract_canonical_runtime_request_v3(
        &request_sha256,
        request_text,
        RuntimeProjectionV3::Responses,
        payload,
        budget,
    )
    .expect("runtime context extraction");
    assert_eq!(
        outcome.receipt().verdict,
        RuntimeContextExtractionVerdictV3::Complete
    );
    assert_eq!(outcome.receipt().extraction_count, 1);
    assert_eq!(outcome.receipt().teacher_or_action_fields_consumed, 0);
    assert_eq!(outcome.receipt().raw_payloads_persisted, 0);
    assert!(!outcome.receipt().execution_authority);
    validate_extraction_receipt_v3(outcome.receipt()).expect("complete extraction receipt");
    outcome.into_context().expect("ready runtime context")
}

fn request_payload(capability_name: &str, output: Value) -> Value {
    json!({
        "model": "ignored-model-surface",
        "tools": [{
            "type": "function",
            "name": capability_name,
            "parameters": {
                "type": "object",
                "properties": {"handle": {"type": "string"}},
                "required": ["handle"]
            }
        }],
        "input": [
            {"type": "message", "role": "user", "content": "continue CellA17"},
            {"type": "function_call", "name": capability_name, "call_id": "call-1"},
            {"type": "function_call_output", "call_id": "call-1", "output": output}
        ]
    })
}

#[test]
fn direct_wrapped_renamed_and_reordered_surfaces_share_one_canonical_view() {
    let direct = request_payload("a", json!({"handle": "CellA17", "state": "running"}));
    let wrapped = request_payload(
        "b",
        json!({"transport": {"renamed_state": "running", "renamed_handle": "CellA17"}}),
    );
    let direct = ready(
        "continue CellA17",
        &direct,
        RuntimeContextBudgetV3::default(),
    );
    let wrapped = ready(
        "continue CellA17",
        &wrapped,
        RuntimeContextBudgetV3::default(),
    );

    assert_eq!(direct.view(), wrapped.view());
    assert_eq!(direct.capability_bindings()[0].physical_symbol, "a");
    assert_eq!(wrapped.capability_bindings()[0].physical_symbol, "b");
}

#[test]
fn teacher_and_post_action_fields_cannot_change_the_runtime_view() {
    let base = request_payload("a", json!({"handle": "CellA17"}));
    let mut polluted = base.clone();
    let object = polluted.as_object_mut().expect("object request");
    object.insert(
        "teacher_response".to_owned(),
        json!("secret-teacher-target-991"),
    );
    object.insert("expected_action".to_owned(), json!("secret-action-992"));
    object.insert(
        "state_after".to_owned(),
        json!({"target_patch": "secret-993"}),
    );

    let base = ready("continue CellA17", &base, RuntimeContextBudgetV3::default());
    let polluted = ready(
        "continue CellA17",
        &polluted,
        RuntimeContextBudgetV3::default(),
    );
    assert_eq!(base.view(), polluted.view());
    let durable =
        serde_json::to_string(polluted.view()).expect("durable context metadata serializes");
    assert!(!durable.contains("secret-teacher"));
    assert!(!durable.contains("secret-action"));
    assert!(!durable.contains("secret-993"));
}

#[test]
fn bounded_extraction_abstains_instead_of_truncating_to_an_earlier_role() {
    let payload = json!({
        "input": (0..64)
            .map(|index| json!({"opaque": format!("candidate-{index:04}")}))
            .collect::<Vec<_>>()
    });
    let request_sha256 = sha256_bytes(
        serde_json::to_vec(&payload)
            .expect("request fixture serializes")
            .as_slice(),
    );
    let outcome = extract_canonical_runtime_request_v3(
        &request_sha256,
        "choose candidate-0063",
        RuntimeProjectionV3::Responses,
        &payload,
        RuntimeContextBudgetV3 {
            max_json_nodes: 8,
            max_text_bytes: 256,
            max_recent_events: 4,
            max_role_candidates: 4,
            max_relations: 8,
            max_capabilities: 4,
        },
    )
    .expect("bounded extraction outcome");
    assert!(outcome.context().is_none());
    let receipt = outcome.receipt();
    assert_eq!(
        receipt.verdict,
        RuntimeContextExtractionVerdictV3::AbstainBudgetExhausted
    );
    assert!(receipt.request_view_sha256.is_none());
    assert_eq!(receipt.extraction_count, 1);
    validate_extraction_receipt_v3(receipt).expect("bounded abstain receipt");
}

#[test]
fn context_keeps_the_original_payload_borrowed_and_receipt_keeps_no_raw_values() {
    let payload = request_payload("a", json!({"handle": "BorrowedValue77"}));
    let context = ready(
        "continue BorrowedValue77",
        &payload,
        RuntimeContextBudgetV3::default(),
    );
    assert!(std::ptr::eq(context.provider_payload(), &payload));
    let durable = serde_json::to_string(context.view()).expect("durable metadata serializes");
    assert!(!durable.contains("BorrowedValue77"));
}

#[test]
fn wide_event_exhausts_inside_the_single_bounded_walker() {
    let payload = json!({
        "input": [{
            "type": "function_call_output",
            "output": (0..1_000)
                .map(|index| json!({"opaque": format!("wide-{index:04}")}))
                .collect::<Vec<_>>()
        }]
    });
    let request_sha256 = sha256_bytes(
        serde_json::to_vec(&payload)
            .expect("wide request serializes")
            .as_slice(),
    );
    let outcome = extract_canonical_runtime_request_v3(
        &request_sha256,
        "continue wide-0999",
        RuntimeProjectionV3::Responses,
        &payload,
        RuntimeContextBudgetV3 {
            max_json_nodes: 16,
            max_text_bytes: 256,
            max_recent_events: 4,
            max_role_candidates: 8,
            max_relations: 8,
            max_capabilities: 4,
        },
    )
    .expect("wide extraction outcome");

    assert!(outcome.context().is_none());
    assert_eq!(outcome.receipt().json_nodes_visited, 16);
    assert_eq!(
        outcome.receipt().verdict,
        RuntimeContextExtractionVerdictV3::AbstainBudgetExhausted
    );
}

#[test]
fn overfull_capability_surface_abstains_without_scanning_an_unbounded_registry() {
    let payload = json!({
        "tools": (0..1_000)
            .map(|index| json!({
                "type": "function",
                "name": format!("capability_{index:04}"),
                "parameters": {"type": "object", "properties": {}}
            }))
            .collect::<Vec<_>>(),
        "input": []
    });
    let request_sha256 = sha256_bytes(
        serde_json::to_vec(&payload)
            .expect("capability request serializes")
            .as_slice(),
    );
    let outcome = extract_canonical_runtime_request_v3(
        &request_sha256,
        "continue",
        RuntimeProjectionV3::Responses,
        &payload,
        RuntimeContextBudgetV3 {
            max_capabilities: 4,
            ..RuntimeContextBudgetV3::default()
        },
    )
    .expect("capability extraction outcome");

    assert!(outcome.context().is_none());
    assert_eq!(outcome.receipt().advertised_capabilities, 4);
    assert_eq!(
        outcome.receipt().verdict,
        RuntimeContextExtractionVerdictV3::AbstainBudgetExhausted
    );
}

#[test]
fn oversized_request_text_abstains_before_payload_extraction() {
    let payload = json!({"input": []});
    let request_sha256 = sha256_bytes(
        serde_json::to_vec(&payload)
            .expect("request fixture serializes")
            .as_slice(),
    );
    let request_text = "x".repeat(RUNTIME_CONTEXT_MAX_REQUEST_TEXT_BYTES_V3 + 1);
    let outcome = extract_canonical_runtime_request_v3(
        &request_sha256,
        &request_text,
        RuntimeProjectionV3::Responses,
        &payload,
        RuntimeContextBudgetV3::default(),
    )
    .expect("oversized request outcome");

    assert!(outcome.context().is_none());
    assert_eq!(outcome.receipt().json_nodes_visited, 0);
    assert_eq!(outcome.receipt().text_bytes_visited, 0);
    assert_eq!(
        outcome.receipt().verdict,
        RuntimeContextExtractionVerdictV3::AbstainBudgetExhausted
    );
}
