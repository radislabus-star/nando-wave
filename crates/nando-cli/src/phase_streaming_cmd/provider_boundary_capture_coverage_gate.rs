use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_PROVIDER_BOUNDARY_CAPTURE_COVERAGE_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-capture-coverage-gate-v1.report.json";

#[derive(Default)]
struct ProviderIndex {
    rows: usize,
    rows_with_join_keys: usize,
    rows_with_provider_keys: usize,
    join_key_to_provider_keys: BTreeMap<String, BTreeSet<String>>,
    provider_key_atom_leak_rows: usize,
}

#[derive(Default)]
struct CoverageState {
    capture_request_rows: usize,
    capture_requests_with_join_keys: usize,
    capture_requests_missing_join_keys: usize,
    covered_capture_requests: usize,
    missing_capture_requests: usize,
    covered_phase_rows: usize,
    missing_phase_rows: usize,
    covered_tokens: usize,
    missing_tokens: usize,
    covered_cost_microusd: u64,
    missing_cost_microusd: u64,
    join_key_collisions: usize,
    missing_samples: Vec<Value>,
}

pub(crate) fn run_phase_stream_provider_boundary_capture_coverage_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_CAPTURE_COVERAGE_REPORT));
    let capture_request_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "capture-request JSONL path is required".to_owned())?;

    let mut provider_paths = Vec::<PathBuf>::new();
    let mut seen_provider_separator = false;
    for arg in args {
        if arg == "--provider" {
            seen_provider_separator = true;
            continue;
        }
        if seen_provider_separator {
            provider_paths.push(PathBuf::from(arg));
        } else {
            return Err(format!(
                "unexpected argument before --provider: {arg}; expected provider-boundary paths after --provider"
            ));
        }
    }
    if provider_paths.is_empty() {
        return Err(
            "at least one provider-boundary JSONL path is required after --provider".to_owned(),
        );
    }

    let provider_index = build_provider_index(&provider_paths)?;
    let state = scan_capture_requests(&capture_request_path, &provider_index)?;

    let provider_correlation_metadata_only = provider_index.provider_key_atom_leak_rows == 0;
    let full_capture_coverage = state.capture_request_rows > 0
        && state.missing_capture_requests == 0
        && state.covered_capture_requests == state.capture_request_rows;
    let provider_capture_complete = provider_correlation_metadata_only && full_capture_coverage;
    let mut blockers = Vec::<&'static str>::new();
    if state.capture_request_rows == 0 {
        blockers.push("no_capture_request_rows");
    }
    if provider_index.rows == 0 {
        blockers.push("no_provider_boundary_rows");
    }
    if provider_index.rows_with_provider_keys == 0 {
        blockers.push("no_provider_rows_with_provider_keys");
    }
    if state.capture_requests_missing_join_keys > 0 {
        blockers.push("some_capture_requests_missing_join_keys");
    }
    if state.missing_capture_requests > 0 {
        blockers.push("some_capture_requests_not_covered_by_provider_rows");
    }
    if !provider_correlation_metadata_only {
        blockers.push("provider_key_atom_leak");
    }

    let verdict = if !provider_correlation_metadata_only {
        "PHASE_STREAM_PROVIDER_BOUNDARY_CAPTURE_COVERAGE_GATE_V1_FAIL_PROVIDER_KEY_ATOM_LEAK"
    } else if provider_capture_complete {
        "PHASE_STREAM_PROVIDER_BOUNDARY_CAPTURE_COVERAGE_GATE_V1_PASS_FULL_CAPTURE_COVERAGE"
    } else {
        "PHASE_STREAM_PROVIDER_BOUNDARY_CAPTURE_COVERAGE_GATE_V1_WATCH_CAPTURE_COVERAGE_INCOMPLETE"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_provider_boundary_capture_coverage_gate_v1",
        "capture_request_path": path_string(&capture_request_path),
        "provider_boundary_paths": provider_paths.iter().map(|path| path_string(path)).collect::<Vec<_>>(),
        "capture_requests": {
            "rows": state.capture_request_rows,
            "rows_with_join_keys": state.capture_requests_with_join_keys,
            "rows_missing_join_keys": state.capture_requests_missing_join_keys,
            "covered_capture_requests": state.covered_capture_requests,
            "missing_capture_requests": state.missing_capture_requests,
            "covered_phase_rows": state.covered_phase_rows,
            "missing_phase_rows": state.missing_phase_rows,
            "covered_tokens": state.covered_tokens,
            "missing_tokens": state.missing_tokens,
            "covered_cost_microusd": state.covered_cost_microusd,
            "missing_cost_microusd": state.missing_cost_microusd,
            "join_key_collisions": state.join_key_collisions,
            "missing_samples": state.missing_samples
        },
        "provider": {
            "rows": provider_index.rows,
            "rows_with_join_keys": provider_index.rows_with_join_keys,
            "rows_with_provider_keys": provider_index.rows_with_provider_keys,
            "join_key_count": provider_index.join_key_to_provider_keys.len(),
            "provider_key_atom_leak_rows": provider_index.provider_key_atom_leak_rows
        },
        "readiness": {
            "provider_correlation_metadata_only": provider_correlation_metadata_only,
            "full_capture_coverage": full_capture_coverage,
            "provider_capture_complete": provider_capture_complete,
            "market_money_claim_allowed": false,
            "local_accept_enabled": false,
            "policy": "coverage gate only: proves whether provider-boundary rows cover capture-request join keys before match-readiness/NP evidence chain; no mining or scoring"
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
        "boundary": "provider-boundary capture coverage proof only: no trace mutation, no phase-center mining, no .nwpc compile, no serving, no promotion, no local accept, no money claim"
    });
    super::write_json_file(&report_path, &report)?;

    println!("phase_stream_provider_boundary_capture_coverage_gate_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  capture_request_rows: {}", state.capture_request_rows);
    println!(
        "  covered_capture_requests: {}",
        state.covered_capture_requests
    );
    println!(
        "  missing_capture_requests: {}",
        state.missing_capture_requests
    );
    println!("  missing_tokens: {}", state.missing_tokens);
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn build_provider_index(paths: &[PathBuf]) -> Result<ProviderIndex, String> {
    let mut index = ProviderIndex::default();
    for path in paths {
        let text = read_text(path)?;
        for (line_index, line) in text.lines().enumerate() {
            let Some(row) = parse_jsonl_row(path, line_index + 1, line)? else {
                continue;
            };
            index.rows += 1;
            let join_keys = metadata_join_keys(&row);
            let provider_keys = super::phase_atom_external_provider_correlation_keys(&row)
                .into_iter()
                .collect::<BTreeSet<_>>();
            index.rows_with_join_keys += usize::from(!join_keys.is_empty());
            index.rows_with_provider_keys += usize::from(!provider_keys.is_empty());
            if provider_key_leaks_into_atoms(&row) {
                index.provider_key_atom_leak_rows += 1;
            }
            if join_keys.is_empty() || provider_keys.is_empty() {
                continue;
            }
            for join_key in join_keys {
                index
                    .join_key_to_provider_keys
                    .entry(join_key)
                    .or_default()
                    .extend(provider_keys.iter().cloned());
            }
        }
    }
    Ok(index)
}

