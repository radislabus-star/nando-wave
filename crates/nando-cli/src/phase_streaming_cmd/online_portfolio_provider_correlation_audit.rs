use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_PROVIDER_CORRELATION_AUDIT_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-provider-correlation-audit-v1.report.json";

#[derive(Default)]
struct ProviderCorrelationAuditState {
    total_rows: usize,
    rows_with_provider_correlation_keys: usize,
    rows_missing_provider_correlation_keys: usize,
    provider_key_count: usize,
    rows_with_provider_key_atom_leak: usize,
    rows_ready_for_route_family_mining: usize,
    rows_ready_for_existing_shadow_scoring: usize,
    rows_with_shadow_request: usize,
    rows_with_verified_safe_accept: usize,
    rows_with_positive_tokens: usize,
    rows_with_positive_cost: usize,
    cpu_shadow_accept_rows: usize,
    cpu_shadow_accept_rows_with_provider_correlation_keys: usize,
    billing_request_rows: usize,
    billing_request_rows_with_provider_correlation_keys: usize,
    schema_counts: BTreeMap<String, usize>,
    traffic_source_counts: BTreeMap<String, usize>,
    missing_samples: Vec<Value>,
    leak_samples: Vec<Value>,
}

pub(crate) fn run_phase_stream_online_miner_portfolio_provider_correlation_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_CORRELATION_AUDIT_REPORT));
    let trace_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if trace_paths.is_empty() {
        return Err(
            "at least one JSONL path is required for provider correlation audit".to_owned(),
        );
    }

    let mut state = ProviderCorrelationAuditState::default();
    for path in &trace_paths {
        scan_jsonl_path(path, &mut state)?;
    }

    let all_rows_have_provider_correlation =
        state.total_rows > 0 && state.rows_missing_provider_correlation_keys == 0;
    let all_cpu_accepts_have_provider_correlation = state.cpu_shadow_accept_rows > 0
        && state.cpu_shadow_accept_rows_with_provider_correlation_keys
            == state.cpu_shadow_accept_rows;
    let all_billing_requests_have_provider_correlation = state.billing_request_rows > 0
        && state.billing_request_rows_with_provider_correlation_keys == state.billing_request_rows;
    let provider_correlation_metadata_only = state.rows_with_provider_key_atom_leak == 0;
    let billing_join_ready_for_selected_accepts =
        all_cpu_accepts_have_provider_correlation && provider_correlation_metadata_only;

    let verdict = if state.rows_with_provider_key_atom_leak > 0 {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_CORRELATION_AUDIT_V1_FAIL_PROVIDER_KEY_ATOM_LEAK"
    } else if all_cpu_accepts_have_provider_correlation
        || all_billing_requests_have_provider_correlation
    {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_CORRELATION_AUDIT_V1_PASS_JOIN_READY_SCOPE"
    } else if state.rows_with_provider_correlation_keys > 0 {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_CORRELATION_AUDIT_V1_WATCH_PARTIAL_PROVIDER_CORRELATION"
    } else if state.total_rows > 0 {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_CORRELATION_AUDIT_V1_WATCH_NO_PROVIDER_CORRELATION"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_CORRELATION_AUDIT_V1_WATCH_EMPTY_INPUT"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_portfolio_provider_correlation_audit_v1",
        "input_paths": trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>(),
        "total_rows": state.total_rows,
        "rows_with_provider_correlation_keys": state.rows_with_provider_correlation_keys,
        "rows_missing_provider_correlation_keys": state.rows_missing_provider_correlation_keys,
        "provider_key_count": state.provider_key_count,
        "rows_with_provider_key_atom_leak": state.rows_with_provider_key_atom_leak,
        "provider_correlation_metadata_only": provider_correlation_metadata_only,
        "all_rows_have_provider_correlation": all_rows_have_provider_correlation,
        "rows_ready_for_route_family_mining": state.rows_ready_for_route_family_mining,
        "rows_ready_for_existing_shadow_scoring": state.rows_ready_for_existing_shadow_scoring,
        "rows_with_shadow_request": state.rows_with_shadow_request,
        "rows_with_verified_safe_accept": state.rows_with_verified_safe_accept,
        "rows_with_positive_tokens": state.rows_with_positive_tokens,
        "rows_with_positive_cost": state.rows_with_positive_cost,
        "cpu_shadow_accept_rows": state.cpu_shadow_accept_rows,
        "cpu_shadow_accept_rows_with_provider_correlation_keys": state.cpu_shadow_accept_rows_with_provider_correlation_keys,
        "all_cpu_accepts_have_provider_correlation": all_cpu_accepts_have_provider_correlation,
        "billing_join_ready_for_selected_accepts": billing_join_ready_for_selected_accepts,
        "billing_request_rows": state.billing_request_rows,
        "billing_request_rows_with_provider_correlation_keys": state.billing_request_rows_with_provider_correlation_keys,
        "all_billing_requests_have_provider_correlation": all_billing_requests_have_provider_correlation,
        "schema_counts": state.schema_counts,
        "traffic_source_counts": state.traffic_source_counts,
        "missing_provider_correlation_samples": state.missing_samples,
        "provider_key_atom_leak_samples": state.leak_samples,
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "product_promotion_allowed": false,
        "market_money_claim_allowed": false,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false
        },
        "verdict": verdict,
        "boundary": "provider correlation audit only: measures whether live rows can be joined to external provider billing evidence; does not compile, score, promote, enable local_accept, estimate missing money, or put provider ids into phase atoms"
    });
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_provider_correlation_audit_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  total_rows: {}", state.total_rows);
    println!(
        "  rows_with_provider_correlation_keys: {}",
        state.rows_with_provider_correlation_keys
    );
    println!(
        "  cpu_shadow_accept_rows_with_provider_correlation_keys: {}/{}",
        state.cpu_shadow_accept_rows_with_provider_correlation_keys, state.cpu_shadow_accept_rows
    );
    println!(
        "  billing_request_rows_with_provider_correlation_keys: {}/{}",
        state.billing_request_rows_with_provider_correlation_keys, state.billing_request_rows
    );
    println!(
        "  rows_with_provider_key_atom_leak: {}",
        state.rows_with_provider_key_atom_leak
    );
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn scan_jsonl_path(path: &Path, state: &mut ProviderCorrelationAuditState) -> Result<(), String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read JSONL '{}': {error}", path.display()))?;
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse JSONL '{}' line {}: {error}",
                path.display(),
                line_index + 1
            )
        })?;
        if !row.is_object() {
            continue;
        }
        scan_row(path, line_index + 1, &row, state);
    }
    Ok(())
}

