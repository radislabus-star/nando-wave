use std::collections::BTreeMap;

use serde_json::Value;

use crate::program::{
    CollectionAggregateOperation, CollectionOutputRenderer, CollectionProgramStep,
    CollectionScalarType, CustomToolResultProjection, MAX_PROJECT_STATUS_CODE,
    ProjectStatusMapping, ResponseArgument, ResponseOperation, ResponseProgram,
    ResponseRenderSegment, ValueProjectionFormat,
};
use crate::verifier::verify_response;
use crate::{AtomValueType, ResponseValueSelector, SemanticRole};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseExecutionStatus {
    Executed,
    Abstain,
    VerifyFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponseExecution {
    pub status: ResponseExecutionStatus,
    pub reason: String,
    pub response: Option<String>,
    pub verification_receipt_id: Option<String>,
}

impl ResponseExecution {
    fn rejected(status: ResponseExecutionStatus, reason: impl Into<String>) -> Self {
        Self {
            status,
            reason: reason.into(),
            response: None,
            verification_receipt_id: None,
        }
    }
}

pub fn execute_response(
    program: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
) -> ResponseExecution {
    if let Err(reason) = program.validate() {
        return ResponseExecution::rejected(
            ResponseExecutionStatus::Abstain,
            format!("invalid_program:{reason}"),
        );
    }
    let response = match &program.operation {
        ResponseOperation::FunctionCallFromRoles {
            function_name,
            selector,
            arguments,
        } => execute_function_call_from_roles(provider_payload, function_name, selector, arguments),
        ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name,
            inner_tool_name,
            selector,
            arguments,
            projection,
        } => execute_custom_tool_call_from_roles(
            provider_payload,
            custom_tool_name,
            inner_tool_name,
            selector,
            arguments,
            projection,
        ),
        ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            renderer,
            ..
        } => project_selected_value(provider_payload, selector, *format)
            .and_then(|computed| apply_value_renderer(provider_payload, computed, renderer)),
        ResponseOperation::ProjectStatus {
            selector, mapping, ..
        } => project_status(provider_payload, selector, *mapping),
        ResponseOperation::ComposeCollection {
            steps,
            format,
            renderer,
            max_items,
            ..
        } => execute_compose_collection(provider_payload, steps, *format, renderer, *max_items),
        ResponseOperation::CopyAfterPrefix {
            prefixes,
            trim,
            allow_multiline,
        } => execute_copy(request_text, prefixes, *trim, *allow_multiline),
        ResponseOperation::TestResultSummary {
            required_intent_phrases,
            forbidden_intent_terms,
        } => execute_test_summary(
            request_text,
            provider_payload,
            required_intent_phrases,
            forbidden_intent_terms,
        ),
        ResponseOperation::WaitOnYieldedCell {
            function_name,
            yield_time_ms,
            max_tokens,
        } => execute_wait(provider_payload, function_name, *yield_time_ms, *max_tokens),
        ResponseOperation::WaitOnAnyYieldedCell {
            function_name,
            yield_time_ms,
            max_tokens,
        } => execute_wait_any(provider_payload, function_name, *yield_time_ms, *max_tokens),
        ResponseOperation::WaitOnYieldedSurfaces {
            surfaces,
            function_name,
            yield_time_ms,
            max_tokens,
        } => execute_wait_surface(
            provider_payload,
            surfaces,
            function_name,
            *yield_time_ms,
            *max_tokens,
        ),
    };
    let Ok(response) = response else {
        return ResponseExecution::rejected(
            ResponseExecutionStatus::Abstain,
            response.err().unwrap_or("abstain"),
        );
    };
    if response.is_empty() || response.len() > program.max_output_bytes {
        return ResponseExecution::rejected(ResponseExecutionStatus::Abstain, "output_budget");
    }
    if let Err(error) = verify_response(program, request_text, provider_payload, &response) {
        return ResponseExecution::rejected(
            ResponseExecutionStatus::VerifyFailed,
            format!("verification:{error}"),
        );
    }
    ResponseExecution {
        status: ResponseExecutionStatus::Executed,
        reason: "executed".to_owned(),
        response: Some(response),
        verification_receipt_id: None,
    }
}

