//! Immutable input observation shared by independent proof programs.

use serde_json::Value;

use super::ResponseVerificationError;

pub(super) fn independently_request_text(
    provider_payload: &Value,
) -> Result<String, ResponseVerificationError> {
    let input = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or(ResponseVerificationError(
            "collection_request_input_missing",
        ))?;
    let mut parts = Vec::new();
    for item in input {
        if item.get("role").and_then(Value::as_str) != Some("user") {
            continue;
        }
        match item.get("content") {
            Some(Value::String(text)) if !text.is_empty() => parts.push(text.as_str()),
            Some(Value::Array(content)) => {
                parts.extend(
                    content
                        .iter()
                        .filter_map(|part| part.get("text").and_then(Value::as_str)),
                );
            }
            _ => {}
        }
    }
    if parts.is_empty() {
        Err(ResponseVerificationError("collection_request_text_missing"))
    } else {
        Ok(parts.join("\n"))
    }
}
