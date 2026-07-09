use std::path::{Path, PathBuf};

use serde_json::Value;

use super::online_portfolio_admission_gate::run_phase_stream_online_miner_portfolio_admission_gate_v1;
use super::online_portfolio_billing_evidence_gate::run_phase_stream_online_miner_portfolio_billing_evidence_gate_v1;
use super::online_portfolio_evidence_chain_audit::run_phase_stream_online_miner_portfolio_evidence_chain_audit_v1;
use super::online_portfolio_promotion_manifest::run_phase_stream_online_miner_portfolio_promotion_manifest_v1;
use super::online_portfolio_provider_export_normalize::run_phase_stream_online_miner_portfolio_provider_export_normalize_v1;
use super::selected_split_nwpc_provider_export_attestation::{
    provider_export_fingerprint64, review_provider_export_attestation,
};

const DEFAULT_PROVIDER_EXPORT_ADMISSION_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-provider-export-admission-v1.report.json";
const DEFAULT_PROVIDER_EXPORT_ADMISSION_WORK_DIR: &str =
    "target/nando-wave/streaming/provider-export-admission-v1";
const DEFAULT_RUNTIME_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-runtime-replay-v1-autogate-v28.report.json";
const DEFAULT_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-request-v1-autogate-v28.report.json";
const DEFAULT_BILLING_REQUEST_JSONL: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-request-v1-autogate-v28.jsonl";
const DEFAULT_BILLING_CONTRACT_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-evidence-contract-v1-autogate-v28.report.json";