fn execute_compose_collection(
    provider_payload: &Value,
    steps: &[CollectionProgramStep],
    format: ValueProjectionFormat,
    renderer: &CollectionOutputRenderer,
    max_items: usize,
) -> Result<String, &'static str> {
    let output =
        immediate_tool_output_value(provider_payload).ok_or("immediate_tool_output_missing")?;
    let mut value = collection_json_from_value(output)?;
    let mut filter_field = None::<String>;
    for step in steps {
        value = match step {
            CollectionProgramStep::SelectOnlyArrayField => {
                let object = value.as_object().ok_or("collection_select_not_object")?;
                let mut arrays = object.values().filter(|candidate| candidate.is_array());
                let selected = arrays.next().cloned().ok_or("collection_select_missing")?;
                if arrays.next().is_some() {
                    return Err("collection_select_ambiguous");
                }
                selected
            }
            CollectionProgramStep::SelectField { field } => value
                .as_object()
                .and_then(|object| object.get(field))
                .cloned()
                .ok_or("collection_select_missing")?,
            CollectionProgramStep::FilterFieldEquals {
                field,
                value: expected,
            } => {
                let rows = value.as_array().ok_or("collection_filter_not_array")?;
                if rows.len() > max_items {
                    return Err("collection_item_budget");
                }
                filter_field = Some(field.clone());
                Value::Array(
                    rows.iter()
                        .filter(|row| {
                            row.as_object().and_then(|object| object.get(field))
                                == Some(&expected.as_json())
                        })
                        .cloned()
                        .collect(),
                )
            }
            CollectionProgramStep::FilterUniqueFieldEquals { value: expected } => {
                let rows = value.as_array().ok_or("collection_filter_not_array")?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err("collection_item_budget");
                }
                let expected = expected.as_json();
                let first = rows[0]
                    .as_object()
                    .ok_or("collection_filter_row_not_object")?;
                let mut fields = first.keys().filter(|field| {
                    rows.iter().all(|row| {
                        row.as_object()
                            .is_some_and(|object| object.contains_key(*field))
                    }) && rows.iter().any(|row| row.get(*field) == Some(&expected))
                });
                let field = fields
                    .next()
                    .cloned()
                    .ok_or("collection_filter_field_missing")?;
                if fields.next().is_some() {
                    return Err("collection_filter_field_ambiguous");
                }
                filter_field = Some(field.clone());
                Value::Array(
                    rows.iter()
                        .filter(|row| row.get(&field) == Some(&expected))
                        .cloned()
                        .collect(),
                )
            }
            CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { value_type } => {
                let rows = value.as_array().ok_or("collection_filter_not_array")?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err("collection_item_budget");
                }
                let (field, expected) =
                    request_grounded_collection_value(provider_payload, rows, *value_type)?;
                filter_field = Some(field.clone());
                Value::Array(
                    rows.iter()
                        .filter(|row| row.get(&field) == Some(&expected))
                        .cloned()
                        .collect(),
                )
            }
            CollectionProgramStep::ProjectField { field } => {
                if let Some(rows) = value.as_array() {
                    if rows.len() > max_items {
                        return Err("collection_item_budget");
                    }
                    let projected = rows
                        .iter()
                        .map(|row| {
                            row.as_object()
                                .and_then(|object| object.get(field))
                                .cloned()
                                .ok_or("collection_project_missing")
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Value::Array(projected)
                } else {
                    value
                        .as_object()
                        .and_then(|object| object.get(field))
                        .cloned()
                        .ok_or("collection_project_missing")?
                }
            }
            CollectionProgramStep::ProjectUniqueFieldByType { value_type } => {
                let rows = value.as_array().ok_or("collection_project_not_array")?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err("collection_item_budget");
                }
                let first = rows[0]
                    .as_object()
                    .ok_or("collection_project_row_not_object")?;
                let mut fields = first.keys().filter(|field| {
                    rows.iter().all(|row| {
                        row.get(*field)
                            .is_some_and(|value| collection_scalar_type(value) == Some(*value_type))
                    })
                });
                let field = fields
                    .next()
                    .cloned()
                    .ok_or("collection_project_field_missing")?;
                if fields.next().is_some() {
                    return Err("collection_project_field_ambiguous");
                }
                Value::Array(
                    rows.iter()
                        .map(|row| row.get(&field).cloned().ok_or("collection_project_missing"))
                        .collect::<Result<Vec<_>, _>>()?,
                )
            }
            CollectionProgramStep::ProjectOnlyNonFilterField => {
                let excluded = filter_field
                    .as_deref()
                    .ok_or("collection_filter_role_missing")?;
                let rows = value.as_array().ok_or("collection_project_not_array")?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err("collection_item_budget");
                }
                let first = rows[0]
                    .as_object()
                    .ok_or("collection_project_row_not_object")?;
                let mut fields = first.keys().filter(|field| {
                    field.as_str() != excluded
                        && rows.iter().all(|row| {
                            row.as_object()
                                .is_some_and(|object| object.contains_key(*field))
                        })
                });
                let field = fields
                    .next()
                    .cloned()
                    .ok_or("collection_project_field_missing")?;
                if fields.next().is_some() {
                    return Err("collection_project_field_ambiguous");
                }
                Value::Array(
                    rows.iter()
                        .map(|row| row.get(&field).cloned().ok_or("collection_project_missing"))
                        .collect::<Result<Vec<_>, _>>()?,
                )
            }
            CollectionProgramStep::AggregateUniqueIntegerField { operation } => {
                let rows = value.as_array().ok_or("collection_aggregate_not_array")?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err("collection_item_budget");
                }
                let first = rows[0]
                    .as_object()
                    .ok_or("collection_aggregate_row_not_object")?;
                let mut fields = first.keys().filter(|field| {
                    rows.iter()
                        .all(|row| row.get(*field).and_then(Value::as_i64).is_some())
                });
                let field = fields
                    .next()
                    .cloned()
                    .ok_or("collection_aggregate_field_missing")?;
                if fields.next().is_some() {
                    return Err("collection_aggregate_field_ambiguous");
                }
                let values = rows
                    .iter()
                    .map(|row| {
                        row.get(&field)
                            .and_then(Value::as_i64)
                            .ok_or("collection_aggregate_value_missing")
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let aggregate = match operation {
                    CollectionAggregateOperation::Sum => values
                        .into_iter()
                        .try_fold(0_i64, i64::checked_add)
                        .ok_or("collection_aggregate_overflow")?,
                    CollectionAggregateOperation::Min => values
                        .into_iter()
                        .min()
                        .ok_or("collection_aggregate_empty")?,
                    CollectionAggregateOperation::Max => values
                        .into_iter()
                        .max()
                        .ok_or("collection_aggregate_empty")?,
                };
                Value::from(aggregate)
            }
            CollectionProgramStep::Count => {
                let count = value
                    .as_array()
                    .map(Vec::len)
                    .or_else(|| value.as_object().map(serde_json::Map::len))
                    .ok_or("collection_count_unsupported")?;
                Value::from(u64::try_from(count).map_err(|_| "collection_count_overflow")?)
            }
        };
    }
    let computed = match format {
        ValueProjectionFormat::CanonicalJson => {
            serde_json::to_string(&value).map_err(|_| "collection_serialization")
        }
        ValueProjectionFormat::PlainText => match value {
            Value::String(text) if !text.contains(['\n', '\r']) => Ok(text),
            Value::String(_) => Err("collection_multiline"),
            Value::Bool(_) | Value::Number(_) | Value::Null => Ok(value.to_string()),
            Value::Array(_) | Value::Object(_) => Err("collection_plain_text_non_scalar"),
        },
    }?;
    apply_value_renderer(provider_payload, computed, renderer)
}

