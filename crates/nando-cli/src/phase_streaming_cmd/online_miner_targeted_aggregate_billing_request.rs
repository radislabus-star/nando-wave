use std::collections::BTreeSet;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use serde_json::Value;

use super::{json_bool, json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_TARGETED_AGGREGATE_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-aggregate-billing-request-v1.report.json";
const DEFAULT_TARGETED_AGGREGATE_BILLING_REQUEST_JSONL: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-aggregate-billing-request-v1.jsonl";
const DEFAULT_TARGETED_AGGREGATE_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-aggregate-gate-v1-agent-followup-12k-current.report.json";

pub(crate) fn run_phase_stream_online_miner_targeted_aggregate_billing_request_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_AGGREGATE_BILLING_REQUEST_REPORT));
    let request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_AGGREGATE_BILLING_REQUEST_JSONL));
    let aggregate_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_AGGREGATE_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let aggregate = read_json_value(&aggregate_report_path)?;
    let aggregate_verdict = json_string(&aggregate, &["verdict"]).unwrap_or_default();
    let aggregate_calls_tokens_allowed =
        json_bool(&aggregate, &["calls_tokens_claim_allowed"]).unwrap_or(false);
    let aggregate_market_money_allowed =
        json_bool(&aggregate, &["market_money_claim_allowed"]).unwrap_or(true);
    let aggregate_local_accept = json_bool(&aggregate, &["local_accept_enabled"]).unwrap_or(true);
    let accepted_events_path = json_string(&aggregate, &["accepted_events_jsonl_path"])
        .map(PathBuf::from)
        .ok_or_else(|| "targeted aggregate report missing accepted_events_jsonl_path".to_owned())?;
    let accepted_events = read_jsonl_values(&accepted_events_path)?;

    if let Some(parent) = request_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create targeted aggregate billing request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let file = std::fs::File::create(&request_jsonl_path).map_err(|error| {
        format!(
            "failed to create targeted aggregate billing request '{}': {error}",
            request_jsonl_path.display()
        )
    })?;
    let mut writer = BufWriter::new(file);

    let mut seen_fingerprints = BTreeSet::new();
    let mut request_rows = 0usize;
    let mut skipped_duplicate_fingerprint_rows = 0usize;
    let mut request_rows_with_match_key = 0usize;
    let mut request_rows_with_exact_cache_key = 0usize;
    let mut provider_correlation_ready_rows = 0usize;
    let mut total_tokens_requiring_billing = 0usize;
    let mut current_known_cost_microusd = 0u64;

    for accepted in &accepted_events {
        let Some(request_fingerprint) = json_string(accepted, &["request_fingerprint"]) else {
            continue;
        };
        if !seen_fingerprints.insert(request_fingerprint.clone()) {
            skipped_duplicate_fingerprint_rows += 1;
            continue;
        }
        let exact_cache_key = json_string(accepted, &["exact_cache_key"]);
        let mut match_keys = vec![format!("request_fingerprint:{request_fingerprint}")];
        if let Some(key) = exact_cache_key.as_deref().filter(|value| !value.is_empty()) {
            request_rows_with_exact_cache_key += 1;
            match_keys.push(format!("exact_cache_key:{key}"));
        }
        match_keys.sort();
        match_keys.dedup();
        if !match_keys.is_empty() {
            request_rows_with_match_key += 1;
        }
        let provider_correlation_ready = !request_fingerprint.is_empty()
            || exact_cache_key
                .as_deref()
                .is_some_and(|value| !value.is_empty());
        provider_correlation_ready_rows += usize::from(provider_correlation_ready);
        let total_tokens = json_u64(accepted, &["total_tokens"])
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let total_cost_microusd = json_u64(accepted, &["total_cost_microusd"]).unwrap_or_default();
        total_tokens_requiring_billing =
            total_tokens_requiring_billing.saturating_add(total_tokens);
        current_known_cost_microusd =
            current_known_cost_microusd.saturating_add(total_cost_microusd);
        request_rows += 1;

        let row = serde_json::json!({
            "schema_version": "phase_stream_online_miner_targeted_aggregate_billing_request_v1",
            "billing_request_id": format!("targeted-aggregate-{request_rows}"),
            "source": json_string(accepted, &["source"]),
            "request_fingerprint": request_fingerprint,
            "exact_cache_key": exact_cache_key,
            "external_provider_correlation_keys": [],
            "provider_correlation_ready": provider_correlation_ready,
            "match_keys": match_keys,
            "package_fingerprint64": json_u64(accepted, &["package_fingerprint64"]),
            "margin_micro": json_i64(accepted, &["margin_micro"]),
            "threshold_micro": json_i64(accepted, &["threshold_micro"]),
            "estimated_total_tokens": total_tokens,
            "current_total_cost_microusd": total_cost_microusd,
            "token_evidence_missing": total_tokens == 0,
            "cost_evidence_missing": true,
            "token_cost_estimate_used": true,
            "provider_billing_evidence_present": false,
            "unique_cpu_accept_over_exact_cache": true,
            "verified_safe_accept": true,
            "false_accept": false,
            "local_accept_enabled": false,
            "market_money_claim_allowed": false,
            "boundary": "targeted aggregate billing request only: asks external provider billing evidence to attach real provider costs to deduped shadow CPU accepts; does not estimate missing money, promote, serve, or enable local_accept"
        });
        serde_json::to_writer(&mut writer, &row).map_err(|error| {
            format!(
                "failed to serialize targeted aggregate billing request '{}': {error}",
                request_jsonl_path.display()
            )
        })?;
        writer.write_all(b"\n").map_err(|error| {
            format!(
                "failed to write targeted aggregate billing request '{}': {error}",
                request_jsonl_path.display()
            )
        })?;
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush targeted aggregate billing request '{}': {error}",
            request_jsonl_path.display()
        )
    })?;

    let aggregate_accepts =
        json_u64(&aggregate, &["aggregate_unique_accepts_over_exact_cache"]).unwrap_or_default();
    let aggregate_tokens = json_u64(&aggregate, &["aggregate_tokens_saved"]).unwrap_or_default();
    let accept_parity = request_rows as u64 == aggregate_accepts;
    let token_parity = total_tokens_requiring_billing as u64 == aggregate_tokens;
    let request_ready = aggregate_verdict
        == "PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_GATE_V1_PASS_CALLS_TOKENS_MONEY_BLOCKED"
        && aggregate_calls_tokens_allowed
        && !aggregate_market_money_allowed
        && !aggregate_local_accept
        && request_rows > 0
        && request_rows_with_match_key == request_rows
        && provider_correlation_ready_rows == request_rows
        && accept_parity
        && token_parity;
    let verdict = if request_ready {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_BILLING_REQUEST_V1_READY"
    } else {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_BILLING_REQUEST_V1_WATCH"
    };
    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_targeted_aggregate_billing_request_v1",
        "aggregate_report_path": aggregate_report_path,
        "accepted_events_jsonl_path": accepted_events_path,
        "billing_request_jsonl_path": request_jsonl_path,
        "billing_request_rows": request_rows,
        "skipped_duplicate_fingerprint_rows": skipped_duplicate_fingerprint_rows,
        "request_rows_with_match_key": request_rows_with_match_key,
        "request_rows_with_exact_cache_key": request_rows_with_exact_cache_key,
        "provider_correlation_ready_rows": provider_correlation_ready_rows,
        "total_tokens_requiring_billing": total_tokens_requiring_billing,
        "current_known_cost_microusd": current_known_cost_microusd,
        "aggregate_unique_accepts_over_exact_cache": aggregate_accepts,
        "aggregate_tokens_saved": aggregate_tokens,
        "accept_parity": accept_parity,
        "token_parity": token_parity,
        "token_cost_estimate_used": true,
        "provider_billing_evidence_present": false,
        "billing_request_only": true,
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
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
        "boundary": "billing request export only: emits deduped aggregate shadow accepted events that need external provider billing evidence; it does not create evidence, estimate missing money, promote, serve, enable local_accept, or revive legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_targeted_aggregate_billing_request_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  billing_request_jsonl_path: {}",
        request_jsonl_path.display()
    );
    println!("  billing_request_rows: {request_rows}");
    println!("  total_tokens_requiring_billing: {total_tokens_requiring_billing}");
    println!("  accept_parity: {accept_parity}");
    println!("  token_parity: {token_parity}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn read_jsonl_values(path: &PathBuf) -> Result<Vec<Value>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let mut rows = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        rows.push(serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse '{}' line {}: {error}",
                path.display(),
                line_index + 1
            )
        })?);
    }
    Ok(rows)
}

fn json_i64(value: &Value, path: &[&str]) -> Option<i64> {
    let current = path
        .iter()
        .try_fold(value, |current, key| current.get(*key))?;
    current.as_i64().or_else(|| {
        current
            .as_u64()
            .and_then(|number| i64::try_from(number).ok())
    })
}
