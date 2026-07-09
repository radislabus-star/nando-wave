use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_REALTRACE_TOKEN_COST_BACKFILL_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-realtrace-token-cost-backfill-v1.report.json";
const DEFAULT_REALTRACE_TOKEN_COST_BACKFILL_PROVIDER_JSONL: &str =
    "target/nando-wave/streaming/provider-boundary-realtrace-token-cost-backfill-v1.provider.jsonl";

#[derive(Clone, Default)]
struct CaptureRequest {
    capture_request_id: String,
    join_keys: BTreeSet<String>,
    total_tokens: usize,
    total_cost_microusd: u64,
}

#[derive(Default)]
struct CaptureIndex {
    requests: BTreeMap<String, CaptureRequest>,
    key_to_capture_ids: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Default)]
struct BackfillState {
    capture_request_rows: usize,
    capture_requests_with_join_keys: usize,
    phase_trace_rows: usize,
    phase_rows_matching_capture: usize,
    phase_rows_with_token_cost: usize,
    phase_rows_with_positive_tokens: usize,
    emitted_provider_boundary_rows: usize,
    unique_capture_requests_emitted: BTreeSet<String>,
    missing_phase_source_capture_ids: BTreeSet<String>,
    missing_token_cost_capture_ids: BTreeSet<String>,
    appended_total_tokens: usize,
    appended_total_cost_microusd: u64,
    provider_key_atom_leak_rows: usize,
}

