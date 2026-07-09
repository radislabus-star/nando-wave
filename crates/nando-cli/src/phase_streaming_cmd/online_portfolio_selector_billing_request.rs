use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_SELECTOR_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-selector-billing-request-v1.report.json";
const DEFAULT_SELECTOR_BILLING_REQUEST_JSONL: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-selector-billing-request-v1.jsonl";

#[derive(Clone)]
struct SelectedBucket {
    rank: usize,
    bucket_key: String,
    task_name: String,
    threshold_micro: i64,
    runtime_replay_start_event_ordinal: usize,
}

#[derive(Clone)]
struct SelectorDecision {
    request_fingerprint: String,
    exact_cache_key: Option<String>,
    trace_id: Option<String>,
    external_provider_correlation_keys: Vec<String>,
    exact_cache_hit: bool,
    verified_safe_accept: bool,
    unique_cpu_accept_over_exact_cache: bool,
    false_accept: bool,
    wrong_win: bool,
    margin_micro: i64,
    denominator_row_index: u64,
    package_fingerprint64: u64,
    total_tokens: usize,
    total_cost_microusd: u64,
    token_evidence_missing: bool,
    cost_evidence_missing: bool,
    source_trace_path: Option<String>,
    source_line_index: Option<u64>,
}

pub(crate) fn run_phase_stream_online_miner_portfolio_selector_billing_request_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTOR_BILLING_REQUEST_REPORT));
    let request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTOR_BILLING_REQUEST_JSONL));
    let selector_report_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "selector report path is required".to_owned())?;

    let selector = read_json_value(&selector_report_path)?;
    let decision_log_path = PathBuf::from(
        json_string(&selector, &["decision_log_path"]).ok_or_else(|| {
            format!(
                "selector report '{}' missing decision_log_path",
                selector_report_path.display()
            )
        })?,
    );
    let selected_buckets = selected_buckets(&selector)?;
    let selected_bucket_keys = selected_buckets
        .iter()
        .map(|bucket| bucket.bucket_key.clone())
        .collect::<BTreeSet<_>>();
    let decisions_by_bucket = decisions_by_bucket(&decision_log_path, &selected_bucket_keys)?;

    if let Some(parent) = request_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create selector billing request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let request_file = std::fs::File::create(&request_jsonl_path).map_err(|error| {
        format!(
            "failed to create selector billing request JSONL '{}': {error}",
            request_jsonl_path.display()
        )
    })?;
    let mut writer = BufWriter::new(request_file);

    let mut selected_decision_rows = 0usize;
    let mut skipped_before_runtime_replay_start = 0usize;
    let mut skipped_exact_cache_hit = 0usize;
    let mut skipped_not_unique_accept = 0usize;
    let mut skipped_not_verified_safe = 0usize;
    let mut skipped_below_threshold = 0usize;
    let mut skipped_false_accept_or_wrong_win = 0usize;
    let mut skipped_duplicate_request_fingerprint = 0usize;
    let mut request_rows = 0usize;
    let mut request_rows_with_exact_cache_key = 0usize;
    let mut request_rows_with_provider_correlation = 0usize;
    let mut total_tokens_requiring_billing = 0usize;
    let mut current_known_cost_microusd = 0u64;
    let mut emitted_request_fingerprints = BTreeSet::<String>::new();

    for bucket in &selected_buckets {
        let Some(decisions) = decisions_by_bucket.get(&bucket.bucket_key) else {
            continue;
        };
        for (ordinal, decision) in decisions.iter().enumerate() {
            selected_decision_rows += 1;
            if ordinal < bucket.runtime_replay_start_event_ordinal {
                skipped_before_runtime_replay_start += 1;
                continue;
            }
            if decision.margin_micro < bucket.threshold_micro {
                skipped_below_threshold += 1;
                continue;
            }
            if decision.false_accept || decision.wrong_win {
                skipped_false_accept_or_wrong_win += 1;
                continue;
            }
            if !decision.verified_safe_accept {
                skipped_not_verified_safe += 1;
                continue;
            }
            if !decision.unique_cpu_accept_over_exact_cache {
                skipped_not_unique_accept += 1;
                continue;
            }
            if decision.exact_cache_hit {
                skipped_exact_cache_hit += 1;
                continue;
            }
            if !emitted_request_fingerprints.insert(decision.request_fingerprint.clone()) {
                skipped_duplicate_request_fingerprint += 1;
                continue;
            }

            let provider_correlation_ready =
                !decision.external_provider_correlation_keys.is_empty();
            if provider_correlation_ready {
                request_rows_with_provider_correlation += 1;
            }
            let mut match_keys = vec![format!(
                "request_fingerprint:{}",
                decision.request_fingerprint
            )];
            if let Some(value) = decision
                .exact_cache_key
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                request_rows_with_exact_cache_key += 1;
                match_keys.push(format!("exact_cache_key:{value}"));
            }
            if let Some(value) = decision
                .trace_id
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                match_keys.push(format!("trace_id:{value}"));
            }
            match_keys.extend(decision.external_provider_correlation_keys.iter().cloned());
            match_keys.sort();
            match_keys.dedup();

            request_rows += 1;
            total_tokens_requiring_billing =
                total_tokens_requiring_billing.saturating_add(decision.total_tokens);
            current_known_cost_microusd =
                current_known_cost_microusd.saturating_add(decision.total_cost_microusd);

            let request = serde_json::json!({
                "schema_version": "phase_stream_online_miner_portfolio_selector_billing_request_v1",
                "billing_request_id": format!(
                    "selector-shadow-cpu-accept-{}-{}",
                    decision.denominator_row_index,
                    bucket.rank
                ),
                "request_fingerprint": decision.request_fingerprint,
                "exact_cache_key": decision.exact_cache_key,
                "trace_id": decision.trace_id,
                "external_provider_correlation_keys": decision.external_provider_correlation_keys,
                "provider_correlation_ready": provider_correlation_ready,
                "match_keys": match_keys,
                "bucket_key": bucket.bucket_key,
                "task_name": bucket.task_name,
                "bucket_rank": bucket.rank,
                "package_fingerprint64": decision.package_fingerprint64,
                "denominator_row_index": decision.denominator_row_index,
                "source_trace_path": decision.source_trace_path,
                "source_line_index": decision.source_line_index,
                "margin_micro": decision.margin_micro,
                "threshold_micro": bucket.threshold_micro,
                "estimated_total_tokens": decision.total_tokens,
                "current_total_cost_microusd": decision.total_cost_microusd,
                "token_evidence_missing": decision.token_evidence_missing,
                "cost_evidence_missing": decision.cost_evidence_missing,
                "unique_cpu_accept_over_exact_cache": true,
                "verified_safe_accept": true,
                "false_accept": false,
                "wrong_win": false,
                "selector_shadow_only": true,
                "local_accept_enabled": false,
                "market_money_claim_allowed": false,
                "boundary": "selector shadow billing request only: selected automatic online-miner candidates are converted to exact external billing evidence keys; no runtime promotion, serving, local_accept, money estimate, or market claim"
            });
            serde_json::to_writer(&mut writer, &request).map_err(|error| {
                format!(
                    "failed to serialize selector billing request '{}': {error}",
                    request_jsonl_path.display()
                )
            })?;
            writer.write_all(b"\n").map_err(|error| {
                format!(
                    "failed to write selector billing request '{}': {error}",
                    request_jsonl_path.display()
                )
            })?;
        }
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush selector billing request '{}': {error}",
            request_jsonl_path.display()
        )
    })?;

    let request_file_bytes = std::fs::read(&request_jsonl_path).map_err(|error| {
        format!(
            "failed to read selector billing request '{}': {error}",
            request_jsonl_path.display()
        )
    })?;
    let request_file_fingerprint64 = fnv1a64(&request_file_bytes);
    let selector_accepts = json_usize(
        &selector,
        &["portfolio_unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or(0);
    let selector_tokens = json_usize(&selector, &["portfolio_tokens_saved"]).unwrap_or(0);
    let accept_parity = request_rows == selector_accepts;
    let token_parity = total_tokens_requiring_billing == selector_tokens;
    let provider_correlation_parity =
        request_rows > 0 && request_rows_with_provider_correlation == request_rows;
    let request_ready =
        request_rows > 0 && accept_parity && token_parity && provider_correlation_parity;
    let verdict = if request_ready {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_SELECTOR_BILLING_REQUEST_V1_READY_FOR_EXTERNAL_EVIDENCE"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_SELECTOR_BILLING_REQUEST_V1_WATCH"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_portfolio_selector_billing_request_v1",
        "selector_report_path": selector_report_path,
        "decision_log_path": decision_log_path,
        "billing_request_jsonl_path": request_jsonl_path,
        "request_file_fingerprint64": request_file_fingerprint64,
        "selector_shadow_only": true,
        "selected_bucket_count": selected_buckets.len(),
        "selected_decision_rows": selected_decision_rows,
        "billing_request_rows": request_rows,
        "request_rows_with_exact_cache_key": request_rows_with_exact_cache_key,
        "request_rows_with_provider_correlation": request_rows_with_provider_correlation,
        "request_rows_missing_provider_correlation": request_rows.saturating_sub(request_rows_with_provider_correlation),
        "selector_portfolio_unique_cpu_accepts_over_exact_cache": selector_accepts,
        "accept_parity": accept_parity,
        "total_tokens_requiring_billing": total_tokens_requiring_billing,
        "selector_portfolio_tokens_saved": selector_tokens,
        "token_parity": token_parity,
        "current_known_cost_microusd": current_known_cost_microusd,
        "skipped_before_runtime_replay_start": skipped_before_runtime_replay_start,
        "skipped_exact_cache_hit": skipped_exact_cache_hit,
        "skipped_not_unique_accept": skipped_not_unique_accept,
        "skipped_not_verified_safe": skipped_not_verified_safe,
        "skipped_below_threshold": skipped_below_threshold,
        "skipped_false_accept_or_wrong_win": skipped_false_accept_or_wrong_win,
        "skipped_duplicate_request_fingerprint": skipped_duplicate_request_fingerprint,
        "billing_gate": {
            "provider_billing_request_only": true,
            "provider_billing_evidence_present": false,
            "market_money_claim_allowed": false,
            "policy": "request rows are exact keys for external provider billing evidence; this selector-shadow artifact is not runtime promotion and not evidence that money was saved"
        },
        "required_next_gate": {
            "command": "phase-stream-online-miner-portfolio-billing-evidence-gate-v1",
            "needs_external_provider_billing_evidence": true,
            "request_file_fingerprint64": request_file_fingerprint64,
            "policy": "external evidence must include positive provider_cost_microusd, positive provider_total_tokens, provider, billing_source, unique billing_evidence_id, request_file_fingerprint64, and matching provider correlation key"
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
        "boundary": "billing request export only: selected online-miner shadow candidates are prepared for external provider billing evidence; no serving, promotion, local_accept, money estimate, lookup, or legacy nwrb"
    });
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_selector_billing_request_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  billing_request_jsonl_path: {}",
        request_jsonl_path.display()
    );
    println!("  billing_request_rows: {request_rows}");
    println!("  accept_parity: {accept_parity}");
    println!("  token_parity: {token_parity}");
    println!("  request_file_fingerprint64: {request_file_fingerprint64}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn selected_buckets(selector: &Value) -> Result<Vec<SelectedBucket>, String> {
    let mut buckets = Vec::new();
    let rows = selector
        .get("selected_buckets")
        .and_then(Value::as_array)
        .ok_or_else(|| "selector report missing selected_buckets".to_owned())?;
    for row in rows {
        let bucket_key = json_string(row, &["bucket_key"])
            .ok_or_else(|| "selected bucket missing bucket_key".to_owned())?;
        let calibration_events = json_usize(row, &["calibration_events"]).unwrap_or(0);
        let policy_event_count = json_usize(row, &["policy_event_count"]).unwrap_or(0);
        buckets.push(SelectedBucket {
            rank: json_usize(row, &["rank"]).unwrap_or(usize::MAX),
            bucket_key,
            task_name: json_string(row, &["task_name"]).unwrap_or_default(),
            threshold_micro: json_i64(row, &["threshold_micro"]).unwrap_or(1).max(1),
            runtime_replay_start_event_ordinal: json_usize(
                row,
                &["runtime_replay_start_event_ordinal"],
            )
            .unwrap_or_else(|| calibration_events.saturating_add(policy_event_count)),
        });
    }
    buckets.sort_by_key(|bucket| bucket.rank);
    Ok(buckets)
}

fn decisions_by_bucket(
    decision_log_path: &Path,
    selected_bucket_keys: &BTreeSet<String>,
) -> Result<BTreeMap<String, Vec<SelectorDecision>>, String> {
    let text = std::fs::read_to_string(decision_log_path).map_err(|error| {
        format!(
            "failed to read selector decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    let mut buckets = BTreeMap::<String, Vec<SelectorDecision>>::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse selector decision log '{}' line {}: {error}",
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
        let decision = SelectorDecision {
            request_fingerprint: json_string(&row, &["request_fingerprint"])
                .unwrap_or_else(|| format!("selector-decision-row:{}", line_index + 1)),
            exact_cache_key: json_string(&row, &["exact_cache_key"]),
            trace_id: json_string(&row, &["trace_id"]),
            external_provider_correlation_keys: external_provider_correlation_keys(&row),
            exact_cache_hit: json_bool(&row, &["exact_cache_hit"]).unwrap_or(false),
            verified_safe_accept: json_bool(&row, &["verified_safe_accept"]).unwrap_or(false),
            unique_cpu_accept_over_exact_cache: json_bool(
                &row,
                &["unique_cpu_accept_over_exact_cache"],
            )
            .unwrap_or(false),
            false_accept: json_bool(&row, &["false_accept"]).unwrap_or(false),
            wrong_win: json_bool(&row, &["wrong_win"]).unwrap_or(false),
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
            source_trace_path: json_string(&row, &["source_trace_path"]),
            source_line_index: json_u64(&row, &["source_line_index"]),
        };
        buckets.entry(bucket_key).or_default().push(decision);
    }
    Ok(buckets)
}

fn external_provider_correlation_keys(row: &Value) -> Vec<String> {
    let mut keys = super::phase_atom_external_provider_correlation_keys(row);
    keys.sort();
    keys.dedup();
    keys
}

fn read_json_value(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse JSON report '{}': {error}", path.display()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
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

fn json_i64(value: &Value, path: &[&str]) -> Option<i64> {
    json_at(value, path).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
    })
}
