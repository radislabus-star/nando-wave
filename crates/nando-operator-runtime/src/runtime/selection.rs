use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{AtomValueType, ResponseValueSelector};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{
    RUNTIME_OUTPUT_SCALAR_BUDGET, canonical_collection_from_provider_output,
    immediate_function_output, request_text,
};

#[derive(Clone, Debug, PartialEq)]
#[doc(hidden)]
pub struct ExtractedScalar {
    pub value: Value,
    pub value_type: AtomValueType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[doc(hidden)]
pub enum ObservedSourceClass {
    ImmediateToolJson,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[doc(hidden)]
pub struct ObservedRoleCandidate {
    pub selector: ResponseValueSelector,
    pub request_position: u16,
    pub json_path_sha256: [u8; 32],
    pub value_type: AtomValueType,
    pub source_class: ObservedSourceClass,
}

#[derive(Clone, Debug)]
struct ReferencedPathScalar {
    field: String,
    request_position: usize,
    path_sha256: [u8; 32],
    value_type: AtomValueType,
}

/// Extracts only roles that are directly observable in the current request and
/// immediate tool output. Ordinals follow request mention order; JSON map order
/// and field names never become runtime authority.
#[doc(hidden)]
pub fn observed_request_ordinal_roles(
    request: &str,
    provider_payload: &Value,
) -> Result<Vec<ObservedRoleCandidate>, &'static str> {
    referenced_path_scalars(request, provider_payload)?
        .into_iter()
        .enumerate()
        .map(|(ordinal, candidate)| {
            Ok(ObservedRoleCandidate {
                selector: ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                    ordinal: u16::try_from(ordinal).map_err(|_| "observed_request_role_count")?,
                    value_type: candidate.value_type,
                },
                request_position: u16::try_from(candidate.request_position)
                    .map_err(|_| "observed_request_position")?,
                json_path_sha256: candidate.path_sha256,
                value_type: candidate.value_type,
                source_class: ObservedSourceClass::ImmediateToolJson,
            })
        })
        .collect()
}

#[doc(hidden)]
pub fn canonical_request_ordinal_selector(
    request: &str,
    provider_payload: &Value,
    selector: &ResponseValueSelector,
) -> Result<Option<ResponseValueSelector>, &'static str> {
    let ResponseValueSelector::JsonField { field, value_type } = selector else {
        return Ok(None);
    };
    let observed = referenced_path_scalars(request, provider_payload)?;
    let mut matches = observed
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.field == *field && candidate.value_type == *value_type);
    let Some((ordinal, _)) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err("observed_request_role_path_ambiguous");
    }
    Ok(Some(
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal: u16::try_from(ordinal).map_err(|_| "observed_request_role_count")?,
            value_type: *value_type,
        },
    ))
}

fn referenced_path_scalars(
    request: &str,
    provider_payload: &Value,
) -> Result<Vec<ReferencedPathScalar>, &'static str> {
    if request.is_empty() || request.len() > 16_384 {
        return Err("selector_request_text_missing");
    }
    let request_tokens = identifier_tokens(request);
    let output =
        immediate_tool_output_value(provider_payload).ok_or("immediate_tool_output_missing")?;
    let mut observed = Vec::new();
    for (object_index, text) in output_text_parts(output)?.into_iter().enumerate() {
        for (embedded_index, object) in runtime_embedded_json_objects(text).into_iter().enumerate()
        {
            let mut path = vec![format!("o:{object_index}"), format!("e:{embedded_index}")];
            collect_observed_request_roles(
                &Value::Object(object),
                &request_tokens,
                &mut path,
                0,
                &mut observed,
            )?;
        }
    }
    observed.sort_by_key(|candidate| candidate.request_position);
    if observed.is_empty() || observed.len() > 16 {
        return Err("observed_request_role_count");
    }
    if observed
        .windows(2)
        .any(|pair| pair[0].request_position == pair[1].request_position)
    {
        return Err("observed_request_role_path_ambiguous");
    }
    Ok(observed)
}

