use std::path::PathBuf;
use std::time::Instant;

use nando_core::{
    PhaseCenterHotRequestEvidence, PhaseCenterHotRouteTable, PhaseCenterHotRuntime,
    PhaseCenterHotShadowEval, PhaseCenterHotWorker, PhaseCenterOffloadPolicy,
    PhaseCenterOffloadRuntime, PhaseCenterPreparedHotEvidenceRow, PhaseCenterPreparedHotRequest,
    phase_vector_from_atom_ids,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{json_bool, json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-shadow-registry-replay-v1.report.json";
const DEFAULT_HANDOFF_REPORT: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-shadow-registry-handoff-v1-current.report.json";
const DEFAULT_PREPARED_HOT_PACK: &str =
    "target/nando-wave/streaming/phase-stream-live-store-prepared-hot-pack-v1.json";

#[derive(Clone, Deserialize)]
struct PreparedHotPack {
    cells: usize,
    rows: Vec<PreparedHotPackRow>,
}

#[derive(Clone, Deserialize)]
struct PreparedHotPackRow {
    route_id: u32,
    atom_ids: Vec<u64>,
    verified_safe_accept: bool,
    exact_cache_hit: bool,
    tokens: u64,
    cost_microusd: u64,
}

#[derive(Clone)]
struct RegistryPackageSpec {
    registry_package_path: String,
    route_id: u32,
    profile_id: u32,
    threshold_micro: i64,
    expected_package_fingerprint64: u64,
    expected_package_bytes: usize,
    expected_package_records: usize,
}

#[derive(Serialize)]
struct RegistryReplayPackageReport {
    registry_package_path: String,
    route_id: u32,
    profile_id: u32,
    threshold_micro: i64,
    package_fingerprint64: u64,
    package_bytes: usize,
    package_records: usize,
    package_matches_handoff: bool,
    matching_rows: usize,
    score_events: usize,
    score_candidate_events: usize,
    verifier_required_events: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    tokens_saved: u64,
    cost_saved_microusd: u64,
    false_accepts: usize,
    local_accept_events: usize,
    runtime_margin_parity_checks: usize,
    runtime_margin_parity_mismatches: usize,
    runtime_decision_parity_mismatches: usize,
    p99_score_latency_ns: u128,
    blockers: Vec<String>,
}

#[derive(Serialize)]
struct CleanManifestShadowRegistryReplayReport {
    report_kind: &'static str,
    mode: &'static str,
    handoff_report_path: String,
    prepared_hot_pack_path: String,
    input_handoff_allowed: bool,
    input_forbidden_flags_clear: bool,
    input_unique_accepts_over_exact_cache: usize,
    input_tokens_saved: u64,
    input_cost_saved_microusd: u64,
    input_false_accepts: usize,
    pack_rows: usize,
    package_count: usize,
    clean_package_count: usize,
    score_events: usize,
    score_candidate_events: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    tokens_saved: u64,
    cost_saved_microusd: u64,
    false_accepts: usize,
    local_accept_events: usize,
    runtime_margin_parity_checks: usize,
    runtime_margin_parity_mismatches: usize,
    runtime_decision_parity_mismatches: usize,
    replay_matches_handoff_metrics: bool,
    p99_score_latency_ns: u128,
    p99_budget_ns: u128,
    packages: Vec<RegistryReplayPackageReport>,
    shadow_registry_replay_allowed: bool,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    serving_registry_mutated: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: serde_json::Value,
    verdict: &'static str,
    boundary: &'static str,
}

pub(crate) fn run_phase_stream_live_store_clean_manifest_shadow_registry_replay_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REPORT));
    let handoff_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HANDOFF_REPORT));
    let prepared_hot_pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PREPARED_HOT_PACK));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let handoff = read_json_value(&handoff_report_path)?;
    let input_handoff_allowed = json_bool(&handoff, &["shadow_registry_handoff_allowed"])
        .unwrap_or(false)
        && json_string(&handoff, &["verdict"]).as_deref()
            == Some("PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REGISTRY_HANDOFF_V1_PASS")
        && json_bool(&handoff, &["local_accept_enabled"]) == Some(false)
        && json_bool(&handoff, &["market_money_claim_allowed"]) == Some(false)
        && json_bool(&handoff, &["serving_registry_mutated"]) == Some(false);
    let input_forbidden_flags_clear = handoff
        .get("forbidden_flags")
        .is_some_and(forbidden_flags_all_bool_false);
    let input_unique_accepts_over_exact_cache =
        json_usize(&handoff, &["input_unique_accepts_over_exact_cache"]).unwrap_or(0);
    let input_tokens_saved = json_u64(&handoff, &["input_tokens_saved"]).unwrap_or(0);
    let input_cost_saved_microusd = json_u64(&handoff, &["input_cost_saved_microusd"]).unwrap_or(0);
    let input_false_accepts = json_usize(&handoff, &["input_false_accepts"]).unwrap_or(usize::MAX);
    let package_specs = registry_package_specs(&handoff);

    let pack_text = std::fs::read_to_string(&prepared_hot_pack_path).map_err(|error| {
        format!(
            "failed to read prepared hot pack '{}': {error}",
            prepared_hot_pack_path.display()
        )
    })?;
    let pack = serde_json::from_str::<PreparedHotPack>(&pack_text).map_err(|error| {
        format!(
            "failed to parse prepared hot pack '{}': {error}",
            prepared_hot_pack_path.display()
        )
    })?;

    let mut packages = Vec::new();
    for spec in &package_specs {
        packages.push(replay_registry_package(spec, &pack)?);
    }

    let package_count = packages.len();
    let clean_package_count = packages
        .iter()
        .filter(|package| package.blockers.is_empty())
        .count();
    let score_events = packages.iter().map(|package| package.score_events).sum();
    let score_candidate_events = packages
        .iter()
        .map(|package| package.score_candidate_events)
        .sum();
    let unique_cpu_accepts_over_exact_cache = packages
        .iter()
        .map(|package| package.unique_cpu_accepts_over_exact_cache)
        .sum();
    let tokens_saved = packages.iter().map(|package| package.tokens_saved).sum();
    let cost_saved_microusd = packages
        .iter()
        .map(|package| package.cost_saved_microusd)
        .sum();
    let false_accepts = packages.iter().map(|package| package.false_accepts).sum();
    let local_accept_events = packages
        .iter()
        .map(|package| package.local_accept_events)
        .sum();
    let runtime_margin_parity_checks = packages
        .iter()
        .map(|package| package.runtime_margin_parity_checks)
        .sum();
    let runtime_margin_parity_mismatches = packages
        .iter()
        .map(|package| package.runtime_margin_parity_mismatches)
        .sum();
    let runtime_decision_parity_mismatches = packages
        .iter()
        .map(|package| package.runtime_decision_parity_mismatches)
        .sum();
    let p99_score_latency_ns = packages
        .iter()
        .map(|package| package.p99_score_latency_ns)
        .max()
        .unwrap_or(0);
    let p99_budget_ns = 1_000u128;
    let replay_matches_handoff_metrics = unique_cpu_accepts_over_exact_cache
        == input_unique_accepts_over_exact_cache
        && tokens_saved == input_tokens_saved
        && cost_saved_microusd == input_cost_saved_microusd
        && false_accepts == input_false_accepts;
    let shadow_registry_replay_allowed = input_handoff_allowed
        && input_forbidden_flags_clear
        && clean_package_count > 0
        && clean_package_count == package_count
        && replay_matches_handoff_metrics
        && false_accepts == 0
        && local_accept_events == 0
        && runtime_margin_parity_mismatches == 0
        && runtime_decision_parity_mismatches == 0
        && p99_score_latency_ns <= p99_budget_ns;
    let verdict = if shadow_registry_replay_allowed {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REGISTRY_REPLAY_V1_PASS"
    } else if false_accepts > 0 {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REGISTRY_REPLAY_V1_FAIL_FALSE_ACCEPTS"
    } else if !replay_matches_handoff_metrics
        || runtime_margin_parity_mismatches > 0
        || runtime_decision_parity_mismatches > 0
    {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REGISTRY_REPLAY_V1_FAIL_REPLAY_MISMATCH"
    } else if p99_score_latency_ns > p99_budget_ns {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REGISTRY_REPLAY_V1_WATCH_LATENCY"
    } else {
        "PHASE_STREAM_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REGISTRY_REPLAY_V1_WATCH"
    };

    let report = CleanManifestShadowRegistryReplayReport {
        report_kind: "phase_stream_live_store_clean_manifest_shadow_registry_replay_v1",
        mode: "shadow_registry_runtime_replay_only",
        handoff_report_path: handoff_report_path.display().to_string(),
        prepared_hot_pack_path: prepared_hot_pack_path.display().to_string(),
        input_handoff_allowed,
        input_forbidden_flags_clear,
        input_unique_accepts_over_exact_cache,
        input_tokens_saved,
        input_cost_saved_microusd,
        input_false_accepts,
        pack_rows: pack.rows.len(),
        package_count,
        clean_package_count,
        score_events,
        score_candidate_events,
        unique_cpu_accepts_over_exact_cache,
        tokens_saved,
        cost_saved_microusd,
        false_accepts,
        local_accept_events,
        runtime_margin_parity_checks,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
        replay_matches_handoff_metrics,
        p99_score_latency_ns,
        p99_budget_ns,
        packages,
        shadow_registry_replay_allowed,
        local_accept_enabled: false,
        auto_promote_enabled: false,
        serving_registry_mutated: false,
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
        boundary: "shadow registry runtime replay only: reloads copied verifier-bound .nwpc packages from the shadow registry and replays prepared numeric rows from the same real trace; it does not mutate serving registry, enable local_accept, auto-promote, claim market money, or use legacy nwrb/role-binding paths",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_stream_live_store_clean_manifest_shadow_registry_replay_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  clean_package_count: {clean_package_count}");
    println!("  unique_cpu_accepts_over_exact_cache: {unique_cpu_accepts_over_exact_cache}");
    println!("  false_accepts: {false_accepts}");
    println!("  replay_matches_handoff_metrics: {replay_matches_handoff_metrics}");
    println!("  p99_score_latency_ns: {p99_score_latency_ns}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn replay_registry_package(
    spec: &RegistryPackageSpec,
    pack: &PreparedHotPack,
) -> Result<RegistryReplayPackageReport, String> {
    let package_bytes = std::fs::read(&spec.registry_package_path).map_err(|error| {
        format!(
            "failed to read clean manifest shadow registry package '{}': {error}",
            spec.registry_package_path
        )
    })?;
    let package_info =
        PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes).map_err(|error| {
            format!(
                "failed to inspect clean manifest shadow registry package '{}': {error:?}",
                spec.registry_package_path
            )
        })?;
    let offload_runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &package_bytes,
        PhaseCenterOffloadPolicy::new(spec.threshold_micro)
            .map_err(|error| format!("invalid clean registry replay threshold: {error:?}"))?,
    )
    .map_err(|error| {
        format!(
            "failed to load clean manifest shadow registry package '{}': {error:?}",
            spec.registry_package_path
        )
    })?;
    let package_matches_handoff = package_info.fingerprint64 == spec.expected_package_fingerprint64
        && package_bytes.len() == spec.expected_package_bytes
        && package_info.record_count == spec.expected_package_records;
    let profile_ids = [spec.profile_id];
    let thresholds = [spec.threshold_micro];
    let hot_runtime = PhaseCenterHotRuntime::from_flat_runtime(
        offload_runtime.runtime(),
        &profile_ids,
        &thresholds,
    )
    .map_err(|error| format!("clean registry hot runtime build error: {error:?}"))?;
    let route_plan = hot_runtime
        .route_plan_from_profile_ids(spec.route_id, profile_ids)
        .map_err(|error| format!("clean registry route plan error: {error:?}"))?
        .ok_or_else(|| "clean registry hot route has no profiles".to_owned())?;
    let route_table = PhaseCenterHotRouteTable::from_plans([route_plan])
        .map_err(|error| format!("clean registry route table error: {error:?}"))?;
    let mut worker = PhaseCenterHotWorker::new(hot_runtime, route_table)
        .map_err(|error| format!("clean registry hot worker error: {error:?}"))?;
    let mut eval = PhaseCenterHotShadowEval::default();
    let mut runtime_margin_parity_checks = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut runtime_decision_parity_mismatches = 0usize;
    let mut prepared_rows = Vec::<PhaseCenterPreparedHotEvidenceRow>::new();
    let mut matching_rows = 0usize;

    for row in pack.rows.iter().filter(|row| row.route_id == spec.route_id) {
        matching_rows += 1;
        let Some(route_index) = worker.resolve_route_index(row.route_id) else {
            runtime_decision_parity_mismatches += 1;
            continue;
        };
        let phase_vector = phase_vector_from_atom_ids(row.atom_ids.iter().copied(), pack.cells);
        let reference_margin_micro = offload_runtime
            .runtime()
            .score_vector_margin_micro(0, &phase_vector)
            .map_err(|error| format!("clean registry reference score error: {error:?}"))?;
        let reference_score_candidate = reference_margin_micro >= spec.threshold_micro;
        let evidence = PhaseCenterHotRequestEvidence {
            verified_safe_accept: row.verified_safe_accept,
            exact_cache_hit: row.exact_cache_hit,
            tokens: row.tokens,
            cost_microusd: row.cost_microusd,
        };
        let prepared_row = PhaseCenterPreparedHotEvidenceRow::new(
            route_index,
            row.atom_ids.clone(),
            phase_vector,
            evidence,
        );
        let decisions = worker
            .score_prepared_row_with_evidence(&prepared_row, &mut eval)
            .map_err(|error| format!("clean registry prepared score error: {error:?}"))?;
        let mut matched_profile = false;
        for decision in decisions {
            if decision.profile_id != spec.profile_id {
                continue;
            }
            matched_profile = true;
            runtime_margin_parity_checks += 1;
            if decision.margin_micro != reference_margin_micro {
                runtime_margin_parity_mismatches += 1;
            }
            if decision.score_candidate != reference_score_candidate {
                runtime_decision_parity_mismatches += 1;
            }
        }
        if !matched_profile {
            runtime_decision_parity_mismatches += 1;
        }
        prepared_rows.push(prepared_row);
    }

    let latency_repeats = 1000usize;
    let mut latencies = Vec::<u128>::with_capacity(prepared_rows.len() * latency_repeats);
    for _ in 0..latency_repeats {
        for row in &prepared_rows {
            let started = Instant::now();
            let _ = worker
                .score_prepared(PhaseCenterPreparedHotRequest::new(
                    row.route_index,
                    &row.phase_vector,
                ))
                .map_err(|error| {
                    format!("clean registry prepared latency score error: {error:?}")
                })?;
            latencies.push(started.elapsed().as_nanos());
        }
    }
    latencies.sort_unstable();
    let p99_score_latency_ns = latency_percentile(&latencies, 99);
    let mut blockers = Vec::new();
    if !package_matches_handoff {
        blockers.push("package_handoff_metadata_mismatch".to_owned());
    }
    if matching_rows == 0 {
        blockers.push("no_matching_prepared_rows".to_owned());
    }
    if eval.false_accepts != 0 {
        blockers.push("false_accepts_nonzero".to_owned());
    }
    if eval.local_accept_events != 0 {
        blockers.push("local_accept_events_nonzero".to_owned());
    }
    if runtime_margin_parity_mismatches != 0 {
        blockers.push("runtime_margin_parity_mismatch".to_owned());
    }
    if runtime_decision_parity_mismatches != 0 {
        blockers.push("runtime_decision_parity_mismatch".to_owned());
    }
    if p99_score_latency_ns > 1_000 {
        blockers.push("p99_budget_exceeded".to_owned());
    }
    blockers.sort();
    blockers.dedup();

    Ok(RegistryReplayPackageReport {
        registry_package_path: spec.registry_package_path.clone(),
        route_id: spec.route_id,
        profile_id: spec.profile_id,
        threshold_micro: spec.threshold_micro,
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: package_bytes.len(),
        package_records: package_info.record_count,
        package_matches_handoff,
        matching_rows,
        score_events: eval.score_events,
        score_candidate_events: eval.score_candidate_events,
        verifier_required_events: eval.verifier_required_events,
        unique_cpu_accepts_over_exact_cache: eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: eval.tokens_saved,
        cost_saved_microusd: eval.cost_saved_microusd,
        false_accepts: eval.false_accepts,
        local_accept_events: eval.local_accept_events,
        runtime_margin_parity_checks,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
        p99_score_latency_ns,
        blockers,
    })
}