fn request_grounded_collection_value(
    provider_payload: &Value,
    rows: &[Value],
    value_type: CollectionScalarType,
) -> Result<(String, Value), &'static str> {
    let request = request_text(provider_payload).ok_or("collection_request_text_missing")?;
    let mut matches = BTreeMap::<Vec<u8>, (String, Value)>::new();
    for row in rows {
        let object = row.as_object().ok_or("collection_filter_row_not_object")?;
        for (field, value) in object {
            if collection_value_type(value) == Some(value_type)
                && request_contains_collection_value(&request, value)
            {
                let key = serde_json::to_vec(&(field, value))
                    .map_err(|_| "collection_request_value_encode")?;
                matches.insert(key, (field.clone(), value.clone()));
            }
        }
    }
    if matches.len() != 1 {
        return Err("collection_request_value_ambiguous");
    }
    matches
        .into_values()
        .next()
        .ok_or("collection_request_value_missing")
}

fn request_text(provider_payload: &Value) -> Option<String> {
    let mut parts = Vec::new();
    for item in provider_payload.get("input")?.as_array()? {
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
    (!parts.is_empty()).then(|| parts.join("\n"))
}

fn collection_value_type(value: &Value) -> Option<CollectionScalarType> {
    match value {
        Value::String(_) => Some(CollectionScalarType::String),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Some(CollectionScalarType::Integer)
        }
        Value::Bool(_) => Some(CollectionScalarType::Boolean),
        _ => None,
    }
}

fn request_contains_collection_value(request: &str, value: &Value) -> bool {
    let needle = match value {
        Value::String(value) if !value.is_empty() && value.len() <= 128 => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        _ => return false,
    };
    request.match_indices(&needle).any(|(start, _)| {
        let end = start.saturating_add(needle.len());
        let left_ok = request[..start]
            .chars()
            .next_back()
            .is_none_or(|character| !character.is_alphanumeric() && character != '_');
        let right_ok = request[end..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_alphanumeric() && character != '_');
        left_ok && right_ok
    })
}

fn apply_output_renderer(
    computed: String,
    renderer: &CollectionOutputRenderer,
) -> Result<String, &'static str> {
    match renderer {
        CollectionOutputRenderer::Direct => Ok(computed),
        CollectionOutputRenderer::RenderTemplate { prefix, suffix } => {
            Ok(format!("{prefix}{computed}{suffix}"))
        }
        CollectionOutputRenderer::RenderSequence { .. } => {
            Err("collection_render_sequence_unsupported")
        }
    }
}

fn apply_value_renderer(
    provider_payload: &Value,
    computed: String,
    renderer: &CollectionOutputRenderer,
) -> Result<String, &'static str> {
    let CollectionOutputRenderer::RenderSequence { segments } = renderer else {
        return apply_output_renderer(computed, renderer);
    };
    let mut output = String::new();
    for segment in segments {
        match segment {
            ResponseRenderSegment::Static { text } => output.push_str(text),
            ResponseRenderSegment::Primary => output.push_str(&computed),
            ResponseRenderSegment::Selected { selector, format } => output.push_str(
                &project_selected_value(provider_payload, selector, *format)?,
            ),
        }
        if output.len() > 16_384 {
            return Err("projection_output_budget");
        }
    }
    Ok(output)
}

fn collection_json_from_value(output: &Value) -> Result<Value, &'static str> {
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
                    return Err("collection_output_part_type");
                }
                texts.push(
                    part.get("text")
                        .and_then(Value::as_str)
                        .ok_or("collection_output_part_text")?,
                );
            }
        }
        _ => return Err("collection_output_not_text"),
    }
    let mut candidates = BTreeMap::<Vec<u8>, Value>::new();
    for text in texts {
        total_bytes = total_bytes
            .checked_add(text.len())
            .ok_or("collection_input_budget")?;
        if text.is_empty() || total_bytes > 65_536 {
            return Err("collection_input_budget");
        }
        collect_runtime_collection_candidates(text, &mut candidates)?;
    }
    if candidates.len() != 1 {
        return Err(if candidates.is_empty() {
            "collection_input_not_json"
        } else {
            "collection_input_ambiguous"
        });
    }
    Ok(candidates.into_values().next().expect("one candidate"))
}

fn collect_runtime_collection_candidates(
    output: &str,
    candidates: &mut BTreeMap<Vec<u8>, Value>,
) -> Result<(), &'static str> {
    let mut sources = vec![output.to_owned()];
    let mut fenced = None::<String>;
    for line in output.lines() {
        let trimmed = line.trim();
        if fenced.is_some() && trimmed == "```" {
            sources.push(fenced.take().unwrap_or_default());
        } else if fenced.is_some() {
            let buffer = fenced.as_mut().expect("checked above");
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
        if source.is_empty() || source.len() > 16_384 {
            continue;
        }
        for object in runtime_embedded_json_objects(&source) {
            let value = Value::Object(object);
            if bounded_collection_root(&value) {
                let key = serde_json::to_vec(&value).map_err(|_| "collection_serialization")?;
                candidates.insert(key, value);
            }
        }
        if let Ok(value @ Value::Array(_)) = serde_json::from_str::<Value>(&source)
            && !is_text_part_array(&value)
        {
            let value = serde_json::json!({"items": value});
            if bounded_collection_root(&value) {
                let key = serde_json::to_vec(&value).map_err(|_| "collection_serialization")?;
                candidates.insert(key, value);
            }
        }
    }
    Ok(())
}

fn is_text_part_array(value: &Value) -> bool {
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

fn bounded_collection_root(value: &Value) -> bool {
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
                    safe_collection_identifier(name) && safe_collection_scalar(value)
                })
        })
    })
}

