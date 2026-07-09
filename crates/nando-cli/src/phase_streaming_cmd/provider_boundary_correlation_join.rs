use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_PROVIDER_BOUNDARY_JOIN_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-correlation-join-v1.report.json";
const DEFAULT_PROVIDER_BOUNDARY_JOIN_OUTPUT: &str =
    "target/nando-wave/streaming/provider-boundary-correlation-joined-phase-atom-trace-v1.jsonl";

#[derive(Default)]
struct ProviderBoundaryIndex {
    provider_rows: usize,
    provider_rows_with_join_keys: usize,
    provider_rows_missing_join_keys: usize,
    provider_rows_with_provider_keys: usize,
    provider_rows_missing_provider_keys: usize,
    join_key_to_provider_keys: BTreeMap<String, BTreeSet<String>>,
    join_key_collisions: usize,
}

#[derive(Default)]
struct ProviderBoundaryJoinState {
    trace_rows: usize,
    output_rows: usize,
    trace_rows_with_join_keys: usize,
    trace_rows_missing_join_keys: usize,
    trace_rows_with_existing_provider_keys: usize,
    trace_rows_joined_with_provider_keys: usize,
    trace_rows_missing_provider_join: usize,
    output_rows_with_provider_correlation_keys: usize,
    rows_with_provider_key_atom_leak: usize,
    rows_ready_for_route_family_mining: usize,
    rows_ready_for_existing_shadow_scoring: usize,
    score_ready_rows_with_provider_correlation: usize,
    score_ready_rows_missing_provider_correlation: usize,
    economic_score_ready_rows: usize,
    economic_score_ready_rows_with_provider_correlation: usize,
    economic_score_ready_rows_missing_provider_correlation: usize,
    zero_denominator_score_ready_rows: usize,
    zero_denominator_score_ready_rows_missing_provider_correlation: usize,
    rows_with_positive_tokens: usize,
}

