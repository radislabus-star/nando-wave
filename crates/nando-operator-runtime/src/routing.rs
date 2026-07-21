use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    AtomValueType, ResponseArgument, ResponseOperation, ResponseProgram, ResponseValueSelector,
    SemanticRole, canonical_response_value_selector, stable_atom_id,
};
use serde_json::Value;

pub fn request_phase_atom_ids(text: &str) -> Vec<u64> {
    let all_tokens = text
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|token| !token.is_empty() && token.len() <= 32)
        .map(str::to_lowercase)
        .take(256)
        .collect::<Vec<_>>();
    let tokens = if all_tokens.len() <= 64 {
        all_tokens
    } else {
        all_tokens[..32]
            .iter()
            .chain(&all_tokens[all_tokens.len().saturating_sub(32)..])
            .cloned()
            .collect()
    };
    let mut atoms = tokens
        .iter()
        .map(|token| stable_atom_id(&format!("request_token:{token}")))
        .collect::<Vec<_>>();
    atoms.extend(
        tokens
            .windows(2)
            .map(|pair| stable_atom_id(&format!("request_bigram:{}:{}", pair[0], pair[1]))),
    );
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

pub fn provider_tool_capability_atom_ids(provider_payload: &Value) -> Vec<u64> {
    let mut declarations = Vec::new();
    if let Some(tools) = provider_payload.get("tools").and_then(Value::as_array) {
        declarations.extend(tools.iter());
    }
    if let Some(input) = provider_payload.get("input").and_then(Value::as_array) {
        for item in input
            .iter()
            .filter(|item| item.get("type").and_then(Value::as_str) == Some("additional_tools"))
        {
            for key in ["tools", "additional_tools", "definitions", "items"] {
                if let Some(tools) = item.get(key).and_then(Value::as_array) {
                    declarations.extend(tools.iter());
                }
            }
        }
    }
    let mut atoms = declarations
        .into_iter()
        .filter_map(|declaration| {
            let raw_kind = declaration
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("function");
            let kind = match raw_kind {
                "custom" | "custom_tool" => "custom",
                "function" => "function",
                other => other,
            };
            let name = declaration
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| {
                    declaration
                        .pointer("/function/name")
                        .and_then(Value::as_str)
                })?;
            valid_capability_symbol(kind)
                .then_some(())
                .filter(|_| valid_capability_symbol(name))
                .map(|()| stable_atom_id(&format!("client_capability:{kind}:{name}")))
        })
        .collect::<Vec<_>>();
    atoms.sort_unstable();
    atoms.dedup();
    atoms.truncate(64);
    atoms
}

fn valid_capability_symbol(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/' | b':')
        })
}