fn collect_observed_request_roles(
    value: &Value,
    request_tokens: &[String],
    path: &mut Vec<String>,
    depth: usize,
    output: &mut Vec<ReferencedPathScalar>,
) -> Result<(), &'static str> {
    if depth > 8 || output.len() >= 64 {
        return Err("observed_request_role_structure_budget");
    }
    match value {
        Value::Object(object) => {
            for (field, value) in object {
                path.push(format!("k:{field}"));
                let positions = request_identifier_positions(request_tokens, field);
                if !positions.is_empty() {
                    if positions.len() != 1 {
                        return Err("observed_request_role_mention_ambiguous");
                    }
                    if let Some(value_type) = observed_scalar_type(value) {
                        output.push(ReferencedPathScalar {
                            field: field.clone(),
                            request_position: positions[0],
                            path_sha256: observed_json_path_digest(path),
                            value_type,
                        });
                    }
                }
                collect_observed_request_roles(
                    value,
                    request_tokens,
                    path,
                    depth.saturating_add(1),
                    output,
                )?;
                path.pop();
            }
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                path.push(format!("i:{index}"));
                collect_observed_request_roles(
                    value,
                    request_tokens,
                    path,
                    depth.saturating_add(1),
                    output,
                )?;
                path.pop();
            }
        }
        _ => {}
    }
    Ok(())
}

fn observed_scalar_type(value: &Value) -> Option<AtomValueType> {
    match value {
        Value::String(_) => Some(AtomValueType::String),
        Value::Number(number) if number.is_i64() || number.is_u64() => Some(AtomValueType::Integer),
        Value::Bool(_) => Some(AtomValueType::Boolean),
        _ => None,
    }
}

pub(super) fn request_identifier_positions(
    request_tokens: &[String],
    identifier: &str,
) -> Vec<usize> {
    let identifier_tokens = identifier_tokens(identifier);
    if identifier_tokens.is_empty() {
        return Vec::new();
    }
    request_tokens
        .windows(identifier_tokens.len())
        .enumerate()
        .filter_map(|(position, window)| (window == identifier_tokens).then_some(position))
        .collect()
}

pub(super) fn observed_json_path_digest(path: &[String]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.observed-json-path.v1");
    for segment in path {
        hasher.update((segment.len() as u64).to_le_bytes());
        hasher.update(segment.as_bytes());
    }
    hasher.finalize().into()
}

#[doc(hidden)]
pub fn immediate_unique_scalar(provider_payload: &Value) -> Result<ExtractedScalar, &'static str> {
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

#[doc(hidden)]
pub fn immediate_selected_scalar(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
) -> Result<ExtractedScalar, &'static str> {
    let request = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .rev()
        .find(|item| item.get("role").and_then(Value::as_str) == Some("user"))
        .and_then(|item| item.get("content"))
        .and_then(runtime_request_content_text)
        .unwrap_or_default();
    immediate_selected_scalar_with_request(&request, provider_payload, selector)
}

