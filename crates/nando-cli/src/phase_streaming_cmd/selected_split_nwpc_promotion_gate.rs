use std::path::{Path, PathBuf};

use nando_core::PhaseCenterOffloadRuntime;
use serde::Serialize;
use serde_json::Value;

use super::{
    json_bool, json_string, json_u64, read_json_value, sanitize_file_stem, write_json_file,
};

const DEFAULT_SELECTED_SPLIT_NWPC_PROMOTION_GATE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-selected-split-nwpc-promotion-gate-v1.report.json";
const DEFAULT_SELECTED_SPLIT_NWPC_SHADOW_REGISTRY_DIR: &str =
    "target/nando-wave/streaming/selected-split-nwpc-shadow-registry-v1";
const DEFAULT_SELECTED_SPLIT_NWPC_QUARANTINE_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-quarantine-v1-realtrace-plus-verifier-sources.report.json";

#[derive(Serialize)]
struct PromotedSelectedSplitPackageReport {
    broad_class_id: String,
    split_rule: String,
    task_name: String,
    source_package_path: String,
    registry_package_path: String,
    source_package_fingerprint64: u64,
    inspected_package_fingerprint64: u64,
    source_package_bytes: usize,
    inspected_package_bytes: usize,
    source_package_records: usize,
    inspected_package_records: usize,
    inspect_matches_quarantine_report: bool,
    registry_copy_exact: bool,
    threshold_micro: i64,
    train_max_false_margin_micro: Option<i64>,
    future_unique_accepts_over_exact_cache: usize,
    future_tokens_saved: usize,
    future_cost_saved_microusd: u64,
    future_false_accepts: usize,
    runtime_margin_parity_mismatches: usize,
    accepted_for_shadow_review: bool,
    promoted_to_shadow_registry: bool,
    blockers: Vec<String>,
}

#[derive(Serialize)]
struct SelectedSplitNwpcPromotionGateReport {
    report_kind: &'static str,
    mode: &'static str,
    quarantine_report_path: String,
    registry_dir: String,
    input_verdict: String,
    input_accepted_package_count: usize,
    input_future_unique_accepts_over_exact_cache: usize,
    input_future_tokens_saved: usize,
    input_future_cost_saved_microusd: u64,
    input_future_false_accepts: usize,
    input_runtime_margin_parity_mismatches: usize,
    input_forbidden_flags_clear: bool,
    promoted_package_count: usize,
    blocked_package_count: usize,
    promoted_unique_accepts_over_exact_cache: usize,
    promoted_tokens_saved: usize,
    promoted_cost_saved_microusd: u64,
    packages: Vec<PromotedSelectedSplitPackageReport>,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    serving_registry_mutated: bool,
    shadow_registry_mutated: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: serde_json::Value,
    verdict: &'static str,
    boundary: &'static str,
}

