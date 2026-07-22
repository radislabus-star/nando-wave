//! Independent operand selection and canonical scalar extraction for proof execution.
//!
//! The selector implementation stays separate from actor/runtime grounding by construction.

use super::input::independently_request_text;
use super::*;

pub(super) fn independently_parse_collection_value(
    output: &Value,
) -> Result<Value, ResponseVerificationError> {
    let mut texts = Vec::new();
    let mut total_bytes = 0_usize;
    match output {
        Value::String(text) => texts.push(text.as_str()),
        Value::Array(parts) if !parts.is_empty() && parts.len() <= 64 => {
            for part in parts {
                if !matches!(
                    part.get("type").and_then(Value::as_str),
                    Some("text" | "input_text" | "output_text")
                ) {
                    return Err(ResponseVerificationError("collection_output_part_type"));
                }
                texts.push(
                    part.get("text")
                        .and_then(Value::as_str)
                        .ok_or(ResponseVerificationError("collection_output_part_text"))?,
                );
            }
        }
        _ => return Err(ResponseVerificationError("collection_output_not_text")),
    }
    let mut candidates = BTreeMap::<Vec<u8>, Value>::new();
    for text in texts {
        total_bytes = total_bytes
            .checked_add(text.len())
            .ok_or(ResponseVerificationError("collection_input_budget"))?;
        if text.is_empty() || total_bytes > 65_536 {
            return Err(ResponseVerificationError("collection_input_budget"));
        }
        independently_collect_collection_candidates(text, &mut candidates)?;
    }
    if candidates.len() != 1 {
        return Err(ResponseVerificationError(if candidates.is_empty() {
            "collection_input_not_json"
        } else {
            "collection_input_ambiguous"
        }));
    }
    Ok(candidates.into_values().next().expect("one candidate"))
}

fn independently_collect_collection_candidates(
    output: &str,
    candidates: &mut BTreeMap<Vec<u8>, Value>,
) -> Result<(), ResponseVerificationError> {
    let mut sources = vec![output.to_owned()];
    let mut fenced = None::<String>;
    for line in output.lines() {
        let trimmed = line.trim();
        if fenced.is_some() && trimmed == "```" {
            sources.push(fenced.take().unwrap_or_default());
        } else if let Some(buffer) = fenced.as_mut() {
            if !buffer.is_empty() {
                buffer.push('\n');
            }
            buffer.push_str(line);
        } else if trimmed == "```" || trimmed.eq_ignore_ascii_case("```json") {
            fenced = Some(String::new());
        } else if trimmed.starts_with(['{', '[']) {
            sources.push(trimmed.to_owned());
        }
    }
    for source in sources {
        if source.is_empty() || source.len() > MAX_VERIFIER_OUTPUT_BYTES {
            continue;
        }
        for object in independently_embedded_json_objects(&source) {
            let value = Value::Object(object);
            if independently_bounded_collection_root(&value) {
                let key = serde_json::to_vec(&value)
                    .map_err(|_| ResponseVerificationError("collection_serialization"))?;
                candidates.insert(key, value);
            }
        }
        if let Ok(value @ Value::Array(_)) = serde_json::from_str::<Value>(&source)
            && !independently_is_text_part_array(&value)
        {
            let value = serde_json::json!({"items": value});
            if independently_bounded_collection_root(&value) {
                let key = serde_json::to_vec(&value)
                    .map_err(|_| ResponseVerificationError("collection_serialization"))?;
                candidates.insert(key, value);
            }
        }
    }
    Ok(())
}

fn independently_is_text_part_array(value: &Value) -> bool {
    value.as_array().is_some_and(|parts| {
        !parts.is_empty()
            && parts.iter().all(|part| {
                matches!(
                    part.get("type").and_then(Value::as_str),
                    Some("text" | "input_text" | "output_text")
                ) && part.get("text").is_some_and(Value::is_string)
            })
    })
}

fn independently_bounded_collection_root(value: &Value) -> bool {
    let Some(object) = value.as_object().filter(|object| object.len() <= 16) else {
        return false;
    };
    let mut arrays = object.values().filter_map(Value::as_array);
    let Some(rows) = arrays.next() else {
        return false;
    };
    if arrays.next().is_some() || rows.is_empty() || rows.len() > 1_024 {
        return false;
    }
    rows.iter().all(|row| {
        row.as_object().is_some_and(|fields| {
            !fields.is_empty()
                && fields.len() <= 16
                && fields.iter().all(|(name, value)| {
                    independently_safe_collection_identifier(name)
                        && independently_safe_collection_scalar(value)
                })
        })
    })
}

fn independently_safe_collection_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if value.len() > 64 || !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
        && ![
            "auth",
            "cookie",
            "credential",
            "passwd",
            "password",
            "secret",
            "token",
            "api_key",
            "apikey",
            "private_key",
            "privatekey",
        ]
        .iter()
        .any(|private| lower.contains(private))
}

fn independently_safe_collection_scalar(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(_) => true,
        Value::Number(number) => {
            number
                .as_i64()
                .is_some_and(|value| (-(1_i64 << 53)..=(1_i64 << 53)).contains(&value))
                || number.as_u64().is_some_and(|value| value <= (1_u64 << 53))
        }
        Value::String(text) => {
            text.len() <= 128
                && ![
                    "auth",
                    "cookie",
                    "credential",
                    "passwd",
                    "password",
                    "secret",
                    "token",
                    "api_key",
                    "apikey",
                    "private_key",
                    "privatekey",
                ]
                .iter()
                .any(|private| text.to_ascii_lowercase().contains(private))
        }
        Value::Array(_) | Value::Object(_) => false,
    }
}