pub(crate) fn run_phase_stream_online_miner_portfolio_provider_export_admission_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_EXPORT_ADMISSION_REPORT));
    let provider_export_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "external provider export JSONL path is required".to_owned())?;
    let work_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_EXPORT_ADMISSION_WORK_DIR));
    let runtime_replay_report_path = next_path(&mut args, DEFAULT_RUNTIME_REPLAY_REPORT);
    let billing_request_report_path = next_path(&mut args, DEFAULT_BILLING_REQUEST_REPORT);
    let billing_request_jsonl_path = next_path(&mut args, DEFAULT_BILLING_REQUEST_JSONL);
    let billing_contract_report_path = next_path(&mut args, DEFAULT_BILLING_CONTRACT_REPORT);
    let provider_correlation_audit_report_path = args.next().map(PathBuf::from);

    std::fs::create_dir_all(&work_dir).map_err(|error| {
        format!(
            "failed to create provider export admission work dir '{}': {error}",
            work_dir.display()
        )
    })?;

    let provider_normalize_report_path = work_dir.join("provider-export-normalize.report.json");
    let normalized_evidence_jsonl_path = work_dir.join("provider-export-normalized.evidence.jsonl");
    let billing_evidence_gate_report_path = work_dir.join("billing-evidence-gate.report.json");
    let missing_request_jsonl_path = work_dir.join("missing-billing-request.jsonl");
    let admission_report_path = work_dir.join("admission-gate.report.json");
    let promotion_report_path = work_dir.join("promotion-manifest.report.json");
    let evidence_chain_audit_report_path = work_dir.join("evidence-chain-audit.report.json");

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
    run_phase_stream_online_miner_portfolio_admission_gate_v1(
        vec![
            admission_report_path.display().to_string(),
            runtime_replay_report_path.display().to_string(),
            billing_evidence_gate_report_path.display().to_string(),
        ]
        .into_iter(),
    )?;
    run_phase_stream_online_miner_portfolio_promotion_manifest_v1(
        vec![
            promotion_report_path.display().to_string(),
            admission_report_path.display().to_string(),
            billing_contract_report_path.display().to_string(),
        ]
        .into_iter(),
    )?;
    let mut evidence_chain_args = vec![
        evidence_chain_audit_report_path.display().to_string(),
        runtime_replay_report_path.display().to_string(),
        billing_request_report_path.display().to_string(),
        billing_contract_report_path.display().to_string(),
        provider_normalize_report_path.display().to_string(),
        billing_evidence_gate_report_path.display().to_string(),
        admission_report_path.display().to_string(),
        promotion_report_path.display().to_string(),
    ];
    if let Some(path) = &provider_correlation_audit_report_path {
        evidence_chain_args.push(path.display().to_string());
    }
    run_phase_stream_online_miner_portfolio_evidence_chain_audit_v1(
        evidence_chain_args.into_iter(),
    )?;

    let provider_normalize = read_json_value(&provider_normalize_report_path)?;
    let billing_evidence_gate = read_json_value(&billing_evidence_gate_report_path)?;
    let admission = read_json_value(&admission_report_path)?;
    let promotion = read_json_value(&promotion_report_path)?;
    let evidence_chain = read_json_value(&evidence_chain_audit_report_path)?;
    let provider_export_fingerprint64 = provider_export_fingerprint64(&provider_export_jsonl_path)?;
    let provider_export_attestation = review_provider_export_attestation(
        &provider_export_jsonl_path,
        provider_export_fingerprint64,
    )?;

    let evidence_chain_product_ready =
        json_bool(&evidence_chain, &["product_ready"]).unwrap_or(false);
    let provider_billing_evidence_present = json_bool(
        &billing_evidence_gate,
        &["provider_billing_evidence_present"],
    )
    .unwrap_or(false);
    let provider_export_attestation_valid = provider_export_attestation.valid;
    let product_ready = evidence_chain_product_ready && provider_export_attestation_valid;
    let raw_promotion_ready =
        json_bool(&promotion, &["promotion", "promotion_ready"]).unwrap_or(false);
    let promotion_ready = raw_promotion_ready && provider_export_attestation_valid;
    let local_accept_enabled =
        json_bool(&promotion, &["promotion", "local_accept_enabled"]).unwrap_or(true);
    let market_money_claim_allowed = json_bool(&evidence_chain, &["market_money_claim_allowed"])
        .unwrap_or(false)
        || json_bool(&promotion, &["promotion", "market_money_claim_allowed"]).unwrap_or(false);
    let market_money_claim_allowed =
        market_money_claim_allowed && provider_export_attestation_valid;
    let request_rows = json_usize(&billing_evidence_gate, &["request_rows"]).unwrap_or(0);
    let rows_enriched_provider_cost =
        json_usize(&billing_evidence_gate, &["rows_enriched_provider_cost"]).unwrap_or(0);
    let rows_enriched_provider_tokens =
        json_usize(&billing_evidence_gate, &["rows_enriched_provider_tokens"]).unwrap_or(0);
    let missing_billing_request_rows =
        json_usize(&billing_evidence_gate, &["missing_billing_request_rows"]).unwrap_or(0);
    let provider_export_format = json_string(&provider_normalize, &["provider_export_format"])
        .unwrap_or_else(|| "unknown".to_owned());
    let verdict = if product_ready && promotion_ready && !local_accept_enabled {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_EXPORT_ADMISSION_V1_READY_FOR_PRODUCT_REVIEW"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_EXPORT_ADMISSION_V1_BLOCKED"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_portfolio_provider_export_admission_v1",
        "provider_export_jsonl_path": provider_export_jsonl_path,
        "work_dir": work_dir,
        "runtime_replay_report_path": runtime_replay_report_path,
        "billing_request_report_path": billing_request_report_path,
        "billing_request_jsonl_path": billing_request_jsonl_path,
        "billing_contract_report_path": billing_contract_report_path,
        "provider_correlation_audit_report_path": provider_correlation_audit_report_path,
        "provider_export_fingerprint64": provider_export_fingerprint64,
        "provider_normalize_report_path": provider_normalize_report_path,
        "normalized_evidence_jsonl_path": normalized_evidence_jsonl_path,
        "billing_evidence_gate_report_path": billing_evidence_gate_report_path,
        "missing_request_jsonl_path": missing_request_jsonl_path,
        "admission_report_path": admission_report_path,
        "promotion_report_path": promotion_report_path,
        "evidence_chain_audit_report_path": evidence_chain_audit_report_path,
        "provider_normalize": {
            "verdict": json_string(&provider_normalize, &["verdict"]),
            "provider_export_format": provider_export_format,
            "provider_export_rows": json_usize(&provider_normalize, &["provider_export_rows"]).unwrap_or(0),
            "normalized_evidence_rows": json_usize(&provider_normalize, &["normalized_evidence_rows"]).unwrap_or(0),
            "normalized_matched_request_rows": json_usize(&provider_normalize, &["normalized_matched_request_rows"]).unwrap_or(0)
        },
        "provider_export_attestation_required_for_money_claim": true,
        "provider_export_attestation": provider_export_attestation,
        "billing_evidence_gate": {
            "verdict": json_string(&billing_evidence_gate, &["verdict"]),
            "provider_billing_evidence_present": provider_billing_evidence_present,
            "request_rows": request_rows,
            "rows_enriched_provider_cost": rows_enriched_provider_cost,
            "rows_enriched_provider_tokens": rows_enriched_provider_tokens,
            "missing_billing_request_rows": missing_billing_request_rows,
            "missing_billing_request_jsonl_path": missing_request_jsonl_path,
            "provider_cost_microusd": json_u64(&billing_evidence_gate, &["provider_cost_microusd"]).unwrap_or(0),
            "provider_total_tokens": json_usize(&billing_evidence_gate, &["provider_total_tokens"]).unwrap_or(0)
        },
        "admission": {
            "verdict": json_string(&admission, &["verdict"]),
            "shadow_admission_candidate_allowed": json_bool(&admission, &["admission_gate", "shadow_admission_candidate_allowed"]).unwrap_or(false),
            "provider_billing_evidence_present": json_bool(&admission, &["billing_gate", "provider_billing_evidence_present"]).unwrap_or(false),
            "market_money_claim_allowed": json_bool(&admission, &["market_money_claim_allowed"]).unwrap_or(false)
        },
        "promotion": {
            "verdict": json_string(&promotion, &["verdict"]),
            "raw_promotion_ready_before_attestation": raw_promotion_ready,
            "promotion_ready": promotion_ready,
            "local_accept_enabled": local_accept_enabled,
            "market_money_claim_allowed": json_bool(&promotion, &["promotion", "market_money_claim_allowed"]).unwrap_or(false)
        },
        "evidence_chain": {
            "verdict": json_string(&evidence_chain, &["verdict"]),
            "raw_product_ready_before_attestation": evidence_chain_product_ready,
            "product_ready": product_ready,
            "blockers": evidence_chain.get("blockers").cloned().unwrap_or_else(|| serde_json::json!([])),
            "portfolio_unique_cpu_accepts_over_exact_cache": json_usize(&evidence_chain, &["runtime", "portfolio_unique_cpu_accepts_over_exact_cache"]).unwrap_or(0),
            "portfolio_tokens_saved": json_usize(&evidence_chain, &["runtime", "portfolio_tokens_saved"]).unwrap_or(0)
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
        "provider_billing_evidence_present": provider_billing_evidence_present,
        "provider_export_attestation_valid": provider_export_attestation_valid,
        "product_ready": product_ready,
        "promotion_ready": promotion_ready,
        "provider_export_format": provider_export_format,
        "request_rows": request_rows,
        "rows_enriched_provider_cost": rows_enriched_provider_cost,
        "rows_enriched_provider_tokens": rows_enriched_provider_tokens,
        "missing_billing_request_rows": missing_billing_request_rows,
        "missing_billing_request_jsonl_path": missing_request_jsonl_path,
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "product_promotion_allowed": false,
        "market_money_claim_allowed": market_money_claim_allowed,
        "verdict": verdict,
        "boundary": "orchestrates provider export normalization, evidence validation, admission, promotion manifest, and chain audit; does not compile, serve, mutate registry, enable local_accept, or estimate missing money"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_provider_export_admission_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  provider_billing_evidence_present: {provider_billing_evidence_present}");
    println!("  product_ready: {product_ready}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: {market_money_claim_allowed}");
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

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path).and_then(Value::as_bool)
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    json_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
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