fn safe_collection_identifier(value: &str) -> bool {
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

fn safe_collection_scalar(value: &Value) -> bool {
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

fn collection_scalar_type(value: &Value) -> Option<CollectionScalarType> {
    match value {
        Value::String(_) => Some(CollectionScalarType::String),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Some(CollectionScalarType::Integer)
        }
        Value::Bool(_) => Some(CollectionScalarType::Boolean),
        Value::Null | Value::Array(_) | Value::Object(_) | Value::Number(_) => None,
    }
}

pub(crate) fn project_selected_value(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
    format: ValueProjectionFormat,
) -> Result<String, &'static str> {
    let selected = immediate_selected_scalar(provider_payload, selector)?;
    let projected = match format {
        ValueProjectionFormat::PlainText => match &selected.value {
            Value::String(text) if !text.contains(['\n', '\r']) => text.clone(),
            Value::String(_) => return Err("projection_multiline"),
            Value::Bool(_) | Value::Number(_) => selected.value.to_string(),
            _ => return Err("projection_non_scalar"),
        },
        ValueProjectionFormat::CanonicalJson => {
            serde_json::to_string(&selected.value).map_err(|_| "projection_serialization")?
        }
    };
    if projected.is_empty() || projected.len() > 16_384 {
        return Err("projection_output_budget");
    }
    Ok(projected)
}

pub(crate) fn project_status(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
    mapping: ProjectStatusMapping,
) -> Result<String, &'static str> {
    let selected = immediate_selected_scalar(provider_payload, selector)?;
    if selected.value_type != AtomValueType::Integer {
        return Err("status_selector_type_mismatch");
    }
    let code = selected
        .value
        .as_u64()
        .filter(|code| *code <= MAX_PROJECT_STATUS_CODE)
        .ok_or("status_integer_out_of_bounds")?;
    let status = match mapping {
        ProjectStatusMapping::ZeroIsSuccess if code == 0 => "success",
        ProjectStatusMapping::ZeroIsSuccess => "failure",
        ProjectStatusMapping::ZeroIsPass if code == 0 => "PASS",
        ProjectStatusMapping::ZeroIsPass => "FAIL",
        ProjectStatusMapping::ZeroIsOk if code == 0 => "OK",
        ProjectStatusMapping::ZeroIsOk => "ERROR",
        ProjectStatusMapping::ZeroIsTrue if code == 0 => "true",
        ProjectStatusMapping::ZeroIsTrue => "false",
    };
    Ok(status.to_owned())
}

fn execute_function_call_from_roles(
    provider_payload: &Value,
    function_name: &str,
    selector: &ResponseValueSelector,
    arguments: &[ResponseArgument],
) -> Result<String, &'static str> {
    let scalar = immediate_selected_scalar(provider_payload, selector)?;
    let mut projected = serde_json::Map::new();
    for argument in arguments {
        match argument {
            ResponseArgument::Role {
                name,
                role: SemanticRole::ContinuationHandle | SemanticRole::SourceValue,
                value_type,
            } => {
                projected.insert(
                    name.clone(),
                    runtime_role_value(&scalar.value, *value_type)?,
                );
            }
            ResponseArgument::Role { .. } => return Err("unsupported_runtime_role"),
            ResponseArgument::Integer { name, value } => {
                projected.insert(name.clone(), Value::from(*value));
            }
            ResponseArgument::String { name, value } => {
                projected.insert(name.clone(), Value::String(value.clone()));
            }
            ResponseArgument::Boolean { name, value } => {
                projected.insert(name.clone(), Value::Bool(*value));
            }
        }
    }
    serde_json::to_string(&serde_json::json!({
        "name": function_name,
        "arguments": projected,
    }))
    .map_err(|_| "function_call_serialization")
}

fn execute_custom_tool_call_from_roles(
    provider_payload: &Value,
    custom_tool_name: &str,
    inner_tool_name: &str,
    selector: &ResponseValueSelector,
    arguments: &[ResponseArgument],
    projection: &CustomToolResultProjection,
) -> Result<String, &'static str> {
    let selected = immediate_selected_scalar(provider_payload, selector)?;
    let projected = project_arguments(arguments, &selected)?;
    let arguments_json =
        serde_json::to_string(&projected).map_err(|_| "custom_tool_arguments_serialization")?;
    let source = match projection {
        CustomToolResultProjection::OutputField { output_field } => format!(
            "const r=await tools.{inner_tool_name}({arguments_json});text(r.{output_field});"
        ),
        CustomToolResultProjection::OutputAndContinuation {
            output_field,
            continuation_field,
            continuation_prefix,
        } => {
            let prefix = serde_json::to_string(continuation_prefix)
                .map_err(|_| "custom_tool_prefix_serialization")?;
            format!(
                "const r=await tools.{inner_tool_name}({arguments_json});text(r.{output_field});if(r.{continuation_field})text({prefix}+r.{continuation_field});"
            )
        }
        CustomToolResultProjection::JsonStringifyResult => format!(
            "const r=await tools.{inner_tool_name}({arguments_json});text(JSON.stringify(r));"
        ),
    };
    serde_json::to_string(&serde_json::json!({
        "kind": "custom_tool_call",
        "name": custom_tool_name,
        "input": source,
    }))
    .map_err(|_| "custom_tool_call_serialization")
}

fn project_arguments(
    arguments: &[ResponseArgument],
    selected: &ExtractedScalar,
) -> Result<serde_json::Map<String, Value>, &'static str> {
    let mut projected = serde_json::Map::new();
    for argument in arguments {
        match argument {
            ResponseArgument::Role {
                name,
                role: SemanticRole::ContinuationHandle | SemanticRole::SourceValue,
                value_type,
            } => {
                projected.insert(
                    name.clone(),
                    runtime_role_value(&selected.value, *value_type)?,
                );
            }
            ResponseArgument::Role { .. } => return Err("unsupported_runtime_role"),
            ResponseArgument::Integer { name, value } => {
                projected.insert(name.clone(), Value::from(*value));
            }
            ResponseArgument::String { name, value } => {
                projected.insert(name.clone(), Value::String(value.clone()));
            }
            ResponseArgument::Boolean { name, value } => {
                projected.insert(name.clone(), Value::Bool(*value));
            }
        }
    }
    Ok(projected)
}

