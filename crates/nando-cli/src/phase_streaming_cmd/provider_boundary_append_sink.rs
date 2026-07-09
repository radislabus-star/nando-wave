use std::fs::OpenOptions;
use std::io::{self, BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_PROVIDER_BOUNDARY_APPEND_SINK_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-append-sink-v1.report.json";
const DEFAULT_PROVIDER_BOUNDARY_APPEND_SINK_JSONL: &str =
    "target/nando-wave/streaming/provider-boundary-append-sink-v1.provider.jsonl";

#[derive(Default)]
struct AppendSinkState {
    input_rows: usize,
    rows_with_join_keys: usize,
    rows_with_provider_keys: usize,
    appended_rows: usize,
    skipped_missing_join_keys: usize,
    skipped_missing_provider_keys: usize,
    skipped_provider_key_atom_leak: usize,
    appended_total_tokens: usize,
    appended_total_cost_microusd: u64,
}

pub(crate) fn run_phase_stream_provider_boundary_append_sink_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_APPEND_SINK_REPORT));
    let append_provider_boundary_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_APPEND_SINK_JSONL));
    let input_paths = args.collect::<Vec<_>>();
    if input_paths.is_empty() {
        return Err(
            "at least one provider event JSONL path or '-' stdin marker is required".to_owned(),
        );
    }

    let output_file = open_append_file(&append_provider_boundary_path)?;
    let mut writer = BufWriter::new(output_file);
    let mut state = AppendSinkState::default();

    for input_path in &input_paths {
        if input_path == "-" {
            let stdin = io::stdin();
            read_provider_event_rows("<stdin>", stdin.lock(), &mut writer, &mut state)?;
        } else {
            let path = PathBuf::from(input_path);
            let file = std::fs::File::open(&path).map_err(|error| {
                format!(
                    "failed to open provider event '{}': {error}",
                    path.display()
                )
            })?;
            let reader = io::BufReader::new(file);
            read_provider_event_rows(&path.display().to_string(), reader, &mut writer, &mut state)?;
        }
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush provider-boundary append sink '{}': {error}",
            append_provider_boundary_path.display()
        )
    })?;

    let metadata_only = state.skipped_provider_key_atom_leak == 0;
    let append_capture_ready = state.appended_rows > 0 && metadata_only;
    let mut blockers = Vec::<&'static str>::new();
    if state.input_rows == 0 {
        blockers.push("no_input_rows");
    }
    if state.rows_with_join_keys == 0 {
        blockers.push("no_rows_with_stable_join_keys");
    }
    if state.rows_with_provider_keys == 0 {
        blockers.push("no_rows_with_provider_keys");
    }
    if state.appended_rows == 0 {
        blockers.push("no_appended_provider_boundary_rows");
    }
    if !metadata_only {
        blockers.push("provider_key_atom_leak");
    }

    let verdict = if !metadata_only {
        "PHASE_STREAM_PROVIDER_BOUNDARY_APPEND_SINK_V1_FAIL_PROVIDER_KEY_ATOM_LEAK"
    } else if append_capture_ready {
        "PHASE_STREAM_PROVIDER_BOUNDARY_APPEND_SINK_V1_READY_FOR_CAPTURE_COVERAGE_GATE"
    } else {
        "PHASE_STREAM_PROVIDER_BOUNDARY_APPEND_SINK_V1_WATCH_NO_APPENDABLE_PROVIDER_ROWS"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_provider_boundary_append_sink_v1",
        "input_paths": input_paths,
        "append_provider_boundary_path": path_string(&append_provider_boundary_path),
        "input_rows": state.input_rows,
        "rows_with_join_keys": state.rows_with_join_keys,
        "rows_with_provider_keys": state.rows_with_provider_keys,
        "appended_rows": state.appended_rows,
        "skipped_missing_join_keys": state.skipped_missing_join_keys,
        "skipped_missing_provider_keys": state.skipped_missing_provider_keys,
        "skipped_provider_key_atom_leak": state.skipped_provider_key_atom_leak,
        "appended_total_tokens": state.appended_total_tokens,
        "appended_total_cost_microusd": state.appended_total_cost_microusd,
        "readiness": {
            "provider_correlation_metadata_only": metadata_only,
            "append_capture_ready": append_capture_ready,
            "market_money_claim_allowed": false,
            "local_accept_enabled": false,
            "policy": "append-only sink for live/provider boundary metadata; emits rows only when stable join keys and provider correlation keys are present"
        },
        "blockers": blockers,
        "manual_class_list_used": false,
        "selector_used": false,
        "dynamic_discovery_performed": false,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false
        },
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "product_promotion_allowed": false,
        "market_money_claim_allowed": false,
        "verdict": verdict,
        "boundary": "append-only provider-boundary sink: reads provider event rows and appends metadata-only correlation rows; no mining, no scoring, no .nwpc compile, no serving, no promotion, no local accept, no money claim"
    });
    super::write_json_file(&report_path, &report)?;

    println!("phase_stream_provider_boundary_append_sink_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  append_provider_boundary_path: {}",
        append_provider_boundary_path.display()
    );
    println!("  input_rows: {}", state.input_rows);
    println!("  appended_rows: {}", state.appended_rows);
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn read_provider_event_rows<R: BufRead>(
    source_label: &str,
    reader: R,
    writer: &mut BufWriter<std::fs::File>,
    state: &mut AppendSinkState,
) -> Result<(), String> {
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| {
            format!(
                "failed to read provider event '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse provider event '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        if !row.is_object() {
            continue;
        }
        state.input_rows += 1;
        let join_keys = match_keys(&row);
        let provider_keys = super::phase_atom_external_provider_correlation_keys(&row);
        state.rows_with_join_keys += usize::from(!join_keys.is_empty());
        state.rows_with_provider_keys += usize::from(!provider_keys.is_empty());
        if provider_key_leaks_into_atoms(&row) {
            state.skipped_provider_key_atom_leak += 1;
            continue;
        }
        if join_keys.is_empty() {
            state.skipped_missing_join_keys += 1;
            continue;
        }
        if provider_keys.is_empty() {
            state.skipped_missing_provider_keys += 1;
            continue;
        }

        let total_tokens = provider_total_tokens(&row).unwrap_or(0);
        let total_cost_microusd = provider_cost_microusd(&row).unwrap_or(0);
        let output_row = provider_boundary_row(
            &join_keys,
            &provider_keys,
            source_label,
            line_index + 1,
            total_tokens,
            total_cost_microusd,
        );
        serde_json::to_writer(&mut *writer, &output_row).map_err(|error| {
            format!("failed to serialize provider-boundary append row: {error}")
        })?;
        writer
            .write_all(b"\n")
            .map_err(|error| format!("failed to write provider-boundary append row: {error}"))?;
        state.appended_rows += 1;
        state.appended_total_tokens = state.appended_total_tokens.saturating_add(total_tokens);
        state.appended_total_cost_microusd = state
            .appended_total_cost_microusd
            .saturating_add(total_cost_microusd);
    }
    Ok(())
}

