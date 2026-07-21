use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

pub(crate) fn stable_atom_id(atom: &str) -> u64 {
    atom.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

pub(crate) fn request_phase_atom_ids(text: &str) -> Vec<u64> {
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

fn provider_tool_capability_atom_ids(provider_payload: &Value) -> Vec<u64> {
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

pub(crate) fn response_pre_action_context_atom_ids(provider_payload: &Value) -> Vec<u64> {
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

fn response_pre_action_context_counts(provider_payload: &Value) -> BTreeMap<String, u32> {
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

fn classify_yielded_surface(source: &str) -> &'static str {
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

fn build_or_test_command(source: &str) -> bool {
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