#[doc(hidden)]
pub fn response_pre_action_context_atom_ids(provider_payload: &Value) -> Vec<u64> {
    let mut atoms = response_pre_action_context_counts(provider_payload)
        .into_iter()
        .filter(|(role, _)| role != "active_pending_handle_count_band")
        .map(|(role, count)| stable_atom_id(&format!("cardinality:{role}:{count}")))
        .collect::<Vec<_>>();
    atoms.extend(provider_tool_capability_atom_ids(provider_payload));
    atoms.extend(response_pre_action_tool_atom_ids(provider_payload));
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

fn response_pre_action_tool_atom_ids(provider_payload: &Value) -> Vec<u64> {
    let Some(input) = provider_payload.get("input").and_then(Value::as_array) else {
        return Vec::new();
    };
    let start = input
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    input[start..]
        .iter()
        .filter_map(|item| {
            let item_type = item.get("type").and_then(Value::as_str)?;
            matches!(item_type, "function_call" | "custom_tool_call")
                .then(|| item.get("name").and_then(Value::as_str))
                .flatten()
                .map(|name| stable_atom_id(&format!("tool_kind:{item_type}:{name}")))
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[doc(hidden)]
pub fn response_pre_action_context_counts(provider_payload: &Value) -> BTreeMap<String, u32> {
    let Some(input) = provider_payload.get("input").and_then(Value::as_array) else {
        return BTreeMap::new();
    };
    let start = input
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    let mut calls = 0_usize;
    let mut outputs = 0_usize;
    let mut pending_outputs = 0_usize;
    let mut messages = 0_usize;
    let mut call_shapes = BTreeSet::new();
    let mut active_pending_handles = BTreeSet::new();
    let mut wait_calls = BTreeMap::<String, String>::new();
    for item in &input[start..] {
        let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");
        match item_type {
            "custom_tool_call" | "function_call" => {
                calls = calls.saturating_add(1);
                let name = item
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unnamed");
                call_shapes.insert(format!("{item_type}:{name}"));
                if item_type == "function_call"
                    && name == "wait"
                    && let (Some(call_id), Some(arguments)) = (
                        item.get("call_id").and_then(Value::as_str),
                        item.get("arguments").and_then(Value::as_str),
                    )
                    && let Some(cell_id) =
                        serde_json::from_str::<Value>(arguments)
                            .ok()
                            .and_then(|value| {
                                value
                                    .get("cell_id")
                                    .and_then(Value::as_str)
                                    .map(str::to_owned)
                            })
                {
                    wait_calls.insert(call_id.to_owned(), cell_id);
                }
            }
            "custom_tool_call_output" | "function_call_output" => {
                outputs = outputs.saturating_add(1);
                let output = item.get("output").unwrap_or(&Value::Null);
                if let Some(handle) = item
                    .get("call_id")
                    .and_then(Value::as_str)
                    .and_then(|call_id| wait_calls.remove(call_id))
                {
                    active_pending_handles.remove(&handle);
                }
                if value_contains_pending_cell(output) {
                    pending_outputs = pending_outputs.saturating_add(1);
                    if let Some(handle) = pending_cell_handle(output) {
                        active_pending_handles.insert(handle);
                    }
                }
            }
            "message" if item.get("role").and_then(Value::as_str) == Some("assistant") => {
                messages = messages.saturating_add(1);
            }
            _ => {}
        }
    }
    [
        ("turn_call_count_band", count_band(calls)),
        ("turn_output_count_band", count_band(outputs)),
        ("turn_pending_count_band", count_band(pending_outputs)),
        ("turn_message_count_band", count_band(messages)),
        ("turn_call_shape_count_band", count_band(call_shapes.len())),
        (
            "active_pending_handle_count_band",
            count_band(active_pending_handles.len()),
        ),
    ]
    .into_iter()
    .map(|(role, count)| (role.to_owned(), count as u32))
    .collect()
}

fn value_contains_pending_cell(value: &Value) -> bool {
    match value {
        Value::String(text) => text.starts_with("Script running with cell ID "),
        Value::Array(items) => items.iter().any(value_contains_pending_cell),
        Value::Object(object) => object.get("text").is_some_and(value_contains_pending_cell),
        _ => false,
    }
}

fn pending_cell_handle(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => text
            .strip_prefix("Script running with cell ID ")
            .and_then(|rest| rest.split_whitespace().next())
            .filter(|handle| !handle.is_empty())
            .map(str::to_owned),
        Value::Array(items) => items.iter().find_map(pending_cell_handle),
        Value::Object(object) => object.get("text").and_then(pending_cell_handle),
        _ => None,
    }
}

const fn count_band(value: usize) -> usize {
    if value == 0 {
        0
    } else {
        1_usize << (usize::BITS - 1 - value.leading_zeros())
    }
}

#[doc(hidden)]
pub fn response_program_grounded_routing_atom_ids(
    program: &ResponseProgram,
    provider_payload: &Value,
) -> Vec<u64> {
    let mut atoms = match &program.operation {
        ResponseOperation::FunctionCallFromRoles {
            selector,
            arguments,
            ..
        } => response_phase_atom_ids_for_grounded_function_call_payload(
            provider_payload,
            selector,
            arguments,
        ),
        ResponseOperation::CustomToolCallFromRoles { selector, .. } => {
            response_phase_atom_ids_for_custom_tool_call_payload(provider_payload, selector)
        }
        _ => return Vec::new(),
    };
    if atoms.is_empty() {
        return atoms;
    }
    atoms.extend(response_pre_action_context_atom_ids(provider_payload));
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

#[doc(hidden)]
pub fn response_phase_atom_ids_for_grounded_function_call_payload(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
    arguments: &[ResponseArgument],
) -> Vec<u64> {
    let Ok(scalar) = crate::immediate_selected_scalar(provider_payload, selector) else {
        return Vec::new();
    };
    let completion = if arguments.iter().any(|argument| {
        matches!(
            argument,
            ResponseArgument::Role {
                role: SemanticRole::ContinuationHandle,
                ..
            }
        )
    }) {
        "pending"
    } else {
        "completed"
    };
    let mut atoms = vec![
        stable_atom_id("relation:tool_kind"),
        stable_atom_id(&format!("completion:{completion}")),
        stable_atom_id(&format!(
            "slot:{}:observation",
            value_type_name(scalar.value_type)
        )),
        stable_atom_id("relation:unique_slot"),
        selector_phase_atom_id(selector),
    ];
    if let Some(shape) = immediate_observation_call_shape(provider_payload) {
        atoms.push(stable_atom_id(&format!("observation_call_shape:{shape}")));
    }
    if let Some(tool_kind) = immediate_observation_tool_kind(provider_payload) {
        atoms.push(stable_atom_id(&format!("tool_kind:{tool_kind}")));
    }
    atoms
}

#[doc(hidden)]
pub fn response_phase_atom_ids_for_custom_tool_call_payload(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
) -> Vec<u64> {
    let Ok(scalar) = crate::immediate_selected_scalar(provider_payload, selector) else {
        return Vec::new();
    };
    let mut atoms = vec![
        stable_atom_id("relation:tool_kind"),
        stable_atom_id("completion:pending"),
        stable_atom_id(&format!(
            "slot:{}:observation",
            value_type_name(scalar.value_type)
        )),
        stable_atom_id("relation:unique_slot"),
        selector_phase_atom_id(selector),
    ];
    if let Some(shape) = immediate_observation_call_shape(provider_payload) {
        atoms.push(stable_atom_id(&format!("observation_call_shape:{shape}")));
    }
    if let Some(tool_kind) = immediate_observation_tool_kind(provider_payload) {
        atoms.push(stable_atom_id(&format!("tool_kind:{tool_kind}")));
    }
    atoms
}

#[doc(hidden)]
pub fn immediate_observation_call_shape(provider_payload: &Value) -> Option<String> {
    let input = provider_payload.get("input")?.as_array()?;
    let (output_index, call_id) = input.iter().enumerate().rev().find_map(|(index, item)| {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        ) {
            return None;
        }
        item.get("call_id")
            .and_then(Value::as_str)
            .map(|call_id| (index, call_id))
    })?;
    input[..output_index].iter().rev().find_map(|item| {
        let item_type = item.get("type").and_then(Value::as_str)?;
        if !matches!(item_type, "function_call" | "custom_tool_call")
            || item.get("call_id").and_then(Value::as_str) != Some(call_id)
        {
            return None;
        }
        item.get("name").and_then(Value::as_str)?;
        Some(item_type.to_owned())
    })
}

#[doc(hidden)]
pub fn immediate_observation_tool_kind(provider_payload: &Value) -> Option<String> {
    let input = provider_payload.get("input")?.as_array()?;
    let (output_index, call_id) = input.iter().enumerate().rev().find_map(|(index, item)| {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        ) {
            return None;
        }
        item.get("call_id")
            .and_then(Value::as_str)
            .map(|call_id| (index, call_id))
    })?;
    input[..output_index].iter().rev().find_map(|item| {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call" | "custom_tool_call")
        ) || item.get("call_id").and_then(Value::as_str) != Some(call_id)
        {
            return None;
        }
        item.get("name").and_then(Value::as_str).map(str::to_owned)
    })
}

#[doc(hidden)]
pub fn selector_phase_atom_id(selector: &ResponseValueSelector) -> u64 {
    let canonical = canonical_response_value_selector(selector);
    stable_atom_id_parts(&["selector:", &canonical])
}

#[doc(hidden)]
pub fn stable_atom_id_parts(parts: &[&str]) -> u64 {
    let mut value = 0xcbf2_9ce4_8422_2325_u64;
    for byte in parts.iter().flat_map(|part| part.bytes()) {
        value = (value ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
    }
    value
}

const fn value_type_name(value: AtomValueType) -> &'static str {
    match value {
        AtomValueType::String => "string",
        AtomValueType::Integer => "integer",
        AtomValueType::Boolean => "boolean",
        AtomValueType::Identifier => "identifier",
        AtomValueType::Collection => "collection",
    }
}
