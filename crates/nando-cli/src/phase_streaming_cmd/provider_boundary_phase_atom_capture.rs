use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

const DEFAULT_PROVIDER_BOUNDARY_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-phase-atom-capture-v1.report.json";
const DEFAULT_PROVIDER_BOUNDARY_TRACE: &str =
    "target/nando-wave/streaming/provider-boundary-phase-atom-trace-v1.jsonl";

#[derive(Default)]
struct ProviderBoundaryCaptureState {
    total_rows: usize,
    output_rows: usize,
    rows_with_provider_correlation_keys: usize,
    rows_missing_provider_correlation_keys: usize,
    provider_key_count: usize,
    rows_with_provider_key_atom_leak: usize,
    rows_ready_for_route_family_mining: usize,
    rows_ready_for_existing_shadow_scoring: usize,
    economic_capture_candidate_rows: usize,
    rows_with_token_or_cost: usize,
    rows_with_positive_tokens: usize,
    rows_with_shadow_request: usize,
    rows_with_verifier_label: usize,
    schema_counts: BTreeMap<String, usize>,
    traffic_source_counts: BTreeMap<String, usize>,
}

pub(crate) fn run_phase_stream_provider_boundary_phase_atom_trace_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_REPORT));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_TRACE));
    let input_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if input_paths.is_empty() {
        return Err("at least one provider-boundary event JSONL path is required".to_owned());
    }

    let mut state = ProviderBoundaryCaptureState::default();
    let mut build_state = super::GenericPhaseAtomTraceBuildState::default();
    let mut output = String::new();

    for input_path in &input_paths {
        let text = std::fs::read_to_string(input_path).map_err(|error| {
            format!(
                "failed to read provider-boundary input '{}': {error}",
                input_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse provider-boundary input '{}' line {}: {error}",
                    input_path.display(),
                    line_index + 1
                )
            })?;
            if !row.is_object() {
                continue;
            }
            state.total_rows += 1;
            let normalized = normalize_provider_boundary_event(input_path, line_index, &row);
            let atom_row =
                super::build_phase_atom_trace_row(input_path, &normalized, &mut build_state);
            observe_output_row(&atom_row, &mut state);
            output.push_str(
                &serde_json::to_string(&atom_row)
                    .map_err(|error| format!("failed to serialize phase-atom row: {error}"))?,
            );
            output.push('\n');
            state.output_rows += 1;
        }
    }

    write_text_file(&output_trace_path, &output)?;

    let provider_correlation_metadata_only = state.rows_with_provider_key_atom_leak == 0;
    let economic_capture_ready = state.economic_capture_candidate_rows > 0
        && provider_correlation_metadata_only
        && state.rows_with_positive_tokens > 0;
    let mut blockers = Vec::<String>::new();
    if state.output_rows == 0 {
        blockers.push("empty_output".to_owned());
    }
    if state.rows_ready_for_route_family_mining == 0 {
        blockers.push("no_route_family_mining_ready_rows".to_owned());
    }
    if state.rows_ready_for_existing_shadow_scoring == 0 {
        blockers.push("no_existing_shadow_scoring_ready_rows".to_owned());
    }
    if state.rows_missing_provider_correlation_keys > 0 {
        blockers.push("provider_correlation_missing_on_some_rows".to_owned());
    }
    if !provider_correlation_metadata_only {
        blockers.push("provider_key_atom_leak".to_owned());
    }
    if state.rows_with_positive_tokens == 0 {
        blockers.push("no_positive_token_denominator".to_owned());
    }
    blockers.sort();
    blockers.dedup();

    let verdict = if !provider_correlation_metadata_only {
        "PROVIDER_BOUNDARY_PHASE_ATOM_CAPTURE_V1_FAIL_PROVIDER_KEY_ATOM_LEAK"
    } else if economic_capture_ready {
        "PROVIDER_BOUNDARY_PHASE_ATOM_CAPTURE_V1_PASS_ECONOMIC_CAPTURE_READY"
    } else if state.rows_with_provider_correlation_keys > 0 {
        "PROVIDER_BOUNDARY_PHASE_ATOM_CAPTURE_V1_WATCH_PARTIAL_PROVIDER_CORRELATION"
    } else {
        "PROVIDER_BOUNDARY_PHASE_ATOM_CAPTURE_V1_WATCH_NO_PROVIDER_CORRELATION"
    };

    let report = serde_json::json!({
        "report_kind": "provider_boundary_phase_atom_capture_v1",
        "input_paths": input_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>(),
        "output_trace_path": output_trace_path.display().to_string(),
        "total_rows": state.total_rows,
        "output_rows": state.output_rows,
        "rows_with_provider_correlation_keys": state.rows_with_provider_correlation_keys,
        "rows_missing_provider_correlation_keys": state.rows_missing_provider_correlation_keys,
        "provider_key_count": state.provider_key_count,
        "rows_with_provider_key_atom_leak": state.rows_with_provider_key_atom_leak,
        "provider_correlation_metadata_only": provider_correlation_metadata_only,
        "rows_with_shadow_request": state.rows_with_shadow_request,
        "rows_with_verifier_label": state.rows_with_verifier_label,
        "rows_ready_for_route_family_mining": state.rows_ready_for_route_family_mining,
        "rows_ready_for_existing_shadow_scoring": state.rows_ready_for_existing_shadow_scoring,
        "economic_capture_candidate_rows": state.economic_capture_candidate_rows,
        "rows_with_token_or_cost": state.rows_with_token_or_cost,
        "rows_with_positive_tokens": state.rows_with_positive_tokens,
        "schema_counts": state.schema_counts,
        "traffic_source_counts": state.traffic_source_counts,
        "readiness": {
            "economic_capture_ready": economic_capture_ready,
            "policy": "provider-boundary adapter preserves provider correlation as metadata before phase atoms reach automatic discovery; provider ids must not become atoms"
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
        "boundary": "source adapter only: transforms provider-boundary event rows into phase-atom trace rows with metadata-only provider correlation; does not mine, compile, score, promote, serve, local-accept, or claim savings"
    });
    write_json_file(&report_path, &report)?;

    println!("provider_boundary_phase_atom_capture_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  output_trace_path: {}", output_trace_path.display());
    println!("  total_rows: {}", state.total_rows);
    println!(
        "  rows_with_provider_correlation_keys: {}",
        state.rows_with_provider_correlation_keys
    );
    println!(
        "  rows_ready_for_existing_shadow_scoring: {}",
        state.rows_ready_for_existing_shadow_scoring
    );
    println!(
        "  economic_capture_candidate_rows: {}",
        state.economic_capture_candidate_rows
    );
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn normalize_provider_boundary_event(input_path: &Path, line_index: usize, row: &Value) -> Value {
    let mut map = row.as_object().cloned().unwrap_or_default();
    let provider_keys = super::phase_atom_external_provider_correlation_keys(row);
    if !provider_keys.is_empty() {
        map.insert(
            "external_provider_correlation_keys".to_owned(),
            Value::Array(provider_keys.into_iter().map(Value::String).collect()),
        );
    }
    insert_default_string(&mut map, "traffic_source", "provider_boundary_live_event");
    insert_default_string(
        &mut map,
        "verification_source",
        "provider_boundary_verifier",
    );
    if !map.contains_key("request_fingerprint") {
        map.insert(
            "request_fingerprint".to_owned(),
            Value::String(format!(
                "provider_boundary_line:{}:{}",
                input_path.display(),
                line_index + 1
            )),
        );
    }
    if !map.contains_key("exact_cache_key")
        && let Some(fingerprint) = map
            .get("request_fingerprint")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    {
        map.insert("exact_cache_key".to_owned(), Value::String(fingerprint));
    }
    if !map.contains_key("nando_shadow_request")
        && let Some(shadow_request) = map
            .get("shadow_request")
            .or_else(|| map.get("nando_shadow"))
            .filter(|value| value.is_object())
            .cloned()
    {
        map.insert("nando_shadow_request".to_owned(), shadow_request);
    }
    map.insert(
        "provider_boundary_capture".to_owned(),
        serde_json::json!({
            "schema_version": "provider_boundary_phase_atom_capture_v1",
            "source_path": input_path.display().to_string(),
            "source_line": line_index + 1,
            "provider_correlation_policy": "metadata_only_not_atoms"
        }),
    );
    Value::Object(map)
}

fn observe_output_row(row: &Value, state: &mut ProviderBoundaryCaptureState) {
    let provider_keys = super::phase_atom_external_provider_correlation_keys(row);
    let has_provider_keys = !provider_keys.is_empty();
    state.provider_key_count = state.provider_key_count.saturating_add(provider_keys.len());
    state.rows_with_provider_correlation_keys += usize::from(has_provider_keys);
    state.rows_missing_provider_correlation_keys += usize::from(!has_provider_keys);
    if provider_key_leaks_into_atoms(row) {
        state.rows_with_provider_key_atom_leak += 1;
    }

    let schema = json_string(row, &["schema_version"]).unwrap_or_else(|| "unknown".to_owned());
    *state.schema_counts.entry(schema).or_default() += 1;
    let traffic_source =
        json_string(row, &["traffic_source"]).unwrap_or_else(|| "unknown".to_owned());
    *state
        .traffic_source_counts
        .entry(traffic_source)
        .or_default() += 1;

    let has_shadow_request = json_bool(row, &["has_shadow_request"]).unwrap_or(false)
        || row
            .get("nando_shadow_request")
            .is_some_and(Value::is_object);
    state.rows_with_shadow_request += usize::from(has_shadow_request);
    let has_verifier_label = row
        .get("verified_safe_accept")
        .is_some_and(|value| !value.is_null());
    state.rows_with_verifier_label += usize::from(has_verifier_label);

    let ready_for_route_family_mining =
        json_bool(row, &["ready_for_route_family_mining"]).unwrap_or(false);
    let ready_for_existing_shadow_scoring =
        json_bool(row, &["ready_for_existing_shadow_scoring"]).unwrap_or(false);
    state.rows_ready_for_route_family_mining += usize::from(ready_for_route_family_mining);
    state.rows_ready_for_existing_shadow_scoring += usize::from(ready_for_existing_shadow_scoring);

    let total_tokens = json_usize(row, &["token_cost", "total_tokens"])
        .or_else(|| json_usize(row, &["estimated_total_tokens"]))
        .unwrap_or(0);
    let total_cost = json_u64(row, &["token_cost", "total_cost_microusd"])
        .or_else(|| json_u64(row, &["estimated_total_cost_microusd"]))
        .unwrap_or(0);
    state.rows_with_positive_tokens += usize::from(total_tokens > 0);
    state.rows_with_token_or_cost += usize::from(total_tokens > 0 || total_cost > 0);
    if ready_for_existing_shadow_scoring && has_provider_keys && total_tokens > 0 {
        state.economic_capture_candidate_rows += 1;
    }
}

fn insert_default_string(map: &mut Map<String, Value>, key: &str, value: &str) {
    if !map.contains_key(key) {
        map.insert(key.to_owned(), Value::String(value.to_owned()));
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

fn collect_atom_strings(value: Option<&Value>, output: &mut Vec<String>) {
    match value {
        Some(Value::String(text)) => output.push(text.clone()),
        Some(Value::Array(items)) => {
            for item in items {
                collect_atom_strings(Some(item), output);
            }
        }
        Some(Value::Object(map)) => {
            for value in map.values() {
                collect_atom_strings(Some(value), output);
            }
        }
        _ => {}
    }
}

fn write_text_file(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create output directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(path, text)
        .map_err(|error| format!("failed to write output '{}': {error}", path.display()))
}

fn write_json_file(path: &Path, value: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize report '{}': {error}", path.display()))?;
    write_text_file(path, &format!("{text}\n"))
}

fn json_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path).and_then(Value::as_bool)
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    json_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_at(value, path)
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
        })
        .and_then(|value| usize::try_from(value).ok())
}

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    json_at(value, path).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
    })
}
