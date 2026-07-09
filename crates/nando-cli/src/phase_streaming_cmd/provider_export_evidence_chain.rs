use std::path::PathBuf;

use super::online_portfolio_billing_evidence_gate::run_phase_stream_online_miner_portfolio_billing_evidence_gate_v1;
use super::online_portfolio_billing_request_provider_correlation_backfill::run_phase_stream_online_miner_portfolio_billing_request_provider_correlation_backfill_v1;
use super::online_portfolio_provider_export_normalize::run_phase_stream_online_miner_portfolio_provider_export_normalize_v1;
use super::provider_boundary_export_ingest::run_phase_stream_provider_boundary_export_ingest_v1;
use super::selected_split_nwpc_provider_export_attestation::{
    provider_export_fingerprint64, review_provider_export_attestation,
};
use super::{json_bool, json_u64, read_json_value, write_json_file};

const DEFAULT_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-provider-export-evidence-chain-v1.report.json";
const DEFAULT_WORK_DIR: &str =
    "target/nando-wave/streaming/provider-export-evidence-chain-v1-current";
const DEFAULT_BILLING_REQUEST_JSONL: &str =
    "target/nando-wave/streaming/provider-export-acquisition-pack-v1-current/billing-request.jsonl";
const DEFAULT_CAPTURE_REQUEST_JSONL: &str = "target/nando-wave/streaming/provider-export-acquisition-pack-v1-current/provider-boundary-capture-request.jsonl";