fn scan_row(
    path: &Path,
    line_number: usize,
    row: &Value,
    state: &mut ProviderCorrelationAuditState,
) {
    state.total_rows += 1;
    let keys = super::phase_atom_external_provider_correlation_keys(row);
    let has_keys = !keys.is_empty();
    state.provider_key_count = state.provider_key_count.saturating_add(keys.len());
    if has_keys {
        state.rows_with_provider_correlation_keys += 1;
    } else {
        state.rows_missing_provider_correlation_keys += 1;
        push_sample(
            &mut state.missing_samples,
            path,
            line_number,
            row,
            "missing_provider_correlation_keys",
        );
    }

    let schema = json_string(row, &["schema_version"]).unwrap_or_else(|| "unknown".to_owned());
    *state.schema_counts.entry(schema.clone()).or_default() += 1;
    let traffic_source =
        json_string(row, &["traffic_source"]).unwrap_or_else(|| "unknown".to_owned());
    *state
        .traffic_source_counts
        .entry(traffic_source)
        .or_default() += 1;

    state.rows_ready_for_route_family_mining +=
        usize::from(json_bool(row, &["ready_for_route_family_mining"]).unwrap_or(false));
    state.rows_ready_for_existing_shadow_scoring +=
        usize::from(json_bool(row, &["ready_for_existing_shadow_scoring"]).unwrap_or(false));
    state.rows_with_shadow_request += usize::from(
        json_bool(row, &["has_shadow_request"]).unwrap_or(false)
            || row
                .get("nando_shadow_request")
                .is_some_and(Value::is_object),
    );
    state.rows_with_verified_safe_accept += usize::from(
        row.get("verified_safe_accept")
            .and_then(Value::as_bool)
            .is_some(),
    );
    state.rows_with_positive_tokens += usize::from(
        json_usize(row, &["estimated_total_tokens"]).unwrap_or(0) > 0
            || json_usize(row, &["token_cost", "total_tokens"]).unwrap_or(0) > 0,
    );
    state.rows_with_positive_cost += usize::from(
        json_u64(row, &["current_total_cost_microusd"]).unwrap_or(0) > 0
            || json_u64(row, &["token_cost", "total_cost_microusd"]).unwrap_or(0) > 0,
    );

    let cpu_shadow_accept = json_bool(row, &["unique_cpu_accept_over_exact_cache"])
        .unwrap_or(false)
        || json_bool(row, &["local_operator_shadow_decision"]).unwrap_or(false);
    if cpu_shadow_accept {
        state.cpu_shadow_accept_rows += 1;
        state.cpu_shadow_accept_rows_with_provider_correlation_keys += usize::from(has_keys);
    }

    let billing_request_row = schema.contains("billing_request")
        || row.get("billing_request_id").is_some()
        || row.get("match_keys").is_some()
            && row.get("estimated_total_tokens").is_some()
            && row.get("current_total_cost_microusd").is_some();
    if billing_request_row {
        state.billing_request_rows += 1;
        state.billing_request_rows_with_provider_correlation_keys += usize::from(has_keys);
    }

    if provider_key_leaks_into_atoms(row) {
        state.rows_with_provider_key_atom_leak += 1;
        push_sample(
            &mut state.leak_samples,
            path,
            line_number,
            row,
            "provider_key_atom_leak",
        );
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
        "result_atoms",
        "route_hint_atoms",
    ] {
        collect_atom_strings(row.get(key), &mut atoms);
    }
    atoms.iter().any(|atom| {
        [
            "provider_request_id:",
            "provider_response_id:",
            "provider_trace_id:",
            "external_provider_request_id:",
            "openai_request_id:",
            "anthropic_request_id:",
            "custom_id:",
        ]
        .iter()
        .any(|prefix| atom.starts_with(prefix))
    })
}

