use std::path::PathBuf;

use super::online_miner_targeted_aggregate_admission_gate::run_phase_stream_online_miner_targeted_aggregate_admission_gate_v1;
use super::online_miner_targeted_aggregate_billing_request::run_phase_stream_online_miner_targeted_aggregate_billing_request_v1;
use super::online_portfolio_billing_evidence_gate::run_phase_stream_online_miner_portfolio_billing_evidence_gate_v1;
use super::online_portfolio_provider_export_normalize::run_phase_stream_online_miner_portfolio_provider_export_normalize_v1;
use super::selected_split_nwpc_provider_export_attestation::{
    provider_export_fingerprint64, review_provider_export_attestation,
};
use super::{json_bool, json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_TARGETED_AGGREGATE_PROVIDER_EXPORT_ADMISSION_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-aggregate-provider-export-admission-v1.report.json";
const DEFAULT_TARGETED_AGGREGATE_PROVIDER_EXPORT_ADMISSION_WORK_DIR: &str =
    "target/nando-wave/streaming/targeted-aggregate-provider-export-admission-v1";
const DEFAULT_TARGETED_AGGREGATE_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-aggregate-gate-v1-agent-followup-12k-current.report.json";

pub(crate) fn run_phase_stream_online_miner_targeted_aggregate_provider_export_admission_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_TARGETED_AGGREGATE_PROVIDER_EXPORT_ADMISSION_REPORT)
    });
    let provider_export_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "external provider export JSONL/CSV/TSV path is required".to_owned())?;
    let work_dir = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_TARGETED_AGGREGATE_PROVIDER_EXPORT_ADMISSION_WORK_DIR)
    });
    let aggregate_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_AGGREGATE_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    std::fs::create_dir_all(&work_dir).map_err(|error| {
        format!(
            "failed to create targeted aggregate provider export admission work dir '{}': {error}",
            work_dir.display()
        )
    })?;

    let billing_request_report_path = work_dir.join("aggregate-billing-request.report.json");
    let billing_request_jsonl_path = work_dir.join("aggregate-billing-request.jsonl");
    let provider_normalize_report_path = work_dir.join("provider-export-normalize.report.json");
    let normalized_evidence_jsonl_path = work_dir.join("provider-export-normalized.evidence.jsonl");
    let billing_evidence_gate_report_path = work_dir.join("billing-evidence-gate.report.json");
    let missing_request_jsonl_path = work_dir.join("missing-billing-request.jsonl");
    let aggregate_admission_report_path = work_dir.join("aggregate-admission-gate.report.json");

    run_phase_stream_online_miner_targeted_aggregate_billing_request_v1(
        vec![
            billing_request_report_path.display().to_string(),
            billing_request_jsonl_path.display().to_string(),
            aggregate_report_path.display().to_string(),
        ]
        .into_iter(),
    )?;
    run_phase_stream_online_miner_portfolio_provider_export_normalize_v1(
        vec![
            provider_normalize_report_path.display().to_string(),
            billing_request_jsonl_path.display().to_string(),
            provider_export_jsonl_path.display().to_string(),
            normalized_evidence_jsonl_path.display().to_string(),
        ]
        .into_iter(),
    )?;
    run_phase_stream_online_miner_portfolio_billing_evidence_gate_v1(
        vec![
            billing_evidence_gate_report_path.display().to_string(),
            billing_request_jsonl_path.display().to_string(),
            normalized_evidence_jsonl_path.display().to_string(),
            missing_request_jsonl_path.display().to_string(),
        ]
        .into_iter(),
    )?;
    run_phase_stream_online_miner_targeted_aggregate_admission_gate_v1(
        vec![
            aggregate_admission_report_path.display().to_string(),
            aggregate_report_path.display().to_string(),
            billing_request_report_path.display().to_string(),
            billing_evidence_gate_report_path.display().to_string(),
        ]
        .into_iter(),
    )?;

    let billing_request = read_json_value(&billing_request_report_path)?;
    let provider_normalize = read_json_value(&provider_normalize_report_path)?;
    let billing_evidence_gate = read_json_value(&billing_evidence_gate_report_path)?;
    let aggregate_admission = read_json_value(&aggregate_admission_report_path)?;
    let provider_export_fingerprint64 = provider_export_fingerprint64(&provider_export_jsonl_path)?;
    let provider_export_attestation = review_provider_export_attestation(
        &provider_export_jsonl_path,
        provider_export_fingerprint64,
    )?;

    let billing_request_ready = json_string(&billing_request, &["verdict"]).as_deref()
        == Some("PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_BILLING_REQUEST_V1_READY");
    let normalize_ready = json_string(&provider_normalize, &["verdict"]).as_deref()
        == Some(
            "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_EXPORT_NORMALIZE_V1_READY_FOR_EVIDENCE_GATE",
        );
    let evidence_ready = json_bool(
        &billing_evidence_gate,
        &["provider_billing_evidence_present"],
    )
    .unwrap_or(false);
    let aggregate_shadow_ready = json_bool(
        &aggregate_admission,
        &["shadow_admission_candidate_allowed"],
    )
    .unwrap_or(false);
    let aggregate_money_ready =
        json_bool(&aggregate_admission, &["market_money_claim_allowed"]).unwrap_or(false);
    let provider_export_attestation_valid = provider_export_attestation.valid;
    let market_money_claim_allowed = billing_request_ready
        && evidence_ready
        && aggregate_money_ready
        && provider_export_attestation_valid;
    let product_promotion_allowed = false;
    let local_accept_enabled = false;
    let blocker = if !billing_request_ready {
        "aggregate_billing_request_not_ready"
    } else if !normalize_ready {
        "provider_export_not_fully_normalized"
    } else if !evidence_ready {
        "provider_billing_evidence_missing_or_incomplete"
    } else if !aggregate_shadow_ready {
        "aggregate_admission_not_shadow_ready"
    } else if !aggregate_money_ready {
        "aggregate_admission_money_blocked"
    } else if !provider_export_attestation_valid {
        "provider_export_attestation_missing_or_invalid"
    } else {
        "none"
    };
    let verdict = if market_money_claim_allowed {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_PROVIDER_EXPORT_ADMISSION_V1_MONEY_READY_PROMOTION_DISABLED"
    } else {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_PROVIDER_EXPORT_ADMISSION_V1_MONEY_BLOCKED"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_targeted_aggregate_provider_export_admission_v1",
        "provider_export_jsonl_path": provider_export_jsonl_path,
        "work_dir": work_dir,
        "aggregate_report_path": aggregate_report_path,
        "billing_request_report_path": billing_request_report_path,
        "billing_request_jsonl_path": billing_request_jsonl_path,
        "provider_normalize_report_path": provider_normalize_report_path,
        "normalized_evidence_jsonl_path": normalized_evidence_jsonl_path,
        "billing_evidence_gate_report_path": billing_evidence_gate_report_path,
        "missing_request_jsonl_path": missing_request_jsonl_path,
        "aggregate_admission_report_path": aggregate_admission_report_path,
        "provider_export_fingerprint64": provider_export_fingerprint64,
        "provider_export_attestation_required_for_money_claim": true,
        "provider_export_attestation": provider_export_attestation,
        "billing_request": {
            "verdict": json_string(&billing_request, &["verdict"]),
            "billing_request_rows": json_u64(&billing_request, &["billing_request_rows"]).unwrap_or(0),
            "total_tokens_requiring_billing": json_u64(&billing_request, &["total_tokens_requiring_billing"]).unwrap_or(0),
            "accept_parity": json_bool(&billing_request, &["accept_parity"]).unwrap_or(false),
            "token_parity": json_bool(&billing_request, &["token_parity"]).unwrap_or(false),
            "ready": billing_request_ready
        },
        "provider_normalize": {
            "verdict": json_string(&provider_normalize, &["verdict"]),
            "provider_export_format": json_string(&provider_normalize, &["provider_export_format"]),
            "provider_export_rows": json_u64(&provider_normalize, &["provider_export_rows"]).unwrap_or(0),
            "normalized_evidence_rows": json_u64(&provider_normalize, &["normalized_evidence_rows"]).unwrap_or(0),
            "normalized_matched_request_rows": json_u64(&provider_normalize, &["normalized_matched_request_rows"]).unwrap_or(0),
            "missing_request_rows": json_u64(&provider_normalize, &["missing_request_rows"]).unwrap_or(0),
            "ready": normalize_ready
        },
        "billing_evidence_gate": {
            "verdict": json_string(&billing_evidence_gate, &["verdict"]),
            "provider_billing_evidence_present": evidence_ready,
            "request_rows": json_u64(&billing_evidence_gate, &["request_rows"]).unwrap_or(0),
            "rows_enriched_provider_cost": json_u64(&billing_evidence_gate, &["rows_enriched_provider_cost"]).unwrap_or(0),
            "rows_enriched_provider_tokens": json_u64(&billing_evidence_gate, &["rows_enriched_provider_tokens"]).unwrap_or(0),
            "missing_billing_request_rows": json_u64(&billing_evidence_gate, &["missing_billing_request_rows"]).unwrap_or(0),
            "provider_cost_microusd": json_u64(&billing_evidence_gate, &["provider_cost_microusd"]).unwrap_or(0),
            "provider_total_tokens": json_u64(&billing_evidence_gate, &["provider_total_tokens"]).unwrap_or(0)
        },
        "aggregate_admission": {
            "verdict": json_string(&aggregate_admission, &["verdict"]),
            "shadow_admission_candidate_allowed": aggregate_shadow_ready,
            "calls_tokens_claim_allowed": json_bool(&aggregate_admission, &["calls_tokens_claim_allowed"]).unwrap_or(false),
            "market_money_claim_allowed_before_attestation": aggregate_money_ready,
            "product_promotion_allowed": json_bool(&aggregate_admission, &["product_promotion_allowed"]).unwrap_or(false),
            "local_accept_enabled": json_bool(&aggregate_admission, &["local_accept_enabled"]).unwrap_or(true),
            "blocker": json_string(&aggregate_admission, &["blocker"])
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
        "provider_billing_evidence_present": evidence_ready,
        "provider_export_attestation_valid": provider_export_attestation_valid,
        "shadow_admission_candidate_allowed": aggregate_shadow_ready,
        "calls_tokens_claim_allowed": aggregate_shadow_ready,
        "market_money_claim_allowed": market_money_claim_allowed,
        "product_promotion_allowed": product_promotion_allowed,
        "local_accept_enabled": local_accept_enabled,
        "auto_promote_enabled": false,
        "serving_registry_mutated": false,
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
        "verdict": verdict,
        "blocker": blocker,
        "boundary": "targeted aggregate provider export admission only: normalizes an external provider export, validates billing evidence coverage, reruns aggregate admission, and requires adjacent provider-export attestation before money; does not compile, promote, serve, mutate registry, enable local_accept, estimate missing money, or use legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_targeted_aggregate_provider_export_admission_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  provider_billing_evidence_present: {evidence_ready}");
    println!("  provider_export_attestation_valid: {provider_export_attestation_valid}");
    println!("  shadow_admission_candidate_allowed: {aggregate_shadow_ready}");
    println!("  market_money_claim_allowed: {market_money_claim_allowed}");
    println!("  product_promotion_allowed: {product_promotion_allowed}");
    println!("  local_accept_enabled: {local_accept_enabled}");
    println!("  verdict: {verdict}");
    println!("  blocker: {blocker}");
    Ok(())
}
