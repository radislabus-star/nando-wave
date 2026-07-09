use std::collections::BTreeMap;

use nando_core::{
    PhaseCenterAtomEncoder, PhaseCenterFlatRecord, PhaseCenterFlatRuntime, PhaseCenterHotRuntime,
    PhaseCenterHotScratch, PhaseCenterOperatorAdmission, PhaseCenterOperatorMemory,
    PhaseCenterOperatorMemoryConfig, PhaseCenterPreparedHotRequest, PhaseCenterPromotionEvidence,
    PhaseCenterThresholdPolicyEvidence,
};

use super::claim_gates::{live_store_admission_blocker_name, live_store_promotion_blocker_name};
use super::frozen_candidates::{LiveStoreCandidateFutureEvent, LiveStoreFrozenCandidate};
use super::promotion_manifests::{
    live_store_candidate_runtime_parity_mismatches,
    live_store_frozen_candidate_runtime_quarantined,
    refresh_live_store_call_token_promotion_manifest_summary,
};
use super::reports::{
    PhaseStreamLiveStoreFutureShadowCandidateReport, PhaseStreamLiveStoreFutureShadowReport,
    PhaseStreamLiveStoreRegistryRouteReport,
};
use super::runtime_metrics::live_store_milli;
use super::source_events::LiveStoreParsedAtomEvent;
use super::state::{LiveStoreCandidateRegistryShadowReport, LiveStoreSharedRegistryShadowReport};

pub(super) fn live_store_future_shadow_candidate_reports(
    frozen_candidates: &BTreeMap<u32, LiveStoreFrozenCandidate>,
    token_cost_denominator_present: bool,
) -> Vec<PhaseStreamLiveStoreFutureShadowCandidateReport> {
    let mut reports = frozen_candidates
        .values()
        .map(|candidate| {
            let promotion_contract =
                live_store_candidate_promotion_contract(candidate, token_cost_denominator_present);
            let registry_shadow =
                live_store_candidate_registry_shadow(candidate, token_cost_denominator_present)
                    .unwrap_or_else(|_| LiveStoreCandidateRegistryShadowReport {
                        admission_attempted: true,
                        admission_blocker: "registry_shadow_error",
                        ..LiveStoreCandidateRegistryShadowReport::default()
                    });
            let future_promotable = candidate.future_false_accepts == 0
                && candidate.future_unique_cpu_accepts_over_exact_cache > 0
                && candidate.future_runtime_margin_parity_mismatches == 0
                && candidate.future_runtime_decision_parity_mismatches == 0
                && candidate.future_tokens_saved > 0
                && candidate.future_cost_saved_microusd > 0;
            let blocker = if candidate.future_false_accepts > 0 {
                "future_shadow_false_accepts_present"
            } else if candidate.future_runtime_margin_parity_mismatches > 0
                || candidate.future_runtime_decision_parity_mismatches > 0
            {
                "runtime_parity_mismatch"
            } else if candidate.future_unique_cpu_accepts_over_exact_cache == 0 {
                "future_shadow_no_unique_accepts"
            } else if candidate.future_tokens_saved == 0
                || candidate.future_cost_saved_microusd == 0
            {
                "token_cost_denominator_missing"
            } else {
                "external_promotion_contract_still_required"
            };
            PhaseStreamLiveStoreFutureShadowCandidateReport {
                bucket_id: candidate.bucket_id,
                route_id: candidate.route_id,
                package_fingerprint64: candidate.package.package_info.fingerprint64,
                scored_events: candidate.future_scored_events,
                score_candidate_events: candidate.future_score_candidate_events,
                runtime_margin_parity_checks: candidate.future_runtime_margin_parity_checks,
                runtime_margin_parity_mismatches: candidate.future_runtime_margin_parity_mismatches,
                runtime_decision_parity_mismatches: candidate
                    .future_runtime_decision_parity_mismatches,
                unique_cpu_accepts_over_exact_cache: candidate
                    .future_unique_cpu_accepts_over_exact_cache,
                false_accepts: candidate.future_false_accepts,
                tokens_saved: candidate.future_tokens_saved,
                cost_saved_microusd: candidate.future_cost_saved_microusd,
                future_promotable,
                blocker,
                promotion_contract_eligible: promotion_contract.0,
                promotion_contract_blocker: promotion_contract.1,
                registry_admission_attempted: registry_shadow.admission_attempted,
                registry_admitted: registry_shadow.admitted,
                registry_admission_blocker: registry_shadow.admission_blocker,
                registry_hot_route_count: registry_shadow.hot_route_count,
                registry_hot_profile_count: registry_shadow.hot_profile_count,
                registry_hot_bytes_estimate: registry_shadow.hot_bytes_estimate,
                registry_budget_passed: registry_shadow.budget_passed,
                registry_shadow_score_events: registry_shadow.score_events,
                registry_shadow_score_candidate_events: registry_shadow.score_candidate_events,
                registry_shadow_unique_cpu_accepts_over_exact_cache: registry_shadow
                    .unique_cpu_accepts_over_exact_cache,
                registry_shadow_tokens_saved: registry_shadow.tokens_saved,
                registry_shadow_cost_saved_microusd: registry_shadow.cost_saved_microusd,
                registry_shadow_false_accepts: registry_shadow.false_accepts,
                registry_shadow_margin_parity_mismatches: registry_shadow.margin_parity_mismatches,
                registry_shadow_decision_parity_mismatches: registry_shadow
                    .decision_parity_mismatches,
            }
        })
        .collect::<Vec<_>>();
    reports.sort_by(|a, b| {
        b.false_accepts
            .cmp(&a.false_accepts)
            .then_with(|| {
                b.unique_cpu_accepts_over_exact_cache
                    .cmp(&a.unique_cpu_accepts_over_exact_cache)
            })
            .then_with(|| a.bucket_id.cmp(&b.bucket_id))
    });
    reports
}

