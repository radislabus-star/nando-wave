use std::path::PathBuf;

use super::{json_bool, json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_TARGETED_AGGREGATE_ADMISSION_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-aggregate-admission-gate-v1.report.json";
const DEFAULT_TARGETED_AGGREGATE_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-aggregate-gate-v1-agent-followup-12k-current.report.json";
const DEFAULT_TARGETED_AGGREGATE_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-aggregate-billing-request-v1-agent-followup-12k-current.report.json";
const DEFAULT_TARGETED_AGGREGATE_BILLING_EVIDENCE_GATE_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-aggregate-billing-evidence-gate-v1-agent-followup-12k-current-template-negative.report.json";

pub(crate) fn run_phase_stream_online_miner_targeted_aggregate_admission_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_AGGREGATE_ADMISSION_REPORT));
    let aggregate_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_AGGREGATE_REPORT));
    let billing_request_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_AGGREGATE_BILLING_REQUEST_REPORT));
    let billing_evidence_gate_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_AGGREGATE_BILLING_EVIDENCE_GATE_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let aggregate = read_json_value(&aggregate_report_path)?;
    let billing = read_json_value(&billing_request_report_path)?;
    let evidence = read_json_value(&billing_evidence_gate_report_path)?;

    let aggregate_ready = json_string(&aggregate, &["verdict"]).as_deref()
        == Some(
            "PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_GATE_V1_PASS_CALLS_TOKENS_MONEY_BLOCKED",
        )
        && json_bool(&aggregate, &["calls_tokens_claim_allowed"]) == Some(true)
        && json_bool(&aggregate, &["local_accept_enabled"]) == Some(false)
        && json_bool(&aggregate, &["market_money_claim_allowed"]) == Some(false)
        && forbidden_flags_clear(&aggregate);
    let billing_ready = json_string(&billing, &["verdict"]).as_deref()
        == Some("PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_BILLING_REQUEST_V1_READY")
        && json_bool(&billing, &["accept_parity"]) == Some(true)
        && json_bool(&billing, &["token_parity"]) == Some(true)
        && json_bool(&billing, &["local_accept_enabled"]) == Some(false)
        && json_bool(&billing, &["market_money_claim_allowed"]) == Some(false)
        && forbidden_flags_clear(&billing);
    let aggregate_accepts =
        json_u64(&aggregate, &["aggregate_unique_accepts_over_exact_cache"]).unwrap_or_default();
    let aggregate_tokens = json_u64(&aggregate, &["aggregate_tokens_saved"]).unwrap_or_default();
    let billing_rows = json_u64(&billing, &["billing_request_rows"]).unwrap_or_default();
    let billing_tokens =
        json_u64(&billing, &["total_tokens_requiring_billing"]).unwrap_or_default();
    let request_set_parity =
        aggregate_accepts == billing_rows && aggregate_tokens == billing_tokens;

    let provider_billing_evidence_present =
        json_bool(&evidence, &["provider_billing_evidence_present"]).unwrap_or(false);
    let evidence_complete = json_string(&evidence, &["verdict"]).as_deref()
        == Some("PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_GATE_V1_PASS")
        && provider_billing_evidence_present
        && json_u64(&evidence, &["rows_enriched_provider_cost"]).unwrap_or_default()
            == billing_rows
        && json_bool(&evidence, &["market_money_claim_allowed"]) == Some(true)
        && json_bool(&evidence, &["local_accept_enabled"]) == Some(false);

    let shadow_admission_candidate_allowed =
        aggregate_ready && billing_ready && request_set_parity && aggregate_accepts > 0;
    let calls_tokens_claim_allowed = shadow_admission_candidate_allowed;
    let market_money_claim_allowed = shadow_admission_candidate_allowed && evidence_complete;
    let product_promotion_allowed = false;
    let local_accept_enabled = false;
    let blocker = if !aggregate_ready {
        "aggregate_gate_not_ready"
    } else if !billing_ready {
        "billing_request_not_ready"
    } else if !request_set_parity {
        "aggregate_billing_request_set_mismatch"
    } else if !evidence_complete {
        "provider_billing_evidence_missing_or_incomplete"
    } else {
        "none"
    };
    let verdict = if shadow_admission_candidate_allowed && !market_money_claim_allowed {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_ADMISSION_GATE_V1_SHADOW_READY_MONEY_BLOCKED"
    } else if shadow_admission_candidate_allowed && market_money_claim_allowed {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_ADMISSION_GATE_V1_MONEY_READY_PROMOTION_DISABLED"
    } else {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_ADMISSION_GATE_V1_WATCH"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_targeted_aggregate_admission_gate_v1",
        "aggregate_report_path": aggregate_report_path,
        "billing_request_report_path": billing_request_report_path,
        "billing_evidence_gate_report_path": billing_evidence_gate_report_path,
        "aggregate_ready": aggregate_ready,
        "billing_request_ready": billing_ready,
        "request_set_parity": request_set_parity,
        "provider_billing_evidence_present": provider_billing_evidence_present,
        "provider_billing_evidence_complete": evidence_complete,
        "aggregate_unique_accepts_over_exact_cache": aggregate_accepts,
        "aggregate_tokens_saved": aggregate_tokens,
        "billing_request_rows": billing_rows,
        "billing_request_tokens": billing_tokens,
        "shadow_admission_candidate_allowed": shadow_admission_candidate_allowed,
        "calls_tokens_claim_allowed": calls_tokens_claim_allowed,
        "market_money_claim_allowed": market_money_claim_allowed,
        "product_promotion_allowed": product_promotion_allowed,
        "local_accept_enabled": local_accept_enabled,
        "auto_promote_enabled": false,
        "serving_registry_mutated": false,
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
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
        "verdict": verdict,
        "blocker": blocker,
        "boundary": "aggregate admission gate only: ties aggregate calls/tokens proof to billing evidence status; does not promote, serve, mutate registry, enable local_accept, claim money without provider evidence, or use legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_targeted_aggregate_admission_gate_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  aggregate_unique_accepts_over_exact_cache: {aggregate_accepts}");
    println!("  aggregate_tokens_saved: {aggregate_tokens}");
    println!("  shadow_admission_candidate_allowed: {shadow_admission_candidate_allowed}");
    println!("  market_money_claim_allowed: {market_money_claim_allowed}");
    println!("  product_promotion_allowed: {product_promotion_allowed}");
    println!("  local_accept_enabled: {local_accept_enabled}");
    println!("  verdict: {verdict}");
    println!("  blocker: {blocker}");
    Ok(())
}

fn forbidden_flags_clear(value: &serde_json::Value) -> bool {
    let Some(flags) = value
        .get("forbidden_flags")
        .and_then(serde_json::Value::as_object)
    else {
        return false;
    };
    !flags.is_empty() && flags.values().all(|value| value.as_bool() == Some(false))
}