fn collect_atom_strings(value: Option<&Value>, atoms: &mut Vec<String>) {
    match value {
        Some(Value::String(text)) => atoms.push(text.clone()),
        Some(Value::Array(items)) => {
            for item in items {
                collect_atom_strings(Some(item), atoms);
            }
        }
        Some(Value::Object(map)) => {
            for value in map.values() {
                collect_atom_strings(Some(value), atoms);
            }
        }
        _ => {}
    }
}

fn push_sample(
    samples: &mut Vec<Value>,
    path: &Path,
    line_number: usize,
    row: &Value,
    reason: &'static str,
) {
    if samples.len() >= 16 {
        return;
    }
    samples.push(serde_json::json!({
        "reason": reason,
        "path": path.display().to_string(),
        "line_number": line_number,
        "schema_version": json_string(row, &["schema_version"]),
        "trace_id": json_string(row, &["trace_id"]),
        "request_fingerprint": json_string(row, &["request_fingerprint"]),
        "exact_cache_key": json_string(row, &["exact_cache_key"]),
        "traffic_source": json_string(row, &["traffic_source"]),
        "has_shadow_request": json_bool(row, &["has_shadow_request"])
            .unwrap_or_else(|| row.get("nando_shadow_request").is_some_and(Value::is_object)),
        "unique_cpu_accept_over_exact_cache": json_bool(row, &["unique_cpu_accept_over_exact_cache"]),
        "local_operator_shadow_decision": json_bool(row, &["local_operator_shadow_decision"])
    }));
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

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path).and_then(Value::as_bool)
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
