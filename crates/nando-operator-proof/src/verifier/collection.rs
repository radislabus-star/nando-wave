//! Independent proof execution for bounded collection programs.
//!
//! This owner intentionally re-derives collection results instead of calling the actor runtime.

use super::input::independently_request_text;
use super::selection::{
    independent_collection_scalar_type, independently_active_turn_output_value,
    independently_format_selected_value, independently_latest_tool_output,
    independently_parse_collection_value, independently_select_scalar,
    independently_select_scalar_with_request,
};
use super::*;

pub(super) fn independently_execute_collection(
    provider_payload: &Value,
    steps: &[CollectionProgramStep],
    format: ValueProjectionFormat,
    renderer: &CollectionOutputRenderer,
    max_items: usize,
) -> Result<String, ResponseVerificationError> {
    let has_explicit_source = matches!(
        steps.first(),
        Some(CollectionProgramStep::SelectTurnOutput { .. })
    );
    if steps.is_empty()
        || steps.len().saturating_sub(usize::from(has_explicit_source)) > 8
        || max_items == 0
        || max_items > 4_096
    {
        return Err(ResponseVerificationError("collection_program_budget"));
    }
    if !independently_safe_collection_renderer(renderer) {
        return Err(ResponseVerificationError("collection_renderer_unsafe"));
    }
    let (output, transform_steps) = match steps.first() {
        Some(CollectionProgramStep::SelectTurnOutput { output_ordinal }) => (
            independently_active_turn_output_value(provider_payload, Some(*output_ordinal))?,
            &steps[1..],
        ),
        _ => (independently_latest_tool_output(provider_payload)?, steps),
    };
    let mut current = independently_parse_collection_value(output)?;
    let mut filter_field = None::<String>;
    for step in transform_steps {
        current = match step {
            CollectionProgramStep::SelectTurnOutput { .. } => {
                return Err(ResponseVerificationError(
                    "collection_output_selector_position",
                ));
            }
            CollectionProgramStep::SelectOnlyArrayField => {
                let object = current
                    .as_object()
                    .ok_or(ResponseVerificationError("collection_select_not_object"))?;
                let mut arrays = object.values().filter(|candidate| candidate.is_array());
                let selected = arrays
                    .next()
                    .cloned()
                    .ok_or(ResponseVerificationError("collection_select_missing"))?;
                if arrays.next().is_some() {
                    return Err(ResponseVerificationError("collection_select_ambiguous"));
                }
                selected
            }
            CollectionProgramStep::SelectField { field } => current
                .as_object()
                .and_then(|object| object.get(field))
                .cloned()
                .ok_or(ResponseVerificationError("collection_select_missing"))?,
            CollectionProgramStep::FilterFieldEquals { field, value } => {
                let rows = current
                    .as_array()
                    .ok_or(ResponseVerificationError("collection_filter_not_array"))?;
                if rows.len() > max_items {
                    return Err(ResponseVerificationError("collection_item_budget"));
                }
                let expected = value.as_json();
                filter_field = Some(field.clone());
                Value::Array(
                    rows.iter()
                        .filter(|row| {
                            row.as_object().and_then(|object| object.get(field)) == Some(&expected)
                        })
                        .cloned()
                        .collect(),
                )
            }
            CollectionProgramStep::FilterUniqueFieldEquals { value } => {
                let rows = current
                    .as_array()
                    .ok_or(ResponseVerificationError("collection_filter_not_array"))?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err(ResponseVerificationError("collection_item_budget"));
                }
                let expected = value.as_json();
                let first = rows[0].as_object().ok_or(ResponseVerificationError(
                    "collection_filter_row_not_object",
                ))?;
                let mut fields = first.keys().filter(|field| {
                    rows.iter().all(|row| {
                        row.as_object()
                            .is_some_and(|object| object.contains_key(*field))
                    }) && rows.iter().any(|row| row.get(*field) == Some(&expected))
                });
                let field = fields
                    .next()
                    .cloned()
                    .ok_or(ResponseVerificationError("collection_filter_field_missing"))?;
                if fields.next().is_some() {
                    return Err(ResponseVerificationError(
                        "collection_filter_field_ambiguous",
                    ));
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
                let rows = current
                    .as_array()
                    .ok_or(ResponseVerificationError("collection_filter_not_array"))?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err(ResponseVerificationError("collection_item_budget"));
                }
                let (field, expected) =
                    independently_request_collection_value(provider_payload, rows, *value_type)?;
                filter_field = Some(field.clone());
                Value::Array(
                    rows.iter()
                        .filter(|row| row.get(&field) == Some(&expected))
                        .cloned()
                        .collect(),
                )
            }
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                selector,
                value_type,
            } => {
                let rows = current
                    .as_array()
                    .ok_or(ResponseVerificationError("collection_filter_not_array"))?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err(ResponseVerificationError("collection_item_budget"));
                }
                let selected = independently_select_scalar(provider_payload, selector)?;
                if independently_collection_atom_type(selected.value_type) != Some(*value_type) {
                    return Err(ResponseVerificationError("collection_filter_value_type"));
                }
                let expected = selected.value;
                let first = rows[0].as_object().ok_or(ResponseVerificationError(
                    "collection_filter_row_not_object",
                ))?;
                let mut fields = first.keys().filter(|field| {
                    rows.iter().all(|row| {
                        row.as_object()
                            .is_some_and(|object| object.contains_key(*field))
                    }) && rows.iter().any(|row| row.get(*field) == Some(&expected))
                });
                let field = fields
                    .next()
                    .cloned()
                    .ok_or(ResponseVerificationError("collection_filter_field_missing"))?;
                if fields.next().is_some() {
                    return Err(ResponseVerificationError(
                        "collection_filter_field_ambiguous",
                    ));
                }
                filter_field = Some(field.clone());
                Value::Array(
                    rows.iter()
                        .filter(|row| row.get(&field) == Some(&expected))
                        .cloned()
                        .collect(),
                )
            }
            CollectionProgramStep::ProjectField { field } => {
                if let Some(rows) = current.as_array() {
                    if rows.len() > max_items {
                        return Err(ResponseVerificationError("collection_item_budget"));
                    }
                    Value::Array(
                        rows.iter()
                            .map(|row| {
                                row.as_object()
                                    .and_then(|object| object.get(field))
                                    .cloned()
                                    .ok_or(ResponseVerificationError("collection_project_missing"))
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    )
                } else {
                    current
                        .as_object()
                        .and_then(|object| object.get(field))
                        .cloned()
                        .ok_or(ResponseVerificationError("collection_project_missing"))?
                }
            }
            CollectionProgramStep::ProjectUniqueFieldByType { value_type } => {
                let rows = current
                    .as_array()
                    .ok_or(ResponseVerificationError("collection_project_not_array"))?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err(ResponseVerificationError("collection_item_budget"));
                }
                let first = rows[0].as_object().ok_or(ResponseVerificationError(
                    "collection_project_row_not_object",
                ))?;
                let mut fields = first.keys().filter(|field| {
                    rows.iter().all(|row| {
                        row.get(*field).is_some_and(|value| {
                            independent_collection_scalar_type(value) == Some(*value_type)
                        })
                    })
                });
                let field = fields.next().cloned().ok_or(ResponseVerificationError(
                    "collection_project_field_missing",
                ))?;
                if fields.next().is_some() {
                    return Err(ResponseVerificationError(
                        "collection_project_field_ambiguous",
                    ));
                }
                Value::Array(
                    rows.iter()
                        .map(|row| {
                            row.get(&field)
                                .cloned()
                                .ok_or(ResponseVerificationError("collection_project_missing"))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                )
            }
            CollectionProgramStep::ProjectOnlyNonFilterField => {
                let excluded = filter_field
                    .as_deref()
                    .ok_or(ResponseVerificationError("collection_filter_role_missing"))?;
                let rows = current
                    .as_array()
                    .ok_or(ResponseVerificationError("collection_project_not_array"))?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err(ResponseVerificationError("collection_item_budget"));
                }
                let first = rows[0].as_object().ok_or(ResponseVerificationError(
                    "collection_project_row_not_object",
                ))?;
                let mut fields = first.keys().filter(|field| {
                    field.as_str() != excluded
                        && rows.iter().all(|row| {
                            row.as_object()
                                .is_some_and(|object| object.contains_key(*field))
                        })
                });
                let field = fields.next().cloned().ok_or(ResponseVerificationError(
                    "collection_project_field_missing",
                ))?;
                if fields.next().is_some() {
                    return Err(ResponseVerificationError(
                        "collection_project_field_ambiguous",
                    ));
                }
                Value::Array(
                    rows.iter()
                        .map(|row| {
                            row.get(&field)
                                .cloned()
                                .ok_or(ResponseVerificationError("collection_project_missing"))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                )
            }
            CollectionProgramStep::AggregateUniqueIntegerField { operation } => {
                let rows = current
                    .as_array()
                    .ok_or(ResponseVerificationError("collection_aggregate_not_array"))?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err(ResponseVerificationError("collection_item_budget"));
                }
                let first = rows[0].as_object().ok_or(ResponseVerificationError(
                    "collection_aggregate_row_not_object",
                ))?;
                let mut fields = first.keys().filter(|field| {
                    rows.iter()
                        .all(|row| row.get(*field).and_then(Value::as_i64).is_some())
                });
                let field = fields.next().cloned().ok_or(ResponseVerificationError(
                    "collection_aggregate_field_missing",
                ))?;
                if fields.next().is_some() {
                    return Err(ResponseVerificationError(
                        "collection_aggregate_field_ambiguous",
                    ));
                }
                let values =
                    rows.iter()
                        .map(|row| {
                            row.get(&field).and_then(Value::as_i64).ok_or(
                                ResponseVerificationError("collection_aggregate_value_missing"),
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                let aggregate = match operation {
                    CollectionAggregateOperation::Sum => values
                        .into_iter()
                        .try_fold(0_i64, i64::checked_add)
                        .ok_or(ResponseVerificationError("collection_aggregate_overflow"))?,
                    CollectionAggregateOperation::Min => values
                        .into_iter()
                        .min()
                        .ok_or(ResponseVerificationError("collection_aggregate_empty"))?,
                    CollectionAggregateOperation::Max => values
                        .into_iter()
                        .max()
                        .ok_or(ResponseVerificationError("collection_aggregate_empty"))?,
                };
                Value::from(aggregate)
            }
            CollectionProgramStep::Count => {
                let count = current
                    .as_array()
                    .map(Vec::len)
                    .or_else(|| current.as_object().map(serde_json::Map::len))
                    .ok_or(ResponseVerificationError("collection_count_unsupported"))?;
                Value::from(
                    u64::try_from(count)
                        .map_err(|_| ResponseVerificationError("collection_count_overflow"))?,
                )
            }
        };
    }
    let computed = match format {
        ValueProjectionFormat::CanonicalJson => serde_json::to_string(&current)
            .map_err(|_| ResponseVerificationError("collection_serialization")),
        ValueProjectionFormat::PlainText => match current {
            Value::String(text) if !text.contains(['\n', '\r']) => Ok(text),
            Value::String(_) => Err(ResponseVerificationError("collection_multiline")),
            Value::Bool(_) | Value::Number(_) | Value::Null => Ok(current.to_string()),
            Value::Array(_) | Value::Object(_) => Err(ResponseVerificationError(
                "collection_plain_text_non_scalar",
            )),
        },
    }?;
    independently_apply_value_renderer(provider_payload, computed, renderer)
}

fn independently_request_collection_value(
    provider_payload: &Value,
    rows: &[Value],
    value_type: CollectionScalarType,
) -> Result<(String, Value), ResponseVerificationError> {
    let request = independently_request_text(provider_payload)?;
    let mut matches = BTreeMap::<Vec<u8>, (String, Value)>::new();
    for row in rows {
        let object = row.as_object().ok_or(ResponseVerificationError(
            "collection_filter_row_not_object",
        ))?;
        for (field, value) in object {
            if independently_collection_value_type(value) == Some(value_type)
                && independently_request_contains_value(&request, value)
            {
                let key = serde_json::to_vec(&(field, value))
                    .map_err(|_| ResponseVerificationError("collection_request_value_encode"))?;
                matches.insert(key, (field.clone(), value.clone()));
            }
        }
    }
    if matches.len() != 1 {
        return Err(ResponseVerificationError(
            "collection_request_value_cardinality",
        ));
    }
    matches
        .into_values()
        .next()
        .ok_or(ResponseVerificationError(
            "collection_request_value_missing",
        ))
}

fn independently_collection_value_type(value: &Value) -> Option<CollectionScalarType> {
    match value {
        Value::String(_) => Some(CollectionScalarType::String),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Some(CollectionScalarType::Integer)
        }
        Value::Bool(_) => Some(CollectionScalarType::Boolean),
        _ => None,
    }
}

const fn independently_collection_atom_type(
    value_type: AtomValueType,
) -> Option<CollectionScalarType> {
    match value_type {
        AtomValueType::String | AtomValueType::Identifier => Some(CollectionScalarType::String),
        AtomValueType::Integer => Some(CollectionScalarType::Integer),
        AtomValueType::Boolean => Some(CollectionScalarType::Boolean),
        AtomValueType::Collection => None,
    }
}

fn independently_request_contains_value(request: &str, value: &Value) -> bool {
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

fn independently_apply_output_renderer(
    provider_payload: &Value,
    computed: String,
    renderer: &CollectionOutputRenderer,
) -> Result<String, ResponseVerificationError> {
    match renderer {
        CollectionOutputRenderer::Direct => Ok(computed),
        CollectionOutputRenderer::RenderTemplate { prefix, suffix } => {
            Ok(format!("{prefix}{computed}{suffix}"))
        }
        CollectionOutputRenderer::RenderSequence { .. } => Err(ResponseVerificationError(
            "collection_render_sequence_unsupported",
        )),
        CollectionOutputRenderer::RequestTemplate { marker } => {
            independently_apply_request_template(provider_payload, computed, *marker)
        }
    }
}

pub(super) fn independently_apply_value_renderer(
    provider_payload: &Value,
    computed: String,
    renderer: &CollectionOutputRenderer,
) -> Result<String, ResponseVerificationError> {
    independently_apply_value_renderer_inner(None, provider_payload, computed, renderer)
}

pub(super) fn independently_apply_value_renderer_with_request(
    request_text: &str,
    provider_payload: &Value,
    computed: String,
    renderer: &CollectionOutputRenderer,
) -> Result<String, ResponseVerificationError> {
    independently_apply_value_renderer_inner(
        Some(request_text),
        provider_payload,
        computed,
        renderer,
    )
}

fn independently_apply_value_renderer_inner(
    request_text: Option<&str>,
    provider_payload: &Value,
    computed: String,
    renderer: &CollectionOutputRenderer,
) -> Result<String, ResponseVerificationError> {
    let CollectionOutputRenderer::RenderSequence { segments } = renderer else {
        return independently_apply_output_renderer(provider_payload, computed, renderer);
    };
    let mut output = String::new();
    for segment in segments {
        match segment {
            ResponseRenderSegment::Static { text } => output.push_str(text),
            ResponseRenderSegment::Primary => output.push_str(&computed),
            ResponseRenderSegment::Selected { selector, format } => {
                let selected = match request_text {
                    Some(request_text) => independently_select_scalar_with_request(
                        request_text,
                        provider_payload,
                        selector,
                    )?,
                    None => independently_select_scalar(provider_payload, selector)?,
                };
                output.push_str(&independently_format_selected_value(&selected, *format)?);
            }
        }
        if output.len() > MAX_VERIFIER_OUTPUT_BYTES {
            return Err(ResponseVerificationError("projection_output_budget"));
        }
    }
    Ok(output)
}

fn independently_apply_request_template(
    provider_payload: &Value,
    computed: String,
    marker: RequestTemplateMarker,
) -> Result<String, ResponseVerificationError> {
    let request = independently_request_text(provider_payload)?;
    let mut templates = BTreeMap::<String, ()>::new();
    for delimiter in ['`', '\'', '"'] {
        let parts = request.split(delimiter).collect::<Vec<_>>();
        for value in parts.iter().skip(1).step_by(2) {
            let value = value.trim();
            if !value.is_empty()
                && value.len() <= 512
                && !value.contains(['\n', '\r'])
                && value.matches(marker.token()).count() == 1
            {
                templates.insert(value.to_owned(), ());
            }
        }
    }
    if templates.len() != 1 {
        return Err(ResponseVerificationError("request_template_cardinality"));
    }
    let template = templates
        .into_keys()
        .next()
        .ok_or(ResponseVerificationError("request_template_missing"))?;
    let output = template.replacen(marker.token(), &computed, 1);
    if output.is_empty() || output.len() > MAX_VERIFIER_OUTPUT_BYTES {
        return Err(ResponseVerificationError("request_template_output_budget"));
    }
    Ok(output)
}

pub(super) fn independently_safe_collection_renderer(renderer: &CollectionOutputRenderer) -> bool {
    let (prefix, suffix) = match renderer {
        CollectionOutputRenderer::Direct => return true,
        CollectionOutputRenderer::RenderTemplate { prefix, suffix } => {
            (prefix.clone(), suffix.clone())
        }
        CollectionOutputRenderer::RenderSequence { segments } => {
            let primary_count = segments
                .iter()
                .filter(|segment| matches!(segment, ResponseRenderSegment::Primary))
                .count();
            let selected_count = segments
                .iter()
                .filter(|segment| matches!(segment, ResponseRenderSegment::Selected { .. }))
                .count();
            let dynamic_count = primary_count.saturating_add(selected_count);
            if !(1..=64).contains(&segments.len()) || dynamic_count == 0 || dynamic_count > 16 {
                return false;
            }
            let static_text = segments
                .iter()
                .filter_map(|segment| match segment {
                    ResponseRenderSegment::Static { text } => Some(text.as_str()),
                    ResponseRenderSegment::Primary | ResponseRenderSegment::Selected { .. } => None,
                })
                .collect::<String>();
            (static_text, String::new())
        }
        CollectionOutputRenderer::RequestTemplate { .. } => return true,
    };
    if prefix.len().saturating_add(suffix.len()) > 512 {
        return false;
    }
    let combined = format!("{prefix}{suffix}");
    if combined
        .chars()
        .any(|character| character.is_control() && character != '\n')
    {
        return false;
    }
    let lower = combined.to_lowercase();
    ![
        "authorization",
        "bearer ",
        "credential",
        "password",
        "passwd",
        "secret",
        "api_key",
        "api-key",
        "apikey",
        "private_key",
        "private-key",
        "privatekey",
        "cookie",
        "token",
        "customer ",
        "client ",
        "phone ",
        "address ",
        "клиент ",
        "телефон ",
        "адрес ",
        "улица ",
        "проспект ",
    ]
    .iter()
    .any(|term| lower.contains(term))
        && !["http://", "https://", "www."]
            .iter()
            .any(|term| lower.contains(term))
        && !["/home/", "/etc/", "/var/", "/opt/", "/root/", "/tmp/"]
            .iter()
            .any(|term| lower.contains(term))
        && !independently_contains_email_like(&combined)
        && !independently_contains_windows_path(&combined)
        && !independently_contains_high_entropy_run(&combined)
        && !independently_contains_phone_like(&combined)
}

fn independently_contains_phone_like(value: &str) -> bool {
    let mut digits = 0_usize;
    let mut span = 0_usize;
    for character in value.chars().chain(std::iter::once(' ')) {
        if character.is_ascii_digit() {
            digits = digits.saturating_add(1);
            span = span.saturating_add(1);
        } else if matches!(character, '+' | '-' | '(' | ')' | ' ') && digits > 0 {
            span = span.saturating_add(1);
        } else {
            if digits >= 7 && span <= 24 {
                return true;
            }
            digits = 0;
            span = 0;
        }
    }
    false
}

fn independently_contains_email_like(value: &str) -> bool {
    value.split_whitespace().any(|word| {
        let word = word.trim_matches(|character: char| {
            !character.is_alphanumeric() && !matches!(character, '@' | '.' | '_' | '-' | '+')
        });
        word.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
        })
    })
}

fn independently_contains_windows_path(value: &str) -> bool {
    value.as_bytes().windows(3).any(|window| {
        window[0].is_ascii_alphabetic() && window[1] == b':' && matches!(window[2], b'\\' | b'/')
    }) || value.contains("\\\\")
}

fn independently_contains_high_entropy_run(value: &str) -> bool {
    value
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || matches!(character, '+' | '/' | '_' | '-'))
        })
        .any(|run| {
            if run.len() < 24 {
                return false;
            }
            let has_lower = run.bytes().any(|byte| byte.is_ascii_lowercase());
            let has_upper = run.bytes().any(|byte| byte.is_ascii_uppercase());
            let has_digit = run.bytes().any(|byte| byte.is_ascii_digit());
            let long_hex = run.len() >= 32 && run.bytes().all(|byte| byte.is_ascii_hexdigit());
            long_hex || (has_lower && has_upper && has_digit)
        })
}
