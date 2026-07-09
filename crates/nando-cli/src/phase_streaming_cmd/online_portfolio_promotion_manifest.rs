use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_ONLINE_MINER_PORTFOLIO_PROMOTION_MANIFEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-promotion-manifest-v1.report.json";
const DEFAULT_ONLINE_MINER_PORTFOLIO_ADMISSION_GATE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-portfolio-admission-gate-v1.report.json";

pub(crate) fn run_phase_stream_online_miner_portfolio_promotion_manifest_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_PROMOTION_MANIFEST_REPORT));
    let admission_gate_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_ADMISSION_GATE_REPORT));
    let billing_contract_report_path = args.next().map(PathBuf::from);

    let admission = read_json_value(&admission_gate_report_path)?;
    let billing_contract = billing_contract_report_path
        .as_deref()
        .map(read_json_value)
        .transpose()?;

    let shadow_admission_candidate_allowed = json_bool(
        &admission,
        &["admission_gate", "shadow_admission_candidate_allowed"],
    )
    .unwrap_or(false);
    let admission_product_promotion_allowed = json_bool(&admission, &["product_promotion_allowed"])
        .or_else(|| json_bool(&admission, &["admission_gate", "product_promotion_allowed"]))
        .unwrap_or(false);
    let admission_market_money_claim_allowed =
        json_bool(&admission, &["market_money_claim_allowed"])
            .or_else(|| {
                json_bool(
                    &admission,
                    &["admission_gate", "market_money_claim_allowed"],
                )
            })
            .unwrap_or(false);
    let provider_billing_evidence_present = json_bool(
        &admission,
        &["billing_gate", "provider_billing_evidence_present"],
    )
    .unwrap_or(false);
    let provider_billing_cost_microusd = json_u64(
        &admission,
        &["billing_gate", "provider_billing_cost_microusd"],
    )
    .unwrap_or(0);
    let provider_billing_total_tokens = json_usize(
        &admission,
        &["billing_gate", "provider_billing_total_tokens"],
    )
    .unwrap_or(0);
    let false_accepts =
        json_usize(&admission, &["runtime_replay", "false_accepts"]).unwrap_or(usize::MAX);
    let parity_clean = json_bool(&admission, &["admission_gate", "parity_clean"]).unwrap_or(false);
    let forbidden_flags_clear =
        json_bool(&admission, &["admission_gate", "forbidden_flags_clear"]).unwrap_or(false);
    let positive_shadow_value =
        json_bool(&admission, &["admission_gate", "positive_shadow_value"]).unwrap_or(false);
    let local_accept_enabled = json_bool(&admission, &["local_accept_enabled"])
        .or_else(|| json_bool(&admission, &["admission_gate", "local_accept_enabled"]))
        .unwrap_or(true);

    let request_rows = billing_contract
        .as_ref()
        .and_then(|value| json_usize(value, &["request_rows"]))
        .unwrap_or(0);
    let request_file_fingerprint64 = billing_contract
        .as_ref()
        .and_then(|value| json_u64(value, &["request_file_fingerprint64"]));
    let contract_ready = billing_contract
        .as_ref()
        .and_then(|value| json_string(value, &["verdict"]))
        .as_deref()
        == Some("PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_CONTRACT_V1_READY");

    let mut blockers = json_string_vec(&admission, &["admission_gate", "blockers"]);
    if !shadow_admission_candidate_allowed {
        blockers.push("shadow_admission_candidate_not_allowed".to_owned());
    }
    if false_accepts != 0 {
        blockers.push("false_accepts_nonzero".to_owned());
    }
    if !parity_clean {
        blockers.push("runtime_or_selector_parity_not_clean".to_owned());
    }
    if !forbidden_flags_clear {
        blockers.push("forbidden_flags_not_clear".to_owned());
    }
    if !positive_shadow_value {
        blockers.push("no_positive_shadow_value".to_owned());
    }
    if !contract_ready {
        blockers.push("billing_contract_missing_or_not_ready".to_owned());
    }
    if !provider_billing_evidence_present {
        blockers.push("provider_billing_evidence_missing".to_owned());
    }
    if provider_billing_cost_microusd == 0 || provider_billing_total_tokens == 0 {
        blockers.push("provider_billing_totals_missing".to_owned());
    }
    if local_accept_enabled {
        blockers.push("input_local_accept_already_enabled".to_owned());
    }
    blockers.sort();
    blockers.dedup();

    let promotion_ready = admission_product_promotion_allowed
        && admission_market_money_claim_allowed
        && shadow_admission_candidate_allowed
        && provider_billing_evidence_present
        && provider_billing_cost_microusd > 0
        && provider_billing_total_tokens > 0
        && false_accepts == 0
        && parity_clean
        && forbidden_flags_clear
        && positive_shadow_value
        && contract_ready
        && !local_accept_enabled
        && blockers.is_empty();
    let verdict = if promotion_ready {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROMOTION_MANIFEST_V1_READY"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROMOTION_MANIFEST_V1_BLOCKED"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_portfolio_promotion_manifest_v1",
        "admission_gate_report_path": admission_gate_report_path,
        "billing_contract_report_path": billing_contract_report_path,
        "runtime_replay_report_path": json_string(&admission, &["runtime_replay_report_path"]),
        "billing_request_jsonl_path": billing_contract
            .as_ref()
            .and_then(|value| json_string(value, &["billing_request_jsonl_path"])),
        "shadow_admission_candidate_allowed": shadow_admission_candidate_allowed,
        "provider_billing_evidence_present": provider_billing_evidence_present,
        "provider_billing_cost_microusd": provider_billing_cost_microusd,
        "provider_billing_total_tokens": provider_billing_total_tokens,
        "false_accepts": false_accepts,
        "parity_clean": parity_clean,
        "forbidden_flags_clear": forbidden_flags_clear,
        "positive_shadow_value": positive_shadow_value,
        "request_rows": request_rows,
        "request_file_fingerprint64": request_file_fingerprint64,
        "contract_ready": contract_ready,
        "blockers": blockers,
        "promotion": {
            "promotion_ready": promotion_ready,
            "provider_billing_cost_microusd": provider_billing_cost_microusd,
            "provider_billing_total_tokens": provider_billing_total_tokens,
            "serving_registry_mutated": false,
            "serving_profile_written": false,
            "runtime_changed": false,
            "local_accept_enabled": false,
            "auto_promote_enabled": false,
            "market_money_claim_allowed": promotion_ready
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
        "boundary": "promotion manifest only: freezes selected online-miner .nwpc portfolio evidence and blockers; does not mutate registry, write serving profiles, enable local_accept, estimate money, or revive legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_promotion_manifest_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  promotion_ready: {promotion_ready}");
    println!("  provider_billing_evidence_present: {provider_billing_evidence_present}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
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

fn json_string_vec(value: &Value, path: &[&str]) -> Vec<String> {
    json_at(value, path)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}