fn live_store_candidate_promotion_contract(
    candidate: &LiveStoreFrozenCandidate,
    token_cost_denominator_present: bool,
) -> (bool, &'static str) {
    let evidence =
        live_store_candidate_promotion_evidence(candidate, token_cost_denominator_present);
    let decision = evidence.evaluate();
    (
        decision.eligible,
        decision
            .blocker
            .map(live_store_promotion_blocker_name)
            .unwrap_or("none"),
    )
}

fn live_store_candidate_registry_shadow(
    candidate: &LiveStoreFrozenCandidate,
    token_cost_denominator_present: bool,
) -> Result<LiveStoreCandidateRegistryShadowReport, String> {
    let evidence =
        live_store_candidate_promotion_evidence(candidate, token_cost_denominator_present);
    let mut memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
        max_hot_profiles_per_worker: 4,
        max_hot_bytes_per_worker: 64 * 1024,
        max_warm_profiles_per_process: 16,
        max_profiles_per_route: 4,
        max_route_top_k: 1,
        min_tokens_saved: 1,
        min_accept_rate_milli: 1,
        false_accepts_must_be_zero: true,
    })
    .map_err(|error| format!("failed to create registry shadow memory: {error:?}"))?;
    let admission = memory.admit(PhaseCenterOperatorAdmission {
        route_id: candidate.route_id,
        profile_id: candidate.bucket_id,
        evidence,
        runtime_bytes_estimate: candidate.hot_runtime.bytes_estimate(),
        last_seen_tick: candidate.future_scored_events as u64,
    });
    if !admission.admitted {
        return Ok(LiveStoreCandidateRegistryShadowReport {
            admission_attempted: true,
            admitted: false,
            admission_blocker: admission
                .blocker
                .map(live_store_admission_blocker_name)
                .unwrap_or("unknown_admission_blocker"),
            ..LiveStoreCandidateRegistryShadowReport::default()
        });
    }

    let hot_routes = memory
        .hot_route_table(&candidate.hot_runtime)
        .map_err(|error| format!("failed to build registry hot route table: {error:?}"))?;
    let route_index = hot_routes
        .resolve_route_index(candidate.route_id)
        .ok_or_else(|| "registry hot route index missing".to_owned())?;
    let budget = memory.runtime_budget_snapshot(&candidate.hot_runtime, &hot_routes);
    let mut scratch = PhaseCenterHotScratch::new(candidate.flat_runtime.cells(), 1)
        .map_err(|error| format!("failed to create registry hot scratch: {error:?}"))?;
    let mut report = LiveStoreCandidateRegistryShadowReport {
        admission_attempted: true,
        admitted: true,
        admission_blocker: "none",
        hot_route_count: hot_routes.route_count(),
        hot_profile_count: candidate.hot_runtime.profile_count(),
        hot_bytes_estimate: budget.hot_bytes_estimate,
        budget_passed: budget.product_runtime_budget_passed(),
        ..LiveStoreCandidateRegistryShadowReport::default()
    };

    for event in &candidate.future_events {
        let reference_margin_micro = candidate
            .flat_runtime
            .score_vector_margin_micro(0, &event.phase_vector)
            .map_err(|error| format!("failed registry reference score: {error:?}"))?;
        let reference_score_candidate = reference_margin_micro >= candidate.package.threshold_micro;
        let decisions = candidate
            .hot_runtime
            .score_prepared_hot_request_candidates(
                &hot_routes,
                PhaseCenterPreparedHotRequest::new(route_index, &event.phase_vector),
                &mut scratch,
            )
            .map_err(|error| format!("failed registry hot score: {error:?}"))?;
        report.score_events += 1;
        for decision in decisions {
            if decision.profile_id != candidate.bucket_id {
                continue;
            }
            if decision.margin_micro != reference_margin_micro {
                report.margin_parity_mismatches += 1;
            }
            if decision.score_candidate != reference_score_candidate {
                report.decision_parity_mismatches += 1;
            }
            if !decision.score_candidate {
                continue;
            }
            report.score_candidate_events += 1;
            if event.verified_safe_accept {
                if !event.exact_cache_hit {
                    report.unique_cpu_accepts_over_exact_cache += 1;
                    report.tokens_saved = report.tokens_saved.saturating_add(event.tokens);
                    report.cost_saved_microusd = report
                        .cost_saved_microusd
                        .saturating_add(event.cost_microusd);
                }
            } else {
                report.false_accepts += 1;
            }
        }
    }
    Ok(report)
}

