use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_AUTOMATIC_DISCOVERY_CHAIN_GATE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-automatic-discovery-chain-gate-v1.report.json";
const DEFAULT_LIVE_CAPTURE_READINESS_REPORT: &str = "target/nando-wave/streaming/phase-atom-live-capture-readiness-v1-command-followup-v25.report.json";
const DEFAULT_SELECTOR_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-selector-v1-autogate-v28.report.json";
const DEFAULT_RUNTIME_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-runtime-replay-v1-autogate-v28.report.json";

pub(crate) fn run_phase_stream_automatic_discovery_chain_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = next_path(&mut args, DEFAULT_AUTOMATIC_DISCOVERY_CHAIN_GATE_REPORT);
    let readiness_report_path = next_path(&mut args, DEFAULT_LIVE_CAPTURE_READINESS_REPORT);
    let selector_report_path = next_path(&mut args, DEFAULT_SELECTOR_REPORT);
    let runtime_replay_report_path = next_path(&mut args, DEFAULT_RUNTIME_REPLAY_REPORT);

    let readiness = read_json_value(&readiness_report_path)?;
    let selector = read_json_value(&selector_report_path)?;
    let runtime = read_json_value(&runtime_replay_report_path)?;

    let readiness_input_paths = json_string_array(&readiness, &["input_paths"]);
    let runtime_trace_paths = json_string_array(&runtime, &["trace_paths"]);
    let trace_lineage_aligned = has_path_overlap(&readiness_input_paths, &runtime_trace_paths);

    let score_ready_rows =
        json_usize(&readiness, &["rows_ready_for_existing_shadow_scoring"]).unwrap_or(0);
    let score_ready_rows_with_provider_correlation =
        json_usize(&readiness, &["score_ready_rows_with_provider_correlation"]).unwrap_or(0);
    let economic_score_ready_rows =
        json_usize(&readiness, &["economic_score_ready_rows"]).unwrap_or(score_ready_rows);
    let economic_score_ready_rows_with_provider_correlation = json_usize(
        &readiness,
        &["economic_score_ready_rows_with_provider_correlation"],
    )
    .unwrap_or(score_ready_rows_with_provider_correlation);
    let economic_score_ready_rows_missing_provider_correlation = json_usize(
        &readiness,
        &["economic_score_ready_rows_missing_provider_correlation"],
    )
    .unwrap_or_else(|| score_ready_rows.saturating_sub(score_ready_rows_with_provider_correlation));
    let zero_denominator_explains_missing_score_ready = json_bool(
        &readiness,
        &["readiness", "zero_denominator_explains_missing_score_ready"],
    )
    .unwrap_or(false);
    let provider_key_atom_leaks =
        json_usize(&readiness, &["rows_with_provider_key_atom_leak"]).unwrap_or(usize::MAX);
    let economic_capture_ready =
        json_bool(&readiness, &["readiness", "economic_capture_ready"]).unwrap_or(false);
    let capture_provider_ready = economic_capture_ready
        && economic_score_ready_rows > 0
        && economic_score_ready_rows_missing_provider_correlation == 0
        && economic_score_ready_rows_with_provider_correlation == economic_score_ready_rows
        && provider_key_atom_leaks == 0;

    let selected_bucket_count = json_usize(&selector, &["selected_bucket_count"]).unwrap_or(0);
    let selector_accepts = json_usize(
        &selector,
        &["portfolio_unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or(0);
    let selector_tokens = json_usize(&selector, &["portfolio_tokens_saved"]).unwrap_or(0);
    let selector_cost = json_u64(&selector, &["portfolio_cost_saved_microusd"]).unwrap_or(0);
    let manual_class_list_used =
        json_bool(&selector, &["discovery_mode", "manual_class_list_used"])
            .or_else(|| json_bool(&selector, &["manual_class_list_used"]))
            .unwrap_or(true);
    let static_topn_seed_used = json_bool(&selector, &["discovery_mode", "static_topn_seed_used"])
        .or_else(|| json_bool(&selector, &["static_topn_seed_used"]))
        .unwrap_or(true);
    let online_discovery_used = json_bool(&selector, &["discovery_mode", "online_discovery_used"])
        .or_else(|| json_bool(&selector, &["online_discovery_used"]))
        .unwrap_or(false);
    let marginal_denominator_delta_used = json_bool(
        &selector,
        &["discovery_mode", "marginal_denominator_delta_used"],
    )
    .or_else(|| json_bool(&selector, &["marginal_denominator_delta_used"]))
    .unwrap_or(false);
    let portfolio_gate_passed = json_bool(&selector, &["discovery_mode", "portfolio_gate_passed"])
        .or_else(|| json_bool(&selector, &["portfolio_gate_passed"]))
        .unwrap_or(false);
    let dynamic_discovery_shadow_claim_allowed = json_bool(
        &selector,
        &["discovery_mode", "dynamic_discovery_shadow_claim_allowed"],
    )
    .or_else(|| json_bool(&selector, &["dynamic_discovery_shadow_claim_allowed"]))
    .unwrap_or(false);
    let constrained_economic_selector_passed =
        json_bool(&selector, &["constrained_economic_selector_passed"]).unwrap_or(false);
    let selector_policy_kind =
        json_string(&selector, &["selector_policy_kind"]).unwrap_or_else(|| "unknown".to_owned());
    let selector_curve_consistent = json_bool(
        &selector,
        &[
            "selector_report_curve_consistency",
            "constrained_economic_main_matches_curve",
        ],
    )
    .unwrap_or(false);
    let selector_clean = !manual_class_list_used
        && !static_topn_seed_used
        && online_discovery_used
        && marginal_denominator_delta_used
        && portfolio_gate_passed
        && dynamic_discovery_shadow_claim_allowed
        && constrained_economic_selector_passed
        && selector_policy_kind == "constrained_economic"
        && selector_curve_consistent
        && selected_bucket_count > 0
        && selector_accepts > 0
        && selector_tokens > 0;

    let runtime_replay_passed =
        json_bool(&runtime, &["discovery_mode", "runtime_replay_passed"]).unwrap_or(false);
    let verifier_binding_bound = json_bool(&runtime, &["verifier_binding_bound"]).unwrap_or(false);
    let runtime_accepts =
        json_usize(&runtime, &["portfolio_unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let runtime_tokens = json_usize(&runtime, &["portfolio_tokens_saved"]).unwrap_or(0);
    let runtime_cost = json_u64(&runtime, &["portfolio_cost_saved_microusd"]).unwrap_or(0);
    let false_accepts = json_usize(&runtime, &["false_accepts"]).unwrap_or(usize::MAX);
    let hot_margin_parity_mismatches =
        json_usize(&runtime, &["hot_margin_parity_mismatches"]).unwrap_or(usize::MAX);
    let hot_decision_parity_mismatches =
        json_usize(&runtime, &["hot_decision_parity_mismatches"]).unwrap_or(usize::MAX);
    let decision_log_margin_mismatches =
        json_usize(&runtime, &["decision_log_margin_mismatches"]).unwrap_or(usize::MAX);
    let selector_accept_parity = json_bool(&runtime, &["selector_accept_parity"]).unwrap_or(false);
    let selector_token_parity = json_bool(&runtime, &["selector_token_parity"]).unwrap_or(false);
    let runtime_forbidden_clear = forbidden_flags_clear(&runtime);
    let selector_forbidden_clear = forbidden_flags_clear(&selector);
    let runtime_local_accept_enabled =
        json_bool(&runtime, &["local_accept_enabled"]).unwrap_or(true);
    let runtime_auto_promote_enabled =
        json_bool(&runtime, &["auto_promote_enabled"]).unwrap_or(true);
    let runtime_clean = runtime_replay_passed
        && runtime_accepts == selector_accepts
        && runtime_tokens == selector_tokens
        && false_accepts == 0
        && hot_margin_parity_mismatches == 0
        && hot_decision_parity_mismatches == 0
        && decision_log_margin_mismatches == 0
        && selector_accept_parity
        && selector_token_parity
        && verifier_binding_bound
        && runtime_forbidden_clear
        && selector_forbidden_clear
        && !runtime_local_accept_enabled
        && !runtime_auto_promote_enabled;

    let automatic_shadow_candidate_allowed =
        trace_lineage_aligned && capture_provider_ready && selector_clean && runtime_clean;
    let mut blockers = Vec::<String>::new();
    if !trace_lineage_aligned {
        blockers.push("trace_lineage_not_aligned".to_owned());
    }
    if !capture_provider_ready {
        blockers.push("provider_boundary_capture_not_ready".to_owned());
    }
    if !selector_clean {
        blockers.push("automatic_selector_not_clean".to_owned());
    }
    if !runtime_clean {
        blockers.push("runtime_replay_not_clean".to_owned());
    }
    if !verifier_binding_bound {
        blockers.push("verifier_binding_missing".to_owned());
    }
    if provider_key_atom_leaks > 0 {
        blockers.push("provider_key_atom_leak".to_owned());
    }
    blockers.sort();
    blockers.dedup();

    let verdict = if provider_key_atom_leaks > 0 {
        "PHASE_STREAM_AUTOMATIC_DISCOVERY_CHAIN_GATE_V1_FAIL_PROVIDER_KEY_ATOM_LEAK"
    } else if automatic_shadow_candidate_allowed {
        "PHASE_STREAM_AUTOMATIC_DISCOVERY_CHAIN_GATE_V1_PASS_SHADOW_READY_BILLING_STILL_REQUIRED"
    } else if trace_lineage_aligned && !capture_provider_ready {
        "PHASE_STREAM_AUTOMATIC_DISCOVERY_CHAIN_GATE_V1_WATCH_PROVIDER_CAPTURE_BLOCKED"
    } else if !trace_lineage_aligned {
        "PHASE_STREAM_AUTOMATIC_DISCOVERY_CHAIN_GATE_V1_WATCH_TRACE_LINEAGE_MISMATCH"
    } else if !selector_clean {
        "PHASE_STREAM_AUTOMATIC_DISCOVERY_CHAIN_GATE_V1_WATCH_SELECTOR_BLOCKED"
    } else {
        "PHASE_STREAM_AUTOMATIC_DISCOVERY_CHAIN_GATE_V1_WATCH_RUNTIME_BLOCKED"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_automatic_discovery_chain_gate_v1",
        "readiness_report_path": readiness_report_path,
        "selector_report_path": selector_report_path,
        "runtime_replay_report_path": runtime_replay_report_path,
        "trace_lineage": {
            "readiness_input_paths": readiness_input_paths,
            "runtime_trace_paths": runtime_trace_paths,
            "trace_lineage_aligned": trace_lineage_aligned,
            "policy": "readiness/provider-boundary report must refer to the same trace family as runtime replay before automatic discovery can be treated as one evidence chain"
        },
        "capture_gate": {
            "score_ready_rows": score_ready_rows,
            "score_ready_rows_with_provider_correlation": score_ready_rows_with_provider_correlation,
            "economic_score_ready_rows": economic_score_ready_rows,
            "economic_score_ready_rows_with_provider_correlation": economic_score_ready_rows_with_provider_correlation,
            "economic_score_ready_rows_missing_provider_correlation": economic_score_ready_rows_missing_provider_correlation,
            "zero_denominator_explains_missing_score_ready": zero_denominator_explains_missing_score_ready,
            "rows_with_provider_key_atom_leak": provider_key_atom_leaks,
            "economic_capture_ready": economic_capture_ready,
            "capture_provider_ready": capture_provider_ready
        },
        "selector_gate": {
            "selected_bucket_count": selected_bucket_count,
            "portfolio_unique_cpu_accepts_over_exact_cache": selector_accepts,
            "portfolio_tokens_saved": selector_tokens,
            "portfolio_cost_saved_microusd": selector_cost,
            "manual_class_list_used": manual_class_list_used,
            "static_topn_seed_used": static_topn_seed_used,
            "online_discovery_used": online_discovery_used,
            "marginal_denominator_delta_used": marginal_denominator_delta_used,
            "portfolio_gate_passed": portfolio_gate_passed,
            "dynamic_discovery_shadow_claim_allowed": dynamic_discovery_shadow_claim_allowed,
            "selector_policy_kind": selector_policy_kind,
            "constrained_economic_selector_passed": constrained_economic_selector_passed,
            "selector_curve_consistent": selector_curve_consistent,
            "selector_clean": selector_clean
        },
        "runtime_gate": {
            "runtime_replay_passed": runtime_replay_passed,
            "portfolio_unique_cpu_accepts_over_exact_cache": runtime_accepts,
            "portfolio_tokens_saved": runtime_tokens,
            "portfolio_cost_saved_microusd": runtime_cost,
            "false_accepts": false_accepts,
            "hot_margin_parity_mismatches": hot_margin_parity_mismatches,
            "hot_decision_parity_mismatches": hot_decision_parity_mismatches,
            "decision_log_margin_mismatches": decision_log_margin_mismatches,
            "selector_accept_parity": selector_accept_parity,
            "selector_token_parity": selector_token_parity,
            "verifier_binding_bound": verifier_binding_bound,
            "runtime_forbidden_clear": runtime_forbidden_clear,
            "selector_forbidden_clear": selector_forbidden_clear,
            "runtime_clean": runtime_clean
        },
        "automatic_shadow_candidate_allowed": automatic_shadow_candidate_allowed,
        "blockers": blockers,
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
        "boundary": "automatic-discovery proof gate only: joins provider-boundary readiness, automatic portfolio selector, and runtime replay evidence; does not mine, compile, promote, serve, enable local_accept, or claim money"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_automatic_discovery_chain_gate_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  trace_lineage_aligned: {trace_lineage_aligned}");
    println!("  capture_provider_ready: {capture_provider_ready}");
    println!("  selector_clean: {selector_clean}");
    println!("  runtime_clean: {runtime_clean}");
    println!("  automatic_shadow_candidate_allowed: {automatic_shadow_candidate_allowed}");
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

fn has_path_overlap(left: &[String], right: &[String]) -> bool {
    let left = left
        .iter()
        .map(|path| normalize_path_key(path))
        .collect::<BTreeSet<_>>();
    right
        .iter()
        .map(|path| normalize_path_key(path))
        .any(|path| left.contains(&path))
}

fn normalize_path_key(path: &str) -> String {
    Path::new(path)
        .components()
        .as_path()
        .to_string_lossy()
        .to_string()
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

fn json_string_array(value: &Value, path: &[&str]) -> Vec<String> {
    json_at(value, path)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
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