pub(super) fn independent_collection_scalar_type(value: &Value) -> Option<CollectionScalarType> {
    match value {
        Value::String(_) => Some(CollectionScalarType::String),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Some(CollectionScalarType::Integer)
        }
        Value::Bool(_) => Some(CollectionScalarType::Boolean),
        Value::Null | Value::Array(_) | Value::Object(_) | Value::Number(_) => None,
    }
}

pub(super) fn independently_project_status(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
    mapping: ProjectStatusMapping,
) -> Result<&'static str, ResponseVerificationError> {
    let selector_type = match selector {
        ResponseValueSelector::ContinuationHandle { value_type }
        | ResponseValueSelector::UniqueScalar { value_type }
        | ResponseValueSelector::UniqueTurnScalar { value_type }
        | ResponseValueSelector::ContentLinePrefix { value_type, .. }
        | ResponseValueSelector::JsonField { value_type, .. }
        | ResponseValueSelector::JsonScalarOrdinal { value_type, .. }
        | ResponseValueSelector::UniqueTurnJsonField { value_type, .. }
        | ResponseValueSelector::UniqueActiveTurnJsonField { value_type, .. }
        | ResponseValueSelector::RequestReferencedJsonField { value_type }
        | ResponseValueSelector::RequestReferencedJsonFieldOrdinal { value_type, .. }
        | ResponseValueSelector::TurnOutputLine { value_type, .. }
        | ResponseValueSelector::TurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputLine { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarFromEnd { value_type, .. } => *value_type,
        ResponseValueSelector::CommandOutputBody
        | ResponseValueSelector::RequestLastToken
        | ResponseValueSelector::RequestUniqueLiteral => AtomValueType::String,
    };
    if selector_type != AtomValueType::Integer {
        return Err(ResponseVerificationError("status_selector_must_be_integer"));
    }
    let selected = independently_select_scalar(provider_payload, selector)?;
    if selected.value_type != AtomValueType::Integer {
        return Err(ResponseVerificationError("status_selector_type_mismatch"));
    }
    let code = selected
        .value
        .as_u64()
        .filter(|code| *code <= MAX_VERIFIER_PROJECT_STATUS_CODE)
        .ok_or(ResponseVerificationError("status_integer_out_of_bounds"))?;
    Ok(match mapping {
        ProjectStatusMapping::ZeroIsSuccess if code == 0 => "success",
        ProjectStatusMapping::ZeroIsSuccess => "failure",
        ProjectStatusMapping::ZeroIsPass if code == 0 => "PASS",
        ProjectStatusMapping::ZeroIsPass => "FAIL",
        ProjectStatusMapping::ZeroIsOk if code == 0 => "OK",
        ProjectStatusMapping::ZeroIsOk => "ERROR",
        ProjectStatusMapping::ZeroIsTrue if code == 0 => "true",
        ProjectStatusMapping::ZeroIsTrue => "false",
    })
}

pub(super) fn independently_select_scalar(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let request = independently_request_text(provider_payload).unwrap_or_default();
    independently_select_scalar_with_request(&request, provider_payload, selector)
}

