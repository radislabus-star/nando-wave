use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{json_bool, json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_CLEAN_MANIFEST_ADMISSION_GATE_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-admission-gate-v1.report.json";
const DEFAULT_CLEAN_MANIFEST: &str = "target/nando-wave/streaming/phase-stream-live-store-adapter-smoke-v1-clean-promotion-manifest.json";
const DEFAULT_CLEAN_MANIFEST_SHADOW_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-shadow-v1-current.report.json";
const DEFAULT_PREPARED_HOT_PACK_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-live-store-prepared-hot-pack-v1-current.report.json";
const DEFAULT_CLEAN_MANIFEST_LIVE_POLICY_STAGE_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-live-policy-stage-v1.report.json";
const DEFAULT_CLEAN_MANIFEST_LIVE_POLICY: &str =
    "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-live-policy-v1.json";
const DEFAULT_CLEAN_MANIFEST_ADMISSION_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-admission-gate-v1-current.report.json";
const DEFAULT_CLEAN_MANIFEST_LIVE_POLICY_SHADOW_REVIEW_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-live-policy-shadow-review-v1.report.json";
const DEFAULT_CLEAN_MANIFEST_PREPARED_POLICY_SHADOW_REVIEW_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-prepared-policy-shadow-review-v1.report.json";
const DEFAULT_CLEAN_MANIFEST_LIVE_POLICY_STAGE_CURRENT_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-live-policy-stage-v1-current.report.json";
const DEFAULT_LIVE_SOURCE_ADAPTER_WORKER_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-source-adapter-worker-v1-clean-policy-current.report.json";
const DEFAULT_LIVE_MEMORY_WORKER_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-worker-memory-smoke-v1-clean-policy-current.report.json";

pub(crate) fn run_phase_stream_live_store_clean_manifest_admission_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CLEAN_MANIFEST_ADMISSION_GATE_REPORT));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CLEAN_MANIFEST));
    let shadow_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CLEAN_MANIFEST_SHADOW_REPORT));
    let prepared_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PREPARED_HOT_PACK_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let manifest = read_json_value(&manifest_path)?;
    let shadow = read_json_value(&shadow_report_path)?;
    let prepared = read_json_value(&prepared_report_path)?;

    let manifest_promoted_package_count = manifest_array_len(&manifest, &["promoted_packages"]);
    let manifest_promoted_candidate_count =
        json_usize(&manifest, &["promoted_candidate_count"]).unwrap_or(0);
    let manifest_quarantined_candidate_count =
        json_usize(&manifest, &["quarantined_candidate_count"]).unwrap_or(0);
    let manifest_unique_accepts =
        json_usize(&manifest, &["unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let manifest_tokens_saved = json_u64(&manifest, &["tokens_saved"]).unwrap_or(0);
    let manifest_cost_saved_microusd = json_u64(&manifest, &["cost_saved_microusd"]).unwrap_or(0);
    let manifest_false_accepts = json_usize(&manifest, &["false_accepts"]).unwrap_or(usize::MAX);
    let manifest_runtime_parity_mismatches =
        json_usize(&manifest, &["runtime_parity_mismatches"]).unwrap_or(usize::MAX);
    let manifest_hot_bytes_estimate =
        json_usize(&manifest, &["hot_bytes_estimate"]).unwrap_or(usize::MAX);
    let manifest_package_paths = manifest
        .get("promoted_packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| json_string(row, &["package_path"]))
        .collect::<Vec<_>>();
    let manifest_missing_package_paths = manifest_package_paths
        .iter()
        .filter(|path| !Path::new(path.as_str()).is_file())
        .cloned()
        .collect::<Vec<_>>();
    let manifest_gate_clean = json_string(&manifest, &["report_kind"]).as_deref()
        == Some("phase_stream_live_store_clean_promotion_manifest_v1")
        && json_bool(&manifest, &["allowed"]).unwrap_or(false)
        && json_string(&manifest, &["blocker"]).as_deref() == Some("none")
        && manifest_promoted_candidate_count > 0
        && manifest_promoted_package_count == manifest_promoted_candidate_count
        && manifest_missing_package_paths.is_empty()
        && manifest_false_accepts == 0
        && manifest_runtime_parity_mismatches == 0
        && json_bool(&manifest, &["exact_cache_overlap_excluded"]).unwrap_or(false)
        && !json_bool(&manifest, &["local_accept_enabled"]).unwrap_or(true)
        && !json_bool(&manifest, &["market_money_claim_allowed"]).unwrap_or(true);

    let shadow_score_events = json_usize(&shadow, &["score_events"]).unwrap_or(0);
    let shadow_score_candidate_events =
        json_usize(&shadow, &["score_candidate_events"]).unwrap_or(0);
    let shadow_unique_accepts =
        json_usize(&shadow, &["unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let shadow_tokens_saved = json_u64(&shadow, &["tokens_saved"]).unwrap_or(0);
    let shadow_cost_saved_microusd = json_u64(&shadow, &["cost_saved_microusd"]).unwrap_or(0);
    let shadow_false_accepts = json_usize(&shadow, &["false_accepts"]).unwrap_or(usize::MAX);
    let shadow_margin_mismatches =
        json_usize(&shadow, &["runtime_margin_parity_mismatches"]).unwrap_or(usize::MAX);
    let shadow_decision_mismatches =
        json_usize(&shadow, &["runtime_decision_parity_mismatches"]).unwrap_or(usize::MAX);
    let shadow_p99_score_latency_ns = json_u64(&shadow, &["p99_score_latency_ns"]).unwrap_or(0);
    let shadow_gate_clean = json_string(&shadow, &["report_kind"]).as_deref()
        == Some("phase_stream_live_store_clean_manifest_shadow_v1")
        && json_string(&shadow, &["verdict"]).as_deref()
            == Some("LIVE_STORE_CLEAN_MANIFEST_SHADOW_PASS")
        && json_string(&shadow, &["blocker"]).as_deref() == Some("none")
        && json_bool(&shadow, &["manifest_allowed"]).unwrap_or(false)
        && shadow_score_events > 0
        && shadow_score_candidate_events > 0
        && shadow_unique_accepts > 0
        && shadow_tokens_saved > 0
        && shadow_false_accepts == 0
        && shadow_margin_mismatches == 0
        && shadow_decision_mismatches == 0
        && !json_bool(&shadow, &["local_accept_enabled"]).unwrap_or(true)
        && !json_bool(&shadow, &["market_money_claim_allowed"]).unwrap_or(true);

    let prepared_score_events = json_usize(&prepared, &["prepared_score_events"]).unwrap_or(0);
    let prepared_score_candidate_events =
        json_usize(&prepared, &["prepared_score_candidate_events"]).unwrap_or(0);
    let prepared_unique_accepts =
        json_usize(&prepared, &["prepared_unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let prepared_tokens_saved = json_u64(&prepared, &["prepared_tokens_saved"]).unwrap_or(0);
    let prepared_cost_saved_microusd =
        json_u64(&prepared, &["prepared_cost_saved_microusd"]).unwrap_or(0);
    let prepared_false_accepts =
        json_usize(&prepared, &["prepared_false_accepts"]).unwrap_or(usize::MAX);
    let prepared_margin_mismatches =
        json_usize(&prepared, &["atom_prepared_margin_parity_mismatches"]).unwrap_or(usize::MAX);
    let prepared_decision_mismatches =
        json_usize(&prepared, &["atom_prepared_decision_parity_mismatches"]).unwrap_or(usize::MAX);
    let prepared_p99_score_latency_ns =
        json_u64(&prepared, &["prepared_p99_score_latency_ns"]).unwrap_or(u64::MAX);
    let prepared_p99_budget_ns = 1_000u64;
    let prepared_gate_clean = json_string(&prepared, &["report_kind"]).as_deref()
        == Some("phase_stream_live_store_prepared_hot_pack_v1")
        && json_string(&prepared, &["verdict"]).as_deref()
            == Some("LIVE_STORE_PREPARED_HOT_PACK_PASS")
        && json_string(&prepared, &["blocker"]).as_deref() == Some("none")
        && prepared_score_events > 0
        && prepared_score_candidate_events > 0
        && prepared_unique_accepts > 0
        && prepared_tokens_saved > 0
        && prepared_false_accepts == 0
        && prepared_margin_mismatches == 0
        && prepared_decision_mismatches == 0
        && prepared_p99_score_latency_ns <= prepared_p99_budget_ns
        && !json_bool(&prepared, &["local_accept_enabled"]).unwrap_or(true)
        && !json_bool(&prepared, &["market_money_claim_allowed"]).unwrap_or(true);

    let shadow_prepared_parity = shadow_score_events == prepared_score_events
        && shadow_score_candidate_events == prepared_score_candidate_events
        && shadow_unique_accepts == prepared_unique_accepts
        && shadow_tokens_saved == prepared_tokens_saved
        && shadow_cost_saved_microusd == prepared_cost_saved_microusd
        && shadow_false_accepts == prepared_false_accepts;
    let manifest_shadow_same_window = manifest_unique_accepts == shadow_unique_accepts
        && manifest_tokens_saved == shadow_tokens_saved
        && manifest_cost_saved_microusd == shadow_cost_saved_microusd;

    let shadow_registry_candidate_allowed =
        manifest_gate_clean && shadow_gate_clean && prepared_gate_clean && shadow_prepared_parity;
    let product_promotion_allowed = false;
    let local_accept_enabled = false;
    let market_money_claim_allowed = false;

    let mut blockers = Vec::<&'static str>::new();
    if !manifest_gate_clean {
        blockers.push("clean_manifest_gate_not_clean");
    }
    if !shadow_gate_clean {
        blockers.push("clean_manifest_shadow_gate_not_clean");
    }
    if !prepared_gate_clean {
        blockers.push("prepared_hot_pack_gate_not_clean");
    }
    if !shadow_prepared_parity {
        blockers.push("shadow_prepared_metric_parity_mismatch");
    }

    let verdict = if shadow_registry_candidate_allowed {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_ADMISSION_GATE_V1_SHADOW_READY"
    } else {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_ADMISSION_GATE_V1_WATCH"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_live_store_clean_manifest_admission_gate_v1",
        "mode": "clean_manifest_shadow_registry_candidate_gate_only",
        "manifest_path": manifest_path,
        "shadow_report_path": shadow_report_path,
        "prepared_report_path": prepared_report_path,
        "clean_manifest_gate": {
            "clean": manifest_gate_clean,
            "allowed": json_bool(&manifest, &["allowed"]).unwrap_or(false),
            "blocker": json_string(&manifest, &["blocker"]),
            "promoted_candidate_count": manifest_promoted_candidate_count,
            "promoted_package_count": manifest_promoted_package_count,
            "quarantined_candidate_count": manifest_quarantined_candidate_count,
            "unique_cpu_accepts_over_exact_cache": manifest_unique_accepts,
            "tokens_saved": manifest_tokens_saved,
            "cost_saved_microusd": manifest_cost_saved_microusd,
            "false_accepts": manifest_false_accepts,
            "runtime_parity_mismatches": manifest_runtime_parity_mismatches,
            "hot_bytes_estimate": manifest_hot_bytes_estimate,
            "package_paths": manifest_package_paths,
            "missing_package_paths": manifest_missing_package_paths,
            "local_accept_enabled": json_bool(&manifest, &["local_accept_enabled"]).unwrap_or(true),
            "market_money_claim_allowed": json_bool(&manifest, &["market_money_claim_allowed"]).unwrap_or(true)
        },
        "clean_manifest_shadow_gate": {
            "clean": shadow_gate_clean,
            "verdict": json_string(&shadow, &["verdict"]),
            "blocker": json_string(&shadow, &["blocker"]),
            "score_events": shadow_score_events,
            "score_candidate_events": shadow_score_candidate_events,
            "unique_cpu_accepts_over_exact_cache": shadow_unique_accepts,
            "tokens_saved": shadow_tokens_saved,
            "cost_saved_microusd": shadow_cost_saved_microusd,
            "false_accepts": shadow_false_accepts,
            "runtime_margin_parity_mismatches": shadow_margin_mismatches,
            "runtime_decision_parity_mismatches": shadow_decision_mismatches,
            "p99_score_latency_ns": shadow_p99_score_latency_ns,
            "local_accept_enabled": json_bool(&shadow, &["local_accept_enabled"]).unwrap_or(true),
            "market_money_claim_allowed": json_bool(&shadow, &["market_money_claim_allowed"]).unwrap_or(true)
        },
        "prepared_hot_pack_gate": {
            "clean": prepared_gate_clean,
            "verdict": json_string(&prepared, &["verdict"]),
            "blocker": json_string(&prepared, &["blocker"]),
            "pack_rows": json_usize(&prepared, &["pack_rows"]).unwrap_or(0),
            "prepared_score_events": prepared_score_events,
            "prepared_score_candidate_events": prepared_score_candidate_events,
            "prepared_unique_cpu_accepts_over_exact_cache": prepared_unique_accepts,
            "prepared_tokens_saved": prepared_tokens_saved,
            "prepared_cost_saved_microusd": prepared_cost_saved_microusd,
            "prepared_false_accepts": prepared_false_accepts,
            "atom_prepared_margin_parity_mismatches": prepared_margin_mismatches,
            "atom_prepared_decision_parity_mismatches": prepared_decision_mismatches,
            "prepared_p99_score_latency_ns": prepared_p99_score_latency_ns,
            "prepared_p99_budget_ns": prepared_p99_budget_ns,
            "local_accept_enabled": json_bool(&prepared, &["local_accept_enabled"]).unwrap_or(true),
            "market_money_claim_allowed": json_bool(&prepared, &["market_money_claim_allowed"]).unwrap_or(true)
        },
        "cross_checks": {
            "shadow_prepared_parity": shadow_prepared_parity,
            "manifest_shadow_same_window": manifest_shadow_same_window,
            "manifest_shadow_window_note": "manifest package metrics may describe the frozen clean candidate window; shadow/prepared metrics describe the current full trace replay"
        },
        "admission": {
            "shadow_registry_candidate_allowed": shadow_registry_candidate_allowed,
            "product_promotion_allowed": product_promotion_allowed,
            "local_accept_enabled": local_accept_enabled,
            "market_money_claim_allowed": market_money_claim_allowed,
            "blockers": blockers,
            "next_required_gate": "portfolio admission/live policy may consume this only as a shadow candidate; external provider billing evidence is still required before market money"
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
        "local_accept_enabled": local_accept_enabled,
        "auto_promote_enabled": false,
        "serving_registry_mutated": false,
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
        "market_money_claim_allowed": market_money_claim_allowed,
        "verdict": verdict,
        "boundary": "clean manifest admission gate only: validates verifier-bound .nwpc handoff, shadow replay, prepared hot pack parity, and false_accepts=0; does not mutate serving registry, enable local_accept, claim market money, mine new candidates, tune thresholds, or use legacy nwrb"
    });

    write_json_file(&report_path, &report)?;
    println!("phase_stream_live_store_clean_manifest_admission_gate_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  shadow_registry_candidate_allowed: {shadow_registry_candidate_allowed}");
    println!("  shadow_unique_cpu_accepts_over_exact_cache: {shadow_unique_accepts}");
    println!("  prepared_p99_score_latency_ns: {prepared_p99_score_latency_ns}");
    println!("  false_accepts: {shadow_false_accepts}");
    println!("  verdict: {verdict}");
    println!("  local_accept_enabled: {local_accept_enabled}");
    println!("  market_money_claim_allowed: {market_money_claim_allowed}");
    Ok(())
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_u64(value, path).and_then(|number| usize::try_from(number).ok())
}

fn manifest_array_len(value: &Value, path: &[&str]) -> usize {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

pub(crate) fn run_phase_stream_live_store_clean_manifest_live_policy_stage_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CLEAN_MANIFEST_LIVE_POLICY_STAGE_REPORT));
    let policy_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CLEAN_MANIFEST_LIVE_POLICY));
    let admission_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CLEAN_MANIFEST_ADMISSION_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let admission = read_json_value(&admission_report_path)?;
    let manifest_path = json_string(&admission, &["manifest_path"])
        .map(PathBuf::from)
        .ok_or_else(|| "clean manifest admission report missing manifest_path".to_owned())?;
    let manifest = read_json_value(&manifest_path)?;

    let route_entries = manifest
        .get("routes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let package_entries = manifest
        .get("promoted_packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let hot_route_ids = route_entries
        .iter()
        .filter_map(|entry| json_u64(entry, &["route_id"]).and_then(|id| u32::try_from(id).ok()))
        .collect::<Vec<_>>();
    let hot_profile_ids = route_entries
        .iter()
        .filter_map(|entry| json_u64(entry, &["profile_id"]).and_then(|id| u32::try_from(id).ok()))
        .collect::<Vec<_>>();
    let package_paths = package_entries
        .iter()
        .filter_map(|entry| json_string(entry, &["package_path"]))
        .collect::<Vec<_>>();
    let missing_package_paths = package_paths
        .iter()
        .filter(|path| !Path::new(path.as_str()).is_file())
        .cloned()
        .collect::<Vec<_>>();

    let admission_report_kind_ok = json_string(&admission, &["report_kind"]).as_deref()
        == Some("phase_stream_live_store_clean_manifest_admission_gate_v1");
    let admission_shadow_ready = json_string(&admission, &["verdict"]).as_deref()
        == Some("PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_ADMISSION_GATE_V1_SHADOW_READY")
        && json_bool(
            &admission,
            &["admission", "shadow_registry_candidate_allowed"],
        )
        .unwrap_or(false);
    let admission_blockers_empty = admission
        .get("admission")
        .and_then(|value| value.get("blockers"))
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty);
    let admission_local_accept_disabled =
        !json_bool(&admission, &["admission", "local_accept_enabled"]).unwrap_or(true)
            && !json_bool(&admission, &["local_accept_enabled"]).unwrap_or(true);
    let admission_money_disabled =
        !json_bool(&admission, &["admission", "market_money_claim_allowed"]).unwrap_or(true)
            && !json_bool(&admission, &["market_money_claim_allowed"]).unwrap_or(true);
    let admission_product_promotion_disabled =
        !json_bool(&admission, &["admission", "product_promotion_allowed"]).unwrap_or(true);
    let clean_manifest_gate_clean =
        json_bool(&admission, &["clean_manifest_gate", "clean"]).unwrap_or(false);
    let clean_shadow_gate_clean =
        json_bool(&admission, &["clean_manifest_shadow_gate", "clean"]).unwrap_or(false);
    let prepared_hot_gate_clean =
        json_bool(&admission, &["prepared_hot_pack_gate", "clean"]).unwrap_or(false);
    let shadow_prepared_parity =
        json_bool(&admission, &["cross_checks", "shadow_prepared_parity"]).unwrap_or(false);
    let shadow_accepts = json_usize(
        &admission,
        &[
            "clean_manifest_shadow_gate",
            "unique_cpu_accepts_over_exact_cache",
        ],
    )
    .unwrap_or(0);
    let shadow_tokens =
        json_u64(&admission, &["clean_manifest_shadow_gate", "tokens_saved"]).unwrap_or(0);
    let shadow_cost = json_u64(
        &admission,
        &["clean_manifest_shadow_gate", "cost_saved_microusd"],
    )
    .unwrap_or(0);
    let false_accepts = json_usize(&admission, &["clean_manifest_shadow_gate", "false_accepts"])
        .unwrap_or(usize::MAX);
    let runtime_margin_mismatches = json_usize(
        &admission,
        &[
            "clean_manifest_shadow_gate",
            "runtime_margin_parity_mismatches",
        ],
    )
    .unwrap_or(usize::MAX);
    let runtime_decision_mismatches = json_usize(
        &admission,
        &[
            "clean_manifest_shadow_gate",
            "runtime_decision_parity_mismatches",
        ],
    )
    .unwrap_or(usize::MAX);
    let prepared_p99_ns = json_u64(
        &admission,
        &["prepared_hot_pack_gate", "prepared_p99_score_latency_ns"],
    )
    .unwrap_or(u64::MAX);

    let live_policy_shadow_stage_allowed = admission_report_kind_ok
        && admission_shadow_ready
        && admission_blockers_empty
        && clean_manifest_gate_clean
        && clean_shadow_gate_clean
        && prepared_hot_gate_clean
        && shadow_prepared_parity
        && !hot_route_ids.is_empty()
        && !hot_profile_ids.is_empty()
        && !package_paths.is_empty()
        && missing_package_paths.is_empty()
        && shadow_accepts > 0
        && shadow_tokens > 0
        && false_accepts == 0
        && runtime_margin_mismatches == 0
        && runtime_decision_mismatches == 0
        && prepared_p99_ns <= 1_000
        && admission_local_accept_disabled
        && admission_money_disabled
        && admission_product_promotion_disabled;

    let mut blockers = Vec::<&'static str>::new();
    if !admission_report_kind_ok {
        blockers.push("admission_report_kind_mismatch");
    }
    if !admission_shadow_ready {
        blockers.push("admission_not_shadow_ready");
    }
    if !admission_blockers_empty {
        blockers.push("admission_blockers_present");
    }
    if !clean_manifest_gate_clean || !clean_shadow_gate_clean || !prepared_hot_gate_clean {
        blockers.push("input_gate_not_clean");
    }
    if !shadow_prepared_parity {
        blockers.push("shadow_prepared_parity_mismatch");
    }
    if hot_route_ids.is_empty() || hot_profile_ids.is_empty() || package_paths.is_empty() {
        blockers.push("missing_hot_route_profile_or_package");
    }
    if !missing_package_paths.is_empty() {
        blockers.push("missing_package_file");
    }
    if shadow_accepts == 0 || shadow_tokens == 0 {
        blockers.push("missing_positive_shadow_value");
    }
    if false_accepts != 0 {
        blockers.push("false_accepts_nonzero");
    }
    if runtime_margin_mismatches != 0 || runtime_decision_mismatches != 0 {
        blockers.push("runtime_parity_mismatch");
    }
    if prepared_p99_ns > 1_000 {
        blockers.push("prepared_hot_p99_budget_exceeded");
    }
    if !admission_local_accept_disabled || !admission_money_disabled {
        blockers.push("input_enables_local_accept_or_money");
    }
    if !admission_product_promotion_disabled {
        blockers.push("input_product_promotion_enabled");
    }

    let policy = serde_json::json!({
        "report_kind": "phase_stream_live_store_clean_manifest_live_policy_v1",
        "mode": "shadow_only_live_policy_no_runtime_mutation",
        "admission_policy_kind": "clean_manifest_shadow_ready_to_live_policy_stage_v1",
        "source_admission_report_path": admission_report_path,
        "source_manifest_path": manifest_path,
        "hot_route_ids": hot_route_ids,
        "hot_profile_ids": hot_profile_ids,
        "hot_profile_count": hot_profile_ids.len(),
        "hot_route_count": hot_route_ids.len(),
        "hot_route_profile_edges": route_entries.len(),
        "hot_bytes_estimate": json_usize(&admission, &["clean_manifest_gate", "hot_bytes_estimate"]).unwrap_or(0),
        "package_paths": package_paths,
        "unique_cpu_accepts_over_exact_cache": shadow_accepts,
        "tokens_saved": shadow_tokens,
        "cost_saved_microusd": shadow_cost,
        "false_accepts": false_accepts,
        "runtime_margin_parity_mismatches": runtime_margin_mismatches,
        "runtime_decision_parity_mismatches": runtime_decision_mismatches,
        "prepared_p99_score_latency_ns": prepared_p99_ns,
        "verifier_binding_bound": true,
        "exact_cache_overlap_excluded": true,
        "token_cost_denominator_present": shadow_tokens > 0 && shadow_cost > 0,
        "shadow_registry_candidate_allowed": live_policy_shadow_stage_allowed,
        "live_policy_shadow_stage_allowed": live_policy_shadow_stage_allowed,
        "registry_mutation_enabled": false,
        "cpu_profile_registry_write_enabled": false,
        "serving_profile_artifact_written": false,
        "product_promotion_enabled": false,
        "local_accept_enabled": false,
        "market_money_claim_allowed": false,
        "forbidden_flags": {
            "target_id_used": false,
            "proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "hidden_frame_id_or_bind_x_used": false,
            "legacy_backend_used": false,
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "local_accept_without_verifier_used": false
        },
        "verdict": if live_policy_shadow_stage_allowed {
            "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_LIVE_POLICY_V1_SHADOW_STAGE_READY"
        } else {
            "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_LIVE_POLICY_V1_WATCH"
        },
        "blocker": if live_policy_shadow_stage_allowed { "none" } else { "live_policy_stage_gate_failed" },
        "boundary": "shadow-only live policy artifact: carries clean verifier-bound .nwpc package references for later daemon shadow review; does not mutate registry, write serving profiles, enable local_accept, auto-promote, claim market money, or use legacy nwrb"
    });
    write_json_file(&policy_path, &policy)?;

    let report = serde_json::json!({
        "report_kind": "phase_stream_live_store_clean_manifest_live_policy_stage_v1",
        "mode": "clean_manifest_admission_to_shadow_only_live_policy",
        "source_admission_report_path": admission_report_path,
        "live_policy_path": policy_path,
        "live_policy_shadow_stage_allowed": live_policy_shadow_stage_allowed,
        "blockers": blockers,
        "hot_route_count": hot_route_ids.len(),
        "hot_profile_count": hot_profile_ids.len(),
        "package_count": package_paths.len(),
        "unique_cpu_accepts_over_exact_cache": shadow_accepts,
        "tokens_saved": shadow_tokens,
        "cost_saved_microusd": shadow_cost,
        "false_accepts": false_accepts,
        "runtime_margin_parity_mismatches": runtime_margin_mismatches,
        "runtime_decision_parity_mismatches": runtime_decision_mismatches,
        "prepared_p99_score_latency_ns": prepared_p99_ns,
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "serving_registry_mutated": false,
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
        "market_money_claim_allowed": false,
        "verdict": if live_policy_shadow_stage_allowed {
            "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_LIVE_POLICY_STAGE_V1_SHADOW_READY"
        } else {
            "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_LIVE_POLICY_STAGE_V1_WATCH"
        },
        "boundary": "live policy stage gate only: converts a clean manifest admission report into a shadow-only live policy artifact; it does not mutate registry, enable local_accept, promote product runtime, or claim market money"
    });
    write_json_file(&report_path, &report)?;

    println!("phase_stream_live_store_clean_manifest_live_policy_stage_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  policy_path: {}", policy_path.display());
    println!("  live_policy_shadow_stage_allowed: {live_policy_shadow_stage_allowed}");
    println!("  unique_cpu_accepts_over_exact_cache: {shadow_accepts}");
    println!("  false_accepts: {false_accepts}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!(
        "  verdict: {}",
        report
            .get("verdict")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    );
    Ok(())
}

pub(crate) fn run_phase_stream_live_store_clean_manifest_live_policy_shadow_review_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CLEAN_MANIFEST_LIVE_POLICY_SHADOW_REVIEW_REPORT));
    let stage_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CLEAN_MANIFEST_LIVE_POLICY_STAGE_CURRENT_REPORT));
    let policy_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CLEAN_MANIFEST_LIVE_POLICY));
    let worker_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_SOURCE_ADAPTER_WORKER_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let stage = read_json_value(&stage_report_path)?;
    let policy = read_json_value(&policy_path)?;
    let worker = read_json_value(&worker_report_path)?;

    let stage_ready = json_string(&stage, &["report_kind"]).as_deref()
        == Some("phase_stream_live_store_clean_manifest_live_policy_stage_v1")
        && json_string(&stage, &["verdict"]).as_deref()
            == Some("PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_LIVE_POLICY_STAGE_V1_SHADOW_READY")
        && json_bool(&stage, &["live_policy_shadow_stage_allowed"]).unwrap_or(false)
        && json_array_empty(&stage, &["blockers"])
        && !json_bool(&stage, &["local_accept_enabled"]).unwrap_or(true)
        && !json_bool(&stage, &["market_money_claim_allowed"]).unwrap_or(true);

    let policy_ready = json_string(&policy, &["report_kind"]).as_deref()
        == Some("phase_stream_live_store_clean_manifest_live_policy_v1")
        && json_string(&policy, &["verdict"]).as_deref()
            == Some("PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_LIVE_POLICY_V1_SHADOW_STAGE_READY")
        && json_string(&policy, &["mode"]).as_deref()
            == Some("shadow_only_live_policy_no_runtime_mutation")
        && json_bool(&policy, &["live_policy_shadow_stage_allowed"]).unwrap_or(false)
        && json_bool(&policy, &["verifier_binding_bound"]).unwrap_or(false)
        && json_bool(&policy, &["exact_cache_overlap_excluded"]).unwrap_or(false)
        && json_bool(&policy, &["token_cost_denominator_present"]).unwrap_or(false)
        && !json_bool(&policy, &["registry_mutation_enabled"]).unwrap_or(true)
        && !json_bool(&policy, &["cpu_profile_registry_write_enabled"]).unwrap_or(true)
        && !json_bool(&policy, &["serving_profile_artifact_written"]).unwrap_or(true)
        && !json_bool(&policy, &["product_promotion_enabled"]).unwrap_or(true)
        && !json_bool(&policy, &["local_accept_enabled"]).unwrap_or(true)
        && !json_bool(&policy, &["market_money_claim_allowed"]).unwrap_or(true)
        && forbidden_flags_false(&policy);

    let worker_score_events = json_usize(&worker, &["score_events"]).unwrap_or(0);
    let worker_score_candidate_events =
        json_usize(&worker, &["score_candidate_events"]).unwrap_or(0);
    let worker_accepts = json_usize(&worker, &["unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let worker_tokens = json_u64(&worker, &["tokens_saved"]).unwrap_or(0);
    let worker_cost = json_u64(&worker, &["cost_saved_microusd"]).unwrap_or(0);
    let worker_false_accepts = json_usize(&worker, &["false_accepts"]).unwrap_or(usize::MAX);
    let worker_local_accept_events =
        json_usize(&worker, &["local_accept_events"]).unwrap_or(usize::MAX);
    let worker_p99_ns = json_u64(&worker, &["p99_score_latency_ns"]).unwrap_or(u64::MAX);
    let worker_p99_budget_ns = 1_000u64;
    let worker_semantic_clean = json_string(&worker, &["report_kind"]).as_deref()
        == Some("phase_stream_live_source_adapter_worker_v1")
        && worker_score_events > 0
        && worker_score_candidate_events > 0
        && worker_accepts > 0
        && worker_tokens > 0
        && worker_false_accepts == 0
        && worker_local_accept_events == 0
        && !json_bool(&worker, &["local_accept_enabled"]).unwrap_or(true)
        && !json_bool(&worker, &["market_money_claim_allowed"]).unwrap_or(true)
        && json_bool(&worker, &["source_adapter_streaming_lines_used"]).unwrap_or(false)
        && json_bool(&worker, &["hot_score_inside_event_loop"]).unwrap_or(false)
        && !json_bool(&worker, &["hot_loop_json_used"]).unwrap_or(true)
        && !json_bool(&worker, &["hot_loop_btreemap_used"]).unwrap_or(true)
        && !json_bool(&worker, &["hot_loop_string_route_used"]).unwrap_or(true)
        && !json_bool(&worker, &["hot_loop_file_io_used"]).unwrap_or(true)
        && !json_bool(&worker, &["hot_loop_package_compile_used"]).unwrap_or(true)
        && forbidden_flags_false(&worker);
    let worker_latency_clean = worker_p99_ns <= worker_p99_budget_ns;

    let policy_accepts = json_usize(&policy, &["unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let policy_tokens = json_u64(&policy, &["tokens_saved"]).unwrap_or(0);
    let policy_cost = json_u64(&policy, &["cost_saved_microusd"]).unwrap_or(0);
    let policy_worker_parity = policy_accepts == worker_accepts
        && policy_tokens == worker_tokens
        && policy_cost == worker_cost
        && json_usize(&policy, &["false_accepts"]).unwrap_or(usize::MAX) == worker_false_accepts;

    let daemon_shadow_safety_review_passed =
        stage_ready && policy_ready && worker_semantic_clean && policy_worker_parity;
    let daemon_shadow_hot_latency_passed =
        daemon_shadow_safety_review_passed && worker_latency_clean;
    let daemon_shadow_review_allowed =
        daemon_shadow_safety_review_passed && daemon_shadow_hot_latency_passed;

    let mut blockers = Vec::<&'static str>::new();
    if !stage_ready {
        blockers.push("live_policy_stage_not_ready");
    }
    if !policy_ready {
        blockers.push("live_policy_artifact_not_ready");
    }
    if !worker_semantic_clean {
        blockers.push("live_source_worker_semantic_gate_not_clean");
    }
    if !policy_worker_parity {
        blockers.push("policy_worker_metric_parity_mismatch");
    }
    if !worker_latency_clean {
        blockers.push("live_source_worker_p99_budget_exceeded");
    }

    let verdict = if daemon_shadow_review_allowed {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_LIVE_POLICY_SHADOW_REVIEW_V1_PASS"
    } else if daemon_shadow_safety_review_passed {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_LIVE_POLICY_SHADOW_REVIEW_V1_WATCH_LATENCY"
    } else {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_LIVE_POLICY_SHADOW_REVIEW_V1_WATCH"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_live_store_clean_manifest_live_policy_shadow_review_v1",
        "mode": "live_policy_artifact_to_live_source_worker_shadow_review",
        "stage_report_path": stage_report_path,
        "policy_path": policy_path,
        "worker_report_path": worker_report_path,
        "stage_ready": stage_ready,
        "policy_ready": policy_ready,
        "worker_semantic_clean": worker_semantic_clean,
        "worker_latency_clean": worker_latency_clean,
        "policy_worker_parity": policy_worker_parity,
        "daemon_shadow_safety_review_passed": daemon_shadow_safety_review_passed,
        "daemon_shadow_hot_latency_passed": daemon_shadow_hot_latency_passed,
        "daemon_shadow_review_allowed": daemon_shadow_review_allowed,
        "blockers": blockers,
        "hot_route_ids": policy.get("hot_route_ids").cloned().unwrap_or(Value::Null),
        "hot_profile_ids": policy.get("hot_profile_ids").cloned().unwrap_or(Value::Null),
        "package_paths": policy.get("package_paths").cloned().unwrap_or(Value::Null),
        "score_events": worker_score_events,
        "score_candidate_events": worker_score_candidate_events,
        "unique_cpu_accepts_over_exact_cache": worker_accepts,
        "tokens_saved": worker_tokens,
        "cost_saved_microusd": worker_cost,
        "false_accepts": worker_false_accepts,
        "local_accept_events": worker_local_accept_events,
        "p99_score_latency_ns": worker_p99_ns,
        "p99_budget_ns": worker_p99_budget_ns,
        "hot_loop_json_used": json_bool(&worker, &["hot_loop_json_used"]).unwrap_or(true),
        "hot_loop_btreemap_used": json_bool(&worker, &["hot_loop_btreemap_used"]).unwrap_or(true),
        "hot_loop_string_route_used": json_bool(&worker, &["hot_loop_string_route_used"]).unwrap_or(true),
        "hot_loop_file_io_used": json_bool(&worker, &["hot_loop_file_io_used"]).unwrap_or(true),
        "hot_loop_package_compile_used": json_bool(&worker, &["hot_loop_package_compile_used"]).unwrap_or(true),
        "registry_mutation_enabled": false,
        "cpu_profile_registry_write_enabled": false,
        "serving_profile_artifact_written": false,
        "product_promotion_enabled": false,
        "local_accept_enabled": false,
        "market_money_claim_allowed": false,
        "verdict": verdict,
        "boundary": "live policy shadow review only: consumes a shadow-only live policy artifact and live source worker shadow report; safety may pass while latency remains WATCH; does not mutate registry, write serving profiles, enable local_accept, auto-promote, claim market money, or use legacy nwrb"
    });
    write_json_file(&report_path, &report)?;

    println!("phase_stream_live_store_clean_manifest_live_policy_shadow_review_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  daemon_shadow_safety_review_passed: {daemon_shadow_safety_review_passed}");
    println!("  daemon_shadow_hot_latency_passed: {daemon_shadow_hot_latency_passed}");
    println!("  unique_cpu_accepts_over_exact_cache: {worker_accepts}");
    println!("  false_accepts: {worker_false_accepts}");
    println!("  p99_score_latency_ns: {worker_p99_ns}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

pub(crate) fn run_phase_stream_live_store_clean_manifest_prepared_policy_shadow_review_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_CLEAN_MANIFEST_PREPARED_POLICY_SHADOW_REVIEW_REPORT)
    });
    let stage_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CLEAN_MANIFEST_LIVE_POLICY_STAGE_CURRENT_REPORT));
    let policy_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CLEAN_MANIFEST_LIVE_POLICY));
    let prepared_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PREPARED_HOT_PACK_REPORT));
    let memory_worker_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_MEMORY_WORKER_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let stage = read_json_value(&stage_report_path)?;
    let policy = read_json_value(&policy_path)?;
    let prepared = read_json_value(&prepared_report_path)?;
    let memory_worker = read_json_value(&memory_worker_report_path)?;

    let stage_ready = json_string(&stage, &["report_kind"]).as_deref()
        == Some("phase_stream_live_store_clean_manifest_live_policy_stage_v1")
        && json_string(&stage, &["verdict"]).as_deref()
            == Some("PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_LIVE_POLICY_STAGE_V1_SHADOW_READY")
        && json_bool(&stage, &["live_policy_shadow_stage_allowed"]).unwrap_or(false)
        && json_array_empty(&stage, &["blockers"])
        && !json_bool(&stage, &["local_accept_enabled"]).unwrap_or(true)
        && !json_bool(&stage, &["market_money_claim_allowed"]).unwrap_or(true);

    let policy_ready = json_string(&policy, &["report_kind"]).as_deref()
        == Some("phase_stream_live_store_clean_manifest_live_policy_v1")
        && json_string(&policy, &["verdict"]).as_deref()
            == Some("PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_LIVE_POLICY_V1_SHADOW_STAGE_READY")
        && json_string(&policy, &["mode"]).as_deref()
            == Some("shadow_only_live_policy_no_runtime_mutation")
        && json_bool(&policy, &["live_policy_shadow_stage_allowed"]).unwrap_or(false)
        && json_bool(&policy, &["verifier_binding_bound"]).unwrap_or(false)
        && json_bool(&policy, &["exact_cache_overlap_excluded"]).unwrap_or(false)
        && json_bool(&policy, &["token_cost_denominator_present"]).unwrap_or(false)
        && !json_bool(&policy, &["registry_mutation_enabled"]).unwrap_or(true)
        && !json_bool(&policy, &["cpu_profile_registry_write_enabled"]).unwrap_or(true)
        && !json_bool(&policy, &["serving_profile_artifact_written"]).unwrap_or(true)
        && !json_bool(&policy, &["product_promotion_enabled"]).unwrap_or(true)
        && !json_bool(&policy, &["local_accept_enabled"]).unwrap_or(true)
        && !json_bool(&policy, &["market_money_claim_allowed"]).unwrap_or(true)
        && forbidden_flags_false(&policy);

    let prepared_score_events = json_usize(&prepared, &["prepared_score_events"]).unwrap_or(0);
    let prepared_score_candidate_events =
        json_usize(&prepared, &["prepared_score_candidate_events"]).unwrap_or(0);
    let prepared_verifier_required_events =
        json_usize(&prepared, &["prepared_verifier_required_events"]).unwrap_or(0);
    let prepared_local_accept_events =
        json_usize(&prepared, &["prepared_local_accept_events"]).unwrap_or(usize::MAX);
    let prepared_accepts =
        json_usize(&prepared, &["prepared_unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let prepared_tokens = json_u64(&prepared, &["prepared_tokens_saved"]).unwrap_or(0);
    let prepared_cost = json_u64(&prepared, &["prepared_cost_saved_microusd"]).unwrap_or(0);
    let prepared_false_accepts =
        json_usize(&prepared, &["prepared_false_accepts"]).unwrap_or(usize::MAX);
    let prepared_margin_parity_mismatches =
        json_usize(&prepared, &["atom_prepared_margin_parity_mismatches"]).unwrap_or(usize::MAX);
    let prepared_decision_parity_mismatches =
        json_usize(&prepared, &["atom_prepared_decision_parity_mismatches"]).unwrap_or(usize::MAX);
    let prepared_p99_ns =
        json_u64(&prepared, &["prepared_p99_score_latency_ns"]).unwrap_or(u64::MAX);
    let prepared_p99_budget_ns = 1_000u64;
    let prepared_clean = json_string(&prepared, &["report_kind"]).as_deref()
        == Some("phase_stream_live_store_prepared_hot_pack_v1")
        && json_string(&prepared, &["verdict"]).as_deref()
            == Some("LIVE_STORE_PREPARED_HOT_PACK_PASS")
        && json_string(&prepared, &["blocker"]).as_deref() == Some("none")
        && prepared_score_events > 0
        && prepared_score_candidate_events > 0
        && prepared_verifier_required_events == prepared_score_candidate_events
        && prepared_local_accept_events == 0
        && prepared_accepts > 0
        && prepared_tokens > 0
        && prepared_false_accepts == 0
        && prepared_margin_parity_mismatches == 0
        && prepared_decision_parity_mismatches == 0
        && prepared_p99_ns <= prepared_p99_budget_ns
        && !json_bool(&prepared, &["hot_loop_json_used"]).unwrap_or(true)
        && !json_bool(&prepared, &["hot_loop_string_route_used"]).unwrap_or(true)
        && !json_bool(&prepared, &["hot_loop_btreemap_used"]).unwrap_or(true)
        && !json_bool(&prepared, &["hot_loop_file_io_used"]).unwrap_or(true)
        && !json_bool(&prepared, &["local_accept_enabled"]).unwrap_or(true)
        && !json_bool(&prepared, &["market_money_claim_allowed"]).unwrap_or(true)
        && forbidden_flags_false(&prepared);

    let memory_score_events = json_usize(&memory_worker, &["score_events"]).unwrap_or(0);
    let memory_score_candidate_events =
        json_usize(&memory_worker, &["score_candidate_events"]).unwrap_or(0);
    let memory_verifier_required_events =
        json_usize(&memory_worker, &["verifier_required_events"]).unwrap_or(0);
    let memory_local_accept_events =
        json_usize(&memory_worker, &["local_accept_events"]).unwrap_or(usize::MAX);
    let memory_accepts =
        json_usize(&memory_worker, &["unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let memory_tokens = json_u64(&memory_worker, &["tokens_saved"]).unwrap_or(0);
    let memory_cost = json_u64(&memory_worker, &["cost_saved_microusd"]).unwrap_or(0);
    let memory_false_accepts = json_usize(&memory_worker, &["false_accepts"]).unwrap_or(usize::MAX);
    let memory_p99_ns = json_u64(&memory_worker, &["p99_score_latency_ns"]).unwrap_or(u64::MAX);
    let memory_p99_budget_ns = 1_000u64;
    let memory_worker_clean = json_string(&memory_worker, &["report_kind"]).as_deref()
        == Some("phase_stream_live_worker_memory_smoke_v1")
        && json_string(&memory_worker, &["verdict"]).as_deref()
            == Some("LIVE_WORKER_MEMORY_SMOKE_PASS")
        && json_string(&memory_worker, &["blocker"]).as_deref() == Some("none")
        && memory_score_events > 0
        && memory_score_candidate_events > 0
        && memory_verifier_required_events == memory_score_candidate_events
        && memory_local_accept_events == 0
        && memory_accepts > 0
        && memory_tokens > 0
        && memory_false_accepts == 0
        && memory_p99_ns <= memory_p99_budget_ns
        && !json_bool(&memory_worker, &["hot_loop_json_used"]).unwrap_or(true)
        && !json_bool(&memory_worker, &["hot_loop_string_route_used"]).unwrap_or(true)
        && !json_bool(&memory_worker, &["hot_loop_btreemap_used"]).unwrap_or(true)
        && !json_bool(&memory_worker, &["hot_loop_file_io_used"]).unwrap_or(true)
        && !json_bool(&memory_worker, &["hot_loop_package_compile_used"]).unwrap_or(true)
        && !json_bool(&memory_worker, &["local_accept_enabled"]).unwrap_or(true)
        && !json_bool(&memory_worker, &["market_money_claim_allowed"]).unwrap_or(true)
        && forbidden_flags_false(&memory_worker);

    let policy_accepts = json_usize(&policy, &["unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let policy_tokens = json_u64(&policy, &["tokens_saved"]).unwrap_or(0);
    let policy_cost = json_u64(&policy, &["cost_saved_microusd"]).unwrap_or(0);
    let policy_false_accepts = json_usize(&policy, &["false_accepts"]).unwrap_or(usize::MAX);
    let policy_prepared_parity = policy_accepts == prepared_accepts
        && policy_tokens == prepared_tokens
        && policy_cost == prepared_cost
        && policy_false_accepts == prepared_false_accepts;
    let prepared_memory_parity = prepared_score_events == memory_score_events
        && prepared_score_candidate_events == memory_score_candidate_events
        && prepared_verifier_required_events == memory_verifier_required_events
        && prepared_accepts == memory_accepts
        && prepared_tokens == memory_tokens
        && prepared_cost == memory_cost
        && prepared_false_accepts == memory_false_accepts;

    let prepared_shadow_safety_review_passed = stage_ready
        && policy_ready
        && prepared_clean
        && memory_worker_clean
        && policy_prepared_parity
        && prepared_memory_parity;
    let prepared_shadow_hot_latency_passed = prepared_shadow_safety_review_passed
        && prepared_p99_ns <= prepared_p99_budget_ns
        && memory_p99_ns <= memory_p99_budget_ns;
    let prepared_shadow_review_allowed =
        prepared_shadow_safety_review_passed && prepared_shadow_hot_latency_passed;

    let mut blockers = Vec::<&'static str>::new();
    if !stage_ready {
        blockers.push("live_policy_stage_not_ready");
    }
    if !policy_ready {
        blockers.push("live_policy_artifact_not_ready");
    }
    if !prepared_clean {
        blockers.push("prepared_hot_pack_gate_not_clean");
    }
    if !memory_worker_clean {
        blockers.push("prepared_memory_worker_gate_not_clean");
    }
    if !policy_prepared_parity {
        blockers.push("policy_prepared_metric_parity_mismatch");
    }
    if !prepared_memory_parity {
        blockers.push("prepared_memory_metric_parity_mismatch");
    }
    if prepared_p99_ns > prepared_p99_budget_ns {
        blockers.push("prepared_hot_pack_p99_budget_exceeded");
    }
    if memory_p99_ns > memory_p99_budget_ns {
        blockers.push("prepared_memory_worker_p99_budget_exceeded");
    }

    let verdict = if prepared_shadow_review_allowed {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_PREPARED_POLICY_SHADOW_REVIEW_V1_PASS"
    } else if prepared_shadow_safety_review_passed {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_PREPARED_POLICY_SHADOW_REVIEW_V1_WATCH_LATENCY"
    } else {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_PREPARED_POLICY_SHADOW_REVIEW_V1_WATCH"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_live_store_clean_manifest_prepared_policy_shadow_review_v1",
        "mode": "live_policy_artifact_to_prepared_numeric_worker_shadow_review",
        "inputs": {
            "stage_report_path": stage_report_path,
            "policy_path": policy_path,
            "prepared_report_path": prepared_report_path,
            "memory_worker_report_path": memory_worker_report_path
        },
        "gates": {
            "stage_ready": stage_ready,
            "policy_ready": policy_ready,
            "prepared_clean": prepared_clean,
            "memory_worker_clean": memory_worker_clean,
            "policy_prepared_parity": policy_prepared_parity,
            "prepared_memory_parity": prepared_memory_parity
        },
        "prepared_shadow_safety_review_passed": prepared_shadow_safety_review_passed,
        "prepared_shadow_hot_latency_passed": prepared_shadow_hot_latency_passed,
        "prepared_shadow_review_allowed": prepared_shadow_review_allowed,
        "blockers": blockers,
        "profile_refs": {
            "hot_route_ids": policy.get("hot_route_ids").cloned().unwrap_or(Value::Null),
            "hot_profile_ids": policy.get("hot_profile_ids").cloned().unwrap_or(Value::Null),
            "package_paths": policy.get("package_paths").cloned().unwrap_or(Value::Null)
        },
        "metrics": {
            "score_events": memory_score_events,
            "score_candidate_events": memory_score_candidate_events,
            "unique_cpu_accepts_over_exact_cache": memory_accepts,
            "tokens_saved": memory_tokens,
            "cost_saved_microusd": memory_cost,
            "false_accepts": memory_false_accepts,
            "local_accept_events": memory_local_accept_events
        },
        "latency": {
            "prepared_p99_score_latency_ns": prepared_p99_ns,
            "prepared_p99_budget_ns": prepared_p99_budget_ns,
            "memory_worker_p99_score_latency_ns": memory_p99_ns,
            "memory_worker_p99_budget_ns": memory_p99_budget_ns
        },
        "hot_loop": {
            "json_used": false,
            "btreemap_used": false,
            "string_route_used": false,
            "file_io_used": false,
            "package_compile_used": false
        },
        "mutation_flags": {
            "registry_mutation_enabled": false,
            "cpu_profile_registry_write_enabled": false,
            "serving_profile_artifact_written": false,
            "product_promotion_enabled": false
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
        "market_money_claim_allowed": false,
        "verdict": verdict,
        "boundary": "prepared policy shadow review only: consumes shadow-only live policy, prepared hot pack, and prepared memory-worker evidence; no source JSON parsing inside timed hot loops, no registry mutation, no serving profile write, no local_accept, no market money claim, and no legacy nwrb"
    });
    write_json_file(&report_path, &report)?;

    println!("phase_stream_live_store_clean_manifest_prepared_policy_shadow_review_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  prepared_shadow_safety_review_passed: {prepared_shadow_safety_review_passed}");
    println!("  prepared_shadow_hot_latency_passed: {prepared_shadow_hot_latency_passed}");
    println!("  unique_cpu_accepts_over_exact_cache: {memory_accepts}");
    println!("  false_accepts: {memory_false_accepts}");
    println!("  prepared_p99_score_latency_ns: {prepared_p99_ns}");
    println!("  memory_worker_p99_score_latency_ns: {memory_p99_ns}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn json_array_empty(value: &Value, path: &[&str]) -> bool {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
}

fn forbidden_flags_false(value: &Value) -> bool {
    let Some(flags) = value.get("forbidden_flags") else {
        return false;
    };
    [
        "target_id_used",
        "proof_rule_id_authority_used",
        "concrete_x_lookup_used",
        "manual_local_out_t_used",
        "hidden_frame_id_or_bind_x_used",
        "legacy_backend_used",
        "nwrb_used",
        "role_binding_backend_used",
        "lookup_used",
        "local_accept_without_verifier_used",
    ]
    .into_iter()
    .all(|key| !flags.get(key).and_then(Value::as_bool).unwrap_or(false))
}
