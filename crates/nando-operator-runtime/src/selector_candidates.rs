use std::collections::BTreeMap;

use nando_operator_kernel::{AtomValueType, ResponseValueSelector};
use serde_json::Value;

pub const MAX_SELECTOR_CANDIDATES: usize = 128;

/// Enumerates physical selectors from the current bounded runtime surface.
/// It observes no teacher value and grants no authority.
#[must_use]
pub fn selector_candidates(payload: &Value) -> Vec<ResponseValueSelector> {
    let mut selectors = [
        AtomValueType::String,
        AtomValueType::Integer,
        AtomValueType::Boolean,
    ]
    .into_iter()
    .flat_map(|value_type| {
        [
            ResponseValueSelector::UniqueScalar { value_type },
            ResponseValueSelector::UniqueTurnScalar { value_type },
        ]
    })
    .collect::<Vec<_>>();
    selectors.push(ResponseValueSelector::CommandOutputBody);
    selectors.push(ResponseValueSelector::RequestLastToken);
    selectors.push(ResponseValueSelector::RequestUniqueLiteral);
    selectors.extend([
        ResponseValueSelector::RequestReferencedJsonField {
            value_type: AtomValueType::String,
        },
        ResponseValueSelector::RequestReferencedJsonField {
            value_type: AtomValueType::Integer,
        },
        ResponseValueSelector::RequestReferencedJsonField {
            value_type: AtomValueType::Boolean,
        },
    ]);
    for ordinal in 0..16_u16 {
        selectors.extend([
            ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                ordinal,
                value_type: AtomValueType::String,
            },
            ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                ordinal,
                value_type: AtomValueType::Integer,
            },
            ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                ordinal,
                value_type: AtomValueType::Boolean,
            },
        ]);
    }
    let Some(items) = payload.get("input").and_then(Value::as_array) else {
        return selectors;
    };
    let outputs = items
        .iter()
        .filter(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })
        .filter_map(|item| item.get("output"))
        .collect::<Vec<_>>();
    let Some(latest_output) = outputs.last().copied() else {
        return selectors;
    };
    if let Some(value) = unique_embedded_json_output(latest_output) {
        let mut fields = Vec::new();
        collect_scalar_field_candidates(&value, 0, &mut fields);
        selectors.extend(
            fields
                .into_iter()
                .map(|(field, value_type)| ResponseValueSelector::JsonField { field, value_type }),
        );
        let mut ordinals = BTreeMap::new();
        let mut ordinal_selectors = Vec::new();
        collect_scalar_ordinal_candidates(&value, 0, &mut ordinals, &mut ordinal_selectors);
        selectors.extend(ordinal_selectors);
    }
    let mut turn_fields = Vec::new();
    for output in &outputs {
        if let Some(value) = unique_embedded_json_output(output) {
            collect_scalar_field_candidates(&value, 0, &mut turn_fields);
        }
    }
    selectors.extend(turn_fields.into_iter().flat_map(|(field, value_type)| {
        [
            ResponseValueSelector::UniqueTurnJsonField {
                field: field.clone(),
                value_type,
            },
            ResponseValueSelector::UniqueActiveTurnJsonField { field, value_type },
        ]
    }));
    for (output_index, output) in outputs.iter().enumerate() {
        let Ok(output_ordinal) = u16::try_from(output_index.saturating_add(1)) else {
            break;
        };
        if output_index < 16 {
            for value_type in [
                AtomValueType::String,
                AtomValueType::Integer,
                AtomValueType::Boolean,
            ] {
                for scalar_ordinal in 0_u16..16 {
                    selectors.push(ResponseValueSelector::TurnOutputScalarOrdinal {
                        output_ordinal,
                        scalar_ordinal,
                        value_type,
                    });
                    if output_index.saturating_add(1) == outputs.len() {
                        selectors.push(ResponseValueSelector::LatestTurnOutputScalarOrdinal {
                            scalar_ordinal,
                            value_type,
                        });
                        selectors.push(ResponseValueSelector::LatestTurnOutputScalarFromEnd {
                            reverse_ordinal: scalar_ordinal,
                            value_type,
                        });
                    }
                }
            }
        }
        let Some(output_text) = bounded_output_text(output) else {
            continue;
        };
        for (line_index, line) in output_text.lines().enumerate().take(256) {
            if !line.is_empty()
                && line.len() <= 512
                && let Ok(line_index) = u16::try_from(line_index)
            {
                selectors.push(ResponseValueSelector::TurnOutputLine {
                    output_ordinal,
                    line_index,
                    value_type: AtomValueType::String,
                });
                if output_index.saturating_add(1) == outputs.len() {
                    selectors.push(ResponseValueSelector::LatestTurnOutputLine {
                        line_index,
                        value_type: AtomValueType::String,
                    });
                }
            }
            if output_index.saturating_add(1) != outputs.len() {
                continue;
            }
            for delimiter in [':', '='] {
                let Some(offset) = line.find(delimiter) else {
                    continue;
                };
                let split = offset.saturating_add(delimiter.len_utf8());
                let whitespace = line[split..]
                    .len()
                    .saturating_sub(line[split..].trim_start().len());
                let prefix_end = split.saturating_add(whitespace);
                if prefix_end == 0 || prefix_end > 128 || prefix_end >= line.len() {
                    continue;
                }
                let candidate = line[prefix_end..].trim();
                if let Some(value_type) = scalar_text_type(candidate) {
                    selectors.push(ResponseValueSelector::ContentLinePrefix {
                        prefix: line[..prefix_end].to_owned(),
                        value_type,
                    });
                }
            }
        }
    }
    selectors.sort();
    selectors.dedup();
    selectors.sort_by(|left, right| {
        selector_priority(left)
            .cmp(&selector_priority(right))
            .then_with(|| left.cmp(right))
    });
    selectors.truncate(MAX_SELECTOR_CANDIDATES);
    selectors.sort();
    selectors
}

