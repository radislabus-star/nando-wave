use std::collections::BTreeSet;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use nando_core::{
    PhaseCenterAtomEncoder, PhaseCenterHotRouteTable, PhaseCenterHotRuntime, PhaseCenterHotWorker,
    PhaseCenterLiveOperatorStore, PhaseCenterLiveOperatorStoreConfig, PhaseCenterOffloadPolicy,
    PhaseCenterOffloadRuntime, PhaseCenterOnlineMinerConfig, PhaseCenterOperatorMemoryConfig,
};

use super::reports::PhaseStreamHotPathDaemonNumericAdmissionPortfolioRuntimeReplayItemReport;
use super::source_events::{LiveStoreAdaptiveBucketPolicy, LiveStoreParsedAtomEvent};
use super::source_readers::{
    live_store_collect_append_shadow_events, live_store_observe_live_loop_budget_events,
};
use super::worker_path::{LiveStorePreparedHotPackEval, LiveStorePreparedMemoryRow};

#[allow(clippy::too_many_arguments)]
pub(super) fn live_store_replay_one_portfolio_admission(
    future_audit_report_path: &Path,
    candidate_package_path: &Path,
    fallback_route_id: u32,
    fallback_bucket_id: u32,
    expected_unique_cpu_accepts_over_exact_cache: usize,
    expected_tokens_saved: u64,
    expected_cost_saved_microusd: u64,
    expected_false_accepts: usize,
) -> Result<PhaseStreamHotPathDaemonNumericAdmissionPortfolioRuntimeReplayItemReport, String> {
    let future = super::super::read_json_value(future_audit_report_path)?;
    let cells = super::super::json_u64(&future, &["cells"]).unwrap_or_default() as usize;
    let min_bucket_events =
        super::super::json_u64(&future, &["min_bucket_events"]).unwrap_or_default() as usize;
    if cells == 0 {
        return Err("future audit cells must be > 0".to_owned());
    }
    if min_bucket_events == 0 {
        return Err("future audit min_bucket_events must be > 0".to_owned());
    }
    let watermark_trace_paths = super::super::json_at(&future, &["watermark_trace_paths"])
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(PathBuf::from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let append_trace_paths = super::super::json_at(&future, &["append_trace_paths"])
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(PathBuf::from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let freeze_after_append_events =
        super::super::json_u64(&future, &["freeze_after_append_events"]).unwrap_or_default()
            as usize;
    let candidate_route_id = super::super::json_u64(&future, &["candidate_route_id"])
        .unwrap_or(fallback_route_id as u64) as u32;
    let candidate_bucket_id = super::super::json_u64(&future, &["candidate_bucket_id"])
        .unwrap_or(fallback_bucket_id as u64) as u32;
    let threshold_micro = super::super::json_i64(&future, &["threshold_micro"]).unwrap_or_default();
    let expected_score_events =
        super::super::json_u64(&future, &["score_events"]).unwrap_or_default() as usize;
    let expected_score_candidate_events =
        super::super::json_u64(&future, &["score_candidate_events"]).unwrap_or_default() as usize;
    let expected_future_matching_bucket_events =
        super::super::json_u64(&future, &["future_matching_bucket_events"]).unwrap_or_default()
            as usize;
    if watermark_trace_paths.is_empty() {
        return Err("future audit has no watermark trace paths".to_owned());
    }
    if append_trace_paths.is_empty() {
        return Err("future audit has no append trace paths".to_owned());
    }
    if candidate_package_path.as_os_str().is_empty() {
        return Err("portfolio item has empty candidate package path".to_owned());
    }

    let mut store = PhaseCenterLiveOperatorStore::new(PhaseCenterLiveOperatorStoreConfig {
        miner: PhaseCenterOnlineMinerConfig {
            cells,
            min_bucket_events,
            threshold_floor_micro: 1,
            calibration_events: min_bucket_events,
            max_buckets: 16_384,
        },
        memory: PhaseCenterOperatorMemoryConfig {
            max_hot_profiles_per_worker:
                super::super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_PROFILES_PER_WORKER,
            max_hot_bytes_per_worker:
                super::super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
            max_warm_profiles_per_process:
                super::super::DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
            max_profiles_per_route:
                super::super::DEFAULT_PHASE_CENTER_SHADOW_MAX_PROFILES_PER_ROUTE,
            max_route_top_k: super::super::DEFAULT_PHASE_CENTER_SHADOW_MAX_ROUTE_TOP_K,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .map_err(|error| format!("failed to create portfolio replay store: {error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create portfolio replay encoder: {error:?}"))?;
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut watermark_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut watermark_total_rows = 0usize;
    let mut watermark_parsed_rows = 0usize;
    let mut watermark_skipped_no_verifier_label = 0usize;
    let mut watermark_skipped_no_safe_atoms = 0usize;
    for watermark_trace_path in &watermark_trace_paths {
        let file = File::open(watermark_trace_path).map_err(|error| {
            format!(
                "failed to open portfolio replay watermark trace '{}': {error}",
                watermark_trace_path.display()
            )
        })?;
        live_store_observe_live_loop_budget_events(
            &watermark_trace_path.display().to_string(),
            io::BufReader::new(file),
            &mut store,
            &mut encoder,
            &mut bucket_policy,
            &mut exact_cache_keys_seen,
            &mut watermark_events,
            &mut watermark_total_rows,
            &mut watermark_parsed_rows,
            &mut watermark_skipped_no_verifier_label,
            &mut watermark_skipped_no_safe_atoms,
        )?;
    }

    let mut append_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut append_total_rows = 0usize;
    let mut append_parsed_rows = 0usize;
    let mut append_skipped_no_verifier_label = 0usize;
    let mut append_skipped_no_safe_atoms = 0usize;
    for append_trace_path in &append_trace_paths {
        let file = File::open(append_trace_path).map_err(|error| {
            format!(
                "failed to open portfolio replay append trace '{}': {error}",
                append_trace_path.display()
            )
        })?;
        live_store_collect_append_shadow_events(
            &append_trace_path.display().to_string(),
            io::BufReader::new(file),
            &bucket_policy,
            &mut exact_cache_keys_seen,
            &mut append_events,
            &mut append_total_rows,
            &mut append_parsed_rows,
            &mut append_skipped_no_verifier_label,
            &mut append_skipped_no_safe_atoms,
        )?;
    }

    let package_bytes = std::fs::read(candidate_package_path).map_err(|error| {
        format!(
            "failed to read portfolio replay package '{}': {error}",
            candidate_package_path.display()
        )
    })?;
    let offload_runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &package_bytes,
        PhaseCenterOffloadPolicy::new(threshold_micro)
            .map_err(|error| format!("portfolio replay policy error: {error:?}"))?,
    )
    .map_err(|error| format!("portfolio replay package load error: {error:?}"))?;
    let hot_runtime = PhaseCenterHotRuntime::from_flat_runtime(
        offload_runtime.runtime(),
        &[candidate_bucket_id],
        &[threshold_micro],
    )
    .map_err(|error| format!("portfolio replay hot runtime build error: {error:?}"))?;
    let route_plan = hot_runtime
        .route_plan_from_profile_ids(candidate_route_id, [candidate_bucket_id])
        .map_err(|error| format!("portfolio replay route plan error: {error:?}"))?
        .ok_or_else(|| "portfolio replay hot route has no profiles".to_owned())?;
    let route_table = PhaseCenterHotRouteTable::from_plans([route_plan])
        .map_err(|error| format!("portfolio replay route table error: {error:?}"))?;
    let mut worker = PhaseCenterHotWorker::new(hot_runtime, route_table)
        .map_err(|error| format!("portfolio replay hot worker error: {error:?}"))?;
    let mut reference_encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("portfolio replay reference encoder error: {error:?}"))?;
    let future_events = append_events
        .get(freeze_after_append_events..)
        .unwrap_or_default();
    let mut eval = LiveStorePreparedHotPackEval::default();
    let mut runtime_margin_parity_checks = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut runtime_decision_parity_mismatches = 0usize;
    let mut future_matching_bucket_events = 0usize;
    for event in future_events {
        if event.bucket_id != candidate_bucket_id {
            continue;
        }
        future_matching_bucket_events += 1;
        let Some(route_index) = worker.resolve_route_index(event.route_id) else {
            runtime_decision_parity_mismatches += 1;
            continue;
        };
        let vector = reference_encoder
            .encode_atom_ids(event.atom_ids.iter().copied())
            .map_err(|error| format!("portfolio replay reference encode error: {error:?}"))?
            .to_vec();
        let reference_margin_micro = offload_runtime
            .runtime()
            .score_vector_margin_micro(0, &vector)
            .map_err(|error| format!("portfolio replay reference score error: {error:?}"))?;
        let reference_score_candidate = reference_margin_micro >= threshold_micro;
        let prepared_row = LiveStorePreparedMemoryRow::new(
            route_index,
            event.atom_ids.clone(),
            vector,
            event.hot_request_evidence(),
        );
        let decisions = worker
            .score_prepared_row_with_evidence(&prepared_row, &mut eval)
            .map_err(|error| format!("portfolio replay prepared score error: {error:?}"))?;
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

    let matches_future_audit = future_matching_bucket_events
        == expected_future_matching_bucket_events
        && eval.score_events == expected_score_events
        && eval.score_candidate_events == expected_score_candidate_events
        && eval.unique_cpu_accepts_over_exact_cache == expected_unique_cpu_accepts_over_exact_cache
        && eval.tokens_saved == expected_tokens_saved
        && eval.cost_saved_microusd == expected_cost_saved_microusd
        && eval.false_accepts == expected_false_accepts
        && runtime_margin_parity_checks == expected_score_events
        && runtime_margin_parity_mismatches == 0
        && runtime_decision_parity_mismatches == 0;
    let blocker = if matches_future_audit {
        "none".to_owned()
    } else if future_matching_bucket_events != expected_future_matching_bucket_events {
        "future_matching_bucket_events_mismatch".to_owned()
    } else if eval.score_events != expected_score_events
        || eval.score_candidate_events != expected_score_candidate_events
    {
        "score_event_count_mismatch".to_owned()
    } else if eval.unique_cpu_accepts_over_exact_cache
        != expected_unique_cpu_accepts_over_exact_cache
        || eval.tokens_saved != expected_tokens_saved
        || eval.cost_saved_microusd != expected_cost_saved_microusd
        || eval.false_accepts != expected_false_accepts
    {
        "savings_or_false_accept_count_mismatch".to_owned()
    } else if runtime_margin_parity_mismatches != 0 || runtime_decision_parity_mismatches != 0 {
        "runtime_parity_mismatch".to_owned()
    } else {
        "runtime_replay_mismatch".to_owned()
    };
    Ok(
        PhaseStreamHotPathDaemonNumericAdmissionPortfolioRuntimeReplayItemReport {
            future_audit_report_path: future_audit_report_path.display().to_string(),
            candidate_package_path: candidate_package_path.display().to_string(),
            candidate_route_id,
            candidate_bucket_id,
            threshold_micro,
            freeze_after_append_events,
            future_matching_bucket_events,
            score_events: eval.score_events,
            score_candidate_events: eval.score_candidate_events,
            unique_cpu_accepts_over_exact_cache: eval.unique_cpu_accepts_over_exact_cache,
            tokens_saved: eval.tokens_saved,
            cost_saved_microusd: eval.cost_saved_microusd,
            false_accepts: eval.false_accepts,
            runtime_margin_parity_checks,
            runtime_margin_parity_mismatches,
            runtime_decision_parity_mismatches,
            expected_score_events,
            expected_score_candidate_events,
            expected_unique_cpu_accepts_over_exact_cache,
            expected_tokens_saved,
            expected_cost_saved_microusd,
            expected_false_accepts,
            matches_future_audit,
            replay_loaded: true,
            blocker,
        },
    )
}
