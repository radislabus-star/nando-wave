use nando_transition_actor::{
    LiveExecutionResult, TransitionPackage, TransitionRequest, execute_live_request,
};
use serde_json::{Value, json};

const PACKAGE_JSON: &str =
    include_str!("../../../ops/phase-center-test-server/packages/rsmod-portable-v1.json");

fn package() -> Result<TransitionPackage, serde_json::Error> {
    serde_json::from_str(PACKAGE_JSON)
}

fn execute(
    operator_id: &str,
    adapter_id: &str,
    before: Value,
    action: Value,
) -> Result<LiveExecutionResult, serde_json::Error> {
    Ok(execute_live_request(
        &package()?,
        &TransitionRequest {
            schema: "nando.transition-request.v1".to_owned(),
            package_id: "rsmod-portable-v1-20260710".to_owned(),
            operator_id: operator_id.to_owned(),
            adapter_id: adapter_id.to_owned(),
            before,
            action,
        },
    ))
}

#[test]
fn package_executes_all_four_operator_families() -> Result<(), serde_json::Error> {
    let cases = [
        execute(
            "set_field",
            "task_map",
            json!({"board":{"items":{"t1":{"state":"open","assignee":"a","points":1}}}}),
            json!({"cmd":"complete","item":"t1","to_state":"done"}),
        )?,
        execute(
            "increment_field",
            "crm_list",
            json!({"workspace":{"leads":[{"lead_id":"l1","stage":"new","rep":"a","touches":2}]}}),
            json!({"command":"touch","lead":"l1","by":3}),
        )?,
        execute(
            "append_record",
            "warehouse_columns",
            json!({"inventory":{"sku":["s1"],"condition":["ok"],"zone":["a"],"units":[2]}}),
            json!({"operation":"register","article":{"sku":"s2","condition":"new","zone":"b","units":4}}),
        )?,
        execute(
            "delete_record",
            "incident_list",
            json!({"payload":{"incidents":[{"incident_no":"i1","phase":"open","responder":"a","attempts":1}]}}),
            json!({"event":"close_incident","ref":"i1"}),
        )?,
    ];
    for result in cases {
        assert!(result.local_accept, "{}", result.reason);
        assert!(result.verifier_ok);
        assert_eq!(result.false_accepts, 0);
        assert!(result.response.is_some());
    }
    Ok(())
}

#[test]
fn ambiguous_target_and_unknown_adapter_abstain() -> Result<(), serde_json::Error> {
    let ambiguous = execute(
        "set_field",
        "crm_list",
        json!({"workspace":{"leads":[
            {"lead_id":"same","stage":"new","rep":"a","touches":1},
            {"lead_id":"same","stage":"new","rep":"b","touches":2}
        ]}}),
        json!({"command":"advance","lead":"same","stage":"won"}),
    )?;
    assert!(!ambiguous.local_accept);
    assert!(ambiguous.reason.contains("target_ambiguous"));

    let unknown = execute("set_field", "unknown", json!({}), json!({}))?;
    assert!(!unknown.local_accept);
    assert_eq!(unknown.reason, "adapter_not_registered");
    Ok(())
}
