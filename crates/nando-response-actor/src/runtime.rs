pub use nando_operator_runtime::*;

use nando_operator_kernel::ResponseProgram;
use serde_json::Value;

/// Compatibility orchestration: runtime proposes an actor result and the
/// independent proof owner decides whether that result may be exposed.
pub fn execute_response(
    program: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
) -> ResponseExecution {
    let derived_request;
    let request_text = if request_text.is_empty() {
        derived_request = request_text_from_provider_payload(provider_payload);
        derived_request.as_deref().unwrap_or_default()
    } else {
        request_text
    };
    execute_response_with_external_validator(
        program,
        request_text,
        provider_payload,
        &|program, request_text, provider_payload, response| {
            nando_operator_proof::verify_response(program, request_text, provider_payload, response)
                .map_err(|error| error.to_string())
        },
    )
}

pub fn request_text_from_provider_payload(payload: &Value) -> Option<String> {
    if let Some(input) = payload.get("input") {
        if let Some(text) = input.as_str().filter(|text| !text.is_empty()) {
            return Some(text.to_owned());
        }
        if let Some(messages) = input.as_array()
            && let Some(text) = latest_request_text(messages)
        {
            return Some(text);
        }
    }
    if let Some(prompt) = payload
        .get("prompt")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
    {
        return Some(prompt.to_owned());
    }
    payload
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| latest_request_text(messages))
}

fn latest_request_text(messages: &[Value]) -> Option<String> {
    let mut fallback = None;
    let mut user = None;
    for message in messages {
        let Some(content) = message.get("content") else {
            continue;
        };
        let text = request_content_text(content);
        if text.is_empty() {
            continue;
        }
        fallback = Some(text.clone());
        if message.get("role").and_then(Value::as_str) == Some("user") {
            user = Some(text);
        }
    }
    user.or(fallback)
}

fn request_content_text(content: &Value) -> String {
    if let Some(text) = content.as_str() {
        return text.to_owned();
    }
    content
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}
