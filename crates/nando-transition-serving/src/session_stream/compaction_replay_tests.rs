use super::*;
use serde_json::json;
use std::{
    fs::{self, File},
    io::Write,
    sync::{Arc, Mutex},
};

#[test]
fn context_refresh_replay_keeps_nonempty_parity_request() {
    let root = std::env::temp_dir().join(format!(
        "nando-context-refresh-replay-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("temp dir");
    let session_path = root.join("session.jsonl");
    let rows = [
        json!({"type":"session_meta","payload":{"id":"refresh-session"}}),
        json!({"type":"turn_context","payload":{"turn_id":"refresh-turn"}}),
        json!({"type":"response_item","payload":{
            "type":"message",
            "role":"user",
            "content":[{"type":"input_text","text":"continue the natural loop"}]
        }}),
        json!({"type":"response_item","payload":{
            "type":"custom_tool_call",
            "name":"exec",
            "call_id":"start",
            "input":"const r=await tools.exec_command({\"cmd\":\"cargo check\"});text(JSON.stringify(r));"
        }}),
        json!({"type":"response_item","payload":{
            "type":"custom_tool_call_output",
            "call_id":"start",
            "output":"{\"chunk_id\":\"chunk-1\",\"session_id\":60906,\"output\":\"Compiling\"}"
        }}),
        json!({"type":"turn_context","payload":{"turn_id":"refresh-turn"}}),
        json!({"type":"event_msg","payload":{"type":"context_compacted"}}),
        json!({"type":"response_item","payload":{
            "type":"custom_tool_call",
            "name":"exec",
            "call_id":"continue",
            "input":"const r=await tools.write_stdin({\"session_id\":60906,\"chars\":\"\",\"yield_time_ms\":1000});text(r.output);"
        }}),
        json!({"type":"response_item","payload":{
            "type":"custom_tool_call_output",
            "call_id":"continue",
            "output":"accepted"
        }}),
        json!({"type":"event_msg","payload":{
            "type":"token_count",
            "info":{"last_token_usage":{"input_tokens":123}}
        }}),
    ];
    let mut session = File::create(&session_path).expect("session");
    for row in rows {
        writeln!(session, "{row}").expect("session row");
    }
    session.sync_all().expect("session sync");

    let evidence = Arc::new(Mutex::new(
        DeterministicEvidenceLedger::open(root.join("evidence.jsonl"), EvidencePolicyV1::default())
            .expect("evidence ledger"),
    ));
    let metrics = Arc::new(SessionStreamMetrics::default());
    let mut state = SessionState {
        session_id: session_path.to_string_lossy().into_owned(),
        ..SessionState::default()
    };

    let frames = read_appended_frames(
        &session_path,
        &mut state,
        SessionReadContext {
            evidence: &evidence,
            evidence_graphs: None,
            miner: None,
            direct_collection_miner: None,
            metrics: &metrics,
            request_learning: &Arc::new(RequestLearningIndex::default()),
        },
    )
    .expect("replay");

    assert_eq!(state.turn_index, 1);
    assert_eq!(frames.len(), 1, "{frames:#?}");
    let parity = state
        .runtime_parity_cases
        .get(&frames[0].frame_id_sha256)
        .expect("runtime parity case");
    assert_eq!(parity.request_text, "continue the natural loop");
    assert!(!parity.request_text.is_empty());
    fs::remove_dir_all(root).expect("cleanup");
}