fn scan_capture_requests(
    path: &Path,
    provider_index: &ProviderIndex,
) -> Result<CoverageState, String> {
    let text = read_text(path)?;
    let mut state = CoverageState::default();
    for (line_index, line) in text.lines().enumerate() {
        let Some(row) = parse_jsonl_row(path, line_index + 1, line)? else {
            continue;
        };
        state.capture_request_rows += 1;
        let join_keys = json_string_vec(json_at(&row, &["join_keys"]));
        state.capture_requests_with_join_keys += usize::from(!join_keys.is_empty());
        state.capture_requests_missing_join_keys += usize::from(join_keys.is_empty());
        let phase_row_count = json_usize(&row, &["phase_row_count"]).unwrap_or(1);
        let total_tokens = json_usize(&row, &["total_tokens"]).unwrap_or(0);
        let total_cost = json_u64(&row, &["total_cost_microusd"]).unwrap_or(0);

        let mut provider_keys = BTreeSet::<String>::new();
        for join_key in &join_keys {
            if let Some(keys) = provider_index.join_key_to_provider_keys.get(join_key) {
                if !provider_keys.is_empty() && !keys.is_subset(&provider_keys) {
                    state.join_key_collisions += 1;
                }
                provider_keys.extend(keys.iter().cloned());
            }
        }
        if !provider_keys.is_empty() {
            state.covered_capture_requests += 1;
            state.covered_phase_rows = state.covered_phase_rows.saturating_add(phase_row_count);
            state.covered_tokens = state.covered_tokens.saturating_add(total_tokens);
            state.covered_cost_microusd = state.covered_cost_microusd.saturating_add(total_cost);
        } else {
            state.missing_capture_requests += 1;
            state.missing_phase_rows = state.missing_phase_rows.saturating_add(phase_row_count);
            state.missing_tokens = state.missing_tokens.saturating_add(total_tokens);
            state.missing_cost_microusd = state.missing_cost_microusd.saturating_add(total_cost);
            if state.missing_samples.len() < 8 {
                state.missing_samples.push(serde_json::json!({
                    "line_number": line_index + 1,
                    "capture_request_id": json_string(&row, &["capture_request_id"]),
                    "primary_join_key": json_string(&row, &["primary_join_key"]),
                    "phase_row_count": phase_row_count,
                    "total_tokens": total_tokens,
                    "action_families": row.get("action_families").cloned().unwrap_or(Value::Null),
                    "route_keys": row.get("route_keys").cloned().unwrap_or(Value::Null),
                    "sample_sources": row.get("sample_sources").cloned().unwrap_or(Value::Null)
                }));
            }
        }
    }
    Ok(state)
}

fn metadata_join_keys(row: &Value) -> Vec<String> {
    let paths: &[(&str, &[&str])] = &[
        ("request_fingerprint", &["request_fingerprint"]),
        ("request_fingerprint", &["metadata", "request_fingerprint"]),
        ("exact_cache_key", &["exact_cache_key"]),
        ("exact_cache_key", &["metadata", "exact_cache_key"]),
        ("trace_id", &["trace_id"]),
        ("trace_id", &["metadata", "trace_id"]),
        ("event_id", &["event_id"]),
        ("event_id", &["metadata", "event_id"]),
        ("client_correlation_id", &["client_correlation_id"]),
        (
            "client_correlation_id",
            &["metadata", "client_correlation_id"],
        ),
    ];
    let mut keys = paths
        .iter()
        .filter_map(|(label, path)| json_string(row, path).map(|value| format!("{label}:{value}")))
        .filter(|value| !value.ends_with(':'))
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    keys
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
        Some(Value::String(text)) if !text.is_empty() => vec![text.clone()],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_at(value, path)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    json_at(value, path).and_then(Value::as_u64)
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}