pub(super) fn independently_select_scalar_with_request(
    request_text: &str,
    provider_payload: &Value,
    selector: &ResponseValueSelector,
) -> Result<VerifierScalar, ResponseVerificationError> {
    match selector {
        ResponseValueSelector::ContinuationHandle { value_type } => {
            independently_continuation_handle(provider_payload, *value_type)
        }
        ResponseValueSelector::UniqueScalar { value_type } => {
            let scalar = if *value_type == AtomValueType::Collection {
                independently_unique_collection(provider_payload)?
            } else {
                independently_unique_scalar(provider_payload)?
            };
            if scalar.value_type == *value_type {
                Ok(scalar)
            } else {
                Err(ResponseVerificationError("selector_type_mismatch"))
            }
        }
        ResponseValueSelector::UniqueTurnScalar { value_type } => {
            independently_unique_turn_scalar(provider_payload, *value_type)
        }
        ResponseValueSelector::ContentLinePrefix { prefix, value_type } => {
            let output = independently_latest_tool_output(provider_payload)?;
            let parts = independently_bounded_output_text_parts(output)?;
            let mut matches = Vec::new();
            for text in parts {
                for line in text.lines() {
                    let Some(value) = line.trim().strip_prefix(prefix).map(str::trim) else {
                        continue;
                    };
                    if !value.is_empty() {
                        matches.push(independently_parse_scalar_text(value, *value_type)?);
                    }
                }
            }
            if matches.len() != 1 {
                return Err(ResponseVerificationError("selector_prefix_cardinality"));
            }
            matches
                .pop()
                .ok_or(ResponseVerificationError("selector_prefix_missing"))
        }
        ResponseValueSelector::JsonField { field, value_type } => {
            let output = independently_latest_tool_output(provider_payload)?;
            let parts = independently_bounded_output_text_parts(output)?;
            let mut matches = Vec::new();
            for text in parts {
                for object in independently_embedded_json_objects(text) {
                    independently_collect_json_field(
                        &Value::Object(object),
                        field,
                        *value_type,
                        0,
                        &mut matches,
                    )?;
                }
            }
            matches.sort_by_cached_key(|item| item.value.to_string());
            matches.dedup();
            if matches.len() != 1 {
                return Err(ResponseVerificationError("selector_field_cardinality"));
            }
            matches
                .pop()
                .ok_or(ResponseVerificationError("selector_field_missing"))
        }
        ResponseValueSelector::JsonScalarOrdinal {
            ordinal,
            value_type,
        } => {
            let output = independently_latest_tool_output(provider_payload)?;
            let mut matches = Vec::new();
            for text in independently_bounded_output_text_parts(output)? {
                for object in independently_embedded_json_objects(text) {
                    independently_collect_json_scalars(
                        &Value::Object(object),
                        *value_type,
                        0,
                        &mut matches,
                    )?;
                }
            }
            matches
                .into_iter()
                .nth(usize::from(*ordinal))
                .ok_or(ResponseVerificationError("selector_scalar_ordinal_missing"))
        }
        ResponseValueSelector::UniqueTurnJsonField { field, value_type } => {
            independently_unique_turn_json_field(provider_payload, field, *value_type)
        }
        ResponseValueSelector::UniqueActiveTurnJsonField { field, value_type } => {
            independently_unique_active_turn_json_field(provider_payload, field, *value_type)
        }
        ResponseValueSelector::RequestReferencedJsonField { value_type } => {
            independently_request_referenced_json_field(request_text, provider_payload, *value_type)
        }
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        } => independently_request_referenced_json_field_ordinal(
            request_text,
            provider_payload,
            *ordinal,
            *value_type,
        ),
        ResponseValueSelector::TurnOutputLine {
            output_ordinal,
            line_index,
            value_type,
        } => independently_turn_output_line(
            provider_payload,
            *output_ordinal,
            *line_index,
            *value_type,
        ),
        ResponseValueSelector::TurnOutputScalarOrdinal {
            output_ordinal,
            scalar_ordinal,
            value_type,
        } => independently_turn_output_scalar_ordinal(
            provider_payload,
            *output_ordinal,
            *scalar_ordinal,
            *value_type,
        ),
        ResponseValueSelector::LatestTurnOutputLine {
            line_index,
            value_type,
        } => independently_latest_turn_output_line(provider_payload, *line_index, *value_type),
        ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal,
            value_type,
        } => independently_latest_turn_output_scalar_ordinal(
            provider_payload,
            *scalar_ordinal,
            *value_type,
        ),
        ResponseValueSelector::LatestTurnOutputScalarFromEnd {
            reverse_ordinal,
            value_type,
        } => independently_latest_turn_output_scalar_from_end(
            provider_payload,
            *reverse_ordinal,
            *value_type,
        ),
        ResponseValueSelector::CommandOutputBody => Ok(VerifierScalar {
            value: Value::String(independently_command_output_body(provider_payload)?),
            value_type: AtomValueType::String,
        }),
        ResponseValueSelector::RequestLastToken => Ok(VerifierScalar {
            value: Value::String(independently_request_last_token(provider_payload)?),
            value_type: AtomValueType::String,
        }),
        ResponseValueSelector::RequestUniqueLiteral => Ok(VerifierScalar {
            value: Value::String(independently_request_unique_literal(provider_payload)?),
            value_type: AtomValueType::String,
        }),
    }
}

fn independently_continuation_handle(
    provider_payload: &Value,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if !matches!(
        value_type,
        AtomValueType::Identifier | AtomValueType::String
    ) {
        return Err(ResponseVerificationError("continuation_handle_type"));
    }
    let output = independently_latest_tool_output(provider_payload)?;
    let mut matches = Vec::new();
    for text in independently_bounded_output_text_parts(output)? {
        for line in text.lines() {
            let line = line.trim();
            let tail = [
                "Script running with cell ID ",
                "Process running with session ID ",
            ]
            .into_iter()
            .find_map(|prefix| line.strip_prefix(prefix));
            let Some(value) = tail.and_then(|tail| tail.split_whitespace().next()) else {
                continue;
            };
            if !value.is_empty()
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
            {
                matches.push(value.to_owned());
            }
        }
    }
    matches.sort();
    matches.dedup();
    if matches.len() != 1 {
        return Err(ResponseVerificationError("continuation_handle_cardinality"));
    }
    Ok(VerifierScalar {
        value: Value::String(matches.remove(0)),
        value_type,
    })
}

fn independently_unique_turn_scalar(
    provider_payload: &Value,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let items = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or(ResponseVerificationError("turn_input_missing"))?;
    let turn_start = items
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    let mut matches = Vec::new();
    for item in &items[turn_start..] {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        ) {
            continue;
        }
        let Some(output) = item.get("output") else {
            continue;
        };
        for text in independently_bounded_output_text_parts(output)? {
            let mut parsed_values = Vec::new();
            if let Ok(value) = serde_json::from_str::<Value>(text) {
                parsed_values.push(value);
            } else {
                parsed_values.extend(
                    independently_embedded_json_objects(text)
                        .into_iter()
                        .map(Value::Object),
                );
            }
            for value in parsed_values {
                independently_collect_json_scalars(&value, value_type, 0, &mut matches)?;
                if matches.len() > MAX_VERIFIER_SCALARS {
                    return Err(ResponseVerificationError(
                        "selector_turn_scalar_structure_budget",
                    ));
                }
            }
        }
    }
    matches.sort_by_cached_key(|scalar| scalar.value.to_string());
    matches.dedup();
    if matches.len() != 1 {
        return Err(ResponseVerificationError(
            "selector_turn_scalar_cardinality",
        ));
    }
    matches
        .pop()
        .ok_or(ResponseVerificationError("selector_turn_scalar_missing"))
}

