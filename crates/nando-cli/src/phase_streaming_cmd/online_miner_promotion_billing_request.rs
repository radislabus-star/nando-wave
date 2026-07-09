use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{json_bool, json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_ONLINE_MINER_PROMOTION_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-promotion-billing-request-v1.report.json";
const DEFAULT_ONLINE_MINER_PROMOTION_BILLING_REQUEST_JSONL: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-promotion-billing-request-v1.jsonl";
const DEFAULT_ONLINE_MINER_TARGETED_BILLING_REQUEST_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-targeted-billing-request-v1.report.json";
const DEFAULT_ONLINE_MINER_TARGETED_BILLING_REQUEST_JSONL: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-targeted-billing-request-v1.jsonl";
const DEFAULT_ONLINE_MINER_TARGETED_SHADOW_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-shadow-v1-agent-followup-12k-current.report.json";
const DEFAULT_ONLINE_MINER_TARGETED_SHADOW_DECISION_LOG: &str = "target/nando-wave/streaming/online-miner-targeted-shadow-v1-agent-followup-12k-current/targeted-shadow.decisions.jsonl";
const DEFAULT_ONLINE_MINER_PROMOTION_REGISTRY_GATE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-promotion-registry-gate-v1.report.json";
const DEFAULT_ONLINE_MINER_DAEMON_DECISION_LOG: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-daemon-v1.decisions.jsonl";
const DEFAULT_ONLINE_MINER_PROMOTION_PROVIDER_CAPTURE_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-promotion-provider-capture-request-v1.report.json";
const DEFAULT_ONLINE_MINER_PROMOTION_PROVIDER_CAPTURE_REQUEST_JSONL: &str = "target/nando-wave/streaming/phase-stream-online-miner-promotion-provider-capture-request-v1.jsonl";
const DEFAULT_ONLINE_MINER_TARGETED_ADMISSION_GATE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-targeted-admission-gate-v1.report.json";

#[derive(Clone, Debug)]
struct PromotionBillingCandidate {
    bucket_key: String,
    task_name: String,
    package_fingerprint64: u64,
    threshold_micro: i64,
    calibration_events: usize,
}

#[derive(Clone, Debug)]
struct PromotionBillingDecision {
    bucket_key: String,
    request_fingerprint: String,
    exact_cache_key: Option<String>,
    external_provider_correlation_keys: Vec<String>,
    exact_cache_hit: bool,
    verified_safe_accept: bool,
    false_accept: bool,
    margin_micro: i64,
    denominator_row_index: u64,
    package_fingerprint64: u64,
    total_tokens: usize,
    total_cost_microusd: u64,
    token_evidence_missing: bool,
    cost_evidence_missing: bool,
    reference_runtime_parity_mismatch: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct TargetedThresholdPolicyAudit {
    clean: bool,
    package_count: usize,
    auto_calibrated_package_count: usize,
    manual_or_missing_threshold_package_count: usize,
    packages_with_calibration_window: usize,
    packages_with_shadow_after_calibration: usize,
    packages_with_threshold_equal_safe_accept: usize,
    min_auto_calibration_events: usize,
    min_shadow_events_after_calibration: usize,
}

pub(crate) fn run_phase_stream_online_miner_promotion_billing_request_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PROMOTION_BILLING_REQUEST_REPORT));
    let request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PROMOTION_BILLING_REQUEST_JSONL));
    let promotion_gate_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PROMOTION_REGISTRY_GATE_REPORT));
    let decision_log_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_DAEMON_DECISION_LOG));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let promotion_gate = read_json_value(&promotion_gate_report_path)?;
    let source_registry_path = PathBuf::from(
        json_string(&promotion_gate, &["source_registry_path"]).ok_or_else(|| {
            format!(
                "promotion gate report '{}' missing source_registry_path",
                promotion_gate_report_path.display()
            )
        })?,
    );
    let source_registry = read_json_value(&source_registry_path)?;
    let candidates = promotion_billing_candidates(&promotion_gate, &source_registry)?;
    let candidate_keys = candidates
        .iter()
        .map(|candidate| candidate.bucket_key.clone())
        .collect::<BTreeSet<_>>();
    let decisions_by_bucket = promotion_decisions_by_bucket(&decision_log_path, &candidate_keys)?;

    if let Some(parent) = request_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create promotion billing request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let file = std::fs::File::create(&request_jsonl_path).map_err(|error| {
        format!(
            "failed to create promotion billing request JSONL '{}': {error}",
            request_jsonl_path.display()
        )
    })?;
    let mut writer = BufWriter::new(file);

    let mut emitted_request_fingerprints = BTreeSet::<String>::new();
    let mut selected_decision_rows = 0usize;
    let mut skipped_calibration_rows = 0usize;
    let mut skipped_below_threshold = 0usize;
    let mut skipped_not_verified_safe = 0usize;
    let mut skipped_exact_cache_hit = 0usize;
    let mut skipped_runtime_parity_mismatch = 0usize;
    let mut skipped_duplicate_request_fingerprint = 0usize;
    let mut false_accept_rows = 0usize;
    let mut request_rows = 0usize;
    let mut request_rows_with_exact_cache_key = 0usize;
    let mut external_provider_correlation_key_rows = 0usize;
    let mut provider_correlation_ready_rows = 0usize;
    let mut total_tokens_requiring_billing = 0usize;
    let mut current_known_cost_microusd = 0u64;

    for candidate in &candidates {
        let Some(decisions) = decisions_by_bucket.get(&candidate.bucket_key) else {
            continue;
        };
        for (ordinal, decision) in decisions.iter().enumerate() {
            selected_decision_rows += 1;
            if ordinal < candidate.calibration_events {
                skipped_calibration_rows += 1;
                continue;
            }
            if decision.reference_runtime_parity_mismatch {
                skipped_runtime_parity_mismatch += 1;
                continue;
            }
            if decision.margin_micro < candidate.threshold_micro {
                skipped_below_threshold += 1;
                continue;
            }
            if !decision.verified_safe_accept {
                false_accept_rows += 1;
                skipped_not_verified_safe += 1;
                continue;
            }
            if decision.false_accept {
                false_accept_rows += 1;
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
            if !decision.external_provider_correlation_keys.is_empty() {
                external_provider_correlation_key_rows += 1;
                match_keys.extend(decision.external_provider_correlation_keys.iter().cloned());
            }
            match_keys.sort();
            match_keys.dedup();
            let provider_correlation_ready = !match_keys.is_empty();
            provider_correlation_ready_rows += usize::from(provider_correlation_ready);
            request_rows += 1;
            total_tokens_requiring_billing =
                total_tokens_requiring_billing.saturating_add(decision.total_tokens);
            current_known_cost_microusd =
                current_known_cost_microusd.saturating_add(decision.total_cost_microusd);

            let request = serde_json::json!({
                "schema_version": "phase_stream_online_miner_promotion_billing_request_v1",
                "billing_request_id": format!(
                    "online-miner-promotion-cpu-accept-{}-{}",
                    decision.denominator_row_index,
                    request_rows
                ),
                "request_fingerprint": decision.request_fingerprint,
                "exact_cache_key": decision.exact_cache_key,
                "external_provider_correlation_keys": decision.external_provider_correlation_keys,
                "provider_correlation_ready": provider_correlation_ready,
                "match_keys": match_keys,
                "bucket_key": decision.bucket_key,
                "task_name": candidate.task_name,
                "package_fingerprint64": candidate.package_fingerprint64,
                "decision_package_fingerprint64": decision.package_fingerprint64,
                "denominator_row_index": decision.denominator_row_index,
                "margin_micro": decision.margin_micro,
                "threshold_micro": candidate.threshold_micro,
                "estimated_total_tokens": decision.total_tokens,
                "current_total_cost_microusd": decision.total_cost_microusd,
                "token_evidence_missing": decision.token_evidence_missing,
                "cost_evidence_missing": decision.cost_evidence_missing,
                "token_cost_estimate_used": true,
                "provider_billing_evidence_present": false,
                "unique_cpu_accept_over_exact_cache": true,
                "verified_safe_accept": true,
                "false_accept": false,
                "local_accept_enabled": false,
                "market_money_claim_allowed": false,
                "boundary": "online-miner promotion billing request only: exports accepted shadow rows for external provider billing evidence; does not compile, promote to serving, enable local_accept, estimate missing money, or claim market savings"
            });
            serde_json::to_writer(&mut writer, &request).map_err(|error| {
                format!(
                    "failed to serialize promotion billing request '{}': {error}",
                    request_jsonl_path.display()
                )
            })?;
            writer.write_all(b"\n").map_err(|error| {
                format!(
                    "failed to write promotion billing request '{}': {error}",
                    request_jsonl_path.display()
                )
            })?;
        }
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush promotion billing request '{}': {error}",
            request_jsonl_path.display()
        )
    })?;

    let registry_unique = json_usize(
        &source_registry,
        &["global_unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or(0);
    let registry_tokens = json_usize(&source_registry, &["nando_cpu_tokens_saved"]).unwrap_or(0);
    let registry_cost = json_u64(&source_registry, &["nando_cpu_cost_saved_microusd"]).unwrap_or(0);
    let gate_unique = json_usize(
        &promotion_gate,
        &["global_unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or(0);
    let gate_false_accepts = json_usize(&promotion_gate, &["false_accepts"]).unwrap_or(usize::MAX);
    let accept_parity = request_rows == registry_unique && request_rows == gate_unique;
    let token_parity = total_tokens_requiring_billing == registry_tokens;
    let cost_parity = current_known_cost_microusd == registry_cost;
    let request_ready = request_rows > 0
        && accept_parity
        && token_parity
        && cost_parity
        && false_accept_rows == 0
        && gate_false_accepts == 0
        && provider_correlation_ready_rows == request_rows
        && json_bool(&promotion_gate, &["local_accept_enabled"]) == Some(false)
        && json_bool(&promotion_gate, &["market_money_claim_allowed"]) == Some(false);
    let verdict = if request_ready {
        "PHASE_STREAM_ONLINE_MINER_PROMOTION_BILLING_REQUEST_V1_READY_FOR_PROVIDER_EVIDENCE"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PROMOTION_BILLING_REQUEST_V1_WATCH"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_promotion_billing_request_v1",
        "promotion_gate_report_path": promotion_gate_report_path,
        "source_registry_path": source_registry_path,
        "decision_log_path": decision_log_path,
        "billing_request_jsonl_path": request_jsonl_path,
        "promotion_candidate_count": candidates.len(),
        "selected_decision_rows": selected_decision_rows,
        "billing_request_rows": request_rows,
        "request_rows_with_exact_cache_key": request_rows_with_exact_cache_key,
        "external_provider_correlation_key_rows": external_provider_correlation_key_rows,
        "provider_correlation_ready_rows": provider_correlation_ready_rows,
        "provider_correlation_missing_rows": request_rows.saturating_sub(provider_correlation_ready_rows),
        "registry_unique_cpu_accepts_over_exact_cache": registry_unique,
        "gate_unique_cpu_accepts_over_exact_cache": gate_unique,
        "accept_parity": accept_parity,
        "total_tokens_requiring_billing": total_tokens_requiring_billing,
        "registry_tokens_saved": registry_tokens,
        "token_parity": token_parity,
        "current_known_cost_microusd": current_known_cost_microusd,
        "registry_cost_microusd": registry_cost,
        "cost_parity": cost_parity,
        "token_cost_estimate_used": true,
        "provider_billing_evidence_present": false,
        "ready_for_external_provider_evidence": request_ready,
        "skipped_calibration_rows": skipped_calibration_rows,
        "skipped_below_threshold": skipped_below_threshold,
        "skipped_not_verified_safe": skipped_not_verified_safe,
        "skipped_exact_cache_hit": skipped_exact_cache_hit,
        "skipped_runtime_parity_mismatch": skipped_runtime_parity_mismatch,
        "skipped_duplicate_request_fingerprint": skipped_duplicate_request_fingerprint,
        "false_accept_rows": false_accept_rows,
        "billing_gate": {
            "provider_billing_request_only": true,
            "provider_billing_evidence_present": false,
            "market_money_claim_allowed": false,
            "policy": "request rows identify shadow accepted calls that need external provider billing evidence; internal cost fields are not market money evidence"
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
        "boundary": "billing request export only: compact online-miner promotion rows are converted to provider billing match keys; no serving, local_accept, auto_promote, money claim, lookup, or legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_promotion_billing_request_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  billing_request_jsonl_path: {}",
        request_jsonl_path.display()
    );
    println!("  billing_request_rows: {request_rows}");
    println!("  accept_parity: {accept_parity}");
    println!("  token_parity: {token_parity}");
    println!("  cost_parity: {cost_parity}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

pub(crate) fn run_phase_stream_online_miner_targeted_billing_request_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_TARGETED_BILLING_REQUEST_REPORT));
    let request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_TARGETED_BILLING_REQUEST_JSONL));
    let targeted_shadow_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_TARGETED_SHADOW_REPORT));
    let decision_log_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_TARGETED_SHADOW_DECISION_LOG));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let targeted = read_json_value(&targeted_shadow_report_path)?;
    let candidates = targeted_billing_candidates(&targeted)?;
    let candidate_keys = candidates
        .iter()
        .map(|candidate| candidate.bucket_key.clone())
        .collect::<BTreeSet<_>>();
    let decisions_by_bucket = promotion_decisions_by_bucket(&decision_log_path, &candidate_keys)?;

    if let Some(parent) = request_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create targeted billing request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let file = std::fs::File::create(&request_jsonl_path).map_err(|error| {
        format!(
            "failed to create targeted billing request JSONL '{}': {error}",
            request_jsonl_path.display()
        )
    })?;
    let mut writer = BufWriter::new(file);

    let mut emitted_request_fingerprints = BTreeSet::<String>::new();
    let mut selected_decision_rows = 0usize;
    let mut skipped_calibration_rows = 0usize;
    let mut skipped_below_threshold = 0usize;
    let mut skipped_not_verified_safe = 0usize;
    let mut skipped_exact_cache_hit = 0usize;
    let mut skipped_runtime_parity_mismatch = 0usize;
    let mut skipped_duplicate_request_fingerprint = 0usize;
    let mut false_accept_rows = 0usize;
    let mut request_rows = 0usize;
    let mut request_rows_with_exact_cache_key = 0usize;
    let mut external_provider_correlation_key_rows = 0usize;
    let mut provider_correlation_ready_rows = 0usize;
    let mut total_tokens_requiring_billing = 0usize;
    let mut current_known_cost_microusd = 0u64;

    for candidate in &candidates {
        let Some(decisions) = decisions_by_bucket.get(&candidate.bucket_key) else {
            continue;
        };
        for (ordinal, decision) in decisions.iter().enumerate() {
            selected_decision_rows += 1;
            if ordinal < candidate.calibration_events {
                skipped_calibration_rows += 1;
                continue;
            }
            if decision.reference_runtime_parity_mismatch {
                skipped_runtime_parity_mismatch += 1;
                continue;
            }
            if decision.margin_micro < candidate.threshold_micro {
                skipped_below_threshold += 1;
                continue;
            }
            if !decision.verified_safe_accept {
                false_accept_rows += 1;
                skipped_not_verified_safe += 1;
                continue;
            }
            if decision.false_accept {
                false_accept_rows += 1;
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
            if !decision.external_provider_correlation_keys.is_empty() {
                external_provider_correlation_key_rows += 1;
                match_keys.extend(decision.external_provider_correlation_keys.iter().cloned());
            }
            match_keys.sort();
            match_keys.dedup();
            let provider_correlation_ready = !match_keys.is_empty();
            provider_correlation_ready_rows += usize::from(provider_correlation_ready);
            request_rows += 1;
            total_tokens_requiring_billing =
                total_tokens_requiring_billing.saturating_add(decision.total_tokens);
            current_known_cost_microusd =
                current_known_cost_microusd.saturating_add(decision.total_cost_microusd);

            let request = serde_json::json!({
                "schema_version": "phase_stream_online_miner_targeted_billing_request_v1",
                "billing_request_id": format!(
                    "online-miner-targeted-cpu-accept-{}-{}",
                    decision.denominator_row_index,
                    request_rows
                ),
                "request_fingerprint": decision.request_fingerprint,
                "exact_cache_key": decision.exact_cache_key,
                "external_provider_correlation_keys": decision.external_provider_correlation_keys,
                "provider_correlation_ready": provider_correlation_ready,
                "match_keys": match_keys,
                "bucket_key": decision.bucket_key,
                "task_name": candidate.task_name,
                "package_fingerprint64": candidate.package_fingerprint64,
                "decision_package_fingerprint64": decision.package_fingerprint64,
                "denominator_row_index": decision.denominator_row_index,
                "margin_micro": decision.margin_micro,
                "threshold_micro": candidate.threshold_micro,
                "estimated_total_tokens": decision.total_tokens,
                "current_total_cost_microusd": decision.total_cost_microusd,
                "token_evidence_missing": decision.token_evidence_missing,
                "cost_evidence_missing": decision.cost_evidence_missing,
                "token_cost_estimate_used": true,
                "provider_billing_evidence_present": false,
                "unique_cpu_accept_over_exact_cache": true,
                "verified_safe_accept": true,
                "false_accept": false,
                "local_accept_enabled": false,
                "market_money_claim_allowed": false,
                "boundary": "online-miner targeted billing request only: exports clean product-hot shadow rows for external provider billing evidence; does not compile, serve, promote, enable local_accept, estimate missing money, or claim market savings"
            });
            serde_json::to_writer(&mut writer, &request).map_err(|error| {
                format!(
                    "failed to serialize targeted billing request '{}': {error}",
                    request_jsonl_path.display()
                )
            })?;
            writer.write_all(b"\n").map_err(|error| {
                format!(
                    "failed to write targeted billing request '{}': {error}",
                    request_jsonl_path.display()
                )
            })?;
        }
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush targeted billing request '{}': {error}",
            request_jsonl_path.display()
        )
    })?;

    let expected_unique = json_usize(
        &targeted,
        &["product_hot_unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or(0);
    let expected_tokens =
        json_usize(&targeted, &["product_hot_nando_cpu_tokens_saved"]).unwrap_or(0);
    let expected_cost =
        json_u64(&targeted, &["product_hot_nando_cpu_cost_saved_microusd"]).unwrap_or(0);
    let targeted_false_accepts =
        json_usize(&targeted, &["product_hot_false_accepts"]).unwrap_or(usize::MAX);
    let runtime_parity_mismatches =
        json_usize(&targeted, &["runtime_margin_parity_mismatches"]).unwrap_or(usize::MAX);
    let accept_parity = request_rows == expected_unique;
    let token_parity = total_tokens_requiring_billing == expected_tokens;
    let cost_parity = current_known_cost_microusd == expected_cost;
    let request_ready = request_rows > 0
        && accept_parity
        && token_parity
        && cost_parity
        && false_accept_rows == 0
        && targeted_false_accepts == 0
        && runtime_parity_mismatches == 0
        && provider_correlation_ready_rows == request_rows
        && json_bool(&targeted, &["local_accept_enabled"]) == Some(false)
        && json_bool(&targeted, &["market_money_claim_allowed"]) == Some(false);
    let verdict = if request_ready {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_BILLING_REQUEST_V1_READY_FOR_PROVIDER_EVIDENCE"
    } else {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_BILLING_REQUEST_V1_WATCH"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_targeted_billing_request_v1",
        "targeted_shadow_report_path": targeted_shadow_report_path,
        "decision_log_path": decision_log_path,
        "billing_request_jsonl_path": request_jsonl_path,
        "product_hot_package_count": candidates.len(),
        "selected_decision_rows": selected_decision_rows,
        "billing_request_rows": request_rows,
        "request_rows_with_exact_cache_key": request_rows_with_exact_cache_key,
        "external_provider_correlation_key_rows": external_provider_correlation_key_rows,
        "provider_correlation_ready_rows": provider_correlation_ready_rows,
        "provider_correlation_missing_rows": request_rows.saturating_sub(provider_correlation_ready_rows),
        "targeted_product_hot_unique_cpu_accepts_over_exact_cache": expected_unique,
        "accept_parity": accept_parity,
        "total_tokens_requiring_billing": total_tokens_requiring_billing,
        "targeted_product_hot_tokens_saved": expected_tokens,
        "token_parity": token_parity,
        "current_known_cost_microusd": current_known_cost_microusd,
        "targeted_product_hot_cost_microusd": expected_cost,
        "cost_parity": cost_parity,
        "token_cost_estimate_used": true,
        "provider_billing_evidence_present": false,
        "ready_for_external_provider_evidence": request_ready,
        "skipped_calibration_rows": skipped_calibration_rows,
        "skipped_below_threshold": skipped_below_threshold,
        "skipped_not_verified_safe": skipped_not_verified_safe,
        "skipped_exact_cache_hit": skipped_exact_cache_hit,
        "skipped_runtime_parity_mismatch": skipped_runtime_parity_mismatch,
        "skipped_duplicate_request_fingerprint": skipped_duplicate_request_fingerprint,
        "false_accept_rows": false_accept_rows,
        "targeted_product_hot_false_accepts": targeted_false_accepts,
        "runtime_margin_parity_mismatches": runtime_parity_mismatches,
        "billing_gate": {
            "provider_billing_request_only": true,
            "provider_billing_evidence_present": false,
            "market_money_claim_allowed": false,
            "policy": "request rows identify clean product-hot shadow accepted calls that need external provider billing evidence; internal cost fields are not market money evidence"
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
        "boundary": "targeted billing request export only: clean product-hot .nwpc shadow rows are converted to provider billing match keys; no serving, local_accept, auto_promote, money claim, lookup, or legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_targeted_billing_request_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  billing_request_jsonl_path: {}",
        request_jsonl_path.display()
    );
    println!("  billing_request_rows: {request_rows}");
    println!("  accept_parity: {accept_parity}");
    println!("  token_parity: {token_parity}");
    println!("  cost_parity: {cost_parity}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

pub(crate) fn run_phase_stream_online_miner_promotion_provider_capture_request_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_ONLINE_MINER_PROMOTION_PROVIDER_CAPTURE_REQUEST_REPORT)
    });
    let output_jsonl_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_ONLINE_MINER_PROMOTION_PROVIDER_CAPTURE_REQUEST_JSONL)
    });
    let billing_request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PROMOTION_BILLING_REQUEST_JSONL));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let billing_request_bytes = std::fs::read(&billing_request_jsonl_path).map_err(|error| {
        format!(
            "failed to read promotion billing request '{}': {error}",
            billing_request_jsonl_path.display()
        )
    })?;
    let billing_request_fingerprint64 = fnv1a64(&billing_request_bytes);
    let billing_request_text = std::str::from_utf8(&billing_request_bytes).map_err(|error| {
        format!(
            "promotion billing request '{}' is not UTF-8: {error}",
            billing_request_jsonl_path.display()
        )
    })?;

    if let Some(parent) = output_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create promotion provider capture request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let output_file = std::fs::File::create(&output_jsonl_path).map_err(|error| {
        format!(
            "failed to create promotion provider capture request '{}': {error}",
            output_jsonl_path.display()
        )
    })?;
    let mut writer = BufWriter::new(output_file);

    let mut billing_request_rows = 0usize;
    let mut capture_request_rows = 0usize;
    let mut rows_missing_join_keys = 0usize;
    let mut rows_with_provider_correlation_keys = 0usize;
    let mut request_rows_with_exact_cache_key = 0usize;
    let mut request_rows_with_request_fingerprint = 0usize;
    let mut total_tokens = 0usize;
    let mut total_cost_microusd = 0u64;
    let mut token_estimate_rows = 0usize;
    let mut cost_estimate_rows = 0usize;

    for (line_index, line) in billing_request_text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        billing_request_rows += 1;
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse promotion billing request '{}' line {}: {error}",
                billing_request_jsonl_path.display(),
                line_index + 1
            )
        })?;

        let mut join_keys = billing_request_match_keys(&row);
        join_keys.sort();
        join_keys.dedup();
        if join_keys.is_empty() {
            rows_missing_join_keys += 1;
            continue;
        }

        let provider_correlation_keys = external_provider_correlation_keys(&row);
        rows_with_provider_correlation_keys += usize::from(!provider_correlation_keys.is_empty());
        request_rows_with_exact_cache_key += usize::from(
            json_string(&row, &["exact_cache_key"])
                .as_deref()
                .is_some_and(|value| !value.is_empty()),
        );
        request_rows_with_request_fingerprint += usize::from(
            json_string(&row, &["request_fingerprint"])
                .as_deref()
                .is_some_and(|value| !value.is_empty()),
        );

        let tokens = json_usize(&row, &["estimated_total_tokens"]).unwrap_or(0);
        let cost = json_u64(&row, &["current_total_cost_microusd"]).unwrap_or(0);
        total_tokens = total_tokens.saturating_add(tokens);
        total_cost_microusd = total_cost_microusd.saturating_add(cost);
        token_estimate_rows += usize::from(tokens > 0);
        cost_estimate_rows += usize::from(cost > 0);

        let capture_request_id = json_string(&row, &["billing_request_id"]).unwrap_or_else(|| {
            format!("online-miner-promotion-provider-capture-{}", line_index + 1)
        });
        let capture_row = serde_json::json!({
            "schema_version": "provider_boundary_capture_request_v1",
            "capture_request_id": capture_request_id,
            "source_schema_version": json_string(&row, &["schema_version"]),
            "source_billing_request_id": json_string(&row, &["billing_request_id"]),
            "source_billing_request_jsonl_path": billing_request_jsonl_path,
            "source_billing_request_fingerprint64": billing_request_fingerprint64,
            "primary_join_key": join_keys.first().cloned(),
            "join_keys": join_keys,
            "provider_correlation_ready": !provider_correlation_keys.is_empty(),
            "external_provider_correlation_keys": provider_correlation_keys,
            "request_fingerprint": json_string(&row, &["request_fingerprint"]),
            "exact_cache_key": json_string(&row, &["exact_cache_key"]),
            "bucket_key": json_string(&row, &["bucket_key"]),
            "task_name": json_string(&row, &["task_name"]),
            "package_fingerprint64": json_u64(&row, &["package_fingerprint64"]).unwrap_or(0),
            "denominator_row_index": json_u64(&row, &["denominator_row_index"]).unwrap_or(line_index as u64 + 1),
            "phase_row_count": 1,
            "total_tokens": tokens,
            "total_cost_microusd": cost,
            "token_cost_estimate_used": json_bool(&row, &["token_cost_estimate_used"]).unwrap_or(true),
            "sample_sources": [{
                "kind": "online_miner_promotion_billing_request",
                "billing_request_id": json_string(&row, &["billing_request_id"]),
                "request_fingerprint": json_string(&row, &["request_fingerprint"]),
                "bucket_key": json_string(&row, &["bucket_key"])
            }],
            "required_provider_fields": [
                "billing_evidence_id",
                "billing_source",
                "provider",
                "provider_request_id or provider_response_id or provider_trace_id or external_provider_request_id or openai_request_id or anthropic_request_id",
                "provider_total_tokens",
                "provider_cost_microusd"
            ],
            "local_accept_enabled": false,
            "market_money_claim_allowed": false,
            "boundary": "provider-boundary capture request derived from compact online-miner promotion billing rows; worklist only, no provider ids, no billing evidence, no mining, no scoring, no serving, no local_accept, no money claim"
        });
        serde_json::to_writer(&mut writer, &capture_row).map_err(|error| {
            format!(
                "failed to serialize promotion provider capture request '{}': {error}",
                output_jsonl_path.display()
            )
        })?;
        writer.write_all(b"\n").map_err(|error| {
            format!(
                "failed to write promotion provider capture request '{}': {error}",
                output_jsonl_path.display()
            )
        })?;
        capture_request_rows += 1;
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush promotion provider capture request '{}': {error}",
            output_jsonl_path.display()
        )
    })?;

    let capture_ready = billing_request_rows > 0
        && capture_request_rows == billing_request_rows
        && rows_missing_join_keys == 0
        && total_tokens > 0;
    let verdict = if capture_ready {
        "PHASE_STREAM_ONLINE_MINER_PROMOTION_PROVIDER_CAPTURE_REQUEST_V1_READY"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PROMOTION_PROVIDER_CAPTURE_REQUEST_V1_WATCH"
    };
    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_promotion_provider_capture_request_v1",
        "billing_request_jsonl_path": billing_request_jsonl_path,
        "billing_request_fingerprint64": billing_request_fingerprint64,
        "output_jsonl_path": output_jsonl_path,
        "billing_request_rows": billing_request_rows,
        "capture_request_rows": capture_request_rows,
        "rows_missing_join_keys": rows_missing_join_keys,
        "rows_with_provider_correlation_keys": rows_with_provider_correlation_keys,
        "request_rows_with_exact_cache_key": request_rows_with_exact_cache_key,
        "request_rows_with_request_fingerprint": request_rows_with_request_fingerprint,
        "total_tokens_requiring_provider_capture": total_tokens,
        "total_cost_requiring_provider_capture_microusd": total_cost_microusd,
        "token_estimate_rows": token_estimate_rows,
        "cost_estimate_rows": cost_estimate_rows,
        "provider_billing_evidence_present": false,
        "capture_ready_for_live_provider_boundary": capture_ready,
        "required_next_gate": {
            "command": "phase-stream-provider-boundary-billing-capture-contract-v1",
            "needs_real_provider_boundary_events": true,
            "policy": "fill capture rows with external provider ids, tokens, and cost; trace estimates are not provider billing evidence"
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
        "boundary": "capture request bridge only: converts compact promotion billing rows into provider-boundary capture worklist; no provider evidence, no serving, no promotion, no local_accept, no market money claim"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_promotion_provider_capture_request_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  output_jsonl_path: {}", output_jsonl_path.display());
    println!("  billing_request_rows: {billing_request_rows}");
    println!("  capture_request_rows: {capture_request_rows}");
    println!("  total_tokens_requiring_provider_capture: {total_tokens}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

pub(crate) fn run_phase_stream_online_miner_targeted_admission_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_TARGETED_ADMISSION_GATE_REPORT));
    let targeted_shadow_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_TARGETED_SHADOW_REPORT));
    let promotion_registry_gate_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PROMOTION_REGISTRY_GATE_REPORT));
    let billing_evidence_gate_report_path = args.next().map(PathBuf::from);
    let provider_coverage_report_path = args.next().map(PathBuf::from);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let targeted = read_json_value(&targeted_shadow_report_path)?;
    let promotion = read_json_value(&promotion_registry_gate_report_path)?;
    let billing = billing_evidence_gate_report_path
        .as_deref()
        .map(read_json_value)
        .transpose()?;
    let coverage = provider_coverage_report_path
        .as_deref()
        .map(read_json_value)
        .transpose()?;

    let product_hot_packages = targeted
        .get("product_hot_packages")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let targeted_accepts = json_usize(
        &targeted,
        &["product_hot_unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or(0);
    let targeted_tokens =
        json_usize(&targeted, &["product_hot_nando_cpu_tokens_saved"]).unwrap_or(0);
    let targeted_cost =
        json_u64(&targeted, &["product_hot_nando_cpu_cost_saved_microusd"]).unwrap_or(0);
    let targeted_false_accepts =
        json_usize(&targeted, &["product_hot_false_accepts"]).unwrap_or(usize::MAX);
    let runtime_margin_parity_mismatches =
        json_usize(&targeted, &["runtime_margin_parity_mismatches"]).unwrap_or(usize::MAX);
    let product_hot_threshold_policy = targeted_product_hot_threshold_policy(&targeted);
    let targeted_shadow_clean = json_string(&targeted, &["report_kind"]).as_deref()
        == Some("phase_stream_online_miner_targeted_shadow_v1")
        && json_string(&targeted, &["verdict"])
            .as_deref()
            .is_some_and(|verdict| verdict.contains("_PASS_"))
        && product_hot_packages > 0
        && targeted_accepts > 0
        && targeted_tokens > 0
        && targeted_false_accepts == 0
        && runtime_margin_parity_mismatches == 0
        && product_hot_threshold_policy.clean
        && json_bool(&targeted, &["local_accept_enabled"]) == Some(false)
        && json_bool(&targeted, &["auto_promote_enabled"]) == Some(false)
        && json_bool(&targeted, &["market_money_claim_allowed"]) == Some(false)
        && forbidden_flags_all_false(&targeted);

    let promoted_count = json_usize(&promotion, &["promoted_candidate_count"]).unwrap_or(0);
    let blocked_count = json_usize(&promotion, &["blocked_candidate_count"]).unwrap_or(usize::MAX);
    let promotion_accepts =
        json_usize(&promotion, &["global_unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let promotion_tokens = json_usize(&promotion, &["nando_cpu_tokens_saved"]).unwrap_or(0);
    let promotion_cost = json_u64(&promotion, &["nando_cpu_cost_saved_microusd"]).unwrap_or(0);
    let promotion_false_accepts = json_usize(&promotion, &["false_accepts"]).unwrap_or(usize::MAX);
    let promotion_gate_clean = json_string(&promotion, &["report_kind"]).as_deref()
        == Some("phase_stream_online_miner_promotion_registry_gate_v1")
        && json_string(&promotion, &["verdict"]).as_deref()
            == Some(
                "PHASE_STREAM_ONLINE_MINER_PROMOTION_REGISTRY_GATE_V1_PASS_SHADOW_REGISTRY_READY",
            )
        && promoted_count == product_hot_packages
        && blocked_count == 0
        && promotion_accepts == targeted_accepts
        && promotion_tokens == targeted_tokens
        && promotion_cost == targeted_cost
        && promotion_false_accepts == 0
        && json_bool(&promotion, &["registry_global_gate_clear"]) == Some(true)
        && json_bool(&promotion, &["local_accept_enabled"]) == Some(false)
        && json_bool(&promotion, &["auto_promote_enabled"]) == Some(false)
        && json_bool(&promotion, &["product_runtime_changed"]) == Some(false)
        && json_bool(&promotion, &["serving_runtime_changed"]) == Some(false)
        && json_bool(&promotion, &["market_money_claim_allowed"]) == Some(false)
        && forbidden_flags_all_false(&promotion);

    let provider_capture_complete = coverage.as_ref().is_some_and(|coverage| {
        json_string(coverage, &["report_kind"]).as_deref()
            == Some("phase_stream_provider_boundary_capture_coverage_gate_v1")
            && json_bool(coverage, &["readiness", "full_capture_coverage"]) == Some(true)
            && json_usize(coverage, &["capture_requests", "covered_capture_requests"]).unwrap_or(0)
                >= targeted_accepts
            && json_usize(coverage, &["capture_requests", "missing_capture_requests"])
                .unwrap_or(usize::MAX)
                == 0
            && json_bool(coverage, &["local_accept_enabled"]) == Some(false)
            && json_bool(coverage, &["market_money_claim_allowed"]) == Some(false)
            && forbidden_flags_all_false(coverage)
    });

    let provider_billing_evidence_present = billing.as_ref().is_some_and(|billing| {
        json_string(billing, &["report_kind"]).as_deref()
            == Some("phase_stream_provider_boundary_billing_capture_evidence_gate_v1")
            && json_bool(
                billing,
                &["readiness", "provider_billing_evidence_complete"],
            ) == Some(true)
            && json_usize(billing, &["valid_evidence_rows"]).unwrap_or(0) > 0
            && json_usize(billing, &["covered_capture_requests"]).unwrap_or(0) >= targeted_accepts
            && json_usize(billing, &["missing_capture_requests"]).unwrap_or(usize::MAX) == 0
            && json_usize(billing, &["provider_total_tokens"]).unwrap_or(0) > 0
            && json_u64(billing, &["provider_cost_microusd"]).unwrap_or(0) > 0
            && json_bool(billing, &["local_accept_enabled"]) == Some(false)
            && json_bool(billing, &["market_money_claim_allowed"]) == Some(false)
            && forbidden_flags_all_false(billing)
    });

    let shadow_admission_candidate_allowed = targeted_shadow_clean && promotion_gate_clean;
    let product_promotion_allowed =
        shadow_admission_candidate_allowed && provider_billing_evidence_present;
    let market_money_claim_allowed = product_promotion_allowed;

    let mut blockers = Vec::<&'static str>::new();
    if !targeted_shadow_clean {
        blockers.push("targeted_shadow_not_clean");
    }
    if !product_hot_threshold_policy.clean {
        blockers.push("targeted_threshold_policy_missing_or_manual");
    }
    if !promotion_gate_clean {
        blockers.push("promotion_registry_gate_not_clean");
    }
    if !provider_capture_complete {
        blockers.push("provider_capture_coverage_missing_or_incomplete");
    }
    if !provider_billing_evidence_present {
        blockers.push("provider_billing_evidence_missing_or_incomplete");
    }

    let verdict = if product_promotion_allowed {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_ADMISSION_GATE_V1_PASS_PROMOTION_READY"
    } else if shadow_admission_candidate_allowed {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_ADMISSION_GATE_V1_PASS_SHADOW_READY_BILLING_BLOCKED"
    } else {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_ADMISSION_GATE_V1_WATCH"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_targeted_admission_gate_v1",
        "targeted_shadow_report_path": targeted_shadow_report_path,
        "promotion_registry_gate_report_path": promotion_registry_gate_report_path,
        "billing_evidence_gate_report_path": billing_evidence_gate_report_path,
        "provider_coverage_report_path": provider_coverage_report_path,
        "targeted_shadow": {
            "product_hot_package_count": product_hot_packages,
            "product_hot_unique_cpu_accepts_over_exact_cache": targeted_accepts,
            "product_hot_tokens_saved": targeted_tokens,
            "product_hot_cost_saved_microusd": targeted_cost,
            "product_hot_false_accepts": targeted_false_accepts,
            "runtime_margin_parity_mismatches": runtime_margin_parity_mismatches,
            "product_hot_threshold_policy_clean": product_hot_threshold_policy.clean,
            "product_hot_threshold_policy": {
                "candidate_package_count": product_hot_threshold_policy.package_count,
                "auto_calibrated_package_count": product_hot_threshold_policy.auto_calibrated_package_count,
                "manual_or_missing_threshold_package_count": product_hot_threshold_policy.manual_or_missing_threshold_package_count,
                "packages_with_calibration_window": product_hot_threshold_policy.packages_with_calibration_window,
                "packages_with_shadow_after_calibration": product_hot_threshold_policy.packages_with_shadow_after_calibration,
                "packages_with_threshold_equal_safe_accept": product_hot_threshold_policy.packages_with_threshold_equal_safe_accept,
                "min_auto_calibration_events": product_hot_threshold_policy.min_auto_calibration_events,
                "min_shadow_events_after_calibration": product_hot_threshold_policy.min_shadow_events_after_calibration,
                "source": "per-product-hot-package auto_calibrated_margin_threshold_micro plus calibration/shadow windows"
            },
            "targeted_shadow_clean": targeted_shadow_clean
        },
        "promotion_registry_gate": {
            "promoted_candidate_count": promoted_count,
            "blocked_candidate_count": blocked_count,
            "global_unique_cpu_accepts_over_exact_cache": promotion_accepts,
            "nando_cpu_tokens_saved": promotion_tokens,
            "nando_cpu_cost_saved_microusd": promotion_cost,
            "false_accepts": promotion_false_accepts,
            "promotion_gate_clean": promotion_gate_clean
        },
        "provider_capture_gate": {
            "provider_capture_complete": provider_capture_complete,
            "policy": "provider-boundary coverage proves join-key capture readiness only; it is not billing money evidence"
        },
        "billing_gate": {
            "provider_billing_evidence_present": provider_billing_evidence_present,
            "policy": "promotion and market money require non-synthetic provider billing evidence with identity, tokens, and cost covering every targeted clean accept"
        },
        "admission_gate": {
            "shadow_admission_candidate_allowed": shadow_admission_candidate_allowed,
            "product_promotion_allowed": product_promotion_allowed,
            "market_money_claim_allowed": market_money_claim_allowed,
            "local_accept_enabled": false,
            "auto_promote_enabled": false,
            "blockers": blockers
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
        "product_promotion_allowed": product_promotion_allowed,
        "market_money_claim_allowed": market_money_claim_allowed,
        "verdict": verdict,
        "boundary": "targeted admission/economics gate only: joins targeted shadow proof, shadow-registry promotion gate, provider capture coverage, and optional provider billing evidence; it does not compile, serve, mutate product registry, enable local_accept, estimate missing money, or revive legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_targeted_admission_gate_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  shadow_admission_candidate_allowed: {shadow_admission_candidate_allowed}");
    println!("  provider_capture_complete: {provider_capture_complete}");
    println!("  provider_billing_evidence_present: {provider_billing_evidence_present}");
    println!("  product_promotion_allowed: {product_promotion_allowed}");
    println!("  market_money_claim_allowed: {market_money_claim_allowed}");
    println!("  verdict: {verdict}");
    Ok(())
}

fn promotion_billing_candidates(
    promotion_gate: &Value,
    registry: &Value,
) -> Result<Vec<PromotionBillingCandidate>, String> {
    let accepted_keys = promotion_gate
        .get("promoted_packages")
        .and_then(Value::as_array)
        .ok_or_else(|| "promotion gate report missing promoted_packages".to_owned())?
        .iter()
        .filter(|package| {
            json_bool(package, &["accepted_for_shadow_registry"]).unwrap_or(false)
                && json_usize(package, &["false_accepts"]).unwrap_or(usize::MAX) == 0
        })
        .filter_map(|package| json_string(package, &["bucket_key"]))
        .collect::<BTreeSet<_>>();
    let mut candidates = Vec::new();
    for candidate in registry
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| "source promotion registry missing candidates".to_owned())?
    {
        let Some(bucket_key) = json_string(candidate, &["bucket_key"]) else {
            continue;
        };
        if !accepted_keys.contains(&bucket_key) {
            continue;
        }
        candidates.push(PromotionBillingCandidate {
            bucket_key,
            task_name: json_string(candidate, &["task_name"]).unwrap_or_default(),
            package_fingerprint64: json_u64(candidate, &["package_fingerprint64"]).unwrap_or(0),
            threshold_micro: json_i64(candidate, &["auto_calibrated_margin_threshold_micro"])
                .or_else(|| json_i64(candidate, &["safe_accept_margin_threshold_micro"]))
                .unwrap_or(1)
                .max(1),
            calibration_events: json_usize(candidate, &["auto_calibration_events"]).unwrap_or(0),
        });
    }
    candidates.sort_by(|left, right| left.bucket_key.cmp(&right.bucket_key));
    Ok(candidates)
}

fn targeted_billing_candidates(targeted: &Value) -> Result<Vec<PromotionBillingCandidate>, String> {
    let mut candidates = Vec::new();
    for candidate in targeted
        .get("product_hot_packages")
        .and_then(Value::as_array)
        .ok_or_else(|| "targeted shadow report missing product_hot_packages".to_owned())?
    {
        if json_bool(candidate, &["promotion_gate_passed"]) != Some(true)
            || json_usize(candidate, &["false_accepts"]).unwrap_or(usize::MAX) != 0
            || json_bool(candidate, &["local_accept_enabled"]).unwrap_or(true)
            || json_bool(candidate, &["auto_promote_enabled"]).unwrap_or(true)
        {
            continue;
        }
        let Some(bucket_key) = json_string(candidate, &["bucket_key"]) else {
            continue;
        };
        candidates.push(PromotionBillingCandidate {
            bucket_key,
            task_name: json_string(candidate, &["task_name"]).unwrap_or_default(),
            package_fingerprint64: json_u64(candidate, &["package_fingerprint64"]).unwrap_or(0),
            threshold_micro: json_i64(candidate, &["auto_calibrated_margin_threshold_micro"])
                .or_else(|| json_i64(candidate, &["safe_accept_margin_threshold_micro"]))
                .unwrap_or(1)
                .max(1),
            calibration_events: json_usize(candidate, &["auto_calibration_events"]).unwrap_or(0),
        });
    }
    candidates.sort_by(|left, right| left.bucket_key.cmp(&right.bucket_key));
    Ok(candidates)
}

fn targeted_product_hot_threshold_policy(targeted: &Value) -> TargetedThresholdPolicyAudit {
    let Some(packages) = targeted
        .get("product_hot_packages")
        .and_then(Value::as_array)
    else {
        return TargetedThresholdPolicyAudit::default();
    };
    let mut audit = TargetedThresholdPolicyAudit {
        package_count: packages.len(),
        min_auto_calibration_events: usize::MAX,
        min_shadow_events_after_calibration: usize::MAX,
        ..TargetedThresholdPolicyAudit::default()
    };
    for package in packages {
        let auto_threshold = json_i64(package, &["auto_calibrated_margin_threshold_micro"]);
        let safe_threshold = json_i64(package, &["safe_accept_margin_threshold_micro"]);
        let calibration_events = json_usize(package, &["auto_calibration_events"]).unwrap_or(0);
        let shadow_events = json_usize(package, &["shadow_events_after_calibration"]).unwrap_or(0);
        let auto_calibrated = auto_threshold.is_some()
            && auto_threshold.unwrap_or_default() > 0
            && calibration_events > 0
            && shadow_events > 0;
        if auto_calibrated {
            audit.auto_calibrated_package_count += 1;
        } else {
            audit.manual_or_missing_threshold_package_count += 1;
        }
        if calibration_events > 0 {
            audit.packages_with_calibration_window += 1;
        }
        if shadow_events > 0 {
            audit.packages_with_shadow_after_calibration += 1;
        }
        if auto_threshold.is_some() && auto_threshold == safe_threshold {
            audit.packages_with_threshold_equal_safe_accept += 1;
        }
        audit.min_auto_calibration_events =
            audit.min_auto_calibration_events.min(calibration_events);
        audit.min_shadow_events_after_calibration =
            audit.min_shadow_events_after_calibration.min(shadow_events);
    }
    if audit.package_count == 0 {
        audit.min_auto_calibration_events = 0;
        audit.min_shadow_events_after_calibration = 0;
    }
    audit.clean = audit.package_count > 0
        && audit.auto_calibrated_package_count == audit.package_count
        && audit.manual_or_missing_threshold_package_count == 0
        && audit.packages_with_calibration_window == audit.package_count
        && audit.packages_with_shadow_after_calibration == audit.package_count;
    audit
}

fn promotion_decisions_by_bucket(
    decision_log_path: &Path,
    selected_bucket_keys: &BTreeSet<String>,
) -> Result<BTreeMap<String, Vec<PromotionBillingDecision>>, String> {
    let text = std::fs::read_to_string(decision_log_path).map_err(|error| {
        format!(
            "failed to read online miner decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    let mut buckets = BTreeMap::<String, Vec<PromotionBillingDecision>>::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse online miner decision log '{}' line {}: {error}",
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
        let decision = PromotionBillingDecision {
            bucket_key: bucket_key.clone(),
            request_fingerprint: json_string(&row, &["request_fingerprint"])
                .unwrap_or_else(|| format!("decision-row:{}", line_index + 1)),
            exact_cache_key: json_string(&row, &["exact_cache_key"]),
            external_provider_correlation_keys: external_provider_correlation_keys(&row),
            exact_cache_hit: json_bool(&row, &["exact_cache_hit"]).unwrap_or(false),
            verified_safe_accept: json_bool(&row, &["verified_safe_accept"]).unwrap_or(false),
            false_accept: json_bool(&row, &["false_accept"]).unwrap_or(false),
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
                .unwrap_or(true),
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

fn external_provider_correlation_keys(row: &Value) -> Vec<String> {
    let mut keys = row
        .get("external_provider_correlation_keys")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    keys.sort();
    keys.dedup();
    keys
}

fn billing_request_match_keys(row: &Value) -> Vec<String> {
    let mut keys = row
        .get("match_keys")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(request_fingerprint) =
        json_string(row, &["request_fingerprint"]).filter(|value| !value.is_empty())
    {
        keys.push(format!("request_fingerprint:{request_fingerprint}"));
    }
    if let Some(exact_cache_key) =
        json_string(row, &["exact_cache_key"]).filter(|value| !value.is_empty())
    {
        keys.push(format!("exact_cache_key:{exact_cache_key}"));
    }
    keys.extend(external_provider_correlation_keys(row));
    keys
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

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_at(value, path)
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
        })
        .and_then(|value| usize::try_from(value).ok())
}

fn json_i64(value: &Value, path: &[&str]) -> Option<i64> {
    json_at(value, path).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
    })
}

fn forbidden_flags_all_false(value: &Value) -> bool {
    [
        "nwrb_used",
        "role_binding_backend_used",
        "lookup_used",
        "target_id_or_proof_rule_id_authority_used",
        "concrete_x_lookup_used",
        "manual_local_out_t_used",
        "local_accept_without_verifier_used",
    ]
    .iter()
    .all(|flag| json_bool(value, &["forbidden_flags", flag]) == Some(false))
}
