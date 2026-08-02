use std::fs::{self, File};
use std::io::Write;

use serde_json::{Value, json};

use super::verified_capture_bound_training_cases_from_sessions;

#[test]
fn capture_bound_replay_censors_frames_before_first_turn_context() {
    let root = std::env::temp_dir().join(format!(
        "nando-capture-boundary-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("capture boundary root");
    let session = root.join("compacted-prefix.jsonl");
    let rows = [
        json!({"type":"session_meta","payload":{"id":"capture-boundary-session"}}),
        json!({"type":"event_msg","payload":{"type":"user_message","message":"unbound prefix"}}),
        json!({"type":"response_item","payload":{"type":"function_call","name":"lookup","call_id":"pre-lookup","arguments":"{}"}}),
        json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"pre-lookup","output":"{\"count\":3}"}}),
        json!({"type":"response_item","payload":{"type":"function_call","name":"submit","call_id":"pre-submit","arguments":"{\"value\":3}"}}),
        json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"pre-submit","output":"accepted"}}),
        json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":30}}}}),
        json!({"type":"turn_context","payload":{"turn_id":"canonical-turn"}}),
        json!({
            "type":"response_item",
            "payload":{
                "type":"message",
                "role":"user",
                "content":[{"type":"input_text","text":"bound request"}]
            }
        }),
        json!({"type":"response_item","payload":{"type":"function_call","name":"lookup","call_id":"post-lookup","arguments":"{}"}}),
        json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"post-lookup","output":"{\"count\":7}"}}),
        json!({"type":"response_item","payload":{"type":"function_call","name":"submit","call_id":"post-submit","arguments":"{\"value\":7}"}}),
        json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"post-submit","output":"accepted"}}),
        json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":70}}}}),
    ];
    let mut file = File::create(&session).expect("session fixture");
    for row in rows {
        writeln!(file, "{row}").expect("session row");
    }
    file.sync_all().expect("session fixture sync");

    let batch = verified_capture_bound_training_cases_from_sessions(
        std::slice::from_ref(&session),
        &root.join("evidence"),
    )
    .expect("capture-bound replay");

    assert_eq!(batch.cases.len(), 1, "{batch:#?}");
    let (frame, parity) = &batch.cases[0];
    assert_eq!(parity.request_text, "bound request");
    assert_eq!(frame.estimated_input_tokens, 70);
    assert_eq!(
        parity
            .provider_payload
            .pointer("/input/0/content/0/text")
            .and_then(Value::as_str),
        Some("bound request")
    );
    assert_ne!(
        frame.session_id_sha256,
        nando_operator_kernel::sha256_bytes(b"capture-boundary-session")
    );

    fs::remove_dir_all(root).expect("capture boundary cleanup");
}