fn runtime_role_value(
    value: &Value,
    value_type: Option<AtomValueType>,
) -> Result<Value, &'static str> {
    match value_type {
        None => Ok(value.clone()),
        Some(AtomValueType::Integer) => value
            .as_u64()
            .or_else(|| value.as_str()?.parse::<u64>().ok())
            .map(Value::from)
            .ok_or("role_integer_parse"),
        Some(AtomValueType::Boolean) => value
            .as_bool()
            .or_else(|| value.as_str()?.parse::<bool>().ok())
            .map(Value::from)
            .ok_or("role_boolean_parse"),
        Some(AtomValueType::String | AtomValueType::Identifier) => value
            .as_str()
            .map(|value| Value::String(value.to_owned()))
            .ok_or("role_string_parse"),
        Some(AtomValueType::Collection) => Err("role_collection_unsupported"),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ExtractedScalar {
    pub value: Value,
    pub value_type: AtomValueType,
}

pub(crate) fn immediate_unique_scalar(
    provider_payload: &Value,
) -> Result<ExtractedScalar, &'static str> {
    let output = immediate_function_output(provider_payload)
        .ok_or("immediate_tool_output_missing")?
        .trim();
    if output.is_empty() || output.len() > 16_384 {
        return Err("scalar_output_budget");
    }
    let value = serde_json::from_str::<Value>(output).unwrap_or_else(|_| {
        if output.contains('\n') || output.contains('\r') {
            Value::Null
        } else {
            Value::String(output.to_owned())
        }
    });
    let mut scalars = Vec::new();
    collect_scalar_values(&value, 0, &mut scalars)?;
    if scalars.len() != 1 {
        return Err("unique_scalar_missing");
    }
    scalars.pop().ok_or("unique_scalar_missing")
}

