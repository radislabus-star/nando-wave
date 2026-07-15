use serde_json::{Value, json};

pub(crate) fn parse_actor_custom_tool_call(response_text: &str) -> Option<Value> {
    serde_json::from_str::<Value>(response_text)
        .ok()
        .filter(|value| {
            value.get("kind").and_then(Value::as_str) == Some("custom_tool_call")
                && value.get("name").and_then(Value::as_str).is_some()
                && value.get("input").and_then(Value::as_str).is_some()
        })
}

pub(crate) fn responses_projection(
    request_hash: &str,
    call: &Value,
    model: &str,
    route: &str,
    input_tokens: u64,
    created_at: u64,
) -> Value {
    let suffix = request_hash.get(..16).unwrap_or(request_hash);
    let name = call.get("name").and_then(Value::as_str).unwrap_or("exec");
    let input = call.get("input").and_then(Value::as_str).unwrap_or("");
    let output_tokens = token_estimate(input);
    json!({
        "id": format!("resp_nando_{suffix}"),
        "object": "response",
        "created_at": created_at,
        "status": "completed",
        "model": model,
        "output": [{
            "id": format!("ctc_nando_{suffix}"),
            "call_id": format!("call_nando_{suffix}"),
            "type": "custom_tool_call",
            "name": name,
            "input": input,
        }],
        "output_text": "",
        "usage": {
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "total_tokens": input_tokens.saturating_add(output_tokens),
        },
        "nando": {
            "api_version": "v2",
            "local_accept": true,
            "route": route,
            "false_accepts": 0,
            "architecture": "wave_router_typed_actor_verifier",
        },
    })
}

pub(crate) fn responses_sse(response: &Value, item: &Value) -> String {
    let item_id = item.get("id").and_then(Value::as_str).unwrap_or("");
    let input = item.get("input").and_then(Value::as_str).unwrap_or("");
    let mut in_progress = response.clone();
    in_progress["status"] = Value::String("in_progress".into());
    in_progress["output"] = Value::Array(Vec::new());
    let mut added = item.clone();
    added["input"] = Value::String(String::new());
    let events = vec![
        ("response.created", json!({"response":in_progress})),
        (
            "response.output_item.added",
            json!({"output_index":0,"item":added}),
        ),
        (
            "response.custom_tool_call_input.delta",
            json!({"item_id":item_id,"output_index":0,"delta":input}),
        ),
        (
            "response.custom_tool_call_input.done",
            json!({"item_id":item_id,"output_index":0,"input":input}),
        ),
        (
            "response.output_item.done",
            json!({"output_index":0,"item":item}),
        ),
        ("response.completed", json!({"response":response})),
    ];
    sse_events(events)
}

fn sse_events(events: Vec<(&str, Value)>) -> String {
    let mut body = String::new();
    for (sequence, (event, mut payload)) in events.into_iter().enumerate() {
        payload["type"] = Value::String(event.into());
        payload["sequence_number"] = Value::from(sequence);
        body.push_str("event: ");
        body.push_str(event);
        body.push_str("\ndata: ");
        body.push_str(&serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into()));
        body.push_str("\n\n");
    }
    body.push_str("data: [DONE]\n\n");
    body
}

fn token_estimate(text: &str) -> u64 {
    u64::try_from(text.len().div_ceil(4)).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_tool_projection_matches_responses_shape_and_stream_events() {
        let call = parse_actor_custom_tool_call(
            r#"{"kind":"custom_tool_call","name":"exec","input":"const r=await tools.write_stdin({session_id:7});text(r.output);"}"#,
        )
        .expect("valid actor custom tool call");
        let response = responses_projection("abcdef", &call, "model", "route", 10, 123);
        assert_eq!(
            response.pointer("/output/0/type"),
            Some(&json!("custom_tool_call"))
        );
        assert_eq!(response.pointer("/output/0/name"), Some(&json!("exec")));
        assert_eq!(response.pointer("/created_at"), Some(&json!(123)));
        let item = &response["output"][0];
        let stream = responses_sse(&response, item);
        assert!(stream.contains("response.custom_tool_call_input.delta"));
        assert!(stream.contains("response.custom_tool_call_input.done"));
        assert!(stream.contains("tools.write_stdin"));
        assert!(stream.ends_with("data: [DONE]\n\n"));
    }

    #[test]
    fn parser_rejects_text_and_function_call_shapes() {
        assert!(parse_actor_custom_tool_call("plain text").is_none());
        assert!(parse_actor_custom_tool_call(r#"{"name":"wait","arguments":{"id":1}}"#).is_none());
        assert!(
            parse_actor_custom_tool_call(r#"{"kind":"custom_tool_call","name":"exec","input":7}"#)
                .is_none()
        );
    }
}