pub(crate) fn run_phase_stream_provider_export_evidence_chain_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REPORT));
    let work_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_WORK_DIR));
    let billing_request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_BILLING_REQUEST_JSONL));
    let capture_request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CAPTURE_REQUEST_JSONL));
    let provider_export_jsonl_path = args.next().map(PathBuf::from);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    std::fs::create_dir_all(&work_dir).map_err(|error| {
        format!(
            "failed to create provider export evidence chain work dir '{}': {error}",
            work_dir.display()
        )
    })?;

    let provider_boundary_ingest_report_path =
        work_dir.join("provider-boundary-export-ingest.report.json");
    let provider_boundary_jsonl_path = work_dir.join("provider-boundary.provider.jsonl");
    let provider_correlation_backfill_report_path =
        work_dir.join("provider-correlation-backfill.report.json");
    let provider_correlated_billing_request_jsonl_path =
        work_dir.join("billing-request.provider-correlated.jsonl");
    let provider_normalize_report_path = work_dir.join("provider-export-normalize.report.json");
    let normalized_evidence_jsonl_path = work_dir.join("provider-export-normalized.evidence.jsonl");
    let billing_evidence_gate_report_path = work_dir.join("billing-evidence-gate.report.json");
    let missing_request_jsonl_path = work_dir.join("missing-billing-request.jsonl");

    if provider_export_jsonl_path.is_none() {
        let mut report = serde_json::Map::new();
        report.insert(
            "report_kind".to_owned(),
            serde_json::json!("phase_stream_provider_export_evidence_chain_v1"),
        );
        report.insert(
            "work_dir".to_owned(),
            serde_json::json!(work_dir.display().to_string()),
        );
        report.insert(
            "billing_request_jsonl_path".to_owned(),
            serde_json::json!(billing_request_jsonl_path.display().to_string()),
        );
        report.insert(
            "provider_boundary_capture_request_jsonl_path".to_owned(),
            serde_json::json!(capture_request_jsonl_path.display().to_string()),
        );
        report.insert(
            "provider_export_jsonl_path".to_owned(),
            serde_json::json!(Option::<String>::None),
        );
        report.insert(
            "provider_export_required".to_owned(),
            serde_json::json!(true),
        );
        report.insert(
            "provider_billing_evidence_present".to_owned(),
            serde_json::json!(false),
        );
        report.insert(
            "provider_boundary_capture_ready".to_owned(),
            serde_json::json!(false),
        );
        report.insert(
            "provider_export_normalize_ready".to_owned(),
            serde_json::json!(false),
        );
        report.insert(
            "billing_gate_money_ready".to_owned(),
            serde_json::json!(false),
        );
        report.insert(
            "provider_export_attestation_valid".to_owned(),
            serde_json::json!(false),
        );
        report.insert(
            "external_evidence_chain_ready".to_owned(),
            serde_json::json!(false),
        );
        report.insert(
            "market_money_claim_allowed".to_owned(),
            serde_json::json!(false),
        );
        report.insert("local_accept_enabled".to_owned(), serde_json::json!(false));
        report.insert(
            "blockers".to_owned(),
            serde_json::json!(["external_provider_export_missing"]),
        );
        report.insert(
            "next_required_external_step".to_owned(),
            serde_json::json!({
                "need_real_external_provider_export": true,
                "need_provider_id_or_call_id": true,
                "need_positive_provider_cost_and_tokens": true,
                "need_attestation": true
            }),
        );
        report.insert(
            "verdict".to_owned(),
            serde_json::json!(
                "PHASE_STREAM_PROVIDER_EXPORT_EVIDENCE_CHAIN_V1_WATCH_EXTERNAL_PROVIDER_EXPORT_MISSING"
            ),
        );
        report.insert(
            "boundary".to_owned(),
            serde_json::json!("provider export evidence chain readiness only: no export file was supplied, so no evidence, money, serving, promotion, or local_accept claim is allowed"),
        );
        let report = serde_json::Value::Object(report);
        write_json_file(&report_path, &report)?;
        println!("phase_stream_provider_export_evidence_chain_v1:");
        println!("  report_path: {}", report_path.display());
        println!("  provider_export_required: true");
        println!("  market_money_claim_allowed: false");
        println!(
            "  verdict: PHASE_STREAM_PROVIDER_EXPORT_EVIDENCE_CHAIN_V1_WATCH_EXTERNAL_PROVIDER_EXPORT_MISSING"
        );
        return Ok(());
    }
    let provider_export_jsonl_path = provider_export_jsonl_path.expect("checked");

    run_phase_stream_provider_boundary_export_ingest_v1(
        vec![
            provider_boundary_ingest_report_path.display().to_string(),
            provider_boundary_jsonl_path.display().to_string(),
            capture_request_jsonl_path.display().to_string(),
            provider_export_jsonl_path.display().to_string(),
        ]
        .into_iter(),
    )?;
    run_phase_stream_online_miner_portfolio_billing_request_provider_correlation_backfill_v1(
        vec![
            provider_correlation_backfill_report_path
                .display()
                .to_string(),
            provider_correlated_billing_request_jsonl_path
                .display()
                .to_string(),
            billing_request_jsonl_path.display().to_string(),
            provider_boundary_jsonl_path.display().to_string(),
        ]
        .into_iter(),
    )?;
    run_phase_stream_online_miner_portfolio_provider_export_normalize_v1(
        vec![
            provider_normalize_report_path.display().to_string(),
            provider_correlated_billing_request_jsonl_path
                .display()
                .to_string(),
            provider_export_jsonl_path.display().to_string(),
            normalized_evidence_jsonl_path.display().to_string(),
        ]
        .into_iter(),
    )?;
    run_phase_stream_online_miner_portfolio_billing_evidence_gate_v1(
        vec![
            billing_evidence_gate_report_path.display().to_string(),
            provider_correlated_billing_request_jsonl_path
                .display()
                .to_string(),
            normalized_evidence_jsonl_path.display().to_string(),
            missing_request_jsonl_path.display().to_string(),
        ]
        .into_iter(),
    )?;

    let provider_boundary_ingest = read_json_value(&provider_boundary_ingest_report_path)?;
    let provider_correlation_backfill =
        read_json_value(&provider_correlation_backfill_report_path)?;
    let provider_normalize = read_json_value(&provider_normalize_report_path)?;
    let billing_evidence_gate = read_json_value(&billing_evidence_gate_report_path)?;
    let provider_export_fingerprint64 = provider_export_fingerprint64(&provider_export_jsonl_path)?;
    let attestation = review_provider_export_attestation(
        &provider_export_jsonl_path,
        provider_export_fingerprint64,
    )?;

    let provider_billing_evidence_present = json_bool(
        &billing_evidence_gate,
        &["provider_billing_evidence_present"],
    )
    .unwrap_or(false);
    let provider_boundary_capture_ready = json_bool(
        &provider_boundary_ingest,
        &["readiness", "capture_coverage_possible"],
    )
    .unwrap_or(false)
        && verdict_eq(
            &provider_boundary_ingest,
            "PHASE_STREAM_PROVIDER_BOUNDARY_EXPORT_INGEST_V1_READY_FOR_CAPTURE_COVERAGE_GATE",
        );
    let provider_export_normalize_ready = verdict_eq(
        &provider_normalize,
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_EXPORT_NORMALIZE_V1_READY_FOR_EVIDENCE_GATE",
    );
    let billing_gate_money = json_bool(
        &billing_evidence_gate,
        &["billing_gate", "market_money_claim_allowed"],
    )
    .unwrap_or(false);
    let provider_export_attestation_valid = attestation.valid;
    let market_money_claim_allowed = provider_boundary_capture_ready
        && provider_export_normalize_ready
        && provider_billing_evidence_present
        && billing_gate_money
        && provider_export_attestation_valid;
    let mut blockers = Vec::new();
    if !provider_boundary_capture_ready {
        blockers.push("provider_boundary_capture_not_ready");
    }
    if !provider_export_normalize_ready {
        blockers.push("provider_export_normalize_not_ready");
    }
    if !provider_billing_evidence_present || !billing_gate_money {
        blockers.push("provider_billing_evidence_not_ready");
    }
    if !provider_export_attestation_valid {
        blockers.push("provider_export_attestation_not_valid");
    }
    let verdict = if market_money_claim_allowed {
        "PHASE_STREAM_PROVIDER_EXPORT_EVIDENCE_CHAIN_V1_PASS_EXTERNAL_EVIDENCE_READY"
    } else if provider_billing_evidence_present {
        "PHASE_STREAM_PROVIDER_EXPORT_EVIDENCE_CHAIN_V1_WATCH_BOUNDARY_ATTESTATION_OR_ADMISSION_BLOCKED"
    } else {
        "PHASE_STREAM_PROVIDER_EXPORT_EVIDENCE_CHAIN_V1_WATCH_EVIDENCE_INCOMPLETE"
    };

    let mut report = serde_json::Map::new();
    report.insert(
        "report_kind".to_owned(),
        serde_json::json!("phase_stream_provider_export_evidence_chain_v1"),
    );
    report.insert(
        "work_dir".to_owned(),
        serde_json::json!(work_dir.display().to_string()),
    );
    report.insert(
        "billing_request_jsonl_path".to_owned(),
        serde_json::json!(billing_request_jsonl_path.display().to_string()),
    );
    report.insert(
        "provider_boundary_capture_request_jsonl_path".to_owned(),
        serde_json::json!(capture_request_jsonl_path.display().to_string()),
    );
    report.insert(
        "provider_export_jsonl_path".to_owned(),
        serde_json::json!(provider_export_jsonl_path.display().to_string()),
    );
    report.insert(
        "provider_export_fingerprint64".to_owned(),
        serde_json::json!(provider_export_fingerprint64),
    );
    report.insert(
        "provider_boundary_ingest_report_path".to_owned(),
        serde_json::json!(provider_boundary_ingest_report_path.display().to_string()),
    );
    report.insert(
        "provider_boundary_jsonl_path".to_owned(),
        serde_json::json!(provider_boundary_jsonl_path.display().to_string()),
    );
    report.insert(
        "provider_correlation_backfill_report_path".to_owned(),
        serde_json::json!(
            provider_correlation_backfill_report_path
                .display()
                .to_string()
        ),
    );
    report.insert(
        "provider_correlated_billing_request_jsonl_path".to_owned(),
        serde_json::json!(
            provider_correlated_billing_request_jsonl_path
                .display()
                .to_string()
        ),
    );
    report.insert(
        "provider_normalize_report_path".to_owned(),
        serde_json::json!(provider_normalize_report_path.display().to_string()),
    );
    report.insert(
        "normalized_evidence_jsonl_path".to_owned(),
        serde_json::json!(normalized_evidence_jsonl_path.display().to_string()),
    );
    report.insert(
        "billing_evidence_gate_report_path".to_owned(),
        serde_json::json!(billing_evidence_gate_report_path.display().to_string()),
    );
    report.insert(
        "missing_request_jsonl_path".to_owned(),
        serde_json::json!(missing_request_jsonl_path.display().to_string()),
    );
    report.insert(
        "provider_boundary_ingest".to_owned(),
        serde_json::json!({
            "verdict": provider_boundary_ingest.get("verdict"),
            "provider_export_rows": json_usize(&provider_boundary_ingest, &["provider_export_rows"]),
            "normalized_provider_boundary_rows": json_usize(&provider_boundary_ingest, &["normalized_provider_boundary_rows"]),
            "normalized_unique_capture_requests": json_usize(&provider_boundary_ingest, &["normalized_unique_capture_requests"]),
            "capture_coverage_possible": json_bool(&provider_boundary_ingest, &["readiness", "capture_coverage_possible"]).unwrap_or(false)
        }),
    );
    report.insert(
        "provider_correlation_backfill".to_owned(),
        serde_json::json!({
            "verdict": provider_correlation_backfill.get("verdict"),
            "billing_request_rows": json_usize(&provider_correlation_backfill, &["billing_request_rows"]),
            "rows_with_any_provider_correlation": json_usize(&provider_correlation_backfill, &["rows_with_any_provider_correlation"]),
            "provider_request_id_ready_rows": json_usize(&provider_correlation_backfill, &["provider_request_id_ready_rows"])
        }),
    );
    report.insert(
        "provider_normalize".to_owned(),
        serde_json::json!({
            "verdict": provider_normalize.get("verdict"),
            "provider_export_rows": json_usize(&provider_normalize, &["provider_export_rows"]),
            "normalized_evidence_rows": json_usize(&provider_normalize, &["normalized_evidence_rows"]),
            "normalized_matched_request_rows": json_usize(&provider_normalize, &["normalized_matched_request_rows"]),
            "ready_for_evidence_gate": provider_export_normalize_ready
        }),
    );
    report.insert(
        "billing_evidence_gate".to_owned(),
        serde_json::json!({
            "verdict": billing_evidence_gate.get("verdict"),
            "provider_billing_evidence_present": provider_billing_evidence_present,
            "request_rows": json_usize(&billing_evidence_gate, &["request_rows"]),
            "rows_enriched_provider_cost": json_usize(&billing_evidence_gate, &["rows_enriched_provider_cost"]),
            "rows_enriched_provider_tokens": json_usize(&billing_evidence_gate, &["rows_enriched_provider_tokens"]),
            "missing_billing_request_rows": json_usize(&billing_evidence_gate, &["missing_billing_request_rows"]),
            "provider_cost_microusd": json_u64(&billing_evidence_gate, &["provider_cost_microusd"]).unwrap_or(0),
            "provider_total_tokens": json_usize(&billing_evidence_gate, &["provider_total_tokens"])
        }),
    );
    report.insert(
        "provider_export_attestation".to_owned(),
        serde_json::json!(attestation),
    );
    report.insert(
        "provider_billing_evidence_present".to_owned(),
        serde_json::json!(provider_billing_evidence_present),
    );
    report.insert(
        "provider_export_attestation_valid".to_owned(),
        serde_json::json!(provider_export_attestation_valid),
    );
    report.insert(
        "provider_boundary_capture_ready".to_owned(),
        serde_json::json!(provider_boundary_capture_ready),
    );
    report.insert(
        "provider_export_normalize_ready".to_owned(),
        serde_json::json!(provider_export_normalize_ready),
    );
    report.insert(
        "billing_gate_money_ready".to_owned(),
        serde_json::json!(billing_gate_money),
    );
    report.insert(
        "external_evidence_chain_ready".to_owned(),
        serde_json::json!(market_money_claim_allowed),
    );
    report.insert("blockers".to_owned(), serde_json::json!(blockers));
    report.insert("local_accept_enabled".to_owned(), serde_json::json!(false));
    report.insert("auto_promote_enabled".to_owned(), serde_json::json!(false));
    report.insert(
        "serving_registry_mutated".to_owned(),
        serde_json::json!(false),
    );
    report.insert(
        "market_money_claim_allowed".to_owned(),
        serde_json::json!(market_money_claim_allowed),
    );
    report.insert(
        "forbidden_flags".to_owned(),
        serde_json::json!({
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        }),
    );
    report.insert("verdict".to_owned(), serde_json::json!(verdict));
    report.insert(
        "boundary".to_owned(),
        serde_json::json!("provider export evidence chain only: joins external provider export to verifier-bound .nwpc billing requests through provider-boundary metadata and billing evidence gates; does not compile, mine, serve, mutate registry, enable local_accept, or estimate missing money"),
    );
    let report = serde_json::Value::Object(report);
    write_json_file(&report_path, &report)?;

    println!("phase_stream_provider_export_evidence_chain_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  provider_billing_evidence_present: {provider_billing_evidence_present}");
    println!("  market_money_claim_allowed: {market_money_claim_allowed}");
    println!("  local_accept_enabled: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn json_usize(value: &serde_json::Value, path: &[&str]) -> Option<usize> {
    json_u64(value, path).and_then(|value| usize::try_from(value).ok())
}

fn verdict_eq(value: &serde_json::Value, expected: &str) -> bool {
    value
        .get("verdict")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|verdict| verdict == expected)
}