pub(crate) fn immediate_selected_scalar(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
) -> Result<ExtractedScalar, &'static str> {
    match selector {
        ResponseValueSelector::UniqueScalar { value_type } => {
            let scalar = immediate_unique_scalar(provider_payload)?;
            (scalar.value_type == *value_type)
                .then_some(scalar)
                .ok_or("selector_type_mismatch")
        }
        ResponseValueSelector::UniqueTurnScalar { value_type } => {
            unique_turn_scalar(provider_payload, *value_type)
        }
        ResponseValueSelector::ContentLinePrefix { prefix, value_type } => {
            let output = immediate_tool_output_value(provider_payload)
                .ok_or("immediate_tool_output_missing")?;
            let mut matches = output_text_parts(output)?
                .into_iter()
                .flat_map(str::lines)
                .filter_map(|line| line.trim().strip_prefix(prefix))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| parse_scalar_text(value, *value_type))
                .collect::<Result<Vec<_>, _>>()?;
            if matches.len() != 1 {
                return Err("selector_prefix_ambiguous");
            }
            matches.pop().ok_or("selector_prefix_missing")
        }
        ResponseValueSelector::JsonField { field, value_type } => {
            let output = immediate_tool_output_value(provider_payload)
                .ok_or("immediate_tool_output_missing")?;
            let mut matches = Vec::new();
            for text in output_text_parts(output)? {
                for object in runtime_embedded_json_objects(text) {
                    collect_runtime_json_field(
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
                return Err("selector_field_ambiguous");
            }
            matches.pop().ok_or("selector_field_missing")
        }
        ResponseValueSelector::JsonScalarOrdinal {
            ordinal,
            value_type,
        } => {
            let output = immediate_tool_output_value(provider_payload)
                .ok_or("immediate_tool_output_missing")?;
            let mut matches = Vec::new();
            for text in output_text_parts(output)? {
                for object in runtime_embedded_json_objects(text) {
                    collect_runtime_json_scalars(
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
                .ok_or("selector_scalar_ordinal_missing")
        }
        ResponseValueSelector::UniqueTurnJsonField { field, value_type } => {
            unique_turn_json_field(provider_payload, field, *value_type)
        }
        ResponseValueSelector::UniqueActiveTurnJsonField { field, value_type } => {
            unique_active_turn_json_field(provider_payload, field, *value_type)
        }
        ResponseValueSelector::TurnOutputLine {
            output_ordinal,
            line_index,
            value_type,
        } => turn_output_line(provider_payload, *output_ordinal, *line_index, *value_type),
        ResponseValueSelector::CommandOutputBody => {
            let value = command_output_body(provider_payload)?;
            Ok(ExtractedScalar {
                value: Value::String(value),
                value_type: AtomValueType::String,
            })
        }
        ResponseValueSelector::RequestLastToken => {
            let value = request_last_token(provider_payload)?;
            Ok(ExtractedScalar {
                value: Value::String(value),
                value_type: AtomValueType::String,
            })
        }
        ResponseValueSelector::RequestUniqueLiteral => {
            let value = request_unique_literal(provider_payload)?;
            Ok(ExtractedScalar {
                value: Value::String(value),
                value_type: AtomValueType::String,
            })
        }
    }
}

fn unique_turn_scalar(
    provider_payload: &Value,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    let items = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or("turn_input_missing")?;
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
        for text in output_text_parts(output)? {
            let mut parsed_values = Vec::new();
            if let Ok(value) = serde_json::from_str::<Value>(text) {
                parsed_values.push(value);
            } else {
                parsed_values.extend(
                    runtime_embedded_json_objects(text)
                        .into_iter()
                        .map(Value::Object),
                );
            }
            for value in parsed_values {
                let mut scalars = Vec::new();
                collect_scalar_values(&value, 0, &mut scalars)?;
                matches.extend(scalars.into_iter().filter(|scalar| {
                    scalar.value_type == value_type
                        || matches!(
                            (scalar.value_type, value_type),
                            (AtomValueType::Identifier, AtomValueType::String)
                        )
                }));
            }
        }
    }
    matches.sort_by_cached_key(|scalar| scalar.value.to_string());
    matches.dedup();
    if matches.len() != 1 {
        return Err("selector_turn_scalar_ambiguous");
    }
    matches
        .pop()
        .map(|mut scalar| {
            scalar.value_type = value_type;
            scalar
        })
        .ok_or("selector_turn_scalar_missing")
}

fn collect_runtime_json_scalars(
    value: &Value,
    value_type: AtomValueType,
    depth: usize,
    output: &mut Vec<ExtractedScalar>,
) -> Result<(), &'static str> {
    if depth > 8 || output.len() >= 64 {
        return Err("selector_scalar_ordinal_structure_budget");
    }
    if let Ok(scalar) = extracted_scalar(value.clone(), value_type) {
        output.push(scalar);
        return Ok(());
    }
    match value {
        Value::Object(object) => {
            for value in object.values() {
                collect_runtime_json_scalars(value, value_type, depth.saturating_add(1), output)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_runtime_json_scalars(value, value_type, depth.saturating_add(1), output)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn collect_runtime_json_field(
    value: &Value,
    field: &str,
    value_type: AtomValueType,
    depth: usize,
    output: &mut Vec<ExtractedScalar>,
) -> Result<(), &'static str> {
    if depth > 8 || output.len() >= 64 {
        return Err("selector_field_structure_budget");
    }
    match value {
        Value::Object(object) => {
            for (name, value) in object {
                if name == field {
                    output.push(extracted_scalar(value.clone(), value_type)?);
                }
                collect_runtime_json_field(
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
                collect_runtime_json_field(
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

fn request_unique_literal(provider_payload: &Value) -> Result<String, &'static str> {
    let request = request_text(provider_payload).ok_or("request_text_missing")?;
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
        return Err("request_unique_literal_ambiguous");
    }
    values
        .into_keys()
        .next()
        .ok_or("request_unique_literal_missing")
}

fn request_last_token(provider_payload: &Value) -> Result<String, &'static str> {
    let request = request_text(provider_payload).ok_or("request_text_missing")?;
    let token = request
        .split_whitespace()
        .next_back()
        .map(|value| {
            value.trim_matches(|character: char| {
                !character.is_alphanumeric() && character != '_' && character != '-'
            })
        })
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .ok_or("request_last_token_missing")?;
    Ok(token.to_owned())
}

fn command_output_body(provider_payload: &Value) -> Result<String, &'static str> {
    let output =
        immediate_tool_output_value(provider_payload).ok_or("immediate_tool_output_missing")?;
    let mut body = String::new();
    let mut after_marker = false;
    for part in command_output_text_parts(output)? {
        if !after_marker {
            let Some((_, suffix)) = part.split_once("\nOutput:\n") else {
                continue;
            };
            after_marker = true;
            body.push_str(suffix);
        } else {
            body.push_str(&part);
        }
        if body.len() > 16_384 {
            return Err("command_output_body_budget");
        }
    }
    let body = body.trim_end_matches(['\n', '\r']).to_owned();
    if !after_marker || body.is_empty() {
        Err("command_output_body_missing")
    } else {
        Ok(body)
    }
}

fn command_output_text_parts(output: &Value) -> Result<Vec<String>, &'static str> {
    if let Some(text) = output.as_str()
        && let Ok(parsed) = serde_json::from_str::<Value>(text)
        && parsed.is_array()
    {
        return output_text_parts(&parsed)
            .map(|parts| parts.into_iter().map(str::to_owned).collect());
    }
    output_text_parts(output).map(|parts| parts.into_iter().map(str::to_owned).collect())
}

fn runtime_embedded_json_objects(text: &str) -> Vec<serde_json::Map<String, Value>> {
    runtime_embedded_json_objects_at_depth(text, 0)
}

fn runtime_embedded_json_objects_at_depth(
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
        runtime_collect_json_objects(&value, &mut candidates, depth);
    }
    candidates.into_values().collect()
}

fn runtime_collect_json_objects(
    value: &Value,
    output: &mut BTreeMap<Vec<u8>, serde_json::Map<String, Value>>,
    depth: usize,
) {
    match value {
        Value::Object(object) => {
            let mut encoded_children = BTreeMap::new();
            if depth < 4 {
                for text in object.values().filter_map(Value::as_str) {
                    for child in runtime_embedded_json_objects_at_depth(text, depth + 1) {
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
                for object in runtime_embedded_json_objects_at_depth(text, depth + 1) {
                    if let Ok(key) = serde_json::to_vec(&object) {
                        output.insert(key, object);
                    }
                }
            }
        }
        _ => {}
    }
}

fn turn_output_line(
    provider_payload: &Value,
    output_ordinal: u16,
    line_index: u16,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    if value_type != AtomValueType::String || output_ordinal == 0 {
        return Err("turn_output_line_selector_invalid");
    }
    let items = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or("turn_input_missing")?;
    let turn_start = items
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    let item = items[turn_start..]
        .iter()
        .filter(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })
        .nth(usize::from(output_ordinal - 1))
        .ok_or("turn_output_ordinal_missing")?;
    let output = item.get("output").ok_or("turn_output_missing")?;
    let lines = output_text_parts(output)?
        .into_iter()
        .flat_map(str::lines)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let line = lines
        .get(usize::from(line_index))
        .filter(|line| line.len() <= 512)
        .ok_or("turn_output_line_missing")?;
    extracted_scalar(Value::String((*line).to_owned()), AtomValueType::String)
}

fn unique_turn_json_field(
    provider_payload: &Value,
    field: &str,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    let items = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or("turn_input_missing")?;
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
        for text in output_text_parts(output)? {
            let Ok(Value::Object(object)) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            if let Some(value) = object.get(field) {
                let scalar = extracted_scalar(value.clone(), value_type)?;
                if !matches.contains(&scalar) {
                    matches.push(scalar);
                }
            }
        }
    }
    if matches.len() != 1 {
        return Err("selector_turn_field_ambiguous");
    }
    matches.pop().ok_or("selector_turn_field_missing")
}

fn unique_active_turn_json_field(
    provider_payload: &Value,
    field: &str,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    let items = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or("turn_input_missing")?;
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
        for text in output_text_parts(output)? {
            let Ok(Value::Object(object)) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            completed |= object.get("exit_code").is_some_and(Value::is_i64);
            if let Some(value) = object.get(field) {
                observed.push(extracted_scalar(value.clone(), value_type)?);
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
        return Err("selector_active_turn_field_ambiguous");
    }
    active.pop().ok_or("selector_active_turn_field_missing")
}

fn immediate_tool_output_value(payload: &Value) -> Option<&Value> {
    let item = payload.get("input")?.as_array()?.last()?;
    matches!(
        item.get("type").and_then(Value::as_str),
        Some("function_call_output" | "custom_tool_call_output")
    )
    .then(|| item.get("output"))?
}

fn output_text_parts(output: &Value) -> Result<Vec<&str>, &'static str> {
    if let Some(text) = output.as_str() {
        return if text.is_empty() || text.len() > 16_384 {
            Err("scalar_output_budget")
        } else {
            Ok(vec![text])
        };
    }
    let parts = output.as_array().ok_or("tool_output_not_text")?;
    if parts.is_empty() || parts.len() > 64 {
        return Err("output_part_cardinality");
    }
    let mut texts = Vec::with_capacity(parts.len());
    let mut total_bytes = 0_usize;
    for part in parts {
        if !matches!(
            part.get("type").and_then(Value::as_str),
            Some("text" | "input_text" | "output_text")
        ) {
            return Err("unsupported_output_part_type");
        }
        let text = part
            .get("text")
            .and_then(Value::as_str)
            .ok_or("output_part_text_missing")?;
        total_bytes = total_bytes
            .checked_add(text.len())
            .ok_or("scalar_output_budget")?;
        texts.push(text);
    }
    if total_bytes == 0 || total_bytes > 16_384 {
        return Err("scalar_output_budget");
    }
    Ok(texts)
}

fn parse_scalar_text(
    value: &str,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    let parsed = match value_type {
        AtomValueType::Integer => value
            .parse::<u64>()
            .map(Value::from)
            .map_err(|_| "selector_integer_parse")?,
        AtomValueType::Boolean => value
            .parse::<bool>()
            .map(Value::from)
            .map_err(|_| "selector_boolean_parse")?,
        AtomValueType::String | AtomValueType::Identifier => Value::String(value.to_owned()),
        AtomValueType::Collection => return Err("selector_collection_unsupported"),
    };
    extracted_scalar(parsed, value_type)
}

fn extracted_scalar(
    value: Value,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    let actual = match &value {
        Value::Bool(_) => AtomValueType::Boolean,
        Value::Number(number) if number.is_i64() || number.is_u64() => AtomValueType::Integer,
        Value::String(text) if identifier_like(text) => AtomValueType::Identifier,
        Value::String(_) => AtomValueType::String,
        _ => return Err("selector_scalar_unsupported"),
    };
    let compatible = actual == value_type
        || matches!(
            (actual, value_type),
            (AtomValueType::Identifier, AtomValueType::String)
        );
    compatible
        .then_some(ExtractedScalar { value, value_type })
        .ok_or("selector_type_mismatch")
}

fn collect_scalar_values(
    value: &Value,
    depth: usize,
    scalars: &mut Vec<ExtractedScalar>,
) -> Result<(), &'static str> {
    if depth > 8 || scalars.len() >= 64 {
        return Err("scalar_structure_budget");
    }
    match value {
        Value::Null => {}
        Value::Bool(_) => scalars.push(ExtractedScalar {
            value: value.clone(),
            value_type: AtomValueType::Boolean,
        }),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            scalars.push(ExtractedScalar {
                value: value.clone(),
                value_type: AtomValueType::Integer,
            });
        }
        Value::Number(_) => return Err("unsupported_scalar_number"),
        Value::String(text) => scalars.push(ExtractedScalar {
            value: value.clone(),
            value_type: if identifier_like(text) {
                AtomValueType::Identifier
            } else {
                AtomValueType::String
            },
        }),
        Value::Array(values) => {
            for value in values {
                collect_scalar_values(value, depth + 1, scalars)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_scalar_values(value, depth + 1, scalars)?;
            }
        }
    }
    Ok(())
}

fn identifier_like(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/')
        })
}

