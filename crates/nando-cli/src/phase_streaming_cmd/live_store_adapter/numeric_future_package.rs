use std::path::{Path, PathBuf};
use std::time::Instant;

use nando_core::{
    PhaseCenterAtomEncoder, PhaseCenterHotRouteTable, PhaseCenterHotRuntime, PhaseCenterHotWorker,
    PhaseCenterOffloadPolicy, PhaseCenterOffloadRuntime, PhaseCenterOnlineCandidatePackage,
};

use super::paths::{
    live_store_hot_path_daemon_admission_policy_path,
    live_store_numeric_future_candidate_package_dir,
};
use super::reports::{
    PhaseStreamHotPathDaemonAdmissionPolicyReport,
    PhaseStreamHotPathDaemonNumericFuturePackageAuditReport,
};
use super::runtime_metrics::live_store_latency_percentile;
use super::source_events::LiveStoreParsedAtomEvent;
use super::state::LiveStoreHotPathDenominator;
use super::worker_path::{LiveStorePreparedHotPackEval, LiveStorePreparedMemoryRow};

pub(super) struct LiveStoreFrozenNumericFuturePackage {
    pub(super) package: PhaseCenterOnlineCandidatePackage,
    pub(super) route_id: u32,
    pub(super) freeze_after_append_events: usize,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn live_store_write_numeric_future_package_audit_from_frozen(
    report_path: &Path,
    watermark_trace_paths: &[PathBuf],
    append_trace_paths: &[PathBuf],
    cells: usize,
    min_bucket_events: usize,
    watermark_parsed_rows: usize,
    append_parsed_rows: usize,
    append_events: &[LiveStoreParsedAtomEvent],
    frozen: LiveStoreFrozenNumericFuturePackage,
) -> Result<(), String> {
    let daemon_admission_policy_report_path =
        live_store_hot_path_daemon_admission_policy_path(report_path);
    let candidate_package_dir = live_store_numeric_future_candidate_package_dir(report_path);
    std::fs::create_dir_all(&candidate_package_dir).map_err(|error| {
        format!(
            "failed to create future package candidate dir '{}': {error}",
            candidate_package_dir.display()
        )
    })?;

    let package = frozen.package;
    let candidate_bucket_id = package.bucket_id;
    let threshold_micro = package.threshold_micro;
    let package_fingerprint64 = package.package_info.fingerprint64;
    let package_record_count = package.package_info.record_count;
    let package_bytes_len = package.package_bytes.len();
    let verifier_binding_bound = package.verifier_binding.is_bound();
    let candidate_package_path = candidate_package_dir.join(format!(
        "bucket-{:08x}-{:016x}.nwpc",
        candidate_bucket_id, package_fingerprint64
    ));
    std::fs::write(&candidate_package_path, &package.package_bytes).map_err(|error| {
        format!(
            "failed to write future candidate package '{}': {error}",
            candidate_package_path.display()
        )
    })?;

    let runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &package.package_bytes,
        PhaseCenterOffloadPolicy::new(threshold_micro)
            .map_err(|error| format!("future portfolio package policy error: {error:?}"))?,
    )
    .map_err(|error| format!("future portfolio package load error: {error:?}"))?;
    let hot_runtime = PhaseCenterHotRuntime::from_flat_runtime(
        runtime.runtime(),
        &[candidate_bucket_id],
        &[threshold_micro],
    )
    .map_err(|error| format!("future portfolio package hot runtime build error: {error:?}"))?;
    let route_plan = hot_runtime
        .route_plan_from_profile_ids(frozen.route_id, [candidate_bucket_id])
        .map_err(|error| format!("future portfolio package route plan error: {error:?}"))?
        .ok_or_else(|| "future portfolio package hot route has no profiles".to_owned())?;
    let route_table = PhaseCenterHotRouteTable::from_plans([route_plan])
        .map_err(|error| format!("future portfolio package route table error: {error:?}"))?;
    let mut worker = PhaseCenterHotWorker::new(hot_runtime, route_table)
        .map_err(|error| format!("future portfolio package hot worker error: {error:?}"))?;
    let mut reference_encoder = PhaseCenterAtomEncoder::new(cells).map_err(|error| {
        format!("failed to create future portfolio package reference encoder: {error:?}")
    })?;