pub(super) fn immediate_selected_scalar_with_request(
    request_text: &str,
    provider_payload: &Value,
    selector: &ResponseValueSelector,
) -> Result<ExtractedScalar, &'static str> {
    match selector {
        ResponseValueSelector::ContinuationHandle { value_type } => {
            continuation_handle_scalar(provider_payload, *value_type)
        }
        ResponseValueSelector::UniqueScalar { value_type } => {
            let scalar = if *value_type == AtomValueType::Collection {
                immediate_unique_collection(provider_payload)?
            } else {
                immediate_unique_scalar(provider_payload)?
            };
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
        ResponseValueSelector::RequestReferencedJsonField { value_type } => {
            request_referenced_json_field(request_text, provider_payload, *value_type)
        }
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        } => request_referenced_json_field_ordinal(
            request_text,
            provider_payload,
            *ordinal,
            *value_type,
        ),
        ResponseValueSelector::TurnOutputLine {
            output_ordinal,
            line_index,
            value_type,
        } => turn_output_line(provider_payload, *output_ordinal, *line_index, *value_type),
        ResponseValueSelector::TurnOutputScalarOrdinal {
            output_ordinal,
            scalar_ordinal,
            value_type,
        } => turn_output_scalar_ordinal(
            provider_payload,
            *output_ordinal,
            *scalar_ordinal,
            *value_type,
        ),
        ResponseValueSelector::LatestTurnOutputLine {
            line_index,
            value_type,
        } => latest_turn_output_line(provider_payload, *line_index, *value_type),
        ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal,
            value_type,
        } => latest_turn_output_scalar_ordinal(provider_payload, *scalar_ordinal, *value_type),
        ResponseValueSelector::LatestTurnOutputScalarFromEnd {
            reverse_ordinal,
            value_type,
        } => latest_turn_output_scalar_from_end(provider_payload, *reverse_ordinal, *value_type),
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

pub(super) fn continuation_handle_scalar(
    provider_payload: &Value,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    if !matches!(
        value_type,
        AtomValueType::Identifier | AtomValueType::String
    ) {
        return Err("continuation_handle_type");
    }
    let output =
        immediate_tool_output_value(provider_payload).ok_or("immediate_tool_output_missing")?;
    continuation_handle_scalar_from_output(output, value_type)
}

pub(super) fn continuation_handle_scalar_from_output(
    output: &Value,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    if !matches!(
        value_type,
        AtomValueType::Identifier | AtomValueType::String
    ) {
        return Err("continuation_handle_type");
    }
    let mut matches = output_text_parts(output)?
        .into_iter()
        .flat_map(str::lines)
        .filter_map(|line| {
            let line = line.trim();
            [
                "Script running with cell ID ",
                "Process running with session ID ",
            ]
            .into_iter()
            .find_map(|prefix| line.strip_prefix(prefix))
        })
        .filter_map(|tail| tail.split_whitespace().next())
        .filter(|value| {
            !value.is_empty()
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        })
        .collect::<Vec<_>>();
    matches.sort_unstable();
    matches.dedup();
    if matches.len() != 1 {
        return Err("continuation_handle_ambiguous");
    }
    Ok(ExtractedScalar {
        value: Value::String(matches[0].to_owned()),
        value_type,
    })
}

#[doc(hidden)]
pub fn structural_output_selectors_for_field_hint(
    provider_payload: &Value,
    field_hint: &str,
    value_type: AtomValueType,
) -> Result<Vec<ResponseValueSelector>, &'static str> {
    if field_hint.is_empty() || field_hint.len() > 128 || value_type == AtomValueType::Collection {
        return Err("field_hint_invalid");
    }
    let output =
        immediate_tool_output_value(provider_payload).ok_or("immediate_tool_output_missing")?;
    let mut scalar_index = 0_usize;
    let mut matched_indices = Vec::new();
    for text in output_text_parts(output)? {
        if let Ok(value) = serde_json::from_str::<Value>(text) {
            collect_field_hint_scalar_ordinals(
                &value,
                field_hint,
                value_type,
                0,
                &mut scalar_index,
                &mut matched_indices,
            )?;
            continue;
        }
        for object in runtime_embedded_json_objects(text) {
            collect_field_hint_scalar_ordinals(
                &Value::Object(object),
                field_hint,
                value_type,
                0,
                &mut scalar_index,
                &mut matched_indices,
            )?;
        }
    }
    if scalar_index == 0 || matched_indices.is_empty() {
        return Err("field_hint_ordinal_missing");
    }
    let mut selectors = BTreeSet::new();
    for index in matched_indices {
        let ordinal = u16::try_from(index).map_err(|_| "field_hint_ordinal_budget")?;
        let reverse = scalar_index
            .checked_sub(index.saturating_add(1))
            .and_then(|value| u16::try_from(value).ok())
            .ok_or("field_hint_ordinal_budget")?;
        selectors.insert(ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal: ordinal,
            value_type,
        });
        selectors.insert(ResponseValueSelector::LatestTurnOutputScalarFromEnd {
            reverse_ordinal: reverse,
            value_type,
        });
    }
    Ok(selectors.into_iter().collect())
}

