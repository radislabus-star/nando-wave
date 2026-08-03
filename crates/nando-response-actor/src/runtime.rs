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

pub(crate) fn request_text_from_provider_payload(payload: &Value) -> Option<String> {
    let request = payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("role").and_then(Value::as_str) == Some("user"))
        .filter_map(|item| item.get("content"))
        .flat_map(|content| match content {
            Value::String(text) => vec![text.as_str()],
            Value::Array(parts) => parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect(),
            _ => Vec::new(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    (!request.is_empty()).then_some(request)
}
