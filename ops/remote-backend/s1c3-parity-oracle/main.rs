use std::{env, fs};

use nando_response_actor::{ResponseExecutor, RoutedResponseExecution};
use serde_json::{Value, json};

fn main() {
    let args = env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 3, "registry and admission paths required");
    let registry = fs::read(&args[1]).expect("registry");
    let admission = fs::read(&args[2]).expect("admission");
    let admission_value: Value = serde_json::from_slice(&admission).expect("admission json");
    let project = admission_value["project_id"].as_str().expect("project");
    let generated = admission_value["generated_at_unix"].as_u64().expect("generated");
    let expires = admission_value["expires_at_unix"].as_u64().expect("expires");
    let gate = admission_value["response_authority"]["gate_build_sha256"]
        .as_str()
        .expect("gate");
    let runtime = admission_value["response_authority"]["runtime_build_sha256"]
        .as_str()
        .expect("runtime");
    let executor = ResponseExecutor::from_authorized_json(
        &registry,
        &admission,
        project,
        gate,
        runtime,
        generated,
        expires.saturating_sub(generated).max(1),
    )
    .expect("authorized executor");

    let cases = vec![
        ("empty", "ordinary question", json!({})),
        ("status_string", "report status", json!({"input":[{"type":"function_call_output","call_id":"c1","output":"{\"status\":\"ready\"}"}]})),
        ("status_zero", "report status", json!({"input":[{"type":"function_call_output","call_id":"c1","output":{"opaque_code":0}}]})),
        ("status_nonzero", "report status", json!({"input":[{"type":"function_call_output","call_id":"c1","output":{"opaque_code":7}}]})),
        ("yielded", "continue CellA17", json!({"input":[{"type":"function_call","name":"exec_command","call_id":"c1"},{"type":"function_call_output","call_id":"c1","output":{"cell_id":"CellA17","status":"running"}}]})),
        ("two_outputs", "summarize both", json!({"input":[{"type":"function_call_output","call_id":"c1","output":{"value":17}},{"type":"function_call_output","call_id":"c2","output":{"value":19}}]})),
        ("collection", "count passing rows", json!({"input":[{"type":"function_call_output","call_id":"c1","output":{"rows":[{"ok":true},{"ok":false},{"ok":true}]}}]})),
        ("tool_failure", "report result", json!({"input":[{"type":"function_call_output","call_id":"c1","output":{"exit_code":1,"status":"failed"}}]})),
    ];
    for (label, request, payload) in cases {
        emit(label, "authorized", executor.execute(request, &payload));
        emit(label, "shadow", executor.execute_shadow(request, &payload));
    }
}

fn emit(label: &str, route: &str, execution: RoutedResponseExecution) {
    println!(
        "{}",
        serde_json::to_string(&json!({
            "label": label,
            "route": route,
            "status": format!("{:?}", execution.status),
            "reason": execution.reason,
            "response": execution.response,
            "package_id": execution.package_id,
            "verification_receipt_id": execution.verification_receipt_id,
            "verifier_schema": execution.verifier_schema,
            "phase_candidates": execution.phase_candidates,
            "exact_actor_checks": execution.exact_actor_checks,
            "phase_margin_micro": execution.phase_margin_micro,
        }))
        .expect("oracle row")
    );
}