    let mut eval = LiveStorePreparedHotPackEval::default();
    let mut future_denominator = LiveStoreHotPathDenominator::default();
    let mut future_matching_bucket_events = 0usize;
    let mut future_non_matching_bucket_events = 0usize;
    let mut runtime_margin_parity_checks = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut runtime_decision_parity_mismatches = 0usize;
    let mut latencies = Vec::new();
    let future_events = append_events
        .get(frozen.freeze_after_append_events..)
        .unwrap_or_default();
    for event in future_events {
        future_denominator.total_tokens =
            future_denominator.total_tokens.saturating_add(event.tokens);
        future_denominator.total_cost_microusd = future_denominator
            .total_cost_microusd
            .saturating_add(event.cost_microusd);
        if event.exact_cache_hit {
            future_denominator.exact_cache_hits += 1;
            future_denominator.exact_cache_tokens = future_denominator
                .exact_cache_tokens
                .saturating_add(event.tokens);
            future_denominator.exact_cache_cost_microusd = future_denominator
                .exact_cache_cost_microusd
                .saturating_add(event.cost_microusd);
        } else {
            future_denominator.non_exact_rows += 1;
        }
        if event.bucket_id != candidate_bucket_id {
            future_non_matching_bucket_events += 1;
            continue;
        }
        future_matching_bucket_events += 1;
        let Some(route_index) = worker.resolve_route_index(event.route_id) else {
            runtime_decision_parity_mismatches += 1;
            continue;
        };
        let vector = reference_encoder
            .encode_atom_ids(event.atom_ids.iter().copied())
            .map_err(|error| format!("future portfolio reference encode error: {error:?}"))?
            .to_vec();
        let reference_margin_micro = runtime
            .runtime()
            .score_vector_margin_micro(0, &vector)
            .map_err(|error| format!("future portfolio reference score error: {error:?}"))?;
        let reference_score_candidate = reference_margin_micro >= threshold_micro;
        let row = LiveStorePreparedMemoryRow::new(
            route_index,
            event.atom_ids.clone(),
            vector,
            event.hot_request_evidence(),
        );
        let started = Instant::now();
        let decisions = worker
            .score_prepared_row_with_evidence(&row, &mut eval)
            .map_err(|error| format!("future portfolio prepared score error: {error:?}"))?;
        latencies.push(started.elapsed().as_nanos());
        let mut matched_profile = false;
        for decision in decisions {
            if decision.profile_id != candidate_bucket_id {
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
    }
    latencies.sort_unstable();

    let exact_cache_overlap_excluded = future_denominator.non_exact_rows > 0
        && eval.unique_cpu_accepts_over_exact_cache <= future_denominator.non_exact_rows;
    let token_cost_denominator_present =
        future_denominator.total_tokens > 0 && future_denominator.total_cost_microusd > 0;
    let fresh_future_split_used = frozen.freeze_after_append_events > 0
        && frozen.freeze_after_append_events < append_events.len();
    let p99_latency_ns = live_store_latency_percentile(&latencies, 99);
    let p99_budget_ns = 1_000;
    let hot_profile_count = worker.profile_count();
    let hot_route_count = worker.route_count();
    let hot_route_profile_edges = worker.route_profile_edge_count();
    let hot_bytes_estimate = worker.bytes_estimate();
    let gate_passed = fresh_future_split_used
        && verifier_binding_bound
        && package_record_count == 1
        && future_matching_bucket_events > 0
        && eval.score_events > 0
        && eval.score_candidate_events > 0
        && eval.unique_cpu_accepts_over_exact_cache > 0
        && eval.tokens_saved > 0
        && eval.cost_saved_microusd > 0
        && eval.false_accepts == 0
        && eval.local_accept_events == 0
        && runtime_margin_parity_checks == eval.score_events
        && runtime_margin_parity_mismatches == 0
        && runtime_decision_parity_mismatches == 0
        && exact_cache_overlap_excluded
        && token_cost_denominator_present
        && p99_latency_ns <= p99_budget_ns;
    let blocker = if gate_passed {
        "none".to_owned()
    } else if !fresh_future_split_used {
        "future_package_no_rows_after_freeze".to_owned()
    } else if !verifier_binding_bound {
        "future_package_not_verifier_bound".to_owned()
    } else if package_record_count != 1 {
        "future_package_record_count_not_one".to_owned()
    } else if future_matching_bucket_events == 0 {
        "future_package_no_matching_future_events".to_owned()
    } else if eval.score_events == 0 || eval.score_candidate_events == 0 {
        "future_package_no_score_candidates".to_owned()
    } else if eval.unique_cpu_accepts_over_exact_cache == 0 {
        "future_package_no_unique_cpu_accepts_over_exact_cache".to_owned()
    } else if eval.false_accepts != 0 {
        "future_package_false_accepts_nonzero".to_owned()
    } else if eval.local_accept_events != 0 {
        "future_package_local_accept_enabled".to_owned()
    } else if runtime_margin_parity_checks != eval.score_events
        || runtime_margin_parity_mismatches != 0
        || runtime_decision_parity_mismatches != 0
    {
        "future_package_runtime_parity_failed".to_owned()
    } else if !exact_cache_overlap_excluded || !token_cost_denominator_present {
        "future_package_denominator_missing".to_owned()
    } else if p99_latency_ns > p99_budget_ns {
        "future_package_hot_path_p99_budget_exceeded".to_owned()
    } else {
        "future_package_audit_failed".to_owned()
    };

    let report = PhaseStreamHotPathDaemonNumericFuturePackageAuditReport {
        report_kind: "phase_stream_hot_path_daemon_numeric_future_package_audit_v1",
        mode: "freeze_quarantine_nwpc_before_future_rows_numeric_ids_only",
        daemon_admission_policy_report_path: daemon_admission_policy_report_path
            .display()
            .to_string(),
        watermark_trace_paths: watermark_trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        append_trace_paths: append_trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        cells,
        min_bucket_events,
        watermark_parsed_rows,
        append_parsed_rows,
        freeze_after_append_events: frozen.freeze_after_append_events,
        candidate_bucket_id,
        candidate_route_id: frozen.route_id,
        candidate_package_path: candidate_package_path.display().to_string(),
        package_fingerprint64,
        package_bytes: package_bytes_len,
        package_record_count,
        threshold_micro,
        verifier_binding_bound,
        future_total_events_after_freeze: future_events.len(),
        future_matching_bucket_events,
        future_non_matching_bucket_events,
        score_events: eval.score_events,
        score_candidate_events: eval.score_candidate_events,
        verifier_required_events: eval.verifier_required_events,
        local_accept_events: eval.local_accept_events,
        unique_cpu_accepts_over_exact_cache: eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: eval.tokens_saved,
        cost_saved_microusd: eval.cost_saved_microusd,
        false_accepts: eval.false_accepts,
        runtime_margin_parity_checks,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
        future_total_tokens: future_denominator.total_tokens,
        future_total_cost_microusd: future_denominator.total_cost_microusd,
        future_exact_cache_hits: future_denominator.exact_cache_hits,
        future_exact_cache_tokens: future_denominator.exact_cache_tokens,
        future_exact_cache_cost_microusd: future_denominator.exact_cache_cost_microusd,
        future_non_exact_rows: future_denominator.non_exact_rows,
        p50_latency_ns: live_store_latency_percentile(&latencies, 50),
        p90_latency_ns: live_store_latency_percentile(&latencies, 90),
        p99_latency_ns,
        max_latency_ns: latencies.last().copied().unwrap_or(0),
        p99_budget_ns,
        hot_profile_count,
        hot_route_count,
        hot_route_profile_edges,
        hot_bytes_estimate,
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        fresh_future_split_used,
        fresh_future_savings_evidence_allowed: gate_passed,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        forbidden_flags: super::super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        verdict: if gate_passed {
            "HOT_PATH_DAEMON_NUMERIC_FUTURE_PACKAGE_AUDIT_PASS"
        } else {
            "HOT_PATH_DAEMON_NUMERIC_FUTURE_PACKAGE_AUDIT_WATCH"
        },
        blocker,
        boundary: "fresh-future numeric package audit only: freezes a verifier-bound quarantine .nwpc candidate before later append rows, then shadow-scores only future matching rows through prepared-vector PhaseCenterHotRuntime; it does not promote, write serving profiles, enable local_accept, allow market claims, use lookup/target/proof authority, or reintroduce legacy nwrb/role-binding paths",
    };
    let daemon_policy = PhaseStreamHotPathDaemonAdmissionPolicyReport {
        report_kind: "phase_stream_hot_path_daemon_admission_policy_v1",
        mode: "shadow_only_daemon_admission_policy_no_runtime_mutation",
        source_benchmark_report_path: report_path.display().to_string(),
        source_promotion_review_report_path: report_path.display().to_string(),
        admission_policy_kind: "hot_path_promotion_review_to_daemon_admission_candidate_v1",
        runtime_source: "fresh_future_quarantine_nwpc_numeric_phase_center",
        hot_route_ids: if hot_route_count > 0 {
            vec![frozen.route_id]
        } else {
            Vec::new()
        },
        hot_profile_ids: if hot_profile_count > 0 {
            vec![candidate_bucket_id]
        } else {
            Vec::new()
        },
        hot_profile_count,
        hot_route_count,
        hot_route_profile_edges,
        hot_bytes_estimate,
        future_shadow_split_used: fresh_future_split_used,
        verifier_binding_bound,
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        unique_cpu_accepts_over_exact_cache: eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: eval.tokens_saved,
        cost_saved_microusd: eval.cost_saved_microusd,
        false_accepts: eval.false_accepts,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
        promotion_review_candidate_allowed: gate_passed,
        shadow_only_daemon_admission_review_allowed: gate_passed,
        admission_policy_candidate_allowed: gate_passed,
        registry_mutation_enabled: false,
        cpu_profile_registry_write_enabled: false,
        serving_profile_artifact_written: false,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        forbidden_flags: super::super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        verdict: if gate_passed {
            "HOT_PATH_DAEMON_ADMISSION_POLICY_READY"
        } else {
            "HOT_PATH_DAEMON_ADMISSION_POLICY_WATCH"
        },
        blocker: if gate_passed {
            "none"
        } else {
            "fresh_future_gate_not_passed"
        },
        boundary: "shadow-only daemon admission policy artifact derived from fresh-future .nwpc evidence: it may enter daemon shadow review, but it does not mutate registry, write serving profiles, promote product runtime, enable local_accept, allow market claims, or use legacy nwrb/role-binding paths",
    };
    super::super::write_json_file(report_path, &report)?;
    super::super::write_json_file(&daemon_admission_policy_report_path, &daemon_policy)?;
    Ok(())
}