pub(crate) fn run_phase_stream_provider_boundary_realtrace_token_cost_backfill_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REALTRACE_TOKEN_COST_BACKFILL_REPORT));
    let output_provider_boundary_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REALTRACE_TOKEN_COST_BACKFILL_PROVIDER_JSONL));
    let capture_request_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "capture-request JSONL path is required".to_owned())?;
    let phase_trace_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if phase_trace_paths.is_empty() {
        return Err("at least one phase-atom trace JSONL path is required".to_owned());
    }

    let mut state = BackfillState::default();
    let capture_index = read_capture_index(&capture_request_path, &mut state)?;
    let mut output = String::new();
    let mut captures_with_phase_source = BTreeSet::<String>::new();

    for path in &phase_trace_paths {
        let text = read_text(path)?;
        for (line_index, line) in text.lines().enumerate() {
            let Some(row) = parse_json_row(path, line_index + 1, line)? else {
                continue;
            };
            state.phase_trace_rows += 1;
            if provider_key_leaks_into_atoms(&row) {
                state.provider_key_atom_leak_rows += 1;
            }

            let join_keys = metadata_join_keys(&row);
            let matched_capture_ids = matched_capture_ids(&capture_index, &join_keys);
            if matched_capture_ids.is_empty() {
                continue;
            }
            state.phase_rows_matching_capture += 1;
            captures_with_phase_source.extend(matched_capture_ids.iter().cloned());

            let total_tokens = json_usize(&row, &["token_cost", "total_tokens"]).unwrap_or(0);
            let total_cost_microusd =
                json_u64(&row, &["token_cost", "total_cost_microusd"]).unwrap_or(0);
            let token_evidence_missing = json_bool(&row, &["token_cost", "token_evidence_missing"])
                .unwrap_or(total_tokens == 0);
            if row.get("token_cost").is_some() {
                state.phase_rows_with_token_cost += 1;
            }
            if total_tokens > 0 {
                state.phase_rows_with_positive_tokens += 1;
            }
            if token_evidence_missing || total_tokens == 0 {
                state
                    .missing_token_cost_capture_ids
                    .extend(matched_capture_ids.iter().cloned());
                continue;
            }

            for capture_id in matched_capture_ids {
                if state.unique_capture_requests_emitted.contains(&capture_id) {
                    continue;
                }
                let capture = capture_index
                    .requests
                    .get(&capture_id)
                    .ok_or_else(|| format!("internal missing capture id '{capture_id}'"))?;
                let provider_row = provider_boundary_row(
                    capture,
                    &row,
                    path,
                    line_index + 1,
                    total_tokens,
                    total_cost_microusd,
                );
                output.push_str(&serde_json::to_string(&provider_row).map_err(|error| {
                    format!("failed to serialize realtrace token-cost boundary row: {error}")
                })?);
                output.push('\n');
                state.emitted_provider_boundary_rows += 1;
                state
                    .unique_capture_requests_emitted
                    .insert(capture_id.clone());
                state.appended_total_tokens =
                    state.appended_total_tokens.saturating_add(total_tokens);
                state.appended_total_cost_microusd = state
                    .appended_total_cost_microusd
                    .saturating_add(total_cost_microusd);
            }
        }
    }

    for capture_id in capture_index.requests.keys() {
        if !captures_with_phase_source.contains(capture_id) {
            state
                .missing_phase_source_capture_ids
                .insert(capture_id.clone());
        }
    }

    write_text_file(&output_provider_boundary_path, &output)?;

    let full_capture_coverage = state.emitted_provider_boundary_rows == state.capture_request_rows
        && state.capture_request_rows > 0;
    let provider_correlation_metadata_only = state.provider_key_atom_leak_rows == 0;
    let verdict = if !provider_correlation_metadata_only {
        "PHASE_STREAM_PROVIDER_BOUNDARY_REALTRACE_TOKEN_COST_BACKFILL_V1_FAIL_PROVIDER_KEY_ATOM_LEAK"
    } else if full_capture_coverage {
        "PHASE_STREAM_PROVIDER_BOUNDARY_REALTRACE_TOKEN_COST_BACKFILL_V1_PASS_TOKEN_COST_BOUNDARY_COVERAGE"
    } else if state.emitted_provider_boundary_rows > 0 {
        "PHASE_STREAM_PROVIDER_BOUNDARY_REALTRACE_TOKEN_COST_BACKFILL_V1_WATCH_PARTIAL_TOKEN_COST_BOUNDARY_COVERAGE"
    } else {
        "PHASE_STREAM_PROVIDER_BOUNDARY_REALTRACE_TOKEN_COST_BACKFILL_V1_WATCH_NO_TOKEN_COST_BOUNDARY_COVERAGE"
    };

    let missing_phase_source = state.missing_phase_source_capture_ids.len();
    let missing_token_cost = state.missing_token_cost_capture_ids.len();
    let report = serde_json::json!({
        "report_kind": "phase_stream_provider_boundary_realtrace_token_cost_backfill_v1",
        "capture_request_path": path_string(&capture_request_path),
        "phase_trace_paths": phase_trace_paths.iter().map(path_string).collect::<Vec<_>>(),
        "output_provider_boundary_path": path_string(&output_provider_boundary_path),
        "token_boundary_source": "realtrace_embedded_token_cost",
        "token_boundary_join_policy": {
            "direction": "same_phase_atom_trace_row_token_cost",
            "provider_request_id_available": false,
            "cost_evidence_available": false,
            "market_money_claim_allowed": false
        },
        "scoreboard": {
            "capture_request_rows": state.capture_request_rows,
            "capture_requests_with_join_keys": state.capture_requests_with_join_keys,
            "phase_trace_rows": state.phase_trace_rows,
            "phase_rows_matching_capture": state.phase_rows_matching_capture,
            "phase_rows_with_token_cost": state.phase_rows_with_token_cost,
            "phase_rows_with_positive_tokens": state.phase_rows_with_positive_tokens,
            "emitted_provider_boundary_rows": state.emitted_provider_boundary_rows,
            "unique_capture_requests_emitted": state.unique_capture_requests_emitted.len(),
            "missing_phase_source_capture_requests": missing_phase_source,
            "missing_token_cost_capture_requests": missing_token_cost,
            "appended_total_tokens": state.appended_total_tokens,
            "appended_total_cost_microusd": state.appended_total_cost_microusd,
            "full_capture_coverage": full_capture_coverage,
            "provider_key_atom_leak_rows": state.provider_key_atom_leak_rows
        },
        "readiness": {
            "provider_correlation_metadata_only": provider_correlation_metadata_only,
            "local_accept_enabled": false,
            "market_money_claim_allowed": false,
            "product_promotion_allowed": false,
            "policy": "Realtrace embedded token-cost metadata only: emits provider-boundary metadata rows for denominator coverage; no external provider request id, no provider cost evidence, no serving, no local accept, no money claim"
        },
        "blockers": blockers(&state, full_capture_coverage, provider_correlation_metadata_only),
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
        "boundary": "source adapter only: converts embedded realtrace token_cost into provider-boundary metadata rows for coverage audit; does not alter atoms, mine, score, compile, promote, serve, local_accept, or claim money"
    });
    super::write_json_file(&report_path, &report)?;

    println!("phase_stream_provider_boundary_realtrace_token_cost_backfill_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  output_provider_boundary_path: {}",
        output_provider_boundary_path.display()
    );
    println!("  capture_request_rows: {}", state.capture_request_rows);
    println!(
        "  emitted_provider_boundary_rows: {}",
        state.emitted_provider_boundary_rows
    );
    println!("  appended_total_tokens: {}", state.appended_total_tokens);
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn read_capture_index(path: &Path, state: &mut BackfillState) -> Result<CaptureIndex, String> {
    let text = read_text(path)?;
    let mut index = CaptureIndex::default();
    for (line_index, line) in text.lines().enumerate() {
        let Some(row) = parse_json_row(path, line_index + 1, line)? else {
            continue;
        };
        state.capture_request_rows += 1;
        let capture_request_id = json_string(&row, &["capture_request_id"])
            .unwrap_or_else(|| format!("capture_request_line_{}", line_index + 1));
        let join_keys = metadata_join_keys(&row)
            .into_iter()
            .collect::<BTreeSet<_>>();
        if !join_keys.is_empty() {
            state.capture_requests_with_join_keys += 1;
        }
        let capture = CaptureRequest {
            capture_request_id: capture_request_id.clone(),
            join_keys: join_keys.clone(),
            total_tokens: json_usize(&row, &["total_tokens"]).unwrap_or(0),
            total_cost_microusd: json_u64(&row, &["total_cost_microusd"]).unwrap_or(0),
        };
        for key in join_keys {
            index
                .key_to_capture_ids
                .entry(key)
                .or_default()
                .insert(capture_request_id.clone());
        }
        index.requests.insert(capture_request_id, capture);
    }
    Ok(index)
}

