use std::path::PathBuf;

use serde_json::Value;

use super::{json_bool, json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_SELECTED_SPLIT_EVIDENCE_CHAIN_AUDIT_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-evidence-chain-audit-v1.report.json";
const DEFAULT_SELECTED_SPLIT_QUARANTINE_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-quarantine-v1-realtrace-plus-verifier-sources.report.json";
const DEFAULT_SELECTED_SPLIT_PROMOTION_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-promotion-gate-v1-realtrace-plus-verifier-sources.report.json";
const DEFAULT_SELECTED_SPLIT_SHADOW_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-shadow-replay-v1-realtrace-plus-verifier-sources.report.json";
const DEFAULT_SELECTED_SPLIT_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-billing-request-v1-realtrace-plus-verifier-sources.report.json";
const DEFAULT_SELECTED_SPLIT_ADMISSION_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-admission-gate-v1-realtrace-plus-verifier-sources.report.json";
const DEFAULT_SELECTED_SPLIT_PROVIDER_EXPORT_ADMISSION_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-provider-export-admission-v1-template-negative.report.json";

pub(crate) fn run_phase_stream_selected_split_nwpc_evidence_chain_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = next_path(
        &mut args,
        DEFAULT_SELECTED_SPLIT_EVIDENCE_CHAIN_AUDIT_REPORT,
    );
    let quarantine_report_path = next_path(&mut args, DEFAULT_SELECTED_SPLIT_QUARANTINE_REPORT);
    let promotion_report_path = next_path(&mut args, DEFAULT_SELECTED_SPLIT_PROMOTION_REPORT);
    let shadow_replay_report_path =
        next_path(&mut args, DEFAULT_SELECTED_SPLIT_SHADOW_REPLAY_REPORT);
    let billing_request_report_path =
        next_path(&mut args, DEFAULT_SELECTED_SPLIT_BILLING_REQUEST_REPORT);
    let admission_report_path = next_path(&mut args, DEFAULT_SELECTED_SPLIT_ADMISSION_REPORT);
    let provider_export_admission_report_path = next_path(
        &mut args,
        DEFAULT_SELECTED_SPLIT_PROVIDER_EXPORT_ADMISSION_REPORT,
    );
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let quarantine = read_json_value(&quarantine_report_path)?;
    let promotion = read_json_value(&promotion_report_path)?;
    let shadow_replay = read_json_value(&shadow_replay_report_path)?;
    let billing_request = read_json_value(&billing_request_report_path)?;
    let admission = read_json_value(&admission_report_path)?;
    let provider_export_admission = read_json_value(&provider_export_admission_report_path)?;

    let quarantine_accepts =
        json_usize_path(&quarantine, &["future_unique_accepts_over_exact_cache"]).unwrap_or(0);
    let quarantine_tokens = json_usize_path(&quarantine, &["future_tokens_saved"]).unwrap_or(0);
    let promotion_accepts =
        json_usize_path(&promotion, &["promoted_unique_accepts_over_exact_cache"]).unwrap_or(0);
    let promotion_tokens = json_usize_path(&promotion, &["promoted_tokens_saved"]).unwrap_or(0);
    let shadow_accepts =
        json_usize_path(&shadow_replay, &["future_unique_accepts_over_exact_cache"]).unwrap_or(0);
    let shadow_tokens = json_usize_path(&shadow_replay, &["future_tokens_saved"]).unwrap_or(0);
    let shadow_false_accepts =
        json_usize_path(&shadow_replay, &["future_false_accepts"]).unwrap_or(usize::MAX);
    let shadow_mismatches =
        json_usize_path(&shadow_replay, &["replay_mismatch_count"]).unwrap_or(usize::MAX);
    let request_rows = json_usize_path(&billing_request, &["billing_request_rows"]).unwrap_or(0);
    let request_tokens =
        json_usize_path(&billing_request, &["total_tokens_requiring_billing"]).unwrap_or(0);
    let provider_request_rows =
        json_usize_path(&provider_export_admission, &["request_rows"]).unwrap_or(0);
    let provider_rows_with_cost =
        json_usize_path(&provider_export_admission, &["rows_enriched_provider_cost"]).unwrap_or(0);
    let provider_rows_with_tokens = json_usize_path(
        &provider_export_admission,
        &["rows_enriched_provider_tokens"],
    )
    .unwrap_or(0);
    let provider_missing_rows = json_usize_path(
        &provider_export_admission,
        &["missing_billing_request_rows"],
    )
    .unwrap_or(request_rows);
    let provider_billing_evidence_present = json_bool(
        &provider_export_admission,
        &["provider_billing_evidence_present"],
    )
    .unwrap_or(false);

    let counts_line_up = quarantine_accepts > 0
        && quarantine_accepts == promotion_accepts
        && promotion_accepts == shadow_accepts
        && shadow_accepts == request_rows
        && request_rows == provider_request_rows
        && quarantine_tokens > 0
        && quarantine_tokens == promotion_tokens
        && promotion_tokens == shadow_tokens
        && shadow_tokens == request_tokens;
    let quarantine_ready = json_string(&quarantine, &["verdict"]).as_deref()
        == Some("PHASE_STREAM_SELECTED_SPLIT_NWPC_QUARANTINE_V1_PASS_SHADOW_READY")
        && json_usize_path(&quarantine, &["accepted_package_count"]).unwrap_or(0) > 0
        && json_usize_path(&quarantine, &["future_false_accepts"]).unwrap_or(usize::MAX) == 0
        && json_usize_path(&quarantine, &["runtime_margin_parity_mismatches"])
            .unwrap_or(usize::MAX)
            == 0
        && !bool_path_or_true(&quarantine, &["local_accept_enabled"])
        && !bool_path_or_true(&quarantine, &["auto_promote_enabled"])
        && !bool_path_or_true(&quarantine, &["market_money_claim_allowed"])
        && forbidden_flags_all_bool_false(&quarantine);
    let promotion_ready = json_string(&promotion, &["verdict"]).as_deref()
        == Some("PHASE_STREAM_SELECTED_SPLIT_NWPC_PROMOTION_GATE_V1_PASS_SHADOW_REGISTRY_READY")
        && json_usize_path(&promotion, &["promoted_package_count"]).unwrap_or(0) > 0
        && !bool_path_or_true(&promotion, &["local_accept_enabled"])
        && !bool_path_or_true(&promotion, &["auto_promote_enabled"])
        && !bool_path_or_true(&promotion, &["serving_registry_mutated"])
        && !bool_path_or_true(&promotion, &["market_money_claim_allowed"])
        && forbidden_flags_all_bool_false(&promotion);
    let shadow_replay_ready = json_string(&shadow_replay, &["verdict"]).as_deref()
        == Some("PHASE_STREAM_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_V1_PASS_RUNTIME_REPLAY")
        && json_usize_path(&shadow_replay, &["clean_package_count"]).unwrap_or(0) > 0
        && shadow_false_accepts == 0
        && shadow_mismatches == 0
        && !bool_path_or_true(&shadow_replay, &["local_accept_enabled"])
        && !bool_path_or_true(&shadow_replay, &["auto_promote_enabled"])
        && !bool_path_or_true(&shadow_replay, &["serving_registry_mutated"])
        && !bool_path_or_true(&shadow_replay, &["product_runtime_changed"])
        && !bool_path_or_true(&shadow_replay, &["serving_runtime_changed"])
        && !bool_path_or_true(&shadow_replay, &["market_money_claim_allowed"])
        && forbidden_flags_all_bool_false(&shadow_replay);
    let billing_request_ready = json_string(&billing_request, &["verdict"]).as_deref()
        == Some("PHASE_STREAM_SELECTED_SPLIT_NWPC_BILLING_REQUEST_V1_READY")
        && json_bool(&billing_request, &["accept_parity"]).unwrap_or(false)
        && json_bool(&billing_request, &["token_parity"]).unwrap_or(false)
        && !bool_path_or_true(&billing_request, &["local_accept_enabled"])
        && !bool_path_or_true(&billing_request, &["auto_promote_enabled"])
        && !bool_path_or_true(&billing_request, &["market_money_claim_allowed"])
        && forbidden_flags_all_bool_false(&billing_request);
    let admission_ready = json_bool(
        &admission,
        &["admission", "shadow_admission_candidate_allowed"],
    )
    .unwrap_or(false)
        && json_bool(&admission, &["admission", "calls_tokens_claim_allowed"]).unwrap_or(false)
        && !bool_path_or_true(&admission, &["local_accept_enabled"])
        && !bool_path_or_true(&admission, &["auto_promote_enabled"])
        && !bool_path_or_true(&admission, &["serving_registry_mutated"])
        && !bool_path_or_true(&admission, &["product_runtime_changed"])
        && !bool_path_or_true(&admission, &["serving_runtime_changed"])
        && forbidden_flags_all_bool_false(&admission);
    let provider_export_admission_ready = json_bool(
        &provider_export_admission,
        &["shadow_admission_candidate_allowed"],
    )
    .unwrap_or(false)
        && json_bool(&provider_export_admission, &["calls_tokens_claim_allowed"]).unwrap_or(false)
        && !bool_path_or_true(&provider_export_admission, &["local_accept_enabled"])
        && !bool_path_or_true(&provider_export_admission, &["auto_promote_enabled"])
        && !bool_path_or_true(&provider_export_admission, &["serving_registry_mutated"])
        && !bool_path_or_true(&provider_export_admission, &["product_runtime_changed"])
        && !bool_path_or_true(&provider_export_admission, &["serving_runtime_changed"])
        && forbidden_flags_all_bool_false(&provider_export_admission);
    let provider_money_ready = provider_billing_evidence_present
        && json_bool(&provider_export_admission, &["product_promotion_allowed"]).unwrap_or(false)
        && json_bool(&provider_export_admission, &["market_money_claim_allowed"]).unwrap_or(false)
        && provider_rows_with_cost == request_rows
        && provider_rows_with_tokens == request_rows
        && provider_missing_rows == 0
        && request_rows > 0;

    let shadow_chain_ready = quarantine_ready
        && promotion_ready
        && shadow_replay_ready
        && billing_request_ready
        && admission_ready
        && provider_export_admission_ready
        && counts_line_up;
    let product_ready = shadow_chain_ready && provider_money_ready;
    let market_money_claim_allowed = product_ready;

    let mut blockers = Vec::<&'static str>::new();
    if !quarantine_ready {
        blockers.push("quarantine_not_shadow_ready");
    }
    if !promotion_ready {
        blockers.push("promotion_gate_not_shadow_registry_ready");
    }
    if !shadow_replay_ready {
        blockers.push("shadow_replay_not_clean");
    }
    if !billing_request_ready {
        blockers.push("billing_request_not_ready");
    }
    if !admission_ready {
        blockers.push("admission_gate_not_shadow_ready");
    }
    if !provider_export_admission_ready {
        blockers.push("provider_export_admission_not_shadow_ready");
    }
    if !counts_line_up {
        blockers.push("chain_counts_or_tokens_do_not_line_up");
    }
    if !provider_money_ready {
        blockers.push("provider_billing_evidence_missing_or_incomplete");
    }

    let verdict = if product_ready {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_EVIDENCE_CHAIN_AUDIT_V1_PASS_PRODUCT_READY"
    } else if shadow_chain_ready {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_EVIDENCE_CHAIN_AUDIT_V1_PASS_SHADOW_READY_MONEY_BLOCKED"
    } else {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_EVIDENCE_CHAIN_AUDIT_V1_WATCH"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_selected_split_nwpc_evidence_chain_audit_v1",
        "quarantine_report_path": quarantine_report_path,
        "promotion_report_path": promotion_report_path,
        "shadow_replay_report_path": shadow_replay_report_path,
        "billing_request_report_path": billing_request_report_path,
        "admission_report_path": admission_report_path,
        "provider_export_admission_report_path": provider_export_admission_report_path,
        "stage_gates": {
            "quarantine_ready": quarantine_ready,
            "promotion_ready": promotion_ready,
            "shadow_replay_ready": shadow_replay_ready,
            "billing_request_ready": billing_request_ready,
            "admission_ready": admission_ready,
            "provider_export_admission_ready": provider_export_admission_ready,
            "provider_money_ready": provider_money_ready,
            "counts_line_up": counts_line_up
        },
        "scoreboard": {
            "unique_accepts_over_exact_cache": shadow_accepts,
            "tokens_saved": shadow_tokens,
            "false_accepts": shadow_false_accepts,
            "replay_mismatch_count": shadow_mismatches,
            "billing_request_rows": request_rows,
            "provider_request_rows": provider_request_rows,
            "rows_enriched_provider_cost": provider_rows_with_cost,
            "rows_enriched_provider_tokens": provider_rows_with_tokens,
            "missing_billing_request_rows": provider_missing_rows
        },
        "admission": {
            "shadow_chain_ready": shadow_chain_ready,
            "calls_tokens_claim_allowed": shadow_chain_ready,
            "provider_billing_evidence_present": provider_billing_evidence_present,
            "product_ready": product_ready,
            "product_promotion_allowed": product_ready,
            "market_money_claim_allowed": market_money_claim_allowed,
            "blockers": blockers
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
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
        "product_ready": product_ready,
        "market_money_claim_allowed": market_money_claim_allowed,
        "verdict": verdict,
        "boundary": "selected split .nwpc evidence-chain audit only: verifies report lineage from quarantine through provider-export admission; does not compile, mine, serve, mutate registry, enable local_accept, estimate missing money, or use legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_selected_split_nwpc_evidence_chain_audit_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  shadow_chain_ready: {shadow_chain_ready}");
    println!("  product_ready: {product_ready}");
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

fn bool_path_or_true(value: &Value, path: &[&str]) -> bool {
    json_bool(value, path).unwrap_or(true)
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