pub(super) fn live_store_shared_registry_shadow(
    frozen_candidates: &BTreeMap<u32, LiveStoreFrozenCandidate>,
    token_cost_denominator_present: bool,
) -> Result<LiveStoreSharedRegistryShadowReport, String> {
    let mut memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
        max_hot_profiles_per_worker: 16,
        max_hot_bytes_per_worker: 64 * 1024,
        max_warm_profiles_per_process: 16,
        max_profiles_per_route: 16,
        max_route_top_k: 16,
        min_tokens_saved: 1,
        min_accept_rate_milli: 1,
        false_accepts_must_be_zero: true,
    })
    .map_err(|error| format!("failed to create shared registry memory: {error:?}"))?;
    let mut report = LiveStoreSharedRegistryShadowReport::default();
    report.exact_cache_overlap_excluded = true;
    let mut records = Vec::<PhaseCenterFlatRecord>::new();
    let mut profile_ids = Vec::<u32>::new();
    let mut thresholds = Vec::<i64>::new();
    let mut admitted_bucket_ids = Vec::<u32>::new();
    let mut cells = None::<usize>;

    for candidate in frozen_candidates.values() {
        report.admission_attempts += 1;
        let evidence =
            live_store_candidate_promotion_evidence(candidate, token_cost_denominator_present);
        let admission = memory.admit(PhaseCenterOperatorAdmission {
            route_id: candidate.route_id,
            profile_id: candidate.bucket_id,
            evidence,
            runtime_bytes_estimate: candidate.hot_runtime.bytes_estimate(),
            last_seen_tick: candidate.future_scored_events as u64,
        });
        if !admission.admitted {
            report.rejected_candidates += 1;
            continue;
        }
        if let Some(expected_cells) = cells {
            if expected_cells != candidate.flat_runtime.cells() {
                return Err("shared registry candidate cell width mismatch".to_owned());
            }
        } else {
            cells = Some(candidate.flat_runtime.cells());
        }
        records.push(
            candidate
                .flat_runtime
                .record(0)
                .map_err(|error| format!("failed to read candidate flat record: {error:?}"))?
                .clone(),
        );
        profile_ids.push(candidate.bucket_id);
        thresholds.push(candidate.package.threshold_micro);
        admitted_bucket_ids.push(candidate.bucket_id);
        report.admitted_candidates += 1;
    }

    if records.is_empty() {
        return Ok(report);
    }

    let flat = PhaseCenterFlatRuntime::new(
        cells.ok_or_else(|| "shared registry cells missing".to_owned())?,
        records,
    )
    .map_err(|error| format!("failed to build shared registry flat runtime: {error:?}"))?;
    let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &profile_ids, &thresholds)
        .map_err(|error| format!("failed to build shared registry hot runtime: {error:?}"))?;
    let hot_routes = memory
        .hot_route_table(&hot)
        .map_err(|error| format!("failed to build shared registry route table: {error:?}"))?;
    let budget = memory.runtime_budget_snapshot(&hot, &hot_routes);
    report.hot_route_count = hot_routes.route_count();
    report.hot_profile_count = hot.profile_count();
    report.hot_route_profile_edges = hot_routes.profile_edge_count();
    report.hot_bytes_estimate = budget.hot_bytes_estimate;
    report.budget_passed = budget.product_runtime_budget_passed();

    let mut scratch = PhaseCenterHotScratch::new(flat.cells(), 1)
        .map_err(|error| format!("failed to create shared registry scratch: {error:?}"))?;
    for bucket_id in admitted_bucket_ids {
        let Some(candidate) = frozen_candidates.get(&bucket_id) else {
            return Err("shared registry admitted bucket missing".to_owned());
        };
        let route_index = hot_routes
            .resolve_route_index(candidate.route_id)
            .ok_or_else(|| "shared registry route index missing".to_owned())?;
        report
            .route_manifest
            .push(PhaseStreamLiveStoreRegistryRouteReport {
                route_id: candidate.route_id,
                route_index,
                profile_id: candidate.bucket_id,
                package_fingerprint64: candidate.package.package_info.fingerprint64,
            });
        for event in &candidate.future_events {
            let reference_margin_micro = candidate
                .flat_runtime
                .score_vector_margin_micro(0, &event.phase_vector)
                .map_err(|error| format!("failed shared registry reference score: {error:?}"))?;
            let reference_score_candidate =
                reference_margin_micro >= candidate.package.threshold_micro;
            let decisions = hot
                .score_prepared_hot_request_candidates(
                    &hot_routes,
                    PhaseCenterPreparedHotRequest::new(route_index, &event.phase_vector),
                    &mut scratch,
                )
                .map_err(|error| format!("failed shared registry hot score: {error:?}"))?;
            report.score_events += 1;
            let mut matched_profile = false;
            for decision in decisions {
                if decision.profile_id != candidate.bucket_id {
                    continue;
                }
                matched_profile = true;
                if decision.margin_micro != reference_margin_micro {
                    report.margin_parity_mismatches += 1;
                }
                if decision.score_candidate != reference_score_candidate {
                    report.decision_parity_mismatches += 1;
                }
                if decision.verifier_required {
                    report.verifier_required_events += 1;
                }
                if decision.local_accept {
                    report.local_accept_events += 1;
                }
                if !decision.score_candidate {
                    continue;
                }
                report.score_candidate_events += 1;
                if event.verified_safe_accept {
                    if !event.exact_cache_hit {
                        report.unique_cpu_accepts_over_exact_cache += 1;
                        report.tokens_saved = report.tokens_saved.saturating_add(event.tokens);
                        report.cost_saved_microusd = report
                            .cost_saved_microusd
                            .saturating_add(event.cost_microusd);
                    }
                } else {
                    report.false_accepts += 1;
                }
            }
            if !matched_profile {
                report.decision_parity_mismatches += 1;
            }
        }
    }
    Ok(report)
}

