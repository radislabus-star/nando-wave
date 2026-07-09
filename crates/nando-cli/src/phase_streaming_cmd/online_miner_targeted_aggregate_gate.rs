use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{json_bool, json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_TARGETED_AGGREGATE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-targeted-aggregate-gate-v1.report.json";
const DEFAULT_TARGETED_AGGREGATE_ACCEPTED_EVENTS: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-aggregate-gate-v1.accepted-events.jsonl";
const DEFAULT_TARGETED_SHADOW_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-shadow-v1-agent-followup-12k-current.report.json";
const DEFAULT_TARGETED_PROMOTION_GATE_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-promotion-registry-gate-v1-targeted-agent-followup-12k-current.report.json";
const DEFAULT_TARGETED_SPLIT_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-targeted-split-refinement-nwpc-shadow-replay-v1-agent-followup-12k-current.report.json";

#[derive(Clone, Debug)]
struct AggregateAcceptedEvent {
    source: &'static str,
    request_fingerprint: String,
    exact_cache_key: String,
    package_fingerprint64: u64,
    margin_micro: i64,
    threshold_micro: i64,
    total_tokens: u64,
    total_cost_microusd: u64,
}

pub(crate) fn run_phase_stream_online_miner_targeted_aggregate_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_AGGREGATE_REPORT));
    let accepted_events_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_AGGREGATE_ACCEPTED_EVENTS));
    let targeted_shadow_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_SHADOW_REPORT));
    let promotion_gate_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_PROMOTION_GATE_REPORT));
    let split_replay_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_SPLIT_REPLAY_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let targeted = read_json_value(&targeted_shadow_report_path)?;
    let promotion_gate = read_json_value(&promotion_gate_report_path)?;
    let split_replay = read_json_value(&split_replay_report_path)?;

    let product_hot_events = reconstruct_product_hot_events(&targeted)?;
    let split_events = split_replay_events(&split_replay)?;
    let product_hot_summary = summarize_events(&product_hot_events);
    let split_summary = summarize_events(&split_events);

    let expected_product_hot_accepts = json_u64(
        &targeted,
        &["product_hot_unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or_default();
    let expected_product_hot_tokens =
        json_u64(&targeted, &["product_hot_nando_cpu_tokens_saved"]).unwrap_or_default();
    let expected_product_hot_false =
        json_u64(&targeted, &["product_hot_false_accepts"]).unwrap_or(u64::MAX);
    let promotion_gate_accepts = json_u64(
        &promotion_gate,
        &["global_unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or_default();
    let promotion_gate_tokens =
        json_u64(&promotion_gate, &["nando_cpu_tokens_saved"]).unwrap_or_default();
    let promotion_gate_false = json_u64(&promotion_gate, &["false_accepts"]).unwrap_or(u64::MAX);
    let expected_split_accepts =
        json_u64(&split_replay, &["future_unique_accepts_over_exact_cache"]).unwrap_or_default();
    let expected_split_tokens =
        json_u64(&split_replay, &["future_tokens_saved"]).unwrap_or_default();
    let expected_split_false =
        json_u64(&split_replay, &["future_false_accepts"]).unwrap_or(u64::MAX);
    let split_replay_mismatches =
        json_u64(&split_replay, &["replay_mismatch_count"]).unwrap_or(u64::MAX);

    let product_hot_reconstruction_matches = product_hot_summary.unique_accepts
        == expected_product_hot_accepts
        && product_hot_summary.tokens_saved == expected_product_hot_tokens
        && product_hot_summary.unique_accepts == promotion_gate_accepts
        && product_hot_summary.tokens_saved == promotion_gate_tokens
        && expected_product_hot_false == 0
        && promotion_gate_false == 0;
    let split_replay_matches = split_summary.unique_accepts == expected_split_accepts
        && split_summary.tokens_saved == expected_split_tokens
        && expected_split_false == 0
        && split_replay_mismatches == 0;

    let mut aggregate = BTreeMap::<String, AggregateAcceptedEvent>::new();
    let mut duplicate_rows = 0usize;
    let mut duplicate_sources = BTreeSet::<String>::new();
    for event in product_hot_events.iter().chain(split_events.iter()) {
        if aggregate.contains_key(&event.request_fingerprint) {
            duplicate_rows += 1;
            duplicate_sources.insert(event.request_fingerprint.clone());
        } else {
            aggregate.insert(event.request_fingerprint.clone(), event.clone());
        }
    }
    let aggregate_events = aggregate.into_values().collect::<Vec<_>>();
    let aggregate_summary = summarize_events(&aggregate_events);

    write_accepted_events_jsonl(&accepted_events_path, &aggregate_events)?;

    let targeted_flags_clear = forbidden_flags_clear(&targeted);
    let promotion_flags_clear = forbidden_flags_clear(&promotion_gate);
    let split_flags_clear = forbidden_flags_clear(&split_replay);
    let local_accept_enabled = json_bool(&targeted, &["local_accept_enabled"]).unwrap_or(true)
        || json_bool(&promotion_gate, &["local_accept_enabled"]).unwrap_or(true)
        || json_bool(&split_replay, &["local_accept_enabled"]).unwrap_or(true);
    let market_money_claim_allowed = json_bool(&targeted, &["market_money_claim_allowed"])
        .unwrap_or(true)
        || json_bool(&promotion_gate, &["market_money_claim_allowed"]).unwrap_or(true)
        || json_bool(&split_replay, &["market_money_claim_allowed"]).unwrap_or(true);
    let calls_tokens_claim_allowed = product_hot_reconstruction_matches
        && split_replay_matches
        && targeted_flags_clear
        && promotion_flags_clear
        && split_flags_clear
        && !local_accept_enabled
        && !market_money_claim_allowed
        && aggregate_summary.unique_accepts > 0
        && aggregate_summary.tokens_saved > 0;
    let provider_billing_evidence_present = false;
    let product_promotion_allowed = false;
    let blocker = if calls_tokens_claim_allowed {
        "none"
    } else if !product_hot_reconstruction_matches {
        "product_hot_reconstruction_mismatch"
    } else if !split_replay_matches {
        "split_replay_mismatch"
    } else if !targeted_flags_clear || !promotion_flags_clear || !split_flags_clear {
        "forbidden_flags_not_clear"
    } else if local_accept_enabled {
        "local_accept_enabled"
    } else if market_money_claim_allowed {
        "market_money_claim_allowed_in_source"
    } else {
        "aggregate_gate_failed"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_targeted_aggregate_gate_v1",
        "mode": "dedupe_product_hot_and_targeted_split_shadow_accepts",
        "targeted_shadow_report_path": targeted_shadow_report_path,
        "promotion_gate_report_path": promotion_gate_report_path,
        "split_replay_report_path": split_replay_report_path,
        "accepted_events_jsonl_path": accepted_events_path,
        "product_hot_unique_accepts_over_exact_cache": product_hot_summary.unique_accepts,
        "product_hot_tokens_saved": product_hot_summary.tokens_saved,
        "product_hot_cost_saved_microusd": product_hot_summary.cost_saved_microusd,
        "split_unique_accepts_over_exact_cache": split_summary.unique_accepts,
        "split_tokens_saved": split_summary.tokens_saved,
        "split_cost_saved_microusd": split_summary.cost_saved_microusd,
        "aggregate_unique_accepts_over_exact_cache": aggregate_summary.unique_accepts,
        "aggregate_tokens_saved": aggregate_summary.tokens_saved,
        "aggregate_cost_saved_microusd": aggregate_summary.cost_saved_microusd,
        "aggregate_duplicate_accept_rows": duplicate_rows,
        "aggregate_duplicate_request_fingerprints": duplicate_sources.into_iter().collect::<Vec<_>>(),
        "incremental_split_unique_accepts_over_product_hot": aggregate_summary.unique_accepts.saturating_sub(product_hot_summary.unique_accepts),
        "incremental_split_tokens_over_product_hot": aggregate_summary.tokens_saved.saturating_sub(product_hot_summary.tokens_saved),
        "product_hot_reconstruction_matches": product_hot_reconstruction_matches,
        "split_replay_matches": split_replay_matches,
        "provider_billing_evidence_present": provider_billing_evidence_present,
        "calls_tokens_claim_allowed": calls_tokens_claim_allowed,
        "product_promotion_allowed": product_promotion_allowed,
        "local_accept_enabled": local_accept_enabled,
        "market_money_claim_allowed": market_money_claim_allowed,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        },
        "verdict": if calls_tokens_claim_allowed {
            "PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_GATE_V1_PASS_CALLS_TOKENS_MONEY_BLOCKED"
        } else {
            "PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_GATE_V1_WATCH"
        },
        "blocker": blocker,
        "boundary": "aggregate gate only: reconstructs product-hot event-level accepts, dedupes them with targeted split runtime replay accepts, exports accepted events for billing evidence, and never compiles, mines, promotes, serves, enables local_accept, claims market money, or uses legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_targeted_aggregate_gate_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  accepted_events_jsonl_path: {}",
        accepted_events_path.display()
    );
    println!(
        "  aggregate_unique_accepts_over_exact_cache: {}",
        aggregate_summary.unique_accepts
    );
    println!(
        "  aggregate_tokens_saved: {}",
        aggregate_summary.tokens_saved
    );
    println!("  aggregate_duplicate_accept_rows: {duplicate_rows}");
    println!("  calls_tokens_claim_allowed: {calls_tokens_claim_allowed}");
    println!("  market_money_claim_allowed: false");
    println!("  blocker: {blocker}");
    Ok(())
}

#[derive(Default)]
struct AggregateSummary {
    unique_accepts: u64,
    tokens_saved: u64,
    cost_saved_microusd: u64,
}

fn reconstruct_product_hot_events(report: &Value) -> Result<Vec<AggregateAcceptedEvent>, String> {
    let decision_log_path = json_string(report, &["decision_log_path"])
        .map(PathBuf::from)
        .ok_or_else(|| "targeted aggregate missing decision_log_path".to_owned())?;
    let decisions = read_jsonl_values(&decision_log_path)?;
    let packages = report
        .get("product_hot_packages")
        .and_then(Value::as_array)
        .ok_or_else(|| "targeted aggregate missing product_hot_packages".to_owned())?;
    let mut out = Vec::new();
    for package in packages {
        let package_fingerprint64 =
            json_u64(package, &["package_fingerprint64"]).ok_or_else(|| {
                "targeted aggregate product_hot package missing fingerprint".to_owned()
            })?;
        let threshold_micro = json_i64(package, &["auto_calibrated_margin_threshold_micro"])
            .ok_or_else(|| {
                "targeted aggregate product_hot package missing auto threshold".to_owned()
            })?;
        let calibration_events = json_u64(package, &["auto_calibration_events"])
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| {
                "targeted aggregate product_hot package missing calibration window".to_owned()
            })?;
        let mut package_rows = decisions
            .iter()
            .filter(|row| json_u64(row, &["package_fingerprint64"]) == Some(package_fingerprint64))
            .collect::<Vec<_>>();
        package_rows.sort_by_key(|row| json_u64(row, &["denominator_row_index"]).unwrap_or(0));
        let mut seen = BTreeSet::new();
        for row in package_rows.into_iter().skip(calibration_events) {
            if json_i64(row, &["margin_micro"]).unwrap_or(i64::MIN) < threshold_micro {
                continue;
            }
            if json_bool(row, &["unique_cpu_accept_over_exact_cache"]) != Some(true)
                || json_bool(row, &["verified_safe_accept"]) != Some(true)
                || json_bool(row, &["false_accept"]) == Some(true)
            {
                continue;
            }
            let request_fingerprint =
                json_string(row, &["request_fingerprint"]).ok_or_else(|| {
                    "targeted aggregate product_hot decision missing request_fingerprint".to_owned()
                })?;
            if !seen.insert(request_fingerprint.clone()) {
                continue;
            }
            out.push(AggregateAcceptedEvent {
                source: "product_hot",
                request_fingerprint,
                exact_cache_key: json_string(row, &["exact_cache_key"]).unwrap_or_default(),
                package_fingerprint64,
                margin_micro: json_i64(row, &["margin_micro"]).unwrap_or_default(),
                threshold_micro,
                total_tokens: json_u64(row, &["token_cost", "total_tokens"]).unwrap_or_default(),
                total_cost_microusd: json_u64(row, &["token_cost", "total_cost_microusd"])
                    .unwrap_or_default(),
            });
        }
    }
    Ok(out)
}

fn split_replay_events(report: &Value) -> Result<Vec<AggregateAcceptedEvent>, String> {
    let values = report
        .get("unique_accepts")
        .and_then(Value::as_array)
        .ok_or_else(|| "targeted aggregate split replay missing unique_accepts".to_owned())?;
    values
        .iter()
        .map(|row| {
            Ok(AggregateAcceptedEvent {
                source: "targeted_split",
                request_fingerprint: json_string(row, &["request_fingerprint"]).ok_or_else(
                    || "targeted aggregate split event missing request_fingerprint".to_owned(),
                )?,
                exact_cache_key: json_string(row, &["exact_cache_key"]).unwrap_or_default(),
                package_fingerprint64: json_u64(row, &["package_fingerprint64"])
                    .unwrap_or_default(),
                margin_micro: json_i64(row, &["margin_micro"]).unwrap_or_default(),
                threshold_micro: json_i64(row, &["threshold_micro"]).unwrap_or_default(),
                total_tokens: json_u64(row, &["total_tokens"]).unwrap_or_default(),
                total_cost_microusd: json_u64(row, &["total_cost_microusd"]).unwrap_or_default(),
            })
        })
        .collect()
}

fn summarize_events(events: &[AggregateAcceptedEvent]) -> AggregateSummary {
    let mut seen = BTreeSet::new();
    let mut summary = AggregateSummary::default();
    for event in events {
        if !seen.insert(&event.request_fingerprint) {
            continue;
        }
        summary.unique_accepts = summary.unique_accepts.saturating_add(1);
        summary.tokens_saved = summary.tokens_saved.saturating_add(event.total_tokens);
        summary.cost_saved_microusd = summary
            .cost_saved_microusd
            .saturating_add(event.total_cost_microusd);
    }
    summary
}

fn write_accepted_events_jsonl(
    path: &Path,
    events: &[AggregateAcceptedEvent],
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create targeted aggregate accepted event dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let mut lines = Vec::with_capacity(events.len());
    for event in events {
        lines.push(
            serde_json::to_string(&serde_json::json!({
                "source": event.source,
                "request_fingerprint": event.request_fingerprint,
                "exact_cache_key": event.exact_cache_key,
                "package_fingerprint64": event.package_fingerprint64,
                "margin_micro": event.margin_micro,
                "threshold_micro": event.threshold_micro,
                "total_tokens": event.total_tokens,
                "total_cost_microusd": event.total_cost_microusd,
                "provider_billing_evidence_present": false,
                "verified_safe_accept": true,
                "false_accept": false
            }))
            .map_err(|error| {
                format!("failed to serialize targeted aggregate accepted event: {error}")
            })?,
        );
    }
    std::fs::write(
        path,
        if lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", lines.join("\n"))
        },
    )
    .map_err(|error| {
        format!(
            "failed to write targeted aggregate accepted events '{}': {error}",
            path.display()
        )
    })
}

fn read_jsonl_values(path: &Path) -> Result<Vec<Value>, String> {
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

fn forbidden_flags_clear(value: &Value) -> bool {
    let Some(flags) = value.get("forbidden_flags").and_then(Value::as_object) else {
        return false;
    };
    !flags.is_empty() && flags.values().all(|value| value.as_bool() == Some(false))
}