#[must_use]
pub const fn is_source_neutral_request_selector(selector: &ResponseValueSelector) -> bool {
    matches!(
        selector,
        ResponseValueSelector::RequestReferencedJsonField { .. }
            | ResponseValueSelector::RequestReferencedJsonFieldOrdinal { .. }
            | ResponseValueSelector::RequestLastToken
            | ResponseValueSelector::RequestUniqueLiteral
    )
}

#[doc(hidden)]
#[must_use]
pub fn bounded_output_text(output: &Value) -> Option<String> {
    match output {
        Value::String(text) if !text.is_empty() && text.len() <= 65_536 => Some(text.clone()),
        Value::Array(parts) if !parts.is_empty() && parts.len() <= 64 => {
            let mut output = String::new();
            for part in parts {
                if !matches!(
                    part.get("type").and_then(Value::as_str),
                    Some("text" | "input_text" | "output_text")
                ) {
                    return None;
                }
                let text = part.get("text").and_then(Value::as_str)?;
                let next_len = output
                    .len()
                    .checked_add(text.len())?
                    .checked_add(usize::from(!output.is_empty()))?;
                if text.is_empty() || next_len > 65_536 {
                    return None;
                }
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str(text);
            }
            Some(output)
        }
        _ => None,
    }
}

#[doc(hidden)]
#[must_use]
pub fn unique_embedded_json_output(output: &Value) -> Option<Value> {
    bounded_output_text(output)?;
    match output {
        Value::String(text) => unique_embedded_json_object(text),
        Value::Array(_) => {
            let mut candidates = BTreeMap::<Vec<u8>, Value>::new();
            collect_embedded_json_objects(output, &mut candidates, 0);
            (candidates.len() == 1)
                .then(|| candidates.into_values().next())
                .flatten()
        }
        _ => None,
    }
}

const fn selector_priority(selector: &ResponseValueSelector) -> u8 {
    match selector {
        ResponseValueSelector::RequestReferencedJsonField { .. }
        | ResponseValueSelector::RequestReferencedJsonFieldOrdinal { .. }
        | ResponseValueSelector::RequestLastToken
        | ResponseValueSelector::RequestUniqueLiteral => 0,
        ResponseValueSelector::ContinuationHandle { .. }
        | ResponseValueSelector::ContentLinePrefix { .. }
        | ResponseValueSelector::CommandOutputBody
        | ResponseValueSelector::LatestTurnOutputLine { .. }
        | ResponseValueSelector::LatestTurnOutputScalarOrdinal { .. }
        | ResponseValueSelector::LatestTurnOutputScalarFromEnd { .. } => 1,
        ResponseValueSelector::UniqueScalar { .. }
        | ResponseValueSelector::UniqueTurnScalar { .. }
        | ResponseValueSelector::JsonField { .. }
        | ResponseValueSelector::JsonScalarOrdinal { .. } => 2,
        ResponseValueSelector::UniqueTurnJsonField { .. }
        | ResponseValueSelector::UniqueActiveTurnJsonField { .. } => 3,
        ResponseValueSelector::TurnOutputLine { .. }
        | ResponseValueSelector::TurnOutputScalarOrdinal { .. } => 4,
    }
}