pub(super) fn live_store_serving_policy_blocker(
    shared_registry: &LiveStoreSharedRegistryShadowReport,
) -> &'static str {
    if shared_registry.route_manifest.is_empty() {
        "serving_policy_no_route_manifest"
    } else if shared_registry.score_events == 0 {
        "serving_policy_no_score_events"
    } else if shared_registry.score_candidate_events == 0 {
        "serving_policy_no_score_candidates"
    } else if shared_registry.verifier_required_events != shared_registry.score_candidate_events {
        "serving_policy_missing_verifier_required"
    } else if shared_registry.local_accept_events > 0 {
        "serving_policy_local_accept_enabled"
    } else if shared_registry.false_accepts > 0 {
        "serving_policy_false_accepts"
    } else if shared_registry.margin_parity_mismatches > 0
        || shared_registry.decision_parity_mismatches > 0
    {
        "serving_policy_runtime_parity_mismatch"
    } else if !shared_registry.exact_cache_overlap_excluded {
        "serving_policy_exact_cache_overlap_not_excluded"
    } else if !shared_registry.budget_passed {
        "serving_policy_budget_failed"
    } else {
        "none"
    }
}

pub(super) fn live_store_clean_promotion_manifest_blocker(
    future_shadow: &PhaseStreamLiveStoreFutureShadowReport,
) -> &'static str {
    if future_shadow.clean_promotion_manifest_promoted_candidates == 0 {
        "clean_promotion_manifest_no_promoted_candidates"
    } else if !future_shadow.serving_policy_passed {
        "clean_promotion_manifest_serving_policy_not_passed"
    } else if !future_shadow.shared_registry_budget_passed {
        "clean_promotion_manifest_budget_failed"
    } else if future_shadow.clean_promotion_manifest_false_accepts > 0 {
        "clean_promotion_manifest_false_accepts"
    } else if future_shadow.clean_promotion_manifest_runtime_parity_mismatches > 0 {
        "clean_promotion_manifest_runtime_parity_mismatch"
    } else if !future_shadow.clean_promotion_manifest_exact_cache_overlap_excluded {
        "clean_promotion_manifest_exact_cache_overlap_not_excluded"
    } else if future_shadow.clean_promotion_manifest_tokens_saved == 0
        || future_shadow.clean_promotion_manifest_cost_saved_microusd == 0
    {
        "clean_promotion_manifest_missing_token_cost_denominator"
    } else if future_shadow.clean_promotion_manifest_local_accept_enabled {
        "clean_promotion_manifest_local_accept_enabled"
    } else if future_shadow.clean_promotion_manifest_routes.is_empty() {
        "clean_promotion_manifest_empty_route_manifest"
    } else {
        "none"
    }
}