/// Uses a completed action value only to discover name-free structural
/// positions during training. Returned selectors contain no teacher value and
/// runtime execution still reads solely from the observed output.
#[doc(hidden)]
pub fn structural_output_selectors_for_teacher_value(
    provider_payload: &Value,
    teacher_value: &Value,
    value_type: AtomValueType,
) -> Result<Vec<ResponseValueSelector>, &'static str> {
    if value_type == AtomValueType::Collection {
        return Err("teacher_value_type_invalid");
    }
    let mut outputs = Vec::new();
    for output_ordinal in 1_u16..=64 {
        let Ok(output) = active_turn_output_value(provider_payload, Some(output_ordinal)) else {
            break;
        };
        let mut scalars = Vec::new();
        for text in output_text_parts(output)? {
            collect_runtime_output_scalars(text, &mut scalars)?;
        }
        outputs.push((
            output_ordinal,
            compatible_runtime_scalars(scalars, value_type),
        ));
    }
    let last_output_ordinal = outputs.last().map(|(ordinal, _)| *ordinal);
    let mut selectors = BTreeSet::new();
    for (output_ordinal, compatible) in outputs {
        for (index, scalar) in compatible.iter().enumerate() {
            if scalar.value != *teacher_value {
                continue;
            }
            let ordinal = u16::try_from(index).map_err(|_| "teacher_value_ordinal_budget")?;
            selectors.insert(ResponseValueSelector::TurnOutputScalarOrdinal {
                output_ordinal,
                scalar_ordinal: ordinal,
                value_type,
            });
            if Some(output_ordinal) == last_output_ordinal {
                let reverse = compatible
                    .len()
                    .checked_sub(index.saturating_add(1))
                    .and_then(|value| u16::try_from(value).ok())
                    .ok_or("teacher_value_ordinal_budget")?;
                selectors.insert(ResponseValueSelector::LatestTurnOutputScalarOrdinal {
                    scalar_ordinal: ordinal,
                    value_type,
                });
                selectors.insert(ResponseValueSelector::LatestTurnOutputScalarFromEnd {
                    reverse_ordinal: reverse,
                    value_type,
                });
            }
        }
    }
    if selectors.is_empty() {
        return Err("teacher_value_ordinal_missing");
    }
    Ok(selectors.into_iter().collect())
}

fn compatible_runtime_scalars(
    scalars: Vec<ExtractedScalar>,
    value_type: AtomValueType,
) -> Vec<ExtractedScalar> {
    scalars
        .into_iter()
        .filter(|scalar| {
            scalar.value_type == value_type
                || matches!(
                    (scalar.value_type, value_type),
                    (AtomValueType::Identifier, AtomValueType::String)
                )
        })
        .collect()
}

