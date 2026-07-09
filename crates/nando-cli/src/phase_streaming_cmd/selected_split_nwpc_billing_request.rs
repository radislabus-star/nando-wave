use std::collections::BTreeSet;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use serde_json::Value;

use super::{json_bool, json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_SELECTED_SPLIT_NWPC_BILLING_REQUEST_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-selected-split-nwpc-billing-request-v1.report.json";
const DEFAULT_SELECTED_SPLIT_NWPC_BILLING_REQUEST_JSONL: &str =
    "target/nando-wave/streaming/phase-stream-selected-split-nwpc-billing-request-v1.jsonl";
const DEFAULT_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-shadow-replay-v1-realtrace-plus-verifier-sources.report.json";

pub(crate) fn run_phase_stream_selected_split_nwpc_billing_request_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_NWPC_BILLING_REQUEST_REPORT));
    let request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_NWPC_BILLING_REQUEST_JSONL));
    let shadow_replay_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let replay = read_json_value(&shadow_replay_report_path)?;
    let replay_verdict = json_string(&replay, &["verdict"]).unwrap_or_default();
    let replay_false_accepts =
        json_usize_path(&replay, &["future_false_accepts"]).unwrap_or(usize::MAX);
    let replay_mismatch_count =
        json_usize_path(&replay, &["replay_mismatch_count"]).unwrap_or(usize::MAX);
    let replay_market_money_claim_allowed =
        json_bool(&replay, &["market_money_claim_allowed"]).unwrap_or(true);
    let replay_local_accept_enabled = json_bool(&replay, &["local_accept_enabled"]).unwrap_or(true);
    let token_cost_estimate_used =
        json_bool(&replay, &["token_cost_estimate_used"]).unwrap_or(true);
    let provider_billing_evidence_present =
        json_bool(&replay, &["provider_billing_evidence_present"]).unwrap_or(false);

    let Some(unique_accepts) = replay.get("unique_accepts").and_then(Value::as_array) else {
        return Err(format!(
            "shadow replay report '{}' missing unique_accepts",
            shadow_replay_report_path.display()
        ));
    };

    if let Some(parent) = request_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create selected split billing request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let file = std::fs::File::create(&request_jsonl_path).map_err(|error| {
        format!(
            "failed to create selected split billing request '{}': {error}",
            request_jsonl_path.display()
        )
    })?;
    let mut writer = BufWriter::new(file);

    let mut seen_fingerprints = BTreeSet::new();
    let mut request_rows = 0usize;
    let mut skipped_duplicate_fingerprint_rows = 0usize;
    let mut request_rows_with_exact_cache_key = 0usize;
    let mut request_rows_with_match_key = 0usize;
    let mut provider_correlation_ready_rows = 0usize;
    let mut total_tokens_requiring_billing = 0usize;
    let mut current_known_cost_microusd = 0u64;

    for accepted in unique_accepts {
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
        let total_tokens = json_usize_path(accepted, &["total_tokens"]).unwrap_or(0);
        let total_cost_microusd = json_u64(accepted, &["total_cost_microusd"]).unwrap_or(0);
        total_tokens_requiring_billing =
            total_tokens_requiring_billing.saturating_add(total_tokens);
        current_known_cost_microusd =
            current_known_cost_microusd.saturating_add(total_cost_microusd);
        let provider_correlation_ready = exact_cache_key
            .as_deref()
            .is_some_and(|value| !value.is_empty())
            || !request_fingerprint.is_empty();
        provider_correlation_ready_rows += usize::from(provider_correlation_ready);
        request_rows += 1;

        let row = serde_json::json!({
            "schema_version": "phase_stream_selected_split_nwpc_billing_request_v1",
            "billing_request_id": format!("selected-split-nwpc-{}", request_rows),
            "request_fingerprint": request_fingerprint,
            "exact_cache_key": exact_cache_key,
            "external_provider_correlation_keys": [],
            "provider_correlation_ready": provider_correlation_ready,
            "match_keys": match_keys,
            "broad_class_id": json_string(accepted, &["broad_class_id"]),
            "task_name": json_string(accepted, &["task_name"]),
            "split_rule": json_string(accepted, &["split_rule"]),
            "package_fingerprint64": json_u64(accepted, &["package_fingerprint64"]),
            "stream_index": json_u64(accepted, &["stream_index"]),
            "margin_micro": json_i64_path(accepted, &["margin_micro"]),
            "threshold_micro": json_i64_path(accepted, &["threshold_micro"]),
            "estimated_total_tokens": total_tokens,
            "current_total_cost_microusd": total_cost_microusd,
            "token_evidence_missing": json_bool(accepted, &["token_evidence_missing"]).unwrap_or(total_tokens == 0),
            "cost_evidence_missing": true,
            "token_cost_estimate_used": token_cost_estimate_used,
            "provider_billing_evidence_present": false,
            "unique_cpu_accept_over_exact_cache": true,
            "verified_safe_accept": true,
            "false_accept": false,
            "local_accept_enabled": false,
            "market_money_claim_allowed": false,
            "boundary": "selected split nwpc billing request only: asks external provider billing evidence to attach real costs to runtime-replayed shadow CPU accepts; does not estimate missing money, promote, serve, or enable local_accept"
        });
        serde_json::to_writer(&mut writer, &row).map_err(|error| {
            format!(
                "failed to serialize selected split billing request '{}': {error}",
                request_jsonl_path.display()
            )
        })?;
        writer.write_all(b"\n").map_err(|error| {
            format!(
                "failed to write selected split billing request '{}': {error}",
                request_jsonl_path.display()
            )
        })?;
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush selected split billing request '{}': {error}",
            request_jsonl_path.display()
        )
    })?;

    let replay_accepts =
        json_usize_path(&replay, &["future_unique_accepts_over_exact_cache"]).unwrap_or(0);
    let replay_tokens = json_usize_path(&replay, &["future_tokens_saved"]).unwrap_or(0);
    let accept_parity = request_rows == replay_accepts;
    let token_parity = total_tokens_requiring_billing == replay_tokens;
    let request_ready = replay_verdict
        == "PHASE_STREAM_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_V1_PASS_RUNTIME_REPLAY"
        && replay_false_accepts == 0
        && replay_mismatch_count == 0
        && !replay_market_money_claim_allowed
        && !replay_local_accept_enabled
        && request_rows > 0
        && request_rows_with_match_key == request_rows
        && provider_correlation_ready_rows == request_rows
        && accept_parity
        && token_parity;
    let verdict = if request_ready {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_BILLING_REQUEST_V1_READY"
    } else {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_BILLING_REQUEST_V1_WATCH"
    };
    let report = serde_json::json!({
        "report_kind": "phase_stream_selected_split_nwpc_billing_request_v1",
        "shadow_replay_report_path": shadow_replay_report_path,
        "billing_request_jsonl_path": request_jsonl_path,
        "billing_request_rows": request_rows,
        "skipped_duplicate_fingerprint_rows": skipped_duplicate_fingerprint_rows,
        "request_rows_with_match_key": request_rows_with_match_key,
        "request_rows_with_exact_cache_key": request_rows_with_exact_cache_key,
        "provider_correlation_ready_rows": provider_correlation_ready_rows,
        "total_tokens_requiring_billing": total_tokens_requiring_billing,
        "current_known_cost_microusd": current_known_cost_microusd,
        "shadow_replay_unique_accepts_over_exact_cache": replay_accepts,
        "shadow_replay_tokens_saved": replay_tokens,
        "accept_parity": accept_parity,
        "token_parity": token_parity,
        "token_cost_estimate_used": token_cost_estimate_used,
        "provider_billing_evidence_present": provider_billing_evidence_present,
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
        "boundary": "billing request export only: emits exact runtime-replayed accepted events that need external provider billing evidence; it does not create evidence, estimate missing money, promote, serve, enable local_accept, or revive legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_selected_split_nwpc_billing_request_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  billing_request_jsonl_path: {}",
        request_jsonl_path.display()
    );
    println!("  billing_request_rows: {request_rows}");
    println!("  total_tokens_requiring_billing: {total_tokens_requiring_billing}");
    println!("  provider_billing_evidence_present: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn json_usize_path(value: &Value, path: &[&str]) -> Option<usize> {
    json_u64(value, path).and_then(|number| usize::try_from(number).ok())
}

fn json_i64_path(value: &Value, path: &[&str]) -> Option<i64> {
    let current = path
        .iter()
        .try_fold(value, |current, key| current.get(*key))?;
    current.as_i64().or_else(|| {
        current
            .as_u64()
            .and_then(|number| i64::try_from(number).ok())
    })
}