fn independently_collect_json_scalars(
    value: &Value,
    value_type: AtomValueType,
    depth: usize,
    output: &mut Vec<VerifierScalar>,
) -> Result<(), ResponseVerificationError> {
    if depth > 8 || output.len() >= 64 {
        return Err(ResponseVerificationError(
            "selector_scalar_ordinal_structure_budget",
        ));
    }
    if let Ok(scalar) = independently_typed_scalar(value.clone(), value_type) {
        output.push(scalar);
        return Ok(());
    }
    match value {
        Value::Object(object) => {
            for value in object.values() {
                independently_collect_json_scalars(
                    value,
                    value_type,
                    depth.saturating_add(1),
                    output,
                )?;
            }
        }
        Value::Array(values) => {
            for value in values {
                independently_collect_json_scalars(
                    value,
                    value_type,
                    depth.saturating_add(1),
                    output,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn independently_collect_json_field(
    value: &Value,
    field: &str,
    value_type: AtomValueType,
    depth: usize,
    output: &mut Vec<VerifierScalar>,
) -> Result<(), ResponseVerificationError> {
    if depth > 8 || output.len() >= 64 {
        return Err(ResponseVerificationError("selector_field_structure_budget"));
    }
    match value {
        Value::Object(object) => {
            for (name, value) in object {
                if name == field {
                    output.push(independently_typed_scalar(value.clone(), value_type)?);
                }
                independently_collect_json_field(
                    value,
                    field,
                    value_type,
                    depth.saturating_add(1),
                    output,
                )?;
            }
        }
        Value::Array(values) => {
            for value in values {
                independently_collect_json_field(
                    value,
                    field,
                    value_type,
                    depth.saturating_add(1),
                    output,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn independently_request_referenced_json_field(
    request: &str,
    provider_payload: &Value,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if request.is_empty() {
        return Err(ResponseVerificationError("selector_request_text_missing"));
    }
    let request_tokens = independently_identifier_tokens(request);
    let output = independently_latest_tool_output(provider_payload)?;
    let mut matches = Vec::<(String, VerifierScalar)>::new();
    for text in independently_bounded_output_text_parts(output)? {
        for object in independently_embedded_json_objects(text) {
            independently_collect_request_referenced_fields(
                &Value::Object(object),
                &request_tokens,
                value_type,
                0,
                &mut matches,
            )?;
        }
    }
    matches.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.value.to_string().cmp(&right.1.value.to_string()))
    });
    matches.dedup();
    if matches.len() != 1 {
        return Err(ResponseVerificationError(
            "selector_request_field_cardinality",
        ));
    }
    matches
        .pop()
        .map(|(_, scalar)| scalar)
        .ok_or(ResponseVerificationError("selector_request_field_missing"))
}

fn independently_request_referenced_json_field_ordinal(
    request: &str,
    provider_payload: &Value,
    ordinal: u16,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if request.is_empty() {
        return Err(ResponseVerificationError("selector_request_text_missing"));
    }
    let request_tokens = independently_identifier_tokens(request);
    let output = independently_latest_tool_output(provider_payload)?;
    let mut matches = Vec::<(String, VerifierScalar)>::new();
    for text in independently_bounded_output_text_parts(output)? {
        for object in independently_embedded_json_objects(text) {
            independently_collect_request_referenced_fields(
                &Value::Object(object),
                &request_tokens,
                value_type,
                0,
                &mut matches,
            )?;
        }
    }
    matches.sort_by(|left, right| {
        independently_request_identifier_position(&request_tokens, &left.0)
            .cmp(&independently_request_identifier_position(
                &request_tokens,
                &right.0,
            ))
            .then_with(|| left.0.cmp(&right.0))
            .then_with(|| left.1.value.to_string().cmp(&right.1.value.to_string()))
    });
    matches.dedup();
    matches
        .into_iter()
        .nth(usize::from(ordinal))
        .map(|(_, scalar)| scalar)
        .ok_or(ResponseVerificationError(
            "selector_request_field_ordinal_missing",
        ))
}

fn independently_request_identifier_position(
    request_tokens: &[String],
    identifier: &str,
) -> Option<usize> {
    let identifier_tokens = independently_identifier_tokens(identifier);
    (!identifier_tokens.is_empty())
        .then(|| {
            request_tokens
                .windows(identifier_tokens.len())
                .position(|window| window == identifier_tokens)
        })
        .flatten()
}

fn independently_collect_request_referenced_fields(
    value: &Value,
    request_tokens: &[String],
    value_type: AtomValueType,
    depth: usize,
    output: &mut Vec<(String, VerifierScalar)>,
) -> Result<(), ResponseVerificationError> {
    if depth > 8 || output.len() >= 64 {
        return Err(ResponseVerificationError(
            "selector_request_field_structure_budget",
        ));
    }
    match value {
        Value::Object(object) => {
            for (field, value) in object {
                if independently_request_mentions_identifier(request_tokens, field)
                    && let Ok(scalar) = independently_typed_scalar(value.clone(), value_type)
                {
                    output.push((field.clone(), scalar));
                }
                independently_collect_request_referenced_fields(
                    value,
                    request_tokens,
                    value_type,
                    depth.saturating_add(1),
                    output,
                )?;
            }
        }
        Value::Array(values) => {
            for value in values {
                independently_collect_request_referenced_fields(
                    value,
                    request_tokens,
                    value_type,
                    depth.saturating_add(1),
                    output,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn independently_identifier_tokens(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_lowercase)
        .take(256)
        .collect()
}

pub(super) fn independently_request_mentions_identifier(
    request_tokens: &[String],
    identifier: &str,
) -> bool {
    let identifier_tokens = independently_identifier_tokens(identifier);
    !identifier_tokens.is_empty()
        && request_tokens
            .windows(identifier_tokens.len())
            .any(|window| window == identifier_tokens)
}

fn independently_request_unique_literal(
    provider_payload: &Value,
) -> Result<String, ResponseVerificationError> {
    let request = independently_request_text(provider_payload)?;
    let mut values = BTreeMap::<String, ()>::new();
    for delimiter in ['`', '\'', '"'] {
        let parts = request.split(delimiter).collect::<Vec<_>>();
        for value in parts.iter().skip(1).step_by(2) {
            let value = value.trim();
            if !value.is_empty() && value.len() <= 128 && !value.contains(['\n', '\r']) {
                values.insert(value.to_owned(), ());
            }
        }
    }
    if values.is_empty() {
        for value in request.split(|character: char| {
            !character.is_alphanumeric() && character != '_' && character != '-'
        }) {
            if (3..=128).contains(&value.len())
                && value
                    .chars()
                    .any(|character| character.is_ascii_uppercase())
                && value.chars().all(|character| {
                    character.is_ascii_uppercase()
                        || character.is_ascii_digit()
                        || matches!(character, '_' | '-')
                })
            {
                values.insert(value.to_owned(), ());
            }
        }
    }
    if values.len() != 1 {
        return Err(ResponseVerificationError(
            "request_unique_literal_cardinality",
        ));
    }
    values
        .into_keys()
        .next()
        .ok_or(ResponseVerificationError("request_unique_literal_missing"))
}

fn independently_request_last_token(
    provider_payload: &Value,
) -> Result<String, ResponseVerificationError> {
    let request = independently_request_text(provider_payload)?;
    let token = request
        .split_whitespace()
        .next_back()
        .map(|value| {
            value.trim_matches(|character: char| {
                !character.is_alphanumeric() && character != '_' && character != '-'
            })
        })
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .ok_or(ResponseVerificationError("request_last_token_missing"))?;
    Ok(token.to_owned())
}

fn independently_command_output_body(
    provider_payload: &Value,
) -> Result<String, ResponseVerificationError> {
    let output = independently_latest_tool_output(provider_payload)?;
    let parts = independently_command_output_text_parts(output)?;
    let mut body = String::new();
    let mut after_marker = false;
    for part in parts {
        if !after_marker {
            let Some((_, suffix)) = part.split_once("\nOutput:\n") else {
                continue;
            };
            after_marker = true;
            body.push_str(suffix);
        } else {
            body.push_str(&part);
        }
        if body.len() > MAX_VERIFIER_OUTPUT_BYTES {
            return Err(ResponseVerificationError("command_output_body_budget"));
        }
    }
    let body = body.trim_end_matches(['\n', '\r']).to_owned();
    if !after_marker || body.is_empty() {
        Err(ResponseVerificationError("command_output_body_missing"))
    } else {
        Ok(body)
    }
}

fn independently_command_output_text_parts(
    output: &Value,
) -> Result<Vec<String>, ResponseVerificationError> {
    if let Some(text) = output.as_str()
        && let Ok(parsed) = serde_json::from_str::<Value>(text)
        && parsed.is_array()
    {
        return independently_bounded_output_text_parts(&parsed)
            .map(|parts| parts.into_iter().map(str::to_owned).collect());
    }
    independently_bounded_output_text_parts(output)
        .map(|parts| parts.into_iter().map(str::to_owned).collect())
}

pub(super) fn independently_embedded_json_objects(
    text: &str,
) -> Vec<serde_json::Map<String, Value>> {
    independently_embedded_json_objects_at_depth(text, 0)
}

fn independently_embedded_json_objects_at_depth(
    text: &str,
    depth: usize,
) -> Vec<serde_json::Map<String, Value>> {
    if depth > 4 {
        return Vec::new();
    }
    let mut sources = vec![text.trim().to_owned()];
    if let Some((_, output)) = text.rsplit_once("\nOutput:\n") {
        sources.push(output.trim().to_owned());
    }
    let mut fence = None::<String>;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "```" {
            if let Some(value) = fence.take() {
                sources.push(value);
            }
        } else if trimmed.eq_ignore_ascii_case("```json") {
            fence = Some(String::new());
        } else if let Some(value) = &mut fence {
            if !value.is_empty() {
                value.push('\n');
            }
            value.push_str(line);
        }
    }
    let mut candidates = BTreeMap::<Vec<u8>, serde_json::Map<String, Value>>::new();
    for source in sources {
        let Ok(value) = serde_json::from_str::<Value>(&source) else {
            continue;
        };
        independently_collect_json_objects(&value, &mut candidates, depth);
    }
    candidates.into_values().collect()
}

fn independently_collect_json_objects(
    value: &Value,
    output: &mut BTreeMap<Vec<u8>, serde_json::Map<String, Value>>,
    depth: usize,
) {
    match value {
        Value::Object(object) => {
            let mut encoded_children = BTreeMap::new();
            if depth < 4 {
                for text in object.values().filter_map(Value::as_str) {
                    for child in independently_embedded_json_objects_at_depth(text, depth + 1) {
                        if let Ok(key) = serde_json::to_vec(&child) {
                            encoded_children.insert(key, child);
                        }
                    }
                }
            }
            if encoded_children.len() == 1 {
                output.extend(encoded_children);
                return;
            }
            if let Ok(key) = serde_json::to_vec(value) {
                output.insert(key, object.clone());
            }
        }
        Value::Array(parts) => {
            for part in parts {
                let Some(text) = part.get("text").and_then(Value::as_str) else {
                    continue;
                };
                for object in independently_embedded_json_objects_at_depth(text, depth + 1) {
                    if let Ok(key) = serde_json::to_vec(&object) {
                        output.insert(key, object);
                    }
                }
            }
        }
        _ => {}
    }
}

fn independently_turn_output_line(
    provider_payload: &Value,
    output_ordinal: u16,
    line_index: u16,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if value_type != AtomValueType::String || output_ordinal == 0 {
        return Err(ResponseVerificationError(
            "turn_output_line_selector_invalid",
        ));
    }
    let output = independently_active_turn_output_value(provider_payload, Some(output_ordinal))?;
    independently_output_line_scalar(output, line_index)
}

fn independently_latest_turn_output_line(
    provider_payload: &Value,
    line_index: u16,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if value_type != AtomValueType::String {
        return Err(ResponseVerificationError(
            "latest_turn_output_line_selector_invalid",
        ));
    }
    let output = independently_active_turn_output_value(provider_payload, None)?;
    independently_output_line_scalar(output, line_index)
}

fn independently_output_line_scalar(
    output: &Value,
    line_index: u16,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let lines = independently_bounded_output_text_parts(output)?
        .into_iter()
        .flat_map(str::lines)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let line = lines
        .get(usize::from(line_index))
        .filter(|line| line.len() <= 512)
        .ok_or(ResponseVerificationError("turn_output_line_missing"))?;
    independently_typed_scalar(Value::String((*line).to_owned()), AtomValueType::String)
}

fn independently_turn_output_scalar_ordinal(
    provider_payload: &Value,
    output_ordinal: u16,
    scalar_ordinal: u16,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if output_ordinal == 0 || matches!(value_type, AtomValueType::Collection) {
        return Err(ResponseVerificationError(
            "turn_output_scalar_ordinal_selector_invalid",
        ));
    }
    let output = independently_active_turn_output_value(provider_payload, Some(output_ordinal))?;
    independently_output_scalar_ordinal(output, scalar_ordinal, value_type)
}

fn independently_latest_turn_output_scalar_ordinal(
    provider_payload: &Value,
    scalar_ordinal: u16,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if matches!(value_type, AtomValueType::Collection) {
        return Err(ResponseVerificationError(
            "latest_turn_output_scalar_ordinal_selector_invalid",
        ));
    }
    let output = independently_active_turn_output_value(provider_payload, None)?;
    independently_output_scalar_ordinal(output, scalar_ordinal, value_type)
}

fn independently_latest_turn_output_scalar_from_end(
    provider_payload: &Value,
    reverse_ordinal: u16,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if matches!(value_type, AtomValueType::Collection) {
        return Err(ResponseVerificationError(
            "latest_turn_output_scalar_from_end_selector_invalid",
        ));
    }
    let output = independently_active_turn_output_value(provider_payload, None)?;
    let mut scalars = Vec::new();
    for text in independently_bounded_output_text_parts(output)? {
        independently_collect_output_scalars(text, &mut scalars)?;
    }
    scalars
        .into_iter()
        .filter(|scalar| {
            scalar.value_type == value_type
                || matches!(
                    (scalar.value_type, value_type),
                    (AtomValueType::Identifier, AtomValueType::String)
                )
        })
        .rev()
        .nth(usize::from(reverse_ordinal))
        .map(|mut scalar| {
            scalar.value_type = value_type;
            scalar
        })
        .ok_or(ResponseVerificationError(
            "latest_turn_output_scalar_from_end_missing",
        ))
}

pub(super) fn independently_active_turn_output_value(
    provider_payload: &Value,
    output_ordinal: Option<u16>,
) -> Result<&Value, ResponseVerificationError> {
    let items = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or(ResponseVerificationError("turn_input_missing"))?;
    let turn_start = items
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    let mut outputs = items[turn_start..].iter().filter(|item| {
        matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        )
    });
    let item = match output_ordinal {
        Some(ordinal) if ordinal > 0 => outputs.nth(usize::from(ordinal - 1)),
        Some(_) => None,
        None => outputs.next_back(),
    }
    .ok_or(ResponseVerificationError("turn_output_ordinal_missing"))?;
    item.get("output")
        .ok_or(ResponseVerificationError("turn_output_missing"))
}

fn independently_output_scalar_ordinal(
    output: &Value,
    scalar_ordinal: u16,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let mut scalars = Vec::new();
    for text in independently_bounded_output_text_parts(output)? {
        independently_collect_output_scalars(text, &mut scalars)?;
    }
    scalars
        .into_iter()
        .filter(|scalar| {
            scalar.value_type == value_type
                || matches!(
                    (scalar.value_type, value_type),
                    (AtomValueType::Identifier, AtomValueType::String)
                )
        })
        .nth(usize::from(scalar_ordinal))
        .map(|mut scalar| {
            scalar.value_type = value_type;
            scalar
        })
        .ok_or(ResponseVerificationError(
            "turn_output_scalar_ordinal_missing",
        ))
}

fn independently_collect_output_scalars(
    text: &str,
    output: &mut Vec<VerifierScalar>,
) -> Result<(), ResponseVerificationError> {
    if output.len() >= MAX_VERIFIER_SCALARS {
        return Err(ResponseVerificationError(
            "turn_output_scalar_ordinal_budget",
        ));
    }
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return independently_collect_scalars(&value, 0, output);
    }
    let embedded = independently_embedded_json_objects(text);
    if !embedded.is_empty() {
        for object in embedded {
            independently_collect_scalars(&Value::Object(object), 0, output)?;
        }
        return Ok(());
    }
    independently_collect_plain_text_scalars(text, output)
}

fn independently_collect_plain_text_scalars(
    text: &str,
    output: &mut Vec<VerifierScalar>,
) -> Result<(), ResponseVerificationError> {
    for (start, token) in text
        .split(|character: char| !character.is_ascii_alphanumeric())
        .scan(0_usize, |offset, token| {
            let relative = text[*offset..].find(token).unwrap_or(0);
            let start = (*offset).saturating_add(relative);
            *offset = start.saturating_add(token.len());
            Some((start, token))
        })
    {
        if token.is_empty() {
            continue;
        }
        if output.len() >= MAX_VERIFIER_SCALARS {
            return Err(ResponseVerificationError(
                "turn_output_scalar_ordinal_budget",
            ));
        }
        let end = start.saturating_add(token.len());
        let decimal_neighbor = text[..start].ends_with('.') || text[end..].starts_with('.');
        if token.bytes().all(|byte| byte.is_ascii_digit()) && !decimal_neighbor {
            if let Ok(value) = token.parse::<u64>() {
                output.push(VerifierScalar {
                    value: Value::from(value),
                    value_type: AtomValueType::Integer,
                });
            }
        } else if token.eq_ignore_ascii_case("true") || token.eq_ignore_ascii_case("false") {
            output.push(VerifierScalar {
                value: Value::Bool(token.eq_ignore_ascii_case("true")),
                value_type: AtomValueType::Boolean,
            });
        }
    }
    Ok(())
}

fn independently_unique_turn_json_field(
    provider_payload: &Value,
    field: &str,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let items = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or(ResponseVerificationError("turn_input_missing"))?;
    let turn_start = items
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    let mut matches = Vec::new();
    for item in &items[turn_start..] {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        ) {
            continue;
        }
        let Some(output) = item.get("output") else {
            continue;
        };
        for text in independently_bounded_output_text_parts(output)? {
            let Ok(Value::Object(object)) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            if let Some(value) = object.get(field) {
                let scalar = independently_typed_scalar(value.clone(), value_type)?;
                if !matches.contains(&scalar) {
                    matches.push(scalar);
                }
            }
        }
    }
    if matches.len() != 1 {
        return Err(ResponseVerificationError("selector_turn_field_cardinality"));
    }
    matches
        .pop()
        .ok_or(ResponseVerificationError("selector_turn_field_missing"))
}

fn independently_unique_active_turn_json_field(
    provider_payload: &Value,
    field: &str,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let items = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or(ResponseVerificationError("turn_input_missing"))?;
    let turn_start = items
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    let mut active = Vec::new();
    for item in &items[turn_start..] {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        ) {
            continue;
        }
        let Some(output) = item.get("output") else {
            continue;
        };
        let mut observed = Vec::new();
        let mut completed = false;
        for text in independently_bounded_output_text_parts(output)? {
            let Ok(Value::Object(object)) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            completed |= object.get("exit_code").is_some_and(Value::is_i64);
            if let Some(value) = object.get(field) {
                observed.push(independently_typed_scalar(value.clone(), value_type)?);
            }
        }
        if completed && observed.is_empty() && active.len() == 1 {
            active.clear();
        }
        for scalar in observed {
            if !active.contains(&scalar) {
                active.push(scalar);
            }
        }
    }
    if active.len() != 1 {
        return Err(ResponseVerificationError(
            "selector_active_turn_field_cardinality",
        ));
    }
    active.pop().ok_or(ResponseVerificationError(
        "selector_active_turn_field_missing",
    ))
}

pub(super) fn independently_latest_tool_output(
    provider_payload: &Value,
) -> Result<&Value, ResponseVerificationError> {
    let item = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .and_then(|items| items.last())
        .ok_or(ResponseVerificationError("immediate_tool_output_missing"))?;
    if !matches!(
        item.get("type").and_then(Value::as_str),
        Some("function_call_output" | "custom_tool_call_output")
    ) {
        return Err(ResponseVerificationError("stale_tool_output"));
    }
    item.get("output")
        .ok_or(ResponseVerificationError("immediate_tool_output_missing"))
}

fn independently_unique_scalar(
    provider_payload: &Value,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let output = independently_latest_tool_output(provider_payload)?
        .as_str()
        .ok_or(ResponseVerificationError("scalar_output_not_text"))?
        .trim();
    if output.is_empty() || output.len() > MAX_VERIFIER_OUTPUT_BYTES {
        return Err(ResponseVerificationError("scalar_output_budget"));
    }
    let value = serde_json::from_str::<Value>(output).unwrap_or_else(|_| {
        if output.contains(['\n', '\r']) {
            Value::Null
        } else {
            Value::String(output.to_owned())
        }
    });
    let mut scalars = Vec::new();
    independently_collect_scalars(&value, 0, &mut scalars)?;
    if scalars.len() != 1 {
        return Err(ResponseVerificationError("unique_scalar_cardinality"));
    }
    scalars
        .pop()
        .ok_or(ResponseVerificationError("unique_scalar_missing"))
}

fn independently_unique_collection(
    provider_payload: &Value,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let output = independently_latest_tool_output(provider_payload)?;
    let value = independently_parse_collection_value(output)?;
    if !matches!(value, Value::Array(_) | Value::Object(_)) {
        return Err(ResponseVerificationError("selector_collection_unsupported"));
    }
    Ok(VerifierScalar {
        value,
        value_type: AtomValueType::Collection,
    })
}

fn independently_bounded_output_text_parts(
    output: &Value,
) -> Result<Vec<&str>, ResponseVerificationError> {
    let parts = if let Some(text) = output.as_str() {
        vec![text]
    } else if let Some(items) = output.as_array() {
        if items.is_empty() || items.len() > MAX_VERIFIER_SCALARS {
            return Err(ResponseVerificationError("output_part_cardinality"));
        }
        let mut parts = Vec::with_capacity(items.len());
        for item in items {
            if !matches!(
                item.get("type").and_then(Value::as_str),
                Some("text" | "input_text" | "output_text")
            ) {
                return Err(ResponseVerificationError("unsupported_output_part_type"));
            }
            parts.push(
                item.get("text")
                    .and_then(Value::as_str)
                    .ok_or(ResponseVerificationError("output_part_text_missing"))?,
            );
        }
        parts
    } else {
        return Err(ResponseVerificationError("tool_output_not_text"));
    };
    let total_bytes = parts
        .iter()
        .try_fold(0_usize, |total, part| total.checked_add(part.len()))
        .ok_or(ResponseVerificationError("scalar_output_budget"))?;
    if total_bytes == 0 || total_bytes > MAX_VERIFIER_OUTPUT_BYTES {
        return Err(ResponseVerificationError("scalar_output_budget"));
    }
    Ok(parts)
}

fn independently_parse_scalar_text(
    value: &str,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if value.len() > MAX_VERIFIER_OUTPUT_BYTES {
        return Err(ResponseVerificationError("scalar_output_budget"));
    }
    let parsed = match value_type {
        AtomValueType::Integer => value
            .parse::<u64>()
            .map(Value::from)
            .map_err(|_| ResponseVerificationError("selector_integer_parse"))?,
        AtomValueType::Boolean => value
            .parse::<bool>()
            .map(Value::from)
            .map_err(|_| ResponseVerificationError("selector_boolean_parse"))?,
        AtomValueType::String | AtomValueType::Identifier => Value::String(value.to_owned()),
        AtomValueType::Collection => {
            return Err(ResponseVerificationError("selector_collection_unsupported"));
        }
    };
    independently_typed_scalar(parsed, value_type)
}

fn independently_typed_scalar(
    value: Value,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let actual = independently_scalar_type(&value)?;
    let compatible = actual == value_type
        || matches!(
            (actual, value_type),
            (AtomValueType::Identifier, AtomValueType::String)
        );
    if !compatible {
        return Err(ResponseVerificationError("selector_type_mismatch"));
    }
    Ok(VerifierScalar { value, value_type })
}

fn independently_collect_scalars(
    value: &Value,
    depth: usize,
    scalars: &mut Vec<VerifierScalar>,
) -> Result<(), ResponseVerificationError> {
    if depth > MAX_VERIFIER_DEPTH || scalars.len() >= MAX_VERIFIER_SCALARS {
        return Err(ResponseVerificationError("scalar_structure_budget"));
    }
    match value {
        Value::Null => {}
        Value::Bool(_) | Value::String(_) => scalars.push(VerifierScalar {
            value: value.clone(),
            value_type: independently_scalar_type(value)?,
        }),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            scalars.push(VerifierScalar {
                value: value.clone(),
                value_type: AtomValueType::Integer,
            });
        }
        Value::Number(_) => {
            return Err(ResponseVerificationError("unsupported_scalar_number"));
        }
        Value::Array(values) => {
            for value in values {
                independently_collect_scalars(value, depth.saturating_add(1), scalars)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                independently_collect_scalars(value, depth.saturating_add(1), scalars)?;
            }
        }
    }
    Ok(())
}

fn independently_scalar_type(value: &Value) -> Result<AtomValueType, ResponseVerificationError> {
    match value {
        Value::Bool(_) => Ok(AtomValueType::Boolean),
        Value::Number(number) if number.is_i64() || number.is_u64() => Ok(AtomValueType::Integer),
        Value::String(text) if independently_identifier_like(text) => Ok(AtomValueType::Identifier),
        Value::String(_) => Ok(AtomValueType::String),
        _ => Err(ResponseVerificationError("selector_scalar_unsupported")),
    }
}

fn independently_identifier_like(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/')
        })
}

pub(super) fn independently_format_selected_value(
    selected: &VerifierScalar,
    format: ValueProjectionFormat,
) -> Result<String, ResponseVerificationError> {
    let projected = match format {
        ValueProjectionFormat::PlainText => match &selected.value {
            Value::String(text) if !text.contains(['\n', '\r']) => text.clone(),
            Value::String(_) => return Err(ResponseVerificationError("projection_multiline")),
            Value::Bool(_) | Value::Number(_) => selected.value.to_string(),
            _ => return Err(ResponseVerificationError("projection_non_scalar")),
        },
        ValueProjectionFormat::CanonicalJson => serde_json::to_string(&selected.value)
            .map_err(|_| ResponseVerificationError("projection_serialization"))?,
    };
    if projected.is_empty() || projected.len() > 16_384 {
        return Err(ResponseVerificationError("projection_output_budget"));
    }
    Ok(projected)
}

pub(super) fn sha256_scalar(value: &Value) -> Result<String, ResponseVerificationError> {
    use sha2::{Digest, Sha256};
    if !matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_)) {
        return Err(ResponseVerificationError("projection_non_scalar"));
    }
    let canonical = serde_json::to_vec(value)
        .map_err(|_| ResponseVerificationError("projection_serialization"))?;
    Ok(format!("{:x}", Sha256::digest(canonical)))
}
