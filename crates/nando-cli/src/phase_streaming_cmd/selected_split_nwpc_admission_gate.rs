use std::path::PathBuf;

use serde_json::Value;

use super::{json_bool, json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_SELECTED_SPLIT_NWPC_ADMISSION_GATE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-selected-split-nwpc-admission-gate-v1.report.json";
const DEFAULT_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-shadow-replay-v1-realtrace-plus-verifier-sources.report.json";
const DEFAULT_SELECTED_SPLIT_NWPC_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-billing-request-v1-realtrace-plus-verifier-sources.report.json";

pub(crate) fn run_phase_stream_selected_split_nwpc_admission_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_NWPC_ADMISSION_GATE_REPORT));
    let shadow_replay_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_REPORT));
    let billing_request_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_NWPC_BILLING_REQUEST_REPORT));
    let billing_evidence_gate_report_path = args.next().map(PathBuf::from);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let replay = read_json_value(&shadow_replay_report_path)?;
    let request = read_json_value(&billing_request_report_path)?;
    let evidence = billing_evidence_gate_report_path
        .as_deref()
        .map(read_json_value)
        .transpose()?;

    let replay_verdict = json_string(&replay, &["verdict"]).unwrap_or_default();
    let replay_unique_accepts =
        json_usize_path(&replay, &["future_unique_accepts_over_exact_cache"]).unwrap_or(0);
    let replay_tokens = json_usize_path(&replay, &["future_tokens_saved"]).unwrap_or(0);
    let replay_cost_microusd = json_u64(&replay, &["future_cost_saved_microusd"]).unwrap_or(0);
    let replay_false_accepts =
        json_usize_path(&replay, &["future_false_accepts"]).unwrap_or(usize::MAX);
    let replay_mismatches =
        json_usize_path(&replay, &["replay_mismatch_count"]).unwrap_or(usize::MAX);
    let replay_local_accept = json_bool(&replay, &["local_accept_enabled"]).unwrap_or(true);
    let replay_auto_promote = json_bool(&replay, &["auto_promote_enabled"]).unwrap_or(true);
    let replay_serving_registry_mutated =
        json_bool(&replay, &["serving_registry_mutated"]).unwrap_or(true);
    let replay_product_runtime_changed =
        json_bool(&replay, &["product_runtime_changed"]).unwrap_or(true);
    let replay_serving_runtime_changed =
        json_bool(&replay, &["serving_runtime_changed"]).unwrap_or(true);
    let replay_market_money_claim_allowed =
        json_bool(&replay, &["market_money_claim_allowed"]).unwrap_or(true);
    let replay_token_cost_estimate_used =
        json_bool(&replay, &["token_cost_estimate_used"]).unwrap_or(true);
    let replay_provider_billing_evidence_present =
        json_bool(&replay, &["provider_billing_evidence_present"]).unwrap_or(false);
    let replay_forbidden_flags_clear = forbidden_flags_all_bool_false(&replay);

    let request_verdict = json_string(&request, &["verdict"]).unwrap_or_default();
    let request_rows = json_usize_path(&request, &["billing_request_rows"]).unwrap_or(0);
    let request_tokens =
        json_usize_path(&request, &["total_tokens_requiring_billing"]).unwrap_or(0);
    let request_known_cost = json_u64(&request, &["current_known_cost_microusd"]).unwrap_or(0);
    let request_provider_ready_rows =
        json_usize_path(&request, &["provider_correlation_ready_rows"]).unwrap_or(0);
    let request_accept_parity = json_bool(&request, &["accept_parity"]).unwrap_or(false);
    let request_token_parity = json_bool(&request, &["token_parity"]).unwrap_or(false);
    let request_token_cost_estimate_used =
        json_bool(&request, &["token_cost_estimate_used"]).unwrap_or(true);
    let request_provider_billing_evidence_present =
        json_bool(&request, &["provider_billing_evidence_present"]).unwrap_or(false);
    let request_local_accept = json_bool(&request, &["local_accept_enabled"]).unwrap_or(true);
    let request_auto_promote = json_bool(&request, &["auto_promote_enabled"]).unwrap_or(true);
    let request_market_money_claim_allowed =
        json_bool(&request, &["market_money_claim_allowed"]).unwrap_or(true);
    let request_forbidden_flags_clear = forbidden_flags_all_bool_false(&request);

    let evidence_report_kind = evidence
        .as_ref()
        .and_then(|value| json_string(value, &["report_kind"]));
    let evidence_verdict = evidence
        .as_ref()
        .and_then(|value| json_string(value, &["verdict"]));
    let rows_enriched_provider_cost = evidence
        .as_ref()
        .and_then(|value| json_usize_path(value, &["rows_enriched_provider_cost"]))
        .unwrap_or(0);
    let rows_enriched_provider_tokens = evidence
        .as_ref()
        .and_then(|value| json_usize_path(value, &["rows_enriched_provider_tokens"]))
        .unwrap_or(0);
    let provider_billing_cost_microusd = evidence
        .as_ref()
        .and_then(|value| json_u64(value, &["provider_cost_microusd"]))
        .unwrap_or(0);
    let provider_billing_total_tokens = evidence
        .as_ref()
        .and_then(|value| json_usize_path(value, &["provider_total_tokens"]))
        .unwrap_or(0);
    let evidence_provider_billing_present = evidence
        .as_ref()
        .and_then(|value| {
            json_bool(
                value,
                &["billing_gate", "provider_billing_evidence_present"],
            )
            .or_else(|| json_bool(value, &["provider_billing_evidence_present"]))
        })
        .unwrap_or(false);
    let evidence_market_money_claim_allowed = evidence
        .as_ref()
        .and_then(|value| json_bool(value, &["market_money_claim_allowed"]))
        .unwrap_or(false);

    let runtime_gate_clean = replay_verdict
        == "PHASE_STREAM_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_V1_PASS_RUNTIME_REPLAY"
        && replay_unique_accepts > 0
        && replay_tokens > 0
        && replay_false_accepts == 0
        && replay_mismatches == 0
        && !replay_local_accept
        && !replay_auto_promote
        && !replay_serving_registry_mutated
        && !replay_product_runtime_changed
        && !replay_serving_runtime_changed
        && !replay_market_money_claim_allowed
        && !replay_provider_billing_evidence_present
        && replay_forbidden_flags_clear;
    let billing_request_gate_clean = request_verdict
        == "PHASE_STREAM_SELECTED_SPLIT_NWPC_BILLING_REQUEST_V1_READY"
        && request_rows == replay_unique_accepts
        && request_rows > 0
        && request_tokens == replay_tokens
        && request_known_cost == replay_cost_microusd
        && request_provider_ready_rows == request_rows
        && request_accept_parity
        && request_token_parity
        && !request_provider_billing_evidence_present
        && !request_local_accept
        && !request_auto_promote
        && !request_market_money_claim_allowed
        && request_forbidden_flags_clear;
    let provider_billing_evidence_complete = evidence_provider_billing_present
        && rows_enriched_provider_cost >= request_rows
        && rows_enriched_provider_tokens >= request_rows
        && provider_billing_cost_microusd > 0
        && provider_billing_total_tokens > 0
        && evidence_market_money_claim_allowed;

    let shadow_admission_candidate_allowed = runtime_gate_clean && billing_request_gate_clean;
    let calls_tokens_claim_allowed = shadow_admission_candidate_allowed;
    let market_money_claim_allowed =
        shadow_admission_candidate_allowed && provider_billing_evidence_complete;
    let product_promotion_allowed = market_money_claim_allowed;

    let mut blockers = Vec::<&'static str>::new();
    if !runtime_gate_clean {
        blockers.push("runtime_shadow_replay_gate_not_clean");
    }
    if !billing_request_gate_clean {
        blockers.push("billing_request_gate_not_clean");
    }
    if !provider_billing_evidence_complete {
        blockers.push("provider_billing_evidence_missing_or_incomplete");
    }

    let verdict = if product_promotion_allowed {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_ADMISSION_GATE_V1_PASS_MARKET_READY"
    } else if shadow_admission_candidate_allowed {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_ADMISSION_GATE_V1_SHADOW_READY_MONEY_BLOCKED"
    } else {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_ADMISSION_GATE_V1_WATCH"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_selected_split_nwpc_admission_gate_v1",
        "mode": "shadow_admission_and_money_boundary_only",
        "shadow_replay_report_path": shadow_replay_report_path,
        "billing_request_report_path": billing_request_report_path,
        "billing_evidence_gate_report_path": billing_evidence_gate_report_path,
        "runtime_gate": {
            "runtime_report_kind": json_string(&replay, &["report_kind"]),
            "runtime_replay_pass": runtime_gate_clean,
            "future_unique_accepts_over_exact_cache": replay_unique_accepts,
            "future_tokens_saved": replay_tokens,
            "future_cost_saved_microusd": replay_cost_microusd,
            "future_false_accepts": replay_false_accepts,
            "replay_mismatch_count": replay_mismatches,
            "local_accept_enabled": replay_local_accept,
            "auto_promote_enabled": replay_auto_promote,
            "serving_registry_mutated": replay_serving_registry_mutated,
            "product_runtime_changed": replay_product_runtime_changed,
            "serving_runtime_changed": replay_serving_runtime_changed,
            "token_cost_estimate_used": replay_token_cost_estimate_used,
            "provider_billing_evidence_present": replay_provider_billing_evidence_present,
            "forbidden_flags_clear": replay_forbidden_flags_clear
        },
        "billing_request_gate": {
            "billing_request_report_kind": json_string(&request, &["report_kind"]),
            "billing_request_ready": billing_request_gate_clean,
            "billing_request_rows": request_rows,
            "total_tokens_requiring_billing": request_tokens,
            "current_known_cost_microusd": request_known_cost,
            "provider_correlation_ready_rows": request_provider_ready_rows,
            "accept_parity": request_accept_parity,
            "token_parity": request_token_parity,
            "token_cost_estimate_used": request_token_cost_estimate_used,
            "provider_billing_evidence_present": request_provider_billing_evidence_present,
            "forbidden_flags_clear": request_forbidden_flags_clear
        },
        "billing_evidence_gate": {
            "billing_report_kind": evidence_report_kind,
            "billing_verdict": evidence_verdict,
            "provider_billing_evidence_present": evidence_provider_billing_present,
            "rows_enriched_provider_cost": rows_enriched_provider_cost,
            "rows_enriched_provider_tokens": rows_enriched_provider_tokens,
            "provider_billing_cost_microusd": provider_billing_cost_microusd,
            "provider_billing_total_tokens": provider_billing_total_tokens,
            "provider_billing_evidence_complete": provider_billing_evidence_complete,
            "policy": "market money claim requires external provider billing evidence covering every selected split runtime-replayed request; internal token estimates and request templates are not evidence"
        },
        "admission": {
            "shadow_admission_candidate_allowed": shadow_admission_candidate_allowed,
            "calls_tokens_claim_allowed": calls_tokens_claim_allowed,
            "market_money_claim_allowed": market_money_claim_allowed,
            "product_promotion_allowed": product_promotion_allowed,
            "blockers": blockers,
            "claim_boundary": "calls/tokens are runtime-replayed shadow evidence; market money remains blocked until external provider billing evidence completes the selected request rows"
        },
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
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "serving_registry_mutated": false,
        "shadow_registry_mutated": false,
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
        "product_promotion_allowed": product_promotion_allowed,
        "market_money_claim_allowed": market_money_claim_allowed,
        "verdict": verdict,
        "boundary": "selected split nwpc admission gate only: consumes runtime replay, billing request, and optional external billing evidence reports; does not compile, mine, promote, serve, mutate registry, enable local_accept, estimate missing money, or use legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_selected_split_nwpc_admission_gate_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  shadow_admission_candidate_allowed: {shadow_admission_candidate_allowed}");
    println!("  calls_tokens_claim_allowed: {calls_tokens_claim_allowed}");
    println!("  market_money_claim_allowed: {market_money_claim_allowed}");
    println!("  product_promotion_allowed: {product_promotion_allowed}");
    println!("  verdict: {verdict}");
    Ok(())
}

fn forbidden_flags_all_bool_false(value: &Value) -> bool {
    let Some(flags) = value.get("forbidden_flags").and_then(Value::as_object) else {
        return false;
    };
    !flags.is_empty() && flags.values().all(|flag| flag.as_bool() == Some(false))
}

fn json_usize_path(value: &Value, path: &[&str]) -> Option<usize> {
    json_u64(value, path).and_then(|number| usize::try_from(number).ok())
}