pub(crate) fn run_phase_stream_selected_split_nwpc_promotion_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_NWPC_PROMOTION_GATE_REPORT));
    let registry_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_NWPC_SHADOW_REGISTRY_DIR));
    let quarantine_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_NWPC_QUARANTINE_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let quarantine = read_json_value(&quarantine_report_path)?;
    let input_verdict = json_string(&quarantine, &["verdict"]).unwrap_or_default();
    let input_accepted_package_count =
        json_usize_path(&quarantine, &["accepted_package_count"]).unwrap_or(0);
    let input_future_unique_accepts_over_exact_cache =
        json_usize_path(&quarantine, &["future_unique_accepts_over_exact_cache"]).unwrap_or(0);
    let input_future_tokens_saved =
        json_usize_path(&quarantine, &["future_tokens_saved"]).unwrap_or(0);
    let input_future_cost_saved_microusd =
        json_u64(&quarantine, &["future_cost_saved_microusd"]).unwrap_or(0);
    let input_future_false_accepts =
        json_usize_path(&quarantine, &["future_false_accepts"]).unwrap_or(usize::MAX);
    let input_runtime_margin_parity_mismatches =
        json_usize_path(&quarantine, &["runtime_margin_parity_mismatches"]).unwrap_or(usize::MAX);
    let input_forbidden_flags_clear = quarantine
        .get("forbidden_flags")
        .is_some_and(forbidden_flags_all_bool_false);
    let input_local_accept_enabled =
        json_bool(&quarantine, &["local_accept_enabled"]).unwrap_or(true);
    let input_auto_promote_enabled =
        json_bool(&quarantine, &["auto_promote_enabled"]).unwrap_or(true);
    let input_market_money_claim_allowed =
        json_bool(&quarantine, &["market_money_claim_allowed"]).unwrap_or(true);

    let global_gate_clear = input_accepted_package_count > 0
        && input_future_unique_accepts_over_exact_cache > 0
        && input_runtime_margin_parity_mismatches == 0
        && input_forbidden_flags_clear
        && !input_local_accept_enabled
        && !input_auto_promote_enabled
        && !input_market_money_claim_allowed;

    let mut packages = Vec::new();
    if let Some(package_values) = quarantine.get("packages").and_then(Value::as_array) {
        for package in package_values {
            packages.push(audit_selected_split_package(
                package,
                &registry_dir,
                global_gate_clear,
            )?);
        }
    }

    let promoted_package_count = packages
        .iter()
        .filter(|package| package.promoted_to_shadow_registry)
        .count();
    let blocked_package_count = packages.len().saturating_sub(promoted_package_count);
    let promoted_unique_accepts_over_exact_cache = packages
        .iter()
        .filter(|package| package.promoted_to_shadow_registry)
        .map(|package| package.future_unique_accepts_over_exact_cache)
        .sum();
    let promoted_tokens_saved = packages
        .iter()
        .filter(|package| package.promoted_to_shadow_registry)
        .map(|package| package.future_tokens_saved)
        .sum();
    let promoted_cost_saved_microusd = packages
        .iter()
        .filter(|package| package.promoted_to_shadow_registry)
        .map(|package| package.future_cost_saved_microusd)
        .sum();
    let verdict = if !global_gate_clear {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_PROMOTION_GATE_V1_BLOCKED_INPUT_GATE"
    } else if promoted_package_count > 0 {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_PROMOTION_GATE_V1_PASS_SHADOW_REGISTRY_READY"
    } else {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_PROMOTION_GATE_V1_WATCH_NO_PROMOTED_PACKAGE"
    };
    let report = SelectedSplitNwpcPromotionGateReport {
        report_kind: "phase_stream_selected_split_nwpc_promotion_gate_v1",
        mode: "shadow_registry_promotion_gate_only",
        quarantine_report_path: quarantine_report_path.display().to_string(),
        registry_dir: registry_dir.display().to_string(),
        input_verdict,
        input_accepted_package_count,
        input_future_unique_accepts_over_exact_cache,
        input_future_tokens_saved,
        input_future_cost_saved_microusd,
        input_future_false_accepts,
        input_runtime_margin_parity_mismatches,
        input_forbidden_flags_clear,
        promoted_package_count,
        blocked_package_count,
        promoted_unique_accepts_over_exact_cache,
        promoted_tokens_saved,
        promoted_cost_saved_microusd,
        packages,
        local_accept_enabled: false,
        auto_promote_enabled: false,
        serving_registry_mutated: false,
        shadow_registry_mutated: promoted_package_count > 0,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: serde_json::json!({
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
        verdict,
        boundary: "shadow registry promotion gate only: copies package-level accepted verifier-bound .nwpc packages from quarantine into a shadow registry when each promoted package has false_accepts=0 and runtime parity is clean; unsafe packages remain blocked; it does not mutate serving registry, enable local_accept, auto-promote runtime behavior, claim market money, or use legacy nwrb/role-binding paths",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_stream_selected_split_nwpc_promotion_gate_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  registry_dir: {}", registry_dir.display());
    println!("  promoted_package_count: {promoted_package_count}");
    println!(
        "  promoted_unique_accepts_over_exact_cache: {promoted_unique_accepts_over_exact_cache}"
    );
    println!("  promoted_tokens_saved: {promoted_tokens_saved}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn audit_selected_split_package(
    package: &Value,
    registry_dir: &Path,
    global_gate_clear: bool,
) -> Result<PromotedSelectedSplitPackageReport, String> {
    let broad_class_id = json_string(package, &["broad_class_id"]).unwrap_or_default();
    let split_rule = json_string(package, &["split_rule"]).unwrap_or_default();
    let task_name = json_string(package, &["task_name"])
        .unwrap_or_else(|| sanitize_file_stem(&format!("{broad_class_id}-{split_rule}")));
    let source_package_path = json_string(package, &["package_path"]).unwrap_or_default();
    let source_package_fingerprint64 =
        json_u64(package, &["package_fingerprint64"]).unwrap_or_default();
    let source_package_bytes = json_usize_path(package, &["package_bytes"]).unwrap_or_default();
    let source_package_records = json_usize_path(package, &["package_records"]).unwrap_or_default();
    let threshold_micro = json_i64_path(package, &["threshold_micro"]).unwrap_or_default();
    let train_max_false_margin_micro = json_i64_path(package, &["train_max_false_margin_micro"]);
    let future_unique_accepts_over_exact_cache =
        json_usize_path(package, &["future_unique_accepts_over_exact_cache"]).unwrap_or_default();
    let future_tokens_saved =
        json_usize_path(package, &["future_tokens_saved"]).unwrap_or_default();
    let future_cost_saved_microusd =
        json_u64(package, &["future_cost_saved_microusd"]).unwrap_or_default();
    let future_false_accepts =
        json_usize_path(package, &["future_false_accepts"]).unwrap_or(usize::MAX);
    let runtime_margin_parity_mismatches =
        json_usize_path(package, &["runtime_margin_parity_mismatches"]).unwrap_or(usize::MAX);
    let accepted_for_shadow_review =
        json_bool(package, &["accepted_for_shadow_review"]).unwrap_or(false);
    let registry_package_path = registry_dir.join(format!(
        "{}.shadow-ready.nwpc",
        sanitize_file_stem(&task_name)
    ));

    let mut blockers = Vec::new();
    if !global_gate_clear {
        blockers.push("global_quarantine_gate_not_clear".to_owned());
    }
    if !accepted_for_shadow_review {
        blockers.push("package_not_accepted_for_shadow_review".to_owned());
    }
    if future_false_accepts != 0 {
        blockers.push("future_false_accepts_nonzero".to_owned());
    }
    if runtime_margin_parity_mismatches != 0 {
        blockers.push("runtime_margin_parity_mismatches_nonzero".to_owned());
    }
    if future_unique_accepts_over_exact_cache == 0 {
        blockers.push("no_future_unique_accepts_over_exact_cache".to_owned());
    }
    if threshold_micro <= 0 {
        blockers.push("threshold_micro_not_positive".to_owned());
    }
    if let Some(max_false) = train_max_false_margin_micro
        && threshold_micro <= max_false
    {
        blockers.push("threshold_not_above_train_false_margin".to_owned());
    }

    let mut inspected_package_fingerprint64 = 0u64;
    let mut inspected_package_bytes = 0usize;
    let mut inspected_package_records = 0usize;
    let mut inspect_matches_quarantine_report = false;
    let mut registry_copy_exact = false;
    let mut package_bytes = Vec::new();
    if !accepted_for_shadow_review {
        // Blocked quarantine rows may legitimately have no package file. Inspecting
        // them would turn a clean package-level rejection into a command failure.
    } else if source_package_path.is_empty() {
        blockers.push("missing_source_package_path".to_owned());
    } else {
        let source_path = PathBuf::from(&source_package_path);
        match std::fs::read(&source_path) {
            Ok(bytes) => {
                package_bytes = bytes;
                match PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes) {
                    Ok(package_info) => {
                        inspected_package_fingerprint64 = package_info.fingerprint64;
                        inspected_package_bytes = package_bytes.len();
                        inspected_package_records = package_info.record_count;
                        inspect_matches_quarantine_report = inspected_package_fingerprint64
                            == source_package_fingerprint64
                            && inspected_package_bytes == source_package_bytes
                            && inspected_package_records == source_package_records;
                        if !inspect_matches_quarantine_report {
                            blockers.push("package_inspect_mismatch".to_owned());
                        }
                    }
                    Err(_) => blockers.push("package_inspect_failed".to_owned()),
                }
            }
            Err(_) => blockers.push("package_read_failed".to_owned()),
        }
    }

    blockers.sort();
    blockers.dedup();
    let promoted_to_shadow_registry = blockers.is_empty();
    if promoted_to_shadow_registry {
        if let Some(parent) = registry_package_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "failed to create selected split shadow registry '{}': {error}",
                    parent.display()
                )
            })?;
        }
        std::fs::write(&registry_package_path, &package_bytes).map_err(|error| {
            format!(
                "failed to write selected split shadow package '{}': {error}",
                registry_package_path.display()
            )
        })?;
        let registry_bytes = std::fs::read(&registry_package_path).map_err(|error| {
            format!(
                "failed to read selected split shadow package '{}': {error}",
                registry_package_path.display()
            )
        })?;
        registry_copy_exact = registry_bytes == package_bytes;
        if !registry_copy_exact {
            return Err(format!(
                "selected split shadow package '{}' copy mismatch",
                registry_package_path.display()
            ));
        }
    }

    Ok(PromotedSelectedSplitPackageReport {
        broad_class_id,
        split_rule,
        task_name,
        source_package_path,
        registry_package_path: registry_package_path.display().to_string(),
        source_package_fingerprint64,
        inspected_package_fingerprint64,
        source_package_bytes,
        inspected_package_bytes,
        source_package_records,
        inspected_package_records,
        inspect_matches_quarantine_report,
        registry_copy_exact,
        threshold_micro,
        train_max_false_margin_micro,
        future_unique_accepts_over_exact_cache,
        future_tokens_saved,
        future_cost_saved_microusd,
        future_false_accepts,
        runtime_margin_parity_mismatches,
        accepted_for_shadow_review,
        promoted_to_shadow_registry,
        blockers,
    })
}

fn json_usize_path(value: &Value, path: &[&str]) -> Option<usize> {
    json_u64(value, path).and_then(|number| usize::try_from(number).ok())
}

fn json_i64_path(value: &Value, path: &[&str]) -> Option<i64> {
    let current = path
        .iter()
        .try_fold(value, |current, key| current.get(*key))?;
    current.as_i64().or_else(|| {
        current
            .as_u64()
            .and_then(|number| i64::try_from(number).ok())
    })
}

fn forbidden_flags_all_bool_false(value: &Value) -> bool {
    let Some(flags) = value.as_object() else {
        return false;
    };
    !flags.is_empty() && flags.values().all(|value| value.as_bool() == Some(false))
}