fn live_store_candidate_promotion_evidence(
    candidate: &LiveStoreFrozenCandidate,
    token_cost_denominator_present: bool,
) -> PhaseCenterPromotionEvidence {
    let parity_mismatches = candidate
        .future_runtime_margin_parity_mismatches
        .saturating_add(candidate.future_runtime_decision_parity_mismatches);
    PhaseCenterPromotionEvidence {
        future_shadow_events: candidate.future_scored_events,
        unique_cpu_accepts_over_exact_cache: candidate.future_unique_cpu_accepts_over_exact_cache,
        tokens_saved: candidate.future_tokens_saved,
        cost_saved_microusd: candidate.future_cost_saved_microusd,
        false_accepts: candidate.future_false_accepts,
        runtime_margin_parity_mismatches: parity_mismatches,
        verifier_binding: candidate.package.verifier_binding,
        threshold_policy: PhaseCenterThresholdPolicyEvidence {
            candidate_bucket_count: 1,
            auto_calibrated_bucket_count: 1,
            calibration_window_before_shadow: true,
            shadow_window_after_calibration: candidate.future_scored_events > 0,
            per_bucket_thresholds_reported: candidate.package.threshold_micro > 0,
            fixed_policy_shadow_replay: candidate.future_runtime_margin_parity_checks > 0,
        },
        exact_cache_overlap_excluded: true,
        token_cost_denominator_present,
        local_accept_enabled: false,
    }
}

