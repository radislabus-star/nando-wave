use std::collections::BTreeSet;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_FRONTIER_BILLING_REQUEST_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-frontier-provider-billing-request-v1.report.json";
const DEFAULT_FRONTIER_BILLING_REQUEST_JSONL: &str =
    "target/nando-wave/streaming/phase-atom-frontier-provider-billing-request-v1.jsonl";
const DEFAULT_FRONTIER_SHADOW_REPLAY_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-frontier-shadow-replay-v1.report.json";

pub(crate) fn run_phase_stream_phase_atom_frontier_billing_request_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_FRONTIER_BILLING_REQUEST_REPORT));
    let request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_FRONTIER_BILLING_REQUEST_JSONL));
    let frontier_shadow_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_FRONTIER_SHADOW_REPLAY_REPORT));

    let replay = read_json_value(&frontier_shadow_report_path)?;
    let decision_log_path =
        PathBuf::from(json_string(&replay, &["decision_log_path"]).ok_or_else(|| {
            format!(
                "frontier shadow report '{}' missing decision_log_path",
                frontier_shadow_report_path.display()
            )
        })?);
    let local_accept_enabled = json_bool(&replay, &["local_accept_enabled"]).unwrap_or(false);
    let market_money_claim_allowed =
        json_bool(&replay, &["market_money_claim_allowed"]).unwrap_or(false);
    let shadow_runtime_kind =
        json_string(&replay, &["shadow_runtime_kind"]).unwrap_or_else(|| "unknown".to_owned());
    let runtime_margin_parity_mismatches =
        json_u64(&replay, &["runtime_margin_parity_mismatches"]).unwrap_or(0);
    let runtime_decision_parity_mismatches =
        json_u64(&replay, &["runtime_decision_parity_mismatches"]).unwrap_or(0);
    let manual_class_list_used = json_bool(&replay, &["discovery_mode", "manual_class_list_used"])
        .or_else(|| json_bool(&replay, &["manual_class_list_used"]))
        .unwrap_or(true);
    let static_topn_seed_used = json_bool(&replay, &["discovery_mode", "static_topn_seed_used"])
        .or_else(|| json_bool(&replay, &["static_topn_seed_used"]))
        .unwrap_or(false);
    let online_discovery_used = json_bool(&replay, &["discovery_mode", "online_discovery_used"])
        .or_else(|| json_bool(&replay, &["online_discovery_used"]))
        .unwrap_or(false);
    let marginal_denominator_delta_used = json_bool(
        &replay,
        &["discovery_mode", "marginal_denominator_delta_used"],
    )
    .or_else(|| json_bool(&replay, &["marginal_denominator_delta_used"]))
    .unwrap_or(false);
    let portfolio_gate_passed = json_bool(&replay, &["discovery_mode", "portfolio_gate_passed"])
        .or_else(|| json_bool(&replay, &["portfolio_gate_passed"]))
        .unwrap_or(false);
    let runtime_replay_passed = json_bool(&replay, &["discovery_mode", "runtime_replay_passed"])
        .or_else(|| json_bool(&replay, &["runtime_replay_passed"]))
        .unwrap_or(
            runtime_margin_parity_mismatches == 0 && runtime_decision_parity_mismatches == 0,
        );
    let product_dynamic_discovery_claim_allowed = !manual_class_list_used
        && !static_topn_seed_used
        && online_discovery_used
        && marginal_denominator_delta_used
        && portfolio_gate_passed
        && runtime_replay_passed;

    let decision_text = std::fs::read_to_string(&decision_log_path).map_err(|error| {
        format!(
            "failed to read frontier decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    if let Some(parent) = request_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create billing request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let request_file = std::fs::File::create(&request_jsonl_path).map_err(|error| {
        format!(
            "failed to create billing request jsonl '{}': {error}",
            request_jsonl_path.display()
        )
    })?;
    let mut request_writer = BufWriter::new(request_file);

    let mut decision_rows = 0usize;
    let mut unique_cpu_accept_rows = 0usize;
    let mut missing_provider_cost_requests = 0usize;
    let mut skipped_known_cost_rows = 0usize;
    let mut skipped_non_unique_accept_rows = 0usize;
    let mut skipped_unsafe_rows = 0usize;
    let mut request_fingerprint_rows = 0usize;
    let mut exact_cache_key_rows = 0usize;
    let mut trace_id_rows = 0usize;
    let mut external_provider_correlation_key_rows = 0usize;
    let mut external_provider_correlation_key_count = 0usize;
    let mut unique_request_fingerprints = BTreeSet::<String>::new();
    let mut unique_exact_cache_keys = BTreeSet::<String>::new();
    let mut total_tokens_requiring_billing = 0usize;
    let mut current_known_cost_microusd = 0u64;

    for (line_index, line) in decision_text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        decision_rows += 1;
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse frontier decision log '{}' line {}: {error}",
                decision_log_path.display(),
                line_index + 1
            )
        })?;

        let unique_accept =
            json_bool(&row, &["unique_cpu_accept_over_exact_cache"]).unwrap_or(false);
        if !unique_accept {
            skipped_non_unique_accept_rows += 1;
            continue;
        }
        unique_cpu_accept_rows += 1;

        let false_accept = json_bool(&row, &["false_accept"]).unwrap_or(false);
        let wrong_win = json_bool(&row, &["wrong_win"]).unwrap_or(false);
        if false_accept || wrong_win {
            skipped_unsafe_rows += 1;
            continue;
        }

        let token_cost = json_at(&row, &["token_cost"]);
        let total_tokens = json_usize(token_cost.and_then(|cost| cost.get("total_tokens")))
            .or_else(|| json_usize(json_at(&row, &["total_tokens"])))
            .unwrap_or(0);
        let current_cost = json_u64_at(token_cost.and_then(|cost| cost.get("total_cost_microusd")))
            .or_else(|| json_u64_at(json_at(&row, &["total_cost_microusd"])))
            .unwrap_or(0);
        let cost_evidence_missing =
            json_bool(&row, &["token_cost", "cost_evidence_missing"]).unwrap_or(current_cost == 0);
        let token_evidence_missing =
            json_bool(&row, &["token_cost", "token_evidence_missing"]).unwrap_or(total_tokens == 0);
        if current_cost > 0 && !cost_evidence_missing {
            skipped_known_cost_rows += 1;
            current_known_cost_microusd = current_known_cost_microusd.saturating_add(current_cost);
            continue;
        }

        let request_fingerprint = json_string(&row, &["request_fingerprint"]);
        let exact_cache_key = json_string(&row, &["exact_cache_key"]);
        let trace_id = json_string(&row, &["trace_id"]);
        let external_provider_correlation_keys =
            super::phase_atom_external_provider_correlation_keys(&row);
        let provider_correlation_ready = !external_provider_correlation_keys.is_empty();
        if provider_correlation_ready {
            external_provider_correlation_key_rows += 1;
            external_provider_correlation_key_count = external_provider_correlation_key_count
                .saturating_add(external_provider_correlation_keys.len());
        }
        if let Some(value) = request_fingerprint
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            request_fingerprint_rows += 1;
            unique_request_fingerprints.insert(value.to_owned());
        }
        if let Some(value) = exact_cache_key.as_deref().filter(|value| !value.is_empty()) {
            exact_cache_key_rows += 1;
            unique_exact_cache_keys.insert(value.to_owned());
        }
        if trace_id.as_deref().is_some_and(|value| !value.is_empty()) {
            trace_id_rows += 1;
        }

        let mut match_keys = Vec::new();
        if let Some(value) = &request_fingerprint {
            match_keys.push(format!("request_fingerprint:{value}"));
        }
        if let Some(value) = &exact_cache_key {
            match_keys.push(format!("exact_cache_key:{value}"));
        }
        if let Some(value) = &trace_id {
            match_keys.push(format!("trace_id:{value}"));
        }
        for value in &external_provider_correlation_keys {
            match_keys.push(format!("provider_correlation:{value}"));
        }

        total_tokens_requiring_billing =
            total_tokens_requiring_billing.saturating_add(total_tokens);
        missing_provider_cost_requests += 1;

        let billing_request = serde_json::json!({
            "schema_version": "phase_atom_frontier_provider_billing_request_v1",
            "billing_request_id": format!(
                "frontier-cpu-accept-{}-{}",
                json_u64(&row, &["denominator_row_index"]).unwrap_or(line_index as u64),
                json_string(&row, &["profile_task_name"]).unwrap_or_else(|| "unknown".to_owned())
            ),
            "request_fingerprint": request_fingerprint,
            "exact_cache_key": exact_cache_key,
            "trace_id": trace_id,
            "external_provider_correlation_keys": external_provider_correlation_keys,
            "provider_correlation_ready": provider_correlation_ready,
            "match_keys": match_keys,
            "source_trace_path": json_string(&row, &["source_trace_path"]),
            "source_line_index": json_u64(&row, &["source_line_index"]),
            "denominator_row_index": json_u64(&row, &["denominator_row_index"]),
            "profile_task_name": json_string(&row, &["profile_task_name"]),
            "profile_action_family": json_string(&row, &["profile_action_family"]),
            "package_fingerprint64": json_u64(&row, &["package_fingerprint64"]),
            "estimated_total_tokens": total_tokens,
            "current_total_cost_microusd": current_cost,
            "token_evidence_missing": token_evidence_missing,
            "cost_evidence_missing": cost_evidence_missing,
            "unique_cpu_accept_over_exact_cache": unique_accept,
            "false_accept": false_accept,
            "wrong_win": wrong_win,
            "local_accept_enabled": false,
            "market_money_claim_allowed": false,
            "boundary": "provider billing request only: asks external billing evidence to fill provider cost for shadow CPU accepts; does not estimate missing money, compile, promote, serve, or enable local_accept"
        });
        serde_json::to_writer(&mut request_writer, &billing_request).map_err(|error| {
            format!(
                "failed to serialize billing request '{}': {error}",
                request_jsonl_path.display()
            )
        })?;
        request_writer.write_all(b"\n").map_err(|error| {
            format!(
                "failed to write billing request '{}': {error}",
                request_jsonl_path.display()
            )
        })?;
    }
    request_writer.flush().map_err(|error| {
        format!(
            "failed to flush billing request '{}': {error}",
            request_jsonl_path.display()
        )
    })?;

    let report = serde_json::json!({
        "report_kind": "phase_atom_frontier_provider_billing_request_v1",
        "frontier_shadow_report_path": frontier_shadow_report_path,
        "decision_log_path": decision_log_path,
        "billing_request_jsonl_path": request_jsonl_path,
        "shadow_runtime_kind": shadow_runtime_kind,
        "runtime_margin_parity_mismatches": runtime_margin_parity_mismatches,
        "runtime_decision_parity_mismatches": runtime_decision_parity_mismatches,
        "input_market_money_claim_allowed": market_money_claim_allowed,
        "discovery_mode": {
            "manual_class_list_used": manual_class_list_used,
            "static_topn_seed_used": static_topn_seed_used,
            "online_discovery_used": online_discovery_used,
            "marginal_denominator_delta_used": marginal_denominator_delta_used,
            "portfolio_gate_passed": portfolio_gate_passed,
            "runtime_replay_passed": runtime_replay_passed,
            "product_dynamic_discovery_claim_allowed": product_dynamic_discovery_claim_allowed,
            "policy": "billing request export is evidence plumbing only; manual/debug frontier cannot become product dynamic discovery"
        },
        "decision_rows": decision_rows,
        "unique_cpu_accept_rows": unique_cpu_accept_rows,
        "missing_provider_cost_requests": missing_provider_cost_requests,
        "skipped_known_cost_rows": skipped_known_cost_rows,
        "skipped_non_unique_accept_rows": skipped_non_unique_accept_rows,
        "skipped_unsafe_rows": skipped_unsafe_rows,
        "request_fingerprint_rows": request_fingerprint_rows,
        "exact_cache_key_rows": exact_cache_key_rows,
        "trace_id_rows": trace_id_rows,
        "external_provider_correlation_key_rows": external_provider_correlation_key_rows,
        "external_provider_correlation_missing_rows": missing_provider_cost_requests.saturating_sub(external_provider_correlation_key_rows),
        "external_provider_correlation_key_count": external_provider_correlation_key_count,
        "unique_request_fingerprints": unique_request_fingerprints.len(),
        "unique_exact_cache_keys": unique_exact_cache_keys.len(),
        "total_tokens_requiring_billing": total_tokens_requiring_billing,
        "current_known_cost_microusd": current_known_cost_microusd,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false
        },
        "local_accept_enabled": local_accept_enabled,
        "market_money_claim_allowed": false,
        "product_promotion_allowed": false,
        "provider_correlation_gate": {
            "external_provider_correlation_key_rows": external_provider_correlation_key_rows,
            "external_provider_correlation_missing_rows": missing_provider_cost_requests.saturating_sub(external_provider_correlation_key_rows),
            "external_provider_correlation_key_count": external_provider_correlation_key_count,
            "provider_correlation_required_for_market_money": true,
            "market_money_claim_allowed": false
        },
        "verdict": if missing_provider_cost_requests > 0 {
            "PHASE_ATOM_FRONTIER_PROVIDER_BILLING_REQUEST_READY"
        } else {
            "PHASE_ATOM_FRONTIER_PROVIDER_BILLING_REQUEST_EMPTY"
        },
        "boundary": "reporting/export only: emits provider-billing request rows for shadow CPU accepts missing external provider cost evidence; does not estimate money, compile, promote, serve, enable local_accept, or revive legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_atom_frontier_provider_billing_request_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  billing_request_jsonl_path: {}",
        request_jsonl_path.display()
    );
    println!("  decision_rows: {decision_rows}");
    println!("  unique_cpu_accept_rows: {unique_cpu_accept_rows}");
    println!("  missing_provider_cost_requests: {missing_provider_cost_requests}");
    println!("  total_tokens_requiring_billing: {total_tokens_requiring_billing}");
    println!("  market_money_claim_allowed: false");
    Ok(())
}

fn read_json_value(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse JSON report '{}': {error}", path.display()))
}

fn write_json_file(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create report dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize report '{}': {error}", path.display()))?;
    std::fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write report '{}': {error}", path.display()))
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

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    json_u64_at(json_at(value, path))
}

fn json_u64_at(value: Option<&Value>) -> Option<u64> {
    value.and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
    })
}

fn json_usize(value: Option<&Value>) -> Option<usize> {
    json_u64_at(value).and_then(|value| usize::try_from(value).ok())
}
