use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_PROVIDER_BOUNDARY_EXPORT_INGEST_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-export-ingest-v1.report.json";
const DEFAULT_PROVIDER_BOUNDARY_EXPORT_INGEST_JSONL: &str =
    "target/nando-wave/streaming/provider-boundary-export-ingest-v1.provider.jsonl";

#[derive(Default)]
struct CaptureRequestIndex {
    rows: usize,
    key_to_capture_ids: BTreeMap<String, BTreeSet<String>>,
    capture_id_to_join_keys: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Default)]
struct IngestState {
    provider_export_rows: usize,
    rows_with_match_keys: usize,
    rows_with_provider_keys: usize,
    rows_matched_to_capture_request: usize,
    rows_skipped_missing_match_keys: usize,
    rows_skipped_missing_provider_keys: usize,
    rows_skipped_unmatched_capture_request: usize,
    rows_skipped_ambiguous_capture_request: usize,
    normalized_provider_boundary_rows: usize,
    normalized_unique_capture_requests: BTreeSet<String>,
    normalized_total_tokens: usize,
    normalized_total_cost_microusd: u64,
    provider_key_atom_leak_rows: usize,
}

pub(crate) fn run_phase_stream_provider_boundary_export_ingest_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_EXPORT_INGEST_REPORT));
    let output_provider_boundary_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_EXPORT_INGEST_JSONL));
    let capture_request_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "capture-request JSONL path is required".to_owned())?;
    let provider_export_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if provider_export_paths.is_empty() {
        return Err("at least one provider export JSONL path is required".to_owned());
    }

    let capture_index = read_capture_request_index(&capture_request_path)?;
    let output_file = create_file(&output_provider_boundary_path)?;
    let mut writer = BufWriter::new(output_file);
    let mut state = IngestState::default();

    for provider_export_path in &provider_export_paths {
        let text = read_text(provider_export_path)?;
        for (line_index, line) in text.lines().enumerate() {
            let Some(row) = parse_jsonl_row(provider_export_path, line_index + 1, line)? else {
                continue;
            };
            state.provider_export_rows += 1;
            if provider_key_leaks_into_atoms(&row) {
                state.provider_key_atom_leak_rows += 1;
            }
            let match_keys = match_keys(&row);
            let provider_keys = super::phase_atom_external_provider_correlation_keys(&row);
            state.rows_with_match_keys += usize::from(!match_keys.is_empty());
            state.rows_with_provider_keys += usize::from(!provider_keys.is_empty());
            if match_keys.is_empty() {
                state.rows_skipped_missing_match_keys += 1;
                continue;
            }
            if provider_keys.is_empty() {
                state.rows_skipped_missing_provider_keys += 1;
                continue;
            }

            let matched_capture_ids = matched_capture_ids(&capture_index, &match_keys);
            if matched_capture_ids.is_empty() {
                state.rows_skipped_unmatched_capture_request += 1;
                continue;
            }
            if matched_capture_ids.len() > 1 {
                state.rows_skipped_ambiguous_capture_request += 1;
                continue;
            }
            let capture_id = matched_capture_ids
                .iter()
                .next()
                .expect("one capture id")
                .clone();
            state.rows_matched_to_capture_request += 1;
            state
                .normalized_unique_capture_requests
                .insert(capture_id.clone());

            let matched_join_keys = capture_index
                .capture_id_to_join_keys
                .get(&capture_id)
                .cloned()
                .unwrap_or_default();
            let total_tokens = provider_total_tokens(&row).unwrap_or(0);
            let total_cost_microusd = provider_cost_microusd(&row).unwrap_or(0);
            state.normalized_total_tokens =
                state.normalized_total_tokens.saturating_add(total_tokens);
            state.normalized_total_cost_microusd = state
                .normalized_total_cost_microusd
                .saturating_add(total_cost_microusd);

            let output_row = provider_boundary_row(
                &capture_id,
                &matched_join_keys,
                &provider_keys,
                provider_export_path,
                line_index + 1,
                total_tokens,
                total_cost_microusd,
            );
            serde_json::to_writer(&mut writer, &output_row).map_err(|error| {
                format!(
                    "failed to serialize provider-boundary row '{}': {error}",
                    output_provider_boundary_path.display()
                )
            })?;
            writer.write_all(b"\n").map_err(|error| {
                format!(
                    "failed to write provider-boundary row '{}': {error}",
                    output_provider_boundary_path.display()
                )
            })?;
            state.normalized_provider_boundary_rows += 1;
        }
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush provider-boundary output '{}': {error}",
            output_provider_boundary_path.display()
        )
    })?;

    let provider_correlation_metadata_only = state.provider_key_atom_leak_rows == 0;
    let capture_coverage_possible = state.normalized_provider_boundary_rows > 0
        && !state.normalized_unique_capture_requests.is_empty()
        && provider_correlation_metadata_only;
    let mut blockers = Vec::<&'static str>::new();
    if capture_index.rows == 0 {
        blockers.push("no_capture_request_rows");
    }
    if state.provider_export_rows == 0 {
        blockers.push("no_provider_export_rows");
    }
    if state.rows_with_match_keys == 0 {
        blockers.push("no_provider_export_match_keys");
    }
    if state.rows_with_provider_keys == 0 {
        blockers.push("no_provider_export_provider_keys");
    }
    if state.normalized_provider_boundary_rows == 0 {
        blockers.push("no_normalized_provider_boundary_rows");
    }
    if !provider_correlation_metadata_only {
        blockers.push("provider_key_atom_leak");
    }

    let verdict = if !provider_correlation_metadata_only {
        "PHASE_STREAM_PROVIDER_BOUNDARY_EXPORT_INGEST_V1_FAIL_PROVIDER_KEY_ATOM_LEAK"
    } else if capture_coverage_possible {
        "PHASE_STREAM_PROVIDER_BOUNDARY_EXPORT_INGEST_V1_READY_FOR_CAPTURE_COVERAGE_GATE"
    } else {
        "PHASE_STREAM_PROVIDER_BOUNDARY_EXPORT_INGEST_V1_WATCH_NO_COVERAGE_ROWS"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_provider_boundary_export_ingest_v1",
        "capture_request_path": path_string(&capture_request_path),
        "provider_export_paths": provider_export_paths.iter().map(|path| path_string(path)).collect::<Vec<_>>(),
        "output_provider_boundary_path": path_string(&output_provider_boundary_path),
        "capture_request_rows": capture_index.rows,
        "capture_request_join_key_count": capture_index.key_to_capture_ids.len(),
        "provider_export_rows": state.provider_export_rows,
        "rows_with_match_keys": state.rows_with_match_keys,
        "rows_with_provider_keys": state.rows_with_provider_keys,
        "rows_matched_to_capture_request": state.rows_matched_to_capture_request,
        "rows_skipped_missing_match_keys": state.rows_skipped_missing_match_keys,
        "rows_skipped_missing_provider_keys": state.rows_skipped_missing_provider_keys,
        "rows_skipped_unmatched_capture_request": state.rows_skipped_unmatched_capture_request,
        "rows_skipped_ambiguous_capture_request": state.rows_skipped_ambiguous_capture_request,
        "normalized_provider_boundary_rows": state.normalized_provider_boundary_rows,
        "normalized_unique_capture_requests": state.normalized_unique_capture_requests.len(),
        "normalized_total_tokens": state.normalized_total_tokens,
        "normalized_total_cost_microusd": state.normalized_total_cost_microusd,
        "provider_key_atom_leak_rows": state.provider_key_atom_leak_rows,
        "readiness": {
            "provider_correlation_metadata_only": provider_correlation_metadata_only,
            "capture_coverage_possible": capture_coverage_possible,
            "market_money_claim_allowed": false,
            "local_accept_enabled": false,
            "policy": "normalizes external provider export rows into provider-boundary metadata rows only when the export carries both capture-request join keys and provider correlation keys"
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
        "boundary": "provider-export to provider-boundary adapter only: does not mine, score, compile .nwpc, serve, promote, local-accept, estimate missing billing, or claim money"
    });
    super::write_json_file(&report_path, &report)?;

    println!("phase_stream_provider_boundary_export_ingest_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  output_provider_boundary_path: {}",
        output_provider_boundary_path.display()
    );
    println!("  provider_export_rows: {}", state.provider_export_rows);
    println!(
        "  normalized_provider_boundary_rows: {}",
        state.normalized_provider_boundary_rows
    );
    println!(
        "  normalized_unique_capture_requests: {}",
        state.normalized_unique_capture_requests.len()
    );
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn read_capture_request_index(path: &Path) -> Result<CaptureRequestIndex, String> {
    let text = read_text(path)?;
    let mut index = CaptureRequestIndex::default();
    for (line_index, line) in text.lines().enumerate() {
        let Some(row) = parse_jsonl_row(path, line_index + 1, line)? else {
            continue;
        };
        index.rows += 1;
        let capture_id = json_string(&row, &["capture_request_id"])
            .unwrap_or_else(|| format!("capture_request_line_{}", line_index + 1));
        let join_keys = json_string_vec(json_at(&row, &["join_keys"]))
            .into_iter()
            .collect::<BTreeSet<_>>();
        index
            .capture_id_to_join_keys
            .insert(capture_id.clone(), join_keys.clone());
        for join_key in join_keys {
            index
                .key_to_capture_ids
                .entry(join_key)
                .or_default()
                .insert(capture_id.clone());
        }
    }
    Ok(index)
}