fn collect_scalar_ordinal_candidates(
    value: &Value,
    depth: usize,
    ordinals: &mut BTreeMap<AtomValueType, u16>,
    output: &mut Vec<ResponseValueSelector>,
) {
    if depth > 8 || output.len() >= 64 {
        return;
    }
    if let Some(value_type) = atom_value_type(value) {
        let ordinal = ordinals.entry(value_type).or_default();
        output.push(ResponseValueSelector::JsonScalarOrdinal {
            ordinal: *ordinal,
            value_type,
        });
        *ordinal = ordinal.saturating_add(1);
        return;
    }
    match value {
        Value::Object(object) => {
            for value in object.values() {
                collect_scalar_ordinal_candidates(value, depth.saturating_add(1), ordinals, output);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_scalar_ordinal_candidates(value, depth.saturating_add(1), ordinals, output);
            }
        }
        _ => {}
    }
}

fn collect_scalar_field_candidates(
    value: &Value,
    depth: usize,
    output: &mut Vec<(String, AtomValueType)>,
) {
    if depth > 8 || output.len() >= 256 {
        return;
    }
    match value {
        Value::Object(object) => {
            for (field, value) in object {
                if let Some(value_type) = atom_value_type(value) {
                    output.push((field.clone(), value_type));
                }
                collect_scalar_field_candidates(value, depth.saturating_add(1), output);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_scalar_field_candidates(value, depth.saturating_add(1), output);
            }
        }
        _ => {}
    }
}

fn atom_value_type(value: &Value) -> Option<AtomValueType> {
    match value {
        Value::String(_) => Some(AtomValueType::String),
        Value::Number(number) if number.is_i64() || number.is_u64() => Some(AtomValueType::Integer),
        Value::Bool(_) => Some(AtomValueType::Boolean),
        _ => None,
    }
}

fn scalar_text_type(value: &str) -> Option<AtomValueType> {
    let parsed =
        serde_json::from_str::<Value>(value).unwrap_or_else(|_| Value::String(value.to_owned()));
    atom_value_type(&parsed)
}

fn unique_embedded_json_object(text: &str) -> Option<Value> {
    unique_embedded_json_object_at_depth(text, 0)
}

fn unique_embedded_json_object_at_depth(text: &str, depth: usize) -> Option<Value> {
    if depth > 4 {
        return None;
    }
    let mut sources = vec![text.trim().to_owned()];
    if let Some((_, output)) = text.rsplit_once("\nOutput:\n") {
        sources.push(output.trim().to_owned());
    }
    let mut fenced = None::<String>;
    for line in text.lines() {
        let trimmed = line.trim();
        if fenced.is_some() && trimmed == "```" {
            sources.push(fenced.take().unwrap_or_default());
        } else if let Some(value) = &mut fenced {
            if !value.is_empty() {
                value.push('\n');
            }
            value.push_str(line);
        } else if trimmed == "```" || trimmed.eq_ignore_ascii_case("```json") {
            fenced = Some(String::new());
        } else if trimmed.starts_with(['{', '[']) {
            sources.push(trimmed.to_owned());
        }
    }
    let mut candidates = BTreeMap::<Vec<u8>, Value>::new();
    for source in sources {
        let Ok(value) = serde_json::from_str::<Value>(&source) else {
            continue;
        };
        collect_embedded_json_objects(&value, &mut candidates, depth);
    }
    (candidates.len() == 1)
        .then(|| candidates.into_values().next())
        .flatten()
}

fn collect_embedded_json_objects(
    value: &Value,
    output: &mut BTreeMap<Vec<u8>, Value>,
    depth: usize,
) {
    match value {
        Value::Object(object) => {
            let mut encoded_children = BTreeMap::new();
            if depth < 4 {
                for text in object.values().filter_map(Value::as_str) {
                    if let Some(child) = unique_embedded_json_object_at_depth(text, depth + 1)
                        && let Ok(key) = serde_json::to_vec(&child)
                    {
                        encoded_children.insert(key, child);
                    }
                }
            }
            if encoded_children.len() == 1 {
                output.extend(encoded_children);
                return;
            }
            if let Ok(key) = serde_json::to_vec(value) {
                output.insert(key, value.clone());
            }
        }
        Value::Array(parts) => {
            let content_parts = !parts.is_empty()
                && parts.iter().all(|part| {
                    part.get("text").and_then(Value::as_str).is_some()
                        && matches!(
                            part.get("type").and_then(Value::as_str),
                            Some("text" | "input_text" | "output_text")
                        )
                });
            if !content_parts {
                let wrapped = serde_json::json!({"items": value.clone()});
                if let Ok(key) = serde_json::to_vec(&wrapped) {
                    output.insert(key, wrapped);
                }
                return;
            }
            for part in parts {
                let Some(text) = part.get("text").and_then(Value::as_str) else {
                    continue;
                };
                if let Some(value) = unique_embedded_json_object_at_depth(text, depth + 1)
                    && let Ok(key) = serde_json::to_vec(&value)
                {
                    output.insert(key, value);
                }
            }
        }
        _ => {}
    }
}