fn provider_boundary_row(
    join_keys: &[String],
    provider_keys: &[String],
    source_label: &str,
    source_line: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
) -> Value {
    let mut row = serde_json::json!({
        "schema_version": "provider_boundary_append_sink_v1",
        "match_keys": join_keys,
        "external_provider_correlation_keys": provider_keys,
        "provider_correlation_ready": true,
        "source_label": source_label,
        "source_line": source_line,
        "token_cost": {
            "total_tokens": total_tokens,
            "total_cost_microusd": total_cost_microusd,
            "token_evidence_missing": total_tokens == 0,
            "cost_evidence_missing": total_cost_microusd == 0
        },
        "local_accept_enabled": false,
        "market_money_claim_allowed": false,
        "boundary": "provider-boundary append row; provider ids are metadata only, not atoms"
    });
    if let Some(map) = row.as_object_mut() {
        for key in join_keys {
            if let Some((label, value)) = key.split_once(':') {
                match label {
                    "request_fingerprint"
                    | "exact_cache_key"
                    | "trace_id"
                    | "event_id"
                    | "client_correlation_id" => {
                        map.entry(label.to_owned())
                            .or_insert_with(|| Value::String(value.to_owned()));
                    }
                    _ => {}
                }
            }
        }
    }
    row
}

fn match_keys(row: &Value) -> Vec<String> {
    let mut keys = Vec::new();
    keys.extend(json_string_vec(json_at(row, &["match_keys"])));
    keys.extend(json_string_vec(json_at(row, &["metadata", "match_keys"])));
    push_match_key_paths(
        &mut keys,
        row,
        "request_fingerprint",
        &[
            &["request_fingerprint"],
            &["metadata", "request_fingerprint"],
        ],
    );
    push_match_key_paths(
        &mut keys,
        row,
        "exact_cache_key",
        &[
            &["exact_cache_key"],
            &["metadata", "exact_cache_key"],
            &["cache_key"],
        ],
    );
    push_match_key_paths(
        &mut keys,
        row,
        "trace_id",
        &[&["trace_id"], &["metadata", "trace_id"]],
    );
    push_match_key_paths(
        &mut keys,
        row,
        "event_id",
        &[&["event_id"], &["metadata", "event_id"]],
    );
    push_match_key_paths(
        &mut keys,
        row,
        "client_correlation_id",
        &[
            &["client_correlation_id"],
            &["metadata", "client_correlation_id"],
        ],
    );
    keys.sort();
    keys.dedup();
    keys
}

