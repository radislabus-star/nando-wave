use nando_core::{
    PhaseCenterAtomEncoder, PhaseCenterCell, PhaseCenterFlatRuntime, PhaseCenterHotRequest,
    PhaseCenterHotRouteTable, PhaseCenterHotRowPreparer, PhaseCenterHotRuntime,
    PhaseCenterHotScratch, PhaseCenterHotWorker, PhaseCenterLiveOperatorStore,
    PhaseCenterLiveOperatorStoreConfig, PhaseCenterOffloadPolicy, PhaseCenterOffloadRuntime,
    PhaseCenterOnlineCandidatePackage, PhaseCenterOnlineMinerConfig, PhaseCenterOperatorAdmission,
    PhaseCenterOperatorMemory, PhaseCenterOperatorMemoryConfig, PhaseCenterPreparedHotRequest,
    PhaseCenterPromotionEvidence, PhaseCenterThresholdPolicyEvidence,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::{Duration, Instant};

mod architecture;
mod bucket_decisions;
mod candidate_packages;
mod claim_gates;
mod defaults;
mod diagnostics;
mod frozen_candidates;
mod future_shadow_registry;
mod hidden_state;
mod hot_path_eval;
mod hot_path_gates;
mod numeric_false_accept_split_audit;
mod numeric_future_package;
mod paths;
mod persistence;
mod policy_json;
mod portfolio_replay;
mod profile_attribution;
mod promotion_manifests;
mod provider_evidence;
mod quarantine;
mod reports;
mod runtime_metrics;
mod runtime_registry;
mod source_events;
mod source_readers;
mod state;
mod survivor_runtime;
mod worker_path;
use architecture::*;
use bucket_decisions::*;
use candidate_packages::*;
use claim_gates::*;
use defaults::*;
use diagnostics::*;
use frozen_candidates::*;
use future_shadow_registry::*;
use hot_path_eval::*;
use hot_path_gates::*;
pub(crate) use numeric_false_accept_split_audit::*;
use numeric_future_package::*;
use paths::*;
use persistence::*;
use policy_json::*;
use portfolio_replay::*;
use profile_attribution::*;
use promotion_manifests::*;
use provider_evidence::*;
use quarantine::*;
use reports::*;
use runtime_metrics::*;
use runtime_registry::*;
use source_events::*;
use source_readers::*;
use state::*;
use survivor_runtime::*;
use worker_path::*;

fn live_store_append_compression_claim_min_rows() -> usize {
    live_store_env_usize(
        "NANDO_APPEND_COMPRESSION_CLAIM_MIN_ROWS",
        DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_COMPRESSION_CLAIM_MIN_ROWS,
    )
    .max(1)
}

fn live_store_route_wide_phase_transfer_allowed(event: &LiveStoreParsedAtomEvent) -> bool {
    event
        .selected_bucket_atoms
        .iter()
        .any(|atom| atom == "state_exit_code_band:zero")
}

pub(crate) fn run_phase_stream_live_store_adapter_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_ADAPTER_SMOKE_REPORT));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_CELLS);
    let min_bucket_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid min_bucket_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    if min_bucket_events == 0 {
        return Err("min_bucket_events must be > 0".to_owned());
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
                DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_HOT_PROFILES_PER_WORKER,
            max_hot_bytes_per_worker: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
            max_warm_profiles_per_process:
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
            max_profiles_per_route: DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_PROFILES_PER_ROUTE,
            max_route_top_k: DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_ROUTE_TOP_K,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .map_err(|error| format!("failed to create live operator store: {error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create atom encoder: {error:?}"))?;
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut total_rows = 0usize;
    let mut parsed_rows = 0usize;
    let mut skipped_no_verifier_label = 0usize;
    let mut skipped_no_safe_atoms = 0usize;
    let mut synthetic_rows = 0usize;
    let mut non_synthetic_rows = 0usize;
    let mut provider_billing_cost_rows = 0usize;
    let mut estimated_cost_rows = 0usize;
    let mut routes = BTreeMap::<u32, LiveStoreRouteAccumulator>::new();
    let mut buckets = BTreeMap::<u32, LiveStoreBucketAccumulator>::new();
    let mut bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let verifier_binding = live_store_verifier_binding();
    let mut frozen_candidates = BTreeMap::<u32, LiveStoreFrozenCandidate>::new();
    let mut parsed_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut future_encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create future-shadow atom encoder: {error:?}"))?;
    let mut future_shadow = PhaseStreamLiveStoreFutureShadowReport::default();

    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path)
            .map_err(|error| format!("failed to read trace '{}': {error}", trace_path.display()))?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows += 1;
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            synthetic_rows += usize::from(
                row.get("synthetic_source")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            );
            non_synthetic_rows += usize::from(
                !row.get("synthetic_source")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            );
            provider_billing_cost_rows += usize::from(live_store_row_has_provider_billing(&row));
            estimated_cost_rows += usize::from(live_store_row_has_estimated_cost(&row));
            let Some(verified_safe_accept) = row
                .get("verified_safe_accept")
                .and_then(serde_json::Value::as_bool)
            else {
                skipped_no_verifier_label += 1;
                continue;
            };
            let Some(adapter_event) = live_store_atom_event_from_row(
                &row,
                verified_safe_accept,
                &bucket_policy,
                &mut exact_cache_keys_seen,
            ) else {
                skipped_no_safe_atoms += 1;
                continue;
            };
            observe_live_store_future_shadow(
                &adapter_event,
                &mut frozen_candidates,
                &mut future_encoder,
                &mut future_shadow,
            )?;
            let decision = store
                .observe_atom_event(&mut encoder, adapter_event.to_live_operator_atom_event())
                .map_err(|error| {
                    format!(
                        "live store observe failed for trace '{}' line {}: {error:?}",
                        trace_path.display(),
                        line_index + 1
                    )
                })?;
            record_live_store_adapter_diagnostics(
                &mut routes,
                &mut buckets,
                &adapter_event,
                decision,
            );
            bucket_policy.observe_decision(&adapter_event, decision);
            freeze_new_live_store_candidates(
                &store,
                verifier_binding,
                &buckets,
                &mut frozen_candidates,
            )?;
            parsed_events.push(adapter_event);
            parsed_rows += 1;
        }
    }

    let summary = store.summary();
    let budget = live_store_budget_report(store.runtime_budget_snapshot());
    let direct_live_hot = live_store_direct_hot_report(&store, &parsed_events, cells)?;
    let route_reports = live_store_route_reports(routes);
    let bucket_reports = live_store_bucket_reports(buckets);
    let mut candidate_packages = Vec::new();
    store
        .candidate_packages_into_with_verifier(verifier_binding, &mut candidate_packages)
        .map_err(|error| format!("failed to build verifier-bound candidate packages: {error:?}"))?;
    let candidate_package_dir =
        report_path.with_file_name("phase-stream-live-store-adapter-smoke-v1-candidates");
    let candidate_package_reports =
        write_live_store_candidate_packages(&candidate_package_dir, candidate_packages)?;
    let total_tokens_seen = route_reports
        .iter()
        .map(|route| route.tokens_seen)
        .sum::<u64>();
    let total_cost_microusd_seen = route_reports
        .iter()
        .map(|route| route.cost_seen_microusd)
        .sum::<u64>();
    let token_cost_denominator_present = total_tokens_seen > 0 && total_cost_microusd_seen > 0;
    let provider_billing_denominator_present = provider_billing_cost_rows > 0;
    let token_cost_estimate_used =
        token_cost_denominator_present && !provider_billing_denominator_present;
    future_shadow.frozen_candidate_count = frozen_candidates.len();
    future_shadow.candidates = live_store_future_shadow_candidate_reports(
        &frozen_candidates,
        token_cost_denominator_present,
    );
    future_shadow.promotable_candidate_count = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.future_promotable)
        .count();
    future_shadow.promotable_unique_cpu_accepts_over_exact_cache = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.future_promotable)
        .map(|candidate| candidate.unique_cpu_accepts_over_exact_cache)
        .sum();
    future_shadow.promotable_tokens_saved = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.future_promotable)
        .map(|candidate| candidate.tokens_saved)
        .sum();
    future_shadow.promotable_cost_saved_microusd = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.future_promotable)
        .map(|candidate| candidate.cost_saved_microusd)
        .sum();
    future_shadow.promotion_contract_eligible_candidate_count = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.promotion_contract_eligible)
        .count();
    future_shadow.promotion_contract_unique_cpu_accepts_over_exact_cache = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.promotion_contract_eligible)
        .map(|candidate| candidate.unique_cpu_accepts_over_exact_cache)
        .sum();
    future_shadow.promotion_contract_tokens_saved = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.promotion_contract_eligible)
        .map(|candidate| candidate.tokens_saved)
        .sum();
    future_shadow.promotion_contract_cost_saved_microusd = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.promotion_contract_eligible)
        .map(|candidate| candidate.cost_saved_microusd)
        .sum();
    future_shadow.registry_admission_attempts = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.registry_admission_attempted)
        .count();
    future_shadow.registry_admitted_candidates = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.registry_admitted)
        .count();
    future_shadow.registry_rejected_candidates = future_shadow
        .registry_admission_attempts
        .saturating_sub(future_shadow.registry_admitted_candidates);
    future_shadow.registry_hot_route_count = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.registry_admitted)
        .map(|candidate| candidate.registry_hot_route_count)
        .sum();
    future_shadow.registry_hot_profile_count = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.registry_admitted)
        .map(|candidate| candidate.registry_hot_profile_count)
        .sum();
    future_shadow.registry_hot_bytes_estimate = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.registry_admitted)
        .map(|candidate| candidate.registry_hot_bytes_estimate)
        .sum();
    future_shadow.registry_budget_passed = future_shadow.registry_admitted_candidates > 0
        && future_shadow
            .candidates
            .iter()
            .filter(|candidate| candidate.registry_admitted)
            .all(|candidate| candidate.registry_budget_passed);
    future_shadow.registry_shadow_score_events = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.registry_admitted)
        .map(|candidate| candidate.registry_shadow_score_events)
        .sum();
    future_shadow.registry_shadow_score_candidate_events = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.registry_admitted)
        .map(|candidate| candidate.registry_shadow_score_candidate_events)
        .sum();
    future_shadow.registry_shadow_unique_cpu_accepts_over_exact_cache = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.registry_admitted)
        .map(|candidate| candidate.registry_shadow_unique_cpu_accepts_over_exact_cache)
        .sum();
    future_shadow.registry_shadow_tokens_saved = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.registry_admitted)
        .map(|candidate| candidate.registry_shadow_tokens_saved)
        .sum();
    future_shadow.registry_shadow_cost_saved_microusd = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.registry_admitted)
        .map(|candidate| candidate.registry_shadow_cost_saved_microusd)
        .sum();
    future_shadow.registry_shadow_false_accepts = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.registry_admitted)
        .map(|candidate| candidate.registry_shadow_false_accepts)
        .sum();
    future_shadow.registry_shadow_margin_parity_mismatches = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.registry_admitted)
        .map(|candidate| candidate.registry_shadow_margin_parity_mismatches)
        .sum();
    future_shadow.registry_shadow_decision_parity_mismatches = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.registry_admitted)
        .map(|candidate| candidate.registry_shadow_decision_parity_mismatches)
        .sum();
    let shared_registry =
        live_store_shared_registry_shadow(&frozen_candidates, token_cost_denominator_present)?;
    let serving_policy_blocker = live_store_serving_policy_blocker(&shared_registry);
    future_shadow.shared_registry_admission_attempts = shared_registry.admission_attempts;
    future_shadow.shared_registry_admitted_candidates = shared_registry.admitted_candidates;
    future_shadow.shared_registry_rejected_candidates = shared_registry.rejected_candidates;
    future_shadow.shared_registry_hot_route_count = shared_registry.hot_route_count;
    future_shadow.shared_registry_hot_profile_count = shared_registry.hot_profile_count;
    future_shadow.shared_registry_hot_route_profile_edges = shared_registry.hot_route_profile_edges;
    future_shadow.shared_registry_hot_bytes_estimate = shared_registry.hot_bytes_estimate;
    future_shadow.shared_registry_budget_passed = shared_registry.budget_passed;
    future_shadow.shared_registry_shadow_score_events = shared_registry.score_events;
    future_shadow.shared_registry_shadow_score_candidate_events =
        shared_registry.score_candidate_events;
    future_shadow.shared_registry_shadow_unique_cpu_accepts_over_exact_cache =
        shared_registry.unique_cpu_accepts_over_exact_cache;
    future_shadow.shared_registry_shadow_tokens_saved = shared_registry.tokens_saved;
    future_shadow.shared_registry_shadow_cost_saved_microusd = shared_registry.cost_saved_microusd;
    future_shadow.shared_registry_shadow_false_accepts = shared_registry.false_accepts;
    future_shadow.shared_registry_shadow_margin_parity_mismatches =
        shared_registry.margin_parity_mismatches;
    future_shadow.shared_registry_shadow_decision_parity_mismatches =
        shared_registry.decision_parity_mismatches;
    future_shadow.shared_registry_route_manifest = shared_registry.route_manifest.clone();
    future_shadow.serving_policy_kind = "manifest_score_candidate_requires_verifier_v1";
    future_shadow.serving_policy_manifest_route_count =
        future_shadow.shared_registry_route_manifest.len();
    future_shadow.serving_policy_score_events = shared_registry.score_events;
    future_shadow.serving_policy_score_candidate_events = shared_registry.score_candidate_events;
    future_shadow.serving_policy_verifier_required_events =
        shared_registry.verifier_required_events;
    future_shadow.serving_policy_local_accept_events = shared_registry.local_accept_events;
    future_shadow.serving_policy_unique_cpu_accepts_over_exact_cache =
        shared_registry.unique_cpu_accepts_over_exact_cache;
    future_shadow.serving_policy_tokens_saved = shared_registry.tokens_saved;
    future_shadow.serving_policy_cost_saved_microusd = shared_registry.cost_saved_microusd;
    future_shadow.serving_policy_false_accepts = shared_registry.false_accepts;
    future_shadow.serving_policy_margin_parity_mismatches =
        shared_registry.margin_parity_mismatches;
    future_shadow.serving_policy_decision_parity_mismatches =
        shared_registry.decision_parity_mismatches;
    future_shadow.serving_policy_exact_cache_overlap_excluded =
        shared_registry.exact_cache_overlap_excluded;
    future_shadow.serving_policy_local_accept_enabled = false;
    future_shadow.serving_policy_blocker = serving_policy_blocker;
    future_shadow.serving_policy_passed = future_shadow.serving_policy_blocker == "none";
    future_shadow.clean_promotion_manifest_kind =
        "per_candidate_false_accept_zero_registry_manifest_v1";
    future_shadow.clean_promotion_manifest_promoted_candidates =
        shared_registry.admitted_candidates;
    future_shadow.clean_promotion_manifest_quarantined_candidates =
        shared_registry.rejected_candidates;
    future_shadow.clean_promotion_manifest_hot_route_count = shared_registry.hot_route_count;
    future_shadow.clean_promotion_manifest_hot_profile_count = shared_registry.hot_profile_count;
    future_shadow.clean_promotion_manifest_hot_route_profile_edges =
        shared_registry.hot_route_profile_edges;
    future_shadow.clean_promotion_manifest_hot_bytes_estimate = shared_registry.hot_bytes_estimate;
    future_shadow.clean_promotion_manifest_unique_cpu_accepts_over_exact_cache =
        shared_registry.unique_cpu_accepts_over_exact_cache;
    future_shadow.clean_promotion_manifest_tokens_saved = shared_registry.tokens_saved;
    future_shadow.clean_promotion_manifest_cost_saved_microusd =
        shared_registry.cost_saved_microusd;
    future_shadow.clean_promotion_manifest_false_accepts = shared_registry.false_accepts;
    future_shadow.clean_promotion_manifest_runtime_parity_mismatches = shared_registry
        .margin_parity_mismatches
        .saturating_add(shared_registry.decision_parity_mismatches);
    future_shadow.clean_promotion_manifest_exact_cache_overlap_excluded =
        shared_registry.exact_cache_overlap_excluded;
    future_shadow.clean_promotion_manifest_local_accept_enabled = false;
    future_shadow.clean_promotion_manifest_market_money_claim_allowed = false;
    future_shadow.clean_promotion_manifest_routes =
        future_shadow.shared_registry_route_manifest.clone();
    future_shadow.clean_promotion_manifest_blocker =
        live_store_clean_promotion_manifest_blocker(&future_shadow);
    future_shadow.clean_promotion_manifest_allowed =
        future_shadow.clean_promotion_manifest_blocker == "none";
    refresh_live_store_call_token_promotion_manifest_summary(
        &mut future_shadow,
        &frozen_candidates,
    );
    let clean_promotion_manifest_path = report_path
        .with_file_name("phase-stream-live-store-adapter-smoke-v1-clean-promotion-manifest.json");
    let clean_promotion_package_dir =
        report_path.with_file_name("phase-stream-live-store-adapter-smoke-v1-clean-promotion");
    let call_token_promotion_manifest_path = report_path.with_file_name(
        "phase-stream-live-store-adapter-smoke-v1-call-token-promotion-manifest.json",
    );
    let call_token_promotion_package_dir =
        report_path.with_file_name("phase-stream-live-store-adapter-smoke-v1-call-token-promotion");
    write_live_store_clean_promotion_manifest(
        &clean_promotion_manifest_path,
        &clean_promotion_package_dir,
        &future_shadow,
        &frozen_candidates,
    )?;
    write_live_store_call_token_promotion_manifest(
        &call_token_promotion_manifest_path,
        &call_token_promotion_package_dir,
        &future_shadow,
        &frozen_candidates,
    )?;
    future_shadow.promotion_gate_allowed = false;
    future_shadow.blocker = if future_shadow.false_accepts > 0 {
        "future_shadow_false_accepts_present"
    } else if future_shadow.promotable_candidate_count == 0 {
        "future_shadow_no_promotable_candidate"
    } else if future_shadow.runtime_margin_parity_mismatches > 0
        || future_shadow.runtime_decision_parity_mismatches > 0
    {
        "runtime_parity_mismatch"
    } else if future_shadow.promotable_tokens_saved == 0
        || future_shadow.promotable_cost_saved_microusd == 0
    {
        "token_cost_denominator_missing"
    } else {
        "external_promotion_contract_still_required"
    };
    future_shadow.promotable_calls_saved_milli_over_parsed_rows = live_store_milli(
        future_shadow.promotable_unique_cpu_accepts_over_exact_cache as u64,
        parsed_rows as u64,
    );
    future_shadow.promotable_tokens_saved_milli_over_total =
        live_store_milli(future_shadow.promotable_tokens_saved, total_tokens_seen);
    future_shadow.promotable_cost_saved_milli_over_total = live_store_milli(
        future_shadow.promotable_cost_saved_microusd,
        total_cost_microusd_seen,
    );
    let auto_calibrated_unique_cpu_accepts_over_exact_cache = bucket_reports
        .iter()
        .filter(|bucket| bucket.product_candidate_allowed)
        .map(|bucket| bucket.unique_cpu_accepts_over_exact_cache)
        .sum::<usize>();
    let auto_calibrated_false_accepts = bucket_reports
        .iter()
        .filter(|bucket| bucket.product_candidate_allowed)
        .map(|bucket| bucket.false_accepts)
        .sum::<usize>();
    let product_metric_blocker = if summary.false_accepts > 0 {
        "exploratory_false_accepts_present_future_shadow_required"
    } else if auto_calibrated_unique_cpu_accepts_over_exact_cache == 0 {
        "no_unique_cpu_accepts_over_exact_cache"
    } else {
        "future_shadow_and_promotion_contract_still_required"
    };
    let verdict = if summary.false_accepts > 0 {
        "LIVE_STORE_ADAPTER_SMOKE_WATCH_FALSE_ACCEPTS"
    } else if summary.unique_cpu_accepts_over_exact_cache > 0 {
        "LIVE_STORE_ADAPTER_SMOKE_PASS_NO_FALSE_ACCEPTS"
    } else {
        "LIVE_STORE_ADAPTER_SMOKE_WATCH_NO_UNIQUE_ACCEPTS"
    };
    let next_action = if summary.false_accepts > 0 {
        "tighten_auto_calibration_or_split_route_buckets_before_any_promotion"
    } else if summary.unique_cpu_accepts_over_exact_cache > 0 {
        "run_future_shadow_with_verifier_bound_candidate_export"
    } else {
        "improve_safe_atoms_or_collect_more_verifier_labeled_events"
    };
    let report = PhaseStreamLiveStoreAdapterSmokeReport {
        report_kind: "phase_stream_live_store_adapter_smoke_v1",
        mode: "cold_trace_jsonl_to_numeric_live_store_boundary",
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        cells,
        min_bucket_events,
        total_rows,
        parsed_rows,
        skipped_no_verifier_label,
        skipped_no_safe_atoms,
        bucket_policy_kind: "adaptive_route_bucket_refine_on_false_accept_v1",
        adaptive_refinement_count: bucket_policy.refinement_count,
        max_bucket_refinement_depth: bucket_policy.max_depth(),
        synthetic_rows,
        non_synthetic_rows,
        route_count: store.route_count(),
        route_bucket_count: store.route_bucket_count(),
        online_bucket_count: summary.bucket_count,
        active_bucket_count: summary.active_bucket_count,
        shadow_ready_bucket_count: summary.shadow_ready_bucket_count,
        candidate_bucket_count: summary.candidate_bucket_count,
        rejected_bucket_count: summary.rejected_bucket_count,
        scored_events: summary.scored_events,
        local_operator_shadow_decisions: summary.local_operator_shadow_decisions,
        unique_cpu_accepts_over_exact_cache: summary.unique_cpu_accepts_over_exact_cache,
        raw_unique_cpu_accepts_over_exact_cache: summary.unique_cpu_accepts_over_exact_cache,
        auto_calibrated_unique_cpu_accepts_over_exact_cache,
        auto_calibrated_false_accepts,
        tokens_saved: summary.tokens_saved,
        cost_saved_microusd: summary.cost_saved_microusd,
        false_accepts: summary.false_accepts,
        token_cost_denominator_present,
        provider_billing_denominator_present,
        token_cost_estimate_used,
        provider_billing_cost_rows,
        estimated_cost_rows,
        total_tokens_seen,
        total_cost_microusd_seen,
        routes: route_reports,
        buckets: bucket_reports,
        future_shadow,
        verifier_binding_bound: verifier_binding.is_bound(),
        candidate_package_count: candidate_package_reports.len(),
        candidate_package_dir: candidate_package_dir.display().to_string(),
        candidate_packages: candidate_package_reports,
        clean_promotion_manifest_path: clean_promotion_manifest_path.display().to_string(),
        clean_promotion_package_dir: clean_promotion_package_dir.display().to_string(),
        call_token_promotion_manifest_path: call_token_promotion_manifest_path
            .display()
            .to_string(),
        call_token_promotion_package_dir: call_token_promotion_package_dir.display().to_string(),
        runtime_budget: budget,
        direct_live_hot,
        promotion_allowed: false,
        product_metric_allowed: false,
        product_metric_blocker,
        verdict,
        next_action,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "adapter smoke only: JSONL is cold input; core live store receives numeric route_id, bucket_id and atom_ids; no local_accept, no promotion, no market claim, no nwrb",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_live_store_adapter_smoke_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  total_rows: {}", report.total_rows);
    println!("  parsed_rows: {}", report.parsed_rows);
    println!("  route_count: {}", report.route_count);
    println!("  route_bucket_count: {}", report.route_bucket_count);
    println!(
        "  adaptive_refinement_count: {}",
        report.adaptive_refinement_count
    );
    println!(
        "  max_bucket_refinement_depth: {}",
        report.max_bucket_refinement_depth
    );
    println!(
        "  candidate_bucket_count: {}",
        report.candidate_bucket_count
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  auto_calibrated_unique_cpu_accepts_over_exact_cache: {}",
        report.auto_calibrated_unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!(
        "  candidate_package_count: {}",
        report.candidate_package_count
    );
    println!(
        "  future_shadow_scored_events: {}",
        report.future_shadow.scored_events
    );
    println!(
        "  future_shadow_unique_cpu_accepts_over_exact_cache: {}",
        report.future_shadow.unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  future_shadow_false_accepts: {}",
        report.future_shadow.false_accepts
    );
    println!(
        "  future_shadow_runtime_margin_parity_mismatches: {}",
        report.future_shadow.runtime_margin_parity_mismatches
    );
    println!(
        "  future_shadow_runtime_decision_parity_mismatches: {}",
        report.future_shadow.runtime_decision_parity_mismatches
    );
    println!(
        "  registry_admitted_candidates: {}",
        report.future_shadow.registry_admitted_candidates
    );
    println!(
        "  registry_shadow_false_accepts: {}",
        report.future_shadow.registry_shadow_false_accepts
    );
    println!(
        "  shared_registry_admitted_candidates: {}",
        report.future_shadow.shared_registry_admitted_candidates
    );
    println!(
        "  shared_registry_shadow_false_accepts: {}",
        report.future_shadow.shared_registry_shadow_false_accepts
    );
    println!(
        "  direct_live_hot_passed: {}",
        report.direct_live_hot.passed
    );
    println!(
        "  direct_live_hot_blocker: {}",
        report.direct_live_hot.blocker
    );
    println!(
        "  serving_policy_passed: {}",
        report.future_shadow.serving_policy_passed
    );
    println!(
        "  serving_policy_blocker: {}",
        report.future_shadow.serving_policy_blocker
    );
    println!(
        "  clean_promotion_manifest_allowed: {}",
        report.future_shadow.clean_promotion_manifest_allowed
    );
    println!(
        "  clean_promotion_manifest_blocker: {}",
        report.future_shadow.clean_promotion_manifest_blocker
    );
    println!(
        "  clean_promotion_manifest_path: {}",
        report.clean_promotion_manifest_path
    );
    println!("  promotion_allowed: {}", report.promotion_allowed);
    println!(
        "  product_metric_allowed: {}",
        report.product_metric_allowed
    );
    println!("  verdict: {}", report.verdict);
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_live_store_clean_manifest_shadow_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_CLEAN_MANIFEST));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REPORT));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };

    let bundle = load_live_store_clean_manifest_runtime(&manifest_path)?;
    let manifest = &bundle.manifest;
    let flat_runtime = &bundle.flat_runtime;
    let hot_runtime = &bundle.hot_runtime;
    let route_table = &bundle.route_table;
    let profile_ids = &bundle.profile_ids;
    let thresholds = &bundle.thresholds;
    let cells = bundle.cells;
    let loaded_record_count = bundle.loaded_record_count;
    let route_manifest_index_mismatches = bundle.route_manifest_index_mismatches;

    let mut exact_cache_keys_seen = BTreeSet::new();
    let bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut scratch = PhaseCenterHotScratch::new(cells, profile_ids.len())
        .map_err(|error| format!("failed to build clean manifest hot scratch: {error:?}"))?;
    let mut reference_encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to build clean manifest reference encoder: {error:?}"))?;
    let mut score_latencies = Vec::<u128>::new();
    let mut total_rows = 0usize;
    let mut parsed_rows = 0usize;
    let mut skipped_no_verifier_label = 0usize;
    let mut skipped_no_safe_atoms = 0usize;
    let mut route_index_missing_events = 0usize;
    let mut score_events = 0usize;
    let mut score_candidate_events = 0usize;
    let mut verifier_required_events = 0usize;
    let mut local_accept_events = 0usize;
    let mut unique_cpu_accepts_over_exact_cache = 0usize;
    let mut tokens_saved = 0u64;
    let mut cost_saved_microusd = 0u64;
    let mut false_accepts = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut runtime_decision_parity_mismatches = 0usize;

    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path)
            .map_err(|error| format!("failed to read trace '{}': {error}", trace_path.display()))?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows += 1;
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            let Some(verified_safe_accept) = row
                .get("verified_safe_accept")
                .and_then(serde_json::Value::as_bool)
            else {
                skipped_no_verifier_label += 1;
                continue;
            };
            let Some(adapter_event) = live_store_atom_event_from_row(
                &row,
                verified_safe_accept,
                &bucket_policy,
                &mut exact_cache_keys_seen,
            ) else {
                skipped_no_safe_atoms += 1;
                continue;
            };
            parsed_rows += 1;
            let Some(route_index) = route_table.resolve_route_index(adapter_event.route_id) else {
                route_index_missing_events += 1;
                continue;
            };
            let start = Instant::now();
            let decisions = hot_runtime
                .score_hot_request_candidates(
                    &route_table,
                    PhaseCenterHotRequest::new(route_index, &adapter_event.atom_ids),
                    &mut scratch,
                )
                .map_err(|error| format!("failed clean manifest hot score: {error:?}"))?;
            score_latencies.push(start.elapsed().as_nanos());
            score_events += 1;
            let vector = reference_encoder
                .encode_atom_ids(adapter_event.atom_ids.iter().copied())
                .map_err(|error| format!("failed clean manifest reference encode: {error:?}"))?
                .to_vec();
            for decision in decisions {
                let Some(profile_index) = profile_ids
                    .iter()
                    .position(|profile_id| *profile_id == decision.profile_id)
                else {
                    runtime_decision_parity_mismatches += 1;
                    continue;
                };
                let reference_margin = flat_runtime
                    .score_vector_margin_micro(profile_index, &vector)
                    .map_err(|error| format!("failed clean manifest reference score: {error:?}"))?;
                let reference_score_candidate = reference_margin >= thresholds[profile_index];
                if decision.margin_micro != reference_margin {
                    runtime_margin_parity_mismatches += 1;
                }
                if decision.score_candidate != reference_score_candidate {
                    runtime_decision_parity_mismatches += 1;
                }
                if !decision.score_candidate {
                    continue;
                }
                score_candidate_events += 1;
                verifier_required_events += usize::from(decision.verifier_required);
                local_accept_events += usize::from(decision.local_accept);
                if adapter_event.verified_safe_accept {
                    if !adapter_event.exact_cache_hit {
                        unique_cpu_accepts_over_exact_cache += 1;
                        tokens_saved = tokens_saved.saturating_add(adapter_event.tokens);
                        cost_saved_microusd =
                            cost_saved_microusd.saturating_add(adapter_event.cost_microusd);
                    }
                } else {
                    false_accepts += 1;
                }
            }
        }
    }
    score_latencies.sort_unstable();

    let hot_bytes_estimate = hot_runtime
        .bytes_estimate()
        .saturating_add(route_table.bytes_estimate());
    let exact_cache_overlap_excluded = true;
    let blocker = live_store_clean_manifest_shadow_blocker(
        &manifest,
        loaded_record_count,
        score_events,
        score_candidate_events,
        verifier_required_events,
        local_accept_events,
        false_accepts,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
        route_manifest_index_mismatches,
    );
    let report = PhaseStreamLiveStoreCleanManifestShadowReport {
        report_kind: "phase_stream_live_store_clean_manifest_shadow_v1",
        manifest_path: manifest_path.display().to_string(),
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        manifest_allowed: manifest.allowed,
        manifest_blocker: manifest.blocker.clone(),
        manifest_false_accepts: manifest.false_accepts,
        manifest_runtime_parity_mismatches: manifest.runtime_parity_mismatches,
        manifest_exact_cache_overlap_excluded: manifest.exact_cache_overlap_excluded,
        promoted_package_count: manifest.promoted_packages.len(),
        route_count: manifest.routes.len(),
        loaded_package_count: manifest.promoted_packages.len(),
        loaded_record_count,
        cells,
        hot_profile_count: hot_runtime.profile_count(),
        hot_route_count: route_table.route_count(),
        hot_route_profile_edges: route_table.profile_edge_count(),
        hot_bytes_estimate,
        route_manifest_index_mismatches,
        total_rows,
        parsed_rows,
        skipped_no_verifier_label,
        skipped_no_safe_atoms,
        route_index_missing_events,
        score_events,
        score_candidate_events,
        verifier_required_events,
        local_accept_events,
        unique_cpu_accepts_over_exact_cache,
        tokens_saved,
        cost_saved_microusd,
        false_accepts,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
        exact_cache_overlap_excluded,
        p50_score_latency_ns: live_store_latency_percentile(&score_latencies, 50),
        p90_score_latency_ns: live_store_latency_percentile(&score_latencies, 90),
        p99_score_latency_ns: live_store_latency_percentile(&score_latencies, 99),
        max_score_latency_ns: score_latencies.last().copied().unwrap_or(0),
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        product_metric_allowed: blocker == "none",
        verdict: if blocker == "none" {
            "LIVE_STORE_CLEAN_MANIFEST_SHADOW_PASS"
        } else {
            "LIVE_STORE_CLEAN_MANIFEST_SHADOW_WATCH"
        },
        blocker,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "clean manifest shadow: preloaded .nwpc package plus numeric route table; trace parsing is cold input; score path keeps local_accept disabled and reports parity/false_accepts over exact cache",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_live_store_clean_manifest_shadow_v1:");
    println!("  manifest_path: {}", report.manifest_path);
    println!("  report_path: {}", report_path.display());
    println!(
        "  promoted_package_count: {}",
        report.promoted_package_count
    );
    println!("  hot_route_count: {}", report.hot_route_count);
    println!("  hot_profile_count: {}", report.hot_profile_count);
    println!("  score_events: {}", report.score_events);
    println!(
        "  score_candidate_events: {}",
        report.score_candidate_events
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!(
        "  runtime_margin_parity_mismatches: {}",
        report.runtime_margin_parity_mismatches
    );
    println!(
        "  runtime_decision_parity_mismatches: {}",
        report.runtime_decision_parity_mismatches
    );
    println!("  p99_score_latency_ns: {}", report.p99_score_latency_ns);
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_live_store_prepared_hot_pack_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_CLEAN_MANIFEST));
    let pack_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_PREPARED_HOT_PACK));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_PREPARED_HOT_PACK_REPORT));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };

    let bundle = load_live_store_clean_manifest_runtime(&manifest_path)?;
    let mut exact_cache_keys_seen = BTreeSet::new();
    let bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut total_rows = 0usize;
    let mut parsed_rows = 0usize;
    let mut route_index_missing_events = 0usize;
    let mut rows = Vec::<PhaseStreamLiveStorePreparedHotPackRow>::new();

    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path)
            .map_err(|error| format!("failed to read trace '{}': {error}", trace_path.display()))?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows += 1;
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            let Some(verified_safe_accept) = row
                .get("verified_safe_accept")
                .and_then(serde_json::Value::as_bool)
            else {
                continue;
            };
            let Some(adapter_event) = live_store_atom_event_from_row(
                &row,
                verified_safe_accept,
                &bucket_policy,
                &mut exact_cache_keys_seen,
            ) else {
                continue;
            };
            parsed_rows += 1;
            let Some(route_index) = bundle
                .route_table
                .resolve_route_index(adapter_event.route_id)
            else {
                route_index_missing_events += 1;
                continue;
            };
            rows.push(PhaseStreamLiveStorePreparedHotPackRow {
                route_id: adapter_event.route_id,
                route_index,
                atom_ids: adapter_event.atom_ids,
                verified_safe_accept: adapter_event.verified_safe_accept,
                exact_cache_hit: adapter_event.exact_cache_hit,
                tokens: adapter_event.tokens,
                cost_microusd: adapter_event.cost_microusd,
            });
        }
    }

    let pack = PhaseStreamLiveStorePreparedHotPack {
        pack_kind: "phase_stream_live_store_prepared_hot_pack_v1".to_owned(),
        manifest_path: manifest_path.display().to_string(),
        cells: bundle.cells,
        route_count: bundle.route_table.route_count(),
        profile_count: bundle.hot_runtime.profile_count(),
        rows,
        boundary: "cold artifact: numeric route_index plus atom_ids only; no route strings or JSON are required inside the hot scoring loop".to_owned(),
    };
    super::write_json_file(&pack_path, &pack)?;

    let loaded_pack_text = std::fs::read_to_string(&pack_path).map_err(|error| {
        format!(
            "failed to read prepared hot pack '{}': {error}",
            pack_path.display()
        )
    })?;
    let loaded_pack = serde_json::from_str::<PhaseStreamLiveStorePreparedHotPack>(
        &loaded_pack_text,
    )
    .map_err(|error| {
        format!(
            "failed to parse prepared hot pack '{}': {error}",
            pack_path.display()
        )
    })?;
    if loaded_pack.cells != bundle.cells {
        return Err("prepared hot pack cell width mismatch".to_owned());
    }

    let mut atom_eval = LiveStorePreparedHotPackEval::default();
    let mut prepared_eval = LiveStorePreparedHotPackEval::default();
    let mut worker =
        PhaseCenterHotWorker::new(bundle.hot_runtime.clone(), bundle.route_table.clone())
            .map_err(|error| format!("failed to build prepared hot worker: {error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(bundle.cells)
        .map_err(|error| format!("failed to build prepared pack encoder: {error:?}"))?;
    let mut prepared_vectors = Vec::<Vec<nando_core::PhaseCenterCell>>::new();
    for row in &loaded_pack.rows {
        let vector = encoder
            .encode_atom_ids(row.atom_ids.iter().copied())
            .map_err(|error| format!("failed to encode prepared pack row: {error:?}"))?
            .to_vec();
        prepared_vectors.push(vector);
    }

    let mut atom_prepared_margin_parity_mismatches = 0usize;
    let mut atom_prepared_decision_parity_mismatches = 0usize;
    for (row, vector) in loaded_pack.rows.iter().zip(prepared_vectors.iter()) {
        let atom_decisions = worker
            .score_atom_ids(PhaseCenterHotRequest::new(row.route_index, &row.atom_ids))
            .map_err(|error| format!("failed atom hot pack score: {error:?}"))?
            .to_vec();
        live_store_update_prepared_hot_pack_eval(row, &atom_decisions, &mut atom_eval);
        let prepared_decisions = worker
            .score_prepared(PhaseCenterPreparedHotRequest::new(row.route_index, vector))
            .map_err(|error| format!("failed prepared hot pack score: {error:?}"))?
            .to_vec();
        live_store_update_prepared_hot_pack_eval(row, &prepared_decisions, &mut prepared_eval);
        if atom_decisions.len() != prepared_decisions.len() {
            atom_prepared_decision_parity_mismatches += 1;
            continue;
        }
        for (atom, prepared) in atom_decisions.iter().zip(prepared_decisions.iter()) {
            if atom.profile_id != prepared.profile_id
                || atom.score_candidate != prepared.score_candidate
                || atom.verifier_required != prepared.verifier_required
                || atom.local_accept != prepared.local_accept
            {
                atom_prepared_decision_parity_mismatches += 1;
            }
            if atom.margin_micro != prepared.margin_micro {
                atom_prepared_margin_parity_mismatches += 1;
            }
        }
    }

    let latency_repeats = 1000usize;
    let mut atom_latencies = Vec::<u128>::with_capacity(loaded_pack.rows.len() * latency_repeats);
    let mut prepared_latencies =
        Vec::<u128>::with_capacity(loaded_pack.rows.len() * latency_repeats);
    for _ in 0..latency_repeats {
        for (row, vector) in loaded_pack.rows.iter().zip(prepared_vectors.iter()) {
            let start = Instant::now();
            let _ = worker
                .score_atom_ids(PhaseCenterHotRequest::new(row.route_index, &row.atom_ids))
                .map_err(|error| format!("failed atom latency score: {error:?}"))?;
            atom_latencies.push(start.elapsed().as_nanos());

            let start = Instant::now();
            let _ = worker
                .score_prepared(PhaseCenterPreparedHotRequest::new(row.route_index, vector))
                .map_err(|error| format!("failed prepared latency score: {error:?}"))?;
            prepared_latencies.push(start.elapsed().as_nanos());
        }
    }
    atom_latencies.sort_unstable();
    prepared_latencies.sort_unstable();

    let hot_bytes_estimate = worker.bytes_estimate();
    let prepared_p99 = live_store_latency_percentile(&prepared_latencies, 99);
    let blocker = live_store_prepared_hot_pack_blocker(
        loaded_pack.rows.len(),
        &atom_eval,
        &prepared_eval,
        atom_prepared_margin_parity_mismatches,
        atom_prepared_decision_parity_mismatches,
        prepared_p99,
    );
    let report = PhaseStreamLiveStorePreparedHotPackReport {
        report_kind: "phase_stream_live_store_prepared_hot_pack_v1",
        manifest_path: manifest_path.display().to_string(),
        pack_path: pack_path.display().to_string(),
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        total_rows,
        parsed_rows,
        route_index_missing_events,
        pack_rows: loaded_pack.rows.len(),
        cells: bundle.cells,
        hot_profile_count: bundle.hot_runtime.profile_count(),
        hot_route_count: bundle.route_table.route_count(),
        hot_route_profile_edges: bundle.route_table.profile_edge_count(),
        hot_bytes_estimate,
        pack_contains_strings_in_rows: false,
        hot_loop_json_used: false,
        hot_loop_string_route_used: false,
        hot_loop_btreemap_used: false,
        hot_loop_file_io_used: false,
        atom_score_events: atom_eval.score_events,
        atom_score_candidate_events: atom_eval.score_candidate_events,
        atom_verifier_required_events: atom_eval.verifier_required_events,
        atom_local_accept_events: atom_eval.local_accept_events,
        atom_unique_cpu_accepts_over_exact_cache: atom_eval.unique_cpu_accepts_over_exact_cache,
        atom_tokens_saved: atom_eval.tokens_saved,
        atom_cost_saved_microusd: atom_eval.cost_saved_microusd,
        atom_false_accepts: atom_eval.false_accepts,
        prepared_score_events: prepared_eval.score_events,
        prepared_score_candidate_events: prepared_eval.score_candidate_events,
        prepared_verifier_required_events: prepared_eval.verifier_required_events,
        prepared_local_accept_events: prepared_eval.local_accept_events,
        prepared_unique_cpu_accepts_over_exact_cache: prepared_eval
            .unique_cpu_accepts_over_exact_cache,
        prepared_tokens_saved: prepared_eval.tokens_saved,
        prepared_cost_saved_microusd: prepared_eval.cost_saved_microusd,
        prepared_false_accepts: prepared_eval.false_accepts,
        atom_prepared_margin_parity_mismatches,
        atom_prepared_decision_parity_mismatches,
        prepared_p50_score_latency_ns: live_store_latency_percentile(&prepared_latencies, 50),
        prepared_p90_score_latency_ns: live_store_latency_percentile(&prepared_latencies, 90),
        prepared_p99_score_latency_ns: prepared_p99,
        prepared_max_score_latency_ns: prepared_latencies.last().copied().unwrap_or(0),
        atom_p50_score_latency_ns: live_store_latency_percentile(&atom_latencies, 50),
        atom_p90_score_latency_ns: live_store_latency_percentile(&atom_latencies, 90),
        atom_p99_score_latency_ns: live_store_latency_percentile(&atom_latencies, 99),
        atom_max_score_latency_ns: atom_latencies.last().copied().unwrap_or(0),
        latency_repeats,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        verdict: if blocker == "none" {
            "LIVE_STORE_PREPARED_HOT_PACK_PASS"
        } else {
            "LIVE_STORE_PREPARED_HOT_PACK_WATCH"
        },
        blocker,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "prepared hot pack: JSONL is cold source only; timed hot loops consume numeric route_index and atom_ids/prepared phase vectors with caller-owned scratch; no local_accept or market claim",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_live_store_prepared_hot_pack_v1:");
    println!("  pack_path: {}", pack_path.display());
    println!("  report_path: {}", report_path.display());
    println!("  pack_rows: {}", report.pack_rows);
    println!(
        "  prepared_score_candidate_events: {}",
        report.prepared_score_candidate_events
    );
    println!(
        "  prepared_unique_cpu_accepts_over_exact_cache: {}",
        report.prepared_unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  prepared_false_accepts: {}",
        report.prepared_false_accepts
    );
    println!(
        "  atom_prepared_margin_parity_mismatches: {}",
        report.atom_prepared_margin_parity_mismatches
    );
    println!(
        "  atom_prepared_decision_parity_mismatches: {}",
        report.atom_prepared_decision_parity_mismatches
    );
    println!(
        "  prepared_p99_score_latency_ns: {}",
        report.prepared_p99_score_latency_ns
    );
    println!(
        "  atom_p99_score_latency_ns: {}",
        report.atom_p99_score_latency_ns
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_live_worker_memory_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_CLEAN_MANIFEST));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_MEMORY_HOT_WORKER_REPORT));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };

    let bundle = load_live_store_clean_manifest_runtime(&manifest_path)?;
    let mut worker =
        PhaseCenterHotWorker::new(bundle.hot_runtime.clone(), bundle.route_table.clone())
            .map_err(|error| format!("failed to build live memory hot worker: {error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(bundle.cells)
        .map_err(|error| format!("failed to build live memory encoder: {error:?}"))?;
    let mut exact_cache_keys_seen = BTreeSet::new();
    let bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut total_rows = 0usize;
    let mut parsed_rows = 0usize;
    let mut route_index_missing_events = 0usize;
    let mut rows = Vec::<LiveStorePreparedMemoryRow>::new();

    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path)
            .map_err(|error| format!("failed to read trace '{}': {error}", trace_path.display()))?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows += 1;
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            let Some(verified_safe_accept) = row
                .get("verified_safe_accept")
                .and_then(serde_json::Value::as_bool)
            else {
                continue;
            };
            let Some(adapter_event) = live_store_atom_event_from_row(
                &row,
                verified_safe_accept,
                &bucket_policy,
                &mut exact_cache_keys_seen,
            ) else {
                continue;
            };
            parsed_rows += 1;
            let Some(route_index) = worker.resolve_route_index(adapter_event.route_id) else {
                route_index_missing_events += 1;
                continue;
            };
            let phase_vector = encoder
                .encode_atom_ids(adapter_event.atom_ids.iter().copied())
                .map_err(|error| format!("failed to encode live memory row: {error:?}"))?
                .to_vec();
            rows.push(LiveStorePreparedMemoryRow::new(
                route_index,
                adapter_event.atom_ids.clone(),
                phase_vector,
                adapter_event.hot_request_evidence(),
            ));
        }
    }

    let mut eval = LiveStorePreparedHotPackEval::default();
    for row in &rows {
        let _ = worker
            .score_prepared_row_with_evidence(row, &mut eval)
            .map_err(|error| format!("failed live memory worker score: {error:?}"))?;
    }

    let latency_repeats = 1000usize;
    let mut latencies = Vec::<u128>::with_capacity(rows.len() * latency_repeats);
    for _ in 0..latency_repeats {
        for row in &rows {
            let start = Instant::now();
            let _ = worker
                .score_prepared(PhaseCenterPreparedHotRequest::new(
                    row.route_index,
                    &row.phase_vector,
                ))
                .map_err(|error| format!("failed live memory worker latency score: {error:?}"))?;
            latencies.push(start.elapsed().as_nanos());
        }
    }
    latencies.sort_unstable();

    let p99 = live_store_latency_percentile(&latencies, 99);
    let blocker = live_store_memory_hot_worker_blocker(rows.len(), &eval, p99);
    let report = PhaseStreamLiveStoreMemoryHotWorkerReport {
        report_kind: "phase_stream_live_worker_memory_smoke_v1",
        manifest_path: manifest_path.display().to_string(),
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        total_rows,
        parsed_rows,
        route_index_missing_events,
        prepared_memory_rows: rows.len(),
        cells: bundle.cells,
        hot_profile_count: worker.profile_count(),
        hot_route_count: worker.route_count(),
        hot_route_profile_edges: worker.route_profile_edge_count(),
        hot_bytes_estimate: worker.bytes_estimate(),
        worker_kind: "PhaseCenterHotWorker",
        source_adapter_json_used: true,
        hot_loop_json_used: false,
        hot_loop_string_route_used: false,
        hot_loop_btreemap_used: false,
        hot_loop_file_io_used: false,
        hot_loop_package_compile_used: false,
        score_events: eval.score_events,
        score_candidate_events: eval.score_candidate_events,
        verifier_required_events: eval.verifier_required_events,
        local_accept_events: eval.local_accept_events,
        unique_cpu_accepts_over_exact_cache: eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: eval.tokens_saved,
        cost_saved_microusd: eval.cost_saved_microusd,
        false_accepts: eval.false_accepts,
        p50_score_latency_ns: live_store_latency_percentile(&latencies, 50),
        p90_score_latency_ns: live_store_latency_percentile(&latencies, 90),
        p99_score_latency_ns: p99,
        max_score_latency_ns: latencies.last().copied().unwrap_or(0),
        latency_repeats,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        verdict: if blocker == "none" {
            "LIVE_WORKER_MEMORY_SMOKE_PASS"
        } else {
            "LIVE_WORKER_MEMORY_SMOKE_WATCH"
        },
        blocker,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "live worker memory smoke: source adapter may parse JSONL once; timed worker loop consumes route_index plus prepared phase vectors only; no local_accept or market claim",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_live_worker_memory_smoke_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  prepared_memory_rows: {}", report.prepared_memory_rows);
    println!(
        "  score_candidate_events: {}",
        report.score_candidate_events
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!("  p99_score_latency_ns: {}", report.p99_score_latency_ns);
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_live_source_adapter_worker_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_CLEAN_MANIFEST));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_SOURCE_ADAPTER_WORKER_REPORT));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };

    let bundle = load_live_store_clean_manifest_runtime(&manifest_path)?;
    let mut worker =
        PhaseCenterHotWorker::new(bundle.hot_runtime.clone(), bundle.route_table.clone())
            .map_err(|error| format!("failed to build source adapter worker: {error:?}"))?;
    let warmup_vector = vec![PhaseCenterCell::default(); bundle.cells];
    let mut worker_warmup_route_scores = 0usize;
    for route_index in 0..worker.route_count() {
        let _ = worker
            .score_prepared(PhaseCenterPreparedHotRequest::new(
                route_index,
                &warmup_vector,
            ))
            .map_err(|error| format!("failed source adapter worker warmup: {error:?}"))?;
        worker_warmup_route_scores += 1;
    }
    let bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut eval = LiveStorePreparedHotPackEval::default();
    let mut latencies = Vec::<u128>::new();
    let mut total_rows = 0usize;
    let mut parsed_rows = 0usize;
    let mut route_index_missing_events = 0usize;

    for trace_path in &trace_paths {
        if trace_path == Path::new("-") {
            let stdin = io::stdin();
            let reader = stdin.lock();
            live_store_score_source_adapter_reader(
                "<stdin>",
                reader,
                &mut worker,
                &bucket_policy,
                &mut exact_cache_keys_seen,
                &mut eval,
                &mut latencies,
                &mut total_rows,
                &mut parsed_rows,
                &mut route_index_missing_events,
            )?;
        } else {
            let file = File::open(trace_path).map_err(|error| {
                format!("failed to open trace '{}': {error}", trace_path.display())
            })?;
            let reader = io::BufReader::new(file);
            live_store_score_source_adapter_reader(
                &trace_path.display().to_string(),
                reader,
                &mut worker,
                &bucket_policy,
                &mut exact_cache_keys_seen,
                &mut eval,
                &mut latencies,
                &mut total_rows,
                &mut parsed_rows,
                &mut route_index_missing_events,
            )?;
        }
    }
    latencies.sort_unstable();

    let p99 = live_store_latency_percentile(&latencies, 99);
    let blocker = live_store_memory_hot_worker_blocker(eval.score_events, &eval, p99);
    let report = PhaseStreamLiveStoreSourceAdapterWorkerReport {
        report_kind: "phase_stream_live_source_adapter_worker_v1",
        manifest_path: manifest_path.display().to_string(),
        input_sources: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        total_rows,
        parsed_rows,
        route_index_missing_events,
        cells: bundle.cells,
        hot_profile_count: worker.profile_count(),
        hot_route_count: worker.route_count(),
        hot_route_profile_edges: worker.route_profile_edge_count(),
        hot_bytes_estimate: worker.bytes_estimate(),
        worker_warmup_route_scores,
        worker_kind: "PhaseCenterHotWorker",
        source_adapter_streaming_lines_used: true,
        source_adapter_json_used: true,
        stdin_supported: true,
        hot_score_inside_event_loop: true,
        hot_loop_json_used: false,
        hot_loop_string_route_used: false,
        hot_loop_btreemap_used: false,
        hot_loop_file_io_used: false,
        hot_loop_package_compile_used: false,
        score_events: eval.score_events,
        score_candidate_events: eval.score_candidate_events,
        verifier_required_events: eval.verifier_required_events,
        local_accept_events: eval.local_accept_events,
        unique_cpu_accepts_over_exact_cache: eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: eval.tokens_saved,
        cost_saved_microusd: eval.cost_saved_microusd,
        false_accepts: eval.false_accepts,
        p50_score_latency_ns: live_store_latency_percentile(&latencies, 50),
        p90_score_latency_ns: live_store_latency_percentile(&latencies, 90),
        p99_score_latency_ns: p99,
        max_score_latency_ns: latencies.last().copied().unwrap_or(0),
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        verdict: if blocker == "none" {
            "LIVE_SOURCE_ADAPTER_WORKER_PASS"
        } else {
            "LIVE_SOURCE_ADAPTER_WORKER_WATCH"
        },
        blocker,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "live source adapter worker: source lines are parsed at the adapter edge; worker scratch is warmed at load; each verified event is immediately encoded and scored by PhaseCenterHotWorker; no local_accept or market claim",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_live_source_adapter_worker_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  score_events: {}", report.score_events);
    println!(
        "  score_candidate_events: {}",
        report.score_candidate_events
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!("  p99_score_latency_ns: {}", report.p99_score_latency_ns);
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_live_worker_queue_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_CLEAN_MANIFEST));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_WORKER_QUEUE_REPORT));
    let queue_batch_capacity = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid queue batch capacity '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_LIVE_STORE_WORKER_QUEUE_BATCH_CAPACITY)
        .max(1);
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };

    let bundle = load_live_store_clean_manifest_runtime(&manifest_path)?;
    let mut worker =
        PhaseCenterHotWorker::new(bundle.hot_runtime.clone(), bundle.route_table.clone())
            .map_err(|error| format!("failed to build queue hot worker: {error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(bundle.cells)
        .map_err(|error| format!("failed to build queue source encoder: {error:?}"))?;
    let warmup_vector = vec![PhaseCenterCell::default(); bundle.cells];
    for route_index in 0..worker.route_count() {
        let _ = worker
            .score_prepared(PhaseCenterPreparedHotRequest::new(
                route_index,
                &warmup_vector,
            ))
            .map_err(|error| format!("failed queue worker warmup: {error:?}"))?;
    }

    let bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut queue = Vec::<LiveStorePreparedMemoryRow>::with_capacity(queue_batch_capacity);
    let mut eval = LiveStorePreparedHotPackEval::default();
    let mut latencies = Vec::<u128>::new();
    let mut total_rows = 0usize;
    let mut parsed_rows = 0usize;
    let mut route_index_missing_events = 0usize;
    let mut queue_flushes = 0usize;
    let mut max_observed_queue_depth = 0usize;

    for trace_path in &trace_paths {
        if trace_path == Path::new("-") {
            let stdin = io::stdin();
            let reader = stdin.lock();
            live_store_queue_source_reader(
                "<stdin>",
                reader,
                &mut worker,
                &mut encoder,
                &bucket_policy,
                &mut exact_cache_keys_seen,
                &mut queue,
                queue_batch_capacity,
                &mut eval,
                &mut latencies,
                &mut total_rows,
                &mut parsed_rows,
                &mut route_index_missing_events,
                &mut queue_flushes,
                &mut max_observed_queue_depth,
            )?;
        } else {
            let file = File::open(trace_path).map_err(|error| {
                format!("failed to open trace '{}': {error}", trace_path.display())
            })?;
            let reader = io::BufReader::new(file);
            live_store_queue_source_reader(
                &trace_path.display().to_string(),
                reader,
                &mut worker,
                &mut encoder,
                &bucket_policy,
                &mut exact_cache_keys_seen,
                &mut queue,
                queue_batch_capacity,
                &mut eval,
                &mut latencies,
                &mut total_rows,
                &mut parsed_rows,
                &mut route_index_missing_events,
                &mut queue_flushes,
                &mut max_observed_queue_depth,
            )?;
        }
    }
    live_store_flush_worker_queue(
        &mut worker,
        &mut queue,
        &mut eval,
        &mut latencies,
        &mut queue_flushes,
    )?;
    latencies.sort_unstable();

    let p99 = live_store_latency_percentile(&latencies, 99);
    let blocker = live_store_memory_hot_worker_blocker(eval.score_events, &eval, p99);
    let report = PhaseStreamLiveStoreWorkerQueueReport {
        report_kind: "phase_stream_live_worker_queue_smoke_v1",
        manifest_path: manifest_path.display().to_string(),
        input_sources: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        total_rows,
        parsed_rows,
        route_index_missing_events,
        cells: bundle.cells,
        queue_batch_capacity,
        queue_flushes,
        max_observed_queue_depth,
        hot_profile_count: worker.profile_count(),
        hot_route_count: worker.route_count(),
        hot_route_profile_edges: worker.route_profile_edge_count(),
        hot_bytes_estimate: worker.bytes_estimate(),
        worker_kind: "PhaseCenterHotWorker",
        source_adapter_json_used: true,
        bounded_memory_queue_used: true,
        hot_worker_drain_loop_isolated: true,
        hot_loop_json_used: false,
        hot_loop_string_route_used: false,
        hot_loop_btreemap_used: false,
        hot_loop_file_io_used: false,
        hot_loop_package_compile_used: false,
        score_events: eval.score_events,
        score_candidate_events: eval.score_candidate_events,
        verifier_required_events: eval.verifier_required_events,
        local_accept_events: eval.local_accept_events,
        unique_cpu_accepts_over_exact_cache: eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: eval.tokens_saved,
        cost_saved_microusd: eval.cost_saved_microusd,
        false_accepts: eval.false_accepts,
        p50_score_latency_ns: live_store_latency_percentile(&latencies, 50),
        p90_score_latency_ns: live_store_latency_percentile(&latencies, 90),
        p99_score_latency_ns: p99,
        max_score_latency_ns: latencies.last().copied().unwrap_or(0),
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        verdict: if blocker == "none" {
            "LIVE_WORKER_QUEUE_SMOKE_PASS"
        } else {
            "LIVE_WORKER_QUEUE_SMOKE_WATCH"
        },
        blocker,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "live worker queue smoke: source adapter parses events and pushes prepared vectors into a bounded memory queue; isolated worker drain loop scores only route_index plus phase vectors; no local_accept or market claim",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_live_worker_queue_smoke_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  queue_batch_capacity: {}", report.queue_batch_capacity);
    println!("  queue_flushes: {}", report.queue_flushes);
    println!("  score_events: {}", report.score_events);
    println!(
        "  score_candidate_events: {}",
        report.score_candidate_events
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!("  p99_score_latency_ns: {}", report.p99_score_latency_ns);
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_live_worker_thread_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_CLEAN_MANIFEST));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_WORKER_THREAD_REPORT));
    let channel_capacity = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid worker channel capacity '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_LIVE_STORE_WORKER_QUEUE_BATCH_CAPACITY)
        .max(1);
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };

    let bundle = load_live_store_clean_manifest_runtime(&manifest_path)?;
    let worker = PhaseCenterHotWorker::new(bundle.hot_runtime.clone(), bundle.route_table.clone())
        .map_err(|error| format!("failed to build thread hot worker: {error:?}"))?;
    let hot_profile_count = worker.profile_count();
    let hot_route_count = worker.route_count();
    let hot_route_profile_edges = worker.route_profile_edge_count();
    let hot_bytes_estimate = worker.bytes_estimate();
    let (sender, receiver) = sync_channel::<LiveStoreWorkerThreadMessage>(channel_capacity);
    let worker_handle = thread::spawn(move || -> Result<LiveStoreWorkerThreadMetrics, String> {
        let mut worker = worker;
        let warmup_vector = vec![PhaseCenterCell::default(); worker.cells()];
        let mut metrics = LiveStoreWorkerThreadMetrics::default();
        for route_index in 0..worker.route_count() {
            let _ = worker
                .score_prepared(PhaseCenterPreparedHotRequest::new(
                    route_index,
                    &warmup_vector,
                ))
                .map_err(|error| format!("failed thread worker warmup: {error:?}"))?;
            metrics.worker_warmup_route_scores += 1;
        }
        while let Ok(message) = receiver.recv() {
            metrics
                .queue_wait_latencies
                .push(message.enqueued_at.elapsed().as_nanos());
            let start = Instant::now();
            let decisions = worker
                .score_prepared(PhaseCenterPreparedHotRequest::new(
                    message.row.route_index,
                    &message.row.phase_vector,
                ))
                .map_err(|error| format!("failed thread worker score: {error:?}"))?;
            metrics
                .worker_score_latencies
                .push(start.elapsed().as_nanos());
            live_store_update_memory_hot_worker_eval(&message.row, decisions, &mut metrics.eval);
        }
        metrics.queue_wait_latencies.sort_unstable();
        metrics.worker_score_latencies.sort_unstable();
        Ok(metrics)
    });

    let mut encoder = PhaseCenterAtomEncoder::new(bundle.cells)
        .map_err(|error| format!("failed to build thread source encoder: {error:?}"))?;
    let bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut source_prepare_latencies = Vec::<u128>::new();
    let mut source_send_latencies = Vec::<u128>::new();
    let mut total_rows = 0usize;
    let mut parsed_rows = 0usize;
    let mut route_index_missing_events = 0usize;
    let mut sent_events = 0usize;

    for trace_path in &trace_paths {
        if trace_path == Path::new("-") {
            let stdin = io::stdin();
            let reader = stdin.lock();
            live_store_thread_source_reader(
                "<stdin>",
                reader,
                &bundle.route_table,
                &mut encoder,
                &bucket_policy,
                &mut exact_cache_keys_seen,
                &sender,
                &mut source_prepare_latencies,
                &mut source_send_latencies,
                &mut total_rows,
                &mut parsed_rows,
                &mut route_index_missing_events,
                &mut sent_events,
            )?;
        } else {
            let file = File::open(trace_path).map_err(|error| {
                format!("failed to open trace '{}': {error}", trace_path.display())
            })?;
            let reader = io::BufReader::new(file);
            live_store_thread_source_reader(
                &trace_path.display().to_string(),
                reader,
                &bundle.route_table,
                &mut encoder,
                &bucket_policy,
                &mut exact_cache_keys_seen,
                &sender,
                &mut source_prepare_latencies,
                &mut source_send_latencies,
                &mut total_rows,
                &mut parsed_rows,
                &mut route_index_missing_events,
                &mut sent_events,
            )?;
        }
    }
    drop(sender);
    source_prepare_latencies.sort_unstable();
    source_send_latencies.sort_unstable();
    let metrics = worker_handle
        .join()
        .map_err(|_| "thread worker panicked".to_owned())??;
    let worker_p99 = live_store_latency_percentile(&metrics.worker_score_latencies, 99);
    let blocker = live_store_worker_thread_blocker(sent_events, &metrics.eval, worker_p99);
    let report = PhaseStreamLiveStoreWorkerThreadReport {
        report_kind: "phase_stream_live_worker_thread_smoke_v1",
        manifest_path: manifest_path.display().to_string(),
        input_sources: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        total_rows,
        parsed_rows,
        route_index_missing_events,
        cells: bundle.cells,
        channel_capacity,
        sent_events,
        worker_warmup_route_scores: metrics.worker_warmup_route_scores,
        hot_profile_count,
        hot_route_count,
        hot_route_profile_edges,
        hot_bytes_estimate,
        worker_kind: "PhaseCenterHotWorker",
        source_adapter_json_used: true,
        bounded_sync_channel_used: true,
        worker_thread_used: true,
        hot_loop_json_used: false,
        hot_loop_string_route_used: false,
        hot_loop_btreemap_used: false,
        hot_loop_file_io_used: false,
        hot_loop_package_compile_used: false,
        score_events: metrics.eval.score_events,
        score_candidate_events: metrics.eval.score_candidate_events,
        verifier_required_events: metrics.eval.verifier_required_events,
        local_accept_events: metrics.eval.local_accept_events,
        unique_cpu_accepts_over_exact_cache: metrics.eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: metrics.eval.tokens_saved,
        cost_saved_microusd: metrics.eval.cost_saved_microusd,
        false_accepts: metrics.eval.false_accepts,
        source_prepare_p50_latency_ns: live_store_latency_percentile(&source_prepare_latencies, 50),
        source_prepare_p90_latency_ns: live_store_latency_percentile(&source_prepare_latencies, 90),
        source_prepare_p99_latency_ns: live_store_latency_percentile(&source_prepare_latencies, 99),
        source_send_p50_latency_ns: live_store_latency_percentile(&source_send_latencies, 50),
        source_send_p90_latency_ns: live_store_latency_percentile(&source_send_latencies, 90),
        source_send_p99_latency_ns: live_store_latency_percentile(&source_send_latencies, 99),
        queue_wait_p50_latency_ns: live_store_latency_percentile(&metrics.queue_wait_latencies, 50),
        queue_wait_p90_latency_ns: live_store_latency_percentile(&metrics.queue_wait_latencies, 90),
        queue_wait_p99_latency_ns: live_store_latency_percentile(&metrics.queue_wait_latencies, 99),
        worker_score_p50_latency_ns: live_store_latency_percentile(
            &metrics.worker_score_latencies,
            50,
        ),
        worker_score_p90_latency_ns: live_store_latency_percentile(
            &metrics.worker_score_latencies,
            90,
        ),
        worker_score_p99_latency_ns: worker_p99,
        worker_score_max_latency_ns: metrics.worker_score_latencies.last().copied().unwrap_or(0),
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        verdict: if blocker == "none" {
            "LIVE_WORKER_THREAD_SMOKE_PASS"
        } else {
            "LIVE_WORKER_THREAD_SMOKE_WATCH"
        },
        blocker,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "live worker thread smoke: source adapter parses/encodes verified events and sends prepared vectors through a bounded sync channel; hot worker thread scores only route_index plus phase vectors; no local_accept or market claim",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_live_worker_thread_smoke_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  channel_capacity: {}", report.channel_capacity);
    println!("  sent_events: {}", report.sent_events);
    println!("  score_events: {}", report.score_events);
    println!(
        "  score_candidate_events: {}",
        report.score_candidate_events
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!(
        "  worker_score_p99_latency_ns: {}",
        report.worker_score_p99_latency_ns
    );
    println!(
        "  queue_wait_p99_latency_ns: {}",
        report.queue_wait_p99_latency_ns
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_live_worker_batch_thread_smoke_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_CLEAN_MANIFEST));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_WORKER_BATCH_THREAD_REPORT));
    let channel_capacity = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid worker channel capacity '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_LIVE_STORE_WORKER_QUEUE_BATCH_CAPACITY)
        .max(1);
    let source_batch_capacity = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid source batch capacity '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_LIVE_STORE_WORKER_QUEUE_BATCH_CAPACITY)
        .max(1);
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };

    let bundle = load_live_store_clean_manifest_runtime(&manifest_path)?;
    let worker = PhaseCenterHotWorker::new(bundle.hot_runtime.clone(), bundle.route_table.clone())
        .map_err(|error| format!("failed to build batch-thread hot worker: {error:?}"))?;
    let hot_profile_count = worker.profile_count();
    let hot_route_count = worker.route_count();
    let hot_route_profile_edges = worker.route_profile_edge_count();
    let hot_bytes_estimate = worker.bytes_estimate();
    let (sender, receiver) = sync_channel::<LiveStoreWorkerBatchMessage>(channel_capacity);
    let worker_handle = thread::spawn(
        move || -> Result<LiveStoreWorkerBatchThreadMetrics, String> {
            let mut worker = worker;
            let warmup_vector = vec![PhaseCenterCell::default(); worker.cells()];
            let mut metrics = LiveStoreWorkerBatchThreadMetrics::default();
            for route_index in 0..worker.route_count() {
                let _ = worker
                    .score_prepared(PhaseCenterPreparedHotRequest::new(
                        route_index,
                        &warmup_vector,
                    ))
                    .map_err(|error| format!("failed batch-thread worker warmup: {error:?}"))?;
                metrics.worker_warmup_route_scores += 1;
            }
            while let Ok(message) = receiver.recv() {
                metrics
                    .batch_wait_latencies
                    .push(message.enqueued_at.elapsed().as_nanos());
                metrics.received_batches += 1;
                metrics.max_received_batch_len =
                    metrics.max_received_batch_len.max(message.rows.len());
                for row in &message.rows {
                    let start = Instant::now();
                    let decisions = worker
                        .score_prepared(PhaseCenterPreparedHotRequest::new(
                            row.route_index,
                            &row.phase_vector,
                        ))
                        .map_err(|error| format!("failed batch-thread worker score: {error:?}"))?;
                    metrics
                        .worker_score_latencies
                        .push(start.elapsed().as_nanos());
                    live_store_update_memory_hot_worker_eval(row, decisions, &mut metrics.eval);
                }
            }
            metrics.batch_wait_latencies.sort_unstable();
            metrics.worker_score_latencies.sort_unstable();
            Ok(metrics)
        },
    );

    let mut encoder = PhaseCenterAtomEncoder::new(bundle.cells)
        .map_err(|error| format!("failed to build batch-thread source encoder: {error:?}"))?;
    let bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut source_prepare_latencies = Vec::<u128>::new();
    let mut source_send_latencies = Vec::<u128>::new();
    let mut batch = Vec::<LiveStorePreparedMemoryRow>::with_capacity(source_batch_capacity);
    let mut total_rows = 0usize;
    let mut parsed_rows = 0usize;
    let mut route_index_missing_events = 0usize;
    let mut sent_events = 0usize;
    let mut sent_batches = 0usize;
    let mut max_sent_batch_len = 0usize;

    for trace_path in &trace_paths {
        if trace_path == Path::new("-") {
            let stdin = io::stdin();
            let reader = stdin.lock();
            live_store_batch_thread_source_reader(
                "<stdin>",
                reader,
                &bundle.route_table,
                &mut encoder,
                &bucket_policy,
                &mut exact_cache_keys_seen,
                &sender,
                source_batch_capacity,
                &mut batch,
                &mut source_prepare_latencies,
                &mut source_send_latencies,
                &mut total_rows,
                &mut parsed_rows,
                &mut route_index_missing_events,
                &mut sent_events,
                &mut sent_batches,
                &mut max_sent_batch_len,
            )?;
        } else {
            let file = File::open(trace_path).map_err(|error| {
                format!("failed to open trace '{}': {error}", trace_path.display())
            })?;
            let reader = io::BufReader::new(file);
            live_store_batch_thread_source_reader(
                &trace_path.display().to_string(),
                reader,
                &bundle.route_table,
                &mut encoder,
                &bucket_policy,
                &mut exact_cache_keys_seen,
                &sender,
                source_batch_capacity,
                &mut batch,
                &mut source_prepare_latencies,
                &mut source_send_latencies,
                &mut total_rows,
                &mut parsed_rows,
                &mut route_index_missing_events,
                &mut sent_events,
                &mut sent_batches,
                &mut max_sent_batch_len,
            )?;
        }
    }
    live_store_send_worker_batch(
        &sender,
        source_batch_capacity,
        &mut batch,
        &mut source_send_latencies,
        &mut sent_events,
        &mut sent_batches,
        &mut max_sent_batch_len,
    )?;
    drop(sender);
    source_prepare_latencies.sort_unstable();
    source_send_latencies.sort_unstable();
    let metrics = worker_handle
        .join()
        .map_err(|_| "batch-thread worker panicked".to_owned())??;
    let worker_p99 = live_store_latency_percentile(&metrics.worker_score_latencies, 99);
    let blocker = live_store_worker_thread_blocker(sent_events, &metrics.eval, worker_p99);
    let report = PhaseStreamLiveStoreWorkerBatchThreadReport {
        report_kind: "phase_stream_live_worker_batch_thread_smoke_v1",
        manifest_path: manifest_path.display().to_string(),
        input_sources: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        total_rows,
        parsed_rows,
        route_index_missing_events,
        cells: bundle.cells,
        channel_capacity,
        source_batch_capacity,
        sent_events,
        sent_batches,
        max_sent_batch_len,
        received_batches: metrics.received_batches,
        max_received_batch_len: metrics.max_received_batch_len,
        worker_warmup_route_scores: metrics.worker_warmup_route_scores,
        hot_profile_count,
        hot_route_count,
        hot_route_profile_edges,
        hot_bytes_estimate,
        worker_kind: "PhaseCenterHotWorker",
        source_adapter_json_used: true,
        bounded_sync_channel_used: true,
        batch_messages_used: true,
        worker_thread_used: true,
        hot_loop_json_used: false,
        hot_loop_string_route_used: false,
        hot_loop_btreemap_used: false,
        hot_loop_file_io_used: false,
        hot_loop_package_compile_used: false,
        score_events: metrics.eval.score_events,
        score_candidate_events: metrics.eval.score_candidate_events,
        verifier_required_events: metrics.eval.verifier_required_events,
        local_accept_events: metrics.eval.local_accept_events,
        unique_cpu_accepts_over_exact_cache: metrics.eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: metrics.eval.tokens_saved,
        cost_saved_microusd: metrics.eval.cost_saved_microusd,
        false_accepts: metrics.eval.false_accepts,
        source_prepare_p50_latency_ns: live_store_latency_percentile(&source_prepare_latencies, 50),
        source_prepare_p90_latency_ns: live_store_latency_percentile(&source_prepare_latencies, 90),
        source_prepare_p99_latency_ns: live_store_latency_percentile(&source_prepare_latencies, 99),
        source_send_p50_latency_ns: live_store_latency_percentile(&source_send_latencies, 50),
        source_send_p90_latency_ns: live_store_latency_percentile(&source_send_latencies, 90),
        source_send_p99_latency_ns: live_store_latency_percentile(&source_send_latencies, 99),
        batch_wait_p50_latency_ns: live_store_latency_percentile(&metrics.batch_wait_latencies, 50),
        batch_wait_p90_latency_ns: live_store_latency_percentile(&metrics.batch_wait_latencies, 90),
        batch_wait_p99_latency_ns: live_store_latency_percentile(&metrics.batch_wait_latencies, 99),
        worker_score_p50_latency_ns: live_store_latency_percentile(
            &metrics.worker_score_latencies,
            50,
        ),
        worker_score_p90_latency_ns: live_store_latency_percentile(
            &metrics.worker_score_latencies,
            90,
        ),
        worker_score_p99_latency_ns: worker_p99,
        worker_score_max_latency_ns: metrics.worker_score_latencies.last().copied().unwrap_or(0),
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        verdict: if blocker == "none" {
            "LIVE_WORKER_BATCH_THREAD_SMOKE_PASS"
        } else {
            "LIVE_WORKER_BATCH_THREAD_SMOKE_WATCH"
        },
        blocker,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "live worker batch-thread smoke: source adapter batches prepared vectors into bounded channel messages; hot worker thread scores batch rows in a tight loop; no local_accept or market claim",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_live_worker_batch_thread_smoke_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  channel_capacity: {}", report.channel_capacity);
    println!("  source_batch_capacity: {}", report.source_batch_capacity);
    println!("  sent_batches: {}", report.sent_batches);
    println!("  score_events: {}", report.score_events);
    println!(
        "  score_candidate_events: {}",
        report.score_candidate_events
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!(
        "  worker_score_p99_latency_ns: {}",
        report.worker_score_p99_latency_ns
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_live_store_direct_batch_thread_smoke_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_STORE_DIRECT_WORKER_BATCH_THREAD_REPORT));
    let channel_capacity = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid worker channel capacity '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_LIVE_STORE_WORKER_QUEUE_BATCH_CAPACITY)
        .max(1);
    let source_batch_capacity = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid source batch capacity '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_LIVE_STORE_WORKER_QUEUE_BATCH_CAPACITY)
        .max(1);
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_CELLS);
    let min_bucket_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid min_bucket_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    if min_bucket_events == 0 {
        return Err("min_bucket_events must be > 0".to_owned());
    }
    let validation_score_event_target = 1usize;

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
                DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_HOT_PROFILES_PER_WORKER,
            max_hot_bytes_per_worker: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
            max_warm_profiles_per_process:
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
            max_profiles_per_route: DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_PROFILES_PER_ROUTE,
            max_route_top_k: DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_ROUTE_TOP_K,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .map_err(|error| format!("failed to create direct live operator store: {error:?}"))?;
    let mut observe_encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create direct observe encoder: {error:?}"))?;
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut parsed_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut total_rows = 0usize;
    let mut parsed_rows = 0usize;
    let mut skipped_no_verifier_label = 0usize;
    let mut skipped_no_safe_atoms = 0usize;
    let mut direct_hot_snapshots =
        LiveStoreDirectHotSnapshotBank::new(DEFAULT_LIVE_STORE_DIRECT_HOT_SNAPSHOT_CAPACITY);

    for trace_path in &trace_paths {
        if trace_path == Path::new("-") {
            let stdin = io::stdin();
            let reader = stdin.lock();
            live_store_collect_direct_store_events(
                "<stdin>",
                reader,
                &mut store,
                &mut observe_encoder,
                &mut bucket_policy,
                &mut exact_cache_keys_seen,
                &mut parsed_events,
                &mut total_rows,
                &mut parsed_rows,
                &mut skipped_no_verifier_label,
                &mut skipped_no_safe_atoms,
                &mut direct_hot_snapshots,
            )?;
        } else {
            let file = File::open(trace_path).map_err(|error| {
                format!("failed to open trace '{}': {error}", trace_path.display())
            })?;
            let reader = io::BufReader::new(file);
            live_store_collect_direct_store_events(
                &trace_path.display().to_string(),
                reader,
                &mut store,
                &mut observe_encoder,
                &mut bucket_policy,
                &mut exact_cache_keys_seen,
                &mut parsed_events,
                &mut total_rows,
                &mut parsed_rows,
                &mut skipped_no_verifier_label,
                &mut skipped_no_safe_atoms,
                &mut direct_hot_snapshots,
            )?;
        }
    }

    let summary = store.summary();
    let runtime_budget = live_store_budget_report(store.runtime_budget_snapshot());
    let Some(selected_snapshot_eval) =
        live_store_select_direct_hot_snapshot(&direct_hot_snapshots, &parsed_events, cells)?
    else {
        let report = PhaseStreamLiveStoreDirectWorkerBatchThreadReport {
            report_kind: "phase_stream_live_store_direct_batch_thread_smoke_v1",
            runtime_source: "mutable_live_store_in_memory_frozen_hot_snapshot",
            input_trace_paths: trace_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            total_rows,
            parsed_rows,
            skipped_no_verifier_label,
            skipped_no_safe_atoms,
            route_index_missing_events: 0,
            cells,
            min_bucket_events,
            channel_capacity,
            source_batch_capacity,
            sent_events: 0,
            sent_batches: 0,
            max_sent_batch_len: 0,
            received_batches: 0,
            max_received_batch_len: 0,
            worker_warmup_route_scores: 0,
            online_bucket_count: summary.bucket_count,
            candidate_bucket_count: summary.candidate_bucket_count,
            rejected_bucket_count: summary.rejected_bucket_count,
            direct_hot_snapshot_capacity: direct_hot_snapshots.capacity(),
            direct_hot_snapshot_captured_count: direct_hot_snapshots.captured_count(),
            direct_hot_snapshot_count: direct_hot_snapshots.retained_count(),
            direct_hot_snapshot_evicted_count: direct_hot_snapshots.evicted_count(),
            selected_direct_hot_snapshot_index: 0,
            selected_direct_hot_snapshot_frozen_after_parsed_rows: 0,
            validation_score_event_target,
            selected_validation_score_events: 0,
            selected_validation_score_candidate_events: 0,
            selected_validation_unique_cpu_accepts_over_exact_cache: 0,
            selected_validation_tokens_saved: 0,
            selected_validation_cost_saved_microusd: 0,
            selected_validation_false_accepts: 0,
            future_eval_start_after_parsed_rows: 0,
            hot_profile_count: 0,
            hot_route_count: 0,
            hot_route_profile_edges: 0,
            hot_bytes_estimate: 0,
            runtime_budget,
            worker_kind: "PhaseCenterHotWorker",
            cold_adapter_json_used: true,
            cold_adapter_strings_used: true,
            direct_mutable_store_export_used: true,
            clean_manifest_loaded: false,
            package_roundtrip_used: false,
            bounded_sync_channel_used: false,
            batch_messages_used: false,
            worker_thread_used: false,
            hot_loop_json_used: false,
            hot_loop_string_route_used: false,
            hot_loop_btreemap_used: false,
            hot_loop_file_io_used: false,
            hot_loop_package_compile_used: false,
            score_events: 0,
            score_candidate_events: 0,
            verifier_required_events: 0,
            local_accept_events: 0,
            unique_cpu_accepts_over_exact_cache: 0,
            tokens_saved: 0,
            cost_saved_microusd: 0,
            false_accepts: 0,
            source_prepare_p50_latency_ns: 0,
            source_prepare_p90_latency_ns: 0,
            source_prepare_p99_latency_ns: 0,
            source_send_p50_latency_ns: 0,
            source_send_p90_latency_ns: 0,
            source_send_p99_latency_ns: 0,
            batch_wait_p50_latency_ns: 0,
            batch_wait_p90_latency_ns: 0,
            batch_wait_p99_latency_ns: 0,
            worker_score_p50_latency_ns: 0,
            worker_score_p90_latency_ns: 0,
            worker_score_p99_latency_ns: 0,
            worker_score_max_latency_ns: 0,
            local_accept_enabled: false,
            market_money_claim_allowed: false,
            verdict: "LIVE_STORE_DIRECT_BATCH_THREAD_SMOKE_WATCH",
            blocker: "direct_mutable_store_no_safe_frozen_snapshot",
            forbidden_flags: super::ForbiddenFlags {
                target_id_used: false,
                proof_rule_id_authority_used: false,
                concrete_x_lookup_used: false,
                manual_local_out_t_used: false,
                hidden_frame_id_or_bind_x_used: false,
                legacy_backend_used: false,
            },
            boundary: "direct mutable live-store batch-thread smoke: cold adapter builds bounded phase-center store, freezes verifier-bound hot snapshots in memory without clean manifest or package roundtrip, then hot worker scores prepared numeric vectors only",
        };
        super::write_json_file(&report_path, &report)?;
        println!("phase_stream_live_store_direct_batch_thread_smoke_v1:");
        println!("  report_path: {}", report_path.display());
        println!("  verdict: {}", report.verdict);
        println!("  blocker: {}", report.blocker);
        return Ok(());
    };
    let selected_snapshot = direct_hot_snapshots
        .get(selected_snapshot_eval.snapshot_index)
        .ok_or_else(|| "selected direct hot snapshot index missing".to_owned())?;
    let hot_runtime = selected_snapshot.hot_runtime.clone();
    let route_table = selected_snapshot.route_table.clone();

    let worker = PhaseCenterHotWorker::new(hot_runtime.clone(), route_table.clone())
        .map_err(|error| format!("failed to build direct batch-thread hot worker: {error:?}"))?;
    let hot_profile_count = worker.profile_count();
    let hot_route_count = worker.route_count();
    let hot_route_profile_edges = worker.route_profile_edge_count();
    let hot_bytes_estimate = worker.bytes_estimate();
    let (sender, receiver) = sync_channel::<LiveStoreWorkerBatchMessage>(channel_capacity);
    let worker_handle = thread::spawn(
        move || -> Result<LiveStoreWorkerBatchThreadMetrics, String> {
            let mut worker = worker;
            let warmup_vector = vec![PhaseCenterCell::default(); worker.cells()];
            let mut metrics = LiveStoreWorkerBatchThreadMetrics::default();
            for route_index in 0..worker.route_count() {
                let _ = worker
                    .score_prepared(PhaseCenterPreparedHotRequest::new(
                        route_index,
                        &warmup_vector,
                    ))
                    .map_err(|error| {
                        format!("failed direct batch-thread worker warmup: {error:?}")
                    })?;
                metrics.worker_warmup_route_scores += 1;
            }
            while let Ok(message) = receiver.recv() {
                metrics
                    .batch_wait_latencies
                    .push(message.enqueued_at.elapsed().as_nanos());
                metrics.received_batches += 1;
                metrics.max_received_batch_len =
                    metrics.max_received_batch_len.max(message.rows.len());
                for row in &message.rows {
                    let start = Instant::now();
                    let decisions = worker
                        .score_prepared(PhaseCenterPreparedHotRequest::new(
                            row.route_index,
                            &row.phase_vector,
                        ))
                        .map_err(|error| {
                            format!("failed direct batch-thread worker score: {error:?}")
                        })?;
                    metrics
                        .worker_score_latencies
                        .push(start.elapsed().as_nanos());
                    live_store_update_memory_hot_worker_eval(row, decisions, &mut metrics.eval);
                }
            }
            metrics.batch_wait_latencies.sort_unstable();
            metrics.worker_score_latencies.sort_unstable();
            Ok(metrics)
        },
    );

    let mut score_encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to build direct score encoder: {error:?}"))?;
    let mut source_prepare_latencies = Vec::<u128>::new();
    let mut source_send_latencies = Vec::<u128>::new();
    let mut batch = Vec::<LiveStorePreparedMemoryRow>::with_capacity(source_batch_capacity);
    let mut route_index_missing_events = 0usize;
    let mut sent_events = 0usize;
    let mut sent_batches = 0usize;
    let mut max_sent_batch_len = 0usize;
    let future_eval_events = parsed_events
        .get(selected_snapshot_eval.future_eval_start_after_parsed_rows..)
        .unwrap_or(&[]);
    live_store_send_parsed_events_to_batch_worker(
        future_eval_events,
        &route_table,
        &mut score_encoder,
        &sender,
        source_batch_capacity,
        &mut batch,
        &mut source_prepare_latencies,
        &mut source_send_latencies,
        &mut route_index_missing_events,
        &mut sent_events,
        &mut sent_batches,
        &mut max_sent_batch_len,
    )?;
    live_store_send_worker_batch(
        &sender,
        source_batch_capacity,
        &mut batch,
        &mut source_send_latencies,
        &mut sent_events,
        &mut sent_batches,
        &mut max_sent_batch_len,
    )?;
    drop(sender);
    source_prepare_latencies.sort_unstable();
    source_send_latencies.sort_unstable();
    let metrics = worker_handle
        .join()
        .map_err(|_| "direct batch-thread worker panicked".to_owned())??;
    let worker_p99 = live_store_latency_percentile(&metrics.worker_score_latencies, 99);
    let mut blocker = live_store_worker_thread_blocker(sent_events, &metrics.eval, worker_p99);
    if !runtime_budget.product_runtime_budget_passed && blocker == "none" {
        blocker = "direct_batch_thread_budget_failed";
    }
    let report = PhaseStreamLiveStoreDirectWorkerBatchThreadReport {
        report_kind: "phase_stream_live_store_direct_batch_thread_smoke_v1",
        runtime_source: "mutable_live_store_in_memory_frozen_hot_snapshot",
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        total_rows,
        parsed_rows,
        skipped_no_verifier_label,
        skipped_no_safe_atoms,
        route_index_missing_events,
        cells,
        min_bucket_events,
        channel_capacity,
        source_batch_capacity,
        sent_events,
        sent_batches,
        max_sent_batch_len,
        received_batches: metrics.received_batches,
        max_received_batch_len: metrics.max_received_batch_len,
        worker_warmup_route_scores: metrics.worker_warmup_route_scores,
        online_bucket_count: summary.bucket_count,
        candidate_bucket_count: summary.candidate_bucket_count,
        rejected_bucket_count: summary.rejected_bucket_count,
        direct_hot_snapshot_capacity: direct_hot_snapshots.capacity(),
        direct_hot_snapshot_captured_count: direct_hot_snapshots.captured_count(),
        direct_hot_snapshot_count: direct_hot_snapshots.retained_count(),
        direct_hot_snapshot_evicted_count: direct_hot_snapshots.evicted_count(),
        selected_direct_hot_snapshot_index: selected_snapshot_eval.snapshot_index,
        selected_direct_hot_snapshot_frozen_after_parsed_rows: selected_snapshot_eval
            .frozen_after_parsed_rows,
        validation_score_event_target,
        selected_validation_score_events: selected_snapshot_eval.validation_score_events,
        selected_validation_score_candidate_events: selected_snapshot_eval
            .validation_eval
            .score_candidate_events,
        selected_validation_unique_cpu_accepts_over_exact_cache: selected_snapshot_eval
            .validation_eval
            .unique_cpu_accepts_over_exact_cache,
        selected_validation_tokens_saved: selected_snapshot_eval.validation_eval.tokens_saved,
        selected_validation_cost_saved_microusd: selected_snapshot_eval
            .validation_eval
            .cost_saved_microusd,
        selected_validation_false_accepts: selected_snapshot_eval.validation_eval.false_accepts,
        future_eval_start_after_parsed_rows: selected_snapshot_eval
            .future_eval_start_after_parsed_rows,
        hot_profile_count,
        hot_route_count,
        hot_route_profile_edges,
        hot_bytes_estimate,
        runtime_budget,
        worker_kind: "PhaseCenterHotWorker",
        cold_adapter_json_used: true,
        cold_adapter_strings_used: true,
        direct_mutable_store_export_used: true,
        clean_manifest_loaded: false,
        package_roundtrip_used: false,
        bounded_sync_channel_used: true,
        batch_messages_used: true,
        worker_thread_used: true,
        hot_loop_json_used: false,
        hot_loop_string_route_used: false,
        hot_loop_btreemap_used: false,
        hot_loop_file_io_used: false,
        hot_loop_package_compile_used: false,
        score_events: metrics.eval.score_events,
        score_candidate_events: metrics.eval.score_candidate_events,
        verifier_required_events: metrics.eval.verifier_required_events,
        local_accept_events: metrics.eval.local_accept_events,
        unique_cpu_accepts_over_exact_cache: metrics.eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: metrics.eval.tokens_saved,
        cost_saved_microusd: metrics.eval.cost_saved_microusd,
        false_accepts: metrics.eval.false_accepts,
        source_prepare_p50_latency_ns: live_store_latency_percentile(&source_prepare_latencies, 50),
        source_prepare_p90_latency_ns: live_store_latency_percentile(&source_prepare_latencies, 90),
        source_prepare_p99_latency_ns: live_store_latency_percentile(&source_prepare_latencies, 99),
        source_send_p50_latency_ns: live_store_latency_percentile(&source_send_latencies, 50),
        source_send_p90_latency_ns: live_store_latency_percentile(&source_send_latencies, 90),
        source_send_p99_latency_ns: live_store_latency_percentile(&source_send_latencies, 99),
        batch_wait_p50_latency_ns: live_store_latency_percentile(&metrics.batch_wait_latencies, 50),
        batch_wait_p90_latency_ns: live_store_latency_percentile(&metrics.batch_wait_latencies, 90),
        batch_wait_p99_latency_ns: live_store_latency_percentile(&metrics.batch_wait_latencies, 99),
        worker_score_p50_latency_ns: live_store_latency_percentile(
            &metrics.worker_score_latencies,
            50,
        ),
        worker_score_p90_latency_ns: live_store_latency_percentile(
            &metrics.worker_score_latencies,
            90,
        ),
        worker_score_p99_latency_ns: worker_p99,
        worker_score_max_latency_ns: metrics.worker_score_latencies.last().copied().unwrap_or(0),
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        verdict: if blocker == "none" {
            "LIVE_STORE_DIRECT_BATCH_THREAD_SMOKE_PASS"
        } else {
            "LIVE_STORE_DIRECT_BATCH_THREAD_SMOKE_WATCH"
        },
        blocker,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "direct mutable live-store batch-thread smoke: cold adapter builds bounded phase-center store, freezes verifier-bound hot snapshots in memory without clean manifest or package roundtrip, then hot worker scores prepared numeric vectors only",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_live_store_direct_batch_thread_smoke_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  direct_mutable_store_export_used: true");
    println!("  clean_manifest_loaded: false");
    println!("  package_roundtrip_used: false");
    println!("  sent_batches: {}", report.sent_batches);
    println!("  score_events: {}", report.score_events);
    println!(
        "  score_candidate_events: {}",
        report.score_candidate_events
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!(
        "  worker_score_p99_latency_ns: {}",
        report.worker_score_p99_latency_ns
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_hot_path_benchmark_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOT_PATH_BENCHMARK_REPORT));
    let timed_score_iterations_requested = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid timed score iterations '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_HOT_PATH_BENCHMARK_ITERATIONS)
        .max(1);
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_CELLS);
    let min_bucket_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid min_bucket_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    if min_bucket_events == 0 {
        return Err("min_bucket_events must be > 0".to_owned());
    }
    let input_trace_paths = trace_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let promotion_review_report_path = live_store_hot_path_promotion_review_path(&report_path);
    let daemon_admission_policy_report_path =
        live_store_hot_path_daemon_admission_policy_path(&report_path);

    let validation_score_event_target = 1usize;
    let verifier_binding = live_store_verifier_binding();
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
                DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_HOT_PROFILES_PER_WORKER,
            max_hot_bytes_per_worker: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
            max_warm_profiles_per_process:
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
            max_profiles_per_route: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_PROFILES_PER_ROUTE,
            max_route_top_k: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_ROUTE_TOP_K,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .map_err(|error| format!("failed to create hot-path live operator store: {error:?}"))?;
    let mut observe_encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create hot-path observe encoder: {error:?}"))?;
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut parsed_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut total_rows = 0usize;
    let mut parsed_rows = 0usize;
    let mut skipped_no_verifier_label = 0usize;
    let mut skipped_no_safe_atoms = 0usize;
    let mut direct_hot_snapshots =
        LiveStoreDirectHotSnapshotBank::new(DEFAULT_LIVE_STORE_DIRECT_HOT_SNAPSHOT_CAPACITY);

    for trace_path in &trace_paths {
        if trace_path == Path::new("-") {
            let stdin = io::stdin();
            let reader = stdin.lock();
            live_store_collect_direct_store_events(
                "<stdin>",
                reader,
                &mut store,
                &mut observe_encoder,
                &mut bucket_policy,
                &mut exact_cache_keys_seen,
                &mut parsed_events,
                &mut total_rows,
                &mut parsed_rows,
                &mut skipped_no_verifier_label,
                &mut skipped_no_safe_atoms,
                &mut direct_hot_snapshots,
            )?;
        } else {
            let file = File::open(trace_path).map_err(|error| {
                format!("failed to open trace '{}': {error}", trace_path.display())
            })?;
            let reader = io::BufReader::new(file);
            live_store_collect_direct_store_events(
                &trace_path.display().to_string(),
                reader,
                &mut store,
                &mut observe_encoder,
                &mut bucket_policy,
                &mut exact_cache_keys_seen,
                &mut parsed_events,
                &mut total_rows,
                &mut parsed_rows,
                &mut skipped_no_verifier_label,
                &mut skipped_no_safe_atoms,
                &mut direct_hot_snapshots,
            )?;
        }
    }

    let summary = store.summary();
    let runtime_budget = live_store_budget_report(store.runtime_budget_snapshot());
    let Some(selected_snapshot_eval) =
        live_store_select_direct_hot_snapshot(&direct_hot_snapshots, &parsed_events, cells)?
    else {
        let report = PhaseStreamHotPathBenchmarkReport {
            report_kind: "phase_stream_hot_path_benchmark_v1",
            runtime_source: "mutable_live_store_in_memory_frozen_hot_snapshot",
            input_trace_paths: input_trace_paths.clone(),
            promotion_review_report_path: promotion_review_report_path.display().to_string(),
            daemon_admission_policy_report_path: daemon_admission_policy_report_path
                .display()
                .to_string(),
            total_rows,
            parsed_rows,
            skipped_no_verifier_label,
            skipped_no_safe_atoms,
            route_index_missing_events: 0,
            cells,
            min_bucket_events,
            timed_score_iterations_requested,
            timed_score_iterations: 0,
            prepared_unique_rows: 0,
            online_bucket_count: summary.bucket_count,
            candidate_bucket_count: summary.candidate_bucket_count,
            rejected_bucket_count: summary.rejected_bucket_count,
            direct_hot_snapshot_capacity: direct_hot_snapshots.capacity(),
            direct_hot_snapshot_captured_count: direct_hot_snapshots.captured_count(),
            direct_hot_snapshot_count: direct_hot_snapshots.retained_count(),
            direct_hot_snapshot_evicted_count: direct_hot_snapshots.evicted_count(),
            selected_direct_hot_snapshot_index: 0,
            selected_direct_hot_snapshot_frozen_after_parsed_rows: 0,
            validation_score_event_target,
            selected_validation_score_events: 0,
            selected_validation_score_candidate_events: 0,
            selected_validation_unique_cpu_accepts_over_exact_cache: 0,
            selected_validation_tokens_saved: 0,
            selected_validation_cost_saved_microusd: 0,
            selected_validation_false_accepts: 0,
            future_eval_start_after_parsed_rows: 0,
            future_shadow_split_used: false,
            future_total_tokens: 0,
            future_total_cost_microusd: 0,
            future_exact_cache_hits: 0,
            future_exact_cache_tokens: 0,
            future_exact_cache_cost_microusd: 0,
            future_non_exact_rows: 0,
            hot_profile_count: 0,
            hot_route_count: 0,
            hot_route_profile_edges: 0,
            hot_bytes_estimate: 0,
            hot_scratch_bytes_estimate: 0,
            runtime_budget,
            cold_adapter_json_used: true,
            cold_adapter_strings_used: true,
            direct_mutable_store_export_used: true,
            clean_manifest_loaded: false,
            package_roundtrip_used: false,
            worker_thread_used: false,
            bounded_sync_channel_used: false,
            batch_messages_used: false,
            hot_path_direct_runtime_used: false,
            hot_loop_json_used: false,
            hot_loop_string_route_used: false,
            hot_loop_btreemap_used: false,
            hot_loop_file_io_used: false,
            hot_loop_package_compile_used: false,
            score_events: 0,
            score_candidate_events: 0,
            verifier_required_events: 0,
            local_accept_events: 0,
            unique_cpu_accepts_over_exact_cache: 0,
            tokens_saved: 0,
            cost_saved_microusd: 0,
            false_accepts: 0,
            runtime_margin_parity_checks: 0,
            runtime_margin_parity_mismatches: 0,
            runtime_decision_parity_mismatches: 0,
            exact_cache_overlap_excluded: false,
            token_cost_denominator_present: false,
            nando_calls_saved_milli: 0,
            nando_tokens_saved_milli: 0,
            nando_cost_saved_milli: 0,
            p5_evidence_ready_for_promotion_audit: false,
            verifier_binding_bound: verifier_binding.is_bound(),
            promotion_contract_evaluated: true,
            promotion_contract_eligible: false,
            promotion_contract_blocker: "hot_path_no_safe_frozen_snapshot",
            promotion_review_ready_not_promoted: false,
            product_promotion_enabled: false,
            hot_path_p50_latency_ns: 0,
            hot_path_p90_latency_ns: 0,
            hot_path_p99_latency_ns: 0,
            hot_path_max_latency_ns: 0,
            hot_path_margin_checksum_micro: 0,
            local_accept_enabled: false,
            market_money_claim_allowed: false,
            verdict: "HOT_PATH_BENCHMARK_WATCH",
            blocker: "hot_path_no_safe_frozen_snapshot",
            forbidden_flags: super::ForbiddenFlags {
                target_id_used: false,
                proof_rule_id_authority_used: false,
                concrete_x_lookup_used: false,
                manual_local_out_t_used: false,
                hidden_frame_id_or_bind_x_used: false,
                legacy_backend_used: false,
            },
            boundary: "hot-path benchmark: no safe verifier-bound frozen snapshot found; no product local_accept or market claim",
        };
        let review = PhaseStreamHotPathPromotionReviewReport {
            report_kind: "phase_stream_hot_path_promotion_review_v1",
            mode: "shadow_only_promotion_review_artifact_no_runtime_mutation",
            source_benchmark_report_path: report_path.display().to_string(),
            daemon_admission_policy_report_path: daemon_admission_policy_report_path
                .display()
                .to_string(),
            runtime_source: report.runtime_source,
            input_trace_paths: input_trace_paths.clone(),
            cells,
            min_bucket_events,
            selected_direct_hot_snapshot_index: 0,
            selected_direct_hot_snapshot_frozen_after_parsed_rows: 0,
            future_eval_start_after_parsed_rows: 0,
            future_shadow_split_used: false,
            hot_route_ids: Vec::new(),
            hot_profile_ids: Vec::new(),
            hot_profile_count: 0,
            hot_route_count: 0,
            hot_route_profile_edges: 0,
            hot_bytes_estimate: 0,
            score_events: 0,
            score_candidate_events: 0,
            unique_cpu_accepts_over_exact_cache: 0,
            tokens_saved: 0,
            cost_saved_microusd: 0,
            false_accepts: 0,
            runtime_margin_parity_checks: 0,
            runtime_margin_parity_mismatches: 0,
            runtime_decision_parity_mismatches: 0,
            exact_cache_overlap_excluded: false,
            token_cost_denominator_present: false,
            verifier_binding_bound: verifier_binding.is_bound(),
            verifier_id: verifier_binding.verifier_id,
            verifier_version: verifier_binding.verifier_version,
            verifier_input_kind_id: verifier_binding.verifier_input_kind_id,
            verifier_evidence_source_id: verifier_binding.verifier_evidence_source_id,
            verifier_false_accept_threshold: verifier_binding.false_accept_threshold,
            p5_evidence_ready_for_promotion_audit: false,
            promotion_contract_eligible: false,
            promotion_contract_blocker: "hot_path_no_safe_frozen_snapshot",
            promotion_review_candidate_allowed: false,
            shadow_only_daemon_admission_review_allowed: false,
            product_promotion_enabled: false,
            local_accept_enabled: false,
            market_money_claim_allowed: false,
            verdict: "HOT_PATH_PROMOTION_REVIEW_WATCH",
            blocker: "hot_path_no_safe_frozen_snapshot",
            forbidden_flags: super::ForbiddenFlags {
                target_id_used: false,
                proof_rule_id_authority_used: false,
                concrete_x_lookup_used: false,
                manual_local_out_t_used: false,
                hidden_frame_id_or_bind_x_used: false,
                legacy_backend_used: false,
            },
            boundary: "shadow-only promotion review artifact: no safe hot-path snapshot exists; no product promotion, no local_accept, no market claim",
        };
        let daemon_policy = PhaseStreamHotPathDaemonAdmissionPolicyReport {
            report_kind: "phase_stream_hot_path_daemon_admission_policy_v1",
            mode: "shadow_only_daemon_admission_policy_no_runtime_mutation",
            source_benchmark_report_path: report_path.display().to_string(),
            source_promotion_review_report_path: promotion_review_report_path.display().to_string(),
            admission_policy_kind: "hot_path_promotion_review_to_daemon_admission_candidate_v1",
            runtime_source: report.runtime_source,
            hot_route_ids: Vec::new(),
            hot_profile_ids: Vec::new(),
            hot_profile_count: 0,
            hot_route_count: 0,
            hot_route_profile_edges: 0,
            hot_bytes_estimate: 0,
            future_shadow_split_used: false,
            verifier_binding_bound: verifier_binding.is_bound(),
            exact_cache_overlap_excluded: false,
            token_cost_denominator_present: false,
            unique_cpu_accepts_over_exact_cache: 0,
            tokens_saved: 0,
            cost_saved_microusd: 0,
            false_accepts: 0,
            runtime_margin_parity_mismatches: 0,
            runtime_decision_parity_mismatches: 0,
            promotion_review_candidate_allowed: false,
            shadow_only_daemon_admission_review_allowed: false,
            admission_policy_candidate_allowed: false,
            registry_mutation_enabled: false,
            cpu_profile_registry_write_enabled: false,
            serving_profile_artifact_written: false,
            product_promotion_enabled: false,
            local_accept_enabled: false,
            market_money_claim_allowed: false,
            verdict: "HOT_PATH_DAEMON_ADMISSION_POLICY_WATCH",
            blocker: "hot_path_no_safe_frozen_snapshot",
            forbidden_flags: super::ForbiddenFlags {
                target_id_used: false,
                proof_rule_id_authority_used: false,
                concrete_x_lookup_used: false,
                manual_local_out_t_used: false,
                hidden_frame_id_or_bind_x_used: false,
                legacy_backend_used: false,
            },
            boundary: "shadow-only daemon admission policy artifact: no safe hot-path snapshot exists; no registry mutation, no serving profile write, no product promotion, no local_accept, no market claim",
        };
        super::write_json_file(&promotion_review_report_path, &review)?;
        super::write_json_file(&daemon_admission_policy_report_path, &daemon_policy)?;
        super::write_json_file(&report_path, &report)?;
        println!("phase_stream_hot_path_benchmark_v1:");
        println!("  report_path: {}", report_path.display());
        println!(
            "  promotion_review_report_path: {}",
            promotion_review_report_path.display()
        );
        println!(
            "  daemon_admission_policy_report_path: {}",
            daemon_admission_policy_report_path.display()
        );
        println!("  verdict: {}", report.verdict);
        println!("  blocker: {}", report.blocker);
        return Ok(());
    };

    let selected_snapshot = direct_hot_snapshots
        .get(selected_snapshot_eval.snapshot_index)
        .ok_or_else(|| "selected hot-path snapshot index missing".to_owned())?;
    let hot_runtime = selected_snapshot.hot_runtime.clone();
    let route_table = selected_snapshot.route_table.clone();
    let future_eval_events = parsed_events
        .get(selected_snapshot_eval.future_eval_start_after_parsed_rows..)
        .unwrap_or(&[]);
    let mut prepare_encoder = PhaseCenterHotRowPreparer::new(cells)
        .map_err(|error| format!("failed to build hot-path row preparer: {error:?}"))?;
    let mut route_index_missing_events = 0usize;
    let prepared_rows = live_store_prepare_parsed_events_for_hot_path(
        future_eval_events,
        &route_table,
        &mut prepare_encoder,
        &mut route_index_missing_events,
    )?;
    let mut scratch = PhaseCenterHotScratch::new(cells, route_table.profile_edge_count().max(1))
        .map_err(|error| format!("failed to build hot-path scratch: {error:?}"))?;
    let hot_scratch_bytes_estimate = scratch.bytes_estimate();

    let mut eval = LiveStorePreparedHotPackEval::default();
    hot_runtime
        .score_prepared_hot_rows_into(&route_table, &prepared_rows, &mut scratch, &mut eval)
        .map_err(|error| format!("failed hot-path unique eval score: {error:?}"))?;
    let denominator = live_store_hot_path_denominator(&prepared_rows);
    let (
        runtime_margin_parity_checks,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
    ) = live_store_hot_path_runtime_parity(&hot_runtime, &route_table, &prepared_rows, cells)?;

    let mut latencies = Vec::<u128>::with_capacity(timed_score_iterations_requested);
    let mut hot_path_margin_checksum_micro = 0i128;
    let timed_score_iterations = if prepared_rows.is_empty() {
        0
    } else {
        timed_score_iterations_requested
    };
    for iteration in 0..timed_score_iterations {
        let row = &prepared_rows[iteration % prepared_rows.len()];
        let start = Instant::now();
        let decisions = hot_runtime
            .score_prepared_hot_request_candidates(
                &route_table,
                PhaseCenterPreparedHotRequest::new(row.route_index, &row.phase_vector),
                &mut scratch,
            )
            .map_err(|error| format!("failed hot-path benchmark score: {error:?}"))?;
        latencies.push(start.elapsed().as_nanos());
        for decision in decisions {
            hot_path_margin_checksum_micro =
                hot_path_margin_checksum_micro.wrapping_add(decision.margin_micro as i128);
            if decision.score_candidate {
                hot_path_margin_checksum_micro = hot_path_margin_checksum_micro.wrapping_add(1);
            }
        }
    }
    latencies.sort_unstable();

    let hot_path_p99 = live_store_latency_percentile(&latencies, 99);
    let mut blocker = live_store_hot_path_benchmark_blocker(
        prepared_rows.len(),
        timed_score_iterations,
        &eval,
        hot_path_p99,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
        runtime_budget.product_runtime_budget_passed,
    );
    if hot_path_margin_checksum_micro == 0 && timed_score_iterations > 0 && blocker == "none" {
        blocker = "hot_path_checksum_zero";
    }
    let future_shadow_split_used = selected_snapshot_eval.future_eval_start_after_parsed_rows
        > selected_snapshot_eval.frozen_after_parsed_rows;
    let exact_cache_overlap_excluded = denominator.non_exact_rows > 0
        && eval.unique_cpu_accepts_over_exact_cache <= denominator.non_exact_rows;
    let token_cost_denominator_present =
        denominator.total_tokens > 0 && denominator.total_cost_microusd > 0;
    let p5_evidence_ready_for_promotion_audit = future_shadow_split_used
        && eval.false_accepts == 0
        && runtime_margin_parity_mismatches == 0
        && runtime_decision_parity_mismatches == 0
        && exact_cache_overlap_excluded
        && token_cost_denominator_present
        && eval.local_accept_events == 0;
    let promotion_evidence = PhaseCenterPromotionEvidence {
        future_shadow_events: eval.score_events,
        unique_cpu_accepts_over_exact_cache: eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: eval.tokens_saved,
        cost_saved_microusd: eval.cost_saved_microusd,
        false_accepts: eval.false_accepts,
        runtime_margin_parity_mismatches: runtime_margin_parity_mismatches
            .saturating_add(runtime_decision_parity_mismatches),
        verifier_binding,
        threshold_policy: PhaseCenterThresholdPolicyEvidence {
            candidate_bucket_count: summary.candidate_bucket_count,
            auto_calibrated_bucket_count: summary.candidate_bucket_count,
            calibration_window_before_shadow: future_shadow_split_used,
            shadow_window_after_calibration: eval.score_events > 0,
            per_bucket_thresholds_reported: summary.candidate_bucket_count > 0,
            fixed_policy_shadow_replay: runtime_margin_parity_checks > 0,
        },
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        local_accept_enabled: false,
    };
    let promotion_contract = promotion_evidence.evaluate();
    let promotion_contract_blocker = promotion_contract
        .blocker
        .map(live_store_promotion_blocker_name)
        .unwrap_or("none");
    let hot_route_ids = live_store_hot_route_ids(&route_table);
    let hot_profile_ids = live_store_hot_profile_ids(&hot_runtime);
    let promotion_review_candidate_allowed =
        p5_evidence_ready_for_promotion_audit && promotion_contract.eligible;
    let promotion_review = PhaseStreamHotPathPromotionReviewReport {
        report_kind: "phase_stream_hot_path_promotion_review_v1",
        mode: "shadow_only_promotion_review_artifact_no_runtime_mutation",
        source_benchmark_report_path: report_path.display().to_string(),
        daemon_admission_policy_report_path: daemon_admission_policy_report_path
            .display()
            .to_string(),
        runtime_source: "mutable_live_store_in_memory_frozen_hot_snapshot",
        input_trace_paths: input_trace_paths.clone(),
        cells,
        min_bucket_events,
        selected_direct_hot_snapshot_index: selected_snapshot_eval.snapshot_index,
        selected_direct_hot_snapshot_frozen_after_parsed_rows: selected_snapshot_eval
            .frozen_after_parsed_rows,
        future_eval_start_after_parsed_rows: selected_snapshot_eval
            .future_eval_start_after_parsed_rows,
        future_shadow_split_used,
        hot_route_ids: hot_route_ids.clone(),
        hot_profile_ids: hot_profile_ids.clone(),
        hot_profile_count: hot_runtime.profile_count(),
        hot_route_count: route_table.route_count(),
        hot_route_profile_edges: route_table.profile_edge_count(),
        hot_bytes_estimate: hot_runtime
            .bytes_estimate()
            .saturating_add(route_table.bytes_estimate()),
        score_events: eval.score_events,
        score_candidate_events: eval.score_candidate_events,
        unique_cpu_accepts_over_exact_cache: eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: eval.tokens_saved,
        cost_saved_microusd: eval.cost_saved_microusd,
        false_accepts: eval.false_accepts,
        runtime_margin_parity_checks,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        verifier_binding_bound: verifier_binding.is_bound(),
        verifier_id: verifier_binding.verifier_id,
        verifier_version: verifier_binding.verifier_version,
        verifier_input_kind_id: verifier_binding.verifier_input_kind_id,
        verifier_evidence_source_id: verifier_binding.verifier_evidence_source_id,
        verifier_false_accept_threshold: verifier_binding.false_accept_threshold,
        p5_evidence_ready_for_promotion_audit,
        promotion_contract_eligible: promotion_contract.eligible,
        promotion_contract_blocker,
        promotion_review_candidate_allowed,
        shadow_only_daemon_admission_review_allowed: promotion_review_candidate_allowed,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        verdict: if promotion_review_candidate_allowed {
            "HOT_PATH_PROMOTION_REVIEW_READY"
        } else {
            "HOT_PATH_PROMOTION_REVIEW_WATCH"
        },
        blocker: if promotion_review_candidate_allowed {
            "none"
        } else {
            promotion_contract_blocker
        },
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "shadow-only promotion review artifact: verifier-bound hot-path evidence is eligible for a separate daemon admission review, but this artifact does not promote product runtime, enable local_accept, write a serving profile, or allow market money claims",
    };
    let daemon_policy = PhaseStreamHotPathDaemonAdmissionPolicyReport {
        report_kind: "phase_stream_hot_path_daemon_admission_policy_v1",
        mode: "shadow_only_daemon_admission_policy_no_runtime_mutation",
        source_benchmark_report_path: report_path.display().to_string(),
        source_promotion_review_report_path: promotion_review_report_path.display().to_string(),
        admission_policy_kind: "hot_path_promotion_review_to_daemon_admission_candidate_v1",
        runtime_source: "mutable_live_store_in_memory_frozen_hot_snapshot",
        hot_route_ids,
        hot_profile_ids,
        hot_profile_count: hot_runtime.profile_count(),
        hot_route_count: route_table.route_count(),
        hot_route_profile_edges: route_table.profile_edge_count(),
        hot_bytes_estimate: hot_runtime
            .bytes_estimate()
            .saturating_add(route_table.bytes_estimate()),
        future_shadow_split_used,
        verifier_binding_bound: verifier_binding.is_bound(),
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        unique_cpu_accepts_over_exact_cache: eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: eval.tokens_saved,
        cost_saved_microusd: eval.cost_saved_microusd,
        false_accepts: eval.false_accepts,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
        promotion_review_candidate_allowed,
        shadow_only_daemon_admission_review_allowed: promotion_review_candidate_allowed,
        admission_policy_candidate_allowed: promotion_review_candidate_allowed,
        registry_mutation_enabled: false,
        cpu_profile_registry_write_enabled: false,
        serving_profile_artifact_written: false,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        verdict: if promotion_review_candidate_allowed {
            "HOT_PATH_DAEMON_ADMISSION_POLICY_READY"
        } else {
            "HOT_PATH_DAEMON_ADMISSION_POLICY_WATCH"
        },
        blocker: if promotion_review_candidate_allowed {
            "none"
        } else {
            promotion_contract_blocker
        },
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "shadow-only daemon admission policy artifact: verifier-bound hot-path evidence may enter daemon admission review, but this artifact does not mutate registry, write a serving profile, promote product runtime, enable local_accept, or allow market money claims",
    };
    let report = PhaseStreamHotPathBenchmarkReport {
        report_kind: "phase_stream_hot_path_benchmark_v1",
        runtime_source: "mutable_live_store_in_memory_frozen_hot_snapshot",
        input_trace_paths: input_trace_paths.clone(),
        promotion_review_report_path: promotion_review_report_path.display().to_string(),
        daemon_admission_policy_report_path: daemon_admission_policy_report_path
            .display()
            .to_string(),
        total_rows,
        parsed_rows,
        skipped_no_verifier_label,
        skipped_no_safe_atoms,
        route_index_missing_events,
        cells,
        min_bucket_events,
        timed_score_iterations_requested,
        timed_score_iterations,
        prepared_unique_rows: prepared_rows.len(),
        online_bucket_count: summary.bucket_count,
        candidate_bucket_count: summary.candidate_bucket_count,
        rejected_bucket_count: summary.rejected_bucket_count,
        direct_hot_snapshot_capacity: direct_hot_snapshots.capacity(),
        direct_hot_snapshot_captured_count: direct_hot_snapshots.captured_count(),
        direct_hot_snapshot_count: direct_hot_snapshots.retained_count(),
        direct_hot_snapshot_evicted_count: direct_hot_snapshots.evicted_count(),
        selected_direct_hot_snapshot_index: selected_snapshot_eval.snapshot_index,
        selected_direct_hot_snapshot_frozen_after_parsed_rows: selected_snapshot_eval
            .frozen_after_parsed_rows,
        validation_score_event_target,
        selected_validation_score_events: selected_snapshot_eval.validation_score_events,
        selected_validation_score_candidate_events: selected_snapshot_eval
            .validation_eval
            .score_candidate_events,
        selected_validation_unique_cpu_accepts_over_exact_cache: selected_snapshot_eval
            .validation_eval
            .unique_cpu_accepts_over_exact_cache,
        selected_validation_tokens_saved: selected_snapshot_eval.validation_eval.tokens_saved,
        selected_validation_cost_saved_microusd: selected_snapshot_eval
            .validation_eval
            .cost_saved_microusd,
        selected_validation_false_accepts: selected_snapshot_eval.validation_eval.false_accepts,
        future_eval_start_after_parsed_rows: selected_snapshot_eval
            .future_eval_start_after_parsed_rows,
        future_shadow_split_used,
        future_total_tokens: denominator.total_tokens,
        future_total_cost_microusd: denominator.total_cost_microusd,
        future_exact_cache_hits: denominator.exact_cache_hits,
        future_exact_cache_tokens: denominator.exact_cache_tokens,
        future_exact_cache_cost_microusd: denominator.exact_cache_cost_microusd,
        future_non_exact_rows: denominator.non_exact_rows,
        hot_profile_count: hot_runtime.profile_count(),
        hot_route_count: route_table.route_count(),
        hot_route_profile_edges: route_table.profile_edge_count(),
        hot_bytes_estimate: hot_runtime
            .bytes_estimate()
            .saturating_add(route_table.bytes_estimate()),
        hot_scratch_bytes_estimate,
        runtime_budget,
        cold_adapter_json_used: true,
        cold_adapter_strings_used: true,
        direct_mutable_store_export_used: true,
        clean_manifest_loaded: false,
        package_roundtrip_used: false,
        worker_thread_used: false,
        bounded_sync_channel_used: false,
        batch_messages_used: false,
        hot_path_direct_runtime_used: true,
        hot_loop_json_used: false,
        hot_loop_string_route_used: false,
        hot_loop_btreemap_used: false,
        hot_loop_file_io_used: false,
        hot_loop_package_compile_used: false,
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
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        nando_calls_saved_milli: live_store_per_thousand(
            eval.unique_cpu_accepts_over_exact_cache,
            prepared_rows.len(),
        ),
        nando_tokens_saved_milli: live_store_per_thousand_u64(
            eval.tokens_saved,
            denominator.total_tokens,
        ),
        nando_cost_saved_milli: live_store_per_thousand_u64(
            eval.cost_saved_microusd,
            denominator.total_cost_microusd,
        ),
        p5_evidence_ready_for_promotion_audit,
        verifier_binding_bound: verifier_binding.is_bound(),
        promotion_contract_evaluated: true,
        promotion_contract_eligible: promotion_contract.eligible,
        promotion_contract_blocker,
        promotion_review_ready_not_promoted: promotion_review_candidate_allowed,
        product_promotion_enabled: false,
        hot_path_p50_latency_ns: live_store_latency_percentile(&latencies, 50),
        hot_path_p90_latency_ns: live_store_latency_percentile(&latencies, 90),
        hot_path_p99_latency_ns: hot_path_p99,
        hot_path_max_latency_ns: latencies.last().copied().unwrap_or(0),
        hot_path_margin_checksum_micro,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        verdict: if blocker == "none" {
            "HOT_PATH_BENCHMARK_PASS"
        } else {
            "HOT_PATH_BENCHMARK_WATCH"
        },
        blocker,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "direct PhaseCenterHotRuntime benchmark: cold adapter builds bounded verifier-bound snapshot and prepared vectors outside timing; timed hot path is route_index + phase_vector + scratch -> margin only, with no JSON/String/BTreeMap/file IO/package compile, no product local_accept, and no market claim",
    };
    super::write_json_file(&promotion_review_report_path, &promotion_review)?;
    super::write_json_file(&daemon_admission_policy_report_path, &daemon_policy)?;
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_hot_path_benchmark_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  promotion_review_report_path: {}",
        promotion_review_report_path.display()
    );
    println!(
        "  daemon_admission_policy_report_path: {}",
        daemon_admission_policy_report_path.display()
    );
    println!("  prepared_unique_rows: {}", report.prepared_unique_rows);
    println!(
        "  timed_score_iterations: {}",
        report.timed_score_iterations
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!(
        "  hot_path_p99_latency_ns: {}",
        report.hot_path_p99_latency_ns
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_hot_path_daemon_admission_policy_smoke_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let daemon_policy_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOT_PATH_DAEMON_ADMISSION_POLICY_REPORT));
    let smoke_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOT_PATH_DAEMON_ADMISSION_POLICY_SMOKE_REPORT));

    let policy = super::read_json_value(&daemon_policy_report_path)?;
    let report_kind = super::json_string(&policy, &["report_kind"]).unwrap_or_default();
    let verdict = super::json_string(&policy, &["verdict"]).unwrap_or_default();
    let blocker = super::json_string(&policy, &["blocker"]).unwrap_or_default();
    let mode = super::json_string(&policy, &["mode"]).unwrap_or_default();
    let admission_policy_kind =
        super::json_string(&policy, &["admission_policy_kind"]).unwrap_or_default();
    let source_benchmark_report_path =
        super::json_string(&policy, &["source_benchmark_report_path"]).unwrap_or_default();
    let source_promotion_review_report_path =
        super::json_string(&policy, &["source_promotion_review_report_path"]).unwrap_or_default();
    let hot_route_ids = live_store_json_u32_vec(&policy, &["hot_route_ids"]);
    let hot_profile_ids = live_store_json_u32_vec(&policy, &["hot_profile_ids"]);
    let hot_profile_count =
        super::json_u64(&policy, &["hot_profile_count"]).unwrap_or_default() as usize;
    let hot_route_count =
        super::json_u64(&policy, &["hot_route_count"]).unwrap_or_default() as usize;
    let hot_route_profile_edges =
        super::json_u64(&policy, &["hot_route_profile_edges"]).unwrap_or_default() as usize;
    let hot_bytes_estimate =
        super::json_u64(&policy, &["hot_bytes_estimate"]).unwrap_or_default() as usize;
    let future_shadow_split_used =
        super::json_bool(&policy, &["future_shadow_split_used"]).unwrap_or(false);
    let verifier_binding_bound =
        super::json_bool(&policy, &["verifier_binding_bound"]).unwrap_or(false);
    let exact_cache_overlap_excluded =
        super::json_bool(&policy, &["exact_cache_overlap_excluded"]).unwrap_or(false);
    let token_cost_denominator_present =
        super::json_bool(&policy, &["token_cost_denominator_present"]).unwrap_or(false);
    let unique_cpu_accepts_over_exact_cache =
        super::json_u64(&policy, &["unique_cpu_accepts_over_exact_cache"]).unwrap_or_default()
            as usize;
    let tokens_saved = super::json_u64(&policy, &["tokens_saved"]).unwrap_or_default();
    let cost_saved_microusd =
        super::json_u64(&policy, &["cost_saved_microusd"]).unwrap_or_default();
    let false_accepts =
        super::json_u64(&policy, &["false_accepts"]).unwrap_or(usize::MAX as u64) as usize;
    let runtime_margin_parity_mismatches = super::json_u64(
        &policy,
        &["runtime_margin_parity_mismatches"],
    )
    .unwrap_or(usize::MAX as u64) as usize;
    let runtime_decision_parity_mismatches =
        super::json_u64(&policy, &["runtime_decision_parity_mismatches"])
            .unwrap_or(usize::MAX as u64) as usize;
    let promotion_review_candidate_allowed =
        super::json_bool(&policy, &["promotion_review_candidate_allowed"]).unwrap_or(false);
    let shadow_only_daemon_admission_review_allowed =
        super::json_bool(&policy, &["shadow_only_daemon_admission_review_allowed"])
            .unwrap_or(false);
    let admission_policy_candidate_allowed =
        super::json_bool(&policy, &["admission_policy_candidate_allowed"]).unwrap_or(false);
    let registry_mutation_enabled =
        super::json_bool(&policy, &["registry_mutation_enabled"]).unwrap_or(true);
    let cpu_profile_registry_write_enabled =
        super::json_bool(&policy, &["cpu_profile_registry_write_enabled"]).unwrap_or(true);
    let serving_profile_artifact_written =
        super::json_bool(&policy, &["serving_profile_artifact_written"]).unwrap_or(true);
    let product_promotion_enabled =
        super::json_bool(&policy, &["product_promotion_enabled"]).unwrap_or(true);
    let local_accept_enabled = super::json_bool(&policy, &["local_accept_enabled"]).unwrap_or(true);
    let market_money_claim_allowed =
        super::json_bool(&policy, &["market_money_claim_allowed"]).unwrap_or(true);
    let forbidden_flags = live_store_forbidden_flags_from_json(&policy);
    let forbidden_flags_clear = !forbidden_flags.target_id_used
        && !forbidden_flags.proof_rule_id_authority_used
        && !forbidden_flags.concrete_x_lookup_used
        && !forbidden_flags.manual_local_out_t_used
        && !forbidden_flags.hidden_frame_id_or_bind_x_used
        && !forbidden_flags.legacy_backend_used;

    let guard = PhaseStreamHotPathDaemonAdmissionPolicySmokeGuard {
        report_kind_matches: report_kind == "phase_stream_hot_path_daemon_admission_policy_v1",
        verdict_ready: verdict == "HOT_PATH_DAEMON_ADMISSION_POLICY_READY",
        blocker_none: blocker == "none",
        mode_shadow_only_no_runtime_mutation: mode
            == "shadow_only_daemon_admission_policy_no_runtime_mutation",
        admission_policy_kind_matches: admission_policy_kind
            == "hot_path_promotion_review_to_daemon_admission_candidate_v1",
        has_hot_route_ids: !hot_route_ids.is_empty(),
        has_hot_profile_ids: !hot_profile_ids.is_empty(),
        future_shadow_split_used,
        verifier_binding_bound,
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        unique_cpu_accepts_positive: unique_cpu_accepts_over_exact_cache > 0,
        tokens_saved_positive: tokens_saved > 0,
        cost_saved_positive: cost_saved_microusd > 0,
        false_accepts_zero: false_accepts == 0,
        runtime_margin_parity_zero: runtime_margin_parity_mismatches == 0,
        runtime_decision_parity_zero: runtime_decision_parity_mismatches == 0,
        promotion_review_candidate_allowed,
        shadow_only_daemon_admission_review_allowed,
        admission_policy_candidate_allowed,
        registry_mutation_disabled: !registry_mutation_enabled,
        cpu_profile_registry_write_disabled: !cpu_profile_registry_write_enabled,
        serving_profile_artifact_not_written: !serving_profile_artifact_written,
        product_promotion_disabled: !product_promotion_enabled,
        local_accept_disabled: !local_accept_enabled,
        market_money_claim_disabled: !market_money_claim_allowed,
        forbidden_flags_clear,
    };
    let would_stage_for_daemon_shadow_only = guard.report_kind_matches
        && guard.verdict_ready
        && guard.blocker_none
        && guard.mode_shadow_only_no_runtime_mutation
        && guard.admission_policy_kind_matches
        && guard.has_hot_route_ids
        && guard.has_hot_profile_ids
        && guard.future_shadow_split_used
        && guard.verifier_binding_bound
        && guard.exact_cache_overlap_excluded
        && guard.token_cost_denominator_present
        && guard.unique_cpu_accepts_positive
        && guard.tokens_saved_positive
        && guard.cost_saved_positive
        && guard.false_accepts_zero
        && guard.runtime_margin_parity_zero
        && guard.runtime_decision_parity_zero
        && guard.promotion_review_candidate_allowed
        && guard.shadow_only_daemon_admission_review_allowed
        && guard.admission_policy_candidate_allowed
        && guard.registry_mutation_disabled
        && guard.cpu_profile_registry_write_disabled
        && guard.serving_profile_artifact_not_written
        && guard.product_promotion_disabled
        && guard.local_accept_disabled
        && guard.market_money_claim_disabled
        && guard.forbidden_flags_clear;
    let rejection_reason = if would_stage_for_daemon_shadow_only {
        "accepted_for_daemon_shadow_only_stage_runtime_accept_still_disabled".to_owned()
    } else if !guard.report_kind_matches {
        "daemon_policy_report_kind_mismatch".to_owned()
    } else if !guard.verdict_ready || !guard.blocker_none {
        "daemon_policy_not_ready".to_owned()
    } else if !guard.mode_shadow_only_no_runtime_mutation {
        "daemon_policy_mode_not_shadow_only".to_owned()
    } else if !guard.admission_policy_kind_matches {
        "daemon_policy_kind_mismatch".to_owned()
    } else if !guard.has_hot_route_ids || !guard.has_hot_profile_ids {
        "daemon_policy_missing_hot_route_or_profile_ids".to_owned()
    } else if !guard.future_shadow_split_used {
        "future_shadow_split_missing".to_owned()
    } else if !guard.verifier_binding_bound {
        "verifier_binding_missing".to_owned()
    } else if !guard.exact_cache_overlap_excluded || !guard.token_cost_denominator_present {
        "denominator_or_exact_cache_overlap_gate_missing".to_owned()
    } else if !guard.unique_cpu_accepts_positive
        || !guard.tokens_saved_positive
        || !guard.cost_saved_positive
    {
        "no_positive_shadow_savings_evidence".to_owned()
    } else if !guard.false_accepts_zero {
        "false_accepts_detected".to_owned()
    } else if !guard.runtime_margin_parity_zero || !guard.runtime_decision_parity_zero {
        "runtime_parity_mismatch".to_owned()
    } else if !guard.promotion_review_candidate_allowed
        || !guard.shadow_only_daemon_admission_review_allowed
        || !guard.admission_policy_candidate_allowed
    {
        "admission_review_not_allowed".to_owned()
    } else if !guard.registry_mutation_disabled
        || !guard.cpu_profile_registry_write_disabled
        || !guard.serving_profile_artifact_not_written
        || !guard.product_promotion_disabled
        || !guard.local_accept_disabled
        || !guard.market_money_claim_disabled
    {
        "daemon_policy_mutates_product_or_runtime_state".to_owned()
    } else if !guard.forbidden_flags_clear {
        "forbidden_flag_detected".to_owned()
    } else {
        "daemon_admission_policy_smoke_gate_failed".to_owned()
    };

    let report = PhaseStreamHotPathDaemonAdmissionPolicySmokeReport {
        report_kind: "phase_stream_hot_path_daemon_admission_policy_smoke_v1",
        mode: "daemon_admission_policy_smoke_no_runtime_mutation",
        source_daemon_admission_policy_report_path: daemon_policy_report_path.display().to_string(),
        source_benchmark_report_path,
        source_promotion_review_report_path,
        admission_policy_kind,
        hot_route_ids,
        hot_profile_ids,
        hot_profile_count,
        hot_route_count,
        hot_route_profile_edges,
        hot_bytes_estimate,
        future_shadow_split_used,
        verifier_binding_bound,
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        unique_cpu_accepts_over_exact_cache,
        tokens_saved,
        cost_saved_microusd,
        false_accepts,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
        promotion_review_candidate_allowed,
        shadow_only_daemon_admission_review_allowed,
        admission_policy_candidate_allowed,
        policy_decision: if would_stage_for_daemon_shadow_only {
            "would_stage_for_daemon_shadow_only"
        } else {
            "reject"
        },
        would_stage_for_daemon_shadow_only,
        registry_mutation_enabled: false,
        cpu_profile_registry_write_enabled: false,
        serving_profile_artifact_written: false,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        forbidden_flags,
        guard,
        verdict: if would_stage_for_daemon_shadow_only {
            "HOT_PATH_DAEMON_ADMISSION_POLICY_SMOKE_PASS"
        } else {
            "HOT_PATH_DAEMON_ADMISSION_POLICY_SMOKE_WATCH"
        },
        blocker: if would_stage_for_daemon_shadow_only {
            "none".to_owned()
        } else {
            rejection_reason.clone()
        },
        boundary: "daemon admission policy smoke only: consumes a hot-path daemon admission policy artifact and may stage it for shadow-only daemon review; it does not mutate registry, write serving profiles, promote runtime, enable local_accept, or allow market money claims",
    };
    super::write_json_file(&smoke_report_path, &report)?;
    println!("phase_stream_hot_path_daemon_admission_policy_smoke_v1:");
    println!("  report_path: {}", smoke_report_path.display());
    println!("  policy_decision: {}", report.policy_decision);
    println!(
        "  would_stage_for_daemon_shadow_only: {}",
        report.would_stage_for_daemon_shadow_only
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_hot_path_daemon_shadow_gate_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let policy_smoke_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOT_PATH_DAEMON_ADMISSION_POLICY_SMOKE_REPORT));
    let shadow_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOT_PATH_DAEMON_SHADOW_GATE_REPORT));
    let decision_log_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOT_PATH_DAEMON_SHADOW_DECISION_LOG));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_CELLS);
    let min_bucket_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid min_bucket_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    if min_bucket_events == 0 {
        return Err("min_bucket_events must be > 0".to_owned());
    }

    let policy_smoke = super::read_json_value(&policy_smoke_report_path)?;
    let staged_hot_route_ids = live_store_json_u32_vec(&policy_smoke, &["hot_route_ids"]);
    let staged_hot_profile_ids = live_store_json_u32_vec(&policy_smoke, &["hot_profile_ids"]);
    let policy_smoke_guard_passed = super::json_string(&policy_smoke, &["report_kind"])
        .is_some_and(|value| value == "phase_stream_hot_path_daemon_admission_policy_smoke_v1")
        && super::json_string(&policy_smoke, &["verdict"])
            .is_some_and(|value| value == "HOT_PATH_DAEMON_ADMISSION_POLICY_SMOKE_PASS")
        && super::json_string(&policy_smoke, &["blocker"]).is_some_and(|value| value == "none")
        && super::json_bool(&policy_smoke, &["would_stage_for_daemon_shadow_only"])
            .unwrap_or(false)
        && !super::json_bool(&policy_smoke, &["registry_mutation_enabled"]).unwrap_or(true)
        && !super::json_bool(&policy_smoke, &["cpu_profile_registry_write_enabled"])
            .unwrap_or(true)
        && !super::json_bool(&policy_smoke, &["serving_profile_artifact_written"]).unwrap_or(true)
        && !super::json_bool(&policy_smoke, &["product_promotion_enabled"]).unwrap_or(true)
        && !super::json_bool(&policy_smoke, &["local_accept_enabled"]).unwrap_or(true)
        && !super::json_bool(&policy_smoke, &["market_money_claim_allowed"]).unwrap_or(true)
        && super::json_bool(&policy_smoke, &["guard", "forbidden_flags_clear"]).unwrap_or(false);
    let would_stage_for_daemon_shadow_only =
        super::json_bool(&policy_smoke, &["would_stage_for_daemon_shadow_only"]).unwrap_or(false);
    let forbidden_flags = live_store_forbidden_flags_from_json(&policy_smoke);
    let input_trace_paths = trace_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

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
                DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_HOT_PROFILES_PER_WORKER,
            max_hot_bytes_per_worker: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
            max_warm_profiles_per_process:
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
            max_profiles_per_route: DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_PROFILES_PER_ROUTE,
            max_route_top_k: DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_ROUTE_TOP_K,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .map_err(|error| format!("failed to create hot-path daemon shadow store: {error:?}"))?;
    let mut observe_encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create daemon-shadow observe encoder: {error:?}"))?;
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut parsed_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut total_rows = 0usize;
    let mut parsed_rows = 0usize;
    let mut skipped_no_verifier_label = 0usize;
    let mut skipped_no_safe_atoms = 0usize;
    let mut direct_hot_snapshots =
        LiveStoreDirectHotSnapshotBank::new(DEFAULT_LIVE_STORE_DIRECT_HOT_SNAPSHOT_CAPACITY);

    for trace_path in &trace_paths {
        if trace_path == Path::new("-") {
            let stdin = io::stdin();
            let reader = stdin.lock();
            live_store_collect_direct_store_events(
                "<stdin>",
                reader,
                &mut store,
                &mut observe_encoder,
                &mut bucket_policy,
                &mut exact_cache_keys_seen,
                &mut parsed_events,
                &mut total_rows,
                &mut parsed_rows,
                &mut skipped_no_verifier_label,
                &mut skipped_no_safe_atoms,
                &mut direct_hot_snapshots,
            )?;
        } else {
            let file = File::open(trace_path).map_err(|error| {
                format!("failed to open trace '{}': {error}", trace_path.display())
            })?;
            let reader = io::BufReader::new(file);
            live_store_collect_direct_store_events(
                &trace_path.display().to_string(),
                reader,
                &mut store,
                &mut observe_encoder,
                &mut bucket_policy,
                &mut exact_cache_keys_seen,
                &mut parsed_events,
                &mut total_rows,
                &mut parsed_rows,
                &mut skipped_no_verifier_label,
                &mut skipped_no_safe_atoms,
                &mut direct_hot_snapshots,
            )?;
        }
    }

    let summary = store.summary();
    let selected_snapshot_eval =
        live_store_select_direct_hot_snapshot(&direct_hot_snapshots, &parsed_events, cells)?;
    let mut observed_hot_route_ids = Vec::new();
    let mut observed_hot_profile_ids = Vec::new();
    let mut route_index_missing_events = 0usize;
    let mut eval = LiveStorePreparedHotPackEval::default();
    let mut denominator = LiveStoreHotPathDenominator::default();
    let mut runtime_margin_parity_checks = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut runtime_decision_parity_mismatches = 0usize;
    let mut selected_direct_hot_snapshot_index = 0usize;
    let mut selected_direct_hot_snapshot_frozen_after_parsed_rows = 0usize;
    let mut future_eval_start_after_parsed_rows = 0usize;
    let mut future_shadow_split_used = false;

    if let Some(snapshot_eval) = selected_snapshot_eval.as_ref() {
        selected_direct_hot_snapshot_index = snapshot_eval.snapshot_index;
        selected_direct_hot_snapshot_frozen_after_parsed_rows =
            snapshot_eval.frozen_after_parsed_rows;
        future_eval_start_after_parsed_rows = snapshot_eval.future_eval_start_after_parsed_rows;
        future_shadow_split_used = snapshot_eval.future_eval_start_after_parsed_rows
            > snapshot_eval.frozen_after_parsed_rows;
        let selected_snapshot = direct_hot_snapshots
            .get(snapshot_eval.snapshot_index)
            .ok_or_else(|| "selected daemon-shadow snapshot index missing".to_owned())?;
        observed_hot_route_ids = live_store_hot_route_ids(&selected_snapshot.route_table);
        observed_hot_profile_ids = live_store_hot_profile_ids(&selected_snapshot.hot_runtime);
        let future_eval_events = parsed_events
            .get(snapshot_eval.future_eval_start_after_parsed_rows..)
            .unwrap_or(&[]);
        let mut prepare_encoder = PhaseCenterHotRowPreparer::new(cells)
            .map_err(|error| format!("failed to build daemon-shadow row preparer: {error:?}"))?;
        let prepared_rows = live_store_prepare_parsed_events_for_hot_path(
            future_eval_events,
            &selected_snapshot.route_table,
            &mut prepare_encoder,
            &mut route_index_missing_events,
        )?;
        denominator = live_store_hot_path_denominator(&prepared_rows);
        (
            runtime_margin_parity_checks,
            runtime_margin_parity_mismatches,
            runtime_decision_parity_mismatches,
        ) = live_store_hot_path_runtime_parity(
            &selected_snapshot.hot_runtime,
            &selected_snapshot.route_table,
            &prepared_rows,
            cells,
        )?;
        let mut scratch = PhaseCenterHotScratch::new(
            cells,
            selected_snapshot.route_table.profile_edge_count().max(1),
        )
        .map_err(|error| format!("failed to create daemon-shadow scratch: {error:?}"))?;
        if let Some(parent) = decision_log_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "failed to create daemon-shadow decision log dir '{}': {error}",
                    parent.display()
                )
            })?;
        }
        let mut decision_log =
            io::BufWriter::new(File::create(&decision_log_path).map_err(|error| {
                format!(
                    "failed to create daemon-shadow decision log '{}': {error}",
                    decision_log_path.display()
                )
            })?);
        for (row_index, row) in prepared_rows.iter().enumerate() {
            let decisions = selected_snapshot
                .hot_runtime
                .score_prepared_hot_request_candidates(
                    &selected_snapshot.route_table,
                    PhaseCenterPreparedHotRequest::new(row.route_index, &row.phase_vector),
                    &mut scratch,
                )
                .map_err(|error| format!("failed daemon-shadow hot score: {error:?}"))?;
            live_store_update_memory_hot_worker_eval(row, decisions, &mut eval);
            let route_id = selected_snapshot
                .route_table
                .route_id_at(row.route_index)
                .unwrap_or_default();
            let decision_rows = decisions
                .iter()
                .map(|decision| {
                    serde_json::json!({
                        "profile_id": decision.profile_id,
                        "margin_micro": decision.margin_micro,
                        "score_candidate": decision.score_candidate,
                        "verifier_required": decision.verifier_required,
                        "local_accept": decision.local_accept
                    })
                })
                .collect::<Vec<_>>();
            let line = serde_json::json!({
                "row_index": row_index,
                "route_index": row.route_index,
                "route_id": route_id,
                "verified_safe_accept": row.verified_safe_accept,
                "exact_cache_hit": row.exact_cache_hit,
                "tokens": row.tokens,
                "cost_microusd": row.cost_microusd,
                "decisions": decision_rows
            });
            serde_json::to_writer(&mut decision_log, &line).map_err(|error| {
                format!(
                    "failed to write daemon-shadow decision '{}': {error}",
                    decision_log_path.display()
                )
            })?;
            decision_log
                .write_all(b"\n")
                .map_err(|error| format!("failed daemon-shadow decision newline: {error}"))?;
        }
        decision_log
            .flush()
            .map_err(|error| format!("failed to flush daemon-shadow decision log: {error}"))?;
    } else {
        if let Some(parent) = decision_log_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "failed to create daemon-shadow decision log dir '{}': {error}",
                    parent.display()
                )
            })?;
        }
        File::create(&decision_log_path).map_err(|error| {
            format!(
                "failed to create empty daemon-shadow decision log '{}': {error}",
                decision_log_path.display()
            )
        })?;
    }

    let staged_hot_route_ids_match = staged_hot_route_ids == observed_hot_route_ids;
    let staged_hot_profile_ids_match = staged_hot_profile_ids == observed_hot_profile_ids;
    let exact_cache_overlap_excluded = denominator.non_exact_rows > 0
        && eval.unique_cpu_accepts_over_exact_cache <= denominator.non_exact_rows;
    let token_cost_denominator_present =
        denominator.total_tokens > 0 && denominator.total_cost_microusd > 0;
    let gate_passed = policy_smoke_guard_passed
        && would_stage_for_daemon_shadow_only
        && selected_snapshot_eval.is_some()
        && future_shadow_split_used
        && staged_hot_route_ids_match
        && staged_hot_profile_ids_match
        && eval.score_events > 0
        && eval.unique_cpu_accepts_over_exact_cache > 0
        && eval.tokens_saved > 0
        && eval.cost_saved_microusd > 0
        && eval.false_accepts == 0
        && eval.local_accept_events == 0
        && runtime_margin_parity_mismatches == 0
        && runtime_decision_parity_mismatches == 0
        && exact_cache_overlap_excluded
        && token_cost_denominator_present;
    let blocker = if gate_passed {
        "none".to_owned()
    } else if !policy_smoke_guard_passed || !would_stage_for_daemon_shadow_only {
        "policy_smoke_not_shadow_stageable".to_owned()
    } else if selected_snapshot_eval.is_none() {
        "daemon_shadow_no_safe_frozen_snapshot".to_owned()
    } else if !future_shadow_split_used {
        "daemon_shadow_future_split_missing".to_owned()
    } else if !staged_hot_route_ids_match || !staged_hot_profile_ids_match {
        "daemon_shadow_staged_ids_mismatch".to_owned()
    } else if eval.false_accepts != 0 {
        "daemon_shadow_false_accepts_nonzero".to_owned()
    } else if eval.local_accept_events != 0 {
        "daemon_shadow_local_accept_enabled".to_owned()
    } else if eval.unique_cpu_accepts_over_exact_cache == 0 {
        "daemon_shadow_unique_accepts_zero".to_owned()
    } else if runtime_margin_parity_mismatches != 0 || runtime_decision_parity_mismatches != 0 {
        "daemon_shadow_runtime_parity_mismatch".to_owned()
    } else if !exact_cache_overlap_excluded || !token_cost_denominator_present {
        "daemon_shadow_denominator_gate_missing".to_owned()
    } else {
        "daemon_shadow_gate_failed".to_owned()
    };

    let report = PhaseStreamHotPathDaemonShadowGateReport {
        report_kind: "phase_stream_hot_path_daemon_shadow_gate_v1",
        mode: "daemon_shadow_gate_no_runtime_mutation",
        source_policy_smoke_report_path: policy_smoke_report_path.display().to_string(),
        decision_log_path: decision_log_path.display().to_string(),
        input_trace_paths,
        cells,
        min_bucket_events,
        total_rows,
        parsed_rows,
        skipped_no_verifier_label,
        skipped_no_safe_atoms,
        online_bucket_count: summary.bucket_count,
        candidate_bucket_count: summary.candidate_bucket_count,
        rejected_bucket_count: summary.rejected_bucket_count,
        direct_hot_snapshot_capacity: direct_hot_snapshots.capacity(),
        direct_hot_snapshot_captured_count: direct_hot_snapshots.captured_count(),
        direct_hot_snapshot_count: direct_hot_snapshots.retained_count(),
        direct_hot_snapshot_evicted_count: direct_hot_snapshots.evicted_count(),
        selected_direct_hot_snapshot_index,
        selected_direct_hot_snapshot_frozen_after_parsed_rows,
        future_eval_start_after_parsed_rows,
        future_shadow_split_used,
        staged_hot_route_ids,
        staged_hot_profile_ids,
        observed_hot_route_ids,
        observed_hot_profile_ids,
        staged_hot_route_ids_match,
        staged_hot_profile_ids_match,
        route_index_missing_events,
        score_events: eval.score_events,
        score_candidate_events: eval.score_candidate_events,
        verifier_required_events: eval.verifier_required_events,
        local_accept_events: eval.local_accept_events,
        unique_cpu_accepts_over_exact_cache: eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: eval.tokens_saved,
        cost_saved_microusd: eval.cost_saved_microusd,
        false_accepts: eval.false_accepts,
        future_total_tokens: denominator.total_tokens,
        future_total_cost_microusd: denominator.total_cost_microusd,
        future_exact_cache_hits: denominator.exact_cache_hits,
        future_exact_cache_tokens: denominator.exact_cache_tokens,
        future_exact_cache_cost_microusd: denominator.exact_cache_cost_microusd,
        future_non_exact_rows: denominator.non_exact_rows,
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        runtime_margin_parity_checks,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
        policy_smoke_guard_passed,
        would_stage_for_daemon_shadow_only,
        registry_mutation_enabled: false,
        cpu_profile_registry_write_enabled: false,
        serving_profile_artifact_written: false,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        forbidden_flags,
        verdict: if gate_passed {
            "HOT_PATH_DAEMON_SHADOW_GATE_PASS"
        } else {
            "HOT_PATH_DAEMON_SHADOW_GATE_WATCH"
        },
        blocker,
        boundary: "daemon shadow gate only: consumes a hot-path admission policy smoke report, rebuilds a bounded verifier-bound hot snapshot from trace, writes shadow decisions, and does not mutate registry, write serving profiles, promote runtime, enable local_accept, or allow market money claims",
    };
    super::write_json_file(&shadow_report_path, &report)?;
    println!("phase_stream_hot_path_daemon_shadow_gate_v1:");
    println!("  report_path: {}", shadow_report_path.display());
    println!("  decision_log_path: {}", decision_log_path.display());
    println!("  score_events: {}", report.score_events);
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_hot_path_daemon_append_shadow_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let policy_smoke_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOT_PATH_DAEMON_ADMISSION_POLICY_SMOKE_REPORT));
    let append_shadow_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOT_PATH_DAEMON_APPEND_SHADOW_GATE_REPORT));
    let decision_log_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOT_PATH_DAEMON_APPEND_SHADOW_DECISION_LOG));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_CELLS);
    let min_bucket_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid min_bucket_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    let watermark_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL));
    let append_trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(
                super::DEFAULT_CODEX_SESSION_TOOL_STATUS_APPEND_LATEST_JSONL,
            )]
        } else {
            rest
        }
    };
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    if min_bucket_events == 0 {
        return Err("min_bucket_events must be > 0".to_owned());
    }

    let policy_smoke = super::read_json_value(&policy_smoke_report_path)?;
    let staged_hot_route_ids = live_store_json_u32_vec(&policy_smoke, &["hot_route_ids"]);
    let staged_hot_profile_ids = live_store_json_u32_vec(&policy_smoke, &["hot_profile_ids"]);
    let policy_smoke_guard_passed = super::json_string(&policy_smoke, &["report_kind"])
        .is_some_and(|value| value == "phase_stream_hot_path_daemon_admission_policy_smoke_v1")
        && super::json_string(&policy_smoke, &["verdict"])
            .is_some_and(|value| value == "HOT_PATH_DAEMON_ADMISSION_POLICY_SMOKE_PASS")
        && super::json_string(&policy_smoke, &["blocker"]).is_some_and(|value| value == "none")
        && super::json_bool(&policy_smoke, &["would_stage_for_daemon_shadow_only"])
            .unwrap_or(false)
        && !super::json_bool(&policy_smoke, &["registry_mutation_enabled"]).unwrap_or(true)
        && !super::json_bool(&policy_smoke, &["cpu_profile_registry_write_enabled"])
            .unwrap_or(true)
        && !super::json_bool(&policy_smoke, &["serving_profile_artifact_written"]).unwrap_or(true)
        && !super::json_bool(&policy_smoke, &["product_promotion_enabled"]).unwrap_or(true)
        && !super::json_bool(&policy_smoke, &["local_accept_enabled"]).unwrap_or(true)
        && !super::json_bool(&policy_smoke, &["market_money_claim_allowed"]).unwrap_or(true)
        && super::json_bool(&policy_smoke, &["guard", "forbidden_flags_clear"]).unwrap_or(false);
    let would_stage_for_daemon_shadow_only =
        super::json_bool(&policy_smoke, &["would_stage_for_daemon_shadow_only"]).unwrap_or(false);
    let forbidden_flags = live_store_forbidden_flags_from_json(&policy_smoke);
    let watermark_trace_paths = vec![watermark_trace_path.display().to_string()];
    let append_trace_path_strings = append_trace_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

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
                DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_HOT_PROFILES_PER_WORKER,
            max_hot_bytes_per_worker: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
            max_warm_profiles_per_process:
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
            max_profiles_per_route: DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_PROFILES_PER_ROUTE,
            max_route_top_k: DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_ROUTE_TOP_K,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .map_err(|error| format!("failed to create append-shadow watermark store: {error:?}"))?;
    let mut observe_encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create append-shadow observe encoder: {error:?}"))?;
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut watermark_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut watermark_total_rows = 0usize;
    let mut watermark_parsed_rows = 0usize;
    let mut watermark_skipped_no_verifier_label = 0usize;
    let mut watermark_skipped_no_safe_atoms = 0usize;
    let mut direct_hot_snapshots =
        LiveStoreDirectHotSnapshotBank::new(DEFAULT_LIVE_STORE_DIRECT_HOT_SNAPSHOT_CAPACITY);

    if watermark_trace_path == Path::new("-") {
        let stdin = io::stdin();
        let reader = stdin.lock();
        live_store_collect_direct_store_events(
            "<stdin-watermark>",
            reader,
            &mut store,
            &mut observe_encoder,
            &mut bucket_policy,
            &mut exact_cache_keys_seen,
            &mut watermark_events,
            &mut watermark_total_rows,
            &mut watermark_parsed_rows,
            &mut watermark_skipped_no_verifier_label,
            &mut watermark_skipped_no_safe_atoms,
            &mut direct_hot_snapshots,
        )?;
    } else {
        let file = File::open(&watermark_trace_path).map_err(|error| {
            format!(
                "failed to open watermark trace '{}': {error}",
                watermark_trace_path.display()
            )
        })?;
        let reader = io::BufReader::new(file);
        live_store_collect_direct_store_events(
            &watermark_trace_path.display().to_string(),
            reader,
            &mut store,
            &mut observe_encoder,
            &mut bucket_policy,
            &mut exact_cache_keys_seen,
            &mut watermark_events,
            &mut watermark_total_rows,
            &mut watermark_parsed_rows,
            &mut watermark_skipped_no_verifier_label,
            &mut watermark_skipped_no_safe_atoms,
            &mut direct_hot_snapshots,
        )?;
    }

    let summary = store.summary();
    let selected_snapshot_eval =
        live_store_select_direct_hot_snapshot(&direct_hot_snapshots, &watermark_events, cells)?;

    let mut append_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut append_total_rows = 0usize;
    let mut append_parsed_rows = 0usize;
    let mut append_skipped_no_verifier_label = 0usize;
    let mut append_skipped_no_safe_atoms = 0usize;
    for append_trace_path in &append_trace_paths {
        if append_trace_path == Path::new("-") {
            let stdin = io::stdin();
            let reader = stdin.lock();
            live_store_collect_append_shadow_events(
                "<stdin-append>",
                reader,
                &bucket_policy,
                &mut exact_cache_keys_seen,
                &mut append_events,
                &mut append_total_rows,
                &mut append_parsed_rows,
                &mut append_skipped_no_verifier_label,
                &mut append_skipped_no_safe_atoms,
            )?;
        } else {
            let file = File::open(append_trace_path).map_err(|error| {
                format!(
                    "failed to open append trace '{}': {error}",
                    append_trace_path.display()
                )
            })?;
            let reader = io::BufReader::new(file);
            live_store_collect_append_shadow_events(
                &append_trace_path.display().to_string(),
                reader,
                &bucket_policy,
                &mut exact_cache_keys_seen,
                &mut append_events,
                &mut append_total_rows,
                &mut append_parsed_rows,
                &mut append_skipped_no_verifier_label,
                &mut append_skipped_no_safe_atoms,
            )?;
        }
    }

    let mut observed_hot_route_ids = Vec::new();
    let mut observed_hot_profile_ids = Vec::new();
    let mut append_route_index_missing_events = 0usize;
    let mut eval = LiveStorePreparedHotPackEval::default();
    let mut denominator = LiveStoreHotPathDenominator::default();
    let mut runtime_margin_parity_checks = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut runtime_decision_parity_mismatches = 0usize;
    let mut selected_direct_hot_snapshot_index = 0usize;
    let mut selected_direct_hot_snapshot_frozen_after_parsed_rows = 0usize;
    let mut future_eval_start_after_parsed_rows = 0usize;
    let mut watermark_future_shadow_split_used = false;

    if let Some(snapshot_eval) = selected_snapshot_eval.as_ref() {
        selected_direct_hot_snapshot_index = snapshot_eval.snapshot_index;
        selected_direct_hot_snapshot_frozen_after_parsed_rows =
            snapshot_eval.frozen_after_parsed_rows;
        future_eval_start_after_parsed_rows = snapshot_eval.future_eval_start_after_parsed_rows;
        watermark_future_shadow_split_used = snapshot_eval.future_eval_start_after_parsed_rows
            > snapshot_eval.frozen_after_parsed_rows;
        let selected_snapshot = direct_hot_snapshots
            .get(snapshot_eval.snapshot_index)
            .ok_or_else(|| "selected append-shadow snapshot index missing".to_owned())?;
        observed_hot_route_ids = live_store_hot_route_ids(&selected_snapshot.route_table);
        observed_hot_profile_ids = live_store_hot_profile_ids(&selected_snapshot.hot_runtime);
        let mut prepare_encoder = PhaseCenterHotRowPreparer::new(cells)
            .map_err(|error| format!("failed to build append-shadow row preparer: {error:?}"))?;
        let prepared_rows = live_store_prepare_parsed_events_for_hot_path(
            &append_events,
            &selected_snapshot.route_table,
            &mut prepare_encoder,
            &mut append_route_index_missing_events,
        )?;
        denominator = live_store_hot_path_denominator(&prepared_rows);
        (
            runtime_margin_parity_checks,
            runtime_margin_parity_mismatches,
            runtime_decision_parity_mismatches,
        ) = live_store_hot_path_runtime_parity(
            &selected_snapshot.hot_runtime,
            &selected_snapshot.route_table,
            &prepared_rows,
            cells,
        )?;
        let mut scratch = PhaseCenterHotScratch::new(
            cells,
            selected_snapshot.route_table.profile_edge_count().max(1),
        )
        .map_err(|error| format!("failed to create append-shadow scratch: {error:?}"))?;
        if let Some(parent) = decision_log_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "failed to create append-shadow decision log dir '{}': {error}",
                    parent.display()
                )
            })?;
        }
        let mut decision_log =
            io::BufWriter::new(File::create(&decision_log_path).map_err(|error| {
                format!(
                    "failed to create append-shadow decision log '{}': {error}",
                    decision_log_path.display()
                )
            })?);
        for (row_index, row) in prepared_rows.iter().enumerate() {
            let decisions = selected_snapshot
                .hot_runtime
                .score_prepared_hot_request_candidates(
                    &selected_snapshot.route_table,
                    PhaseCenterPreparedHotRequest::new(row.route_index, &row.phase_vector),
                    &mut scratch,
                )
                .map_err(|error| format!("failed append-shadow hot score: {error:?}"))?;
            live_store_update_memory_hot_worker_eval(row, decisions, &mut eval);
            let route_id = selected_snapshot
                .route_table
                .route_id_at(row.route_index)
                .unwrap_or_default();
            let decision_rows = decisions
                .iter()
                .map(|decision| {
                    serde_json::json!({
                        "profile_id": decision.profile_id,
                        "margin_micro": decision.margin_micro,
                        "score_candidate": decision.score_candidate,
                        "verifier_required": decision.verifier_required,
                        "local_accept": decision.local_accept
                    })
                })
                .collect::<Vec<_>>();
            let line = serde_json::json!({
                "row_index": row_index,
                "route_index": row.route_index,
                "route_id": route_id,
                "verified_safe_accept": row.verified_safe_accept,
                "exact_cache_hit": row.exact_cache_hit,
                "tokens": row.tokens,
                "cost_microusd": row.cost_microusd,
                "decisions": decision_rows
            });
            serde_json::to_writer(&mut decision_log, &line).map_err(|error| {
                format!(
                    "failed to write append-shadow decision '{}': {error}",
                    decision_log_path.display()
                )
            })?;
            decision_log
                .write_all(b"\n")
                .map_err(|error| format!("failed append-shadow decision newline: {error}"))?;
        }
        decision_log
            .flush()
            .map_err(|error| format!("failed to flush append-shadow decision log: {error}"))?;
    } else {
        if let Some(parent) = decision_log_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "failed to create append-shadow decision log dir '{}': {error}",
                    parent.display()
                )
            })?;
        }
        File::create(&decision_log_path).map_err(|error| {
            format!(
                "failed to create empty append-shadow decision log '{}': {error}",
                decision_log_path.display()
            )
        })?;
    }

    let staged_hot_route_ids_match = staged_hot_route_ids == observed_hot_route_ids;
    let staged_hot_profile_ids_match = staged_hot_profile_ids == observed_hot_profile_ids;
    let append_shadow_window_used = append_parsed_rows > 0;
    let exact_cache_overlap_excluded = denominator.non_exact_rows > 0
        && eval.unique_cpu_accepts_over_exact_cache <= denominator.non_exact_rows;
    let token_cost_denominator_present =
        denominator.total_tokens > 0 && denominator.total_cost_microusd > 0;
    let gate_passed = policy_smoke_guard_passed
        && would_stage_for_daemon_shadow_only
        && selected_snapshot_eval.is_some()
        && watermark_future_shadow_split_used
        && append_shadow_window_used
        && staged_hot_route_ids_match
        && staged_hot_profile_ids_match
        && eval.score_events > 0
        && eval.unique_cpu_accepts_over_exact_cache > 0
        && eval.tokens_saved > 0
        && eval.cost_saved_microusd > 0
        && eval.false_accepts == 0
        && eval.local_accept_events == 0
        && runtime_margin_parity_mismatches == 0
        && runtime_decision_parity_mismatches == 0
        && exact_cache_overlap_excluded
        && token_cost_denominator_present;
    let blocker = if gate_passed {
        "none".to_owned()
    } else if !policy_smoke_guard_passed || !would_stage_for_daemon_shadow_only {
        "policy_smoke_not_shadow_stageable".to_owned()
    } else if selected_snapshot_eval.is_none() {
        "append_shadow_no_safe_frozen_snapshot".to_owned()
    } else if !watermark_future_shadow_split_used {
        "append_shadow_watermark_future_split_missing".to_owned()
    } else if !append_shadow_window_used {
        "append_shadow_window_empty".to_owned()
    } else if !staged_hot_route_ids_match || !staged_hot_profile_ids_match {
        "append_shadow_staged_ids_mismatch".to_owned()
    } else if eval.false_accepts != 0 {
        "append_shadow_false_accepts_nonzero".to_owned()
    } else if eval.local_accept_events != 0 {
        "append_shadow_local_accept_enabled".to_owned()
    } else if eval.unique_cpu_accepts_over_exact_cache == 0 {
        "append_shadow_unique_accepts_zero".to_owned()
    } else if runtime_margin_parity_mismatches != 0 || runtime_decision_parity_mismatches != 0 {
        "append_shadow_runtime_parity_mismatch".to_owned()
    } else if !exact_cache_overlap_excluded || !token_cost_denominator_present {
        "append_shadow_denominator_gate_missing".to_owned()
    } else {
        "append_shadow_gate_failed".to_owned()
    };

    let report = PhaseStreamHotPathDaemonAppendShadowGateReport {
        report_kind: "phase_stream_hot_path_daemon_append_shadow_gate_v1",
        mode: "daemon_append_shadow_gate_no_runtime_mutation",
        source_policy_smoke_report_path: policy_smoke_report_path.display().to_string(),
        decision_log_path: decision_log_path.display().to_string(),
        watermark_trace_paths,
        append_trace_paths: append_trace_path_strings,
        cells,
        min_bucket_events,
        watermark_total_rows,
        watermark_parsed_rows,
        watermark_skipped_no_verifier_label,
        watermark_skipped_no_safe_atoms,
        append_total_rows,
        append_parsed_rows,
        append_skipped_no_verifier_label,
        append_skipped_no_safe_atoms,
        online_bucket_count: summary.bucket_count,
        candidate_bucket_count: summary.candidate_bucket_count,
        rejected_bucket_count: summary.rejected_bucket_count,
        direct_hot_snapshot_capacity: direct_hot_snapshots.capacity(),
        direct_hot_snapshot_captured_count: direct_hot_snapshots.captured_count(),
        direct_hot_snapshot_count: direct_hot_snapshots.retained_count(),
        direct_hot_snapshot_evicted_count: direct_hot_snapshots.evicted_count(),
        selected_direct_hot_snapshot_index,
        selected_direct_hot_snapshot_frozen_after_parsed_rows,
        future_eval_start_after_parsed_rows,
        watermark_future_shadow_split_used,
        append_shadow_window_used,
        staged_hot_route_ids,
        staged_hot_profile_ids,
        observed_hot_route_ids,
        observed_hot_profile_ids,
        staged_hot_route_ids_match,
        staged_hot_profile_ids_match,
        append_route_index_missing_events,
        append_score_events: eval.score_events,
        append_score_candidate_events: eval.score_candidate_events,
        verifier_required_events: eval.verifier_required_events,
        local_accept_events: eval.local_accept_events,
        unique_cpu_accepts_over_exact_cache: eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: eval.tokens_saved,
        cost_saved_microusd: eval.cost_saved_microusd,
        false_accepts: eval.false_accepts,
        append_total_tokens: denominator.total_tokens,
        append_total_cost_microusd: denominator.total_cost_microusd,
        append_exact_cache_hits: denominator.exact_cache_hits,
        append_exact_cache_tokens: denominator.exact_cache_tokens,
        append_exact_cache_cost_microusd: denominator.exact_cache_cost_microusd,
        append_non_exact_rows: denominator.non_exact_rows,
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        runtime_margin_parity_checks,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
        policy_smoke_guard_passed,
        would_stage_for_daemon_shadow_only,
        registry_mutation_enabled: false,
        cpu_profile_registry_write_enabled: false,
        serving_profile_artifact_written: false,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        forbidden_flags,
        verdict: if gate_passed {
            "HOT_PATH_DAEMON_APPEND_SHADOW_GATE_PASS"
        } else {
            "HOT_PATH_DAEMON_APPEND_SHADOW_GATE_WATCH"
        },
        blocker,
        boundary: "daemon append shadow gate only: watermark trace builds and validates a bounded verifier-bound hot snapshot; append trace is scored through PhaseCenterHotRuntime without mutating registry, writing serving profiles, promoting runtime, enabling local_accept, or allowing market money claims",
    };
    super::write_json_file(&append_shadow_report_path, &report)?;
    println!("phase_stream_hot_path_daemon_append_shadow_gate_v1:");
    println!("  report_path: {}", append_shadow_report_path.display());
    println!("  decision_log_path: {}", decision_log_path.display());
    println!("  append_score_events: {}", report.append_score_events);
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_hot_path_daemon_live_loop_budget_smoke_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOT_PATH_DAEMON_LIVE_LOOP_BUDGET_SMOKE_REPORT));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_CELLS);
    let min_bucket_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid min_bucket_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    if min_bucket_events == 0 {
        return Err("min_bucket_events must be > 0".to_owned());
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
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_PROFILES_PER_WORKER,
            max_hot_bytes_per_worker: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
            max_warm_profiles_per_process:
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
            max_profiles_per_route: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_PROFILES_PER_ROUTE,
            max_route_top_k: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_ROUTE_TOP_K,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .map_err(|error| format!("failed to create live-loop budget store: {error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create live-loop encoder: {error:?}"))?;
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut parsed_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut total_rows = 0usize;
    let mut parsed_rows = 0usize;
    let mut skipped_no_verifier_label = 0usize;
    let mut skipped_no_safe_atoms = 0usize;

    for trace_path in &trace_paths {
        if trace_path == Path::new("-") {
            let stdin = io::stdin();
            let reader = stdin.lock();
            live_store_observe_live_loop_budget_events(
                "<stdin>",
                reader,
                &mut store,
                &mut encoder,
                &mut bucket_policy,
                &mut exact_cache_keys_seen,
                &mut parsed_events,
                &mut total_rows,
                &mut parsed_rows,
                &mut skipped_no_verifier_label,
                &mut skipped_no_safe_atoms,
            )?;
        } else {
            let file = File::open(trace_path).map_err(|error| {
                format!("failed to open trace '{}': {error}", trace_path.display())
            })?;
            let reader = io::BufReader::new(file);
            live_store_observe_live_loop_budget_events(
                &trace_path.display().to_string(),
                reader,
                &mut store,
                &mut encoder,
                &mut bucket_policy,
                &mut exact_cache_keys_seen,
                &mut parsed_events,
                &mut total_rows,
                &mut parsed_rows,
                &mut skipped_no_verifier_label,
                &mut skipped_no_safe_atoms,
            )?;
        }
    }

    let summary = store.summary();
    let runtime_budget = live_store_budget_report(store.runtime_budget_snapshot());
    let direct_live_hot = live_store_direct_hot_report(&store, &parsed_events, cells)?;
    let (
        hot_runtime_available,
        hot_route_ids,
        hot_profile_ids,
        hot_route_count,
        hot_profile_count,
        hot_route_profile_edges,
    ) = if let Some((hot_runtime, route_table)) = store
        .candidate_hot_runtime_and_route_table()
        .map_err(|error| format!("failed to build live-loop hot view: {error:?}"))?
    {
        (
            true,
            live_store_hot_route_ids(&route_table),
            live_store_hot_profile_ids(&hot_runtime),
            route_table.route_count(),
            hot_runtime.profile_count(),
            route_table.profile_edge_count(),
        )
    } else {
        (false, Vec::new(), Vec::new(), 0, 0, 0)
    };

    let gate_passed = parsed_rows > 0
        && summary.scored_events > 0
        && summary.unique_cpu_accepts_over_exact_cache > 0
        && summary.tokens_saved > 0
        && summary.cost_saved_microusd > 0
        && direct_live_hot.passed
        && direct_live_hot.score_false_label_events == 0
        && direct_live_hot.local_accept_events == 0
        && hot_runtime_available
        && runtime_budget.product_runtime_budget_passed;
    let blocker = if gate_passed {
        "none".to_owned()
    } else if parsed_rows == 0 {
        "live_loop_no_parsed_events".to_owned()
    } else if summary.scored_events == 0 {
        "live_loop_no_score_before_update_events".to_owned()
    } else if summary.unique_cpu_accepts_over_exact_cache == 0 {
        "live_loop_unique_accepts_zero".to_owned()
    } else if summary.tokens_saved == 0 || summary.cost_saved_microusd == 0 {
        "live_loop_token_cost_denominator_missing".to_owned()
    } else if !direct_live_hot.passed || direct_live_hot.score_false_label_events != 0 {
        format!("live_loop_current_hot_view_{}", direct_live_hot.blocker)
    } else if direct_live_hot.local_accept_events != 0 {
        "live_loop_current_hot_view_local_accept_enabled".to_owned()
    } else if !hot_runtime_available {
        "live_loop_no_hot_runtime_view".to_owned()
    } else if !runtime_budget.product_runtime_budget_passed {
        "live_loop_budget_failed".to_owned()
    } else {
        "live_loop_budget_smoke_failed".to_owned()
    };

    let report = PhaseStreamHotPathDaemonLiveLoopBudgetSmokeReport {
        report_kind: "phase_stream_hot_path_daemon_live_loop_budget_smoke_v1",
        mode: "mutable_live_store_online_score_before_update_budget_smoke",
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        cells,
        min_bucket_events,
        total_rows,
        parsed_rows,
        skipped_no_verifier_label,
        skipped_no_safe_atoms,
        online_bucket_count: summary.bucket_count,
        active_bucket_count: summary.active_bucket_count,
        shadow_ready_bucket_count: summary.shadow_ready_bucket_count,
        candidate_bucket_count: summary.candidate_bucket_count,
        rejected_bucket_count: summary.rejected_bucket_count,
        live_route_count: store.route_count(),
        live_route_bucket_count: store.route_bucket_count(),
        scored_events_before_update: summary.scored_events,
        local_operator_shadow_decisions: summary.local_operator_shadow_decisions,
        unique_cpu_accepts_over_exact_cache: summary.unique_cpu_accepts_over_exact_cache,
        tokens_saved: summary.tokens_saved,
        cost_saved_microusd: summary.cost_saved_microusd,
        false_accepts: summary.false_accepts,
        learning_false_accepts_before_rejection: summary.false_accepts,
        current_hot_view_false_label_events: direct_live_hot.score_false_label_events,
        hot_runtime_available,
        hot_route_ids,
        hot_profile_ids,
        hot_route_count,
        hot_profile_count,
        hot_route_profile_edges,
        direct_live_hot,
        runtime_budget,
        budget_snapshot_live: true,
        direct_mutable_store_used: true,
        score_before_update_used: true,
        core_numeric_route_bucket_ids_used: true,
        cold_adapter_json_used: true,
        cold_adapter_strings_used: true,
        quarantine_nwpc_checkpoint_compile_used: false,
        package_roundtrip_used: false,
        registry_mutation_enabled: false,
        cpu_profile_registry_write_enabled: false,
        serving_profile_artifact_written: false,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        verdict: if gate_passed {
            "HOT_PATH_DAEMON_LIVE_LOOP_BUDGET_SMOKE_PASS"
        } else {
            "HOT_PATH_DAEMON_LIVE_LOOP_BUDGET_SMOKE_WATCH"
        },
        blocker,
        boundary: "live-loop budget smoke only: phase-atom JSONL is a cold source adapter, but each parsed event enters PhaseCenterLiveOperatorStore as numeric route_id/bucket_id/atom_ids, is scored before update by the mutable phase-center store, exposes HOT/WARM budgets, and does not compile quarantine .nwpc checkpoints, roundtrip packages, mutate registry, write serving profiles, promote runtime, enable local_accept, or allow market money claims",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_hot_path_daemon_live_loop_budget_smoke_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  scored_events_before_update: {}",
        report.scored_events_before_update
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!(
        "  product_runtime_budget_passed: {}",
        report.runtime_budget.product_runtime_budget_passed
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_hot_path_daemon_append_live_loop_smoke_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_LOOP_SMOKE_REPORT));
    let decision_log_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_LOOP_DECISION_LOG));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_CELLS);
    let min_bucket_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid min_bucket_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    let watermark_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL));
    let append_trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    if min_bucket_events == 0 {
        return Err("min_bucket_events must be > 0".to_owned());
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
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_PROFILES_PER_WORKER,
            max_hot_bytes_per_worker: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
            max_warm_profiles_per_process:
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
            max_profiles_per_route: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_PROFILES_PER_ROUTE,
            max_route_top_k: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_ROUTE_TOP_K,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .map_err(|error| format!("failed to create append live-loop store: {error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create append live-loop encoder: {error:?}"))?;
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut watermark_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut watermark_total_rows = 0usize;
    let mut watermark_parsed_rows = 0usize;
    let mut watermark_skipped_no_verifier_label = 0usize;
    let mut watermark_skipped_no_safe_atoms = 0usize;

    if watermark_trace_path == Path::new("-") {
        let stdin = io::stdin();
        let reader = stdin.lock();
        live_store_observe_live_loop_budget_events(
            "<stdin-watermark>",
            reader,
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
    } else {
        let file = File::open(&watermark_trace_path).map_err(|error| {
            format!(
                "failed to open watermark trace '{}': {error}",
                watermark_trace_path.display()
            )
        })?;
        let reader = io::BufReader::new(file);
        live_store_observe_live_loop_budget_events(
            &watermark_trace_path.display().to_string(),
            reader,
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

    if let Some(parent) = decision_log_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create append live-loop decision dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let mut decision_log =
        io::BufWriter::new(File::create(&decision_log_path).map_err(|error| {
            format!(
                "failed to create append live-loop decision log '{}': {error}",
                decision_log_path.display()
            )
        })?);

    let mut append_total_rows = 0usize;
    let mut append_parsed_rows = 0usize;
    let mut append_skipped_no_verifier_label = 0usize;
    let mut append_skipped_no_safe_atoms = 0usize;
    let mut append_hot_view_available_events = 0usize;
    let mut append_route_index_missing_events = 0usize;
    let mut append_eval = LiveStorePreparedHotPackEval::default();
    let mut append_denominator = LiveStoreHotPathDenominator::default();
    let mut append_row_index = 0usize;

    for append_trace_path in &append_trace_paths {
        let mut reader: Box<dyn BufRead> = if append_trace_path == Path::new("-") {
            Box::new(io::BufReader::new(io::stdin()))
        } else {
            let file = File::open(append_trace_path).map_err(|error| {
                format!(
                    "failed to open append trace '{}': {error}",
                    append_trace_path.display()
                )
            })?;
            Box::new(io::BufReader::new(file))
        };
        let source_label = append_trace_path.display().to_string();
        let mut line = String::new();
        let mut line_index = 0usize;
        loop {
            line.clear();
            let bytes = reader.read_line(&mut line).map_err(|error| {
                format!(
                    "failed to read append live-loop source '{}' line {}: {error}",
                    source_label,
                    line_index + 1
                )
            })?;
            if bytes == 0 {
                break;
            }
            line_index += 1;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            append_total_rows += 1;
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse append live-loop source '{}' line {}: {error}",
                    source_label, line_index
                )
            })?;
            let Some(verified_safe_accept) = row
                .get("verified_safe_accept")
                .and_then(serde_json::Value::as_bool)
            else {
                append_skipped_no_verifier_label += 1;
                continue;
            };
            let Some(adapter_event) = live_store_atom_event_from_row(
                &row,
                verified_safe_accept,
                &bucket_policy,
                &mut exact_cache_keys_seen,
            ) else {
                append_skipped_no_safe_atoms += 1;
                continue;
            };

            append_denominator.total_tokens = append_denominator
                .total_tokens
                .saturating_add(adapter_event.tokens);
            append_denominator.total_cost_microusd = append_denominator
                .total_cost_microusd
                .saturating_add(adapter_event.cost_microusd);
            if adapter_event.exact_cache_hit {
                append_denominator.exact_cache_hits += 1;
                append_denominator.exact_cache_tokens = append_denominator
                    .exact_cache_tokens
                    .saturating_add(adapter_event.tokens);
                append_denominator.exact_cache_cost_microusd = append_denominator
                    .exact_cache_cost_microusd
                    .saturating_add(adapter_event.cost_microusd);
            } else {
                append_denominator.non_exact_rows += 1;
            }

            let mut decision_rows = Vec::new();
            let mut scored_before_update = false;
            if let Some((hot_runtime, route_table)) = store
                .candidate_hot_runtime_and_route_table()
                .map_err(|error| format!("failed to build current append hot view: {error:?}"))?
            {
                append_hot_view_available_events += 1;
                if let Some(route_index) = route_table.resolve_route_index(adapter_event.route_id) {
                    let mut scratch =
                        PhaseCenterHotScratch::new(cells, route_table.profile_edge_count().max(1))
                            .map_err(|error| {
                                format!("failed to build append live-loop scratch: {error:?}")
                            })?;
                    let decisions = hot_runtime
                        .score_hot_request_candidates(
                            &route_table,
                            PhaseCenterHotRequest::new(route_index, &adapter_event.atom_ids),
                            &mut scratch,
                        )
                        .map_err(|error| format!("failed append live-loop hot score: {error:?}"))?;
                    let decisions =
                        live_store_exact_bucket_decisions(decisions, adapter_event.bucket_id);
                    scored_before_update = true;
                    live_store_update_candidate_decision_eval(
                        adapter_event.verified_safe_accept,
                        adapter_event.exact_cache_hit,
                        adapter_event.tokens,
                        adapter_event.cost_microusd,
                        &decisions,
                        &mut append_eval,
                    );
                    decision_rows = decisions
                        .iter()
                        .map(|decision| {
                            serde_json::json!({
                                "profile_id": decision.profile_id,
                                "margin_micro": decision.margin_micro,
                                "score_candidate": decision.score_candidate,
                                "verifier_required": decision.verifier_required,
                                "local_accept": decision.local_accept
                            })
                        })
                        .collect();
                } else {
                    append_route_index_missing_events += 1;
                }
            }

            let learning_decision = store
                .observe_atom_event(&mut encoder, adapter_event.to_live_operator_atom_event())
                .map_err(|error| {
                    format!(
                        "append live-loop observe failed for '{}' line {}: {error:?}",
                        source_label, line_index
                    )
                })?;
            bucket_policy.observe_decision(&adapter_event, learning_decision);
            let line = serde_json::json!({
                "append_row_index": append_row_index,
                "source": source_label,
                "line_index": line_index,
                "route_id": adapter_event.route_id,
                "bucket_id": adapter_event.bucket_id,
                "verified_safe_accept": adapter_event.verified_safe_accept,
                "exact_cache_hit": adapter_event.exact_cache_hit,
                "tokens": adapter_event.tokens,
                "cost_microusd": adapter_event.cost_microusd,
                "scored_before_update": scored_before_update,
                "learning_active_before_update": learning_decision.active_before_update,
                "learning_false_accept": learning_decision.false_accept,
                "decisions": decision_rows
            });
            serde_json::to_writer(&mut decision_log, &line).map_err(|error| {
                format!(
                    "failed to write append live-loop decision '{}': {error}",
                    decision_log_path.display()
                )
            })?;
            decision_log
                .write_all(b"\n")
                .map_err(|error| format!("failed append live-loop decision newline: {error}"))?;
            append_parsed_rows += 1;
            append_row_index += 1;
        }
    }
    decision_log
        .flush()
        .map_err(|error| format!("failed to flush append live-loop decision log: {error}"))?;

    let summary = store.summary();
    let runtime_budget = live_store_budget_report(store.runtime_budget_snapshot());
    let final_hot_view = store
        .candidate_hot_runtime_and_route_table()
        .map_err(|error| format!("failed to build final append live-loop hot view: {error:?}"))?;
    let (
        final_hot_runtime_available,
        final_hot_route_ids,
        final_hot_profile_ids,
        final_hot_route_count,
        final_hot_profile_count,
        final_hot_route_profile_edges,
        promotion_evidence_eligible,
        promotion_evidence_blocker,
        admission_attempts,
        admission_admitted,
        admission_rejected,
        admission_hot_route_count,
        admission_hot_profile_count,
        admission_hot_route_profile_edges,
        admission_hot_bytes_estimate,
        admission_budget_passed,
    ) = if let Some((hot_runtime, route_table)) = final_hot_view {
        let exact_cache_overlap_excluded = append_denominator.non_exact_rows > 0
            && append_eval.unique_cpu_accepts_over_exact_cache <= append_denominator.non_exact_rows;
        let token_cost_denominator_present =
            append_denominator.total_tokens > 0 && append_denominator.total_cost_microusd > 0;
        let evidence = PhaseCenterPromotionEvidence {
            future_shadow_events: append_eval.score_events,
            unique_cpu_accepts_over_exact_cache: append_eval.unique_cpu_accepts_over_exact_cache,
            tokens_saved: append_eval.tokens_saved,
            cost_saved_microusd: append_eval.cost_saved_microusd,
            false_accepts: append_eval.false_accepts,
            runtime_margin_parity_mismatches: 0,
            verifier_binding: live_store_verifier_binding(),
            threshold_policy: store.threshold_policy_evidence(),
            exact_cache_overlap_excluded,
            token_cost_denominator_present,
            local_accept_enabled: false,
        };
        let promotion_decision = evidence.evaluate();
        let mut memory =
            PhaseCenterOperatorMemory::new(store.memory_config()).map_err(|error| {
                format!("failed to create append live-loop admission memory: {error:?}")
            })?;
        let mut attempts = 0usize;
        let mut admitted = 0usize;
        let mut rejected = 0usize;
        let runtime_bytes_per_profile =
            hot_runtime.bytes_estimate() / hot_runtime.profile_count().max(1);
        for route_index in 0..route_table.route_count() {
            let route_id = route_table.route_id_at(route_index).unwrap_or_default();
            let route_plan = route_table
                .route_plan_at(route_index)
                .map_err(|error| format!("failed to read append route plan: {error:?}"))?;
            for profile_index in route_plan.profile_indexes() {
                let Some(profile_id) = hot_runtime.profile_id_at(*profile_index) else {
                    continue;
                };
                attempts += 1;
                let decision = memory.admit(PhaseCenterOperatorAdmission {
                    route_id,
                    profile_id,
                    evidence,
                    runtime_bytes_estimate: runtime_bytes_per_profile,
                    last_seen_tick: append_parsed_rows as u64,
                });
                if decision.admitted {
                    admitted += 1;
                } else {
                    rejected += 1;
                }
            }
        }
        let admission_routes = memory
            .hot_route_table(&hot_runtime)
            .map_err(|error| format!("failed to build append admission route table: {error:?}"))?;
        let admission_budget = memory.runtime_budget_snapshot(&hot_runtime, &admission_routes);
        (
            true,
            live_store_hot_route_ids(&route_table),
            live_store_hot_profile_ids(&hot_runtime),
            route_table.route_count(),
            hot_runtime.profile_count(),
            route_table.profile_edge_count(),
            promotion_decision.eligible,
            promotion_decision
                .blocker
                .map(live_store_promotion_blocker_name)
                .unwrap_or("none"),
            attempts,
            admitted,
            rejected,
            admission_routes.route_count(),
            hot_runtime.profile_count(),
            admission_routes.profile_edge_count(),
            admission_budget.hot_bytes_estimate,
            admission_budget.product_runtime_budget_passed(),
        )
    } else {
        (
            false,
            Vec::new(),
            Vec::new(),
            0,
            0,
            0,
            false,
            "append_live_loop_no_final_hot_view",
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            false,
        )
    };

    let exact_cache_overlap_excluded = append_denominator.non_exact_rows > 0
        && append_eval.unique_cpu_accepts_over_exact_cache <= append_denominator.non_exact_rows;
    let token_cost_denominator_present =
        append_denominator.total_tokens > 0 && append_denominator.total_cost_microusd > 0;
    let gate_passed = watermark_parsed_rows > 0
        && append_parsed_rows > 0
        && append_eval.score_events > 0
        && append_eval.unique_cpu_accepts_over_exact_cache > 0
        && append_eval.tokens_saved > 0
        && append_eval.cost_saved_microusd > 0
        && append_eval.false_accepts == 0
        && append_eval.local_accept_events == 0
        && final_hot_runtime_available
        && promotion_evidence_eligible
        && admission_attempts > 0
        && admission_admitted > 0
        && admission_rejected == 0
        && admission_budget_passed
        && exact_cache_overlap_excluded
        && token_cost_denominator_present
        && runtime_budget.product_runtime_budget_passed;
    let append_compression_claim_blocker = live_store_append_compression_claim_blocker(
        append_parsed_rows,
        append_eval.false_accepts,
        append_eval.local_accept_events,
        append_eval.unique_cpu_accepts_over_exact_cache,
        append_eval.tokens_saved,
        token_cost_denominator_present,
        final_hot_runtime_available,
        true,
        0,
        live_store_append_compression_claim_min_rows(),
    )
    .to_owned();
    let append_compression_claim_allowed = append_compression_claim_blocker == "none";
    let blocker = if gate_passed {
        "none".to_owned()
    } else if watermark_parsed_rows == 0 {
        "append_live_loop_no_watermark_events".to_owned()
    } else if append_parsed_rows == 0 {
        "append_live_loop_no_append_events".to_owned()
    } else if append_eval.score_events == 0 {
        "append_live_loop_no_score_before_update_events".to_owned()
    } else if append_eval.false_accepts != 0 {
        "append_live_loop_false_accepts_nonzero".to_owned()
    } else if append_eval.local_accept_events != 0 {
        "append_live_loop_local_accept_enabled".to_owned()
    } else if append_eval.unique_cpu_accepts_over_exact_cache == 0 {
        "append_live_loop_unique_accepts_zero".to_owned()
    } else if append_eval.tokens_saved == 0 || append_eval.cost_saved_microusd == 0 {
        "append_live_loop_token_cost_denominator_missing".to_owned()
    } else if !final_hot_runtime_available {
        "append_live_loop_no_final_hot_view".to_owned()
    } else if !promotion_evidence_eligible {
        format!("append_live_loop_promotion_evidence_{promotion_evidence_blocker}")
    } else if admission_attempts == 0 || admission_admitted == 0 {
        "append_live_loop_admission_queue_empty".to_owned()
    } else if admission_rejected != 0 {
        "append_live_loop_admission_rejected_candidate".to_owned()
    } else if !admission_budget_passed || !runtime_budget.product_runtime_budget_passed {
        "append_live_loop_budget_failed".to_owned()
    } else if !exact_cache_overlap_excluded || !token_cost_denominator_present {
        "append_live_loop_denominator_missing".to_owned()
    } else {
        "append_live_loop_smoke_failed".to_owned()
    };

    let report = PhaseStreamHotPathDaemonAppendLiveLoopSmokeReport {
        report_kind: "phase_stream_hot_path_daemon_append_live_loop_smoke_v1",
        architecture_versions: live_store_architecture_versions(),
        mode: "append_live_loop_score_before_update_shadow_admission_queue",
        decision_log_path: decision_log_path.display().to_string(),
        watermark_trace_paths: vec![watermark_trace_path.display().to_string()],
        append_trace_paths: append_trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        cells,
        min_bucket_events,
        watermark_total_rows,
        watermark_parsed_rows,
        watermark_skipped_no_verifier_label,
        watermark_skipped_no_safe_atoms,
        append_total_rows,
        append_parsed_rows,
        append_skipped_no_verifier_label,
        append_skipped_no_safe_atoms,
        append_hot_view_available_events,
        append_route_index_missing_events,
        append_score_events_before_update: append_eval.score_events,
        append_score_candidate_events: append_eval.score_candidate_events,
        append_observable_score_candidate_events: 0,
        append_hidden_state_score_candidate_events: 0,
        append_unknown_profile_score_candidate_events: 0,
        append_verifier_required_events: append_eval.verifier_required_events,
        append_local_accept_events: append_eval.local_accept_events,
        append_unique_cpu_accepts_over_exact_cache: append_eval.unique_cpu_accepts_over_exact_cache,
        append_observable_unique_cpu_accepts_over_exact_cache: 0,
        append_hidden_state_unique_cpu_accepts_over_exact_cache: 0,
        append_unknown_profile_unique_cpu_accepts_over_exact_cache: 0,
        append_profile_attribution_overlap_accepts: 0,
        append_observable_only_unique_cpu_accepts_over_exact_cache: 0,
        append_hidden_state_only_unique_cpu_accepts_over_exact_cache: 0,
        append_mixed_profile_unique_cpu_accepts_over_exact_cache: 0,
        append_unknown_only_unique_cpu_accepts_over_exact_cache: 0,
        append_tokens_saved: append_eval.tokens_saved,
        append_observable_tokens_saved: 0,
        append_hidden_state_tokens_saved: 0,
        append_unknown_profile_tokens_saved: 0,
        append_observable_only_tokens_saved: 0,
        append_hidden_state_only_tokens_saved: 0,
        append_mixed_profile_tokens_saved: 0,
        append_unknown_only_tokens_saved: 0,
        append_observable_only_cost_saved_microusd: 0,
        append_hidden_state_only_cost_saved_microusd: 0,
        append_mixed_profile_cost_saved_microusd: 0,
        append_unknown_only_cost_saved_microusd: 0,
        append_cost_saved_microusd: append_eval.cost_saved_microusd,
        append_exact_cache_calls_saved_milli_over_parsed_rows: 0,
        append_exact_cache_tokens_saved_milli_over_total: 0,
        append_exact_cache_cost_saved_milli_over_total: 0,
        append_active_clean_calls_saved_milli_over_parsed_rows: 0,
        append_active_clean_tokens_saved_milli_over_total: 0,
        append_active_clean_cost_saved_milli_over_total: 0,
        append_combined_calls_saved_milli_over_parsed_rows: 0,
        append_combined_tokens_saved_milli_over_total: 0,
        append_combined_cost_saved_milli_over_total: 0,
        append_hidden_state_only_calls_saved_milli_over_parsed_rows: 0,
        append_hidden_state_only_tokens_saved_milli_over_total: 0,
        append_hidden_state_only_cost_saved_milli_over_total: 0,
        append_cost_estimate_used: false,
        append_estimated_cost_events: 0,
        append_estimated_total_cost_microusd: 0,
        append_estimated_cost_saved_microusd: 0,
        append_false_accepts: append_eval.false_accepts,
        append_compression_claim_min_rows: live_store_append_compression_claim_min_rows(),
        append_compression_claim_allowed,
        append_compression_claim_blocker,
        append_total_tokens: append_denominator.total_tokens,
        append_total_cost_microusd: append_denominator.total_cost_microusd,
        append_exact_cache_hits: append_denominator.exact_cache_hits,
        append_exact_cache_tokens: append_denominator.exact_cache_tokens,
        append_exact_cache_cost_microusd: append_denominator.exact_cache_cost_microusd,
        append_non_exact_rows: append_denominator.non_exact_rows,
        online_bucket_count: summary.bucket_count,
        active_bucket_count: summary.active_bucket_count,
        shadow_ready_bucket_count: summary.shadow_ready_bucket_count,
        candidate_bucket_count: summary.candidate_bucket_count,
        rejected_bucket_count: summary.rejected_bucket_count,
        live_route_count: store.route_count(),
        live_route_bucket_count: store.route_bucket_count(),
        learning_false_accepts_before_rejection: summary.false_accepts,
        final_hot_runtime_available,
        final_hot_route_ids,
        final_hot_profile_ids,
        final_hot_route_count,
        final_hot_profile_count,
        final_hot_route_profile_edges,
        runtime_budget,
        promotion_evidence_eligible,
        promotion_evidence_blocker,
        admission_attempts,
        admission_admitted,
        admission_rejected,
        admission_hot_route_count,
        admission_hot_profile_count,
        admission_hot_route_profile_edges,
        admission_hot_bytes_estimate,
        admission_budget_passed,
        admission_queue_shadow_only: true,
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        verifier_binding_bound: live_store_verifier_binding().is_bound(),
        budget_snapshot_live: true,
        direct_mutable_store_used: true,
        score_before_update_used: true,
        current_hot_view_refreshed_during_append: true,
        core_numeric_route_bucket_ids_used: true,
        cold_adapter_json_used: true,
        cold_adapter_strings_used: true,
        quarantine_nwpc_checkpoint_compile_used: false,
        package_roundtrip_used: false,
        registry_mutation_enabled: false,
        cpu_profile_registry_write_enabled: false,
        serving_profile_artifact_written: false,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        verdict: if gate_passed {
            "HOT_PATH_DAEMON_APPEND_LIVE_LOOP_SMOKE_PASS"
        } else {
            "HOT_PATH_DAEMON_APPEND_LIVE_LOOP_SMOKE_WATCH"
        },
        blocker,
        boundary: "append live-loop smoke only: watermark events initialize PhaseCenterLiveOperatorStore, append events are scored against the current hot view before update and then observed into the mutable phase-center store; an in-memory admission queue is evaluated from verifier-bound shadow evidence, but no quarantine .nwpc checkpoint compile, package roundtrip, registry mutation, serving profile write, product promotion, local_accept, or market money claim occurs",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_hot_path_daemon_append_live_loop_smoke_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  decision_log_path: {}", decision_log_path.display());
    println!(
        "  append_score_events_before_update: {}",
        report.append_score_events_before_update
    );
    println!(
        "  append_unique_cpu_accepts_over_exact_cache: {}",
        report.append_unique_cpu_accepts_over_exact_cache
    );
    println!("  append_false_accepts: {}", report.append_false_accepts);
    println!("  admission_admitted: {}", report.admission_admitted);
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_hot_path_daemon_append_live_tail_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_REPORT));
    let decision_log_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_DECISION_LOG));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_CELLS);
    let min_bucket_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid min_bucket_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    let idle_sleep_ms = args
        .next()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid idle_sleep_ms value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(100);
    let max_idle_ms = args
        .next()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid max_idle_ms value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(0);
    let max_append_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_append_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(0);
    let watermark_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL));
    let append_tail_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL));
    let external_product_hot_registry_enabled =
        std::env::var("NANDO_PHASE_ENABLE_EXTERNAL_PRODUCT_HOT_REGISTRY")
            .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
    let product_hot_registry_path = args.next().map(PathBuf::from).or_else(|| {
        external_product_hot_registry_enabled
            .then(|| std::env::var_os("NANDO_PHASE_PRODUCT_HOT_REGISTRY_PATH").map(PathBuf::from))
            .flatten()
    });
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    if min_bucket_events == 0 {
        return Err("min_bucket_events must be > 0".to_owned());
    }
    if idle_sleep_ms == 0 {
        return Err("idle_sleep_ms must be > 0".to_owned());
    }
    let miner_discovery_sample_permille = live_store_env_usize(
        "NANDO_PHASE_DISCOVERY_SAMPLE_PERMILLE",
        DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_DISCOVERY_SAMPLE_PERMILLE,
    )
    .min(1_000);
    let miner_saturation_min_idle_heartbeats =
        live_store_env_usize("NANDO_PHASE_MINER_SATURATION_MIN_IDLE_HEARTBEATS", 3).max(1);
    let miner_saturation_sleep_ms =
        live_store_env_usize("NANDO_PHASE_MINER_SATURATION_SLEEP_MS", 5_000) as u64;
    let miner_saturation_control_enabled = miner_saturation_sleep_ms > idle_sleep_ms;
    let miner_active_batch_rows = live_store_env_usize("NANDO_PHASE_MINER_ACTIVE_BATCH_ROWS", 64);
    let miner_active_batch_sleep_ms =
        live_store_env_usize("NANDO_PHASE_MINER_ACTIVE_BATCH_SLEEP_MS", 5) as u64;
    let price_config =
        super::read_json_file::<super::ModelPriceConfig>(Path::new(super::DEFAULT_PRICE_CONFIG))?;
    let call_token_promotion_manifest_path =
        live_store_append_tail_call_token_promotion_manifest_path(&report_path);
    let call_token_active_manifest_path =
        live_store_append_tail_call_token_active_manifest_path(&report_path);
    let call_token_promotion_package_dir =
        live_store_append_tail_call_token_promotion_package_dir(&report_path);
    let architecture_version_key = live_store_architecture_version_key();
    let persisted_product_hot_quarantine =
        live_store_load_persisted_product_hot_quarantine(&report_path)?;
    let mut product_hot_score_only_quarantined_profile_ids =
        persisted_product_hot_quarantine.profile_ids;
    let stable_non_exact_false_profile_ids =
        live_store_stable_decision_log_non_exact_false_profile_ids(
            &decision_log_path,
            &architecture_version_key,
        )?;
    product_hot_score_only_quarantined_profile_ids
        .extend(stable_non_exact_false_profile_ids.iter().copied());
    let mut product_hot_score_only_quarantine_reason =
        if product_hot_score_only_quarantined_profile_ids.is_empty() {
            String::new()
        } else if !stable_non_exact_false_profile_ids.is_empty() {
            "stable_decision_log_non_exact_false_profile_quarantine".to_owned()
        } else {
            persisted_product_hot_quarantine.reason
        };
    let mut product_hot_score_only_quarantine_false_accepts =
        persisted_product_hot_quarantine.false_accepts;
    let mut product_hot_score_only_post_quarantine_score_candidate_events = 0usize;
    let mut product_hot_score_only_post_quarantine_false_accepts = 0usize;
    let mut product_hot_phase_trust_filtered_events = 0usize;
    let mut product_hot_score_only_credit_rows = Vec::<LiveStoreProductHotCreditRow>::new();
    let mut active_clean_calls_saved = 0usize;
    let mut active_clean_tokens_saved = 0u64;
    let mut lost_calls_due_to_quarantine = 0usize;
    let mut lost_tokens_due_to_quarantine = 0u64;
    let mut product_hot_score_only_quarantine_trace_id = persisted_product_hot_quarantine.trace_id;
    let mut product_hot_score_only_quarantine_route_key =
        persisted_product_hot_quarantine.route_key;
    let mut product_hot_score_only_quarantine_bucket_key =
        persisted_product_hot_quarantine.bucket_key;
    let safe_manifest_profile_ids =
        live_store_allowed_call_token_manifest_profile_ids(&call_token_active_manifest_path)?
            .into_iter()
            .chain(
                live_store_allowed_call_token_manifest_profile_ids(
                    &call_token_promotion_manifest_path,
                )?
                .into_iter(),
            )
            .chain(live_store_trusted_clean_report_profile_ids(&report_path)?.into_iter())
            .collect::<BTreeSet<_>>();
    if !safe_manifest_profile_ids.is_empty()
        && !product_hot_score_only_quarantined_profile_ids.is_empty()
    {
        product_hot_score_only_quarantined_profile_ids
            .retain(|profile_id| !safe_manifest_profile_ids.contains(profile_id));
        if product_hot_score_only_quarantined_profile_ids.is_empty() {
            product_hot_score_only_quarantine_reason.clear();
            product_hot_score_only_quarantine_false_accepts = 0;
            product_hot_score_only_quarantine_trace_id.clear();
            product_hot_score_only_quarantine_route_key.clear();
            product_hot_score_only_quarantine_bucket_key.clear();
        }
    }
    let mut product_hot_score_only_active_manifest_disabled = false;
    let mut product_hot_score_only_active_manifest_disable_reason = String::new();
    let mut product_hot_registry_source_report = String::new();
    if call_token_active_manifest_path.exists() {
        let active_manifest = super::read_json_value(&call_token_active_manifest_path)?;
        if super::json_bool(&active_manifest, &["live_score_only_disabled"]).unwrap_or(false) {
            product_hot_score_only_active_manifest_disabled = true;
            product_hot_score_only_active_manifest_disable_reason =
                super::json_string(&active_manifest, &["live_score_only_disable_reason"])
                    .unwrap_or_else(|| "active_manifest_already_disabled".to_owned());
        } else if super::json_bool(&active_manifest, &["allowed"]).unwrap_or(false)
            && live_store_call_token_manifest_promotes_quarantined_profile(
                &active_manifest,
                &product_hot_score_only_quarantined_profile_ids,
            )
        {
            product_hot_score_only_active_manifest_disabled = true;
            product_hot_score_only_active_manifest_disable_reason =
                "active_manifest_promotes_quarantined_profile".to_owned();
            disable_live_store_call_token_active_manifest(
                &call_token_active_manifest_path,
                &product_hot_score_only_active_manifest_disable_reason,
                product_hot_score_only_quarantine_false_accepts,
                &product_hot_score_only_quarantine_trace_id,
                &product_hot_score_only_quarantine_route_key,
                &product_hot_score_only_quarantine_bucket_key,
                &product_hot_score_only_quarantined_profile_ids,
            )?;
        }
    }
    let mut product_hot_registry_runtime = if let Some(bundle) =
        try_load_live_store_allowed_call_token_runtime(
            &call_token_active_manifest_path,
            cells,
            &product_hot_score_only_quarantined_profile_ids,
        )? {
        product_hot_registry_source_report = "call_token_active_manifest".to_owned();
        Some(bundle)
    } else if let Some(bundle) = try_load_live_store_allowed_call_token_runtime(
        &call_token_promotion_manifest_path,
        cells,
        &product_hot_score_only_quarantined_profile_ids,
    )? {
        std::fs::copy(
            &call_token_promotion_manifest_path,
            &call_token_active_manifest_path,
        )
        .map_err(|error| {
            format!(
                "failed to persist startup active call-token manifest '{}': {error}",
                call_token_active_manifest_path.display()
            )
        })?;
        product_hot_score_only_active_manifest_disabled = false;
        product_hot_score_only_active_manifest_disable_reason.clear();
        product_hot_registry_source_report = "call_token_promotion_manifest".to_owned();
        Some(bundle)
    } else {
        let loaded = product_hot_registry_path
            .as_deref()
            .map(|path| load_live_store_product_hot_registry_runtime(path, cells))
            .transpose()?;
        if loaded.is_some() {
            product_hot_registry_source_report = "product_hot_registry".to_owned();
        }
        loaded
    };
    let mut product_hot_registry_path_report = product_hot_registry_runtime
        .as_ref()
        .map(|bundle| bundle.registry_path.display().to_string())
        .unwrap_or_default();

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
                DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_HOT_PROFILES_PER_WORKER,
            max_hot_bytes_per_worker: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
            max_warm_profiles_per_process:
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
            max_profiles_per_route: DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_PROFILES_PER_ROUTE,
            max_route_top_k: DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_ROUTE_TOP_K,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .map_err(|error| format!("failed to create append live-tail store: {error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create append live-tail encoder: {error:?}"))?;
    let mut future_encoder = PhaseCenterAtomEncoder::new(cells).map_err(|error| {
        format!("failed to create append live-tail future-shadow encoder: {error:?}")
    })?;
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let verifier_binding = live_store_verifier_binding();
    let mut frozen_candidates = BTreeMap::<u32, LiveStoreFrozenCandidate>::new();
    let mut future_shadow = PhaseStreamLiveStoreFutureShadowReport::default();
    let mut append_profile_kind_by_id = BTreeMap::<u32, &'static str>::new();
    let mut watermark_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut watermark_total_rows = 0usize;
    let mut watermark_parsed_rows = 0usize;
    let mut watermark_skipped_no_verifier_label = 0usize;
    let mut watermark_skipped_no_safe_atoms = 0usize;

    let watermark_file = File::open(&watermark_trace_path).map_err(|error| {
        format!(
            "failed to open live-tail watermark trace '{}': {error}",
            watermark_trace_path.display()
        )
    })?;
    live_store_observe_live_loop_budget_events(
        &watermark_trace_path.display().to_string(),
        io::BufReader::new(watermark_file),
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
    for event in &watermark_events {
        live_store_record_event_profile_kinds(event, &mut append_profile_kind_by_id);
    }
    watermark_events.clear();
    watermark_events.shrink_to_fit();

    if let Some(parent) = append_tail_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create append live-tail source dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    if let Some(parent) = decision_log_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create append live-tail decision dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let append_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&append_tail_path)
        .map_err(|error| {
            format!(
                "failed to open append live-tail source '{}': {error}",
                append_tail_path.display()
            )
        })?;
    let mut append_history_warm_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut append_history_warm_total_rows = 0usize;
    let mut append_history_warm_parsed_rows = 0usize;
    let mut append_history_warm_skipped_no_verifier_label = 0usize;
    let mut append_history_warm_skipped_no_safe_atoms = 0usize;
    let append_history_file = append_file
        .try_clone()
        .map_err(|error| format!("failed to clone append live-tail source: {error}"))?;
    live_store_observe_live_loop_budget_events(
        &format!("{}#warm-history", append_tail_path.display()),
        io::BufReader::new(append_history_file),
        &mut store,
        &mut encoder,
        &mut bucket_policy,
        &mut exact_cache_keys_seen,
        &mut append_history_warm_events,
        &mut append_history_warm_total_rows,
        &mut append_history_warm_parsed_rows,
        &mut append_history_warm_skipped_no_verifier_label,
        &mut append_history_warm_skipped_no_safe_atoms,
    )?;
    for event in &append_history_warm_events {
        live_store_record_event_profile_kinds(event, &mut append_profile_kind_by_id);
    }
    append_history_warm_events.clear();
    append_history_warm_events.shrink_to_fit();
    if product_hot_registry_runtime.is_none() {
        if let Some(survivor_runtime) = live_store_clean_candidate_survivor_runtime_from_store(
            &store,
            cells,
            &report_path.with_file_name("live-store-clean-candidate-survivors.shadow-runtime"),
            &product_hot_score_only_quarantined_profile_ids,
            &[],
            &append_profile_kind_by_id,
            min_bucket_events,
        )? {
            product_hot_registry_path_report = survivor_runtime.registry_path.display().to_string();
            product_hot_registry_source_report = "live_store_clean_candidate_survivors".to_owned();
            product_hot_registry_runtime = Some(survivor_runtime);
        }
    }
    let mut reader = io::BufReader::new(append_file);
    reader
        .seek(SeekFrom::End(0))
        .map_err(|error| format!("failed to seek append live-tail source to end: {error}"))?;
    let mut stable_decision_log_window = live_store_stable_decision_log_window_from_path(
        &decision_log_path,
        &architecture_version_key,
    )?;
    let mut stable_decision_log_clean_suffix =
        live_store_stable_decision_log_clean_suffix_from_path(
            &decision_log_path,
            &architecture_version_key,
        )?;
    let mut stable_serving_cpu_window = live_store_stable_decision_log_serving_window_from_path(
        &decision_log_path,
        &architecture_version_key,
    )?;
    let mut stable_serving_cpu_clean_suffix =
        live_store_stable_decision_log_serving_clean_suffix_from_path(
            &decision_log_path,
            &architecture_version_key,
        )?;
    let mut decision_log = io::BufWriter::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&decision_log_path)
            .map_err(|error| {
                format!(
                    "failed to open append live-tail decision log '{}': {error}",
                    decision_log_path.display()
                )
            })?,
    );

    let mut append_total_lines_seen = 0usize;
    let mut append_parsed_rows = 0usize;
    let mut append_skipped_no_verifier_label = 0usize;
    let mut append_skipped_no_safe_atoms = 0usize;
    let mut append_hot_view_available_events = 0usize;
    let mut append_route_index_missing_events = 0usize;
    let mut append_route_index_missing_before_first_score = 0usize;
    let mut append_route_index_missing_after_first_score = 0usize;
    let mut append_auto_subcenter_observe_events = 0usize;
    let mut append_auto_subcenter_throttled_events = 0usize;
    let mut append_hidden_state_subcenter_observe_events = 0usize;
    let mut miner_clean_hot_runtime_throttle_events = 0usize;
    let mut append_observable_score_candidate_events = 0usize;
    let mut append_hidden_state_score_candidate_events = 0usize;
    let mut append_unknown_profile_score_candidate_events = 0usize;
    let mut append_observable_unique_cpu_accepts_over_exact_cache = 0usize;
    let mut append_hidden_state_unique_cpu_accepts_over_exact_cache = 0usize;
    let mut append_unknown_profile_unique_cpu_accepts_over_exact_cache = 0usize;
    let mut append_profile_attribution_overlap_accepts = 0usize;
    let mut append_observable_only_unique_cpu_accepts_over_exact_cache = 0usize;
    let mut append_hidden_state_only_unique_cpu_accepts_over_exact_cache = 0usize;
    let mut append_mixed_profile_unique_cpu_accepts_over_exact_cache = 0usize;
    let mut append_unknown_only_unique_cpu_accepts_over_exact_cache = 0usize;
    let mut append_observable_tokens_saved = 0u64;
    let mut append_hidden_state_tokens_saved = 0u64;
    let mut append_unknown_profile_tokens_saved = 0u64;
    let mut append_observable_only_tokens_saved = 0u64;
    let mut append_hidden_state_only_tokens_saved = 0u64;
    let mut append_mixed_profile_tokens_saved = 0u64;
    let mut append_unknown_only_tokens_saved = 0u64;
    let mut append_observable_only_cost_saved_microusd = 0u64;
    let mut append_hidden_state_only_cost_saved_microusd = 0u64;
    let mut append_mixed_profile_cost_saved_microusd = 0u64;
    let mut append_unknown_only_cost_saved_microusd = 0u64;
    let mut append_scoring_started = false;
    let mut append_eval = LiveStorePreparedHotPackEval::default();
    let mut append_denominator = LiveStoreHotPathDenominator::default();
    let mut append_estimated_cost_events = 0usize;
    let mut append_estimated_total_cost_microusd = 0u64;
    let mut append_estimated_cost_saved_microusd = 0u64;
    let mut append_clean_suffix_rows = 0usize;
    let mut append_clean_suffix_score_events = 0usize;
    let mut append_clean_suffix_unique_cpu_accepts_over_exact_cache = 0usize;
    let mut append_clean_suffix_tokens_saved = 0u64;
    let mut append_clean_suffix_cost_saved_microusd = 0u64;
    let mut append_clean_suffix_false_accepts = 0usize;
    let mut append_clean_suffix_local_accept_events = 0usize;
    let mut append_clean_suffix_last_quarantine_row_index = None::<usize>;
    let mut hot_scratch = None::<PhaseCenterHotScratch>;
    let mut product_hot_scratch = product_hot_registry_runtime
        .as_ref()
        .map(|bundle| {
            PhaseCenterHotScratch::new(bundle.cells, bundle.route_table.profile_edge_count().max(1))
        })
        .transpose()
        .map_err(|error| format!("failed to build product-hot score-only scratch: {error:?}"))?;
    let mut product_hot_score_only_auto_refinement_candidate_atoms = Vec::<String>::new();
    let mut product_hot_score_only_auto_refinement_selected_atoms = Vec::<String>::new();
    let mut quarantine_recovery_discovery_events = 0usize;
    let mut quarantine_recovery_discovery_tokens = 0u64;
    let mut quarantine_recovery_auto_subcenter_observe_events = 0usize;
    let mut hot_scratch_rebuilds = 0usize;
    let mut decision_log_flush_count = 0usize;
    let mut decision_log_pending_rows = 0usize;
    let mut idle_elapsed_ms = 0u64;
    let mut line = String::new();
    let mut last_heartbeat = Instant::now();
    let mut last_cold_artifact_refresh = Instant::now();
    let mut cold_artifact_refresh_count = 0usize;
    let mut cold_artifact_refresh_append_rows = 0usize;
    let candidate_package_dir = live_store_numeric_candidate_package_dir(&report_path);
    let mut candidate_package_reports = Vec::<PhaseStreamLiveStoreCandidatePackageReport>::new();
    let clean_promotion_manifest_path =
        live_store_append_tail_clean_promotion_manifest_path(&report_path);
    let clean_promotion_package_dir =
        live_store_append_tail_clean_promotion_package_dir(&report_path);
    let mut clean_promotion_manifest_written = false;
    let mut call_token_promotion_manifest_written = false;
    let future_shadow_billing_request_path =
        live_store_future_shadow_billing_request_path(&report_path);
    let mut future_shadow_billing_request = LiveStoreFutureShadowBillingRequestSummary::default();
    let mut provider_artifact_signature = None::<LiveStoreProviderArtifactSignature>;
    let provider_evidence_artifacts = LiveStoreProviderEvidenceArtifactsReport::default();
    let mut miner_saturation = LiveStoreMinerSaturationController::default();
    let mut miner_active_batch_rows_seen = 0usize;
    let mut miner_active_batch_sleep_events = 0usize;

    loop {
        line.clear();
        let line_start = reader.stream_position().map_err(|error| {
            format!("failed to capture append live-tail cursor before read: {error}")
        })?;
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("failed to read append live-tail source: {error}"))?;
        if bytes == 0 || !line.ends_with('\n') {
            if bytes == 0 {
                reader.seek(SeekFrom::Current(0)).map_err(|error| {
                    format!("failed to refresh append live-tail source: {error}")
                })?;
            } else {
                reader.seek(SeekFrom::Start(line_start)).map_err(|error| {
                    format!("failed to rewind partial append live-tail line: {error}")
                })?;
            }
            if max_append_events > 0 && append_parsed_rows >= max_append_events {
                break;
            }
            if max_idle_ms > 0 && idle_elapsed_ms >= max_idle_ms {
                break;
            }
            let mut planned_idle_sleep_ms = None::<u64>;
            if last_heartbeat.elapsed()
                >= Duration::from_secs(DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_HEARTBEAT_SECS)
            {
                if decision_log_pending_rows > 0 {
                    decision_log.flush().map_err(|error| {
                        format!("failed append live-tail heartbeat decision flush: {error}")
                    })?;
                    decision_log_flush_count = decision_log_flush_count.saturating_add(1);
                    decision_log_pending_rows = 0;
                }
                let summary = store.summary();
                let runtime_budget = live_store_budget_report(store.runtime_budget_snapshot());
                let product_hot_budget_passed = runtime_budget.hot_budget_passed;
                let warm_miner_budget_passed = runtime_budget.warm_budget_passed;
                let warm_miner_budget_blocker = if warm_miner_budget_passed {
                    "none"
                } else {
                    "warm_miner_budget_watch"
                }
                .to_owned();
                let exact_cache_overlap_excluded = append_denominator.non_exact_rows > 0
                    && append_eval.unique_cpu_accepts_over_exact_cache
                        <= append_denominator.non_exact_rows;
                let token_cost_denominator_present = append_denominator.total_tokens > 0
                    && append_denominator.total_cost_microusd > 0;
                let cold_artifact_refresh_due = append_parsed_rows > 0
                    && (cold_artifact_refresh_count == 0
                        || append_parsed_rows != cold_artifact_refresh_append_rows)
                    && last_cold_artifact_refresh.elapsed()
                        >= Duration::from_secs(
                            DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_COLD_REFRESH_SECS,
                        );
                if cold_artifact_refresh_due {
                    freeze_new_live_store_candidates_from_store(
                        &store,
                        verifier_binding,
                        &mut frozen_candidates,
                    )?;
                    let mut candidate_packages = Vec::new();
                    store
                        .candidate_packages_into_with_verifier(
                            verifier_binding,
                            &mut candidate_packages,
                        )
                        .map_err(|error| {
                            format!(
                                "failed to build append live-tail verifier-bound candidates: {error:?}"
                            )
                        })?;
                    candidate_package_reports =
                        write_live_store_candidate_packages_with_route_lookup(
                            &candidate_package_dir,
                            candidate_packages,
                            |bucket_id| store.route_id_for_bucket(bucket_id),
                        )?;
                    live_store_refresh_future_shadow_summary(
                        &mut future_shadow,
                        &frozen_candidates,
                        token_cost_denominator_present,
                        append_parsed_rows,
                        append_denominator.total_tokens,
                        append_denominator.total_cost_microusd,
                    )?;
                    refresh_live_store_call_token_promotion_manifest_summary_with_quarantine(
                        &mut future_shadow,
                        &frozen_candidates,
                        &product_hot_score_only_quarantined_profile_ids,
                    );
                    write_live_store_clean_promotion_manifest(
                        &clean_promotion_manifest_path,
                        &clean_promotion_package_dir,
                        &future_shadow,
                        &frozen_candidates,
                    )?;
                    clean_promotion_manifest_written = true;
                    if future_shadow.clean_promotion_manifest_allowed
                        && future_shadow.clean_promotion_manifest_false_accepts == 0
                        && future_shadow.clean_promotion_manifest_runtime_parity_mismatches == 0
                    {
                        for route in &future_shadow.clean_promotion_manifest_routes {
                            product_hot_score_only_quarantined_profile_ids
                                .remove(&route.profile_id);
                        }
                        refresh_live_store_call_token_promotion_manifest_summary_with_quarantine(
                            &mut future_shadow,
                            &frozen_candidates,
                            &product_hot_score_only_quarantined_profile_ids,
                        );
                    }
                    write_live_store_call_token_promotion_manifest_with_quarantine(
                        &call_token_promotion_manifest_path,
                        &call_token_promotion_package_dir,
                        &future_shadow,
                        &frozen_candidates,
                        &product_hot_score_only_quarantined_profile_ids,
                    )?;
                    write_live_store_clean_survivor_call_token_promotion_manifest(
                        &call_token_promotion_manifest_path,
                        &call_token_promotion_package_dir,
                        &store,
                        &frozen_candidates,
                        &product_hot_score_only_quarantined_profile_ids,
                        &append_profile_kind_by_id,
                        &mut future_shadow,
                    )?;
                    call_token_promotion_manifest_written = true;
                    let promotion_manifest_loadable =
                        if future_shadow.call_token_promotion_manifest_allowed {
                            let promotion_manifest =
                                super::read_json_value(&call_token_promotion_manifest_path)?;
                            !live_store_call_token_manifest_promotes_quarantined_profile(
                                &promotion_manifest,
                                &product_hot_score_only_quarantined_profile_ids,
                            )
                        } else {
                            false
                        };
                    if promotion_manifest_loadable
                        && future_shadow.call_token_promotion_manifest_false_accepts == 0
                    {
                        match load_live_store_product_hot_runtime_from_clean_manifest(
                            &call_token_promotion_manifest_path,
                            cells,
                        ) {
                            Ok(refreshed_runtime) => {
                                std::fs::copy(
                                    &call_token_promotion_manifest_path,
                                    &call_token_active_manifest_path,
                                )
                                .map_err(|error| {
                                    format!(
                                        "failed to persist active call-token manifest '{}': {error}",
                                        call_token_active_manifest_path.display()
                                    )
                                })?;
                                product_hot_scratch = Some(
                                    PhaseCenterHotScratch::new(
                                        refreshed_runtime.cells,
                                        refreshed_runtime
                                            .route_table
                                            .profile_edge_count()
                                            .max(1),
                                    )
                                    .map_err(|error| {
                                        format!(
                                            "failed to build refreshed call-token score-only scratch: {error:?}"
                                        )
                                    })?,
                                );
                                product_hot_registry_path_report =
                                    refreshed_runtime.registry_path.display().to_string();
                                product_hot_registry_source_report =
                                    "call_token_active_manifest".to_owned();
                                product_hot_registry_runtime = Some(refreshed_runtime);
                            }
                            Err(error) => {
                                product_hot_registry_source_report =
                                    format!("call_token_manifest_load_failed:{error}");
                                product_hot_registry_path_report.clear();
                            }
                        }
                    }
                    if !product_hot_registry_runtime.as_ref().is_some_and(|bundle| {
                        live_store_active_hot_profile_count(
                            &bundle.hot_runtime,
                            &product_hot_score_only_quarantined_profile_ids,
                        ) > 0
                    }) {
                        if let Some(survivor_runtime) =
                            live_store_clean_candidate_survivor_runtime_from_store(
                                &store,
                                cells,
                                &report_path.with_file_name(
                                    "live-store-clean-candidate-survivors.shadow-runtime",
                                ),
                                &product_hot_score_only_quarantined_profile_ids,
                                &[],
                                &append_profile_kind_by_id,
                                min_bucket_events,
                            )?
                        {
                            product_hot_scratch = Some(
                                PhaseCenterHotScratch::new(
                                    survivor_runtime.cells,
                                    survivor_runtime.route_table.profile_edge_count().max(1),
                                )
                                .map_err(|error| {
                                    format!(
                                        "failed to build clean-survivor call-token scratch: {error:?}"
                                    )
                                })?,
                            );
                            product_hot_registry_path_report =
                                survivor_runtime.registry_path.display().to_string();
                            product_hot_registry_source_report =
                                "live_store_clean_candidate_survivors".to_owned();
                            product_hot_registry_runtime = Some(survivor_runtime);
                        }
                    }
                    future_shadow_billing_request =
                        write_live_store_future_shadow_billing_requests(
                            &future_shadow_billing_request_path,
                            &frozen_candidates,
                        )?;
                    if future_shadow_billing_request.rows == 0 {
                        future_shadow_billing_request =
                            write_live_store_stable_clean_suffix_billing_requests(
                                &future_shadow_billing_request_path,
                                &decision_log_path,
                                &architecture_version_key,
                            )?;
                    }
                    let provider_signature = live_store_provider_artifact_signature(
                        &report_path,
                        &future_shadow_billing_request,
                    );
                    if provider_artifact_signature != Some(provider_signature) {
                        provider_artifact_signature = Some(provider_signature);
                    }
                    cold_artifact_refresh_count = cold_artifact_refresh_count.saturating_add(1);
                    cold_artifact_refresh_append_rows = append_parsed_rows;
                    last_cold_artifact_refresh = Instant::now();
                }
                let (
                    snapshot_hot_runtime_available,
                    snapshot_hot_route_ids,
                    snapshot_hot_profile_ids,
                    snapshot_hot_route_count,
                    snapshot_hot_profile_count,
                    snapshot_hot_route_profile_edges,
                ) = if let Some(bundle) = product_hot_registry_runtime.as_ref().filter(|bundle| {
                    live_store_active_hot_profile_count(
                        &bundle.hot_runtime,
                        &product_hot_score_only_quarantined_profile_ids,
                    ) > 0
                }) {
                    (
                        true,
                        live_store_hot_route_ids(&bundle.route_table),
                        live_store_hot_profile_ids(&bundle.hot_runtime),
                        bundle.route_table.route_count(),
                        bundle.hot_runtime.profile_count(),
                        bundle.route_table.profile_edge_count(),
                    )
                } else {
                    (false, Vec::new(), Vec::new(), 0, 0, 0)
                };
                let snapshot_product_hot_runtime_source_claim_ready =
                    live_store_product_hot_runtime_source_claim_ready(
                        &product_hot_registry_source_report,
                    );
                let (
                    clean_candidate_profile_ids,
                    clean_candidate_quarantined_profile_ids,
                    clean_candidate_exportable_profile_ids,
                ) = live_store_clean_candidate_frontier(
                    &store,
                    &product_hot_score_only_quarantined_profile_ids,
                );
                let clean_candidate_reports = live_store_clean_candidate_value_reports(
                    &store,
                    &clean_candidate_profile_ids,
                    &product_hot_score_only_quarantined_profile_ids,
                    &append_profile_kind_by_id,
                    &snapshot_hot_profile_ids,
                    min_bucket_events,
                );
                let provider_money_claim_blocker = live_store_provider_money_claim_blocker(
                    &provider_evidence_artifacts,
                    &future_shadow_billing_request,
                )
                .to_owned();
                let (
                    append_known_profile_kind_count,
                    append_observable_known_profile_count,
                    append_hidden_state_known_profile_count,
                ) = live_store_known_profile_kind_counts(&append_profile_kind_by_id);
                let (
                    clean_product_hot_calls,
                    clean_product_hot_tokens,
                    clean_product_hot_cost_microusd,
                ) = live_store_product_hot_clean_credit_totals(
                    &product_hot_score_only_credit_rows,
                    &product_hot_score_only_quarantined_profile_ids,
                );
                let product_hot_score_only_unique_cpu_accepts_over_exact_cache =
                    clean_product_hot_calls;
                let product_hot_score_only_tokens_saved = clean_product_hot_tokens;
                let product_hot_score_only_cost_saved_microusd = clean_product_hot_cost_microusd;
                let append_exact_cache_calls_saved_milli_over_parsed_rows = live_store_milli(
                    append_denominator.exact_cache_hits as u64,
                    append_parsed_rows as u64,
                );
                let append_exact_cache_tokens_saved_milli_over_total = live_store_milli(
                    append_denominator.exact_cache_tokens,
                    append_denominator.total_tokens,
                );
                let append_exact_cache_cost_saved_milli_over_total = live_store_milli(
                    append_denominator.exact_cache_cost_microusd,
                    append_denominator.total_cost_microusd,
                );
                let append_active_clean_calls_saved_milli_over_parsed_rows =
                    live_store_milli(active_clean_calls_saved as u64, append_parsed_rows as u64);
                let append_active_clean_tokens_saved_milli_over_total =
                    live_store_milli(active_clean_tokens_saved, append_denominator.total_tokens);
                let append_active_clean_cost_saved_milli_over_total = live_store_milli(
                    append_eval.cost_saved_microusd,
                    append_denominator.total_cost_microusd,
                );
                let append_combined_calls_saved_milli_over_parsed_rows = live_store_milli(
                    append_denominator
                        .exact_cache_hits
                        .saturating_add(active_clean_calls_saved) as u64,
                    append_parsed_rows as u64,
                );
                let append_combined_tokens_saved_milli_over_total = live_store_milli(
                    append_denominator
                        .exact_cache_tokens
                        .saturating_add(active_clean_tokens_saved),
                    append_denominator.total_tokens,
                );
                let append_combined_cost_saved_milli_over_total = live_store_milli(
                    append_denominator
                        .exact_cache_cost_microusd
                        .saturating_add(append_eval.cost_saved_microusd),
                    append_denominator.total_cost_microusd,
                );
                let append_hidden_state_only_calls_saved_milli_over_parsed_rows = live_store_milli(
                    append_hidden_state_only_unique_cpu_accepts_over_exact_cache as u64,
                    append_parsed_rows as u64,
                );
                let append_hidden_state_only_tokens_saved_milli_over_total = live_store_milli(
                    append_hidden_state_only_tokens_saved,
                    append_denominator.total_tokens,
                );
                let append_hidden_state_only_cost_saved_milli_over_total = live_store_milli(
                    append_hidden_state_only_cost_saved_microusd,
                    append_denominator.total_cost_microusd,
                );
                let append_compression_claim_blocker = live_store_append_compression_claim_blocker(
                    append_parsed_rows,
                    append_eval.false_accepts,
                    append_eval.local_accept_events,
                    product_hot_score_only_unique_cpu_accepts_over_exact_cache,
                    product_hot_score_only_tokens_saved,
                    token_cost_denominator_present,
                    snapshot_hot_runtime_available,
                    snapshot_product_hot_runtime_source_claim_ready,
                    product_hot_score_only_post_quarantine_false_accepts,
                    live_store_append_compression_claim_min_rows(),
                )
                .to_owned();
                let append_compression_claim_allowed = append_compression_claim_blocker == "none";
                let append_clean_suffix_claim_blocker =
                    live_store_append_compression_claim_blocker(
                        append_clean_suffix_rows,
                        append_clean_suffix_false_accepts,
                        append_clean_suffix_local_accept_events,
                        append_clean_suffix_unique_cpu_accepts_over_exact_cache,
                        append_clean_suffix_tokens_saved,
                        token_cost_denominator_present,
                        snapshot_hot_runtime_available,
                        snapshot_product_hot_runtime_source_claim_ready,
                        0,
                        live_store_append_compression_claim_min_rows(),
                    )
                    .to_owned();
                let append_clean_suffix_claim_allowed = append_clean_suffix_claim_blocker == "none";
                let stable_decision_log_claim_blocker =
                    live_store_append_compression_claim_blocker(
                        stable_decision_log_window.rows,
                        stable_decision_log_window.false_accepts,
                        stable_decision_log_window.local_accept_events,
                        stable_decision_log_window.unique_cpu_accepts_over_exact_cache,
                        stable_decision_log_window.tokens_saved,
                        stable_decision_log_window.total_tokens > 0
                            && stable_decision_log_window.total_cost_microusd > 0,
                        snapshot_hot_runtime_available,
                        snapshot_product_hot_runtime_source_claim_ready,
                        0,
                        live_store_append_compression_claim_min_rows(),
                    )
                    .to_owned();
                let stable_decision_log_claim_allowed = stable_decision_log_claim_blocker == "none";
                let stable_decision_log_clean_suffix_claim_blocker =
                    live_store_append_compression_claim_blocker(
                        stable_decision_log_clean_suffix.window.rows,
                        stable_decision_log_clean_suffix.window.false_accepts,
                        stable_decision_log_clean_suffix.window.local_accept_events,
                        stable_decision_log_clean_suffix
                            .window
                            .unique_cpu_accepts_over_exact_cache,
                        stable_decision_log_clean_suffix.window.tokens_saved,
                        stable_decision_log_clean_suffix.window.total_tokens > 0
                            && stable_decision_log_clean_suffix.window.total_cost_microusd > 0,
                        snapshot_hot_runtime_available,
                        snapshot_product_hot_runtime_source_claim_ready,
                        0,
                        live_store_append_compression_claim_min_rows(),
                    )
                    .to_owned();
                let stable_decision_log_clean_suffix_claim_allowed =
                    stable_decision_log_clean_suffix_claim_blocker == "none";
                let stable_decision_log_clean_suffix_rows_to_min =
                    live_store_append_compression_claim_min_rows()
                        .saturating_sub(stable_decision_log_clean_suffix.window.rows);
                let stable_clean_token_compression_saved_milli = live_store_milli(
                    stable_decision_log_clean_suffix.window.tokens_saved,
                    stable_decision_log_clean_suffix.window.total_tokens,
                );
                let stable_serving_cpu_claim_blocker =
                    live_store_serving_cpu_compression_claim_blocker(
                        stable_serving_cpu_window.rows,
                        stable_serving_cpu_window.false_accepts,
                        stable_serving_cpu_window.local_accept_events,
                        stable_serving_cpu_window.unique_cpu_accepts_over_exact_cache,
                        stable_serving_cpu_window.tokens_saved,
                        stable_serving_cpu_window.total_tokens > 0
                            && stable_serving_cpu_window.total_cost_microusd > 0,
                        snapshot_hot_runtime_available,
                        snapshot_product_hot_runtime_source_claim_ready,
                        live_store_append_compression_claim_min_rows(),
                    )
                    .to_owned();
                let stable_serving_cpu_claim_allowed = stable_serving_cpu_claim_blocker == "none";
                let stable_serving_cpu_clean_suffix_claim_blocker =
                    live_store_serving_cpu_compression_claim_blocker(
                        stable_serving_cpu_clean_suffix.window.rows,
                        stable_serving_cpu_clean_suffix.window.false_accepts,
                        stable_serving_cpu_clean_suffix.window.local_accept_events,
                        stable_serving_cpu_clean_suffix
                            .window
                            .unique_cpu_accepts_over_exact_cache,
                        stable_serving_cpu_clean_suffix.window.tokens_saved,
                        stable_serving_cpu_clean_suffix.window.total_tokens > 0
                            && stable_serving_cpu_clean_suffix.window.total_cost_microusd > 0,
                        snapshot_hot_runtime_available,
                        snapshot_product_hot_runtime_source_claim_ready,
                        live_store_append_compression_claim_min_rows(),
                    )
                    .to_owned();
                let stable_serving_cpu_clean_suffix_claim_allowed =
                    stable_serving_cpu_clean_suffix_claim_blocker == "none";
                let stable_serving_cpu_clean_suffix_saved_milli = live_store_milli(
                    stable_serving_cpu_clean_suffix.window.tokens_saved,
                    stable_serving_cpu_clean_suffix.window.total_tokens,
                );
                let miner_saturation_snapshot = LiveStoreMinerSaturationSnapshot {
                    append_parsed_rows,
                    score_events: append_eval.score_events,
                    unique_cpu_accepts_over_exact_cache: append_eval
                        .unique_cpu_accepts_over_exact_cache,
                    tokens_saved: append_eval.tokens_saved,
                    false_accepts: append_eval.false_accepts,
                    bucket_count: summary.bucket_count,
                    active_bucket_count: summary.active_bucket_count,
                    refinement_count: bucket_policy.refinement_count,
                    quarantined_profile_count: product_hot_score_only_quarantined_profile_ids.len(),
                };
                miner_saturation.observe_heartbeat(miner_saturation_snapshot);
                let base_adaptive_idle_sleep_ms = if idle_sleep_ms >= 1_000 {
                    idle_sleep_ms
                } else {
                    idle_sleep_ms
                        .saturating_mul(1 + idle_elapsed_ms / 1_000)
                        .min(1_000)
                        .max(idle_sleep_ms)
                };
                let max_remaining_idle_ms =
                    (max_idle_ms > 0).then(|| max_idle_ms.saturating_sub(idle_elapsed_ms));
                planned_idle_sleep_ms = Some(if miner_saturation_control_enabled {
                    miner_saturation.select_sleep_ms(
                        base_adaptive_idle_sleep_ms,
                        miner_saturation_sleep_ms,
                        miner_saturation_min_idle_heartbeats,
                        max_remaining_idle_ms,
                    )
                } else {
                    miner_saturation.select_sleep_ms(
                        base_adaptive_idle_sleep_ms,
                        base_adaptive_idle_sleep_ms,
                        usize::MAX,
                        max_remaining_idle_ms,
                    )
                });
                let miner_saturation_last_snapshot = miner_saturation.last_snapshot();
                let snapshot = PhaseStreamHotPathDaemonAppendLiveTailReport {
                    report_kind: "phase_stream_hot_path_daemon_append_live_tail_v1",
                    architecture_versions: live_store_architecture_versions(),
                    mode: "append_file_tail_follow_score_before_update_shadow_only",
                    snapshot_in_progress: true,
                    decision_log_path: decision_log_path.display().to_string(),
                    watermark_trace_path: watermark_trace_path.display().to_string(),
                    append_tail_path: append_tail_path.display().to_string(),
                    cells,
                    min_bucket_events,
                    idle_sleep_ms,
                    max_idle_ms,
                    max_append_events,
                    idle_elapsed_ms,
                    miner_saturation_control_enabled,
                    miner_saturation_min_idle_heartbeats,
                    miner_saturation_sleep_ms,
                    miner_saturation_idle_heartbeats: miner_saturation.idle_heartbeats(),
                    miner_saturation_active: miner_saturation.active(),
                    miner_saturation_sleep_events: miner_saturation.sleep_events(),
                    miner_saturation_last_sleep_ms: miner_saturation.last_sleep_ms(),
                    miner_saturation_last_append_parsed_rows: miner_saturation_last_snapshot
                        .append_parsed_rows,
                    miner_saturation_last_score_events: miner_saturation_last_snapshot.score_events,
                    miner_saturation_last_unique_cpu_accepts_over_exact_cache:
                        miner_saturation_last_snapshot.unique_cpu_accepts_over_exact_cache,
                    miner_saturation_last_tokens_saved: miner_saturation_last_snapshot.tokens_saved,
                    miner_saturation_last_false_accepts: miner_saturation_last_snapshot
                        .false_accepts,
                    miner_saturation_last_bucket_count: miner_saturation_last_snapshot.bucket_count,
                    miner_saturation_last_active_bucket_count: miner_saturation_last_snapshot
                        .active_bucket_count,
                    miner_saturation_last_refinement_count: miner_saturation_last_snapshot
                        .refinement_count,
                    miner_saturation_last_quarantined_profile_count: miner_saturation_last_snapshot
                        .quarantined_profile_count,
                    miner_active_batch_rows,
                    miner_active_batch_sleep_ms,
                    miner_active_batch_sleep_events,
                    cold_artifact_refresh_interval_secs:
                        DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_COLD_REFRESH_SECS,
                    cold_artifact_refresh_count,
                    cold_artifact_refresh_used_this_snapshot: cold_artifact_refresh_due,
                    cold_artifact_refresh_append_rows,
                    decision_log_flush_interval_rows:
                        DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_DECISION_FLUSH_ROWS,
                    decision_log_flush_count,
                    decision_log_pending_rows,
                    start_at_end: true,
                    watermark_total_rows,
                    watermark_parsed_rows,
                    watermark_skipped_no_verifier_label,
                    watermark_skipped_no_safe_atoms,
                    append_history_warm_total_rows,
                    append_history_warm_parsed_rows,
                    append_history_warm_skipped_no_verifier_label,
                    append_history_warm_skipped_no_safe_atoms,
                    tail_total_lines_seen: append_total_lines_seen,
                    append_parsed_rows,
                    append_skipped_no_verifier_label,
                    append_skipped_no_safe_atoms,
                    append_hot_view_available_events,
                    append_route_index_missing_events,
                    append_route_index_missing_before_first_score,
                    append_route_index_missing_after_first_score,
                    append_auto_subcenter_observe_events,
                    append_auto_subcenter_throttled_events,
                    append_hidden_state_subcenter_observe_events,
                    miner_discovery_sample_permille,
                    miner_clean_hot_runtime_throttle_events,
                    append_known_profile_kind_count,
                    append_observable_known_profile_count,
                    append_hidden_state_known_profile_count,
                    append_scoring_started,
                    append_score_events_before_update: append_eval.score_events,
                    append_score_candidate_events: append_eval.score_candidate_events,
                    append_observable_score_candidate_events,
                    append_hidden_state_score_candidate_events,
                    append_unknown_profile_score_candidate_events,
                    append_verifier_required_events: append_eval.verifier_required_events,
                    append_local_accept_events: append_eval.local_accept_events,
                    append_unique_cpu_accepts_over_exact_cache: append_eval
                        .unique_cpu_accepts_over_exact_cache,
                    append_observable_unique_cpu_accepts_over_exact_cache,
                    append_hidden_state_unique_cpu_accepts_over_exact_cache,
                    append_unknown_profile_unique_cpu_accepts_over_exact_cache,
                    append_profile_attribution_overlap_accepts,
                    append_observable_only_unique_cpu_accepts_over_exact_cache,
                    append_hidden_state_only_unique_cpu_accepts_over_exact_cache,
                    append_mixed_profile_unique_cpu_accepts_over_exact_cache,
                    append_unknown_only_unique_cpu_accepts_over_exact_cache,
                    append_tokens_saved: append_eval.tokens_saved,
                    append_observable_tokens_saved,
                    append_hidden_state_tokens_saved,
                    append_unknown_profile_tokens_saved,
                    append_observable_only_tokens_saved,
                    append_hidden_state_only_tokens_saved,
                    append_mixed_profile_tokens_saved,
                    append_unknown_only_tokens_saved,
                    append_observable_only_cost_saved_microusd,
                    append_hidden_state_only_cost_saved_microusd,
                    append_mixed_profile_cost_saved_microusd,
                    append_unknown_only_cost_saved_microusd,
                    append_cost_saved_microusd: append_eval.cost_saved_microusd,
                    append_exact_cache_calls_saved_milli_over_parsed_rows,
                    append_exact_cache_tokens_saved_milli_over_total,
                    append_exact_cache_cost_saved_milli_over_total,
                    append_active_clean_calls_saved_milli_over_parsed_rows,
                    append_active_clean_tokens_saved_milli_over_total,
                    append_active_clean_cost_saved_milli_over_total,
                    append_combined_calls_saved_milli_over_parsed_rows,
                    append_combined_tokens_saved_milli_over_total,
                    append_combined_cost_saved_milli_over_total,
                    append_hidden_state_only_calls_saved_milli_over_parsed_rows,
                    append_hidden_state_only_tokens_saved_milli_over_total,
                    append_hidden_state_only_cost_saved_milli_over_total,
                    append_cost_estimate_used: append_estimated_cost_events > 0,
                    append_estimated_cost_events,
                    append_estimated_total_cost_microusd,
                    append_estimated_cost_saved_microusd,
                    append_false_accepts: append_eval.false_accepts,
                    append_compression_claim_min_rows: live_store_append_compression_claim_min_rows(
                    ),
                    append_compression_claim_allowed,
                    append_compression_claim_blocker,
                    append_clean_suffix_rows,
                    append_clean_suffix_score_events,
                    append_clean_suffix_unique_cpu_accepts_over_exact_cache,
                    append_clean_suffix_tokens_saved,
                    append_clean_suffix_cost_saved_microusd,
                    append_clean_suffix_false_accepts,
                    append_clean_suffix_local_accept_events,
                    append_clean_suffix_last_quarantine_row_index,
                    append_clean_suffix_claim_allowed,
                    append_clean_suffix_claim_blocker,
                    stable_decision_log_architecture_key: architecture_version_key.clone(),
                    stable_decision_log_rows: stable_decision_log_window.rows,
                    stable_decision_log_score_candidate_events: stable_decision_log_window
                        .score_candidate_events,
                    stable_decision_log_unique_cpu_accepts_over_exact_cache:
                        stable_decision_log_window.unique_cpu_accepts_over_exact_cache,
                    stable_decision_log_tokens_saved: stable_decision_log_window.tokens_saved,
                    stable_decision_log_cost_saved_microusd: stable_decision_log_window
                        .cost_saved_microusd,
                    stable_decision_log_false_accepts: stable_decision_log_window.false_accepts,
                    stable_decision_log_local_accept_events: stable_decision_log_window
                        .local_accept_events,
                    stable_decision_log_total_tokens: stable_decision_log_window.total_tokens,
                    stable_decision_log_total_cost_microusd: stable_decision_log_window
                        .total_cost_microusd,
                    stable_decision_log_claim_allowed,
                    stable_decision_log_claim_blocker,
                    stable_decision_log_clean_suffix_rows: stable_decision_log_clean_suffix
                        .window
                        .rows,
                    stable_decision_log_clean_suffix_score_candidate_events:
                        stable_decision_log_clean_suffix
                            .window
                            .score_candidate_events,
                    stable_decision_log_clean_suffix_unique_cpu_accepts_over_exact_cache:
                        stable_decision_log_clean_suffix
                            .window
                            .unique_cpu_accepts_over_exact_cache,
                    stable_decision_log_clean_suffix_tokens_saved: stable_decision_log_clean_suffix
                        .window
                        .tokens_saved,
                    stable_decision_log_clean_suffix_cost_saved_microusd:
                        stable_decision_log_clean_suffix.window.cost_saved_microusd,
                    stable_decision_log_clean_suffix_false_accepts:
                        stable_decision_log_clean_suffix.window.false_accepts,
                    stable_decision_log_clean_suffix_local_accept_events:
                        stable_decision_log_clean_suffix.window.local_accept_events,
                    stable_decision_log_clean_suffix_total_tokens: stable_decision_log_clean_suffix
                        .window
                        .total_tokens,
                    stable_decision_log_clean_suffix_total_cost_microusd:
                        stable_decision_log_clean_suffix.window.total_cost_microusd,
                    stable_decision_log_clean_suffix_last_quarantine_row_index:
                        stable_decision_log_clean_suffix.last_quarantine_row_index,
                    stable_decision_log_clean_suffix_min_rows:
                        live_store_append_compression_claim_min_rows(),
                    stable_decision_log_clean_suffix_rows_to_min,
                    stable_decision_log_clean_suffix_claim_allowed,
                    stable_decision_log_clean_suffix_claim_blocker:
                        stable_decision_log_clean_suffix_claim_blocker.clone(),
                    stable_clean_token_compression_claim_allowed:
                        stable_decision_log_clean_suffix_claim_allowed,
                    stable_clean_token_compression_claim_blocker:
                        stable_decision_log_clean_suffix_claim_blocker,
                    stable_clean_token_compression_unique_cpu_accepts_over_exact_cache:
                        stable_decision_log_clean_suffix
                            .window
                            .unique_cpu_accepts_over_exact_cache,
                    stable_clean_token_compression_saved_tokens: stable_decision_log_clean_suffix
                        .window
                        .tokens_saved,
                    stable_clean_token_compression_total_tokens: stable_decision_log_clean_suffix
                        .window
                        .total_tokens,
                    stable_clean_token_compression_saved_milli,
                    stable_clean_token_compression_false_accepts: stable_decision_log_clean_suffix
                        .window
                        .false_accepts,
                    stable_serving_cpu_rows: stable_serving_cpu_window.rows,
                    stable_serving_cpu_score_candidate_events: stable_serving_cpu_window
                        .score_candidate_events,
                    stable_serving_cpu_local_accept_events: stable_serving_cpu_window
                        .local_accept_events,
                    stable_serving_cpu_unique_cpu_accepts_over_exact_cache:
                        stable_serving_cpu_window.unique_cpu_accepts_over_exact_cache,
                    stable_serving_cpu_tokens_saved: stable_serving_cpu_window.tokens_saved,
                    stable_serving_cpu_total_tokens: stable_serving_cpu_window.total_tokens,
                    stable_serving_cpu_false_accepts: stable_serving_cpu_window.false_accepts,
                    stable_serving_cpu_claim_allowed,
                    stable_serving_cpu_claim_blocker,
                    stable_serving_cpu_clean_suffix_rows: stable_serving_cpu_clean_suffix
                        .window
                        .rows,
                    stable_serving_cpu_clean_suffix_score_candidate_events:
                        stable_serving_cpu_clean_suffix
                            .window
                            .score_candidate_events,
                    stable_serving_cpu_clean_suffix_local_accept_events:
                        stable_serving_cpu_clean_suffix.window.local_accept_events,
                    stable_serving_cpu_clean_suffix_unique_cpu_accepts_over_exact_cache:
                        stable_serving_cpu_clean_suffix
                            .window
                            .unique_cpu_accepts_over_exact_cache,
                    stable_serving_cpu_clean_suffix_tokens_saved: stable_serving_cpu_clean_suffix
                        .window
                        .tokens_saved,
                    stable_serving_cpu_clean_suffix_total_tokens: stable_serving_cpu_clean_suffix
                        .window
                        .total_tokens,
                    stable_serving_cpu_clean_suffix_false_accepts: stable_serving_cpu_clean_suffix
                        .window
                        .false_accepts,
                    stable_serving_cpu_clean_suffix_saved_milli,
                    stable_serving_cpu_clean_suffix_claim_allowed,
                    stable_serving_cpu_clean_suffix_claim_blocker,
                    append_total_tokens: append_denominator.total_tokens,
                    append_total_cost_microusd: append_denominator.total_cost_microusd,
                    append_exact_cache_hits: append_denominator.exact_cache_hits,
                    append_exact_cache_tokens: append_denominator.exact_cache_tokens,
                    append_exact_cache_cost_microusd: append_denominator.exact_cache_cost_microusd,
                    append_non_exact_rows: append_denominator.non_exact_rows,
                    online_bucket_count: summary.bucket_count,
                    active_bucket_count: summary.active_bucket_count,
                    shadow_ready_bucket_count: summary.shadow_ready_bucket_count,
                    candidate_bucket_count: summary.candidate_bucket_count,
                    clean_candidate_profile_ids,
                    clean_candidate_quarantined_profile_ids,
                    clean_candidate_exportable_profile_ids,
                    clean_candidate_reports,
                    rejected_bucket_count: summary.rejected_bucket_count,
                    live_route_count: store.route_count(),
                    live_route_bucket_count: store.route_bucket_count(),
                    adaptive_refinement_count: bucket_policy.refinement_count,
                    max_bucket_refinement_depth: bucket_policy.max_depth(),
                    product_hot_score_only_registry_path: product_hot_registry_path_report.clone(),
                    product_hot_score_only_runtime_source: product_hot_registry_source_report
                        .clone(),
                    product_hot_score_only_runtime_loaded: product_hot_registry_runtime.is_some(),
                    product_hot_score_only_runtime_active: product_hot_registry_runtime
                        .as_ref()
                        .is_some_and(|bundle| {
                            live_store_active_hot_profile_count(
                                &bundle.hot_runtime,
                                &product_hot_score_only_quarantined_profile_ids,
                            ) > 0
                        }),
                    product_hot_score_only_active_manifest_disabled:
                        product_hot_score_only_active_manifest_disabled,
                    product_hot_score_only_active_manifest_disable_reason:
                        product_hot_score_only_active_manifest_disable_reason.clone(),
                    product_hot_score_only_quarantined:
                        !product_hot_score_only_quarantined_profile_ids.is_empty(),
                    product_hot_score_only_quarantine_reason:
                        product_hot_score_only_quarantine_reason.clone(),
                    product_hot_score_only_quarantine_false_accepts,
                    product_hot_score_only_post_quarantine_score_candidate_events,
                    product_hot_score_only_post_quarantine_false_accepts,
                    product_hot_phase_trust_filtered_events,
                    product_hot_score_only_unique_cpu_accepts_over_exact_cache,
                    product_hot_score_only_tokens_saved,
                    product_hot_score_only_cost_saved_microusd,
                    active_clean_calls_saved,
                    active_clean_tokens_saved,
                    lost_calls_due_to_quarantine,
                    lost_tokens_due_to_quarantine,
                    quarantine_recovery_discovery_events,
                    quarantine_recovery_discovery_tokens,
                    quarantine_recovery_auto_subcenter_observe_events,
                    product_hot_score_only_active_profile_count: product_hot_registry_runtime
                        .as_ref()
                        .map_or(0, |bundle| {
                            live_store_active_hot_profile_count(
                                &bundle.hot_runtime,
                                &product_hot_score_only_quarantined_profile_ids,
                            )
                        }),
                    product_hot_score_only_quarantined_profile_count:
                        product_hot_score_only_quarantined_profile_ids.len(),
                    product_hot_score_only_quarantined_profile_ids:
                        product_hot_score_only_quarantined_profile_ids
                            .iter()
                            .copied()
                            .collect(),
                    product_hot_score_only_quarantine_trace_id:
                        product_hot_score_only_quarantine_trace_id.clone(),
                    product_hot_score_only_quarantine_route_key:
                        product_hot_score_only_quarantine_route_key.clone(),
                    product_hot_score_only_quarantine_bucket_key:
                        product_hot_score_only_quarantine_bucket_key.clone(),
                    product_hot_score_only_auto_refinement_candidate_atoms:
                        product_hot_score_only_auto_refinement_candidate_atoms.clone(),
                    product_hot_score_only_auto_refinement_selected_atoms:
                        product_hot_score_only_auto_refinement_selected_atoms.clone(),
                    product_hot_score_only_route_count: product_hot_registry_runtime
                        .as_ref()
                        .map_or(0, |bundle| bundle.route_table.route_count()),
                    product_hot_score_only_profile_count: product_hot_registry_runtime
                        .as_ref()
                        .map_or(0, |bundle| bundle.hot_runtime.profile_count()),
                    product_hot_score_only_package_bytes: product_hot_registry_runtime
                        .as_ref()
                        .map_or(0, |bundle| bundle.package_bytes),
                    final_hot_runtime_available: snapshot_hot_runtime_available,
                    final_hot_route_ids: snapshot_hot_route_ids,
                    final_hot_profile_ids: snapshot_hot_profile_ids,
                    final_hot_route_count: snapshot_hot_route_count,
                    final_hot_profile_count: snapshot_hot_profile_count,
                    final_hot_route_profile_edges: snapshot_hot_route_profile_edges,
                    hot_scratch_rebuilds,
                    hot_scratch_candidate_capacity: hot_scratch
                        .as_ref()
                        .map_or(0, PhaseCenterHotScratch::candidate_capacity),
                    hot_scratch_bytes_estimate: hot_scratch
                        .as_ref()
                        .map_or(0, PhaseCenterHotScratch::bytes_estimate),
                    runtime_budget,
                    product_hot_budget_passed,
                    warm_miner_budget_passed,
                    warm_miner_budget_blocker,
                    exact_cache_overlap_excluded,
                    token_cost_denominator_present,
                    verifier_binding_bound: verifier_binding.is_bound(),
                    candidate_package_count: candidate_package_reports.len(),
                    candidate_package_dir: candidate_package_dir.display().to_string(),
                    candidate_packages: candidate_package_reports.clone(),
                    clean_promotion_manifest_path: clean_promotion_manifest_path
                        .display()
                        .to_string(),
                    clean_promotion_package_dir: clean_promotion_package_dir.display().to_string(),
                    clean_promotion_manifest_written,
                    call_token_promotion_manifest_path: call_token_promotion_manifest_path
                        .display()
                        .to_string(),
                    call_token_promotion_package_dir: call_token_promotion_package_dir
                        .display()
                        .to_string(),
                    call_token_promotion_manifest_written,
                    future_shadow: future_shadow.clone(),
                    future_shadow_route_level_scoring_used: true,
                    future_shadow_billing_request_path: future_shadow_billing_request_path
                        .display()
                        .to_string(),
                    future_shadow_billing_request_rows: future_shadow_billing_request.rows,
                    future_shadow_billing_request_tokens: future_shadow_billing_request.tokens,
                    future_shadow_billing_request_current_cost_microusd:
                        future_shadow_billing_request.current_cost_microusd,
                    future_shadow_billing_request_ready_for_external_provider_evidence:
                        future_shadow_billing_request.ready_for_external_provider_evidence,
                    provider_evidence_artifacts: provider_evidence_artifacts.clone(),
                    provider_export_drop_path: provider_evidence_artifacts
                        .provider_export_drop_path
                        .clone(),
                    provider_export_present: provider_evidence_artifacts.provider_export_present,
                    provider_evidence_chain_report_path: provider_evidence_artifacts
                        .evidence_chain_report_path
                        .clone(),
                    provider_billing_capture_contract_ready: provider_evidence_artifacts
                        .capture_contract_ready,
                    provider_market_money_claim_allowed: provider_evidence_artifacts
                        .market_money_claim_allowed,
                    provider_money_claim_blocker,
                    tail_follow_mode_used: true,
                    cold_adapter_json_used: true,
                    cold_adapter_strings_used: true,
                    timed_lane_json_used: false,
                    timed_lane_string_route_used: false,
                    timed_lane_btreemap_used: false,
                    timed_lane_file_io_used: false,
                    direct_mutable_store_used: true,
                    score_before_update_used: true,
                    quarantine_nwpc_checkpoint_compile_used: cold_artifact_refresh_due,
                    registry_mutation_enabled: false,
                    cpu_profile_registry_write_enabled: false,
                    serving_profile_artifact_written: false,
                    product_promotion_enabled: false,
                    local_accept_enabled: false,
                    market_money_claim_allowed: false,
                    forbidden_flags: super::ForbiddenFlags {
                        target_id_used: false,
                        proof_rule_id_authority_used: false,
                        concrete_x_lookup_used: false,
                        manual_local_out_t_used: false,
                        hidden_frame_id_or_bind_x_used: false,
                        legacy_backend_used: false,
                    },
                    verdict: "HOT_PATH_DAEMON_APPEND_LIVE_TAIL_RUNNING",
                    blocker: "append_live_tail_running".to_owned(),
                    boundary: "heartbeat snapshot only: watermark events initialize PhaseCenterLiveOperatorStore, then appended events are scored before update and observed; verifier-bound quarantine .nwpc candidates are exported on the cold/report side only; this snapshot does not mutate registry, write serving profiles, promote, local_accept, or claim money",
                };
                super::write_json_file(&report_path, &snapshot)?;
                println!(
                    "append_live_tail_heartbeat: parsed={} scored={} buckets={} active={} idle_ms={} miner_saturated={} sleep_ms={}",
                    append_parsed_rows,
                    append_eval.score_events,
                    summary.bucket_count,
                    summary.active_bucket_count,
                    idle_elapsed_ms,
                    miner_saturation.active(),
                    miner_saturation.last_sleep_ms()
                );
                last_heartbeat = Instant::now();
            }
            let adaptive_idle_sleep_ms = if let Some(planned_idle_sleep_ms) = planned_idle_sleep_ms
            {
                planned_idle_sleep_ms
            } else {
                let base_adaptive_idle_sleep_ms = if idle_sleep_ms >= 1_000 {
                    idle_sleep_ms
                } else {
                    idle_sleep_ms
                        .saturating_mul(1 + idle_elapsed_ms / 1_000)
                        .min(1_000)
                        .max(idle_sleep_ms)
                };
                let max_remaining_idle_ms =
                    (max_idle_ms > 0).then(|| max_idle_ms.saturating_sub(idle_elapsed_ms));
                if miner_saturation_control_enabled {
                    miner_saturation.select_sleep_ms(
                        base_adaptive_idle_sleep_ms,
                        miner_saturation_sleep_ms,
                        miner_saturation_min_idle_heartbeats,
                        max_remaining_idle_ms,
                    )
                } else {
                    miner_saturation.select_sleep_ms(
                        base_adaptive_idle_sleep_ms,
                        base_adaptive_idle_sleep_ms,
                        usize::MAX,
                        max_remaining_idle_ms,
                    )
                }
            };
            thread::sleep(Duration::from_millis(adaptive_idle_sleep_ms));
            idle_elapsed_ms = idle_elapsed_ms.saturating_add(adaptive_idle_sleep_ms);
            continue;
        }
        idle_elapsed_ms = 0;
        append_total_lines_seen += 1;
        if miner_active_batch_rows > 0 && miner_active_batch_sleep_ms > 0 {
            miner_active_batch_rows_seen = miner_active_batch_rows_seen.saturating_add(1);
            if miner_active_batch_rows_seen >= miner_active_batch_rows {
                thread::sleep(Duration::from_millis(miner_active_batch_sleep_ms));
                miner_active_batch_sleep_events = miner_active_batch_sleep_events.saturating_add(1);
                miner_active_batch_rows_seen = 0;
            }
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse append live-tail source '{}' line {}: {error}",
                append_tail_path.display(),
                append_total_lines_seen
            )
        })?;
        let Some(verified_safe_accept) = row
            .get("verified_safe_accept")
            .and_then(serde_json::Value::as_bool)
        else {
            append_skipped_no_verifier_label += 1;
            continue;
        };
        let Some(mut adapter_event) = live_store_atom_event_from_row(
            &row,
            verified_safe_accept,
            &bucket_policy,
            &mut exact_cache_keys_seen,
        ) else {
            append_skipped_no_safe_atoms += 1;
            continue;
        };
        let estimated_cost_microusd =
            live_store_apply_tail_cost_estimate(&mut adapter_event, &price_config);
        let mut quarantine_recovery_split_basis =
            adapter_event.bucket_selector_candidate_atoms.clone();
        quarantine_recovery_split_basis.extend(adapter_event.auto_subcenter_atoms.iter().cloned());
        quarantine_recovery_split_basis.sort();
        quarantine_recovery_split_basis.dedup();
        let quarantine_recovery_atoms =
            hidden_state::live_store_quarantine_recovery_subcenter_atoms(
                &adapter_event.route_key,
                &adapter_event.auto_subcenter_atoms,
                &quarantine_recovery_split_basis,
                &product_hot_score_only_quarantined_profile_ids,
                DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_AUTO_SUBCENTER_ATOMS / 2,
            );
        if !quarantine_recovery_atoms.is_empty() {
            adapter_event
                .auto_subcenter_atoms
                .extend(quarantine_recovery_atoms);
            adapter_event.auto_subcenter_atoms.sort();
            adapter_event.auto_subcenter_atoms.dedup();
            adapter_event.auto_subcenter_bucket_ids =
                hidden_state::live_store_auto_subcenter_bucket_ids(
                    &adapter_event.route_key,
                    adapter_event.bucket_id,
                    &adapter_event.auto_subcenter_atoms,
                );
        }
        live_store_record_event_profile_kinds(&adapter_event, &mut append_profile_kind_by_id);
        if estimated_cost_microusd > 0 {
            append_estimated_cost_events = append_estimated_cost_events.saturating_add(1);
            append_estimated_total_cost_microusd =
                append_estimated_total_cost_microusd.saturating_add(estimated_cost_microusd);
        }
        append_denominator.total_tokens = append_denominator
            .total_tokens
            .saturating_add(adapter_event.tokens);
        append_denominator.total_cost_microusd = append_denominator
            .total_cost_microusd
            .saturating_add(adapter_event.cost_microusd);
        if adapter_event.exact_cache_hit {
            append_denominator.exact_cache_hits += 1;
            append_denominator.exact_cache_tokens = append_denominator
                .exact_cache_tokens
                .saturating_add(adapter_event.tokens);
            append_denominator.exact_cache_cost_microusd = append_denominator
                .exact_cache_cost_microusd
                .saturating_add(adapter_event.cost_microusd);
        } else {
            append_denominator.non_exact_rows += 1;
        }
        observe_live_store_future_shadow(
            &adapter_event,
            &mut frozen_candidates,
            &mut future_encoder,
            &mut future_shadow,
        )?;

        let mut decision_rows = Vec::new();
        let mut scored_before_update = false;
        let mut disable_product_hot_runtime_after_score = false;
        let mut rebuild_product_hot_survivors_after_score = false;
        let mut row_product_hot_phase_trust_filtered = false;
        let row_eval_score_events_before = append_eval.score_events;
        let row_eval_unique_accepts_before = append_eval.unique_cpu_accepts_over_exact_cache;
        let row_eval_tokens_saved_before = append_eval.tokens_saved;
        let row_eval_cost_saved_before = append_eval.cost_saved_microusd;
        let row_eval_false_accepts_before = append_eval.false_accepts;
        let row_eval_local_accepts_before = append_eval.local_accept_events;
        let relevant_online_bucket_ids = live_store_relevant_online_bucket_ids(&adapter_event);
        let route_wide_phase_transfer_allowed =
            live_store_route_wide_phase_transfer_allowed(&adapter_event);
        let mut online_shadow_ready_relevant_bucket_ids = Vec::<u32>::new();
        if product_hot_registry_runtime
            .as_ref()
            .filter(|bundle| {
                live_store_active_hot_profile_count(
                    &bundle.hot_runtime,
                    &product_hot_score_only_quarantined_profile_ids,
                ) > 0
            })
            .is_some_and(|bundle| {
                live_store_product_hot_route_index(bundle, &adapter_event, &row).is_none()
            })
        {
            if let Some(survivor_runtime) = live_store_clean_candidate_survivor_runtime_from_store(
                &store,
                cells,
                &report_path.with_file_name("live-store-clean-candidate-survivors.shadow-runtime"),
                &product_hot_score_only_quarantined_profile_ids,
                &relevant_online_bucket_ids,
                &append_profile_kind_by_id,
                min_bucket_events,
            )? {
                if live_store_product_hot_route_index(&survivor_runtime, &adapter_event, &row)
                    .is_some()
                {
                    product_hot_scratch = Some(
                        PhaseCenterHotScratch::new(
                            survivor_runtime.cells,
                            survivor_runtime.route_table.profile_edge_count().max(1),
                        )
                        .map_err(|error| {
                            format!(
                                "failed to build route-refreshed clean-survivor product-hot scratch: {error:?}"
                            )
                        })?,
                    );
                    product_hot_registry_path_report =
                        survivor_runtime.registry_path.display().to_string();
                    product_hot_registry_source_report =
                        "live_store_clean_candidate_survivors_route_refreshed".to_owned();
                    product_hot_registry_runtime = Some(survivor_runtime);
                }
            }
        }
        if let Some(bundle) = product_hot_registry_runtime.as_ref().filter(|bundle| {
            live_store_active_hot_profile_count(
                &bundle.hot_runtime,
                &product_hot_score_only_quarantined_profile_ids,
            ) > 0
        }) {
            append_hot_view_available_events += 1;
            let product_hot_route_index =
                live_store_product_hot_route_index(bundle, &adapter_event, &row);
            if let Some(route_index) = product_hot_route_index {
                let scratch = product_hot_scratch
                    .as_mut()
                    .expect("product-hot scratch is initialized");
                let decisions = bundle
                    .hot_runtime
                    .score_hot_request_candidates(
                        &bundle.route_table,
                        PhaseCenterHotRequest::new(route_index, &adapter_event.atom_ids),
                        scratch,
                    )
                    .map_err(|error| {
                        format!("failed append live-tail product-hot score: {error:?}")
                    })?;
                let relevant_product_hot_decisions = if route_wide_phase_transfer_allowed {
                    decisions.to_vec()
                } else {
                    live_store_relevant_bucket_decisions(&decisions, &relevant_online_bucket_ids)
                };
                let active_decisions = relevant_product_hot_decisions
                    .iter()
                    .copied()
                    .filter(|decision| {
                        !product_hot_score_only_quarantined_profile_ids
                            .contains(&decision.profile_id)
                            && live_store_product_hot_profile_phase_trusted(
                                &store,
                                decision.profile_id,
                                min_bucket_events,
                            )
                    })
                    .collect::<Vec<_>>();
                if relevant_product_hot_decisions.iter().any(|decision| {
                    decision.score_candidate
                        && !product_hot_score_only_quarantined_profile_ids
                            .contains(&decision.profile_id)
                        && !live_store_product_hot_profile_phase_trusted(
                            &store,
                            decision.profile_id,
                            min_bucket_events,
                        )
                }) {
                    row_product_hot_phase_trust_filtered = true;
                    product_hot_phase_trust_filtered_events =
                        product_hot_phase_trust_filtered_events.saturating_add(1);
                }
                let eval_decisions = live_store_union_score_candidate_decision(&active_decisions);
                let product_hot_quarantine_was_active =
                    !product_hot_score_only_quarantined_profile_ids.is_empty();
                let false_accepts_before = append_eval.false_accepts;
                let quarantined_score_candidate_seen =
                    relevant_product_hot_decisions.iter().any(|decision| {
                        decision.score_candidate
                            && product_hot_score_only_quarantined_profile_ids
                                .contains(&decision.profile_id)
                    });
                if quarantined_score_candidate_seen
                    && adapter_event.verified_safe_accept
                    && !adapter_event.exact_cache_hit
                {
                    lost_calls_due_to_quarantine = lost_calls_due_to_quarantine.saturating_add(1);
                    lost_tokens_due_to_quarantine =
                        lost_tokens_due_to_quarantine.saturating_add(adapter_event.tokens);
                }
                if !eval_decisions.is_empty() {
                    scored_before_update = true;
                    let attribution = live_store_profile_attribution(
                        &adapter_event,
                        &active_decisions,
                        &append_profile_kind_by_id,
                    );
                    live_store_update_profile_attribution_counters(
                        attribution,
                        &adapter_event,
                        &mut append_observable_score_candidate_events,
                        &mut append_hidden_state_score_candidate_events,
                        &mut append_unknown_profile_score_candidate_events,
                        &mut append_observable_unique_cpu_accepts_over_exact_cache,
                        &mut append_hidden_state_unique_cpu_accepts_over_exact_cache,
                        &mut append_unknown_profile_unique_cpu_accepts_over_exact_cache,
                        &mut append_profile_attribution_overlap_accepts,
                        &mut append_observable_only_unique_cpu_accepts_over_exact_cache,
                        &mut append_hidden_state_only_unique_cpu_accepts_over_exact_cache,
                        &mut append_mixed_profile_unique_cpu_accepts_over_exact_cache,
                        &mut append_unknown_only_unique_cpu_accepts_over_exact_cache,
                        &mut append_observable_tokens_saved,
                        &mut append_hidden_state_tokens_saved,
                        &mut append_unknown_profile_tokens_saved,
                        &mut append_observable_only_tokens_saved,
                        &mut append_hidden_state_only_tokens_saved,
                        &mut append_mixed_profile_tokens_saved,
                        &mut append_unknown_only_tokens_saved,
                        &mut append_observable_only_cost_saved_microusd,
                        &mut append_hidden_state_only_cost_saved_microusd,
                        &mut append_mixed_profile_cost_saved_microusd,
                        &mut append_unknown_only_cost_saved_microusd,
                    );
                    live_store_update_candidate_decision_eval(
                        adapter_event.verified_safe_accept,
                        adapter_event.exact_cache_hit,
                        adapter_event.tokens,
                        adapter_event.cost_microusd,
                        &eval_decisions,
                        &mut append_eval,
                    );
                    if adapter_event.verified_safe_accept && !adapter_event.exact_cache_hit {
                        let mut credit_profile_ids = active_decisions
                            .iter()
                            .filter(|decision| decision.score_candidate)
                            .map(|decision| decision.profile_id)
                            .collect::<Vec<_>>();
                        credit_profile_ids.sort_unstable();
                        credit_profile_ids.dedup();
                        if !credit_profile_ids.is_empty() {
                            product_hot_score_only_credit_rows.push(LiveStoreProductHotCreditRow {
                                profile_ids: credit_profile_ids,
                                tokens: adapter_event.tokens,
                                cost_microusd: adapter_event.cost_microusd,
                            });
                        }
                        active_clean_calls_saved = active_clean_calls_saved.saturating_add(1);
                        active_clean_tokens_saved =
                            active_clean_tokens_saved.saturating_add(adapter_event.tokens);
                        if adapter_event.cost_estimate_used {
                            append_estimated_cost_saved_microusd =
                                append_estimated_cost_saved_microusd
                                    .saturating_add(adapter_event.cost_microusd);
                        }
                    }
                }
                let row_added_false_accept = append_eval.false_accepts > false_accepts_before;
                if product_hot_quarantine_was_active && !eval_decisions.is_empty() {
                    product_hot_score_only_post_quarantine_score_candidate_events =
                        product_hot_score_only_post_quarantine_score_candidate_events
                            .saturating_add(1);
                    if !row_added_false_accept
                        && !adapter_event.verified_safe_accept
                        && !adapter_event.exact_cache_hit
                    {
                        product_hot_score_only_post_quarantine_false_accepts =
                            product_hot_score_only_post_quarantine_false_accepts.saturating_add(1);
                    }
                }
                if row_added_false_accept {
                    bucket_policy.observe_rejected_bucket(&adapter_event);
                    for decision in active_decisions
                        .iter()
                        .filter(|decision| decision.score_candidate)
                    {
                        product_hot_score_only_quarantined_profile_ids.insert(decision.profile_id);
                    }
                    product_hot_score_only_quarantine_false_accepts = append_eval.false_accepts;
                    product_hot_score_only_quarantine_reason =
                        "product_hot_shadow_false_accepts_nonzero".to_owned();
                    product_hot_score_only_quarantine_trace_id =
                        adapter_event.trace_id.clone().unwrap_or_default();
                    product_hot_score_only_quarantine_route_key = adapter_event.route_key.clone();
                    product_hot_score_only_quarantine_bucket_key = adapter_event.bucket_key.clone();
                    if product_hot_registry_source_report == "call_token_active_manifest"
                        || product_hot_registry_path_report
                            == call_token_active_manifest_path.display().to_string()
                    {
                        product_hot_score_only_active_manifest_disabled = true;
                        product_hot_score_only_active_manifest_disable_reason =
                            product_hot_score_only_quarantine_reason.clone();
                        disable_live_store_call_token_active_manifest(
                            &call_token_active_manifest_path,
                            &product_hot_score_only_quarantine_reason,
                            append_eval.false_accepts,
                            &product_hot_score_only_quarantine_trace_id,
                            &product_hot_score_only_quarantine_route_key,
                            &product_hot_score_only_quarantine_bucket_key,
                            &product_hot_score_only_quarantined_profile_ids,
                        )?;
                        product_hot_registry_path_report.clear();
                        product_hot_registry_source_report =
                            "disabled_after_live_false_accept".to_owned();
                        disable_product_hot_runtime_after_score = true;
                    }
                    let token_cost_denominator_present = append_denominator.total_tokens > 0
                        && append_denominator.total_cost_microusd > 0;
                    live_store_refresh_future_shadow_summary(
                        &mut future_shadow,
                        &frozen_candidates,
                        token_cost_denominator_present,
                        append_parsed_rows,
                        append_denominator.total_tokens,
                        append_denominator.total_cost_microusd,
                    )?;
                    refresh_live_store_call_token_promotion_manifest_summary_with_quarantine(
                        &mut future_shadow,
                        &frozen_candidates,
                        &product_hot_score_only_quarantined_profile_ids,
                    );
                    write_live_store_call_token_promotion_manifest_with_quarantine(
                        &call_token_promotion_manifest_path,
                        &call_token_promotion_package_dir,
                        &future_shadow,
                        &frozen_candidates,
                        &product_hot_score_only_quarantined_profile_ids,
                    )?;
                    write_live_store_clean_survivor_call_token_promotion_manifest(
                        &call_token_promotion_manifest_path,
                        &call_token_promotion_package_dir,
                        &store,
                        &frozen_candidates,
                        &product_hot_score_only_quarantined_profile_ids,
                        &append_profile_kind_by_id,
                        &mut future_shadow,
                    )?;
                    call_token_promotion_manifest_written = true;
                    rebuild_product_hot_survivors_after_score = true;
                    product_hot_score_only_auto_refinement_selected_atoms =
                        adapter_event.selected_bucket_atoms.clone();
                    let selected_atoms = adapter_event
                        .selected_bucket_atoms
                        .iter()
                        .cloned()
                        .collect::<BTreeSet<_>>();
                    product_hot_score_only_auto_refinement_candidate_atoms = adapter_event
                        .bucket_selector_candidate_atoms
                        .iter()
                        .filter(|atom| {
                            !selected_atoms.contains(*atom)
                                && live_store_false_accept_split_atom_refinement_blocker(atom)
                                    == "none"
                        })
                        .cloned()
                        .collect();
                    if product_hot_score_only_auto_refinement_candidate_atoms.is_empty() {
                        product_hot_score_only_auto_refinement_candidate_atoms =
                            live_store_auto_refinement_candidate_atoms_from_row(
                                &row,
                                &adapter_event.selected_bucket_atoms,
                            );
                    }
                }
                if !eval_decisions.is_empty() {
                    append_scoring_started = true;
                }
                decision_rows.extend(decisions
                    .iter()
                    .map(|decision| {
                        serde_json::json!({
                            "score_source": "product_hot_score_only",
                            "profile_id": decision.profile_id,
                            "margin_micro": decision.margin_micro,
                            "score_candidate": decision.score_candidate,
                            "verifier_required": decision.verifier_required,
                            "local_accept": decision.local_accept,
                            "product_hot_profile_quarantined": product_hot_score_only_quarantined_profile_ids.contains(&decision.profile_id)
                        })
                    })
                    .collect::<Vec<_>>());
                if rebuild_product_hot_survivors_after_score {
                    if let Some(survivor_runtime) =
                        live_store_clean_candidate_survivor_runtime_from_store(
                            &store,
                            cells,
                            &report_path.with_file_name(
                                "live-store-clean-candidate-survivors.shadow-runtime",
                            ),
                            &product_hot_score_only_quarantined_profile_ids,
                            &relevant_online_bucket_ids,
                            &append_profile_kind_by_id,
                            min_bucket_events,
                        )?
                    {
                        product_hot_scratch = Some(
                            PhaseCenterHotScratch::new(
                                survivor_runtime.cells,
                                survivor_runtime.route_table.profile_edge_count().max(1),
                            )
                            .map_err(|error| {
                                format!(
                                    "failed to rebuild clean-survivor product-hot scratch: {error:?}"
                                )
                            })?,
                        );
                        product_hot_registry_path_report =
                            survivor_runtime.registry_path.display().to_string();
                        product_hot_registry_source_report =
                            "live_store_clean_candidate_survivors".to_owned();
                        product_hot_registry_runtime = Some(survivor_runtime);
                        disable_product_hot_runtime_after_score = false;
                    }
                }
            } else {
                append_route_index_missing_events += 1;
                if append_scoring_started {
                    append_route_index_missing_after_first_score += 1;
                } else {
                    append_route_index_missing_before_first_score += 1;
                }
            }
        }
        if !scored_before_update {
            let has_candidate_subcenter = relevant_online_bucket_ids.iter().any(|bucket_id| {
                live_store_product_hot_subcenter_candidate_allowed(
                    &store,
                    &append_profile_kind_by_id,
                    &product_hot_score_only_quarantined_profile_ids,
                    *bucket_id,
                    min_bucket_events,
                )
            });
            let excluded_profile_ids = live_store_product_hot_excluded_profile_ids(
                &product_hot_score_only_quarantined_profile_ids,
                &append_profile_kind_by_id,
                has_candidate_subcenter,
            );
            if let Some((hot_runtime, route_table)) = store
                .candidate_hot_runtime_and_route_table_excluding_prioritized(
                    &excluded_profile_ids,
                    &relevant_online_bucket_ids,
                )
                .map_err(|error| {
                    format!("failed to build current append live-tail clean candidate hot view: {error:?}")
                })?
            {
                append_hot_view_available_events += 1;
                online_shadow_ready_relevant_bucket_ids = relevant_online_bucket_ids
                    .iter()
                    .copied()
                    .filter(|bucket_id| hot_runtime.resolve_profile_index(*bucket_id).is_some())
                    .collect();
                if let Some(route_index) = route_table.resolve_route_index(adapter_event.route_id) {
                    let required_candidate_capacity = route_table.profile_edge_count().max(1);
                    let scratch_rebuild_needed = hot_scratch.as_ref().is_none_or(|scratch| {
                        scratch.cells() != cells
                            || scratch.candidate_capacity() < required_candidate_capacity
                    });
                    if scratch_rebuild_needed {
                        hot_scratch = Some(
                            PhaseCenterHotScratch::new(cells, required_candidate_capacity)
                                .map_err(|error| {
                                    format!("failed to build append live-tail scratch: {error:?}")
                                })?,
                        );
                        hot_scratch_rebuilds = hot_scratch_rebuilds.saturating_add(1);
                    }
                    let scratch = hot_scratch
                        .as_mut()
                        .expect("append live-tail scratch is initialized");
                    let decisions = hot_runtime
                        .score_hot_request_candidates(
                            &route_table,
                            PhaseCenterHotRequest::new(route_index, &adapter_event.atom_ids),
                            scratch,
                        )
                        .map_err(|error| format!("failed append live-tail hot score: {error:?}"))?;
                    let decisions = if route_wide_phase_transfer_allowed {
                        decisions.to_vec()
                    } else {
                        live_store_relevant_bucket_decisions(&decisions, &relevant_online_bucket_ids)
                    }
                    .into_iter()
                    .filter(|decision| {
                        !product_hot_score_only_quarantined_profile_ids
                            .contains(&decision.profile_id)
                            && live_store_product_hot_profile_phase_trusted(
                                &store,
                                decision.profile_id,
                                min_bucket_events,
                            )
                    })
                    .collect::<Vec<_>>();
                    if decisions.iter().any(|decision| {
                        decision.score_candidate
                            && !product_hot_score_only_quarantined_profile_ids
                                .contains(&decision.profile_id)
                            && !live_store_product_hot_profile_phase_trusted(
                                &store,
                                decision.profile_id,
                                min_bucket_events,
                            )
                    }) {
                        row_product_hot_phase_trust_filtered = true;
                        product_hot_phase_trust_filtered_events =
                            product_hot_phase_trust_filtered_events.saturating_add(1);
                    }
                    scored_before_update = true;
                    let attribution = live_store_profile_attribution(
                        &adapter_event,
                        &decisions,
                        &append_profile_kind_by_id,
                    );
                    live_store_update_profile_attribution_counters(
                        attribution,
                        &adapter_event,
                        &mut append_observable_score_candidate_events,
                        &mut append_hidden_state_score_candidate_events,
                        &mut append_unknown_profile_score_candidate_events,
                        &mut append_observable_unique_cpu_accepts_over_exact_cache,
                        &mut append_hidden_state_unique_cpu_accepts_over_exact_cache,
                        &mut append_unknown_profile_unique_cpu_accepts_over_exact_cache,
                        &mut append_profile_attribution_overlap_accepts,
                        &mut append_observable_only_unique_cpu_accepts_over_exact_cache,
                        &mut append_hidden_state_only_unique_cpu_accepts_over_exact_cache,
                        &mut append_mixed_profile_unique_cpu_accepts_over_exact_cache,
                        &mut append_unknown_only_unique_cpu_accepts_over_exact_cache,
                        &mut append_observable_tokens_saved,
                        &mut append_hidden_state_tokens_saved,
                        &mut append_unknown_profile_tokens_saved,
                        &mut append_observable_only_tokens_saved,
                        &mut append_hidden_state_only_tokens_saved,
                        &mut append_mixed_profile_tokens_saved,
                        &mut append_unknown_only_tokens_saved,
                        &mut append_observable_only_cost_saved_microusd,
                        &mut append_hidden_state_only_cost_saved_microusd,
                        &mut append_mixed_profile_cost_saved_microusd,
                        &mut append_unknown_only_cost_saved_microusd,
                    );
                    live_store_update_candidate_decision_eval(
                        adapter_event.verified_safe_accept,
                        adapter_event.exact_cache_hit,
                        adapter_event.tokens,
                        adapter_event.cost_microusd,
                        &decisions,
                        &mut append_eval,
                    );
                    if decisions.iter().any(|decision| decision.score_candidate)
                        && adapter_event.verified_safe_accept
                        && !adapter_event.exact_cache_hit
                    {
                        active_clean_calls_saved = active_clean_calls_saved.saturating_add(1);
                        active_clean_tokens_saved =
                            active_clean_tokens_saved.saturating_add(adapter_event.tokens);
                        if adapter_event.cost_estimate_used {
                            append_estimated_cost_saved_microusd =
                                append_estimated_cost_saved_microusd
                                    .saturating_add(adapter_event.cost_microusd);
                        }
                    }
                    append_scoring_started = true;
                    decision_rows.extend(
                        decisions
                            .iter()
                            .map(|decision| {
                                serde_json::json!({
                                    "score_source": "online_store_candidate",
                                    "profile_id": decision.profile_id,
                                    "margin_micro": decision.margin_micro,
                                    "score_candidate": decision.score_candidate,
                                    "verifier_required": decision.verifier_required,
                                    "local_accept": decision.local_accept
                                })
                            })
                            .collect::<Vec<_>>(),
                    );
                } else {
                    append_route_index_missing_events += 1;
                    if append_scoring_started {
                        append_route_index_missing_after_first_score += 1;
                    } else {
                        append_route_index_missing_before_first_score += 1;
                    }
                }
            }
        }
        if disable_product_hot_runtime_after_score {
            product_hot_registry_runtime = None;
            product_hot_scratch = None;
        }

        let row_added_shadow_false_accept =
            append_eval.false_accepts > row_eval_false_accepts_before;
        let stable_clean_hot_runtime_available =
            product_hot_registry_runtime.as_ref().is_some_and(|bundle| {
                live_store_active_hot_profile_count(
                    &bundle.hot_runtime,
                    &product_hot_score_only_quarantined_profile_ids,
                ) > 0
                    && product_hot_score_only_post_quarantine_false_accepts == 0
                    && product_hot_score_only_quarantine_false_accepts == 0
            });
        let quarantine_recovery_discovery = product_hot_score_only_quarantined_profile_ids
            .contains(&adapter_event.bucket_id)
            || adapter_event
                .auto_subcenter_bucket_ids
                .iter()
                .any(|bucket_id| {
                    product_hot_score_only_quarantined_profile_ids.contains(bucket_id)
                });
        if quarantine_recovery_discovery {
            quarantine_recovery_discovery_events =
                quarantine_recovery_discovery_events.saturating_add(1);
            quarantine_recovery_discovery_tokens =
                quarantine_recovery_discovery_tokens.saturating_add(adapter_event.tokens);
        }
        let discovery_sampled = row_added_shadow_false_accept
            || quarantine_recovery_discovery
            || row_product_hot_phase_trust_filtered
            || !stable_clean_hot_runtime_available
            || miner_discovery_sample_permille >= 1_000
            || (miner_discovery_sample_permille > 0
                && (super::stable_fingerprint([
                    "live_store_discovery_sample",
                    adapter_event.route_key.as_str(),
                    adapter_event.trace_id.as_deref().unwrap_or(""),
                    adapter_event.request_fingerprint.as_deref().unwrap_or(""),
                    &append_parsed_rows.to_string(),
                ]) % 1_000)
                    < miner_discovery_sample_permille as u64);
        if stable_clean_hot_runtime_available && !discovery_sampled {
            miner_clean_hot_runtime_throttle_events =
                miner_clean_hot_runtime_throttle_events.saturating_add(1);
        }

        let learning_decision = store
            .observe_atom_event(&mut encoder, adapter_event.to_live_operator_atom_event())
            .map_err(|error| {
                format!(
                    "append live-tail observe failed for '{}' line {}: {error:?}",
                    append_tail_path.display(),
                    append_total_lines_seen
                )
            })?;
        let mut freeze_candidate_after_learning = learning_decision.active_before_update
            && !frozen_candidates.contains_key(&learning_decision.bucket_id);
        if discovery_sampled {
            for bucket_id in &adapter_event.auto_subcenter_bucket_ids {
                let subcenter_decision = store
                    .observe_atom_event(
                        &mut encoder,
                        adapter_event.to_live_operator_atom_event_for_bucket(*bucket_id),
                    )
                    .map_err(|error| {
                        format!(
                            "append live-tail auto-subcenter observe failed for '{}' line {}: {error:?}",
                            append_tail_path.display(),
                            append_total_lines_seen
                        )
                    })?;
                append_auto_subcenter_observe_events =
                    append_auto_subcenter_observe_events.saturating_add(1);
                if subcenter_decision.false_accept {
                    bucket_policy.observe_rejected_bucket(&adapter_event);
                }
                if quarantine_recovery_discovery {
                    quarantine_recovery_auto_subcenter_observe_events =
                        quarantine_recovery_auto_subcenter_observe_events.saturating_add(1);
                }
                if subcenter_decision.active_before_update
                    && !frozen_candidates.contains_key(&subcenter_decision.bucket_id)
                {
                    freeze_candidate_after_learning = true;
                }
            }
        } else {
            append_auto_subcenter_throttled_events = append_auto_subcenter_throttled_events
                .saturating_add(adapter_event.auto_subcenter_bucket_ids.len());
        }
        if freeze_candidate_after_learning {
            freeze_new_live_store_candidates_from_store(
                &store,
                verifier_binding,
                &mut frozen_candidates,
            )?;
        }
        append_hidden_state_subcenter_observe_events = append_hidden_state_subcenter_observe_events
            .saturating_add(
                adapter_event
                    .auto_subcenter_atoms
                    .iter()
                    .filter(|atom| atom.starts_with("hidden_state:"))
                    .count(),
            );
        bucket_policy.observe_decision(&adapter_event, learning_decision);
        let learning_bucket_rejected_after_update = store
            .miner()
            .bucket(adapter_event.bucket_id)
            .is_some_and(|bucket| bucket.rejected);
        if learning_bucket_rejected_after_update {
            bucket_policy.observe_rejected_bucket(&adapter_event);
        }
        let row_score_events = append_eval
            .score_events
            .saturating_sub(row_eval_score_events_before);
        let row_unique_accepts = append_eval
            .unique_cpu_accepts_over_exact_cache
            .saturating_sub(row_eval_unique_accepts_before);
        let row_tokens_saved = append_eval
            .tokens_saved
            .saturating_sub(row_eval_tokens_saved_before);
        let row_cost_saved = append_eval
            .cost_saved_microusd
            .saturating_sub(row_eval_cost_saved_before);
        let row_false_accepts = append_eval
            .false_accepts
            .saturating_sub(row_eval_false_accepts_before);
        let row_local_accepts = append_eval
            .local_accept_events
            .saturating_sub(row_eval_local_accepts_before);
        if row_false_accepts > 0 {
            append_clean_suffix_rows = 0;
            append_clean_suffix_score_events = 0;
            append_clean_suffix_unique_cpu_accepts_over_exact_cache = 0;
            append_clean_suffix_tokens_saved = 0;
            append_clean_suffix_cost_saved_microusd = 0;
            append_clean_suffix_false_accepts = 0;
            append_clean_suffix_local_accept_events = 0;
            append_clean_suffix_last_quarantine_row_index = Some(append_parsed_rows);
        } else {
            append_clean_suffix_rows = append_clean_suffix_rows.saturating_add(1);
            append_clean_suffix_score_events =
                append_clean_suffix_score_events.saturating_add(row_score_events);
            append_clean_suffix_unique_cpu_accepts_over_exact_cache =
                append_clean_suffix_unique_cpu_accepts_over_exact_cache
                    .saturating_add(row_unique_accepts);
            append_clean_suffix_tokens_saved =
                append_clean_suffix_tokens_saved.saturating_add(row_tokens_saved);
            append_clean_suffix_cost_saved_microusd =
                append_clean_suffix_cost_saved_microusd.saturating_add(row_cost_saved);
            append_clean_suffix_false_accepts =
                append_clean_suffix_false_accepts.saturating_add(row_false_accepts);
            append_clean_suffix_local_accept_events =
                append_clean_suffix_local_accept_events.saturating_add(row_local_accepts);
        }
        let relevant_bucket_states_after_update = relevant_online_bucket_ids
            .iter()
            .filter_map(|bucket_id| {
                store.miner().bucket(*bucket_id).map(|bucket| {
                    serde_json::json!({
                        "bucket_id": bucket.bucket_id,
                        "events_seen": bucket.events_seen,
                        "positive_events": bucket.positive_events,
                        "negative_events": bucket.negative_events,
                        "scored_events": bucket.scored_events,
                        "calibration_events_seen": bucket.calibration_events_seen,
                        "learned_threshold_micro": bucket.learned_threshold_micro,
                        "max_calibration_false_margin_micro": bucket.max_calibration_false_margin_micro,
                        "active": bucket.is_active(min_bucket_events),
                        "shadow_ready": bucket.is_shadow_ready(min_bucket_events, min_bucket_events),
                        "unique_cpu_accepts_over_exact_cache": bucket.unique_cpu_accepts_over_exact_cache,
                        "false_accepts": bucket.false_accepts,
                        "rejected": bucket.rejected
                    })
                })
            })
            .collect::<Vec<_>>();
        let mut decision_line = serde_json::json!({
            "architecture_versions": live_store_architecture_versions(),
            "architecture_version_key": architecture_version_key.clone(),
            "decision_schema_version": "append_live_tail_decision_v2",
            "append_row_index": append_parsed_rows,
            "source": append_tail_path.display().to_string(),
            "tail_line_index": append_total_lines_seen,
            "route_key": adapter_event.route_key.as_str(),
            "bucket_key": adapter_event.bucket_key.as_str(),
            "bucket_refinement_depth": adapter_event.bucket_refinement_depth,
            "safe_atom_count": adapter_event.safe_atom_count,
            "bucket_selector_candidate_atoms": adapter_event.bucket_selector_candidate_atoms,
            "selected_bucket_atoms": adapter_event.selected_bucket_atoms,
            "auto_subcenter_atoms": adapter_event.auto_subcenter_atoms,
            "auto_subcenter_bucket_ids": adapter_event.auto_subcenter_bucket_ids,
            "relevant_online_bucket_ids": relevant_online_bucket_ids,
            "online_shadow_ready_relevant_bucket_ids": online_shadow_ready_relevant_bucket_ids,
            "relevant_bucket_states_after_update": relevant_bucket_states_after_update,
            "route_id": adapter_event.route_id,
            "bucket_id": adapter_event.bucket_id,
            "verified_safe_accept": adapter_event.verified_safe_accept,
            "exact_cache_hit": adapter_event.exact_cache_hit,
            "tokens": adapter_event.tokens,
            "cost_microusd": adapter_event.cost_microusd,
            "cost_estimate_used": adapter_event.cost_estimate_used,
            "scored_before_update": scored_before_update,
            "learning_active_before_update": learning_decision.active_before_update,
            "learning_calibration_event": learning_decision.calibration_event,
            "learning_margin_micro": learning_decision.margin_micro,
            "learning_threshold_micro": learning_decision.threshold_micro,
            "learning_raw_local_operator": learning_decision.raw_local_operator,
            "learning_unique_cpu_accept_over_exact_cache": learning_decision.unique_cpu_accept_over_exact_cache,
            "learning_false_accept": learning_decision.false_accept,
            "learning_bucket_rejected_after_update": learning_bucket_rejected_after_update,
            "product_hot_phase_trust_filtered": row_product_hot_phase_trust_filtered,
            "route_wide_phase_transfer_allowed": route_wide_phase_transfer_allowed,
            "decisions": decision_rows
        });
        if let Some(object) = decision_line.as_object_mut() {
            object.insert(
                "row_score_events".to_owned(),
                serde_json::Value::from(row_score_events as u64),
            );
            object.insert(
                "row_unique_cpu_accepts_over_exact_cache".to_owned(),
                serde_json::Value::from(row_unique_accepts as u64),
            );
            object.insert(
                "row_tokens_saved".to_owned(),
                serde_json::Value::from(row_tokens_saved),
            );
            object.insert(
                "row_cost_saved_microusd".to_owned(),
                serde_json::Value::from(row_cost_saved),
            );
            object.insert(
                "row_false_accepts".to_owned(),
                serde_json::Value::from(row_false_accepts as u64),
            );
            object.insert(
                "append_clean_suffix_rows".to_owned(),
                serde_json::Value::from(append_clean_suffix_rows as u64),
            );
            object.insert(
                "append_clean_suffix_score_events".to_owned(),
                serde_json::Value::from(append_clean_suffix_score_events as u64),
            );
            object.insert(
                "append_clean_suffix_unique_cpu_accepts_over_exact_cache".to_owned(),
                serde_json::Value::from(
                    append_clean_suffix_unique_cpu_accepts_over_exact_cache as u64,
                ),
            );
            object.insert(
                "append_clean_suffix_tokens_saved".to_owned(),
                serde_json::Value::from(append_clean_suffix_tokens_saved),
            );
            object.insert(
                "append_clean_suffix_false_accepts".to_owned(),
                serde_json::Value::from(append_clean_suffix_false_accepts as u64),
            );
        }
        live_store_observe_stable_decision_log_row(
            &mut stable_decision_log_window,
            &decision_line,
            &architecture_version_key,
        );
        live_store_observe_stable_decision_log_clean_suffix_row(
            &mut stable_decision_log_clean_suffix,
            &decision_line,
            &architecture_version_key,
        );
        live_store_observe_stable_decision_log_serving_row(
            &mut stable_serving_cpu_window,
            &decision_line,
            &architecture_version_key,
        );
        live_store_observe_stable_decision_log_serving_clean_suffix_row(
            &mut stable_serving_cpu_clean_suffix,
            &decision_line,
            &architecture_version_key,
        );
        serde_json::to_writer(&mut decision_log, &decision_line).map_err(|error| {
            format!(
                "failed to write append live-tail decision '{}': {error}",
                decision_log_path.display()
            )
        })?;
        decision_log
            .write_all(b"\n")
            .map_err(|error| format!("failed append live-tail decision newline: {error}"))?;
        decision_log_pending_rows = decision_log_pending_rows.saturating_add(1);
        if decision_log_pending_rows >= DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_DECISION_FLUSH_ROWS
        {
            decision_log
                .flush()
                .map_err(|error| format!("failed append live-tail decision flush: {error}"))?;
            decision_log_flush_count = decision_log_flush_count.saturating_add(1);
            decision_log_pending_rows = 0;
        }
        append_parsed_rows += 1;
        if max_append_events > 0 && append_parsed_rows >= max_append_events {
            break;
        }
    }
    decision_log
        .flush()
        .map_err(|error| format!("failed to flush append live-tail decision log: {error}"))?;

    let summary = store.summary();
    let runtime_budget = live_store_budget_report(store.runtime_budget_snapshot());
    let (
        mut final_hot_runtime_available,
        mut final_hot_route_ids,
        mut final_hot_profile_ids,
        mut final_hot_route_count,
        mut final_hot_profile_count,
        mut final_hot_route_profile_edges,
    ) = if let Some(bundle) = product_hot_registry_runtime.as_ref().filter(|bundle| {
        live_store_active_hot_profile_count(
            &bundle.hot_runtime,
            &product_hot_score_only_quarantined_profile_ids,
        ) > 0
    }) {
        (
            true,
            live_store_hot_route_ids(&bundle.route_table),
            live_store_hot_profile_ids(&bundle.hot_runtime),
            bundle.route_table.route_count(),
            bundle.hot_runtime.profile_count(),
            bundle.route_table.profile_edge_count(),
        )
    } else if let Some((hot_runtime, route_table)) = store
        .candidate_hot_runtime_and_route_table_excluding(
            &product_hot_score_only_quarantined_profile_ids
                .iter()
                .copied()
                .collect::<Vec<_>>(),
        )
        .map_err(|error| {
            format!("failed to build final append live-tail clean candidate hot view: {error:?}")
        })?
    {
        (
            true,
            live_store_hot_route_ids(&route_table),
            live_store_hot_profile_ids(&hot_runtime),
            route_table.route_count(),
            hot_runtime.profile_count(),
            route_table.profile_edge_count(),
        )
    } else {
        (false, Vec::new(), Vec::new(), 0, 0, 0)
    };
    let exact_cache_overlap_excluded = append_denominator.non_exact_rows > 0
        && append_eval.unique_cpu_accepts_over_exact_cache <= append_denominator.non_exact_rows;
    let token_cost_denominator_present =
        append_denominator.total_tokens > 0 && append_denominator.total_cost_microusd > 0;
    freeze_new_live_store_candidates_from_store(&store, verifier_binding, &mut frozen_candidates)?;
    let mut candidate_packages = Vec::new();
    store
        .candidate_packages_into_with_verifier(verifier_binding, &mut candidate_packages)
        .map_err(|error| {
            format!("failed to build append live-tail verifier-bound candidates: {error:?}")
        })?;
    candidate_package_reports = write_live_store_candidate_packages_with_route_lookup(
        &candidate_package_dir,
        candidate_packages,
        |bucket_id| store.route_id_for_bucket(bucket_id),
    )?;
    let (trusted_clean_profile_ids, _, _) = live_store_clean_candidate_frontier(
        &store,
        &product_hot_score_only_quarantined_profile_ids,
    );
    for profile_id in live_store_trusted_clean_candidate_profile_ids(
        &store,
        &trusted_clean_profile_ids,
        min_bucket_events,
    ) {
        product_hot_score_only_quarantined_profile_ids.remove(&profile_id);
    }
    live_store_refresh_future_shadow_summary(
        &mut future_shadow,
        &frozen_candidates,
        token_cost_denominator_present,
        append_parsed_rows,
        append_denominator.total_tokens,
        append_denominator.total_cost_microusd,
    )?;
    refresh_live_store_call_token_promotion_manifest_summary_with_quarantine(
        &mut future_shadow,
        &frozen_candidates,
        &product_hot_score_only_quarantined_profile_ids,
    );
    write_live_store_clean_promotion_manifest(
        &clean_promotion_manifest_path,
        &clean_promotion_package_dir,
        &future_shadow,
        &frozen_candidates,
    )?;
    clean_promotion_manifest_written = true;
    if future_shadow.clean_promotion_manifest_allowed
        && future_shadow.clean_promotion_manifest_false_accepts == 0
        && future_shadow.clean_promotion_manifest_runtime_parity_mismatches == 0
    {
        for route in &future_shadow.clean_promotion_manifest_routes {
            product_hot_score_only_quarantined_profile_ids.remove(&route.profile_id);
        }
        refresh_live_store_call_token_promotion_manifest_summary_with_quarantine(
            &mut future_shadow,
            &frozen_candidates,
            &product_hot_score_only_quarantined_profile_ids,
        );
    }
    write_live_store_call_token_promotion_manifest_with_quarantine(
        &call_token_promotion_manifest_path,
        &call_token_promotion_package_dir,
        &future_shadow,
        &frozen_candidates,
        &product_hot_score_only_quarantined_profile_ids,
    )?;
    write_live_store_clean_survivor_call_token_promotion_manifest(
        &call_token_promotion_manifest_path,
        &call_token_promotion_package_dir,
        &store,
        &frozen_candidates,
        &product_hot_score_only_quarantined_profile_ids,
        &append_profile_kind_by_id,
        &mut future_shadow,
    )?;
    call_token_promotion_manifest_written = true;
    let final_promotion_manifest_loadable = if future_shadow.call_token_promotion_manifest_allowed {
        let promotion_manifest = super::read_json_value(&call_token_promotion_manifest_path)?;
        !live_store_call_token_manifest_promotes_quarantined_profile(
            &promotion_manifest,
            &product_hot_score_only_quarantined_profile_ids,
        )
    } else {
        false
    };
    if final_promotion_manifest_loadable
        && future_shadow.call_token_promotion_manifest_false_accepts == 0
    {
        match load_live_store_product_hot_runtime_from_clean_manifest(
            &call_token_promotion_manifest_path,
            cells,
        ) {
            Ok(refreshed_runtime) => {
                std::fs::copy(
                    &call_token_promotion_manifest_path,
                    &call_token_active_manifest_path,
                )
                .map_err(|error| {
                    format!(
                        "failed to persist final active call-token manifest '{}': {error}",
                        call_token_active_manifest_path.display()
                    )
                })?;
                final_hot_runtime_available = true;
                final_hot_route_ids = live_store_hot_route_ids(&refreshed_runtime.route_table);
                final_hot_profile_ids = live_store_hot_profile_ids(&refreshed_runtime.hot_runtime);
                final_hot_route_count = refreshed_runtime.route_table.route_count();
                final_hot_profile_count = refreshed_runtime.hot_runtime.profile_count();
                final_hot_route_profile_edges = refreshed_runtime.route_table.profile_edge_count();
                product_hot_registry_path_report =
                    refreshed_runtime.registry_path.display().to_string();
                product_hot_registry_source_report = "call_token_active_manifest".to_owned();
                product_hot_registry_runtime = Some(refreshed_runtime);
            }
            Err(error) => {
                eprintln!(
                    "phase-stream live-tail: final call/token manifest was not promoted because it is not loadable: {error}"
                );
            }
        }
    }
    let final_product_hot_runtime_source_claim_ready =
        live_store_product_hot_runtime_source_claim_ready(&product_hot_registry_source_report);
    future_shadow_billing_request = write_live_store_future_shadow_billing_requests(
        &future_shadow_billing_request_path,
        &frozen_candidates,
    )?;
    if future_shadow_billing_request.rows == 0 {
        future_shadow_billing_request = write_live_store_stable_clean_suffix_billing_requests(
            &future_shadow_billing_request_path,
            &decision_log_path,
            &architecture_version_key,
        )?;
    }
    cold_artifact_refresh_count = cold_artifact_refresh_count.saturating_add(1);
    cold_artifact_refresh_append_rows = append_parsed_rows;
    let product_hot_budget_passed = runtime_budget.hot_budget_passed;
    let warm_miner_budget_passed = runtime_budget.warm_budget_passed;
    let warm_miner_budget_blocker = if warm_miner_budget_passed {
        "none"
    } else {
        "warm_miner_budget_watch"
    }
    .to_owned();
    let adapter_gate_passed = watermark_parsed_rows > 0
        && append_parsed_rows > 0
        && append_eval.score_events > 0
        && append_eval.false_accepts == 0
        && append_eval.local_accept_events == 0
        && product_hot_budget_passed;
    let blocker = if adapter_gate_passed {
        "none".to_owned()
    } else if watermark_parsed_rows == 0 {
        "append_live_tail_no_watermark_events".to_owned()
    } else if append_parsed_rows == 0 {
        "append_live_tail_no_appended_events_seen".to_owned()
    } else if append_eval.score_events == 0 {
        "append_live_tail_no_score_before_update_events".to_owned()
    } else if append_eval.false_accepts != 0 {
        "append_live_tail_false_accepts_nonzero".to_owned()
    } else if append_eval.local_accept_events != 0 {
        "append_live_tail_local_accept_enabled".to_owned()
    } else if !product_hot_budget_passed {
        "append_live_tail_hot_runtime_budget_failed".to_owned()
    } else {
        "append_live_tail_watch".to_owned()
    };
    let (
        clean_candidate_profile_ids,
        clean_candidate_quarantined_profile_ids,
        clean_candidate_exportable_profile_ids,
    ) = live_store_clean_candidate_frontier(
        &store,
        &product_hot_score_only_quarantined_profile_ids,
    );
    let clean_candidate_reports = live_store_clean_candidate_value_reports(
        &store,
        &clean_candidate_profile_ids,
        &product_hot_score_only_quarantined_profile_ids,
        &append_profile_kind_by_id,
        &final_hot_profile_ids,
        min_bucket_events,
    );
    let provider_money_claim_blocker = live_store_provider_money_claim_blocker(
        &provider_evidence_artifacts,
        &future_shadow_billing_request,
    )
    .to_owned();
    let (
        append_known_profile_kind_count,
        append_observable_known_profile_count,
        append_hidden_state_known_profile_count,
    ) = live_store_known_profile_kind_counts(&append_profile_kind_by_id);
    let (clean_product_hot_calls, clean_product_hot_tokens, clean_product_hot_cost_microusd) =
        live_store_product_hot_clean_credit_totals(
            &product_hot_score_only_credit_rows,
            &product_hot_score_only_quarantined_profile_ids,
        );
    let product_hot_score_only_unique_cpu_accepts_over_exact_cache = clean_product_hot_calls;
    let product_hot_score_only_tokens_saved = clean_product_hot_tokens;
    let product_hot_score_only_cost_saved_microusd = clean_product_hot_cost_microusd;
    let append_exact_cache_calls_saved_milli_over_parsed_rows = live_store_milli(
        append_denominator.exact_cache_hits as u64,
        append_parsed_rows as u64,
    );
    let append_exact_cache_tokens_saved_milli_over_total = live_store_milli(
        append_denominator.exact_cache_tokens,
        append_denominator.total_tokens,
    );
    let append_exact_cache_cost_saved_milli_over_total = live_store_milli(
        append_denominator.exact_cache_cost_microusd,
        append_denominator.total_cost_microusd,
    );
    let append_active_clean_calls_saved_milli_over_parsed_rows =
        live_store_milli(active_clean_calls_saved as u64, append_parsed_rows as u64);
    let append_active_clean_tokens_saved_milli_over_total =
        live_store_milli(active_clean_tokens_saved, append_denominator.total_tokens);
    let append_active_clean_cost_saved_milli_over_total = live_store_milli(
        append_eval.cost_saved_microusd,
        append_denominator.total_cost_microusd,
    );
    let append_combined_calls_saved_milli_over_parsed_rows = live_store_milli(
        append_denominator
            .exact_cache_hits
            .saturating_add(active_clean_calls_saved) as u64,
        append_parsed_rows as u64,
    );
    let append_combined_tokens_saved_milli_over_total = live_store_milli(
        append_denominator
            .exact_cache_tokens
            .saturating_add(active_clean_tokens_saved),
        append_denominator.total_tokens,
    );
    let append_combined_cost_saved_milli_over_total = live_store_milli(
        append_denominator
            .exact_cache_cost_microusd
            .saturating_add(append_eval.cost_saved_microusd),
        append_denominator.total_cost_microusd,
    );
    let append_hidden_state_only_calls_saved_milli_over_parsed_rows = live_store_milli(
        append_hidden_state_only_unique_cpu_accepts_over_exact_cache as u64,
        append_parsed_rows as u64,
    );
    let append_hidden_state_only_tokens_saved_milli_over_total = live_store_milli(
        append_hidden_state_only_tokens_saved,
        append_denominator.total_tokens,
    );
    let append_hidden_state_only_cost_saved_milli_over_total = live_store_milli(
        append_hidden_state_only_cost_saved_microusd,
        append_denominator.total_cost_microusd,
    );
    let append_compression_claim_blocker = live_store_append_compression_claim_blocker(
        append_parsed_rows,
        append_eval.false_accepts,
        append_eval.local_accept_events,
        product_hot_score_only_unique_cpu_accepts_over_exact_cache,
        product_hot_score_only_tokens_saved,
        token_cost_denominator_present,
        final_hot_runtime_available,
        final_product_hot_runtime_source_claim_ready,
        product_hot_score_only_post_quarantine_false_accepts,
        live_store_append_compression_claim_min_rows(),
    )
    .to_owned();
    let append_compression_claim_allowed = append_compression_claim_blocker == "none";
    let append_clean_suffix_claim_blocker = live_store_append_compression_claim_blocker(
        append_clean_suffix_rows,
        append_clean_suffix_false_accepts,
        append_clean_suffix_local_accept_events,
        append_clean_suffix_unique_cpu_accepts_over_exact_cache,
        append_clean_suffix_tokens_saved,
        token_cost_denominator_present,
        final_hot_runtime_available,
        final_product_hot_runtime_source_claim_ready,
        0,
        live_store_append_compression_claim_min_rows(),
    )
    .to_owned();
    let append_clean_suffix_claim_allowed = append_clean_suffix_claim_blocker == "none";
    let stable_decision_log_claim_blocker = live_store_append_compression_claim_blocker(
        stable_decision_log_window.rows,
        stable_decision_log_window.false_accepts,
        stable_decision_log_window.local_accept_events,
        stable_decision_log_window.unique_cpu_accepts_over_exact_cache,
        stable_decision_log_window.tokens_saved,
        stable_decision_log_window.total_tokens > 0
            && stable_decision_log_window.total_cost_microusd > 0,
        final_hot_runtime_available,
        final_product_hot_runtime_source_claim_ready,
        0,
        live_store_append_compression_claim_min_rows(),
    )
    .to_owned();
    let stable_decision_log_claim_allowed = stable_decision_log_claim_blocker == "none";
    let stable_decision_log_clean_suffix_claim_blocker =
        live_store_append_compression_claim_blocker(
            stable_decision_log_clean_suffix.window.rows,
            stable_decision_log_clean_suffix.window.false_accepts,
            stable_decision_log_clean_suffix.window.local_accept_events,
            stable_decision_log_clean_suffix
                .window
                .unique_cpu_accepts_over_exact_cache,
            stable_decision_log_clean_suffix.window.tokens_saved,
            stable_decision_log_clean_suffix.window.total_tokens > 0
                && stable_decision_log_clean_suffix.window.total_cost_microusd > 0,
            final_hot_runtime_available,
            final_product_hot_runtime_source_claim_ready,
            0,
            live_store_append_compression_claim_min_rows(),
        )
        .to_owned();
    let stable_decision_log_clean_suffix_claim_allowed =
        stable_decision_log_clean_suffix_claim_blocker == "none";
    let stable_decision_log_clean_suffix_rows_to_min =
        live_store_append_compression_claim_min_rows()
            .saturating_sub(stable_decision_log_clean_suffix.window.rows);
    let stable_clean_token_compression_saved_milli = live_store_milli(
        stable_decision_log_clean_suffix.window.tokens_saved,
        stable_decision_log_clean_suffix.window.total_tokens,
    );
    let stable_serving_cpu_claim_blocker = live_store_serving_cpu_compression_claim_blocker(
        stable_serving_cpu_window.rows,
        stable_serving_cpu_window.false_accepts,
        stable_serving_cpu_window.local_accept_events,
        stable_serving_cpu_window.unique_cpu_accepts_over_exact_cache,
        stable_serving_cpu_window.tokens_saved,
        stable_serving_cpu_window.total_tokens > 0
            && stable_serving_cpu_window.total_cost_microusd > 0,
        final_hot_runtime_available,
        final_product_hot_runtime_source_claim_ready,
        live_store_append_compression_claim_min_rows(),
    )
    .to_owned();
    let stable_serving_cpu_claim_allowed = stable_serving_cpu_claim_blocker == "none";
    let stable_serving_cpu_clean_suffix_claim_blocker =
        live_store_serving_cpu_compression_claim_blocker(
            stable_serving_cpu_clean_suffix.window.rows,
            stable_serving_cpu_clean_suffix.window.false_accepts,
            stable_serving_cpu_clean_suffix.window.local_accept_events,
            stable_serving_cpu_clean_suffix
                .window
                .unique_cpu_accepts_over_exact_cache,
            stable_serving_cpu_clean_suffix.window.tokens_saved,
            stable_serving_cpu_clean_suffix.window.total_tokens > 0
                && stable_serving_cpu_clean_suffix.window.total_cost_microusd > 0,
            final_hot_runtime_available,
            final_product_hot_runtime_source_claim_ready,
            live_store_append_compression_claim_min_rows(),
        )
        .to_owned();
    let stable_serving_cpu_clean_suffix_claim_allowed =
        stable_serving_cpu_clean_suffix_claim_blocker == "none";
    let stable_serving_cpu_clean_suffix_saved_milli = live_store_milli(
        stable_serving_cpu_clean_suffix.window.tokens_saved,
        stable_serving_cpu_clean_suffix.window.total_tokens,
    );
    let miner_saturation_last_snapshot = miner_saturation.last_snapshot();
    let report = PhaseStreamHotPathDaemonAppendLiveTailReport {
        report_kind: "phase_stream_hot_path_daemon_append_live_tail_v1",
        architecture_versions: live_store_architecture_versions(),
        mode: "append_file_tail_follow_score_before_update_shadow_only",
        snapshot_in_progress: false,
        decision_log_path: decision_log_path.display().to_string(),
        watermark_trace_path: watermark_trace_path.display().to_string(),
        append_tail_path: append_tail_path.display().to_string(),
        cells,
        min_bucket_events,
        idle_sleep_ms,
        max_idle_ms,
        max_append_events,
        idle_elapsed_ms,
        miner_saturation_control_enabled,
        miner_saturation_min_idle_heartbeats,
        miner_saturation_sleep_ms,
        miner_saturation_idle_heartbeats: miner_saturation.idle_heartbeats(),
        miner_saturation_active: miner_saturation.active(),
        miner_saturation_sleep_events: miner_saturation.sleep_events(),
        miner_saturation_last_sleep_ms: miner_saturation.last_sleep_ms(),
        miner_saturation_last_append_parsed_rows: miner_saturation_last_snapshot.append_parsed_rows,
        miner_saturation_last_score_events: miner_saturation_last_snapshot.score_events,
        miner_saturation_last_unique_cpu_accepts_over_exact_cache: miner_saturation_last_snapshot
            .unique_cpu_accepts_over_exact_cache,
        miner_saturation_last_tokens_saved: miner_saturation_last_snapshot.tokens_saved,
        miner_saturation_last_false_accepts: miner_saturation_last_snapshot.false_accepts,
        miner_saturation_last_bucket_count: miner_saturation_last_snapshot.bucket_count,
        miner_saturation_last_active_bucket_count: miner_saturation_last_snapshot
            .active_bucket_count,
        miner_saturation_last_refinement_count: miner_saturation_last_snapshot.refinement_count,
        miner_saturation_last_quarantined_profile_count: miner_saturation_last_snapshot
            .quarantined_profile_count,
        miner_active_batch_rows,
        miner_active_batch_sleep_ms,
        miner_active_batch_sleep_events,
        cold_artifact_refresh_interval_secs:
            DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_COLD_REFRESH_SECS,
        cold_artifact_refresh_count,
        cold_artifact_refresh_used_this_snapshot: true,
        cold_artifact_refresh_append_rows,
        decision_log_flush_interval_rows:
            DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_DECISION_FLUSH_ROWS,
        decision_log_flush_count,
        decision_log_pending_rows,
        start_at_end: true,
        watermark_total_rows,
        watermark_parsed_rows,
        watermark_skipped_no_verifier_label,
        watermark_skipped_no_safe_atoms,
        append_history_warm_total_rows,
        append_history_warm_parsed_rows,
        append_history_warm_skipped_no_verifier_label,
        append_history_warm_skipped_no_safe_atoms,
        tail_total_lines_seen: append_total_lines_seen,
        append_parsed_rows,
        append_skipped_no_verifier_label,
        append_skipped_no_safe_atoms,
        append_hot_view_available_events,
        append_route_index_missing_events,
        append_route_index_missing_before_first_score,
        append_route_index_missing_after_first_score,
        append_auto_subcenter_observe_events,
        append_auto_subcenter_throttled_events,
        append_hidden_state_subcenter_observe_events,
        miner_discovery_sample_permille,
        miner_clean_hot_runtime_throttle_events,
        append_known_profile_kind_count,
        append_observable_known_profile_count,
        append_hidden_state_known_profile_count,
        append_scoring_started,
        append_score_events_before_update: append_eval.score_events,
        append_score_candidate_events: append_eval.score_candidate_events,
        append_observable_score_candidate_events,
        append_hidden_state_score_candidate_events,
        append_unknown_profile_score_candidate_events,
        append_verifier_required_events: append_eval.verifier_required_events,
        append_local_accept_events: append_eval.local_accept_events,
        append_unique_cpu_accepts_over_exact_cache: append_eval.unique_cpu_accepts_over_exact_cache,
        append_observable_unique_cpu_accepts_over_exact_cache,
        append_hidden_state_unique_cpu_accepts_over_exact_cache,
        append_unknown_profile_unique_cpu_accepts_over_exact_cache,
        append_profile_attribution_overlap_accepts,
        append_observable_only_unique_cpu_accepts_over_exact_cache,
        append_hidden_state_only_unique_cpu_accepts_over_exact_cache,
        append_mixed_profile_unique_cpu_accepts_over_exact_cache,
        append_unknown_only_unique_cpu_accepts_over_exact_cache,
        append_tokens_saved: append_eval.tokens_saved,
        append_observable_tokens_saved,
        append_hidden_state_tokens_saved,
        append_unknown_profile_tokens_saved,
        append_observable_only_tokens_saved,
        append_hidden_state_only_tokens_saved,
        append_mixed_profile_tokens_saved,
        append_unknown_only_tokens_saved,
        append_observable_only_cost_saved_microusd,
        append_hidden_state_only_cost_saved_microusd,
        append_mixed_profile_cost_saved_microusd,
        append_unknown_only_cost_saved_microusd,
        append_cost_saved_microusd: append_eval.cost_saved_microusd,
        append_exact_cache_calls_saved_milli_over_parsed_rows,
        append_exact_cache_tokens_saved_milli_over_total,
        append_exact_cache_cost_saved_milli_over_total,
        append_active_clean_calls_saved_milli_over_parsed_rows,
        append_active_clean_tokens_saved_milli_over_total,
        append_active_clean_cost_saved_milli_over_total,
        append_combined_calls_saved_milli_over_parsed_rows,
        append_combined_tokens_saved_milli_over_total,
        append_combined_cost_saved_milli_over_total,
        append_hidden_state_only_calls_saved_milli_over_parsed_rows,
        append_hidden_state_only_tokens_saved_milli_over_total,
        append_hidden_state_only_cost_saved_milli_over_total,
        append_cost_estimate_used: append_estimated_cost_events > 0,
        append_estimated_cost_events,
        append_estimated_total_cost_microusd,
        append_estimated_cost_saved_microusd,
        append_false_accepts: append_eval.false_accepts,
        append_compression_claim_min_rows: live_store_append_compression_claim_min_rows(),
        append_compression_claim_allowed,
        append_compression_claim_blocker,
        append_clean_suffix_rows,
        append_clean_suffix_score_events,
        append_clean_suffix_unique_cpu_accepts_over_exact_cache,
        append_clean_suffix_tokens_saved,
        append_clean_suffix_cost_saved_microusd,
        append_clean_suffix_false_accepts,
        append_clean_suffix_local_accept_events,
        append_clean_suffix_last_quarantine_row_index,
        append_clean_suffix_claim_allowed,
        append_clean_suffix_claim_blocker,
        stable_decision_log_architecture_key: architecture_version_key,
        stable_decision_log_rows: stable_decision_log_window.rows,
        stable_decision_log_score_candidate_events: stable_decision_log_window
            .score_candidate_events,
        stable_decision_log_unique_cpu_accepts_over_exact_cache: stable_decision_log_window
            .unique_cpu_accepts_over_exact_cache,
        stable_decision_log_tokens_saved: stable_decision_log_window.tokens_saved,
        stable_decision_log_cost_saved_microusd: stable_decision_log_window.cost_saved_microusd,
        stable_decision_log_false_accepts: stable_decision_log_window.false_accepts,
        stable_decision_log_local_accept_events: stable_decision_log_window.local_accept_events,
        stable_decision_log_total_tokens: stable_decision_log_window.total_tokens,
        stable_decision_log_total_cost_microusd: stable_decision_log_window.total_cost_microusd,
        stable_decision_log_claim_allowed,
        stable_decision_log_claim_blocker,
        stable_decision_log_clean_suffix_rows: stable_decision_log_clean_suffix.window.rows,
        stable_decision_log_clean_suffix_score_candidate_events: stable_decision_log_clean_suffix
            .window
            .score_candidate_events,
        stable_decision_log_clean_suffix_unique_cpu_accepts_over_exact_cache:
            stable_decision_log_clean_suffix
                .window
                .unique_cpu_accepts_over_exact_cache,
        stable_decision_log_clean_suffix_tokens_saved: stable_decision_log_clean_suffix
            .window
            .tokens_saved,
        stable_decision_log_clean_suffix_cost_saved_microusd: stable_decision_log_clean_suffix
            .window
            .cost_saved_microusd,
        stable_decision_log_clean_suffix_false_accepts: stable_decision_log_clean_suffix
            .window
            .false_accepts,
        stable_decision_log_clean_suffix_local_accept_events: stable_decision_log_clean_suffix
            .window
            .local_accept_events,
        stable_decision_log_clean_suffix_total_tokens: stable_decision_log_clean_suffix
            .window
            .total_tokens,
        stable_decision_log_clean_suffix_total_cost_microusd: stable_decision_log_clean_suffix
            .window
            .total_cost_microusd,
        stable_decision_log_clean_suffix_last_quarantine_row_index:
            stable_decision_log_clean_suffix.last_quarantine_row_index,
        stable_decision_log_clean_suffix_min_rows: live_store_append_compression_claim_min_rows(),
        stable_decision_log_clean_suffix_rows_to_min,
        stable_decision_log_clean_suffix_claim_allowed,
        stable_decision_log_clean_suffix_claim_blocker:
            stable_decision_log_clean_suffix_claim_blocker.clone(),
        stable_clean_token_compression_claim_allowed:
            stable_decision_log_clean_suffix_claim_allowed,
        stable_clean_token_compression_claim_blocker:
            stable_decision_log_clean_suffix_claim_blocker,
        stable_clean_token_compression_unique_cpu_accepts_over_exact_cache:
            stable_decision_log_clean_suffix
                .window
                .unique_cpu_accepts_over_exact_cache,
        stable_clean_token_compression_saved_tokens: stable_decision_log_clean_suffix
            .window
            .tokens_saved,
        stable_clean_token_compression_total_tokens: stable_decision_log_clean_suffix
            .window
            .total_tokens,
        stable_clean_token_compression_saved_milli,
        stable_clean_token_compression_false_accepts: stable_decision_log_clean_suffix
            .window
            .false_accepts,
        stable_serving_cpu_rows: stable_serving_cpu_window.rows,
        stable_serving_cpu_score_candidate_events: stable_serving_cpu_window.score_candidate_events,
        stable_serving_cpu_local_accept_events: stable_serving_cpu_window.local_accept_events,
        stable_serving_cpu_unique_cpu_accepts_over_exact_cache: stable_serving_cpu_window
            .unique_cpu_accepts_over_exact_cache,
        stable_serving_cpu_tokens_saved: stable_serving_cpu_window.tokens_saved,
        stable_serving_cpu_total_tokens: stable_serving_cpu_window.total_tokens,
        stable_serving_cpu_false_accepts: stable_serving_cpu_window.false_accepts,
        stable_serving_cpu_claim_allowed,
        stable_serving_cpu_claim_blocker,
        stable_serving_cpu_clean_suffix_rows: stable_serving_cpu_clean_suffix.window.rows,
        stable_serving_cpu_clean_suffix_score_candidate_events: stable_serving_cpu_clean_suffix
            .window
            .score_candidate_events,
        stable_serving_cpu_clean_suffix_local_accept_events: stable_serving_cpu_clean_suffix
            .window
            .local_accept_events,
        stable_serving_cpu_clean_suffix_unique_cpu_accepts_over_exact_cache:
            stable_serving_cpu_clean_suffix
                .window
                .unique_cpu_accepts_over_exact_cache,
        stable_serving_cpu_clean_suffix_tokens_saved: stable_serving_cpu_clean_suffix
            .window
            .tokens_saved,
        stable_serving_cpu_clean_suffix_total_tokens: stable_serving_cpu_clean_suffix
            .window
            .total_tokens,
        stable_serving_cpu_clean_suffix_false_accepts: stable_serving_cpu_clean_suffix
            .window
            .false_accepts,
        stable_serving_cpu_clean_suffix_saved_milli,
        stable_serving_cpu_clean_suffix_claim_allowed,
        stable_serving_cpu_clean_suffix_claim_blocker,
        append_total_tokens: append_denominator.total_tokens,
        append_total_cost_microusd: append_denominator.total_cost_microusd,
        append_exact_cache_hits: append_denominator.exact_cache_hits,
        append_exact_cache_tokens: append_denominator.exact_cache_tokens,
        append_exact_cache_cost_microusd: append_denominator.exact_cache_cost_microusd,
        append_non_exact_rows: append_denominator.non_exact_rows,
        online_bucket_count: summary.bucket_count,
        active_bucket_count: summary.active_bucket_count,
        shadow_ready_bucket_count: summary.shadow_ready_bucket_count,
        candidate_bucket_count: summary.candidate_bucket_count,
        clean_candidate_profile_ids,
        clean_candidate_quarantined_profile_ids,
        clean_candidate_exportable_profile_ids,
        clean_candidate_reports,
        rejected_bucket_count: summary.rejected_bucket_count,
        live_route_count: store.route_count(),
        live_route_bucket_count: store.route_bucket_count(),
        adaptive_refinement_count: bucket_policy.refinement_count,
        max_bucket_refinement_depth: bucket_policy.max_depth(),
        product_hot_score_only_registry_path: product_hot_registry_path_report,
        product_hot_score_only_runtime_source: product_hot_registry_source_report,
        product_hot_score_only_runtime_loaded: product_hot_registry_runtime.is_some(),
        product_hot_score_only_runtime_active: product_hot_registry_runtime.as_ref().is_some_and(
            |bundle| {
                live_store_active_hot_profile_count(
                    &bundle.hot_runtime,
                    &product_hot_score_only_quarantined_profile_ids,
                ) > 0
            },
        ),
        product_hot_score_only_active_manifest_disabled:
            product_hot_score_only_active_manifest_disabled,
        product_hot_score_only_active_manifest_disable_reason:
            product_hot_score_only_active_manifest_disable_reason,
        product_hot_score_only_quarantined: !product_hot_score_only_quarantined_profile_ids
            .is_empty(),
        product_hot_score_only_quarantine_reason,
        product_hot_score_only_quarantine_false_accepts,
        product_hot_score_only_post_quarantine_score_candidate_events,
        product_hot_score_only_post_quarantine_false_accepts,
        product_hot_phase_trust_filtered_events,
        product_hot_score_only_unique_cpu_accepts_over_exact_cache,
        product_hot_score_only_tokens_saved,
        product_hot_score_only_cost_saved_microusd,
        active_clean_calls_saved,
        active_clean_tokens_saved,
        lost_calls_due_to_quarantine,
        lost_tokens_due_to_quarantine,
        quarantine_recovery_discovery_events,
        quarantine_recovery_discovery_tokens,
        quarantine_recovery_auto_subcenter_observe_events,
        product_hot_score_only_active_profile_count: product_hot_registry_runtime.as_ref().map_or(
            0,
            |bundle| {
                live_store_active_hot_profile_count(
                    &bundle.hot_runtime,
                    &product_hot_score_only_quarantined_profile_ids,
                )
            },
        ),
        product_hot_score_only_quarantined_profile_count:
            product_hot_score_only_quarantined_profile_ids.len(),
        product_hot_score_only_quarantined_profile_ids:
            product_hot_score_only_quarantined_profile_ids
                .iter()
                .copied()
                .collect(),
        product_hot_score_only_quarantine_trace_id,
        product_hot_score_only_quarantine_route_key,
        product_hot_score_only_quarantine_bucket_key,
        product_hot_score_only_auto_refinement_candidate_atoms,
        product_hot_score_only_auto_refinement_selected_atoms,
        product_hot_score_only_route_count: product_hot_registry_runtime
            .as_ref()
            .map_or(0, |bundle| bundle.route_table.route_count()),
        product_hot_score_only_profile_count: product_hot_registry_runtime
            .as_ref()
            .map_or(0, |bundle| bundle.hot_runtime.profile_count()),
        product_hot_score_only_package_bytes: product_hot_registry_runtime
            .as_ref()
            .map_or(0, |bundle| bundle.package_bytes),
        final_hot_runtime_available,
        final_hot_route_ids,
        final_hot_profile_ids,
        final_hot_route_count,
        final_hot_profile_count,
        final_hot_route_profile_edges,
        hot_scratch_rebuilds,
        hot_scratch_candidate_capacity: hot_scratch
            .as_ref()
            .map_or(0, PhaseCenterHotScratch::candidate_capacity),
        hot_scratch_bytes_estimate: hot_scratch
            .as_ref()
            .map_or(0, PhaseCenterHotScratch::bytes_estimate),
        runtime_budget,
        product_hot_budget_passed,
        warm_miner_budget_passed,
        warm_miner_budget_blocker,
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        verifier_binding_bound: verifier_binding.is_bound(),
        candidate_package_count: candidate_package_reports.len(),
        candidate_package_dir: candidate_package_dir.display().to_string(),
        candidate_packages: candidate_package_reports,
        clean_promotion_manifest_path: clean_promotion_manifest_path.display().to_string(),
        clean_promotion_package_dir: clean_promotion_package_dir.display().to_string(),
        clean_promotion_manifest_written,
        call_token_promotion_manifest_path: call_token_promotion_manifest_path
            .display()
            .to_string(),
        call_token_promotion_package_dir: call_token_promotion_package_dir.display().to_string(),
        call_token_promotion_manifest_written,
        future_shadow,
        future_shadow_route_level_scoring_used: true,
        future_shadow_billing_request_path: future_shadow_billing_request_path
            .display()
            .to_string(),
        future_shadow_billing_request_rows: future_shadow_billing_request.rows,
        future_shadow_billing_request_tokens: future_shadow_billing_request.tokens,
        future_shadow_billing_request_current_cost_microusd: future_shadow_billing_request
            .current_cost_microusd,
        future_shadow_billing_request_ready_for_external_provider_evidence:
            future_shadow_billing_request.ready_for_external_provider_evidence,
        provider_evidence_artifacts: provider_evidence_artifacts.clone(),
        provider_export_drop_path: provider_evidence_artifacts
            .provider_export_drop_path
            .clone(),
        provider_export_present: provider_evidence_artifacts.provider_export_present,
        provider_evidence_chain_report_path: provider_evidence_artifacts
            .evidence_chain_report_path
            .clone(),
        provider_billing_capture_contract_ready: provider_evidence_artifacts.capture_contract_ready,
        provider_market_money_claim_allowed: provider_evidence_artifacts.market_money_claim_allowed,
        provider_money_claim_blocker,
        tail_follow_mode_used: true,
        cold_adapter_json_used: true,
        cold_adapter_strings_used: true,
        timed_lane_json_used: false,
        timed_lane_string_route_used: false,
        timed_lane_btreemap_used: false,
        timed_lane_file_io_used: false,
        direct_mutable_store_used: true,
        score_before_update_used: true,
        quarantine_nwpc_checkpoint_compile_used: true,
        registry_mutation_enabled: false,
        cpu_profile_registry_write_enabled: false,
        serving_profile_artifact_written: false,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        verdict: if adapter_gate_passed {
            "HOT_PATH_DAEMON_APPEND_LIVE_TAIL_PASS"
        } else {
            "HOT_PATH_DAEMON_APPEND_LIVE_TAIL_WATCH"
        },
        blocker,
        boundary: "append live-tail source adapter: watermark events initialize PhaseCenterLiveOperatorStore, then the command seeks to the end of an append-only JSONL file and waits for new lines; appended events are scored before update and then observed into the mutable phase-center store; verifier-bound quarantine .nwpc candidates are exported on the cold/report side only; no registry mutation, serving profile write, product promotion, local_accept, or market money claim occurs",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_hot_path_daemon_append_live_tail_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  decision_log_path: {}", decision_log_path.display());
    println!("  append_tail_path: {}", append_tail_path.display());
    println!("  append_parsed_rows: {}", report.append_parsed_rows);
    println!(
        "  append_score_events_before_update: {}",
        report.append_score_events_before_update
    );
    println!(
        "  append_unique_cpu_accepts_over_exact_cache: {}",
        report.append_unique_cpu_accepts_over_exact_cache
    );
    println!("  append_false_accepts: {}", report.append_false_accepts);
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_hot_path_daemon_live_loop_numeric_benchmark_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_HOT_PATH_DAEMON_LIVE_LOOP_NUMERIC_BENCHMARK_REPORT)
    });
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_CELLS);
    let min_bucket_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid min_bucket_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    let watermark_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL));
    let append_trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    if min_bucket_events == 0 {
        return Err("min_bucket_events must be > 0".to_owned());
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
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_PROFILES_PER_WORKER,
            max_hot_bytes_per_worker: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
            max_warm_profiles_per_process:
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
            max_profiles_per_route: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_PROFILES_PER_ROUTE,
            max_route_top_k: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_ROUTE_TOP_K,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .map_err(|error| format!("failed to create numeric benchmark store: {error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create numeric benchmark encoder: {error:?}"))?;
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut watermark_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut watermark_total_rows = 0usize;
    let mut watermark_parsed_rows = 0usize;
    let mut watermark_skipped_no_verifier_label = 0usize;
    let mut watermark_skipped_no_safe_atoms = 0usize;

    if watermark_trace_path == Path::new("-") {
        let stdin = io::stdin();
        let reader = stdin.lock();
        live_store_observe_live_loop_budget_events(
            "<stdin-watermark>",
            reader,
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
    } else {
        let file = File::open(&watermark_trace_path).map_err(|error| {
            format!(
                "failed to open watermark trace '{}': {error}",
                watermark_trace_path.display()
            )
        })?;
        let reader = io::BufReader::new(file);
        live_store_observe_live_loop_budget_events(
            &watermark_trace_path.display().to_string(),
            reader,
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
        if append_trace_path == Path::new("-") {
            let stdin = io::stdin();
            let reader = stdin.lock();
            live_store_collect_append_shadow_events(
                "<stdin-append>",
                reader,
                &bucket_policy,
                &mut exact_cache_keys_seen,
                &mut append_events,
                &mut append_total_rows,
                &mut append_parsed_rows,
                &mut append_skipped_no_verifier_label,
                &mut append_skipped_no_safe_atoms,
            )?;
        } else {
            let file = File::open(append_trace_path).map_err(|error| {
                format!(
                    "failed to open append trace '{}': {error}",
                    append_trace_path.display()
                )
            })?;
            let reader = io::BufReader::new(file);
            live_store_collect_append_shadow_events(
                &append_trace_path.display().to_string(),
                reader,
                &bucket_policy,
                &mut exact_cache_keys_seen,
                &mut append_events,
                &mut append_total_rows,
                &mut append_parsed_rows,
                &mut append_skipped_no_verifier_label,
                &mut append_skipped_no_safe_atoms,
            )?;
        }
    }

    let mut latencies = Vec::with_capacity(append_events.len());
    let mut active_before_update_events = 0usize;
    let mut calibration_events = 0usize;
    let mut raw_local_operator_events = 0usize;
    let mut local_operator_shadow_decisions = 0usize;
    let mut unique_cpu_accepts_over_exact_cache = 0usize;
    let mut tokens_saved = 0u64;
    let mut cost_saved_microusd = 0u64;
    let mut false_accepts = 0usize;
    let mut append_denominator = LiveStoreHotPathDenominator::default();

    for event in &append_events {
        append_denominator.total_tokens =
            append_denominator.total_tokens.saturating_add(event.tokens);
        append_denominator.total_cost_microusd = append_denominator
            .total_cost_microusd
            .saturating_add(event.cost_microusd);
        if event.exact_cache_hit {
            append_denominator.exact_cache_hits += 1;
            append_denominator.exact_cache_tokens = append_denominator
                .exact_cache_tokens
                .saturating_add(event.tokens);
            append_denominator.exact_cache_cost_microusd = append_denominator
                .exact_cache_cost_microusd
                .saturating_add(event.cost_microusd);
        } else {
            append_denominator.non_exact_rows += 1;
        }

        let start = Instant::now();
        let decision = store
            .observe_atom_event(&mut encoder, event.to_live_operator_atom_event())
            .map_err(|error| format!("numeric live-loop observe failed: {error:?}"))?;
        latencies.push(start.elapsed().as_nanos());
        active_before_update_events += usize::from(decision.active_before_update);
        calibration_events += usize::from(decision.calibration_event);
        raw_local_operator_events += usize::from(decision.raw_local_operator);
        local_operator_shadow_decisions += usize::from(decision.local_operator_shadow_decision);
        unique_cpu_accepts_over_exact_cache +=
            usize::from(decision.unique_cpu_accept_over_exact_cache);
        false_accepts += usize::from(decision.false_accept);
        if decision.unique_cpu_accept_over_exact_cache {
            tokens_saved = tokens_saved.saturating_add(event.tokens);
            cost_saved_microusd = cost_saved_microusd.saturating_add(event.cost_microusd);
        }
        bucket_policy.observe_decision(event, decision);
    }
    latencies.sort_unstable();

    let runtime_budget = live_store_budget_report(store.runtime_budget_snapshot());
    let verifier_binding = live_store_verifier_binding();
    let mut candidate_packages = Vec::new();
    store
        .candidate_packages_into_with_verifier(verifier_binding, &mut candidate_packages)
        .map_err(|error| {
            format!("failed to build numeric live-loop verifier-bound candidates: {error:?}")
        })?;
    let candidate_package_dir = live_store_numeric_candidate_package_dir(&report_path);
    let candidate_package_reports =
        write_live_store_candidate_packages(&candidate_package_dir, candidate_packages)?;
    let (
        hot_runtime_available_after_benchmark,
        hot_route_ids_after_benchmark,
        hot_profile_ids_after_benchmark,
    ) = if let Some((hot_runtime, route_table)) = store
        .candidate_hot_runtime_and_route_table()
        .map_err(|error| format!("failed to build post-benchmark hot view: {error:?}"))?
    {
        (
            true,
            live_store_hot_route_ids(&route_table),
            live_store_hot_profile_ids(&hot_runtime),
        )
    } else {
        (false, Vec::new(), Vec::new())
    };
    let exact_cache_overlap_excluded = append_denominator.non_exact_rows > 0
        && unique_cpu_accepts_over_exact_cache <= append_denominator.non_exact_rows;
    let token_cost_denominator_present =
        append_denominator.total_tokens > 0 && append_denominator.total_cost_microusd > 0;
    let threshold_policy = store.threshold_policy_evidence();
    let promotion_evidence = PhaseCenterPromotionEvidence {
        future_shadow_events: append_parsed_rows,
        unique_cpu_accepts_over_exact_cache,
        tokens_saved,
        cost_saved_microusd,
        false_accepts,
        runtime_margin_parity_mismatches: 0,
        verifier_binding,
        threshold_policy,
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        local_accept_enabled: false,
    };
    let promotion_decision = promotion_evidence.evaluate();
    let p99_latency_ns = live_store_latency_percentile(&latencies, 99);
    let gate_passed = append_parsed_rows > 0
        && active_before_update_events > 0
        && unique_cpu_accepts_over_exact_cache > 0
        && tokens_saved > 0
        && cost_saved_microusd > 0
        && false_accepts == 0
        && p99_latency_ns <= DEFAULT_ONLINE_MINER_NUMERIC_LANE_P99_BUDGET_NS
        && hot_runtime_available_after_benchmark
        && runtime_budget.product_runtime_budget_passed
        && exact_cache_overlap_excluded
        && token_cost_denominator_present;
    let blocker = if gate_passed {
        "none".to_owned()
    } else if append_parsed_rows == 0 {
        "numeric_live_loop_no_append_events".to_owned()
    } else if active_before_update_events == 0 {
        "numeric_live_loop_no_score_before_update_events".to_owned()
    } else if false_accepts != 0 {
        "numeric_live_loop_false_accepts_nonzero".to_owned()
    } else if unique_cpu_accepts_over_exact_cache == 0 {
        "numeric_live_loop_unique_accepts_zero".to_owned()
    } else if tokens_saved == 0 || cost_saved_microusd == 0 {
        "numeric_live_loop_token_cost_denominator_missing".to_owned()
    } else if p99_latency_ns > DEFAULT_ONLINE_MINER_NUMERIC_LANE_P99_BUDGET_NS {
        "numeric_live_loop_p99_budget_exceeded".to_owned()
    } else if !hot_runtime_available_after_benchmark {
        "numeric_live_loop_no_hot_view_after_benchmark".to_owned()
    } else if !runtime_budget.product_runtime_budget_passed {
        "numeric_live_loop_runtime_budget_failed".to_owned()
    } else if !exact_cache_overlap_excluded || !token_cost_denominator_present {
        "numeric_live_loop_denominator_missing".to_owned()
    } else {
        "numeric_live_loop_benchmark_failed".to_owned()
    };

    let report = PhaseStreamHotPathDaemonLiveLoopNumericBenchmarkReport {
        report_kind: "phase_stream_hot_path_daemon_live_loop_numeric_benchmark_v1",
        mode: "timed_numeric_phase_center_live_store_observe_atom_event",
        watermark_trace_paths: vec![watermark_trace_path.display().to_string()],
        append_trace_paths: append_trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        cells,
        min_bucket_events,
        watermark_total_rows,
        watermark_parsed_rows,
        append_total_rows,
        append_parsed_rows,
        timed_events: append_events.len(),
        active_before_update_events,
        calibration_events,
        raw_local_operator_events,
        local_operator_shadow_decisions,
        unique_cpu_accepts_over_exact_cache,
        tokens_saved,
        cost_saved_microusd,
        false_accepts,
        append_total_tokens: append_denominator.total_tokens,
        append_total_cost_microusd: append_denominator.total_cost_microusd,
        append_exact_cache_hits: append_denominator.exact_cache_hits,
        append_exact_cache_tokens: append_denominator.exact_cache_tokens,
        append_exact_cache_cost_microusd: append_denominator.exact_cache_cost_microusd,
        append_non_exact_rows: append_denominator.non_exact_rows,
        p50_latency_ns: live_store_latency_percentile(&latencies, 50),
        p90_latency_ns: live_store_latency_percentile(&latencies, 90),
        p99_latency_ns,
        max_latency_ns: latencies.last().copied().unwrap_or(0),
        timed_lane_p99_budget_ns: DEFAULT_ONLINE_MINER_NUMERIC_LANE_P99_BUDGET_NS,
        runtime_budget,
        hot_runtime_available_after_benchmark,
        hot_route_ids_after_benchmark,
        hot_profile_ids_after_benchmark,
        verifier_binding_bound: verifier_binding.is_bound(),
        candidate_package_count: candidate_package_reports.len(),
        candidate_package_dir: candidate_package_dir.display().to_string(),
        candidate_packages: candidate_package_reports,
        threshold_candidate_bucket_count: threshold_policy.candidate_bucket_count,
        threshold_auto_calibrated_bucket_count: threshold_policy.auto_calibrated_bucket_count,
        threshold_calibration_window_before_shadow: threshold_policy
            .calibration_window_before_shadow,
        threshold_shadow_window_after_calibration: threshold_policy.shadow_window_after_calibration,
        threshold_per_bucket_reported: threshold_policy.per_bucket_thresholds_reported,
        threshold_fixed_policy_shadow_replay: threshold_policy.fixed_policy_shadow_replay,
        promotion_evidence_eligible: promotion_decision.eligible,
        promotion_evidence_blocker: promotion_decision
            .blocker
            .map(live_store_promotion_blocker_name)
            .unwrap_or("none"),
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        cold_adapter_json_used: true,
        cold_adapter_strings_used: true,
        timed_lane_json_used: false,
        timed_lane_string_route_used: false,
        timed_lane_btreemap_used: false,
        timed_lane_file_io_used: false,
        timed_lane_report_aggregation_used: false,
        timed_lane_package_compile_used: false,
        timed_lane_numeric_route_bucket_atom_ids_used: true,
        direct_mutable_store_used: true,
        score_before_update_used: true,
        registry_mutation_enabled: false,
        cpu_profile_registry_write_enabled: false,
        serving_profile_artifact_written: false,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        verdict: if gate_passed {
            "HOT_PATH_DAEMON_LIVE_LOOP_NUMERIC_BENCHMARK_PASS"
        } else {
            "HOT_PATH_DAEMON_LIVE_LOOP_NUMERIC_BENCHMARK_WATCH"
        },
        blocker,
        boundary: "numeric live-loop benchmark only: watermark and append traces are parsed by the cold adapter before timing; the timed lane only feeds prepared route_id/bucket_id/atom_ids into PhaseCenterLiveOperatorStore::observe_atom_event for score-before-update plus center update, with no JSON, string route keys, BTreeMap, file IO, report aggregation, package compile, registry mutation, product promotion, local_accept, or market money claim",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_hot_path_daemon_live_loop_numeric_benchmark_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  timed_events: {}", report.timed_events);
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!("  p99_latency_ns: {}", report.p99_latency_ns);
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_hot_path_daemon_numeric_package_shadow_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let source_report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_HOT_PATH_DAEMON_LIVE_LOOP_NUMERIC_BENCHMARK_REPORT)
    });
    let audit_report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_HOT_PATH_DAEMON_NUMERIC_PACKAGE_SHADOW_AUDIT_REPORT)
    });
    let source = super::read_json_value(&source_report_path)?;
    let candidate_index = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid candidate index '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(0);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let candidates = super::json_at(&source, &["candidate_packages"])
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            format!(
                "numeric package shadow audit source '{}' missing candidate_packages",
                source_report_path.display()
            )
        })?;
    let candidate = candidates.get(candidate_index).ok_or_else(|| {
        format!(
            "numeric package shadow audit candidate index {candidate_index} out of range for {} candidates",
            candidates.len()
        )
    })?;
    let candidate_bucket_id = super::json_u64(candidate, &["bucket_id"])
        .ok_or_else(|| "numeric package shadow audit candidate missing bucket_id".to_owned())?
        as u32;
    let report_candidate_route_id =
        super::json_u64(candidate, &["route_id"]).unwrap_or_default() as u32;
    let threshold_micro = super::json_i64(candidate, &["threshold_micro"]).ok_or_else(|| {
        "numeric package shadow audit candidate missing threshold_micro".to_owned()
    })?;
    let candidate_package_path = super::json_string(candidate, &["package_path"])
        .map(PathBuf::from)
        .ok_or_else(|| "numeric package shadow audit candidate missing package_path".to_owned())?;
    let report_package_bytes =
        super::json_u64(candidate, &["package_bytes"]).unwrap_or_default() as usize;
    let report_package_fingerprint64 =
        super::json_u64(candidate, &["package_fingerprint64"]).unwrap_or_default();
    let report_package_records =
        super::json_u64(candidate, &["record_count"]).unwrap_or_default() as usize;
    let report_verifier_bound = super::json_bool(candidate, &["verifier_bound"]).unwrap_or(false);

    let cells =
        super::json_u64(&source, &["cells"]).unwrap_or(super::DEFAULT_CELLS as u64) as usize;
    let min_bucket_events = super::json_u64(&source, &["min_bucket_events"])
        .unwrap_or(super::DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS as u64)
        as usize;
    let mut watermark_trace_paths =
        super::json_string_vec(super::json_at(&source, &["watermark_trace_paths"]))
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
    if watermark_trace_paths.is_empty() {
        if let Some(path) = super::json_string(&source, &["watermark_trace_path"]) {
            watermark_trace_paths.push(PathBuf::from(path));
        }
    }
    let mut append_trace_paths =
        super::json_string_vec(super::json_at(&source, &["append_trace_paths"]))
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
    if append_trace_paths.is_empty() {
        if let Some(path) = super::json_string(&source, &["append_tail_path"]) {
            append_trace_paths.push(PathBuf::from(path));
        }
    }
    if watermark_trace_paths.is_empty() || append_trace_paths.is_empty() {
        return Err(
            "numeric package shadow audit needs watermark and append trace paths".to_owned(),
        );
    }

    let package_bytes = std::fs::read(&candidate_package_path).map_err(|error| {
        format!(
            "failed to read numeric shadow package '{}': {error}",
            candidate_package_path.display()
        )
    })?;
    let package_info = PhaseCenterFlatRuntime::inspect_bytes(&package_bytes)
        .map_err(|error| format!("numeric shadow package inspect error: {error:?}"))?;
    let flat_runtime = PhaseCenterFlatRuntime::from_bytes(&package_bytes)
        .map_err(|error| format!("numeric shadow package load error: {error:?}"))?;
    let package_matches_report = report_package_fingerprint64 == package_info.fingerprint64
        && report_package_bytes == package_bytes.len()
        && report_package_records == package_info.record_count;

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
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_PROFILES_PER_WORKER,
            max_hot_bytes_per_worker: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
            max_warm_profiles_per_process:
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
            max_profiles_per_route: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_PROFILES_PER_ROUTE,
            max_route_top_k: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_ROUTE_TOP_K,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .map_err(|error| format!("failed to create numeric package audit store: {error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create numeric package audit encoder: {error:?}"))?;
    let mut bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut watermark_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut watermark_total_rows = 0usize;
    let mut watermark_parsed_rows = 0usize;
    let mut watermark_skipped_no_verifier_label = 0usize;
    let mut watermark_skipped_no_safe_atoms = 0usize;
    for watermark_trace_path in &watermark_trace_paths {
        let file = File::open(watermark_trace_path).map_err(|error| {
            format!(
                "failed to open numeric package audit watermark trace '{}': {error}",
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
                "failed to open numeric package audit append trace '{}': {error}",
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

    let mut append_denominator = LiveStoreHotPathDenominator::default();
    let mut matching_route_ids = BTreeSet::new();
    let mut append_matching_bucket_events = 0usize;
    let mut append_non_matching_bucket_events = 0usize;
    for event in &append_events {
        append_denominator.total_tokens =
            append_denominator.total_tokens.saturating_add(event.tokens);
        append_denominator.total_cost_microusd = append_denominator
            .total_cost_microusd
            .saturating_add(event.cost_microusd);
        if event.exact_cache_hit {
            append_denominator.exact_cache_hits += 1;
            append_denominator.exact_cache_tokens = append_denominator
                .exact_cache_tokens
                .saturating_add(event.tokens);
            append_denominator.exact_cache_cost_microusd = append_denominator
                .exact_cache_cost_microusd
                .saturating_add(event.cost_microusd);
        } else {
            append_denominator.non_exact_rows += 1;
        }
        if event.bucket_id == candidate_bucket_id {
            append_matching_bucket_events += 1;
            matching_route_ids.insert(event.route_id);
        } else {
            append_non_matching_bucket_events += 1;
        }
    }
    let source_route_ids = super::json_at(&source, &["hot_route_ids_after_benchmark"])
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_u64().and_then(|id| u32::try_from(id).ok()))
        .collect::<Vec<_>>();
    let candidate_route_id = (report_candidate_route_id != 0)
        .then_some(report_candidate_route_id)
        .or_else(|| matching_route_ids.iter().copied().next())
        .or_else(|| source_route_ids.first().copied())
        .unwrap_or(0);

    let mut atom_eval = LiveStorePreparedHotPackEval::default();
    let mut prepared_eval = LiveStorePreparedHotPackEval::default();
    let mut runtime_margin_parity_checks = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut runtime_decision_parity_mismatches = 0usize;
    let mut route_index_missing_events = 0usize;
    let mut atom_latencies = Vec::new();
    let mut prepared_latencies = Vec::new();
    let mut package_loaded_into_hot_runtime = false;
    let mut worker = if package_info.record_count == 1 && candidate_route_id != 0 {
        let profile_ids = [candidate_bucket_id];
        let thresholds = [threshold_micro];
        let hot_runtime =
            PhaseCenterHotRuntime::from_flat_runtime(&flat_runtime, &profile_ids, &thresholds)
                .map_err(|error| format!("numeric package hot runtime build error: {error:?}"))?;
        let route_plan = hot_runtime
            .route_plan_from_profile_ids(candidate_route_id, profile_ids)
            .map_err(|error| format!("numeric package hot route plan error: {error:?}"))?
            .ok_or_else(|| "numeric package hot route has no profiles".to_owned())?;
        let route_table = PhaseCenterHotRouteTable::from_plans([route_plan])
            .map_err(|error| format!("numeric package hot route table error: {error:?}"))?;
        package_loaded_into_hot_runtime = true;
        Some(
            PhaseCenterHotWorker::new(hot_runtime, route_table)
                .map_err(|error| format!("numeric package hot worker error: {error:?}"))?,
        )
    } else {
        None
    };

    let mut reference_encoder = PhaseCenterAtomEncoder::new(cells).map_err(|error| {
        format!("failed to create numeric package audit reference encoder: {error:?}")
    })?;
    for event in append_events
        .iter()
        .filter(|event| event.bucket_id == candidate_bucket_id)
    {
        let Some(worker) = worker.as_mut() else {
            break;
        };
        let Some(route_index) = worker.resolve_route_index(event.route_id) else {
            route_index_missing_events += 1;
            continue;
        };
        let reference_vector = reference_encoder
            .encode_atom_ids(event.atom_ids.iter().copied())
            .map_err(|error| format!("numeric package reference encode error: {error:?}"))?;
        let phase_vector = reference_vector.to_vec();
        let reference_margin_micro = flat_runtime
            .score_vector_margin_micro(0, &phase_vector)
            .map_err(|error| format!("numeric package reference score error: {error:?}"))?;
        let reference_score_candidate = reference_margin_micro >= threshold_micro;
        let atom_started = Instant::now();
        let atom_decisions = worker
            .score_live_atom_event_with_evidence(
                event.to_live_operator_atom_event(),
                &mut atom_eval,
            )
            .map_err(|error| format!("numeric package hot worker score error: {error:?}"))?;
        atom_latencies.push(atom_started.elapsed().as_nanos());
        if atom_decisions.is_none() {
            route_index_missing_events += 1;
            continue;
        }
        let prepared_row = LiveStorePreparedMemoryRow::new(
            route_index,
            event.atom_ids.clone(),
            phase_vector,
            event.hot_request_evidence(),
        );
        let prepared_started = Instant::now();
        let decisions = worker
            .score_prepared_row_with_evidence(&prepared_row, &mut prepared_eval)
            .map_err(|error| format!("numeric package prepared hot score error: {error:?}"))?;
        prepared_latencies.push(prepared_started.elapsed().as_nanos());
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
    atom_latencies.sort_unstable();
    prepared_latencies.sort_unstable();

    let source_report_kind = super::json_string(&source, &["report_kind"]).unwrap_or_default();
    let source_verdict = super::json_string(&source, &["verdict"]).unwrap_or_default();
    let source_blocker = super::json_string(&source, &["blocker"]).unwrap_or_default();
    let source_local_accept_enabled =
        super::json_bool(&source, &["local_accept_enabled"]).unwrap_or(true);
    let source_market_money_claim_allowed =
        super::json_bool(&source, &["market_money_claim_allowed"]).unwrap_or(true);
    let source_numeric_benchmark_passed = source_verdict
        == "HOT_PATH_DAEMON_LIVE_LOOP_NUMERIC_BENCHMARK_PASS"
        && source_blocker == "none";
    let source_live_tail_shadow_ready = source_report_kind
        == "phase_stream_hot_path_daemon_append_live_tail_v1"
        && (source_verdict == "HOT_PATH_DAEMON_APPEND_LIVE_TAIL_RUNNING"
            || source_verdict == "HOT_PATH_DAEMON_APPEND_LIVE_TAIL_PASS")
        && super::json_u64(&source, &["append_false_accepts"]).unwrap_or(1) == 0
        && super::json_bool(
            &source,
            &["runtime_budget", "product_runtime_budget_passed"],
        )
        .unwrap_or(false)
        && super::json_bool(&source, &["verifier_binding_bound"]).unwrap_or(false)
        && super::json_u64(&source, &["candidate_package_count"]).unwrap_or_default() > 0
        && !source_local_accept_enabled
        && !source_market_money_claim_allowed;
    let source_gate_passed = source_numeric_benchmark_passed || source_live_tail_shadow_ready;
    let source_promotion_evidence_eligible =
        super::json_bool(&source, &["promotion_evidence_eligible"])
            .unwrap_or(source_live_tail_shadow_ready);
    let exact_cache_overlap_excluded = append_denominator.non_exact_rows > 0
        && prepared_eval.unique_cpu_accepts_over_exact_cache <= append_denominator.non_exact_rows;
    let token_cost_denominator_present =
        append_denominator.total_tokens > 0 && append_denominator.total_cost_microusd > 0;
    let p99_latency_ns = live_store_latency_percentile(&prepared_latencies, 99);
    let p99_budget_ns = 1_000;
    let forbidden_flags = super::ForbiddenFlags {
        target_id_used: false,
        proof_rule_id_authority_used: false,
        concrete_x_lookup_used: false,
        manual_local_out_t_used: false,
        hidden_frame_id_or_bind_x_used: false,
        legacy_backend_used: false,
    };
    let gate_passed = source_gate_passed
        && source_promotion_evidence_eligible
        && !source_local_accept_enabled
        && !source_market_money_claim_allowed
        && report_verifier_bound
        && package_matches_report
        && package_loaded_into_hot_runtime
        && package_info.record_count == 1
        && candidate_route_id != 0
        && append_matching_bucket_events > 0
        && prepared_eval.score_events > 0
        && prepared_eval.score_candidate_events > 0
        && prepared_eval.false_accepts == 0
        && prepared_eval.local_accept_events == 0
        && runtime_margin_parity_checks == prepared_eval.score_events
        && runtime_margin_parity_mismatches == 0
        && runtime_decision_parity_mismatches == 0
        && exact_cache_overlap_excluded
        && token_cost_denominator_present
        && p99_latency_ns <= p99_budget_ns;
    let blocker = if gate_passed {
        "none".to_owned()
    } else if !source_gate_passed {
        "source_numeric_gate_not_passed".to_owned()
    } else if !source_promotion_evidence_eligible {
        "source_promotion_evidence_not_eligible".to_owned()
    } else if source_local_accept_enabled || source_market_money_claim_allowed {
        "source_product_accept_or_market_claim_enabled".to_owned()
    } else if !report_verifier_bound {
        "candidate_not_verifier_bound".to_owned()
    } else if !package_matches_report {
        "package_inspect_mismatch".to_owned()
    } else if package_info.record_count != 1 {
        "candidate_package_record_count_not_one".to_owned()
    } else if !package_loaded_into_hot_runtime {
        "package_not_loaded_into_hot_runtime".to_owned()
    } else if candidate_route_id == 0 {
        "candidate_bucket_route_ambiguous_or_missing".to_owned()
    } else if append_matching_bucket_events == 0 {
        "no_append_events_match_candidate_bucket".to_owned()
    } else if prepared_eval.score_events == 0 || prepared_eval.score_candidate_events == 0 {
        "package_shadow_no_score_candidates".to_owned()
    } else if prepared_eval.false_accepts != 0 {
        "package_shadow_false_accepts_nonzero".to_owned()
    } else if prepared_eval.local_accept_events != 0 {
        "package_shadow_local_accept_enabled".to_owned()
    } else if runtime_margin_parity_checks != prepared_eval.score_events
        || runtime_margin_parity_mismatches != 0
        || runtime_decision_parity_mismatches != 0
    {
        "package_shadow_runtime_parity_failed".to_owned()
    } else if !exact_cache_overlap_excluded || !token_cost_denominator_present {
        "package_shadow_denominator_missing".to_owned()
    } else if p99_latency_ns > p99_budget_ns {
        "package_shadow_hot_path_p99_budget_exceeded".to_owned()
    } else {
        "package_shadow_audit_failed".to_owned()
    };

    let report = PhaseStreamHotPathDaemonNumericPackageShadowAuditReport {
        report_kind: "phase_stream_hot_path_daemon_numeric_package_shadow_audit_v1",
        mode: "quarantine_nwpc_package_load_shadow_audit_numeric_ids_only",
        source_numeric_report_path: source_report_path.display().to_string(),
        candidate_package_path: candidate_package_path.display().to_string(),
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
        candidate_bucket_id,
        candidate_route_id,
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: package_bytes.len(),
        package_record_count: package_info.record_count,
        threshold_micro,
        package_matches_report,
        verifier_binding_bound: report_verifier_bound,
        watermark_parsed_rows,
        append_parsed_rows,
        append_matching_bucket_events,
        append_non_matching_bucket_events,
        route_index_missing_events,
        score_events: prepared_eval.score_events,
        score_candidate_events: prepared_eval.score_candidate_events,
        verifier_required_events: prepared_eval.verifier_required_events,
        local_accept_events: prepared_eval.local_accept_events,
        unique_cpu_accepts_over_exact_cache: prepared_eval.unique_cpu_accepts_over_exact_cache,
        tokens_saved: prepared_eval.tokens_saved,
        cost_saved_microusd: prepared_eval.cost_saved_microusd,
        false_accepts: prepared_eval.false_accepts,
        runtime_margin_parity_checks,
        runtime_margin_parity_mismatches,
        runtime_decision_parity_mismatches,
        append_total_tokens: append_denominator.total_tokens,
        append_total_cost_microusd: append_denominator.total_cost_microusd,
        append_exact_cache_hits: append_denominator.exact_cache_hits,
        append_exact_cache_tokens: append_denominator.exact_cache_tokens,
        append_exact_cache_cost_microusd: append_denominator.exact_cache_cost_microusd,
        append_non_exact_rows: append_denominator.non_exact_rows,
        p50_latency_ns: live_store_latency_percentile(&prepared_latencies, 50),
        p90_latency_ns: live_store_latency_percentile(&prepared_latencies, 90),
        p99_latency_ns,
        max_latency_ns: prepared_latencies.last().copied().unwrap_or(0),
        p99_budget_ns,
        atom_worker_p50_latency_ns: live_store_latency_percentile(&atom_latencies, 50),
        atom_worker_p90_latency_ns: live_store_latency_percentile(&atom_latencies, 90),
        atom_worker_p99_latency_ns: live_store_latency_percentile(&atom_latencies, 99),
        atom_worker_max_latency_ns: atom_latencies.last().copied().unwrap_or(0),
        hot_profile_count: worker
            .as_ref()
            .map_or(0, PhaseCenterHotWorker::profile_count),
        hot_route_count: worker.as_ref().map_or(0, PhaseCenterHotWorker::route_count),
        hot_route_profile_edges: worker
            .as_ref()
            .map_or(0, PhaseCenterHotWorker::route_profile_edge_count),
        hot_bytes_estimate: worker
            .as_ref()
            .map_or(0, PhaseCenterHotWorker::bytes_estimate),
        exact_cache_overlap_excluded,
        token_cost_denominator_present,
        package_loaded_into_hot_runtime,
        source_numeric_gate_passed: source_gate_passed,
        source_promotion_evidence_eligible,
        source_local_accept_enabled,
        source_market_money_claim_allowed,
        market_savings_count_allowed: false,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        forbidden_flags,
        verdict: if gate_passed {
            "HOT_PATH_DAEMON_NUMERIC_PACKAGE_SHADOW_AUDIT_PASS"
        } else {
            "HOT_PATH_DAEMON_NUMERIC_PACKAGE_SHADOW_AUDIT_WATCH"
        },
        blocker,
        boundary: "numeric package shadow audit only: loads one quarantine .nwpc into PhaseCenterHotRuntime by numeric route_id/profile_id and shadow-scores matching append rows; this is package-load/parity evidence, not fresh-future market savings, and it does not compile, promote, write serving profiles, enable local_accept, allow market claims, or use legacy nwrb/role-binding paths",
    };
    super::write_json_file(&audit_report_path, &report)?;
    println!("phase_stream_hot_path_daemon_numeric_package_shadow_audit_v1:");
    println!("  report_path: {}", audit_report_path.display());
    println!(
        "  append_matching_bucket_events: {}",
        report.append_matching_bucket_events
    );
    println!(
        "  score_candidate_events: {}",
        report.score_candidate_events
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!("  p99_latency_ns: {}", report.p99_latency_ns);
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_hot_path_daemon_numeric_future_package_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_HOT_PATH_DAEMON_NUMERIC_FUTURE_PACKAGE_AUDIT_REPORT)
    });
    let daemon_admission_policy_report_path =
        live_store_hot_path_daemon_admission_policy_path(&report_path);
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_CELLS);
    let min_bucket_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid min_bucket_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    let watermark_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL));
    let append_trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    if min_bucket_events == 0 {
        return Err("min_bucket_events must be > 0".to_owned());
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
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_PROFILES_PER_WORKER,
            max_hot_bytes_per_worker: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
            max_warm_profiles_per_process:
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
            max_profiles_per_route: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_PROFILES_PER_ROUTE,
            max_route_top_k: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_ROUTE_TOP_K,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .map_err(|error| format!("failed to create numeric future package store: {error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create numeric future package encoder: {error:?}"))?;
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut watermark_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut watermark_total_rows = 0usize;
    let mut watermark_parsed_rows = 0usize;
    let mut watermark_skipped_no_verifier_label = 0usize;
    let mut watermark_skipped_no_safe_atoms = 0usize;

    let file = File::open(&watermark_trace_path).map_err(|error| {
        format!(
            "failed to open future package watermark trace '{}': {error}",
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

    let mut append_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut append_total_rows = 0usize;
    let mut append_parsed_rows = 0usize;
    let mut append_skipped_no_verifier_label = 0usize;
    let mut append_skipped_no_safe_atoms = 0usize;
    for append_trace_path in &append_trace_paths {
        let file = File::open(append_trace_path).map_err(|error| {
            format!(
                "failed to open future package append trace '{}': {error}",
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

    let verifier_binding = live_store_verifier_binding();
    let candidate_package_dir = live_store_numeric_future_candidate_package_dir(&report_path);
    let mut frozen_package = None::<PhaseCenterOnlineCandidatePackage>;
    let mut frozen_route_id = 0u32;
    let mut freeze_after_append_events = 0usize;
    let mut live_encoder = PhaseCenterAtomEncoder::new(cells).map_err(|error| {
        format!("failed to create numeric future package live encoder: {error:?}")
    })?;
    for (index, event) in append_events.iter().enumerate() {
        if frozen_package.is_some() {
            break;
        }
        let decision = store
            .observe_atom_event(&mut live_encoder, event.to_live_operator_atom_event())
            .map_err(|error| format!("future package live observe failed: {error:?}"))?;
        bucket_policy.observe_decision(event, decision);
        if !decision.unique_cpu_accept_over_exact_cache {
            continue;
        }
        let mut packages = Vec::new();
        store
            .candidate_packages_into_with_verifier(verifier_binding, &mut packages)
            .map_err(|error| format!("future package candidate build failed: {error:?}"))?;
        if let Some(package) = packages
            .into_iter()
            .find(|package| package.bucket_id == decision.bucket_id)
        {
            frozen_route_id = event.route_id;
            freeze_after_append_events = index + 1;
            frozen_package = Some(package);
        }
    }

    let mut eval = LiveStorePreparedHotPackEval::default();
    let mut future_denominator = LiveStoreHotPathDenominator::default();
    let mut future_matching_bucket_events = 0usize;
    let mut future_non_matching_bucket_events = 0usize;
    let mut runtime_margin_parity_checks = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut runtime_decision_parity_mismatches = 0usize;
    let mut latencies = Vec::new();
    let mut candidate_package_path = PathBuf::new();
    let mut package_fingerprint64 = 0u64;
    let mut package_bytes_len = 0usize;
    let mut package_record_count = 0usize;
    let mut candidate_bucket_id = 0u32;
    let mut threshold_micro = 0i64;
    let mut verifier_binding_bound = false;
    let mut worker = None::<PhaseCenterHotWorker>;
    let mut offload_runtime = None::<PhaseCenterOffloadRuntime>;

    if let Some(package) = frozen_package {
        std::fs::create_dir_all(&candidate_package_dir).map_err(|error| {
            format!(
                "failed to create future package candidate dir '{}': {error}",
                candidate_package_dir.display()
            )
        })?;
        candidate_package_path = candidate_package_dir.join(format!(
            "bucket-{:08x}-{:016x}.nwpc",
            package.bucket_id, package.package_info.fingerprint64
        ));
        std::fs::write(&candidate_package_path, &package.package_bytes).map_err(|error| {
            format!(
                "failed to write future candidate package '{}': {error}",
                candidate_package_path.display()
            )
        })?;
        candidate_bucket_id = package.bucket_id;
        threshold_micro = package.threshold_micro;
        package_fingerprint64 = package.package_info.fingerprint64;
        package_bytes_len = package.package_bytes.len();
        package_record_count = package.package_info.record_count;
        verifier_binding_bound = package.verifier_binding.is_bound();
        let runtime = PhaseCenterOffloadRuntime::from_package_bytes(
            &package.package_bytes,
            PhaseCenterOffloadPolicy::new(package.threshold_micro)
                .map_err(|error| format!("future package policy error: {error:?}"))?,
        )
        .map_err(|error| format!("future package load error: {error:?}"))?;
        let hot_runtime = PhaseCenterHotRuntime::from_flat_runtime(
            runtime.runtime(),
            &[candidate_bucket_id],
            &[threshold_micro],
        )
        .map_err(|error| format!("future package hot runtime build error: {error:?}"))?;
        let route_plan = hot_runtime
            .route_plan_from_profile_ids(frozen_route_id, [candidate_bucket_id])
            .map_err(|error| format!("future package route plan error: {error:?}"))?
            .ok_or_else(|| "future package hot route has no profiles".to_owned())?;
        let route_table = PhaseCenterHotRouteTable::from_plans([route_plan])
            .map_err(|error| format!("future package route table error: {error:?}"))?;
        worker = Some(
            PhaseCenterHotWorker::new(hot_runtime, route_table)
                .map_err(|error| format!("future package hot worker error: {error:?}"))?,
        );
        offload_runtime = Some(runtime);
    }

    let mut reference_encoder = PhaseCenterAtomEncoder::new(cells).map_err(|error| {
        format!("failed to create numeric future package reference encoder: {error:?}")
    })?;
    let future_events = append_events
        .get(freeze_after_append_events..)
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
        let Some(worker) = worker.as_mut() else {
            continue;
        };
        let Some(offload_runtime) = offload_runtime.as_ref() else {
            continue;
        };
        let Some(route_index) = worker.resolve_route_index(event.route_id) else {
            runtime_decision_parity_mismatches += 1;
            continue;
        };
        let vector = reference_encoder
            .encode_atom_ids(event.atom_ids.iter().copied())
            .map_err(|error| format!("future package reference encode error: {error:?}"))?
            .to_vec();
        let reference_margin_micro = offload_runtime
            .runtime()
            .score_vector_margin_micro(0, &vector)
            .map_err(|error| format!("future package reference score error: {error:?}"))?;
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
            .map_err(|error| format!("future package prepared score error: {error:?}"))?;
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
    let fresh_future_split_used =
        freeze_after_append_events > 0 && freeze_after_append_events < append_events.len();
    let p99_latency_ns = live_store_latency_percentile(&latencies, 99);
    let p99_budget_ns = 1_000;
    let hot_profile_count = worker
        .as_ref()
        .map_or(0, PhaseCenterHotWorker::profile_count);
    let hot_route_count = worker.as_ref().map_or(0, PhaseCenterHotWorker::route_count);
    let hot_route_profile_edges = worker
        .as_ref()
        .map_or(0, PhaseCenterHotWorker::route_profile_edge_count);
    let hot_bytes_estimate = worker
        .as_ref()
        .map_or(0, PhaseCenterHotWorker::bytes_estimate);
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
    } else if freeze_after_append_events == 0 {
        "future_package_no_candidate_before_future_window".to_owned()
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
        watermark_trace_paths: vec![watermark_trace_path.display().to_string()],
        append_trace_paths: append_trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        cells,
        min_bucket_events,
        watermark_parsed_rows,
        append_parsed_rows,
        freeze_after_append_events,
        candidate_bucket_id,
        candidate_route_id: frozen_route_id,
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
        forbidden_flags: super::ForbiddenFlags {
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
            vec![frozen_route_id]
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
        forbidden_flags: super::ForbiddenFlags {
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
    super::write_json_file(&report_path, &report)?;
    super::write_json_file(&daemon_admission_policy_report_path, &daemon_policy)?;
    println!("phase_stream_hot_path_daemon_numeric_future_package_audit_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  daemon_admission_policy_report_path: {}",
        daemon_admission_policy_report_path.display()
    );
    println!(
        "  freeze_after_append_events: {}",
        report.freeze_after_append_events
    );
    println!(
        "  future_matching_bucket_events: {}",
        report.future_matching_bucket_events
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!("  p99_latency_ns: {}", report.p99_latency_ns);
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_hot_path_daemon_numeric_future_portfolio_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_HOT_PATH_DAEMON_NUMERIC_FUTURE_PORTFOLIO_AUDIT_REPORT)
    });
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_CELLS);
    let min_bucket_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid min_bucket_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(super::DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    let watermark_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL));
    let append_trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(super::DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    if min_bucket_events == 0 {
        return Err("min_bucket_events must be > 0".to_owned());
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
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_PROFILES_PER_WORKER,
            max_hot_bytes_per_worker: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
            max_warm_profiles_per_process:
                super::DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
            max_profiles_per_route: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_PROFILES_PER_ROUTE,
            max_route_top_k: super::DEFAULT_PHASE_CENTER_SHADOW_MAX_ROUTE_TOP_K,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .map_err(|error| format!("failed to create numeric future portfolio store: {error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(cells)
        .map_err(|error| format!("failed to create numeric future portfolio encoder: {error:?}"))?;
    let mut exact_cache_keys_seen = BTreeSet::new();
    let mut bucket_policy = LiveStoreAdaptiveBucketPolicy::default();
    let mut watermark_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut watermark_total_rows = 0usize;
    let mut watermark_parsed_rows = 0usize;
    let mut watermark_skipped_no_verifier_label = 0usize;
    let mut watermark_skipped_no_safe_atoms = 0usize;
    let watermark_file = File::open(&watermark_trace_path).map_err(|error| {
        format!(
            "failed to open future portfolio watermark trace '{}': {error}",
            watermark_trace_path.display()
        )
    })?;
    live_store_observe_live_loop_budget_events(
        &watermark_trace_path.display().to_string(),
        io::BufReader::new(watermark_file),
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

    let mut append_events = Vec::<LiveStoreParsedAtomEvent>::new();
    let mut append_total_rows = 0usize;
    let mut append_parsed_rows = 0usize;
    let mut append_skipped_no_verifier_label = 0usize;
    let mut append_skipped_no_safe_atoms = 0usize;
    for append_trace_path in &append_trace_paths {
        let file = File::open(append_trace_path).map_err(|error| {
            format!(
                "failed to open future portfolio append trace '{}': {error}",
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

    let verifier_binding = live_store_verifier_binding();
    let mut live_encoder = PhaseCenterAtomEncoder::new(cells).map_err(|error| {
        format!("failed to create numeric future portfolio live encoder: {error:?}")
    })?;
    let mut frozen_bucket_ids = BTreeSet::<u32>::new();
    let mut frozen_packages = Vec::<LiveStoreFrozenNumericFuturePackage>::new();
    for (index, event) in append_events.iter().enumerate() {
        if frozen_packages.len() >= DEFAULT_HOT_PATH_DAEMON_NUMERIC_FUTURE_PORTFOLIO_MAX_CHILDREN {
            break;
        }
        let decision = store
            .observe_atom_event(&mut live_encoder, event.to_live_operator_atom_event())
            .map_err(|error| format!("future portfolio live observe failed: {error:?}"))?;
        bucket_policy.observe_decision(event, decision);
        if !decision.unique_cpu_accept_over_exact_cache
            || frozen_bucket_ids.contains(&decision.bucket_id)
        {
            continue;
        }
        let mut packages = Vec::new();
        store
            .candidate_packages_into_with_verifier(verifier_binding, &mut packages)
            .map_err(|error| format!("future portfolio candidate build failed: {error:?}"))?;
        if let Some(package) = packages
            .into_iter()
            .find(|package| package.bucket_id == decision.bucket_id)
        {
            frozen_bucket_ids.insert(decision.bucket_id);
            frozen_packages.push(LiveStoreFrozenNumericFuturePackage {
                package,
                route_id: event.route_id,
                freeze_after_append_events: index + 1,
            });
        }
    }

    let mut future_audit_report_paths = Vec::with_capacity(frozen_packages.len());
    let mut policy_smoke_report_paths = Vec::with_capacity(frozen_packages.len());
    for (index, frozen) in frozen_packages.into_iter().enumerate() {
        let future_report_path =
            live_store_numeric_future_portfolio_child_report_path(&report_path, index);
        live_store_write_numeric_future_package_audit_from_frozen(
            &future_report_path,
            std::slice::from_ref(&watermark_trace_path),
            &append_trace_paths,
            cells,
            min_bucket_events,
            watermark_parsed_rows,
            append_parsed_rows,
            &append_events,
            frozen,
        )?;
        let daemon_policy_path =
            live_store_hot_path_daemon_admission_policy_path(&future_report_path);
        let policy_smoke_path = live_store_numeric_future_policy_smoke_path(&future_report_path);
        run_phase_stream_hot_path_daemon_admission_policy_smoke_v1(
            vec![
                daemon_policy_path.display().to_string(),
                policy_smoke_path.display().to_string(),
            ]
            .into_iter(),
        )?;
        future_audit_report_paths.push(future_report_path);
        policy_smoke_report_paths.push(policy_smoke_path);
    }

    let portfolio_gate_report_path =
        live_store_numeric_future_portfolio_gate_report_path(&report_path);
    let runtime_replay_report_path =
        live_store_numeric_future_portfolio_runtime_replay_report_path(&report_path);
    if future_audit_report_paths.is_empty() {
        let report = PhaseStreamHotPathDaemonNumericFuturePortfolioAuditReport {
            report_kind: "phase_stream_hot_path_daemon_numeric_future_portfolio_audit_v1",
            mode: "orchestrate_multiple_fresh_future_nwpc_audits_into_shadow_only_portfolio",
            generated_future_audit_report_paths: Vec::new(),
            generated_policy_smoke_report_paths: Vec::new(),
            portfolio_gate_report_path: portfolio_gate_report_path.display().to_string(),
            runtime_replay_report_path: runtime_replay_report_path.display().to_string(),
            cells,
            min_bucket_events,
            watermark_trace_path: watermark_trace_path.display().to_string(),
            append_trace_paths: append_trace_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            generated_future_audit_count: 0,
            generated_policy_smoke_count: 0,
            portfolio_accepted_report_count: 0,
            portfolio_rejected_report_count: 0,
            portfolio_shadow_evidence_ready: false,
            runtime_replayed_report_count: 0,
            runtime_replay_failed_report_count: 0,
            accepted_unique_cpu_accepts_over_exact_cache: 0,
            accepted_tokens_saved: 0,
            accepted_cost_saved_microusd: 0,
            accepted_false_accepts: 0,
            runtime_total_unique_cpu_accepts_over_exact_cache: 0,
            runtime_total_tokens_saved: 0,
            runtime_total_cost_saved_microusd: 0,
            runtime_total_false_accepts: 0,
            runtime_total_margin_parity_mismatches: 0,
            runtime_total_decision_parity_mismatches: 0,
            runtime_total_mismatched_reports: 0,
            product_promotion_enabled: false,
            local_accept_enabled: false,
            market_money_claim_allowed: false,
            registry_mutation_enabled: false,
            cpu_profile_registry_write_enabled: false,
            serving_profile_artifact_written: false,
            forbidden_flags: super::ForbiddenFlags {
                target_id_used: false,
                proof_rule_id_authority_used: false,
                concrete_x_lookup_used: false,
                manual_local_out_t_used: false,
                hidden_frame_id_or_bind_x_used: false,
                legacy_backend_used: false,
            },
            verdict: "HOT_PATH_DAEMON_NUMERIC_FUTURE_PORTFOLIO_AUDIT_WATCH",
            blocker: "future_portfolio_no_frozen_packages",
            boundary: "portfolio audit only: creates fresh-future .nwpc child audits, policy smokes, portfolio gate, and runtime replay; it does not mutate registry, write serving profiles, promote product runtime, enable local_accept, allow market money claims, use lookup/target/proof authority, or reintroduce legacy nwrb/role-binding paths",
        };
        super::write_json_file(&report_path, &report)?;
        println!("phase_stream_hot_path_daemon_numeric_future_portfolio_audit_v1:");
        println!("  report_path: {}", report_path.display());
        println!("  portfolio_accepted_report_count: 0");
        println!("  accepted_unique_cpu_accepts_over_exact_cache: 0");
        println!("  accepted_false_accepts: 0");
        println!("  local_accept_enabled: false");
        println!("  market_money_claim_allowed: false");
        println!("  verdict: {}", report.verdict);
        println!("  blocker: {}", report.blocker);
        return Ok(());
    }

    let mut portfolio_args = Vec::with_capacity(future_audit_report_paths.len() + 1);
    portfolio_args.push(portfolio_gate_report_path.display().to_string());
    portfolio_args.extend(
        future_audit_report_paths
            .iter()
            .map(|path| path.display().to_string()),
    );
    run_phase_stream_hot_path_daemon_numeric_admission_portfolio_gate_v1(
        portfolio_args.into_iter(),
    )?;
    run_phase_stream_hot_path_daemon_numeric_admission_portfolio_runtime_replay_v1(
        vec![
            portfolio_gate_report_path.display().to_string(),
            runtime_replay_report_path.display().to_string(),
        ]
        .into_iter(),
    )?;

    let portfolio = super::read_json_value(&portfolio_gate_report_path)?;
    let runtime_replay = super::read_json_value(&runtime_replay_report_path)?;
    let portfolio_accepted_report_count =
        super::json_u64(&portfolio, &["accepted_report_count"]).unwrap_or_default() as usize;
    let portfolio_rejected_report_count =
        super::json_u64(&portfolio, &["rejected_report_count"]).unwrap_or_default() as usize;
    let portfolio_shadow_evidence_ready =
        super::json_bool(&portfolio, &["shadow_evidence_ready"]).unwrap_or(false);
    let accepted_unique_cpu_accepts_over_exact_cache = super::json_u64(
        &portfolio,
        &["accepted_unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or_default() as usize;
    let accepted_tokens_saved =
        super::json_u64(&portfolio, &["accepted_tokens_saved"]).unwrap_or_default();
    let accepted_cost_saved_microusd =
        super::json_u64(&portfolio, &["accepted_cost_saved_microusd"]).unwrap_or_default();
    let accepted_false_accepts = super::json_u64(&portfolio, &["accepted_false_accepts"])
        .unwrap_or(usize::MAX as u64) as usize;

    let runtime_replayed_report_count =
        super::json_u64(&runtime_replay, &["replayed_report_count"]).unwrap_or_default() as usize;
    let runtime_replay_failed_report_count =
        super::json_u64(&runtime_replay, &["replay_failed_report_count"]).unwrap_or_default()
            as usize;
    let runtime_total_unique_cpu_accepts_over_exact_cache = super::json_u64(
        &runtime_replay,
        &["total_unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or_default() as usize;
    let runtime_total_tokens_saved =
        super::json_u64(&runtime_replay, &["total_tokens_saved"]).unwrap_or_default();
    let runtime_total_cost_saved_microusd =
        super::json_u64(&runtime_replay, &["total_cost_saved_microusd"]).unwrap_or_default();
    let runtime_total_false_accepts = super::json_u64(&runtime_replay, &["total_false_accepts"])
        .unwrap_or(usize::MAX as u64) as usize;
    let runtime_total_margin_parity_mismatches =
        super::json_u64(&runtime_replay, &["total_runtime_margin_parity_mismatches"])
            .unwrap_or(usize::MAX as u64) as usize;
    let runtime_total_decision_parity_mismatches = super::json_u64(
        &runtime_replay,
        &["total_runtime_decision_parity_mismatches"],
    )
    .unwrap_or(usize::MAX as u64) as usize;
    let runtime_total_mismatched_reports = super::json_u64(
        &runtime_replay,
        &["total_mismatched_reports"],
    )
    .unwrap_or(usize::MAX as u64) as usize;

    let gate_passed = portfolio_shadow_evidence_ready
        && portfolio_accepted_report_count > 0
        && accepted_unique_cpu_accepts_over_exact_cache > 0
        && accepted_tokens_saved > 0
        && accepted_cost_saved_microusd > 0
        && accepted_false_accepts == 0
        && runtime_replayed_report_count == portfolio_accepted_report_count
        && runtime_replay_failed_report_count == 0
        && runtime_total_unique_cpu_accepts_over_exact_cache
            == accepted_unique_cpu_accepts_over_exact_cache
        && runtime_total_tokens_saved == accepted_tokens_saved
        && runtime_total_cost_saved_microusd == accepted_cost_saved_microusd
        && runtime_total_false_accepts == 0
        && runtime_total_margin_parity_mismatches == 0
        && runtime_total_decision_parity_mismatches == 0
        && runtime_total_mismatched_reports == 0;
    let blocker = if gate_passed {
        "none"
    } else if portfolio_accepted_report_count == 0 {
        "future_portfolio_no_admitted_packages"
    } else if accepted_false_accepts != 0 || runtime_total_false_accepts != 0 {
        "future_portfolio_false_accepts_nonzero"
    } else if runtime_replay_failed_report_count != 0
        || runtime_total_margin_parity_mismatches != 0
        || runtime_total_decision_parity_mismatches != 0
        || runtime_total_mismatched_reports != 0
    {
        "future_portfolio_runtime_replay_or_parity_failed"
    } else if accepted_tokens_saved == 0
        || accepted_cost_saved_microusd == 0
        || accepted_unique_cpu_accepts_over_exact_cache == 0
    {
        "future_portfolio_no_positive_call_token_cost_savings"
    } else {
        "future_portfolio_gate_failed"
    };

    let report = PhaseStreamHotPathDaemonNumericFuturePortfolioAuditReport {
        report_kind: "phase_stream_hot_path_daemon_numeric_future_portfolio_audit_v1",
        mode: "orchestrate_multiple_fresh_future_nwpc_audits_into_shadow_only_portfolio",
        generated_future_audit_report_paths: future_audit_report_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        generated_policy_smoke_report_paths: policy_smoke_report_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        portfolio_gate_report_path: portfolio_gate_report_path.display().to_string(),
        runtime_replay_report_path: runtime_replay_report_path.display().to_string(),
        cells,
        min_bucket_events,
        watermark_trace_path: watermark_trace_path.display().to_string(),
        append_trace_paths: append_trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        generated_future_audit_count: future_audit_report_paths.len(),
        generated_policy_smoke_count: policy_smoke_report_paths.len(),
        portfolio_accepted_report_count,
        portfolio_rejected_report_count,
        portfolio_shadow_evidence_ready,
        runtime_replayed_report_count,
        runtime_replay_failed_report_count,
        accepted_unique_cpu_accepts_over_exact_cache,
        accepted_tokens_saved,
        accepted_cost_saved_microusd,
        accepted_false_accepts,
        runtime_total_unique_cpu_accepts_over_exact_cache,
        runtime_total_tokens_saved,
        runtime_total_cost_saved_microusd,
        runtime_total_false_accepts,
        runtime_total_margin_parity_mismatches,
        runtime_total_decision_parity_mismatches,
        runtime_total_mismatched_reports,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        registry_mutation_enabled: false,
        cpu_profile_registry_write_enabled: false,
        serving_profile_artifact_written: false,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        verdict: if gate_passed {
            "HOT_PATH_DAEMON_NUMERIC_FUTURE_PORTFOLIO_AUDIT_PASS"
        } else {
            "HOT_PATH_DAEMON_NUMERIC_FUTURE_PORTFOLIO_AUDIT_WATCH"
        },
        blocker,
        boundary: "portfolio audit only: creates fresh-future .nwpc child audits, policy smokes, portfolio gate, and runtime replay; it does not mutate registry, write serving profiles, promote product runtime, enable local_accept, allow market money claims, use lookup/target/proof authority, or reintroduce legacy nwrb/role-binding paths",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_hot_path_daemon_numeric_future_portfolio_audit_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  portfolio_gate_report_path: {}",
        report.portfolio_gate_report_path
    );
    println!(
        "  runtime_replay_report_path: {}",
        report.runtime_replay_report_path
    );
    println!(
        "  portfolio_accepted_report_count: {}",
        report.portfolio_accepted_report_count
    );
    println!(
        "  accepted_unique_cpu_accepts_over_exact_cache: {}",
        report.accepted_unique_cpu_accepts_over_exact_cache
    );
    println!("  accepted_tokens_saved: {}", report.accepted_tokens_saved);
    println!(
        "  accepted_cost_saved_microusd: {}",
        report.accepted_cost_saved_microusd
    );
    println!(
        "  accepted_false_accepts: {}",
        report.accepted_false_accepts
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_hot_path_daemon_numeric_admission_portfolio_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_HOT_PATH_DAEMON_NUMERIC_ADMISSION_PORTFOLIO_GATE_REPORT)
    });
    let future_audit_report_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(
                DEFAULT_HOT_PATH_DAEMON_NUMERIC_FUTURE_PACKAGE_AUDIT_REPORT,
            )]
        } else {
            rest
        }
    };

    let mut admitted_reports = Vec::new();
    let mut rejected_reports = Vec::new();
    let mut accepted_unique_cpu_accepts_over_exact_cache = 0usize;
    let mut accepted_tokens_saved = 0u64;
    let mut accepted_cost_saved_microusd = 0u64;
    let mut accepted_false_accepts = 0usize;
    let mut accepted_hot_bytes_estimate = 0usize;
    let mut max_accepted_p99_latency_ns = 0u128;

    for future_path in &future_audit_report_paths {
        let policy_smoke_path = live_store_numeric_future_policy_smoke_path(future_path);
        let future = match super::read_json_value(future_path) {
            Ok(value) => value,
            Err(error) => {
                rejected_reports.push(
                    PhaseStreamHotPathDaemonNumericAdmissionPortfolioRejectedReport {
                        future_audit_report_path: future_path.display().to_string(),
                        policy_smoke_report_path: policy_smoke_path.display().to_string(),
                        reason: format!("future_audit_read_failed:{error}"),
                        future_verdict: String::new(),
                        future_blocker: String::new(),
                        policy_verdict: String::new(),
                        policy_blocker: String::new(),
                        unique_cpu_accepts_over_exact_cache: 0,
                        tokens_saved: 0,
                        cost_saved_microusd: 0,
                        false_accepts: usize::MAX,
                        token_cost_denominator_present: false,
                    },
                );
                continue;
            }
        };
        let policy = match super::read_json_value(&policy_smoke_path) {
            Ok(value) => Some(value),
            Err(_) => None,
        };

        let future_report_kind = super::json_string(&future, &["report_kind"]).unwrap_or_default();
        let future_verdict = super::json_string(&future, &["verdict"]).unwrap_or_default();
        let future_blocker = super::json_string(&future, &["blocker"]).unwrap_or_default();
        let policy_verdict = policy
            .as_ref()
            .and_then(|value| super::json_string(value, &["verdict"]))
            .unwrap_or_default();
        let policy_blocker = policy
            .as_ref()
            .and_then(|value| super::json_string(value, &["blocker"]))
            .unwrap_or_default();
        let unique_cpu_accepts_over_exact_cache =
            super::json_u64(&future, &["unique_cpu_accepts_over_exact_cache"]).unwrap_or_default()
                as usize;
        let tokens_saved = super::json_u64(&future, &["tokens_saved"]).unwrap_or_default();
        let cost_saved_microusd =
            super::json_u64(&future, &["cost_saved_microusd"]).unwrap_or_default();
        let false_accepts =
            super::json_u64(&future, &["false_accepts"]).unwrap_or(usize::MAX as u64) as usize;
        let runtime_margin_parity_mismatches =
            super::json_u64(&future, &["runtime_margin_parity_mismatches"])
                .unwrap_or(usize::MAX as u64) as usize;
        let runtime_decision_parity_mismatches =
            super::json_u64(&future, &["runtime_decision_parity_mismatches"])
                .unwrap_or(usize::MAX as u64) as usize;
        let exact_cache_overlap_excluded =
            super::json_bool(&future, &["exact_cache_overlap_excluded"]).unwrap_or(false);
        let token_cost_denominator_present =
            super::json_bool(&future, &["token_cost_denominator_present"]).unwrap_or(false);
        let fresh_future_savings_evidence_allowed =
            super::json_bool(&future, &["fresh_future_savings_evidence_allowed"]).unwrap_or(false);
        let product_promotion_enabled =
            super::json_bool(&future, &["product_promotion_enabled"]).unwrap_or(true);
        let local_accept_enabled =
            super::json_bool(&future, &["local_accept_enabled"]).unwrap_or(true);
        let market_money_claim_allowed =
            super::json_bool(&future, &["market_money_claim_allowed"]).unwrap_or(true);
        let p99_latency_ns =
            super::json_u64(&future, &["p99_latency_ns"]).unwrap_or_default() as u128;
        let hot_bytes_estimate =
            super::json_u64(&future, &["hot_bytes_estimate"]).unwrap_or_default() as usize;
        let candidate_package_path =
            super::json_string(&future, &["candidate_package_path"]).unwrap_or_default();
        let candidate_route_id =
            super::json_u64(&future, &["candidate_route_id"]).unwrap_or_default() as u32;
        let candidate_bucket_id =
            super::json_u64(&future, &["candidate_bucket_id"]).unwrap_or_default() as u32;
        let future_policy_path =
            super::json_string(&future, &["daemon_admission_policy_report_path"])
                .unwrap_or_default();
        let forbidden_flags = live_store_forbidden_flags_from_json(&future);
        let forbidden_flags_clear = !forbidden_flags.target_id_used
            && !forbidden_flags.proof_rule_id_authority_used
            && !forbidden_flags.concrete_x_lookup_used
            && !forbidden_flags.manual_local_out_t_used
            && !forbidden_flags.hidden_frame_id_or_bind_x_used
            && !forbidden_flags.legacy_backend_used;

        let policy_smoke_ok = policy.as_ref().is_some_and(|policy| {
            let policy_source_path =
                super::json_string(policy, &["source_daemon_admission_policy_report_path"])
                    .unwrap_or_default();
            let policy_flags = live_store_forbidden_flags_from_json(policy);
            let policy_flags_clear = !policy_flags.target_id_used
                && !policy_flags.proof_rule_id_authority_used
                && !policy_flags.concrete_x_lookup_used
                && !policy_flags.manual_local_out_t_used
                && !policy_flags.hidden_frame_id_or_bind_x_used
                && !policy_flags.legacy_backend_used;
            super::json_string(policy, &["report_kind"]).unwrap_or_default()
                == "phase_stream_hot_path_daemon_admission_policy_smoke_v1"
                && super::json_string(policy, &["verdict"]).unwrap_or_default()
                    == "HOT_PATH_DAEMON_ADMISSION_POLICY_SMOKE_PASS"
                && super::json_string(policy, &["blocker"]).unwrap_or_default() == "none"
                && super::json_bool(policy, &["would_stage_for_daemon_shadow_only"])
                    .unwrap_or(false)
                && super::json_bool(policy, &["token_cost_denominator_present"]).unwrap_or(false)
                && super::json_u64(policy, &["unique_cpu_accepts_over_exact_cache"])
                    .unwrap_or_default()
                    == unique_cpu_accepts_over_exact_cache as u64
                && super::json_u64(policy, &["tokens_saved"]).unwrap_or_default() == tokens_saved
                && super::json_u64(policy, &["cost_saved_microusd"]).unwrap_or_default()
                    == cost_saved_microusd
                && super::json_u64(policy, &["false_accepts"]).unwrap_or(usize::MAX as u64) == 0
                && !super::json_bool(policy, &["registry_mutation_enabled"]).unwrap_or(true)
                && !super::json_bool(policy, &["cpu_profile_registry_write_enabled"])
                    .unwrap_or(true)
                && !super::json_bool(policy, &["serving_profile_artifact_written"]).unwrap_or(true)
                && !super::json_bool(policy, &["product_promotion_enabled"]).unwrap_or(true)
                && !super::json_bool(policy, &["local_accept_enabled"]).unwrap_or(true)
                && !super::json_bool(policy, &["market_money_claim_allowed"]).unwrap_or(true)
                && policy_flags_clear
                && policy_source_path == future_policy_path
        });

        let rejection_reason = if future_report_kind
            != "phase_stream_hot_path_daemon_numeric_future_package_audit_v1"
        {
            "future_report_kind_mismatch"
        } else if future_verdict != "HOT_PATH_DAEMON_NUMERIC_FUTURE_PACKAGE_AUDIT_PASS"
            || future_blocker != "none"
        {
            "future_audit_not_pass"
        } else if !fresh_future_savings_evidence_allowed {
            "fresh_future_evidence_not_allowed"
        } else if unique_cpu_accepts_over_exact_cache == 0
            || tokens_saved == 0
            || cost_saved_microusd == 0
        {
            "no_positive_call_token_cost_savings"
        } else if false_accepts != 0 {
            "false_accepts_nonzero"
        } else if runtime_margin_parity_mismatches != 0 || runtime_decision_parity_mismatches != 0 {
            "runtime_parity_mismatch"
        } else if !exact_cache_overlap_excluded || !token_cost_denominator_present {
            "denominator_or_exact_cache_overlap_missing"
        } else if product_promotion_enabled || local_accept_enabled || market_money_claim_allowed {
            "future_audit_mutates_product_state"
        } else if !forbidden_flags_clear {
            "forbidden_flag_detected"
        } else if policy.is_none() {
            "policy_smoke_missing"
        } else if !policy_smoke_ok {
            "policy_smoke_not_pass"
        } else {
            "none"
        };

        if rejection_reason == "none" {
            accepted_unique_cpu_accepts_over_exact_cache =
                accepted_unique_cpu_accepts_over_exact_cache
                    .saturating_add(unique_cpu_accepts_over_exact_cache);
            accepted_tokens_saved = accepted_tokens_saved.saturating_add(tokens_saved);
            accepted_cost_saved_microusd =
                accepted_cost_saved_microusd.saturating_add(cost_saved_microusd);
            accepted_false_accepts = accepted_false_accepts.saturating_add(false_accepts);
            accepted_hot_bytes_estimate =
                accepted_hot_bytes_estimate.saturating_add(hot_bytes_estimate);
            max_accepted_p99_latency_ns = max_accepted_p99_latency_ns.max(p99_latency_ns);
            admitted_reports.push(
                PhaseStreamHotPathDaemonNumericAdmissionPortfolioAcceptedReport {
                    future_audit_report_path: future_path.display().to_string(),
                    policy_smoke_report_path: policy_smoke_path.display().to_string(),
                    candidate_package_path,
                    candidate_route_id,
                    candidate_bucket_id,
                    unique_cpu_accepts_over_exact_cache,
                    tokens_saved,
                    cost_saved_microusd,
                    false_accepts,
                    p99_latency_ns,
                    hot_bytes_estimate,
                },
            );
        } else {
            rejected_reports.push(
                PhaseStreamHotPathDaemonNumericAdmissionPortfolioRejectedReport {
                    future_audit_report_path: future_path.display().to_string(),
                    policy_smoke_report_path: policy_smoke_path.display().to_string(),
                    reason: rejection_reason.to_owned(),
                    future_verdict,
                    future_blocker,
                    policy_verdict,
                    policy_blocker,
                    unique_cpu_accepts_over_exact_cache,
                    tokens_saved,
                    cost_saved_microusd,
                    false_accepts,
                    token_cost_denominator_present,
                },
            );
        }
    }

    let shadow_evidence_ready = !admitted_reports.is_empty() && accepted_false_accepts == 0;
    let report = PhaseStreamHotPathDaemonNumericAdmissionPortfolioGateReport {
        report_kind: "phase_stream_hot_path_daemon_numeric_admission_portfolio_gate_v1",
        mode: "shadow_only_fresh_future_portfolio_gate_no_runtime_mutation",
        input_future_audit_report_paths: future_audit_report_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        accepted_report_count: admitted_reports.len(),
        rejected_report_count: rejected_reports.len(),
        accepted_unique_cpu_accepts_over_exact_cache,
        accepted_tokens_saved,
        accepted_cost_saved_microusd,
        accepted_false_accepts,
        accepted_hot_bytes_estimate,
        max_accepted_p99_latency_ns,
        shadow_evidence_ready,
        market_money_claim_allowed: false,
        product_promotion_enabled: false,
        local_accept_enabled: false,
        registry_mutation_enabled: false,
        cpu_profile_registry_write_enabled: false,
        serving_profile_artifact_written: false,
        admitted_reports,
        rejected_reports,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        verdict: if shadow_evidence_ready {
            "HOT_PATH_DAEMON_NUMERIC_ADMISSION_PORTFOLIO_GATE_PASS"
        } else {
            "HOT_PATH_DAEMON_NUMERIC_ADMISSION_PORTFOLIO_GATE_WATCH"
        },
        blocker: if shadow_evidence_ready {
            "none"
        } else {
            "portfolio_no_admissible_money_evidence"
        },
        boundary: "portfolio gate only: aggregates already-produced fresh-future .nwpc audits plus daemon policy smoke reports, rejects WATCH/costless/false-accept/parity-failing evidence, and never compiles packages, mutates registry, writes serving profiles, promotes runtime, enables local_accept, allows market money claims, or uses legacy nwrb/role-binding paths",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_hot_path_daemon_numeric_admission_portfolio_gate_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  accepted_report_count: {}", report.accepted_report_count);
    println!("  rejected_report_count: {}", report.rejected_report_count);
    println!(
        "  accepted_unique_cpu_accepts_over_exact_cache: {}",
        report.accepted_unique_cpu_accepts_over_exact_cache
    );
    println!("  accepted_tokens_saved: {}", report.accepted_tokens_saved);
    println!(
        "  accepted_cost_saved_microusd: {}",
        report.accepted_cost_saved_microusd
    );
    println!(
        "  accepted_false_accepts: {}",
        report.accepted_false_accepts
    );
    println!("  shadow_evidence_ready: {}", report.shadow_evidence_ready);
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_hot_path_daemon_numeric_admission_portfolio_runtime_replay_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let portfolio_gate_report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_HOT_PATH_DAEMON_NUMERIC_ADMISSION_PORTFOLIO_GATE_REPORT)
    });
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_HOT_PATH_DAEMON_NUMERIC_ADMISSION_PORTFOLIO_RUNTIME_REPLAY_REPORT)
    });
    if let Some(extra) = args.next() {
        return Err(format!(
            "unexpected extra argument '{extra}' for phase-stream-hot-path-daemon-numeric-admission-portfolio-runtime-replay-v1"
        ));
    }

    let portfolio = super::read_json_value(&portfolio_gate_report_path)?;
    let portfolio_gate_passed = super::json_string(&portfolio, &["report_kind"])
        .unwrap_or_default()
        == "phase_stream_hot_path_daemon_numeric_admission_portfolio_gate_v1"
        && super::json_string(&portfolio, &["verdict"]).unwrap_or_default()
            == "HOT_PATH_DAEMON_NUMERIC_ADMISSION_PORTFOLIO_GATE_PASS"
        && super::json_string(&portfolio, &["blocker"]).unwrap_or_default() == "none";
    let portfolio_shadow_evidence_ready =
        super::json_bool(&portfolio, &["shadow_evidence_ready"]).unwrap_or(false);
    let portfolio_local_accept =
        super::json_bool(&portfolio, &["local_accept_enabled"]).unwrap_or(true);
    let portfolio_market_claim =
        super::json_bool(&portfolio, &["market_money_claim_allowed"]).unwrap_or(true);
    let portfolio_product_promotion =
        super::json_bool(&portfolio, &["product_promotion_enabled"]).unwrap_or(true);
    let portfolio_registry_mutation =
        super::json_bool(&portfolio, &["registry_mutation_enabled"]).unwrap_or(true);
    let portfolio_cpu_registry_write =
        super::json_bool(&portfolio, &["cpu_profile_registry_write_enabled"]).unwrap_or(true);
    let portfolio_serving_written =
        super::json_bool(&portfolio, &["serving_profile_artifact_written"]).unwrap_or(true);
    let portfolio_flags = live_store_forbidden_flags_from_json(&portfolio);
    let portfolio_forbidden_flags_clear = !portfolio_flags.target_id_used
        && !portfolio_flags.proof_rule_id_authority_used
        && !portfolio_flags.concrete_x_lookup_used
        && !portfolio_flags.manual_local_out_t_used
        && !portfolio_flags.hidden_frame_id_or_bind_x_used
        && !portfolio_flags.legacy_backend_used;

    let admitted = super::json_at(&portfolio, &["admitted_reports"])
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let expected_unique_cpu_accepts_over_exact_cache = super::json_u64(
        &portfolio,
        &["accepted_unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or_default() as usize;
    let expected_tokens_saved =
        super::json_u64(&portfolio, &["accepted_tokens_saved"]).unwrap_or_default();
    let expected_cost_saved_microusd =
        super::json_u64(&portfolio, &["accepted_cost_saved_microusd"]).unwrap_or_default();
    let expected_false_accepts = super::json_u64(&portfolio, &["accepted_false_accepts"])
        .unwrap_or(usize::MAX as u64) as usize;

    let mut replay_reports = Vec::new();
    let mut total_score_events = 0usize;
    let mut total_score_candidate_events = 0usize;
    let mut total_unique_cpu_accepts_over_exact_cache = 0usize;
    let mut total_tokens_saved = 0u64;
    let mut total_cost_saved_microusd = 0u64;
    let mut total_false_accepts = 0usize;
    let mut total_runtime_margin_parity_checks = 0usize;
    let mut total_runtime_margin_parity_mismatches = 0usize;
    let mut total_runtime_decision_parity_mismatches = 0usize;
    let mut replay_failed_report_count = 0usize;
    let mut total_mismatched_reports = 0usize;

    for item in admitted {
        let future_audit_report_path = super::json_string(&item, &["future_audit_report_path"])
            .map(PathBuf::from)
            .unwrap_or_default();
        let candidate_package_path = super::json_string(&item, &["candidate_package_path"])
            .map(PathBuf::from)
            .unwrap_or_default();
        let fallback_route_id =
            super::json_u64(&item, &["candidate_route_id"]).unwrap_or_default() as u32;
        let fallback_bucket_id =
            super::json_u64(&item, &["candidate_bucket_id"]).unwrap_or_default() as u32;
        let expected_item_unique = super::json_u64(&item, &["unique_cpu_accepts_over_exact_cache"])
            .unwrap_or_default() as usize;
        let expected_item_tokens = super::json_u64(&item, &["tokens_saved"]).unwrap_or_default();
        let expected_item_cost =
            super::json_u64(&item, &["cost_saved_microusd"]).unwrap_or_default();
        let expected_item_false =
            super::json_u64(&item, &["false_accepts"]).unwrap_or(usize::MAX as u64) as usize;

        let item_report = match live_store_replay_one_portfolio_admission(
            &future_audit_report_path,
            &candidate_package_path,
            fallback_route_id,
            fallback_bucket_id,
            expected_item_unique,
            expected_item_tokens,
            expected_item_cost,
            expected_item_false,
        ) {
            Ok(report) => report,
            Err(error) => {
                replay_failed_report_count += 1;
                PhaseStreamHotPathDaemonNumericAdmissionPortfolioRuntimeReplayItemReport {
                    future_audit_report_path: future_audit_report_path.display().to_string(),
                    candidate_package_path: candidate_package_path.display().to_string(),
                    candidate_route_id: fallback_route_id,
                    candidate_bucket_id: fallback_bucket_id,
                    threshold_micro: 0,
                    freeze_after_append_events: 0,
                    future_matching_bucket_events: 0,
                    score_events: 0,
                    score_candidate_events: 0,
                    unique_cpu_accepts_over_exact_cache: 0,
                    tokens_saved: 0,
                    cost_saved_microusd: 0,
                    false_accepts: usize::MAX,
                    runtime_margin_parity_checks: 0,
                    runtime_margin_parity_mismatches: usize::MAX,
                    runtime_decision_parity_mismatches: usize::MAX,
                    expected_score_events: 0,
                    expected_score_candidate_events: 0,
                    expected_unique_cpu_accepts_over_exact_cache: expected_item_unique,
                    expected_tokens_saved: expected_item_tokens,
                    expected_cost_saved_microusd: expected_item_cost,
                    expected_false_accepts: expected_item_false,
                    matches_future_audit: false,
                    replay_loaded: false,
                    blocker: format!("runtime_replay_failed:{error}"),
                }
            }
        };
        if !item_report.matches_future_audit {
            total_mismatched_reports += 1;
        }
        total_score_events = total_score_events.saturating_add(item_report.score_events);
        total_score_candidate_events =
            total_score_candidate_events.saturating_add(item_report.score_candidate_events);
        total_unique_cpu_accepts_over_exact_cache = total_unique_cpu_accepts_over_exact_cache
            .saturating_add(item_report.unique_cpu_accepts_over_exact_cache);
        total_tokens_saved = total_tokens_saved.saturating_add(item_report.tokens_saved);
        total_cost_saved_microusd =
            total_cost_saved_microusd.saturating_add(item_report.cost_saved_microusd);
        total_false_accepts = total_false_accepts.saturating_add(item_report.false_accepts);
        total_runtime_margin_parity_checks = total_runtime_margin_parity_checks
            .saturating_add(item_report.runtime_margin_parity_checks);
        total_runtime_margin_parity_mismatches = total_runtime_margin_parity_mismatches
            .saturating_add(item_report.runtime_margin_parity_mismatches);
        total_runtime_decision_parity_mismatches = total_runtime_decision_parity_mismatches
            .saturating_add(item_report.runtime_decision_parity_mismatches);
        replay_reports.push(item_report);
    }

    let totals_match = total_unique_cpu_accepts_over_exact_cache
        == expected_unique_cpu_accepts_over_exact_cache
        && total_tokens_saved == expected_tokens_saved
        && total_cost_saved_microusd == expected_cost_saved_microusd
        && total_false_accepts == expected_false_accepts;
    let gate_passed = portfolio_gate_passed
        && portfolio_shadow_evidence_ready
        && portfolio_forbidden_flags_clear
        && !portfolio_local_accept
        && !portfolio_market_claim
        && !portfolio_product_promotion
        && !portfolio_registry_mutation
        && !portfolio_cpu_registry_write
        && !portfolio_serving_written
        && !replay_reports.is_empty()
        && replay_failed_report_count == 0
        && total_mismatched_reports == 0
        && totals_match
        && total_unique_cpu_accepts_over_exact_cache > 0
        && total_tokens_saved > 0
        && total_cost_saved_microusd > 0
        && total_false_accepts == 0
        && total_runtime_margin_parity_mismatches == 0
        && total_runtime_decision_parity_mismatches == 0;
    let blocker = if gate_passed {
        "none"
    } else if !portfolio_gate_passed || !portfolio_shadow_evidence_ready {
        "portfolio_gate_not_shadow_ready"
    } else if !portfolio_forbidden_flags_clear {
        "portfolio_forbidden_flag_detected"
    } else if portfolio_local_accept
        || portfolio_market_claim
        || portfolio_product_promotion
        || portfolio_registry_mutation
        || portfolio_cpu_registry_write
        || portfolio_serving_written
    {
        "portfolio_mutates_product_state"
    } else if replay_reports.is_empty() {
        "portfolio_no_admitted_reports"
    } else if replay_failed_report_count != 0 {
        "portfolio_runtime_replay_failed"
    } else if total_mismatched_reports != 0 || !totals_match {
        "portfolio_runtime_replay_mismatch"
    } else if total_false_accepts != 0 {
        "portfolio_runtime_replay_false_accepts_nonzero"
    } else if total_runtime_margin_parity_mismatches != 0
        || total_runtime_decision_parity_mismatches != 0
    {
        "portfolio_runtime_replay_parity_mismatch"
    } else if total_unique_cpu_accepts_over_exact_cache == 0
        || total_tokens_saved == 0
        || total_cost_saved_microusd == 0
    {
        "portfolio_runtime_replay_no_positive_savings"
    } else {
        "portfolio_runtime_replay_gate_failed"
    };

    let report = PhaseStreamHotPathDaemonNumericAdmissionPortfolioRuntimeReplayReport {
        report_kind: "phase_stream_hot_path_daemon_numeric_admission_portfolio_runtime_replay_v1",
        mode: "accepted_portfolio_nwpc_runtime_replay_no_mutation",
        source_portfolio_gate_report_path: portfolio_gate_report_path.display().to_string(),
        portfolio_gate_passed,
        portfolio_shadow_evidence_ready,
        portfolio_admitted_report_count: replay_reports.len(),
        replayed_report_count: replay_reports
            .iter()
            .filter(|report| report.replay_loaded)
            .count(),
        replay_failed_report_count,
        total_score_events,
        total_score_candidate_events,
        total_unique_cpu_accepts_over_exact_cache,
        total_tokens_saved,
        total_cost_saved_microusd,
        total_false_accepts,
        total_runtime_margin_parity_checks,
        total_runtime_margin_parity_mismatches,
        total_runtime_decision_parity_mismatches,
        expected_unique_cpu_accepts_over_exact_cache,
        expected_tokens_saved,
        expected_cost_saved_microusd,
        expected_false_accepts,
        total_mismatched_reports,
        replay_reports,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        product_promotion_enabled: false,
        registry_mutation_enabled: false,
        cpu_profile_registry_write_enabled: false,
        serving_profile_artifact_written: false,
        forbidden_flags: super::ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        verdict: if gate_passed {
            "HOT_PATH_DAEMON_NUMERIC_ADMISSION_PORTFOLIO_RUNTIME_REPLAY_PASS"
        } else {
            "HOT_PATH_DAEMON_NUMERIC_ADMISSION_PORTFOLIO_RUNTIME_REPLAY_WATCH"
        },
        blocker,
        boundary: "runtime replay only: loads accepted portfolio .nwpc packages into PhaseCenterHotRuntime and re-scores their recorded fresh-future windows to verify the portfolio evidence is runtime-replayable; it does not compile packages, mutate registry, write serving profiles, promote runtime, enable local_accept, allow market money claims, or use legacy nwrb/role-binding paths",
    };
    super::write_json_file(&report_path, &report)?;
    println!("phase_stream_hot_path_daemon_numeric_admission_portfolio_runtime_replay_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  replayed_report_count: {}", report.replayed_report_count);
    println!(
        "  total_unique_cpu_accepts_over_exact_cache: {}",
        report.total_unique_cpu_accepts_over_exact_cache
    );
    println!("  total_tokens_saved: {}", report.total_tokens_saved);
    println!(
        "  total_cost_saved_microusd: {}",
        report.total_cost_saved_microusd
    );
    println!("  total_false_accepts: {}", report.total_false_accepts);
    println!(
        "  total_mismatched_reports: {}",
        report.total_mismatched_reports
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}