fn matched_capture_ids(index: &CaptureRequestIndex, match_keys: &[String]) -> BTreeSet<String> {
    let mut matched = BTreeSet::new();
    for key in match_keys {
        if let Some(capture_ids) = index.key_to_capture_ids.get(key) {
            matched.extend(capture_ids.iter().cloned());
        }
    }
    matched
}

fn provider_boundary_row(
    capture_id: &str,
    matched_join_keys: &BTreeSet<String>,
    provider_keys: &[String],
    provider_export_path: &Path,
    provider_export_line: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
) -> Value {
    let mut row = serde_json::json!({
        "schema_version": "provider_boundary_export_ingest_v1",
        "capture_request_id": capture_id,
        "match_keys": matched_join_keys.iter().cloned().collect::<Vec<_>>(),
        "external_provider_correlation_keys": provider_keys,
        "provider_correlation_ready": true,
        "provider_export_path": path_string(provider_export_path),
        "provider_export_line": provider_export_line,
        "token_cost": {
            "total_tokens": total_tokens,
            "total_cost_microusd": total_cost_microusd,
            "token_evidence_missing": total_tokens == 0,
            "cost_evidence_missing": total_cost_microusd == 0
        },
        "local_accept_enabled": false,
        "market_money_claim_allowed": false,
        "boundary": "provider-boundary metadata row normalized from external provider export; provider ids are metadata only, not atoms"
    });
    if let Some(map) = row.as_object_mut() {
        for key in matched_join_keys {
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
            &["custom_id"],
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

fn create_file(path: &Path) -> Result<std::fs::File, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    std::fs::File::create(path)
        .map_err(|error| format!("failed to create '{}': {error}", path.display()))
}

fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read JSONL '{}': {error}", path.display()))
}

fn parse_jsonl_row(path: &Path, line_number: usize, line: &str) -> Result<Option<Value>, String> {
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