fn provider_boundary_row(
    capture: &CaptureRequest,
    row: &Value,
    phase_path: &Path,
    phase_line_number: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
) -> Value {
    let request_fingerprint = json_string(row, &["request_fingerprint"]);
    let exact_cache_key = json_string(row, &["exact_cache_key"]);
    let trace_id = json_string(row, &["trace_id"]);
    let provider_key = format!(
        "realtrace_token_cost_event:{:016x}",
        super::stable_fingerprint([
            path_string(phase_path).as_str(),
            phase_line_number.to_string().as_str(),
            request_fingerprint.as_deref().unwrap_or(""),
            exact_cache_key.as_deref().unwrap_or(""),
            trace_id.as_deref().unwrap_or(""),
        ])
    );
    let mut out = serde_json::json!({
        "schema_version": "provider_boundary_realtrace_token_cost_backfill_v1",
        "capture_request_id": capture.capture_request_id,
        "match_keys": capture.join_keys.iter().cloned().collect::<Vec<_>>(),
        "external_provider_correlation_keys": [provider_key],
        "provider_correlation_ready": true,
        "token_boundary_source": "realtrace_embedded_token_cost",
        "token_boundary_join_policy": "same_phase_atom_trace_row_token_cost",
        "phase_trace_path": path_string(phase_path),
        "phase_line_number": phase_line_number,
        "phase_event_time_ms": json_i64(row, &["time_ms"]).unwrap_or(0),
        "provider_total_tokens": total_tokens,
        "input_tokens": json_usize(row, &["token_cost", "input_tokens"]).unwrap_or(0),
        "cached_input_tokens": json_usize(row, &["token_cost", "cached_input_tokens"]).unwrap_or(0),
        "output_tokens": json_usize(row, &["token_cost", "output_tokens"]).unwrap_or(0),
        "reasoning_output_tokens": json_usize(row, &["token_cost", "reasoning_output_tokens"]).unwrap_or(0),
        "capture_request_total_tokens": capture.total_tokens,
        "capture_request_total_cost_microusd": capture.total_cost_microusd,
        "token_cost": {
            "total_tokens": total_tokens,
            "input_tokens": json_usize(row, &["token_cost", "input_tokens"]).unwrap_or(0),
            "cached_input_tokens": json_usize(row, &["token_cost", "cached_input_tokens"]).unwrap_or(0),
            "output_tokens": json_usize(row, &["token_cost", "output_tokens"]).unwrap_or(0),
            "reasoning_output_tokens": json_usize(row, &["token_cost", "reasoning_output_tokens"]).unwrap_or(0),
            "total_cost_microusd": total_cost_microusd,
            "token_evidence_missing": false,
            "cost_evidence_missing": json_bool(row, &["token_cost", "cost_evidence_missing"]).unwrap_or(true),
            "token_cost_estimate_used": json_bool(row, &["token_cost", "token_cost_estimate_used"]).unwrap_or(true)
        },
        "local_accept_enabled": false,
        "market_money_claim_allowed": false,
        "boundary": "provider-boundary metadata row from embedded realtrace token_cost; no external provider request id and no provider billing evidence"
    });
    if let Some(map) = out.as_object_mut() {
        if let Some(value) = trace_id {
            map.insert("trace_id".to_owned(), Value::String(value));
        }
        if let Some(value) = request_fingerprint {
            map.insert("request_fingerprint".to_owned(), Value::String(value));
        }
        if let Some(value) = exact_cache_key {
            map.insert("exact_cache_key".to_owned(), Value::String(value));
        }
    }
    out
}