pub(crate) fn run_phase_stream_provider_boundary_correlation_join_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_JOIN_REPORT));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_JOIN_OUTPUT));
    let phase_atom_trace_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "phase-atom trace JSONL path is required".to_owned())?;
    let provider_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if provider_paths.is_empty() {
        return Err("at least one provider-boundary JSONL path is required".to_owned());
    }

    let provider_index = build_provider_index(&provider_paths)?;
    let mut state = ProviderBoundaryJoinState::default();
    let mut output = String::new();
    let trace_text = std::fs::read_to_string(&phase_atom_trace_path).map_err(|error| {
        format!(
            "failed to read phase-atom trace '{}': {error}",
            phase_atom_trace_path.display()
        )
    })?;
    for (line_index, line) in trace_text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse phase-atom trace '{}' line {}: {error}",
                phase_atom_trace_path.display(),
                line_index + 1
            )
        })?;
        if !row.is_object() {
            continue;
        }
        let joined = join_trace_row(&row, &provider_index, &mut state);
        output.push_str(
            &serde_json::to_string(&joined)
                .map_err(|error| format!("failed to serialize joined row: {error}"))?,
        );
        output.push('\n');
        state.output_rows += 1;
    }
    write_text_file(&output_trace_path, &output)?;

    let provider_correlation_metadata_only = state.rows_with_provider_key_atom_leak == 0;
    let provider_correlation_complete_for_score_ready = state.economic_score_ready_rows > 0
        && state.economic_score_ready_rows_missing_provider_correlation == 0
        && state.economic_score_ready_rows_with_provider_correlation
            == state.economic_score_ready_rows;
    let full_score_ready_provider_correlation = state.rows_ready_for_existing_shadow_scoring > 0
        && state.score_ready_rows_missing_provider_correlation == 0
        && state.score_ready_rows_with_provider_correlation
            == state.rows_ready_for_existing_shadow_scoring;
    let zero_denominator_explains_missing_score_ready =
        state.score_ready_rows_missing_provider_correlation > 0
            && state.score_ready_rows_missing_provider_correlation
                == state.zero_denominator_score_ready_rows_missing_provider_correlation;
    let economic_capture_ready = provider_correlation_metadata_only
        && provider_correlation_complete_for_score_ready
        && state.rows_with_positive_tokens > 0;
    let mut blockers = Vec::<String>::new();
    if state.output_rows == 0 {
        blockers.push("empty_trace_output".to_owned());
    }
    if provider_index.provider_rows_with_provider_keys == 0 {
        blockers.push("no_provider_boundary_rows_with_provider_keys".to_owned());
    }
    if state.trace_rows_missing_provider_join > 0 && !economic_capture_ready {
        blockers.push("some_trace_rows_missing_provider_join".to_owned());
    }
    if state.economic_score_ready_rows_missing_provider_correlation > 0 {
        blockers.push("score_ready_rows_missing_provider_correlation".to_owned());
    }
    if state.rows_with_provider_key_atom_leak > 0 {
        blockers.push("provider_key_atom_leak".to_owned());
    }
    if state.rows_with_positive_tokens == 0 {
        blockers.push("no_positive_token_denominator".to_owned());
    }
    blockers.sort();
    blockers.dedup();

    let verdict = if !provider_correlation_metadata_only {
        "PROVIDER_BOUNDARY_CORRELATION_JOIN_V1_FAIL_PROVIDER_KEY_ATOM_LEAK"
    } else if economic_capture_ready && full_score_ready_provider_correlation {
        "PROVIDER_BOUNDARY_CORRELATION_JOIN_V1_PASS_ECONOMIC_CAPTURE_READY"
    } else if economic_capture_ready && zero_denominator_explains_missing_score_ready {
        "PROVIDER_BOUNDARY_CORRELATION_JOIN_V1_PASS_ECONOMIC_CAPTURE_READY_WITH_ZERO_DENOMINATOR_EXCLUSIONS"
    } else if state.trace_rows_joined_with_provider_keys > 0 {
        "PROVIDER_BOUNDARY_CORRELATION_JOIN_V1_WATCH_PARTIAL_PROVIDER_JOIN"
    } else {
        "PROVIDER_BOUNDARY_CORRELATION_JOIN_V1_WATCH_NO_PROVIDER_JOIN"
    };

    let provider_boundary_paths = provider_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let readiness = serde_json::json!({
        "provider_correlation_complete_for_score_ready": provider_correlation_complete_for_score_ready,
        "full_score_ready_provider_correlation": full_score_ready_provider_correlation,
        "zero_denominator_explains_missing_score_ready": zero_denominator_explains_missing_score_ready,
        "economic_capture_ready": economic_capture_ready,
        "policy": "join only enriches provider correlation metadata using request/exact/trace keys; phase atoms are not changed and provider ids must not become atoms"
    });
    let forbidden_flags = serde_json::json!({
        "nwrb_used": false,
        "role_binding_backend_used": false,
        "lookup_used": false,
        "target_id_or_proof_rule_id_authority_used": false,
        "concrete_x_lookup_used": false,
        "manual_local_out_t_used": false,
        "local_accept_without_verifier_used": false
    });
    let mut report = serde_json::Map::new();
    report.insert(
        "report_kind".to_owned(),
        Value::String("provider_boundary_correlation_join_v1".to_owned()),
    );
    report.insert(
        "phase_atom_trace_path".to_owned(),
        Value::String(phase_atom_trace_path.display().to_string()),
    );
    report.insert(
        "provider_boundary_paths".to_owned(),
        serde_json::to_value(provider_boundary_paths)
            .map_err(|error| format!("failed to encode provider paths: {error}"))?,
    );
    report.insert(
        "output_trace_path".to_owned(),
        Value::String(output_trace_path.display().to_string()),
    );
    report.insert(
        "provider_rows".to_owned(),
        value_usize(provider_index.provider_rows),
    );
    report.insert(
        "provider_rows_with_join_keys".to_owned(),
        value_usize(provider_index.provider_rows_with_join_keys),
    );
    report.insert(
        "provider_rows_missing_join_keys".to_owned(),
        value_usize(provider_index.provider_rows_missing_join_keys),
    );
    report.insert(
        "provider_rows_with_provider_keys".to_owned(),
        value_usize(provider_index.provider_rows_with_provider_keys),
    );
    report.insert(
        "provider_rows_missing_provider_keys".to_owned(),
        value_usize(provider_index.provider_rows_missing_provider_keys),
    );
    report.insert(
        "provider_join_key_count".to_owned(),
        value_usize(provider_index.join_key_to_provider_keys.len()),
    );
    report.insert(
        "provider_join_key_collisions".to_owned(),
        value_usize(provider_index.join_key_collisions),
    );
    report.insert("trace_rows".to_owned(), value_usize(state.trace_rows));
    report.insert("output_rows".to_owned(), value_usize(state.output_rows));
    report.insert(
        "trace_rows_with_join_keys".to_owned(),
        value_usize(state.trace_rows_with_join_keys),
    );
    report.insert(
        "trace_rows_missing_join_keys".to_owned(),
        value_usize(state.trace_rows_missing_join_keys),
    );
    report.insert(
        "trace_rows_with_existing_provider_keys".to_owned(),
        value_usize(state.trace_rows_with_existing_provider_keys),
    );
    report.insert(
        "trace_rows_joined_with_provider_keys".to_owned(),
        value_usize(state.trace_rows_joined_with_provider_keys),
    );
    report.insert(
        "trace_rows_missing_provider_join".to_owned(),
        value_usize(state.trace_rows_missing_provider_join),
    );
    report.insert(
        "output_rows_with_provider_correlation_keys".to_owned(),
        value_usize(state.output_rows_with_provider_correlation_keys),
    );
    report.insert(
        "rows_with_provider_key_atom_leak".to_owned(),
        value_usize(state.rows_with_provider_key_atom_leak),
    );
    report.insert(
        "provider_correlation_metadata_only".to_owned(),
        Value::Bool(provider_correlation_metadata_only),
    );
    report.insert(
        "rows_ready_for_route_family_mining".to_owned(),
        value_usize(state.rows_ready_for_route_family_mining),
    );
    report.insert(
        "rows_ready_for_existing_shadow_scoring".to_owned(),
        value_usize(state.rows_ready_for_existing_shadow_scoring),
    );
    report.insert(
        "score_ready_rows_with_provider_correlation".to_owned(),
        value_usize(state.score_ready_rows_with_provider_correlation),
    );
    report.insert(
        "score_ready_rows_missing_provider_correlation".to_owned(),
        value_usize(state.score_ready_rows_missing_provider_correlation),
    );
    report.insert(
        "economic_score_ready_rows".to_owned(),
        value_usize(state.economic_score_ready_rows),
    );
    report.insert(
        "economic_score_ready_rows_with_provider_correlation".to_owned(),
        value_usize(state.economic_score_ready_rows_with_provider_correlation),
    );
    report.insert(
        "economic_score_ready_rows_missing_provider_correlation".to_owned(),
        value_usize(state.economic_score_ready_rows_missing_provider_correlation),
    );
    report.insert(
        "zero_denominator_score_ready_rows".to_owned(),
        value_usize(state.zero_denominator_score_ready_rows),
    );
    report.insert(
        "zero_denominator_score_ready_rows_missing_provider_correlation".to_owned(),
        value_usize(state.zero_denominator_score_ready_rows_missing_provider_correlation),
    );
    report.insert(
        "rows_with_positive_tokens".to_owned(),
        value_usize(state.rows_with_positive_tokens),
    );
    report.insert("readiness".to_owned(), readiness);
    report.insert(
        "blockers".to_owned(),
        serde_json::to_value(blockers)
            .map_err(|error| format!("failed to encode blockers: {error}"))?,
    );
    report.insert("manual_class_list_used".to_owned(), Value::Bool(false));
    report.insert("selector_used".to_owned(), Value::Bool(false));
    report.insert("dynamic_discovery_performed".to_owned(), Value::Bool(false));
    report.insert("forbidden_flags".to_owned(), forbidden_flags);
    report.insert("local_accept_enabled".to_owned(), Value::Bool(false));
    report.insert("auto_promote_enabled".to_owned(), Value::Bool(false));
    report.insert("product_promotion_allowed".to_owned(), Value::Bool(false));
    report.insert("market_money_claim_allowed".to_owned(), Value::Bool(false));
    report.insert("verdict".to_owned(), Value::String(verdict.to_owned()));
    report.insert(
        "boundary".to_owned(),
        Value::String("metadata join only: enriches phase-atom trace rows with provider correlation keys from provider-boundary events; does not mine, compile, score, promote, serve, local-accept, or claim savings".to_owned()),
    );
    let report = Value::Object(report);
    write_json_file(&report_path, &report)?;
    println!("provider_boundary_correlation_join_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  output_trace_path: {}", output_trace_path.display());
    println!("  trace_rows: {}", state.trace_rows);
    println!(
        "  trace_rows_joined_with_provider_keys: {}",
        state.trace_rows_joined_with_provider_keys
    );
    println!(
        "  score_ready_rows_with_provider_correlation: {}",
        state.score_ready_rows_with_provider_correlation
    );
    println!("  economic_capture_ready: {economic_capture_ready}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn build_provider_index(paths: &[PathBuf]) -> Result<ProviderBoundaryIndex, String> {
    let mut index = ProviderBoundaryIndex::default();
    for path in paths {
        let text = std::fs::read_to_string(path).map_err(|error| {
            format!(
                "failed to read provider-boundary source '{}': {error}",
                path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse provider-boundary source '{}' line {}: {error}",
                    path.display(),
                    line_index + 1
                )
            })?;
            if !row.is_object() {
                continue;
            }
            index.provider_rows += 1;
            let join_keys = metadata_join_keys(&row);
            let provider_keys = super::phase_atom_external_provider_correlation_keys(&row);
            index.provider_rows_with_join_keys += usize::from(!join_keys.is_empty());
            index.provider_rows_missing_join_keys += usize::from(join_keys.is_empty());
            index.provider_rows_with_provider_keys += usize::from(!provider_keys.is_empty());
            index.provider_rows_missing_provider_keys += usize::from(provider_keys.is_empty());
            if join_keys.is_empty() || provider_keys.is_empty() {
                continue;
            }
            let provider_keys = provider_keys.into_iter().collect::<BTreeSet<_>>();
            for join_key in join_keys {
                let entry = index.join_key_to_provider_keys.entry(join_key).or_default();
                if !entry.is_empty() && *entry != provider_keys {
                    index.join_key_collisions += 1;
                }
                entry.extend(provider_keys.iter().cloned());
            }
        }
    }
    Ok(index)
}