pub(super) fn live_store_refresh_future_shadow_summary(
    future_shadow: &mut PhaseStreamLiveStoreFutureShadowReport,
    frozen_candidates: &BTreeMap<u32, LiveStoreFrozenCandidate>,
    token_cost_denominator_present: bool,
    parsed_rows: usize,
    total_tokens_seen: u64,
    total_cost_microusd_seen: u64,
) -> Result<(), String> {
    future_shadow.frozen_candidate_count = frozen_candidates.len();
    future_shadow.candidates = live_store_future_shadow_candidate_reports(
        frozen_candidates,
        token_cost_denominator_present,
    );
    future_shadow.runtime_quarantined_candidate_ids = future_shadow
        .candidates
        .iter()
        .filter(|candidate| {
            candidate.false_accepts > 0
                || live_store_candidate_runtime_parity_mismatches(candidate) > 0
        })
        .map(|candidate| candidate.bucket_id)
        .collect();
    future_shadow.runtime_quarantined_candidate_count =
        future_shadow.runtime_quarantined_candidate_ids.len();
    future_shadow.runtime_active_candidate_count = future_shadow
        .frozen_candidate_count
        .saturating_sub(future_shadow.runtime_quarantined_candidate_count);
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

    let shared_registry =
        live_store_shared_registry_shadow(frozen_candidates, token_cost_denominator_present)?;
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
    future_shadow.serving_policy_blocker = live_store_serving_policy_blocker(&shared_registry);
    future_shadow.serving_policy_passed = future_shadow.serving_policy_blocker == "none";
    future_shadow.clean_promotion_manifest_kind =
        "verifier_bound_false_accept_zero_nwpc_handoff_v1";
    future_shadow.clean_promotion_manifest_promoted_candidates =
        future_shadow.shared_registry_admitted_candidates;
    future_shadow.clean_promotion_manifest_quarantined_candidates = future_shadow
        .frozen_candidate_count
        .saturating_sub(future_shadow.clean_promotion_manifest_promoted_candidates);
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
        live_store_clean_promotion_manifest_blocker(future_shadow);
    future_shadow.clean_promotion_manifest_allowed =
        future_shadow.clean_promotion_manifest_blocker == "none";
    refresh_live_store_call_token_promotion_manifest_summary(future_shadow, frozen_candidates);
    future_shadow.promotion_gate_allowed = false;
    future_shadow.blocker = if future_shadow.frozen_candidate_count == 0 {
        "future_shadow_no_frozen_candidates"
    } else if future_shadow.scored_events == 0 {
        "future_shadow_no_future_matching_events"
    } else if future_shadow.false_accepts > 0 {
        "future_shadow_false_accepts_present"
    } else if future_shadow.runtime_margin_parity_mismatches > 0
        || future_shadow.runtime_decision_parity_mismatches > 0
    {
        "future_shadow_runtime_parity_mismatch"
    } else if future_shadow.promotable_candidate_count == 0 {
        "future_shadow_no_promotable_candidate"
    } else if future_shadow.promotable_tokens_saved == 0
        || future_shadow.promotable_cost_saved_microusd == 0
    {
        "future_shadow_missing_token_cost_denominator"
    } else {
        "promotion_gate_manual_review_required"
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
    Ok(())
}

pub(super) fn observe_live_store_future_shadow(
    event: &LiveStoreParsedAtomEvent,
    frozen_candidates: &mut BTreeMap<u32, LiveStoreFrozenCandidate>,
    encoder: &mut PhaseCenterAtomEncoder,
    future_shadow: &mut PhaseStreamLiveStoreFutureShadowReport,
) -> Result<(), String> {
    let vector = encoder
        .encode_atom_ids(event.atom_ids.iter().copied())
        .map_err(|error| format!("failed to encode future-shadow atom ids: {error:?}"))?;

    for candidate in frozen_candidates
        .values_mut()
        .filter(|candidate| candidate.route_id == event.route_id)
    {
        if live_store_frozen_candidate_runtime_quarantined(candidate) {
            continue;
        }
        let reference_margin_micro = candidate
            .flat_runtime
            .score_vector_margin_micro(0, vector)
            .map_err(|error| format!("failed to score future-shadow reference path: {error:?}"))?;
        let reference_score_candidate = reference_margin_micro >= candidate.package.threshold_micro;
        let decisions = candidate
            .hot_runtime
            .score_prepared_hot_request_candidates(
                &candidate.route_table,
                PhaseCenterPreparedHotRequest::new(candidate.route_index, vector),
                &mut candidate.scratch,
            )
            .map_err(|error| format!("failed to score future-shadow candidate: {error:?}"))?;
        future_shadow.scored_events += 1;
        candidate.future_scored_events += 1;
        candidate.future_events.push(LiveStoreCandidateFutureEvent {
            phase_vector: vector.to_vec(),
            verified_safe_accept: event.verified_safe_accept,
            exact_cache_hit: event.exact_cache_hit,
            request_fingerprint: event.request_fingerprint.clone(),
            exact_cache_key: event.exact_cache_key.clone(),
            trace_id: event.trace_id.clone(),
            input_trace_path: event.input_trace_path.clone(),
            event_timestamp: event.event_timestamp.clone(),
            tokens: event.tokens,
            cost_microusd: event.cost_microusd,
        });
        for decision in decisions {
            if decision.profile_id != candidate.package.bucket_id {
                continue;
            }
            future_shadow.runtime_margin_parity_checks += 1;
            candidate.future_runtime_margin_parity_checks += 1;
            if decision.margin_micro != reference_margin_micro {
                future_shadow.runtime_margin_parity_mismatches += 1;
                candidate.future_runtime_margin_parity_mismatches += 1;
            }
            if decision.score_candidate != reference_score_candidate {
                future_shadow.runtime_decision_parity_mismatches += 1;
                candidate.future_runtime_decision_parity_mismatches += 1;
            }
            if !decision.score_candidate {
                continue;
            }
            future_shadow.score_candidate_events += 1;
            candidate.future_score_candidate_events += 1;
            if event.verified_safe_accept {
                if !event.exact_cache_hit {
                    future_shadow.unique_cpu_accepts_over_exact_cache += 1;
                    future_shadow.tokens_saved =
                        future_shadow.tokens_saved.saturating_add(event.tokens);
                    future_shadow.cost_saved_microusd = future_shadow
                        .cost_saved_microusd
                        .saturating_add(event.cost_microusd);
                    candidate.future_unique_cpu_accepts_over_exact_cache += 1;
                    candidate.future_tokens_saved =
                        candidate.future_tokens_saved.saturating_add(event.tokens);
                    candidate.future_cost_saved_microusd = candidate
                        .future_cost_saved_microusd
                        .saturating_add(event.cost_microusd);
                }
            } else {
                future_shadow.false_accepts += 1;
                candidate.future_false_accepts += 1;
            }
        }
    }
    Ok(())
}