fn collect_field_hint_scalar_ordinals(
    value: &Value,
    field_hint: &str,
    value_type: AtomValueType,
    depth: usize,
    scalar_index: &mut usize,
    matched_indices: &mut Vec<usize>,
) -> Result<(), &'static str> {
    if depth > 8 || *scalar_index >= RUNTIME_OUTPUT_SCALAR_BUDGET {
        return Err("field_hint_structure_budget");
    }
    match value {
        Value::Object(values) => {
            for (field, value) in values {
                if extracted_scalar(value.clone(), value_type).is_ok() {
                    if field == field_hint {
                        matched_indices.push(*scalar_index);
                    }
                    *scalar_index = scalar_index.saturating_add(1);
                } else {
                    collect_field_hint_scalar_ordinals(
                        value,
                        field_hint,
                        value_type,
                        depth.saturating_add(1),
                        scalar_index,
                        matched_indices,
                    )?;
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                if extracted_scalar(value.clone(), value_type).is_ok() {
                    *scalar_index = scalar_index.saturating_add(1);
                } else {
                    collect_field_hint_scalar_ordinals(
                        value,
                        field_hint,
                        value_type,
                        depth.saturating_add(1),
                        scalar_index,
                        matched_indices,
                    )?;
                }
            }
        }
        _ if extracted_scalar(value.clone(), value_type).is_ok() => {
            *scalar_index = scalar_index.saturating_add(1);
        }
        _ => {}
    }
    Ok(())
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

fn request_referenced_json_field(
    request: &str,
    provider_payload: &Value,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    if request.is_empty() {
        return Err("selector_request_text_missing");
    }
    let request_tokens = identifier_tokens(request);
    let output =
        immediate_tool_output_value(provider_payload).ok_or("immediate_tool_output_missing")?;
    let mut matches = Vec::<(String, ExtractedScalar)>::new();
    for text in output_text_parts(output)? {
        for object in runtime_embedded_json_objects(text) {
            collect_runtime_request_referenced_fields(
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
        return Err("selector_request_field_ambiguous");
    }
    matches
        .pop()
        .map(|(_, scalar)| scalar)
        .ok_or("selector_request_field_missing")
}

fn request_referenced_json_field_ordinal(
    request: &str,
    provider_payload: &Value,
    ordinal: u16,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    if request.is_empty() {
        return Err("selector_request_text_missing");
    }
    let request_tokens = identifier_tokens(request);
    let output =
        immediate_tool_output_value(provider_payload).ok_or("immediate_tool_output_missing")?;
    let mut matches = Vec::<(String, ExtractedScalar)>::new();
    for text in output_text_parts(output)? {
        for object in runtime_embedded_json_objects(text) {
            collect_runtime_request_referenced_fields(
                &Value::Object(object),
                &request_tokens,
                value_type,
                0,
                &mut matches,
            )?;
        }
    }
    matches.sort_by(|left, right| {
        request_identifier_position(&request_tokens, &left.0)
            .cmp(&request_identifier_position(&request_tokens, &right.0))
            .then_with(|| left.0.cmp(&right.0))
            .then_with(|| left.1.value.to_string().cmp(&right.1.value.to_string()))
    });
    matches.dedup();
    matches
        .into_iter()
        .nth(usize::from(ordinal))
        .map(|(_, scalar)| scalar)
        .ok_or("selector_request_field_ordinal_missing")
}

fn request_identifier_position(request_tokens: &[String], identifier: &str) -> Option<usize> {
    let identifier_tokens = identifier_tokens(identifier);
    (!identifier_tokens.is_empty())
        .then(|| {
            request_tokens
                .windows(identifier_tokens.len())
                .position(|window| window == identifier_tokens)
        })
        .flatten()
}

fn collect_runtime_request_referenced_fields(
    value: &Value,
    request_tokens: &[String],
    value_type: AtomValueType,
    depth: usize,
    output: &mut Vec<(String, ExtractedScalar)>,
) -> Result<(), &'static str> {
    if depth > 8 || output.len() >= 64 {
        return Err("selector_request_field_structure_budget");
    }
    match value {
        Value::Object(object) => {
            for (field, value) in object {
                if request_mentions_identifier(request_tokens, field)
                    && let Ok(scalar) = extracted_scalar(value.clone(), value_type)
                {
                    output.push((field.clone(), scalar));
                }
                collect_runtime_request_referenced_fields(
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
                collect_runtime_request_referenced_fields(
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

fn runtime_request_content_text(content: &Value) -> Option<String> {
    if let Some(text) = content.as_str() {
        return (!text.is_empty()).then(|| text.to_owned());
    }
    let text = content
        .as_array()?
        .iter()
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("");
    (!text.is_empty()).then_some(text)
}

pub(super) fn identifier_tokens(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_lowercase)
        .take(256)
        .collect()
}

pub(super) fn request_mentions_identifier(request_tokens: &[String], identifier: &str) -> bool {
    let identifier_tokens = identifier_tokens(identifier);
    !identifier_tokens.is_empty()
        && request_tokens
            .windows(identifier_tokens.len())
            .any(|window| window == identifier_tokens)
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

pub(super) fn runtime_embedded_json_objects(text: &str) -> Vec<serde_json::Map<String, Value>> {
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
    let output = active_turn_output_value(provider_payload, Some(output_ordinal))?;
    output_line_scalar(output, line_index)
}

fn latest_turn_output_line(
    provider_payload: &Value,
    line_index: u16,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    if value_type != AtomValueType::String {
        return Err("latest_turn_output_line_selector_invalid");
    }
    let output = active_turn_output_value(provider_payload, None)?;
    output_line_scalar(output, line_index)
}

fn output_line_scalar(output: &Value, line_index: u16) -> Result<ExtractedScalar, &'static str> {
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

fn turn_output_scalar_ordinal(
    provider_payload: &Value,
    output_ordinal: u16,
    scalar_ordinal: u16,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    if output_ordinal == 0 || matches!(value_type, AtomValueType::Collection) {
        return Err("turn_output_scalar_ordinal_selector_invalid");
    }
    let output = active_turn_output_value(provider_payload, Some(output_ordinal))?;
    output_scalar_ordinal(output, scalar_ordinal, value_type)
}

fn latest_turn_output_scalar_ordinal(
    provider_payload: &Value,
    scalar_ordinal: u16,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    if matches!(value_type, AtomValueType::Collection) {
        return Err("latest_turn_output_scalar_ordinal_selector_invalid");
    }
    let output = active_turn_output_value(provider_payload, None)?;
    output_scalar_ordinal(output, scalar_ordinal, value_type)
}

pub(super) fn latest_turn_output_scalar_from_end(
    provider_payload: &Value,
    reverse_ordinal: u16,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    if matches!(value_type, AtomValueType::Collection) {
        return Err("latest_turn_output_scalar_from_end_selector_invalid");
    }
    let output = active_turn_output_value(provider_payload, None)?;
    let mut scalars = Vec::new();
    for text in output_text_parts(output)? {
        collect_runtime_output_scalars(text, &mut scalars)?;
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
        .ok_or("latest_turn_output_scalar_from_end_missing")
}

pub(super) fn active_turn_output_value(
    provider_payload: &Value,
    output_ordinal: Option<u16>,
) -> Result<&Value, &'static str> {
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
    .ok_or("turn_output_ordinal_missing")?;
    item.get("output").ok_or("turn_output_missing")
}

fn output_scalar_ordinal(
    output: &Value,
    scalar_ordinal: u16,
    value_type: AtomValueType,
) -> Result<ExtractedScalar, &'static str> {
    let mut scalars = Vec::new();
    for text in output_text_parts(output)? {
        collect_runtime_output_scalars(text, &mut scalars)?;
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
        .ok_or("turn_output_scalar_ordinal_missing")
}

fn collect_runtime_output_scalars(
    text: &str,
    output: &mut Vec<ExtractedScalar>,
) -> Result<(), &'static str> {
    if output.len() >= RUNTIME_OUTPUT_SCALAR_BUDGET {
        return Err("turn_output_scalar_ordinal_budget");
    }
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return collect_scalar_values_bounded(&value, 0, output, RUNTIME_OUTPUT_SCALAR_BUDGET);
    }
    let embedded = runtime_embedded_json_objects(text);
    if !embedded.is_empty() {
        for object in embedded {
            collect_scalar_values_bounded(
                &Value::Object(object),
                0,
                output,
                RUNTIME_OUTPUT_SCALAR_BUDGET,
            )?;
        }
        return Ok(());
    }
    collect_plain_text_scalars(text, output)
}

pub(crate) fn observed_output_scalar_type_counts(
    output: &Value,
) -> Result<BTreeMap<AtomValueType, usize>, &'static str> {
    let mut scalars = Vec::new();
    for text in output_text_parts(output)? {
        collect_runtime_output_scalars(text, &mut scalars)?;
    }
    let mut counts = BTreeMap::new();
    for scalar in scalars {
        *counts.entry(scalar.value_type).or_default() += 1;
    }
    Ok(counts)
}

fn collect_plain_text_scalars(
    text: &str,
    output: &mut Vec<ExtractedScalar>,
) -> Result<(), &'static str> {
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
        if output.len() >= RUNTIME_OUTPUT_SCALAR_BUDGET {
            return Err("turn_output_scalar_ordinal_budget");
        }
        let end = start.saturating_add(token.len());
        let decimal_neighbor = text[..start].ends_with('.') || text[end..].starts_with('.');
        if token.bytes().all(|byte| byte.is_ascii_digit()) && !decimal_neighbor {
            if let Ok(value) = token.parse::<u64>() {
                output.push(ExtractedScalar {
                    value: Value::from(value),
                    value_type: AtomValueType::Integer,
                });
            }
        } else if token.eq_ignore_ascii_case("true") || token.eq_ignore_ascii_case("false") {
            output.push(ExtractedScalar {
                value: Value::Bool(token.eq_ignore_ascii_case("true")),
                value_type: AtomValueType::Boolean,
            });
        }
    }
    Ok(())
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

/// Bounded structural parser shared with read-only diagnostics. It does not
/// rank candidates or grant selector, execution, or admission authority.
#[doc(hidden)]
pub fn immediate_tool_output_value(payload: &Value) -> Option<&Value> {
    let item = payload.get("input")?.as_array()?.last()?;
    matches!(
        item.get("type").and_then(Value::as_str),
        Some("function_call_output" | "custom_tool_call_output")
    )
    .then(|| item.get("output"))?
}

#[doc(hidden)]
pub fn output_text_parts(output: &Value) -> Result<Vec<&str>, &'static str> {
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

#[doc(hidden)]
pub fn parse_scalar_text(
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
        AtomValueType::Collection => {
            let parsed =
                serde_json::from_str::<Value>(value).map_err(|_| "selector_collection_parse")?;
            if !matches!(parsed, Value::Array(_) | Value::Object(_)) {
                return Err("selector_collection_unsupported");
            }
            parsed
        }
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
        Value::Array(_) | Value::Object(_) => AtomValueType::Collection,
        Value::Null | Value::Number(_) => return Err("selector_scalar_unsupported"),
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

fn immediate_unique_collection(provider_payload: &Value) -> Result<ExtractedScalar, &'static str> {
    let output =
        immediate_tool_output_value(provider_payload).ok_or("immediate_tool_output_missing")?;
    let value = canonical_collection_from_provider_output(output)?;
    extracted_scalar(value, AtomValueType::Collection)
}

fn collect_scalar_values(
    value: &Value,
    depth: usize,
    scalars: &mut Vec<ExtractedScalar>,
) -> Result<(), &'static str> {
    collect_scalar_values_bounded(value, depth, scalars, 64)
}

fn collect_scalar_values_bounded(
    value: &Value,
    depth: usize,
    scalars: &mut Vec<ExtractedScalar>,
    scalar_budget: usize,
) -> Result<(), &'static str> {
    if depth > 8 || scalars.len() >= scalar_budget {
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
                collect_scalar_values_bounded(value, depth + 1, scalars, scalar_budget)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_scalar_values_bounded(value, depth + 1, scalars, scalar_budget)?;
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