fn blockers(
    state: &BackfillState,
    full_capture_coverage: bool,
    provider_correlation_metadata_only: bool,
) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if state.capture_request_rows == 0 {
        blockers.push("no_capture_request_rows");
    }
    if state.emitted_provider_boundary_rows == 0 {
        blockers.push("no_realtrace_token_cost_boundary_rows_emitted");
    }
    if !full_capture_coverage {
        blockers.push("realtrace_token_cost_boundary_coverage_incomplete");
    }
    if !state.missing_phase_source_capture_ids.is_empty() {
        blockers.push("some_capture_requests_missing_phase_source");
    }
    if !state.missing_token_cost_capture_ids.is_empty() {
        blockers.push("some_capture_requests_missing_token_cost");
    }
    if !provider_correlation_metadata_only {
        blockers.push("provider_key_atom_leak");
    }
    blockers.push("provider_request_id_absent");
    blockers.push("provider_cost_evidence_absent");
    blockers
}

fn matched_capture_ids(index: &CaptureIndex, join_keys: &[String]) -> BTreeSet<String> {
    let mut matched = BTreeSet::new();
    for key in join_keys {
        if let Some(capture_ids) = index.key_to_capture_ids.get(key) {
            matched.extend(capture_ids.iter().cloned());
        }
    }
    matched
}

fn metadata_join_keys(row: &Value) -> Vec<String> {
    let mut keys = Vec::new();
    keys.extend(json_string_vec(row.get("join_keys")));
    keys.extend(json_string_vec(row.get("match_keys")));
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
        &[&["exact_cache_key"], &["metadata", "exact_cache_key"]],
    );
    push_match_key_paths(
        &mut keys,
        row,
        "trace_id",
        &[&["trace_id"], &["metadata", "trace_id"]],
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

fn collect_atom_strings(value: Option<&Value>, out: &mut Vec<String>) {
    match value {
        Some(Value::String(text)) => out.push(text.clone()),
        Some(Value::Array(values)) => {
            for value in values {
                collect_atom_strings(Some(value), out);
            }
        }
        Some(Value::Object(map)) => {
            for value in map.values() {
                collect_atom_strings(Some(value), out);
            }
        }
        _ => {}
    }
}

fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read JSONL '{}': {error}", path.display()))
}

fn write_text_file(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    std::fs::write(path, text)
        .map_err(|error| format!("failed to write '{}': {error}", path.display()))
}

fn parse_json_row(path: &Path, line_number: usize, line: &str) -> Result<Option<Value>, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
        format!(
            "failed to parse JSONL '{}' line {}: {error}",
            path.display(),
            line_number
        )
    })?;
    if row.is_object() {
        Ok(Some(row))
    } else {
        Ok(None)
    }
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(str::to_owned)
}

fn json_string_vec(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect(),
        Some(Value::String(value)) if !value.is_empty() => vec![value.clone()],
        _ => Vec::new(),
    }
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
}

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_u64()
}

fn json_i64(value: &Value, path: &[&str]) -> Option<i64> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_i64()
}

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_bool()
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}
