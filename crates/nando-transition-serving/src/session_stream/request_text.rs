use serde_json::Value;

const MAX_RUNTIME_REQUEST_CHARS: usize = 16_384;

pub(super) fn bounded_runtime_request_text(row: &Value) -> Option<String> {
    let row_type = row.get("type").and_then(Value::as_str)?;
    let payload = row.get("payload")?.as_object()?;
    let payload_type = payload.get("type").and_then(Value::as_str)?;

    match (row_type, payload_type) {
        ("event_msg", "user_message") => Some(bounded_parts(
            payload.get("message").and_then(Value::as_str),
        )),
        ("response_item", "message")
            if payload.get("role").and_then(Value::as_str) == Some("user") =>
        {
            Some(bounded_parts(
                payload
                    .get("content")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter(|part| {
                        matches!(
                            part.get("type").and_then(Value::as_str),
                            Some("input_text" | "text")
                        )
                    })
                    .filter_map(|part| part.get("text").and_then(Value::as_str)),
            ))
        }
        _ => None,
    }
}

fn bounded_parts<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut output = String::new();
    let mut chars = 0_usize;

    for part in parts.into_iter().filter(|part| !part.is_empty()) {
        if chars > 0 {
            if chars == MAX_RUNTIME_REQUEST_CHARS {
                break;
            }
            output.push('\n');
            chars += 1;
        }
        let remaining = MAX_RUNTIME_REQUEST_CHARS.saturating_sub(chars);
        for ch in part.chars().take(remaining) {
            output.push(ch);
            chars += 1;
        }
        if chars == MAX_RUNTIME_REQUEST_CHARS {
            break;
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_stream::{
        SessionRowBoundary, SessionState, begin_new_context_turn, classify_session_row_boundary,
        observe_row,
    };
    use serde_json::json;

    #[test]
    fn extracts_legacy_event_message_with_the_same_bound() {
        let row = json!({
            "type": "event_msg",
            "payload": {"type": "user_message", "message": "a".repeat(20_000)}
        });

        let text = bounded_runtime_request_text(&row).expect("recognized user message");

        assert_eq!(text.chars().count(), MAX_RUNTIME_REQUEST_CHARS);
    }

    #[test]
    fn codex_v0146_row_order_populates_runtime_request_text() {
        let rows = [
            json!({
                "timestamp": "2026-08-02T06:08:19Z",
                "type": "event_msg",
                "payload": {"type": "task_started"}
            }),
            json!({
                "timestamp": "2026-08-02T06:08:19Z",
                "type": "turn_context",
                "payload": {"turn_id": "turn-v0146"}
            }),
            json!({
                "timestamp": "2026-08-02T06:08:19Z",
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": "continue the natural loop"}]
                }
            }),
        ];
        let mut state = SessionState::default();
        let mut emitted = Vec::new();

        for row in rows {
            let boundary =
                classify_session_row_boundary(&row, &state.turn_intent_id, state.turn_index);
            if boundary == SessionRowBoundary::NewContext {
                begin_new_context_turn(&row, &mut state, &mut emitted);
            }
            observe_row(&row, &mut state, &mut emitted);
        }

        assert_eq!(state.runtime_request_text, "continue the natural loop");
        assert_eq!(
            state
                .collection_request_item
                .as_ref()
                .and_then(|item| item.pointer("/content/0/text"))
                .and_then(Value::as_str),
            Some("continue the natural loop")
        );
    }

    #[test]
    fn codex_context_refresh_preserves_request_provenance_until_a_new_turn() {
        let rows = [
            json!({
                "type": "turn_context",
                "payload": {"turn_id": "turn-v0146"}
            }),
            json!({
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": "continue the natural loop"}]
                }
            }),
            json!({
                "type": "turn_context",
                "payload": {"turn_id": "turn-v0146"}
            }),
            json!({"type": "compacted", "payload": {}}),
            json!({
                "type": "event_msg",
                "payload": {"type": "context_compacted"}
            }),
        ];
        let mut state = SessionState::default();
        let mut emitted = Vec::new();

        for row in rows {
            let boundary =
                classify_session_row_boundary(&row, &state.turn_intent_id, state.turn_index);
            if boundary == SessionRowBoundary::NewContext {
                begin_new_context_turn(&row, &mut state, &mut emitted);
            }
            observe_row(&row, &mut state, &mut emitted);
        }

        assert_eq!(state.turn_index, 1);
        assert_eq!(state.runtime_request_text, "continue the natural loop");

        let next = json!({
            "type": "turn_context",
            "payload": {"turn_id": "turn-next"}
        });
        assert_eq!(
            classify_session_row_boundary(&next, &state.turn_intent_id, state.turn_index),
            SessionRowBoundary::NewContext
        );
        begin_new_context_turn(&next, &mut state, &mut emitted);

        assert_eq!(state.turn_index, 2);
        assert!(state.runtime_request_text.is_empty());
    }
}