fn execute_wait_surface(
    provider_payload: &Value,
    surfaces: &[String],
    function_name: &str,
    yield_time_ms: u64,
    max_tokens: u64,
) -> Result<String, &'static str> {
    let surface = immediate_yielded_surface(provider_payload).ok_or("yielded_surface_missing")?;
    if !surfaces.iter().any(|allowed| allowed == surface) {
        return Err("yielded_surface_guard_mismatch");
    }
    execute_wait_any(provider_payload, function_name, yield_time_ms, max_tokens)
}

fn execute_wait_any(
    provider_payload: &Value,
    function_name: &str,
    yield_time_ms: u64,
    max_tokens: u64,
) -> Result<String, &'static str> {
    let output =
        immediate_function_output(provider_payload).ok_or("immediate_tool_output_missing")?;
    let cell_id = yielded_cell_id(output)?;
    serde_json::to_string(&serde_json::json!({
        "name": function_name,
        "arguments": {
            "cell_id": cell_id,
            "yield_time_ms": yield_time_ms,
            "max_tokens": max_tokens,
        }
    }))
    .map_err(|_| "wait_serialization")
}

pub(crate) fn yielded_cell_id(output: &str) -> Result<&str, &'static str> {
    let tail = output
        .strip_prefix("Script running with cell ID ")
        .ok_or("running_cell_marker_missing")?;
    let cell_id = tail.split_whitespace().next().ok_or("cell_id_missing")?;
    if cell_id.is_empty()
        || !cell_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err("invalid_cell_id");
    }
    Ok(cell_id)
}

fn execute_wait(
    provider_payload: &Value,
    function_name: &str,
    yield_time_ms: u64,
    max_tokens: u64,
) -> Result<String, &'static str> {
    let output =
        immediate_function_output(provider_payload).ok_or("immediate_tool_output_missing")?;
    if !immediate_yielded_build_or_test(provider_payload) {
        return Err("build_or_test_guard_mismatch");
    }
    let cell_id = yielded_cell_id(output)?;
    serde_json::to_string(&serde_json::json!({
        "name": function_name,
        "arguments": {
            "cell_id": cell_id,
            "yield_time_ms": yield_time_ms,
            "max_tokens": max_tokens,
        }
    }))
    .map_err(|_| "wait_serialization")
}

pub(crate) fn immediate_function_output(payload: &Value) -> Option<&str> {
    let item = payload.get("input")?.as_array()?.last()?;
    matches!(
        item.get("type").and_then(Value::as_str),
        Some("function_call_output" | "custom_tool_call_output")
    )
    .then(|| item.get("output").and_then(Value::as_str))?
}

