use nando_operator_kernel::RuntimeProjectionV3;
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RequestTextErrorV3 {
    MissingOrInvalid,
    BudgetExhausted,
}

pub(super) fn derive_request_text_v3(
    payload: &Value,
    projection: RuntimeProjectionV3,
    max_bytes: usize,
) -> Result<String, RequestTextErrorV3> {
    let text = match projection {
        RuntimeProjectionV3::Responses => responses_request_text_v3(payload),
        RuntimeProjectionV3::ChatCompletions => payload
            .get("messages")
            .and_then(Value::as_array)
            .and_then(|messages| latest_user_text_v3(messages)),
        RuntimeProjectionV3::TransitionApi => None,
    }
    .filter(|value| !value.is_empty())
    .ok_or(RequestTextErrorV3::MissingOrInvalid)?;
    if text.len() > max_bytes {
        return Err(RequestTextErrorV3::BudgetExhausted);
    }
    Ok(text)
}

fn responses_request_text_v3(payload: &Value) -> Option<String> {
    if let Some(input) = payload.get("input") {
        if let Some(text) = input.as_str() {
            return Some(text.to_owned());
        }
        if let Some(messages) = input.as_array()
            && let Some(text) = latest_user_text_v3(messages)
        {
            return Some(text);
        }
    }
    payload
        .get("prompt")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn latest_user_text_v3(messages: &[Value]) -> Option<String> {
    messages
        .iter()
        .rev()
        .find(|message| message.get("role").and_then(Value::as_str) == Some("user"))
        .and_then(|message| message.get("content"))
        .and_then(message_content_text_v3)
}

fn message_content_text_v3(content: &Value) -> Option<String> {
    if let Some(text) = content.as_str() {
        return Some(text.to_owned());
    }
    let parts = content.as_array()?;
    let text = parts
        .iter()
        .filter_map(|part| {
            let part_type = part.get("type").and_then(Value::as_str)?;
            matches!(part_type, "text" | "input_text" | "output_text")
                .then(|| part.get("text").and_then(Value::as_str))
                .flatten()
        })
        .collect::<Vec<_>>()
        .join("\n");
    (!text.is_empty()).then_some(text)
}
