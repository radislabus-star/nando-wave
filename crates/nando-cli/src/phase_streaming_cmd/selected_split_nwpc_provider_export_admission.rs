use std::path::PathBuf;

use serde_json::Value;

use super::online_portfolio_billing_evidence_gate::run_phase_stream_online_miner_portfolio_billing_evidence_gate_v1;
use super::online_portfolio_provider_export_normalize::run_phase_stream_online_miner_portfolio_provider_export_normalize_v1;
use super::selected_split_nwpc_admission_gate::run_phase_stream_selected_split_nwpc_admission_gate_v1;
use super::selected_split_nwpc_provider_export_attestation::{
    provider_export_fingerprint64, review_provider_export_attestation,
};
use super::{json_bool, json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_SELECTED_SPLIT_PROVIDER_EXPORT_ADMISSION_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-provider-export-admission-v1.report.json";
const DEFAULT_SELECTED_SPLIT_PROVIDER_EXPORT_ADMISSION_WORK_DIR: &str =
    "target/nando-wave/streaming/selected-split-nwpc-provider-export-admission-v1";
const DEFAULT_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-shadow-replay-v1-realtrace-plus-verifier-sources.report.json";
const DEFAULT_SELECTED_SPLIT_NWPC_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-billing-request-v1-realtrace-plus-verifier-sources.report.json";
const DEFAULT_SELECTED_SPLIT_NWPC_BILLING_REQUEST_JSONL: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-billing-request-v1-realtrace-plus-verifier-sources.jsonl";

pub(crate) fn run_phase_stream_selected_split_nwpc_provider_export_admission_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_PROVIDER_EXPORT_ADMISSION_REPORT));
    let provider_export_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "external provider export JSONL path is required".to_owned())?;
    let work_dir = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_SELECTED_SPLIT_PROVIDER_EXPORT_ADMISSION_WORK_DIR)
    });
    let shadow_replay_report_path =
        next_path(&mut args, DEFAULT_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_REPORT);
    let billing_request_report_path = next_path(
        &mut args,
        DEFAULT_SELECTED_SPLIT_NWPC_BILLING_REQUEST_REPORT,
    );
    let billing_request_jsonl_path =
        next_path(&mut args, DEFAULT_SELECTED_SPLIT_NWPC_BILLING_REQUEST_JSONL);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    std::fs::create_dir_all(&work_dir).map_err(|error| {
        format!(
            "failed to create selected split provider export admission work dir '{}': {error}",
            work_dir.display()
        )
    })?;

    let provider_normalize_report_path = work_dir.join("provider-export-normalize.report.json");
    let normalized_evidence_jsonl_path = work_dir.join("provider-export-normalized.evidence.jsonl");
    let billing_evidence_gate_report_path = work_dir.join("billing-evidence-gate.report.json");
    let missing_request_jsonl_path = work_dir.join("missing-billing-request.jsonl");
    let admission_report_path = work_dir.join("admission-gate.report.json");

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
    run_phase_stream_selected_split_nwpc_admission_gate_v1(
        vec![
            admission_report_path.display().to_string(),
            shadow_replay_report_path.display().to_string(),
            billing_request_report_path.display().to_string(),
            billing_evidence_gate_report_path.display().to_string(),
        ]
        .into_iter(),
    )?;

    let provider_normalize = read_json_value(&provider_normalize_report_path)?;
    let billing_evidence_gate = read_json_value(&billing_evidence_gate_report_path)?;
    let admission = read_json_value(&admission_report_path)?;
    let provider_export_fingerprint64 = provider_export_fingerprint64(&provider_export_jsonl_path)?;
    let provider_export_attestation = review_provider_export_attestation(
        &provider_export_jsonl_path,
        provider_export_fingerprint64,
    )?;

    let provider_billing_evidence_present = json_bool(
        &billing_evidence_gate,
        &["provider_billing_evidence_present"],
    )
    .unwrap_or(false);
    let shadow_admission_candidate_allowed = json_bool(
        &admission,
        &["admission", "shadow_admission_candidate_allowed"],
    )
    .unwrap_or(false);
    let calls_tokens_claim_allowed =
        json_bool(&admission, &["admission", "calls_tokens_claim_allowed"]).unwrap_or(false);
    let provider_gate_product_promotion_allowed =
        json_bool(&admission, &["admission", "product_promotion_allowed"]).unwrap_or(false);
    let provider_gate_market_money_claim_allowed =
        json_bool(&admission, &["admission", "market_money_claim_allowed"]).unwrap_or(false);
    let product_promotion_allowed =
        provider_gate_product_promotion_allowed && provider_export_attestation.valid;
    let market_money_claim_allowed =
        provider_gate_market_money_claim_allowed && provider_export_attestation.valid;
    let local_accept_enabled = json_bool(&admission, &["local_accept_enabled"]).unwrap_or(true);
    let request_rows = json_usize_path(&billing_evidence_gate, &["request_rows"]).unwrap_or(0);
    let rows_enriched_provider_cost =
        json_usize_path(&billing_evidence_gate, &["rows_enriched_provider_cost"]).unwrap_or(0);
    let rows_enriched_provider_tokens =
        json_usize_path(&billing_evidence_gate, &["rows_enriched_provider_tokens"]).unwrap_or(0);
    let missing_billing_request_rows =
        json_usize_path(&billing_evidence_gate, &["missing_billing_request_rows"]).unwrap_or(0);
    let provider_cost_microusd =
        json_u64(&billing_evidence_gate, &["provider_cost_microusd"]).unwrap_or(0);
    let provider_total_tokens =
        json_usize_path(&billing_evidence_gate, &["provider_total_tokens"]).unwrap_or(0);
    let provider_export_format = json_string(&provider_normalize, &["provider_export_format"])
        .unwrap_or_else(|| "unknown".to_owned());
    let verdict = if product_promotion_allowed
        && market_money_claim_allowed
        && !local_accept_enabled
    {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_PROVIDER_EXPORT_ADMISSION_V1_READY_FOR_PRODUCT_REVIEW"
    } else if shadow_admission_candidate_allowed && calls_tokens_claim_allowed {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_PROVIDER_EXPORT_ADMISSION_V1_SHADOW_READY_MONEY_BLOCKED"
    } else {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_PROVIDER_EXPORT_ADMISSION_V1_BLOCKED"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_selected_split_nwpc_provider_export_admission_v1",
        "provider_export_jsonl_path": provider_export_jsonl_path,
        "work_dir": work_dir,
        "shadow_replay_report_path": shadow_replay_report_path,
        "billing_request_report_path": billing_request_report_path,
        "billing_request_jsonl_path": billing_request_jsonl_path,
        "provider_export_fingerprint64": provider_export_fingerprint64,
        "provider_export_attestation_required_for_money_claim": true,
        "provider_export_attestation": provider_export_attestation,
        "provider_normalize_report_path": provider_normalize_report_path,
        "normalized_evidence_jsonl_path": normalized_evidence_jsonl_path,
        "billing_evidence_gate_report_path": billing_evidence_gate_report_path,
        "missing_request_jsonl_path": missing_request_jsonl_path,
        "admission_report_path": admission_report_path,
        "provider_normalize": {
            "verdict": json_string(&provider_normalize, &["verdict"]),
            "provider_export_format": provider_export_format,
            "provider_export_rows": json_usize_path(&provider_normalize, &["provider_export_rows"]).unwrap_or(0),
            "normalized_evidence_rows": json_usize_path(&provider_normalize, &["normalized_evidence_rows"]).unwrap_or(0),
            "normalized_matched_request_rows": json_usize_path(&provider_normalize, &["normalized_matched_request_rows"]).unwrap_or(0)
        },
        "billing_evidence_gate": {
            "verdict": json_string(&billing_evidence_gate, &["verdict"]),
            "provider_billing_evidence_present": provider_billing_evidence_present,
            "request_rows": request_rows,
            "rows_enriched_provider_cost": rows_enriched_provider_cost,
            "rows_enriched_provider_tokens": rows_enriched_provider_tokens,
            "missing_billing_request_rows": missing_billing_request_rows,
            "missing_billing_request_jsonl_path": missing_request_jsonl_path,
            "provider_cost_microusd": provider_cost_microusd,
            "provider_total_tokens": provider_total_tokens
        },
        "admission": {
            "verdict": json_string(&admission, &["verdict"]),
            "shadow_admission_candidate_allowed": shadow_admission_candidate_allowed,
            "calls_tokens_claim_allowed": calls_tokens_claim_allowed,
            "provider_gate_product_promotion_allowed": provider_gate_product_promotion_allowed,
            "provider_gate_market_money_claim_allowed": provider_gate_market_money_claim_allowed,
            "product_promotion_allowed": product_promotion_allowed,
            "market_money_claim_allowed": market_money_claim_allowed,
            "blockers": admission
                .get("admission")
                .and_then(|value| value.get("blockers"))
                .cloned()
                .unwrap_or_else(|| serde_json::json!([]))
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
        "provider_billing_evidence_present": provider_billing_evidence_present,
        "shadow_admission_candidate_allowed": shadow_admission_candidate_allowed,
        "calls_tokens_claim_allowed": calls_tokens_claim_allowed,
        "provider_gate_product_promotion_allowed": provider_gate_product_promotion_allowed,
        "provider_gate_market_money_claim_allowed": provider_gate_market_money_claim_allowed,
        "product_promotion_allowed": product_promotion_allowed,
        "market_money_claim_allowed": market_money_claim_allowed,
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "serving_registry_mutated": false,
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
        "provider_export_format": provider_export_format,
        "request_rows": request_rows,
        "rows_enriched_provider_cost": rows_enriched_provider_cost,
        "rows_enriched_provider_tokens": rows_enriched_provider_tokens,
        "missing_billing_request_rows": missing_billing_request_rows,
        "verdict": verdict,
        "boundary": "selected split .nwpc provider export admission wrapper: normalizes external provider export rows, validates billing evidence, and reruns selected-split admission; it does not compile, mine, serve, mutate registry, enable local_accept, estimate missing money, or use legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_selected_split_nwpc_provider_export_admission_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  provider_billing_evidence_present: {provider_billing_evidence_present}");
    println!("  shadow_admission_candidate_allowed: {shadow_admission_candidate_allowed}");
    println!("  product_promotion_allowed: {product_promotion_allowed}");
    println!("  market_money_claim_allowed: {market_money_claim_allowed}");
    println!("  local_accept_enabled: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn next_path<I>(args: &mut I, default_path: &str) -> PathBuf
where
    I: Iterator<Item = String>,
{
    args.next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_path))
}

fn json_usize_path(value: &Value, path: &[&str]) -> Option<usize> {
    json_u64(value, path).and_then(|number| usize::try_from(number).ok())
}