pub(crate) fn immediate_yielded_build_or_test(payload: &Value) -> bool {
    immediate_yielded_source(payload).is_some_and(|source| build_or_test_command(&source))
}

pub(crate) fn immediate_yielded_surface(payload: &Value) -> Option<&'static str> {
    immediate_yielded_source(payload).map(|source| classify_yielded_surface(&source))
}

fn immediate_yielded_source(payload: &Value) -> Option<String> {
    let items = payload.get("input").and_then(Value::as_array)?;
    let output = items.last()?;
    let call_id = output.get("call_id").and_then(Value::as_str)?;
    items[..items.len().saturating_sub(1)]
        .iter()
        .rev()
        .find(|item| item.get("call_id").and_then(Value::as_str) == Some(call_id))
        .and_then(|item| item.get("arguments").or_else(|| item.get("input")))
        .map(|value| {
            value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned)
        })
}

pub(crate) fn classify_yielded_surface(source: &str) -> &'static str {
    let lower = source.to_ascii_lowercase();
    if build_or_test_command(source) {
        return "build_or_test";
    }
    if lower.contains("nando-live-transition-gate") {
        return "live_transition_gate";
    }
    if lower.contains("systemctl ") || lower.contains("journalctl ") {
        return "service_observation";
    }
    if lower.contains("curl ") || lower.contains("wget ") || lower.contains("ping ") {
        return "network_observation";
    }
    if lower.contains("git ") {
        return "version_control";
    }
    if lower.contains("python") {
        return "python_batch";
    }
    if lower.contains("nando-") {
        return "nando_ops";
    }
    if lower.contains("nginx") {
        return "nginx_ops";
    }
    if lower.contains("sleep ") || lower.contains("timeout ") {
        return "timed_wait";
    }
    if ["install ", "mkdir ", " cp ", " mv ", "chmod ", "chown "]
        .iter()
        .any(|term| lower.contains(term))
    {
        return "filesystem_mutation";
    }
    if ["tar ", "gzip ", "zstd ", "xz "]
        .iter()
        .any(|term| lower.contains(term))
    {
        return "archive_batch";
    }
    if lower.contains("sha256sum") || lower.contains("b2sum") {
        return "checksum_batch";
    }
    if lower.contains("ps ") || lower.contains("ss ") || lower.contains("lsof ") {
        return "process_observation";
    }
    if ["rg ", "find ", "sed ", "jq ", "ls "]
        .iter()
        .any(|tool| lower.contains(tool))
    {
        return "filesystem_observation";
    }
    if lower.contains("&&") || lower.contains(';') || lower.contains("set -") {
        return "shell_batch";
    }
    "generic_long_command"
}

pub(crate) fn build_or_test_command(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    ((lower.contains("cargo ")
        || lower.contains("pytest")
        || lower.contains("unittest")
        || lower.contains("npm test")
        || lower.contains("pnpm test"))
        && ["test", "build", "check", "clippy", "bench"]
            .iter()
            .any(|term| lower.contains(term)))
        || lower.contains("graphify update")
        || lower.contains("rust-action-memory")
        || (lower.contains("find ") && lower.contains("xargs"))
        || lower.contains("apt-get ")
}

fn execute_copy(
    request_text: &str,
    prefixes: &[String],
    trim: bool,
    allow_multiline: bool,
) -> Result<String, &'static str> {
    let request = request_text.trim_start();
    let lower = request.to_ascii_lowercase();
    let mut matches = prefixes.iter().filter_map(|prefix| {
        let normalized = prefix.to_ascii_lowercase();
        lower.starts_with(&normalized).then_some(normalized.len())
    });
    let Some(offset) = matches.next() else {
        return Err("prefix_mismatch");
    };
    if matches.next().is_some() {
        return Err("ambiguous_prefix");
    }
    let mut output = request.get(offset..).ok_or("prefix_boundary")?;
    if trim {
        output = output.trim();
    }
    if output.is_empty() {
        return Err("empty_capture");
    }
    if !allow_multiline && (output.contains('\n') || output.contains('\r')) {
        return Err("multiline_capture");
    }
    Ok(output.to_owned())
}

fn execute_test_summary(
    request_text: &str,
    provider_payload: &Value,
    required: &[String],
    forbidden: &[String],
) -> Result<String, &'static str> {
    let request = request_text.to_ascii_lowercase();
    if !required
        .iter()
        .any(|phrase| request.contains(&phrase.to_ascii_lowercase()))
    {
        return Err("test_intent_missing");
    }
    if forbidden
        .iter()
        .any(|term| request.contains(&term.to_ascii_lowercase()))
    {
        return Err("broad_intent");
    }
    let output = latest_function_output_text(provider_payload).ok_or("tool_output_missing")?;
    classify_test_output(&output).map(str::to_owned)
}

pub(crate) fn latest_function_output_text(payload: &Value) -> Option<String> {
    let output = payload
        .get("input")?
        .as_array()?
        .iter()
        .rev()
        .find(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })?
        .get("output")?;
    if let Some(text) = output.as_str() {
        return Some(text.to_owned());
    }
    output_text_parts(output).ok().map(|parts| parts.join("\n"))
}

pub(crate) fn classify_test_output(output: &str) -> Result<&'static str, &'static str> {
    let lower = output.to_ascii_lowercase();
    if lower.contains("error[e") || lower.contains("could not compile") {
        Ok("Tests did not run because compilation failed.")
    } else if lower.contains("panicked at") || lower.contains("thread 'main' panicked") {
        Ok("Tests failed with a runtime panic.")
    } else if lower.contains("test result: failed")
        || lower.contains("failures:")
        || lower.contains(" ... failed")
    {
        Ok("Tests failed.")
    } else if lower.contains("test result: ok")
        || (lower.contains("0 failed") && lower.contains("passed"))
        || lower.contains("process exited with code 0")
        || lower.contains("\"exit_code\":0")
    {
        Ok("Validation passed.")
    } else {
        Err("test_result_ambiguous")
    }
}
