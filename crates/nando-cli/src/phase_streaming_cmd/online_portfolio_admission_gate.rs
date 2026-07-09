use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_ONLINE_MINER_PORTFOLIO_ADMISSION_GATE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-portfolio-admission-gate-v1.report.json";
const DEFAULT_ONLINE_MINER_PORTFOLIO_RUNTIME_REPLAY_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-portfolio-runtime-replay-v1.report.json";

pub(crate) fn run_phase_stream_online_miner_portfolio_admission_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_ADMISSION_GATE_REPORT));
    let runtime_replay_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_RUNTIME_REPLAY_REPORT));
    let billing_evidence_join_report_path = args.next().map(PathBuf::from);

    let runtime = read_json_value(&runtime_replay_report_path)?;
    let billing = billing_evidence_join_report_path
        .as_deref()
        .map(read_json_value)
        .transpose()?;

    let runtime_report_kind = json_string(&runtime, &["report_kind"]).unwrap_or_default();
    let np_rescue_runtime_replay =
        runtime_report_kind == "phase_stream_online_miner_portfolio_np_rescue_runtime_replay_v1";
    let future_tail_runtime_replay =
        runtime_report_kind == "phase_stream_online_miner_portfolio_future_tail_replay_v1";
    let replay_rows = json_usize(&runtime, &["replay_rows"])
        .or_else(|| json_usize(&runtime, &["future_rows"]))
        .unwrap_or(0);
    let portfolio_accepts =
        json_usize(&runtime, &["portfolio_unique_cpu_accepts_over_exact_cache"])
            .or_else(|| json_usize(&runtime, &["unique_cpu_accepts_over_exact_cache"]))
            .or_else(|| json_usize(&runtime, &["future_unique_accepts_over_exact_cache"]))
            .unwrap_or(0);
    let portfolio_tokens = json_usize(&runtime, &["portfolio_tokens_saved"])
        .or_else(|| json_usize(&runtime, &["tokens_saved"]))
        .or_else(|| json_usize(&runtime, &["future_tokens_saved"]))
        .unwrap_or(0);
    let portfolio_cost_microusd = json_u64(&runtime, &["portfolio_cost_saved_microusd"])
        .or_else(|| json_u64(&runtime, &["cost_saved_microusd"]))
        .or_else(|| json_u64(&runtime, &["future_cost_saved_microusd"]))
        .unwrap_or(0);
    let external_provider_correlation_key_rows =
        json_usize(&runtime, &["external_provider_correlation_key_rows"]);
    let external_provider_correlation_missing_rows =
        json_usize(&runtime, &["external_provider_correlation_missing_rows"]);
    let provider_correlation_known = external_provider_correlation_key_rows.is_some()
        || external_provider_correlation_missing_rows.is_some();
    let provider_correlation_rows = external_provider_correlation_key_rows.unwrap_or(0);
    let provider_correlation_complete_for_shadow_accepts =
        provider_correlation_known && provider_correlation_rows >= portfolio_accepts;
    let false_accepts = json_usize(&runtime, &["false_accepts"])
        .or_else(|| json_usize(&runtime, &["future_false_accepts"]))
        .unwrap_or(usize::MAX);
    let wrong_wins = json_usize(&runtime, &["wrong_wins"]).unwrap_or(0);
    let hot_margin_parity_mismatches =
        json_usize(&runtime, &["hot_margin_parity_mismatches"]).unwrap_or(usize::MAX);
    let hot_decision_parity_mismatches =
        json_usize(&runtime, &["hot_decision_parity_mismatches"]).unwrap_or(usize::MAX);
    let decision_log_margin_mismatches = json_usize(&runtime, &["decision_log_margin_mismatches"])
        .unwrap_or(if future_tail_runtime_replay {
            0
        } else {
            usize::MAX
        });
    let selector_accept_parity = json_bool(&runtime, &["selector_accept_parity"])
        .or_else(|| json_bool(&runtime, &["replay_accept_parity"]))
        .unwrap_or(future_tail_runtime_replay);
    let selector_token_parity = json_bool(&runtime, &["selector_token_parity"])
        .or_else(|| json_bool(&runtime, &["replay_token_parity"]))
        .unwrap_or(future_tail_runtime_replay);
    let runtime_replay_passed = json_bool(&runtime, &["discovery_mode", "runtime_replay_passed"])
        .or_else(|| json_bool(&runtime, &["runtime_replay_passed"]))
        .unwrap_or(false);
    let verifier_bound_score_candidate_rows =
        json_usize(&runtime, &["verifier_bound_score_candidate_rows"]).unwrap_or(0);
    let verifier_rejected_score_candidate_rows =
        json_usize(&runtime, &["verifier_rejected_score_candidate_rows"]).unwrap_or(usize::MAX);
    let future_tail_verifier_binding_bound = future_tail_runtime_replay
        && verifier_bound_score_candidate_rows > 0
        && verifier_rejected_score_candidate_rows == 0;
    let verifier_binding_bound = json_bool(&runtime, &["verifier_binding_bound"])
        .unwrap_or(future_tail_verifier_binding_bound);
    let manual_class_list_used =
        json_bool(&runtime, &["discovery_mode", "manual_class_list_used"]).unwrap_or(true);
    let static_topn_seed_used =
        json_bool(&runtime, &["discovery_mode", "static_topn_seed_used"]).unwrap_or(true);
    let online_discovery_used =
        json_bool(&runtime, &["discovery_mode", "online_discovery_used"]).unwrap_or(false);
    let marginal_denominator_delta_used = json_bool(
        &runtime,
        &["discovery_mode", "marginal_denominator_delta_used"],
    )
    .unwrap_or(false);
    let aggressive_np_rescue_used =
        json_bool(&runtime, &["discovery_mode", "aggressive_np_rescue_used"]).unwrap_or(false);
    let portfolio_gate_passed =
        json_bool(&runtime, &["discovery_mode", "portfolio_gate_passed"]).unwrap_or(false);
    let dynamic_discovery_shadow_claim_allowed = json_bool(
        &runtime,
        &["discovery_mode", "dynamic_discovery_shadow_claim_allowed"],
    )
    .unwrap_or(false);
    let local_accept_enabled = json_bool(&runtime, &["local_accept_enabled"]).unwrap_or(true);
    let auto_promote_enabled = json_bool(&runtime, &["auto_promote_enabled"]).unwrap_or(true);
    let forbidden_flags_clear = forbidden_flags_clear(&runtime);

    let parity_clean = hot_margin_parity_mismatches == 0
        && hot_decision_parity_mismatches == 0
        && decision_log_margin_mismatches == 0
        && selector_accept_parity
        && selector_token_parity;
    let positive_shadow_value = replay_rows > 0 && portfolio_accepts > 0 && portfolio_tokens > 0;
    let standard_online_portfolio_runtime_replay =
        runtime_report_kind == "phase_stream_online_miner_portfolio_runtime_replay_v1";
    let standard_online_portfolio_shadow_clean = standard_online_portfolio_runtime_replay
        && !manual_class_list_used
        && !static_topn_seed_used
        && online_discovery_used
        && marginal_denominator_delta_used
        && portfolio_gate_passed
        && runtime_replay_passed;
    let standard_selector_clean = standard_online_portfolio_shadow_clean;
    let standard_selector_admission_deprecated = standard_selector_clean;
    let np_rescue_shadow_clean = np_rescue_runtime_replay
        && !manual_class_list_used
        && !static_topn_seed_used
        && online_discovery_used
        && aggressive_np_rescue_used
        && runtime_replay_passed;
    let future_tail_shadow_clean = future_tail_runtime_replay
        && !manual_class_list_used
        && !static_topn_seed_used
        && online_discovery_used
        && marginal_denominator_delta_used
        && portfolio_gate_passed
        && runtime_replay_passed;
    let automatic_selector_clean = standard_online_portfolio_shadow_clean
        || np_rescue_shadow_clean
        || future_tail_shadow_clean;
    let shadow_admission_candidate_allowed = runtime_replay_passed
        && automatic_selector_clean
        && parity_clean
        && false_accepts == 0
        && verifier_binding_bound
        && positive_shadow_value
        && !local_accept_enabled
        && !auto_promote_enabled
        && forbidden_flags_clear;

    let billing_report_kind = billing
        .as_ref()
        .and_then(|value| json_string(value, &["report_kind"]));
    let billing_rows_with_provider_cost = billing
        .as_ref()
        .and_then(|value| json_usize(value, &["billing_rows_with_provider_cost"]))
        .unwrap_or(0);
    let rows_enriched_provider_cost = billing
        .as_ref()
        .and_then(|value| json_usize(value, &["rows_enriched_provider_cost"]))
        .unwrap_or(0);
    let rows_enriched_provider_tokens = billing
        .as_ref()
        .and_then(|value| json_usize(value, &["rows_enriched_provider_tokens"]))
        .unwrap_or(0);
    let provider_billing_cost_microusd = billing
        .as_ref()
        .and_then(|value| json_u64(value, &["provider_cost_microusd"]))
        .unwrap_or(0);
    let provider_billing_total_tokens = billing
        .as_ref()
        .and_then(|value| json_usize(value, &["provider_total_tokens"]))
        .unwrap_or(0);
    let billing_request_provider_correlation_ready_rows = billing
        .as_ref()
        .and_then(|value| json_usize(value, &["request_provider_correlation_ready_rows"]))
        .unwrap_or(0);
    let billing_request_provider_correlation_missing_rows = billing
        .as_ref()
        .and_then(|value| json_usize(value, &["request_provider_correlation_missing_rows"]))
        .unwrap_or(0);
    let billing_request_provider_correlation_complete = portfolio_accepts > 0
        && billing_request_provider_correlation_ready_rows >= portfolio_accepts
        && billing_request_provider_correlation_missing_rows == 0;
    let billing_gate_provider_evidence = billing
        .as_ref()
        .and_then(|value| {
            json_bool(
                value,
                &["billing_gate", "provider_billing_evidence_present"],
            )
            .or_else(|| json_bool(value, &["provider_billing_evidence_present"]))
        })
        .unwrap_or(false);
    let provider_billing_evidence_present = match billing_report_kind.as_deref() {
        Some("provider_billing_evidence_join_v1") => {
            billing_rows_with_provider_cost > 0
                && rows_enriched_provider_cost >= portfolio_accepts
                && rows_enriched_provider_tokens >= portfolio_accepts
                && provider_billing_cost_microusd > 0
                && provider_billing_total_tokens > 0
        }
        Some("phase_stream_online_miner_portfolio_billing_evidence_gate_v1") => {
            billing_gate_provider_evidence
                && billing_rows_with_provider_cost > 0
                && rows_enriched_provider_cost >= portfolio_accepts
                && rows_enriched_provider_tokens >= portfolio_accepts
                && provider_billing_cost_microusd > 0
                && provider_billing_total_tokens > 0
        }
        _ => false,
    };
    let provider_billing_request_only = billing_report_kind
        .as_deref()
        .is_some_and(|kind| kind.contains("billing_request"))
        || billing
            .as_ref()
            .and_then(|value| json_bool(value, &["billing_gate", "provider_billing_request_only"]))
            .unwrap_or(false);

    let mut blockers = Vec::<String>::new();
    if !runtime_replay_passed {
        blockers.push("runtime_replay_not_passed".to_owned());
    }
    if !automatic_selector_clean {
        blockers.push("automatic_selector_gate_not_clean".to_owned());
    }
    if !parity_clean {
        blockers.push("runtime_or_selector_parity_not_clean".to_owned());
    }
    if false_accepts != 0 {
        blockers.push("false_accepts_nonzero".to_owned());
    }
    if !verifier_binding_bound {
        blockers.push("verifier_binding_missing".to_owned());
    }
    if !positive_shadow_value {
        blockers.push("no_positive_unique_accept_token_value".to_owned());
    }
    if local_accept_enabled {
        blockers.push("input_local_accept_already_enabled".to_owned());
    }
    if auto_promote_enabled {
        blockers.push("input_auto_promote_already_enabled".to_owned());
    }
    if !forbidden_flags_clear {
        blockers.push("forbidden_flags_not_clear".to_owned());
    }
    if !provider_billing_evidence_present {
        blockers.push("provider_billing_evidence_missing_or_incomplete".to_owned());
    }
    if provider_correlation_known
        && !provider_correlation_complete_for_shadow_accepts
        && !billing_request_provider_correlation_complete
    {
        blockers.push("provider_correlation_missing_for_shadow_accepts".to_owned());
    }
    if billing_request_provider_correlation_missing_rows > 0 {
        blockers.push("provider_correlation_missing_for_billing_requests".to_owned());
    }
    if provider_billing_cost_microusd == 0 || provider_billing_total_tokens == 0 {
        blockers.push("provider_billing_totals_missing".to_owned());
    }

    let product_promotion_allowed =
        shadow_admission_candidate_allowed && provider_billing_evidence_present;
    let market_money_claim_allowed = product_promotion_allowed;
    let verdict = if product_promotion_allowed {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_ADMISSION_GATE_V1_PASS_PROMOTION_READY"
    } else if shadow_admission_candidate_allowed {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_ADMISSION_GATE_V1_PASS_SHADOW_READY_BILLING_BLOCKED"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_ADMISSION_GATE_V1_WATCH"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_portfolio_admission_gate_v1",
        "runtime_replay_report_path": runtime_replay_report_path,
        "billing_evidence_join_report_path": billing_evidence_join_report_path,
        "runtime_replay": {
            "runtime_report_kind": runtime_report_kind,
            "np_rescue_runtime_replay": np_rescue_runtime_replay,
            "replay_rows": replay_rows,
            "runtime_replay_passed": runtime_replay_passed,
            "portfolio_unique_cpu_accepts_over_exact_cache": portfolio_accepts,
            "portfolio_tokens_saved": portfolio_tokens,
            "portfolio_cost_saved_microusd": portfolio_cost_microusd,
            "false_accepts": false_accepts,
            "wrong_wins": wrong_wins,
            "wrong_wins_policy": "fallback-only diagnostic: promotion is blocked by false_accepts/local_accept, not by low-margin wrong rows that did not pass admission threshold",
            "hot_margin_parity_mismatches": hot_margin_parity_mismatches,
            "hot_decision_parity_mismatches": hot_decision_parity_mismatches,
            "decision_log_margin_mismatches": decision_log_margin_mismatches,
            "selector_accept_parity": selector_accept_parity,
            "selector_token_parity": selector_token_parity,
            "verifier_binding_bound": verifier_binding_bound,
            "verifier_binding": runtime.get("verifier_binding").cloned()
        },
        "discovery_mode": {
            "manual_class_list_used": manual_class_list_used,
            "static_topn_seed_used": static_topn_seed_used,
            "online_discovery_used": online_discovery_used,
            "marginal_denominator_delta_used": marginal_denominator_delta_used,
            "aggressive_np_rescue_used": aggressive_np_rescue_used,
            "portfolio_gate_passed": portfolio_gate_passed,
            "runtime_replay_passed": runtime_replay_passed,
            "dynamic_discovery_shadow_claim_allowed": dynamic_discovery_shadow_claim_allowed,
            "standard_online_portfolio_runtime_replay": standard_online_portfolio_runtime_replay,
            "standard_online_portfolio_shadow_clean": standard_online_portfolio_shadow_clean,
            "standard_selector_clean": standard_selector_clean,
            "standard_selector_admission_deprecated": standard_selector_admission_deprecated,
            "np_rescue_shadow_clean": np_rescue_shadow_clean,
            "automatic_selector_clean": automatic_selector_clean,
            "product_dynamic_discovery_claim_allowed": product_promotion_allowed
        },
        "billing_gate": {
            "billing_report_kind": billing_report_kind,
            "billing_rows_with_provider_cost": billing_rows_with_provider_cost,
            "rows_enriched_provider_cost": rows_enriched_provider_cost,
            "rows_enriched_provider_tokens": rows_enriched_provider_tokens,
            "provider_billing_cost_microusd": provider_billing_cost_microusd,
            "provider_billing_total_tokens": provider_billing_total_tokens,
            "billing_gate_provider_evidence": billing_gate_provider_evidence,
            "provider_billing_request_only": provider_billing_request_only,
            "provider_billing_evidence_present": provider_billing_evidence_present,
            "request_provider_correlation_ready_rows": billing_request_provider_correlation_ready_rows,
            "request_provider_correlation_missing_rows": billing_request_provider_correlation_missing_rows,
            "policy": "market money claim requires external provider billing evidence joined to selected shadow accepts; internal estimates, request-only exports, and legacy selector-only reports are not enough"
        },
        "provider_correlation_gate": {
            "provider_correlation_known": provider_correlation_known,
            "external_provider_correlation_key_rows": external_provider_correlation_key_rows,
            "external_provider_correlation_missing_rows": external_provider_correlation_missing_rows,
            "shadow_accept_rows": portfolio_accepts,
            "provider_correlation_complete_for_shadow_accepts": provider_correlation_complete_for_shadow_accepts,
            "billing_request_provider_correlation_complete": billing_request_provider_correlation_complete,
            "policy": "selected shadow accepts must carry provider correlation metadata before external billing evidence can prove real money savings"
        },
        "admission_gate": {
            "parity_clean": parity_clean,
            "verifier_binding_bound": verifier_binding_bound,
            "positive_shadow_value": positive_shadow_value,
            "forbidden_flags_clear": forbidden_flags_clear,
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
        "boundary": "admission/economics gate only: consumes runtime-replayed selected .nwpc portfolio evidence and optional external billing join evidence; does not compile, promote, serve, mutate registry, enable local_accept, or revive legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_admission_gate_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  shadow_admission_candidate_allowed: {shadow_admission_candidate_allowed}");
    println!("  provider_billing_evidence_present: {provider_billing_evidence_present}");
    println!("  product_promotion_allowed: {product_promotion_allowed}");
    println!("  market_money_claim_allowed: {market_money_claim_allowed}");
    println!("  verdict: {verdict}");
    Ok(())
}

fn forbidden_flags_clear(value: &Value) -> bool {
    let flags = [
        "nwrb_used",
        "role_binding_backend_used",
        "lookup_used",
        "target_id_or_proof_rule_id_authority_used",
        "concrete_x_lookup_used",
        "manual_local_out_t_used",
        "local_accept_without_verifier_used",
    ];
    flags
        .iter()
        .all(|flag| json_bool(value, &["forbidden_flags", flag]).is_some_and(|used| !used))
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