fn registry_package_specs(handoff: &Value) -> Vec<RegistryPackageSpec> {
    handoff
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|package| json_bool(package, &["accepted_for_shadow_registry"]) == Some(true))
        .filter_map(|package| {
            Some(RegistryPackageSpec {
                registry_package_path: json_string(package, &["registry_package_path"])?,
                route_id: json_u32(package, &["route_id"])?,
                profile_id: json_u32(package, &["profile_id"])?,
                threshold_micro: json_i64(package, &["threshold_micro"])?,
                expected_package_fingerprint64: json_u64(package, &["package_fingerprint64"])?,
                expected_package_bytes: json_usize(package, &["package_bytes"])?,
                expected_package_records: json_usize(package, &["package_records"])?,
            })
        })
        .collect()
}

fn latency_percentile(latencies: &[u128], percentile: usize) -> u128 {
    if latencies.is_empty() {
        return 0;
    }
    let index = latencies
        .len()
        .saturating_mul(percentile)
        .checked_div(100)
        .unwrap_or(0)
        .min(latencies.len() - 1);
    latencies[index]
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_u64(value, path).and_then(|number| usize::try_from(number).ok())
}

fn json_u32(value: &Value, path: &[&str]) -> Option<u32> {
    json_u64(value, path).and_then(|number| u32::try_from(number).ok())
}

fn json_i64(value: &Value, path: &[&str]) -> Option<i64> {
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