fn join_trace_row(
    row: &Value,
    index: &ProviderBoundaryIndex,
    state: &mut ProviderBoundaryJoinState,
) -> Value {
    state.trace_rows += 1;
    let join_keys = metadata_join_keys(row);
    state.trace_rows_with_join_keys += usize::from(!join_keys.is_empty());
    state.trace_rows_missing_join_keys += usize::from(join_keys.is_empty());

    let mut provider_keys = super::phase_atom_external_provider_correlation_keys(row)
        .into_iter()
        .collect::<BTreeSet<_>>();
    state.trace_rows_with_existing_provider_keys += usize::from(!provider_keys.is_empty());
    let before_join_len = provider_keys.len();
    for join_key in &join_keys {
        if let Some(keys) = index.join_key_to_provider_keys.get(join_key) {
            provider_keys.extend(keys.iter().cloned());
        }
    }
    let joined_new_keys = provider_keys.len() > before_join_len;
    state.trace_rows_joined_with_provider_keys += usize::from(joined_new_keys);
    state.trace_rows_missing_provider_join += usize::from(provider_keys.is_empty());

    let mut joined = row.as_object().cloned().unwrap_or_default();
    if !provider_keys.is_empty() {
        joined.insert(
            "external_provider_correlation_keys".to_owned(),
            Value::Array(provider_keys.iter().cloned().map(Value::String).collect()),
        );
    }
    joined.insert(
        "provider_correlation_ready".to_owned(),
        Value::Bool(!provider_keys.is_empty()),
    );
    joined.insert(
        "provider_boundary_join".to_owned(),
        serde_json::json!({
            "schema_version": "provider_boundary_correlation_join_v1",
            "join_keys": join_keys,
            "joined_new_provider_keys": joined_new_keys,
            "metadata_only_not_atoms": true
        }),
    );
    let joined = Value::Object(joined);
    observe_joined_trace_row(&joined, state);
    joined
}

