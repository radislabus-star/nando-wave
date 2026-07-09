use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-request-v1.report.json";
const DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_JSONL: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-request-v1.jsonl";
const DEFAULT_ONLINE_MINER_PORTFOLIO_RUNTIME_REPLAY_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-portfolio-runtime-replay-v1.report.json";

#[derive(Clone, Debug)]
struct BillingSelectedBucket {
    rank: usize,
    bucket_key: String,
    task_name: String,
    threshold_micro: i64,
    runtime_replay_start_event_ordinal: usize,
}

#[derive(Clone, Debug)]
struct BillingDecision {
    request_fingerprint: String,
    exact_cache_key: Option<String>,
    external_provider_correlation_keys: Vec<String>,
    exact_cache_hit: bool,
    verified_safe_accept: bool,
    margin_micro: i64,
    denominator_row_index: u64,
    package_fingerprint64: u64,
    total_tokens: usize,
    total_cost_microusd: u64,
    token_evidence_missing: bool,
    cost_evidence_missing: bool,
    reference_runtime_parity_mismatch: bool,
}

pub(crate) fn run_phase_stream_online_miner_portfolio_billing_request_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_REPORT));
    let request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_JSONL));
    let runtime_replay_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_RUNTIME_REPLAY_REPORT));

    let runtime = read_json_value(&runtime_replay_report_path)?;
    let selector_report_path = PathBuf::from(
        json_string(&runtime, &["selector_report_path"]).ok_or_else(|| {
            format!(
                "runtime replay report '{}' missing selector_report_path",
                runtime_replay_report_path.display()
            )
        })?,
    );
    let decision_log_path = PathBuf::from(
        json_string(&runtime, &["decision_log_path"]).ok_or_else(|| {
            format!(
                "runtime replay report '{}' missing decision_log_path",
                runtime_replay_report_path.display()
            )
        })?,
    );
    let selector = read_json_value(&selector_report_path)?;
    let selected_buckets = selected_buckets_by_rank(&selector)?;
    let selected_bucket_keys = selected_buckets
        .iter()
        .map(|bucket| bucket.bucket_key.clone())
        .collect::<BTreeSet<_>>();
    let decisions_by_bucket = decisions_by_bucket(&decision_log_path, &selected_bucket_keys)?;

    if let Some(parent) = request_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create portfolio billing request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let request_file = std::fs::File::create(&request_jsonl_path).map_err(|error| {
        format!(
            "failed to create portfolio billing request JSONL '{}': {error}",
            request_jsonl_path.display()
        )
    })?;
    let mut writer = BufWriter::new(request_file);

    let mut emitted_request_fingerprints = BTreeSet::<String>::new();
    let mut selected_decision_rows = 0usize;
    let mut skipped_before_calibration = 0usize;
    let mut skipped_exact_cache_hit = 0usize;
    let mut skipped_not_verified_safe = 0usize;
    let mut skipped_below_threshold = 0usize;
    let mut skipped_runtime_parity_mismatch = 0usize;
    let mut skipped_duplicate_portfolio_fingerprint = 0usize;
    let mut request_rows = 0usize;
    let mut request_rows_with_exact_cache_key = 0usize;
    let mut external_provider_correlation_key_rows = 0usize;
    let mut total_tokens_requiring_billing = 0usize;
    let mut current_known_cost_microusd = 0u64;

    for bucket in &selected_buckets {
        let Some(decisions) = decisions_by_bucket.get(&bucket.bucket_key) else {
            continue;
        };
        for (ordinal, decision) in decisions.iter().enumerate() {
            selected_decision_rows += 1;
            if ordinal < bucket.runtime_replay_start_event_ordinal {
                skipped_before_calibration += 1;
                continue;
            }
            if decision.reference_runtime_parity_mismatch {
                skipped_runtime_parity_mismatch += 1;
                continue;
            }
            if decision.margin_micro < bucket.threshold_micro {
                skipped_below_threshold += 1;
                continue;
            }
            if !decision.verified_safe_accept {
                skipped_not_verified_safe += 1;
                continue;
            }
            if decision.exact_cache_hit {
                skipped_exact_cache_hit += 1;
                continue;
            }
            if !emitted_request_fingerprints.insert(decision.request_fingerprint.clone()) {
                skipped_duplicate_portfolio_fingerprint += 1;
                continue;
            }

            let mut match_keys = vec![format!(
                "request_fingerprint:{}",
                decision.request_fingerprint
            )];
            if let Some(exact_cache_key) = decision
                .exact_cache_key
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                request_rows_with_exact_cache_key += 1;
                match_keys.push(format!("exact_cache_key:{exact_cache_key}"));
            }
            let provider_correlation_ready =
                !decision.external_provider_correlation_keys.is_empty();
            if provider_correlation_ready {
                external_provider_correlation_key_rows += 1;
                match_keys.extend(decision.external_provider_correlation_keys.iter().cloned());
            }
            match_keys.sort();
            match_keys.dedup();
            total_tokens_requiring_billing =
                total_tokens_requiring_billing.saturating_add(decision.total_tokens);
            current_known_cost_microusd =
                current_known_cost_microusd.saturating_add(decision.total_cost_microusd);
            request_rows += 1;

            let request = serde_json::json!({
                "schema_version": "phase_stream_online_miner_portfolio_billing_request_v1",
                "billing_request_id": format!(
                    "online-portfolio-cpu-accept-{}-{}",
                    decision.denominator_row_index,
                    bucket.rank
                ),
                "request_fingerprint": decision.request_fingerprint,
                "exact_cache_key": decision.exact_cache_key,
                "external_provider_correlation_keys": decision.external_provider_correlation_keys,
                "provider_correlation_ready": provider_correlation_ready,
                "match_keys": match_keys,
                "bucket_key": bucket.bucket_key,
                "task_name": bucket.task_name,
                "bucket_rank": bucket.rank,
                "package_fingerprint64": decision.package_fingerprint64,
                "denominator_row_index": decision.denominator_row_index,
                "margin_micro": decision.margin_micro,
                "threshold_micro": bucket.threshold_micro,
                "estimated_total_tokens": decision.total_tokens,
                "current_total_cost_microusd": decision.total_cost_microusd,
                "token_evidence_missing": decision.token_evidence_missing,
                "cost_evidence_missing": decision.cost_evidence_missing,
                "unique_cpu_accept_over_exact_cache": true,
                "verified_safe_accept": true,
                "false_accept": false,
                "local_accept_enabled": false,
                "market_money_claim_allowed": false,
                "boundary": "selected online-miner portfolio billing request only: asks external billing evidence to attach real provider costs; does not estimate missing money, compile, promote, serve, or enable local_accept"
            });
            serde_json::to_writer(&mut writer, &request).map_err(|error| {
                format!(
                    "failed to serialize portfolio billing request '{}': {error}",
                    request_jsonl_path.display()
                )
            })?;
            writer.write_all(b"\n").map_err(|error| {
                format!(
                    "failed to write portfolio billing request '{}': {error}",
                    request_jsonl_path.display()
                )
            })?;
        }
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush portfolio billing request '{}': {error}",
            request_jsonl_path.display()
        )
    })?;

    let runtime_accepts =
        json_usize(&runtime, &["portfolio_unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let runtime_tokens = json_usize(&runtime, &["portfolio_tokens_saved"]).unwrap_or(0);
    let accept_parity = request_rows == runtime_accepts;
    let token_parity = total_tokens_requiring_billing == runtime_tokens;
    let provider_correlation_parity =
        request_rows > 0 && external_provider_correlation_key_rows == request_rows;
    let request_ready = request_rows > 0 && accept_parity && token_parity;
    let ready_for_external_provider_evidence = request_ready && provider_correlation_parity;
    let verdict = if ready_for_external_provider_evidence {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_V1_READY_FOR_EXTERNAL_EVIDENCE"
    } else if request_ready {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_V1_WATCH_PROVIDER_CORRELATION_MISSING"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_V1_WATCH"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_portfolio_billing_request_v1",
        "runtime_replay_report_path": runtime_replay_report_path,
        "selector_report_path": selector_report_path,
        "decision_log_path": decision_log_path,
        "billing_request_jsonl_path": request_jsonl_path,
        "selected_bucket_count": selected_buckets.len(),
        "selected_decision_rows": selected_decision_rows,
        "billing_request_rows": request_rows,
        "request_rows_with_exact_cache_key": request_rows_with_exact_cache_key,
        "external_provider_correlation_key_rows": external_provider_correlation_key_rows,
        "external_provider_correlation_missing_rows": request_rows.saturating_sub(external_provider_correlation_key_rows),
        "provider_correlation_parity": provider_correlation_parity,
        "ready_for_external_provider_evidence": ready_for_external_provider_evidence,
        "runtime_portfolio_unique_cpu_accepts_over_exact_cache": runtime_accepts,
        "accept_parity": accept_parity,
        "total_tokens_requiring_billing": total_tokens_requiring_billing,
        "runtime_portfolio_tokens_saved": runtime_tokens,
        "token_parity": token_parity,
        "current_known_cost_microusd": current_known_cost_microusd,
        "skipped_before_calibration": skipped_before_calibration,
        "skipped_before_runtime_replay_start": skipped_before_calibration,
        "skipped_exact_cache_hit": skipped_exact_cache_hit,
        "skipped_not_verified_safe": skipped_not_verified_safe,
        "skipped_below_threshold": skipped_below_threshold,
        "skipped_runtime_parity_mismatch": skipped_runtime_parity_mismatch,
        "skipped_duplicate_portfolio_fingerprint": skipped_duplicate_portfolio_fingerprint,
        "billing_gate": {
            "provider_billing_request_only": true,
            "provider_billing_evidence_present": false,
            "market_money_claim_allowed": false,
            "policy": "request rows are exact keys for external provider billing evidence; this artifact is not evidence that money was saved"
        },
        "provider_correlation_gate": {
            "external_provider_correlation_key_rows": external_provider_correlation_key_rows,
            "external_provider_correlation_missing_rows": request_rows.saturating_sub(external_provider_correlation_key_rows),
            "provider_correlation_parity": provider_correlation_parity,
            "ready_for_external_provider_evidence": ready_for_external_provider_evidence,
            "policy": "request rows can be joined only if the external provider export carries request_fingerprint, exact_cache_key, or external provider correlation keys; internal match keys alone are not provider billing evidence"
        },
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
        "boundary": "billing request export only: selected automatic online-miner .nwpc portfolio accepts are converted to provider billing match keys; no serving, local_accept, promotion, money claim, lookup, or legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_billing_request_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  billing_request_jsonl_path: {}",
        request_jsonl_path.display()
    );
    println!("  billing_request_rows: {request_rows}");
    println!("  accept_parity: {accept_parity}");
    println!("  token_parity: {token_parity}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn selected_buckets_by_rank(selector: &Value) -> Result<Vec<BillingSelectedBucket>, String> {
    let mut buckets = Vec::new();
    for row in selector
        .get("selected_buckets")
        .and_then(Value::as_array)
        .ok_or_else(|| "selector report missing selected_buckets".to_owned())?
    {
        let bucket_key = json_string(row, &["bucket_key"])
            .ok_or_else(|| "selected bucket missing bucket_key".to_owned())?;
        buckets.push(BillingSelectedBucket {
            rank: json_usize(row, &["rank"]).unwrap_or(usize::MAX),
            bucket_key,
            task_name: json_string(row, &["task_name"]).unwrap_or_default(),
            threshold_micro: json_i64(row, &["threshold_micro"]).unwrap_or(1).max(1),
            runtime_replay_start_event_ordinal: json_usize(
                row,
                &["runtime_replay_start_event_ordinal"],
            )
            .unwrap_or_else(|| {
                let calibration_events = json_usize(row, &["calibration_events"]).unwrap_or(0);
                let policy_event_count = json_usize(row, &["policy_event_count"]).unwrap_or(0);
                calibration_events.saturating_add(policy_event_count)
            }),
        });
    }
    buckets.sort_by_key(|bucket| bucket.rank);
    Ok(buckets)
}

fn decisions_by_bucket(
    decision_log_path: &Path,
    selected_bucket_keys: &BTreeSet<String>,
) -> Result<BTreeMap<String, Vec<BillingDecision>>, String> {
    let text = std::fs::read_to_string(decision_log_path).map_err(|error| {
        format!(
            "failed to read online portfolio decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    let mut buckets = BTreeMap::<String, Vec<BillingDecision>>::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse online portfolio decision log '{}' line {}: {error}",
                decision_log_path.display(),
                line_index + 1
            )
        })?;
        if !json_bool(&row, &["future_only_shadow_scoring"]).unwrap_or(false) {
            continue;
        }
        let Some(bucket_key) = json_string(&row, &["bucket_key"]) else {
            continue;
        };
        if !selected_bucket_keys.contains(&bucket_key) {
            continue;
        }
        let token_cost = row.get("token_cost");
        let decision = BillingDecision {
            request_fingerprint: json_string(&row, &["request_fingerprint"])
                .unwrap_or_else(|| format!("decision-row:{}", line_index + 1)),
            exact_cache_key: json_string(&row, &["exact_cache_key"]),
            external_provider_correlation_keys: external_provider_correlation_keys(&row),
            exact_cache_hit: json_bool(&row, &["exact_cache_hit"]).unwrap_or(false),
            verified_safe_accept: json_bool(&row, &["verified_safe_accept"]).unwrap_or(false),
            margin_micro: json_i64(&row, &["margin_micro"]).unwrap_or(0),
            denominator_row_index: json_u64(&row, &["denominator_row_index"])
                .unwrap_or(line_index as u64 + 1),
            package_fingerprint64: json_u64(&row, &["package_fingerprint64"]).unwrap_or(0),
            total_tokens: token_cost
                .and_then(|value| json_usize(value, &["total_tokens"]))
                .unwrap_or(0),
            total_cost_microusd: token_cost
                .and_then(|value| json_u64(value, &["total_cost_microusd"]))
                .unwrap_or(0),
            token_evidence_missing: token_cost
                .and_then(|value| json_bool(value, &["token_evidence_missing"]))
                .unwrap_or(false),
            cost_evidence_missing: token_cost
                .and_then(|value| json_bool(value, &["cost_evidence_missing"]))
                .unwrap_or(false),
            reference_runtime_parity_mismatch: json_bool(
                &row,
                &["reference_runtime_parity_mismatch"],
            )
            .unwrap_or(false),
        };
        buckets.entry(bucket_key).or_default().push(decision);
    }
    Ok(buckets)
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

fn external_provider_correlation_keys(row: &Value) -> Vec<String> {
    let mut keys = super::phase_atom_external_provider_correlation_keys(row);
    keys.sort();
    keys.dedup();
    keys
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

fn json_i64(value: &Value, path: &[&str]) -> Option<i64> {
    json_at(value, path).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
    })
}
