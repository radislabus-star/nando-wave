use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_EVIDENCE_CHAIN_AUDIT_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-evidence-chain-audit-v1.report.json";
const DEFAULT_RUNTIME_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-runtime-replay-v1-autogate-v28.report.json";
const DEFAULT_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-request-v1-autogate-v28.report.json";
const DEFAULT_BILLING_CONTRACT_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-evidence-contract-v1-autogate-v28.report.json";
const DEFAULT_PROVIDER_NORMALIZE_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-provider-export-normalize-v1-template-negative.report.json";
const DEFAULT_BILLING_EVIDENCE_GATE_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-evidence-gate-v1-template-normalized-negative.report.json";
const DEFAULT_ADMISSION_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-admission-gate-v1-risk-aware-template-negative.report.json";
const DEFAULT_PROMOTION_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-promotion-manifest-v1-risk-aware-template-negative.report.json";

pub(crate) fn run_phase_stream_online_miner_portfolio_evidence_chain_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = next_path(&mut args, DEFAULT_EVIDENCE_CHAIN_AUDIT_REPORT);
    let runtime_replay_report_path = next_path(&mut args, DEFAULT_RUNTIME_REPLAY_REPORT);
    let billing_request_report_path = next_path(&mut args, DEFAULT_BILLING_REQUEST_REPORT);
    let billing_contract_report_path = next_path(&mut args, DEFAULT_BILLING_CONTRACT_REPORT);
    let provider_normalize_report_path = next_path(&mut args, DEFAULT_PROVIDER_NORMALIZE_REPORT);
    let billing_evidence_gate_report_path =
        next_path(&mut args, DEFAULT_BILLING_EVIDENCE_GATE_REPORT);
    let admission_report_path = next_path(&mut args, DEFAULT_ADMISSION_REPORT);
    let promotion_report_path = next_path(&mut args, DEFAULT_PROMOTION_REPORT);
    let provider_correlation_audit_report_path = args.next().map(PathBuf::from);

    let runtime = read_json_value(&runtime_replay_report_path)?;
    let billing_request = read_json_value(&billing_request_report_path)?;
    let billing_contract = read_json_value(&billing_contract_report_path)?;
    let provider_normalize = read_json_value(&provider_normalize_report_path)?;
    let billing_evidence_gate = read_json_value(&billing_evidence_gate_report_path)?;
    let admission = read_json_value(&admission_report_path)?;
    let promotion = read_json_value(&promotion_report_path)?;
    let provider_correlation_audit = provider_correlation_audit_report_path
        .as_deref()
        .map(read_json_value)
        .transpose()?;

    let runtime_replay_passed =
        json_bool(&runtime, &["discovery_mode", "runtime_replay_passed"]).unwrap_or(false);
    let automatic_selector_clean =
        json_bool(&admission, &["discovery_mode", "automatic_selector_clean"]).unwrap_or(false);
    let manual_class_list_used =
        json_bool(&runtime, &["discovery_mode", "manual_class_list_used"]).unwrap_or(true);
    let online_discovery_used =
        json_bool(&runtime, &["discovery_mode", "online_discovery_used"]).unwrap_or(false);
    let portfolio_accepts =
        json_usize(&runtime, &["portfolio_unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let portfolio_tokens = json_usize(&runtime, &["portfolio_tokens_saved"]).unwrap_or(0);
    let trace_denominator = runtime
        .get("trace_denominator")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let savings_over_exact_cache = runtime
        .get("savings_over_exact_cache")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let exact_cache_baseline = runtime
        .get("exact_cache_baseline")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let combined_exact_cache_plus_nando_shadow = runtime
        .get("combined_exact_cache_plus_nando_shadow")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let runtime_false_accepts = json_usize(&runtime, &["false_accepts"]).unwrap_or(usize::MAX);
    let hot_margin_parity_mismatches =
        json_usize(&runtime, &["hot_margin_parity_mismatches"]).unwrap_or(usize::MAX);
    let hot_decision_parity_mismatches =
        json_usize(&runtime, &["hot_decision_parity_mismatches"]).unwrap_or(usize::MAX);
    let selector_accept_parity = json_bool(&runtime, &["selector_accept_parity"])
        .or_else(|| json_bool(&runtime, &["replay_accept_parity"]))
        .unwrap_or(false);
    let selector_token_parity = json_bool(&runtime, &["selector_token_parity"])
        .or_else(|| json_bool(&runtime, &["replay_token_parity"]))
        .unwrap_or(false);
    let recovered_billing_request_parity =
        json_bool(&runtime, &["recovered_billing_request_parity"]).unwrap_or(false);
    let runtime_parity_clean = hot_margin_parity_mismatches == 0
        && hot_decision_parity_mismatches == 0
        && selector_accept_parity
        && selector_token_parity;

    let billing_request_rows = json_usize(&billing_request, &["billing_request_rows"]).unwrap_or(0);
    let billing_request_tokens =
        json_usize(&billing_request, &["total_tokens_requiring_billing"]).unwrap_or(0);
    let billing_accept_parity = json_bool(&billing_request, &["accept_parity"])
        .or_else(|| json_bool(&billing_request, &["replay_accept_parity"]))
        .or_else(|| json_bool(&billing_request, &["recovered_billing_request_parity"]))
        .unwrap_or(false);
    let billing_token_parity = json_bool(&billing_request, &["token_parity"])
        .or_else(|| json_bool(&billing_request, &["replay_token_parity"]))
        .or_else(|| json_bool(&billing_request, &["recovered_billing_request_parity"]))
        .unwrap_or(false);
    let external_provider_correlation_key_rows = json_usize(
        &billing_request,
        &["external_provider_correlation_key_rows"],
    )
    .unwrap_or(0);
    let external_provider_correlation_missing_rows = json_usize(
        &billing_request,
        &["external_provider_correlation_missing_rows"],
    )
    .unwrap_or(billing_request_rows);
    let provider_correlation_gate = billing_request
        .get("provider_correlation_gate")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let billing_request_ready = billing_request_rows == portfolio_accepts
        && billing_request_tokens == portfolio_tokens
        && billing_accept_parity
        && billing_token_parity
        && (!recovered_billing_request_parity || billing_request_rows == portfolio_accepts)
        && billing_request_rows > 0;

    let contract_request_rows = json_usize(&billing_contract, &["request_rows"]).unwrap_or(0);
    let contract_template_rows = json_usize(&billing_contract, &["template_rows"]).unwrap_or(0);
    let contract_rows_match =
        json_bool(&billing_contract, &["template_rows_match_request_rows"]).unwrap_or(false);
    let contract_ready = json_string(&billing_contract, &["verdict"]).as_deref()
        == Some("PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_CONTRACT_V1_READY")
        && contract_request_rows == billing_request_rows
        && contract_template_rows == billing_request_rows
        && contract_rows_match;
    let request_file_fingerprint64 =
        json_u64(&billing_contract, &["request_file_fingerprint64"]).unwrap_or(0);

    let normalized_evidence_rows =
        json_usize(&provider_normalize, &["normalized_evidence_rows"]).unwrap_or(0);
    let normalized_matched_rows =
        json_usize(&provider_normalize, &["normalized_matched_request_rows"]).unwrap_or(0);
    let normalizer_ready_for_gate = json_string(&provider_normalize, &["verdict"]).as_deref()
        == Some(
            "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_EXPORT_NORMALIZE_V1_READY_FOR_EVIDENCE_GATE",
        )
        && normalized_evidence_rows == billing_request_rows
        && normalized_matched_rows == billing_request_rows
        && billing_request_rows > 0;

    let evidence_request_rows = json_usize(&billing_evidence_gate, &["request_rows"]).unwrap_or(0);
    let evidence_rows = json_usize(&billing_evidence_gate, &["evidence_rows"]).unwrap_or(0);
    let rows_enriched_provider_cost =
        json_usize(&billing_evidence_gate, &["rows_enriched_provider_cost"]).unwrap_or(0);
    let rows_enriched_provider_tokens =
        json_usize(&billing_evidence_gate, &["rows_enriched_provider_tokens"]).unwrap_or(0);
    let missing_billing_request_rows =
        json_usize(&billing_evidence_gate, &["missing_billing_request_rows"]).unwrap_or(0);
    let missing_billing_request_jsonl_path = billing_evidence_gate
        .get("missing_billing_request_jsonl_path")
        .cloned()
        .unwrap_or_else(|| serde_json::json!(null));
    let required_provider_evidence_fields = billing_evidence_gate
        .get("required_provider_evidence_fields")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let accepted_external_source_policy = billing_evidence_gate
        .get("accepted_external_source_policy")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let provider_billing_evidence_present = json_bool(
        &billing_evidence_gate,
        &["provider_billing_evidence_present"],
    )
    .unwrap_or(false);
    let provider_cost_microusd =
        json_u64(&billing_evidence_gate, &["provider_cost_microusd"]).unwrap_or(0);
    let provider_total_tokens =
        json_usize(&billing_evidence_gate, &["provider_total_tokens"]).unwrap_or(0);
    let evidence_gate_passed = provider_billing_evidence_present
        && evidence_request_rows == billing_request_rows
        && rows_enriched_provider_cost == billing_request_rows
        && rows_enriched_provider_tokens == billing_request_rows
        && provider_cost_microusd > 0
        && provider_total_tokens > 0;

    let shadow_admission_candidate_allowed = json_bool(
        &admission,
        &["admission_gate", "shadow_admission_candidate_allowed"],
    )
    .unwrap_or(false);
    let admission_provider_evidence = json_bool(
        &admission,
        &["billing_gate", "provider_billing_evidence_present"],
    )
    .unwrap_or(false);
    let admission_market_money_claim_allowed =
        json_bool(&admission, &["market_money_claim_allowed"]).unwrap_or(false);
    let promotion_ready = json_bool(&promotion, &["promotion", "promotion_ready"]).unwrap_or(false);
    let promotion_local_accept_enabled =
        json_bool(&promotion, &["promotion", "local_accept_enabled"]).unwrap_or(true);
    let promotion_market_money_claim_allowed =
        json_bool(&promotion, &["promotion", "market_money_claim_allowed"]).unwrap_or(false);

    let provider_correlation_audit_present = provider_correlation_audit.is_some();
    let provider_correlation_report_kind = provider_correlation_audit
        .as_ref()
        .and_then(|report| json_string(report, &["report_kind"]));
    let provider_correlation_total_rows = provider_correlation_audit
        .as_ref()
        .and_then(|report| json_usize(report, &["total_rows"]))
        .unwrap_or(0);
    let provider_correlation_rows_with_keys = provider_correlation_audit
        .as_ref()
        .and_then(|report| json_usize(report, &["rows_with_provider_correlation_keys"]))
        .unwrap_or(0);
    let provider_correlation_rows_missing_keys = provider_correlation_audit
        .as_ref()
        .and_then(|report| json_usize(report, &["rows_missing_provider_correlation_keys"]))
        .unwrap_or(0);
    let provider_correlation_cpu_shadow_accept_rows = provider_correlation_audit
        .as_ref()
        .and_then(|report| json_usize(report, &["cpu_shadow_accept_rows"]))
        .unwrap_or(0);
    let provider_correlation_cpu_shadow_accept_rows_with_keys = provider_correlation_audit
        .as_ref()
        .and_then(|report| {
            json_usize(
                report,
                &["cpu_shadow_accept_rows_with_provider_correlation_keys"],
            )
        })
        .unwrap_or(0);
    let provider_correlation_billing_request_rows = provider_correlation_audit
        .as_ref()
        .and_then(|report| json_usize(report, &["billing_request_rows"]))
        .unwrap_or(0);
    let provider_correlation_billing_request_rows_with_keys = provider_correlation_audit
        .as_ref()
        .and_then(|report| {
            json_usize(
                report,
                &["billing_request_rows_with_provider_correlation_keys"],
            )
        })
        .unwrap_or(0);
    let provider_correlation_rows_with_atom_leak = provider_correlation_audit
        .as_ref()
        .and_then(|report| json_usize(report, &["rows_with_provider_key_atom_leak"]))
        .unwrap_or(0);
    let provider_correlation_metadata_only = provider_correlation_audit
        .as_ref()
        .and_then(|report| json_bool(report, &["provider_correlation_metadata_only"]))
        .unwrap_or(!provider_correlation_audit_present);
    let provider_correlation_billing_join_ready = provider_correlation_audit
        .as_ref()
        .and_then(|report| json_bool(report, &["billing_join_ready_for_selected_accepts"]))
        .unwrap_or(false);
    let provider_correlation_verdict = provider_correlation_audit
        .as_ref()
        .and_then(|report| json_string(report, &["verdict"]));

    let forbidden_flags_clear = forbidden_flags_clear(&runtime)
        && forbidden_flags_clear(&billing_request)
        && forbidden_flags_clear(&billing_contract)
        && forbidden_flags_clear(&provider_normalize)
        && forbidden_flags_clear(&billing_evidence_gate)
        && forbidden_flags_clear(&admission)
        && forbidden_flags_clear(&promotion);

    let mut blockers = Vec::<String>::new();
    if manual_class_list_used || !online_discovery_used || !automatic_selector_clean {
        blockers.push("automatic_online_discovery_not_clean".to_owned());
    }
    if !runtime_replay_passed || runtime_false_accepts != 0 || !runtime_parity_clean {
        blockers.push("runtime_replay_or_parity_not_clean".to_owned());
    }
    if !billing_request_ready {
        blockers.push("billing_request_not_aligned_with_runtime_accepts".to_owned());
    }
    if !contract_ready {
        blockers.push("billing_contract_not_ready".to_owned());
    }
    if !normalizer_ready_for_gate {
        blockers.push("provider_export_missing_or_partial".to_owned());
    }
    if !evidence_gate_passed {
        blockers.push("provider_billing_evidence_gate_not_passed".to_owned());
    }
    if !shadow_admission_candidate_allowed {
        blockers.push("shadow_admission_candidate_not_allowed".to_owned());
    }
    if !admission_provider_evidence || !admission_market_money_claim_allowed {
        blockers.push("admission_billing_or_money_claim_blocked".to_owned());
    }
    if !promotion_ready || promotion_local_accept_enabled || !promotion_market_money_claim_allowed {
        blockers.push("promotion_not_ready".to_owned());
    }
    if provider_correlation_audit_present {
        if provider_correlation_rows_with_atom_leak > 0 || !provider_correlation_metadata_only {
            blockers.push("provider_correlation_atom_leak".to_owned());
        }
        if provider_correlation_cpu_shadow_accept_rows > 0
            && provider_correlation_cpu_shadow_accept_rows_with_keys
                < provider_correlation_cpu_shadow_accept_rows
        {
            blockers.push("provider_correlation_missing_for_cpu_shadow_accepts".to_owned());
        }
        if provider_correlation_billing_request_rows > 0
            && provider_correlation_billing_request_rows_with_keys
                < provider_correlation_billing_request_rows
        {
            blockers.push("provider_correlation_missing_for_billing_requests".to_owned());
        }
        if provider_correlation_verdict
            .as_deref()
            .is_some_and(|verdict| verdict.contains("WATCH_NO_PROVIDER_CORRELATION"))
        {
            blockers.push("provider_correlation_absent".to_owned());
        }
    }
    if !forbidden_flags_clear {
        blockers.push("forbidden_flags_not_clear".to_owned());
    }
    blockers.sort();
    blockers.dedup();

    let product_ready = blockers.is_empty();
    let verdict = if product_ready {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_EVIDENCE_CHAIN_AUDIT_V1_READY_FOR_PRODUCT_REVIEW"
    } else if runtime_replay_passed && billing_request_ready && contract_ready {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_EVIDENCE_CHAIN_AUDIT_V1_WATCH_BILLING_EVIDENCE_BLOCKED"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_EVIDENCE_CHAIN_AUDIT_V1_WATCH"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_portfolio_evidence_chain_audit_v1",
        "runtime_replay_report_path": runtime_replay_report_path,
        "billing_request_report_path": billing_request_report_path,
        "billing_contract_report_path": billing_contract_report_path,
        "provider_normalize_report_path": provider_normalize_report_path,
        "billing_evidence_gate_report_path": billing_evidence_gate_report_path,
        "admission_report_path": admission_report_path,
        "promotion_report_path": promotion_report_path,
        "provider_correlation_audit_report_path": provider_correlation_audit_report_path,
        "runtime": {
            "runtime_replay_passed": runtime_replay_passed,
            "automatic_selector_clean": automatic_selector_clean,
            "manual_class_list_used": manual_class_list_used,
            "online_discovery_used": online_discovery_used,
            "portfolio_unique_cpu_accepts_over_exact_cache": portfolio_accepts,
            "portfolio_tokens_saved": portfolio_tokens,
            "false_accepts": runtime_false_accepts,
            "runtime_parity_clean": runtime_parity_clean,
            "selector_or_replay_accept_parity": selector_accept_parity,
            "selector_or_replay_token_parity": selector_token_parity,
            "recovered_billing_request_parity": recovered_billing_request_parity,
            "trace_denominator": trace_denominator,
            "savings_over_exact_cache": savings_over_exact_cache,
            "exact_cache_baseline": exact_cache_baseline,
            "combined_exact_cache_plus_nando_shadow": combined_exact_cache_plus_nando_shadow
        },
        "billing_request": {
            "billing_request_rows": billing_request_rows,
            "total_tokens_requiring_billing": billing_request_tokens,
            "external_provider_correlation_key_rows": external_provider_correlation_key_rows,
            "external_provider_correlation_missing_rows": external_provider_correlation_missing_rows,
            "provider_correlation_gate": provider_correlation_gate,
            "billing_request_ready": billing_request_ready
        },
        "billing_contract": {
            "request_file_fingerprint64": request_file_fingerprint64,
            "request_rows": contract_request_rows,
            "template_rows": contract_template_rows,
            "contract_ready": contract_ready
        },
        "provider_normalize": {
            "normalized_evidence_rows": normalized_evidence_rows,
            "normalized_matched_request_rows": normalized_matched_rows,
            "normalizer_ready_for_gate": normalizer_ready_for_gate
        },
        "billing_evidence_gate": {
            "request_rows": evidence_request_rows,
            "evidence_rows": evidence_rows,
            "rows_enriched_provider_cost": rows_enriched_provider_cost,
            "rows_enriched_provider_tokens": rows_enriched_provider_tokens,
            "missing_billing_request_rows": missing_billing_request_rows,
            "missing_billing_request_jsonl_path": missing_billing_request_jsonl_path,
            "required_provider_evidence_fields": required_provider_evidence_fields,
            "accepted_external_source_policy": accepted_external_source_policy,
            "provider_billing_evidence_present": provider_billing_evidence_present,
            "provider_cost_microusd": provider_cost_microusd,
            "provider_total_tokens": provider_total_tokens,
            "evidence_gate_passed": evidence_gate_passed
        },
        "admission": {
            "shadow_admission_candidate_allowed": shadow_admission_candidate_allowed,
            "provider_billing_evidence_present": admission_provider_evidence,
            "market_money_claim_allowed": admission_market_money_claim_allowed
        },
        "promotion": {
            "promotion_ready": promotion_ready,
            "local_accept_enabled": promotion_local_accept_enabled,
            "market_money_claim_allowed": promotion_market_money_claim_allowed
        },
        "provider_correlation_audit": {
            "present": provider_correlation_audit_present,
            "report_kind": provider_correlation_report_kind,
            "total_rows": provider_correlation_total_rows,
            "rows_with_provider_correlation_keys": provider_correlation_rows_with_keys,
            "rows_missing_provider_correlation_keys": provider_correlation_rows_missing_keys,
            "cpu_shadow_accept_rows": provider_correlation_cpu_shadow_accept_rows,
            "cpu_shadow_accept_rows_with_provider_correlation_keys": provider_correlation_cpu_shadow_accept_rows_with_keys,
            "billing_request_rows": provider_correlation_billing_request_rows,
            "billing_request_rows_with_provider_correlation_keys": provider_correlation_billing_request_rows_with_keys,
            "rows_with_provider_key_atom_leak": provider_correlation_rows_with_atom_leak,
            "provider_correlation_metadata_only": provider_correlation_metadata_only,
            "billing_join_ready_for_selected_accepts": provider_correlation_billing_join_ready,
            "verdict": provider_correlation_verdict
        },
        "blockers": blockers,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false,
            "forbidden_flags_clear": forbidden_flags_clear
        },
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "product_ready": product_ready,
        "product_promotion_allowed": false,
        "market_money_claim_allowed": false,
        "verdict": verdict,
        "boundary": "evidence chain audit only: summarizes runtime replay, billing request, provider evidence, admission, and promotion blockers; does not compile, promote, serve, enable local_accept, or estimate missing money"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_evidence_chain_audit_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  portfolio_unique_cpu_accepts_over_exact_cache: {portfolio_accepts}");
    println!("  portfolio_tokens_saved: {portfolio_tokens}");
    println!("  provider_billing_evidence_present: {provider_billing_evidence_present}");
    println!("  product_ready: {product_ready}");
    println!("  market_money_claim_allowed: false");
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

fn forbidden_flags_clear(value: &Value) -> bool {
    !json_bool(value, &["forbidden_flags", "nwrb_used"]).unwrap_or(false)
        && !json_bool(value, &["forbidden_flags", "role_binding_backend_used"]).unwrap_or(false)
        && !json_bool(value, &["forbidden_flags", "lookup_used"]).unwrap_or(false)
        && !json_bool(
            value,
            &[
                "forbidden_flags",
                "target_id_or_proof_rule_id_authority_used",
            ],
        )
        .unwrap_or(false)
        && !json_bool(value, &["forbidden_flags", "concrete_x_lookup_used"]).unwrap_or(false)
        && !json_bool(value, &["forbidden_flags", "manual_local_out_t_used"]).unwrap_or(false)
        && !json_bool(
            value,
            &["forbidden_flags", "local_accept_without_verifier_used"],
        )
        .unwrap_or(false)
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