fn push_match_key_paths(keys: &mut Vec<String>, row: &Value, prefix: &str, paths: &[&[&str]]) {
    for path in paths {
        if let Some(value) = json_string(row, path).filter(|value| !value.is_empty()) {
            if value.contains(':') && value.starts_with(prefix) {
                keys.push(value);
            } else {
                keys.push(format!("{prefix}:{value}"));
            }
        }
    }
}

fn provider_total_tokens(row: &Value) -> Option<usize> {
    for path in [
        &["provider_total_tokens"][..],
        &["total_tokens"],
        &["tokens_total"],
        &["token_cost", "total_tokens"],
    ] {
        if let Some(value) = json_usize(row, path) {
            return Some(value);
        }
    }
    let component_sum = [
        &["input_tokens"][..],
        &["output_tokens"],
        &["cached_input_tokens"],
        &["token_cost", "input_tokens"],
        &["token_cost", "output_tokens"],
        &["token_cost", "cached_input_tokens"],
    ]
    .into_iter()
    .filter_map(|path| json_usize(row, path))
    .sum::<usize>();
    if component_sum > 0 {
        Some(component_sum)
    } else {
        None
    }
}

fn provider_cost_microusd(row: &Value) -> Option<u64> {
    for path in [
        &["provider_cost_microusd"][..],
        &["cost_microusd"],
        &["total_cost_microusd"],
        &["token_cost", "provider_cost_microusd"],
        &["token_cost", "total_cost_microusd"],
    ] {
        if let Some(value) = json_u64(row, path) {
            return Some(value);
        }
    }
    for path in [
        &["provider_cost_usd"][..],
        &["total_cost_usd"],
        &["cost_usd"],
        &["token_cost", "provider_cost_usd"],
        &["token_cost", "total_cost_usd"],
    ] {
        if let Some(value) = json_f64(row, path) {
            if value.is_finite() && value > 0.0 {
                return Some((value * 1_000_000.0).round() as u64);
            }
        }
    }
    None
}

fn provider_key_leaks_into_atoms(row: &Value) -> bool {
    let mut atoms = Vec::new();
    collect_atom_strings(row.get("atom_groups"), &mut atoms);
    for key in [
        "request_atoms",
        "state_atoms",
        "action_atoms",
        "tool_atoms",
        "route_hint_atoms",
        "metadata_atoms",
    ] {
        collect_atom_strings(row.get(key), &mut atoms);
    }
    atoms.iter().any(|atom| {
        atom.starts_with("provider_correlation:")
            || atom.starts_with("provider_request_id:")
            || atom.starts_with("provider_response_id:")
            || atom.starts_with("provider_trace_id:")
            || atom.starts_with("external_provider_request_id:")
            || atom.starts_with("openai_request_id:")
            || atom.starts_with("anthropic_request_id:")
            || atom.starts_with("custom_id:")
    })
}

fn collect_atom_strings(value: Option<&Value>, output: &mut Vec<String>) {
    match value {
        Some(Value::String(text)) => output.push(text.clone()),
        Some(Value::Array(items)) => {
            for item in items {
                collect_atom_strings(Some(item), output);
            }
        }
        Some(Value::Object(map)) => {
            for item in map.values() {
                collect_atom_strings(Some(item), output);
            }
        }
        _ => {}
    }
}

fn open_append_file(path: &Path) -> Result<std::fs::File, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("failed to open append log '{}': {error}", path.display()))
}

fn json_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    json_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn json_string_vec(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(text)) if !text.is_empty() => split_match_key_text(text),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .flat_map(split_match_key_text)
            .filter(|value| !value.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn split_match_key_text(text: &str) -> Vec<String> {
    text.split([',', ';', '|', '\n'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_at(value, path)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    json_at(value, path).and_then(Value::as_u64)
}

fn json_f64(value: &Value, path: &[&str]) -> Option<f64> {
    json_at(value, path).and_then(Value::as_f64)
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}