fn observe_joined_trace_row(row: &Value, state: &mut ProviderBoundaryJoinState) {
    let provider_keys = super::phase_atom_external_provider_correlation_keys(row);
    let has_provider_keys = !provider_keys.is_empty();
    state.output_rows_with_provider_correlation_keys += usize::from(has_provider_keys);
    if provider_key_leaks_into_atoms(row) {
        state.rows_with_provider_key_atom_leak += 1;
    }
    let ready_for_route_family_mining =
        json_bool(row, &["ready_for_route_family_mining"]).unwrap_or(false);
    let ready_for_existing_shadow_scoring =
        json_bool(row, &["ready_for_existing_shadow_scoring"]).unwrap_or(false);
    state.rows_ready_for_route_family_mining += usize::from(ready_for_route_family_mining);
    state.rows_ready_for_existing_shadow_scoring += usize::from(ready_for_existing_shadow_scoring);
    let total_tokens = json_usize(row, &["token_cost", "total_tokens"])
        .or_else(|| json_usize(row, &["estimated_total_tokens"]))
        .unwrap_or(0);
    let economic_score_ready = ready_for_existing_shadow_scoring && total_tokens > 0;
    let zero_denominator_score_ready = ready_for_existing_shadow_scoring && total_tokens == 0;
    if ready_for_existing_shadow_scoring && has_provider_keys {
        state.score_ready_rows_with_provider_correlation += 1;
    }
    if ready_for_existing_shadow_scoring && !has_provider_keys {
        state.score_ready_rows_missing_provider_correlation += 1;
    }
    if economic_score_ready {
        state.economic_score_ready_rows += 1;
        if has_provider_keys {
            state.economic_score_ready_rows_with_provider_correlation += 1;
        } else {
            state.economic_score_ready_rows_missing_provider_correlation += 1;
        }
    }
    if zero_denominator_score_ready {
        state.zero_denominator_score_ready_rows += 1;
        if !has_provider_keys {
            state.zero_denominator_score_ready_rows_missing_provider_correlation += 1;
        }
    }
    state.rows_with_positive_tokens += usize::from(total_tokens > 0);
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

fn value_usize(value: usize) -> Value {
    Value::Number(serde_json::Number::from(value as u64))
}
