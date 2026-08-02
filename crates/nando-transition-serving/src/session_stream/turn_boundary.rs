use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SessionRowBoundary {
    None,
    NewContext,
    ContextRefresh,
    UserMessage,
}

impl SessionRowBoundary {
    pub(super) const fn starts_turn(self) -> bool {
        matches!(self, Self::NewContext | Self::UserMessage)
    }

    pub(super) const fn is_new_context(self) -> bool {
        matches!(self, Self::NewContext)
    }
}

pub(super) fn classify_session_row_boundary(
    row: &Value,
    current_turn_intent_id: &str,
    turn_index: u64,
) -> SessionRowBoundary {
    if row.get("type").and_then(Value::as_str) == Some("turn_context") {
        let next = turn_intent_id_from_context(row);
        return if turn_index > 0 && next.is_some_and(|turn_id| turn_id == current_turn_intent_id) {
            SessionRowBoundary::ContextRefresh
        } else {
            SessionRowBoundary::NewContext
        };
    }

    let row_type = row.get("type").and_then(Value::as_str).unwrap_or("");
    let payload = row.get("payload").and_then(Value::as_object);
    let payload_type = payload
        .and_then(|payload| payload.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if row_type == "event_msg" && payload_type == "user_message"
        || row_type == "response_item"
            && payload_type == "message"
            && payload
                .and_then(|payload| payload.get("role"))
                .and_then(Value::as_str)
                == Some("user")
    {
        SessionRowBoundary::UserMessage
    } else {
        SessionRowBoundary::None
    }
}

pub(super) fn turn_intent_id_from_context(row: &Value) -> Option<&str> {
    (row.get("type").and_then(Value::as_str) == Some("turn_context"))
        .then(|| row.get("payload")?.get("turn_id")?.as_str())
        .flatten()
        .filter(|value| !value.is_empty() && value.len() <= 256)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn repeated_explicit_turn_id_is_a_context_refresh() {
        let row = json!({"type":"turn_context","payload":{"turn_id":"turn-1"}});

        assert_eq!(
            classify_session_row_boundary(&row, "turn-1", 1),
            SessionRowBoundary::ContextRefresh
        );
        assert_eq!(
            classify_session_row_boundary(&row, "turn-0", 1),
            SessionRowBoundary::NewContext
        );
    }
}
