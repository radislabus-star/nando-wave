use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use nando_core::PhaseCenterLiveOperatorStore;

use super::frozen_candidates::LiveStoreFrozenCandidate;
use super::reports::{
    PhaseStreamLiveStoreCleanPromotionManifest, PhaseStreamLiveStoreFutureShadowCandidateReport,
    PhaseStreamLiveStoreFutureShadowReport, PhaseStreamLiveStorePromotedPackageManifestEntry,
    PhaseStreamLiveStoreQuarantinedCandidateManifestEntry, PhaseStreamLiveStoreRegistryRouteReport,
};
use super::survivor_runtime::{
    live_store_product_hot_excluded_profile_ids,
    live_store_product_hot_subcenter_priority_bucket_ids,
};

pub(super) fn write_live_store_clean_promotion_manifest(
    manifest_path: &Path,
    promoted_package_dir: &Path,
    future_shadow: &PhaseStreamLiveStoreFutureShadowReport,
    frozen_candidates: &BTreeMap<u32, LiveStoreFrozenCandidate>,
) -> Result<(), String> {
    std::fs::create_dir_all(promoted_package_dir).map_err(|error| {
        format!(
            "failed to create clean promotion package dir '{}': {error}",
            promoted_package_dir.display()
        )
    })?;
    let route_by_profile = future_shadow
        .clean_promotion_manifest_routes
        .iter()
        .map(|route| (route.profile_id, route))
        .collect::<BTreeMap<_, _>>();
    let promoted_packages = future_shadow
        .candidates
        .iter()
        .filter(|candidate| candidate.promotion_contract_eligible && candidate.registry_admitted)
        .map(
            |candidate| -> Result<PhaseStreamLiveStorePromotedPackageManifestEntry, String> {
                let frozen = frozen_candidates.get(&candidate.bucket_id).ok_or_else(|| {
                    format!(
                        "clean promotion candidate bucket {:08x} missing frozen package",
                        candidate.bucket_id
                    )
                })?;
                let package_path = promoted_package_dir.join(format!(
                    "bucket-{:08x}-{:016x}.nwpc",
                    frozen.package.bucket_id, frozen.package.package_info.fingerprint64
                ));
                std::fs::write(&package_path, &frozen.package.package_bytes).map_err(|error| {
                    format!(
                        "failed to write clean promotion package '{}': {error}",
                        package_path.display()
                    )
                })?;
                let route = route_by_profile.get(&candidate.bucket_id);
                Ok(PhaseStreamLiveStorePromotedPackageManifestEntry {
                    bucket_id: candidate.bucket_id,
                    route_id: candidate.route_id,
                    profile_id: route.map_or(candidate.bucket_id, |route| route.profile_id),
                    threshold_micro: frozen.package.threshold_micro,
                    package_path: package_path.display().to_string(),
                    package_fingerprint64: frozen.package.package_info.fingerprint64,
                    unique_cpu_accepts_over_exact_cache: candidate
                        .unique_cpu_accepts_over_exact_cache,
                    tokens_saved: candidate.tokens_saved,
                    cost_saved_microusd: candidate.cost_saved_microusd,
                })
            },
        )
        .collect::<Result<Vec<_>, _>>()?;
    let quarantined_candidates = future_shadow
        .candidates
        .iter()
        .filter(|candidate| !candidate.promotion_contract_eligible || !candidate.registry_admitted)
        .map(
            |candidate| PhaseStreamLiveStoreQuarantinedCandidateManifestEntry {
                bucket_id: candidate.bucket_id,
                route_id: candidate.route_id,
                blocker: if candidate.promotion_contract_blocker != "none" {
                    candidate.promotion_contract_blocker
                } else {
                    candidate.registry_admission_blocker
                },
                false_accepts: candidate.false_accepts,
                runtime_parity_mismatches: candidate
                    .runtime_margin_parity_mismatches
                    .saturating_add(candidate.runtime_decision_parity_mismatches),
            },
        )
        .collect::<Vec<_>>();
    let manifest = PhaseStreamLiveStoreCleanPromotionManifest {
        report_kind: "phase_stream_live_store_clean_promotion_manifest_v1",
        manifest_kind: future_shadow.clean_promotion_manifest_kind,
        allowed: future_shadow.clean_promotion_manifest_allowed,
        blocker: future_shadow.clean_promotion_manifest_blocker,
        promoted_candidate_count: future_shadow.clean_promotion_manifest_promoted_candidates,
        quarantined_candidate_count: future_shadow.clean_promotion_manifest_quarantined_candidates,
        hot_route_count: future_shadow.clean_promotion_manifest_hot_route_count,
        hot_profile_count: future_shadow.clean_promotion_manifest_hot_profile_count,
        hot_route_profile_edges: future_shadow.clean_promotion_manifest_hot_route_profile_edges,
        hot_bytes_estimate: future_shadow.clean_promotion_manifest_hot_bytes_estimate,
        unique_cpu_accepts_over_exact_cache: future_shadow
            .clean_promotion_manifest_unique_cpu_accepts_over_exact_cache,
        tokens_saved: future_shadow.clean_promotion_manifest_tokens_saved,
        cost_saved_microusd: future_shadow.clean_promotion_manifest_cost_saved_microusd,
        false_accepts: future_shadow.clean_promotion_manifest_false_accepts,
        runtime_parity_mismatches: future_shadow.clean_promotion_manifest_runtime_parity_mismatches,
        exact_cache_overlap_excluded: future_shadow
            .clean_promotion_manifest_exact_cache_overlap_excluded,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        routes: future_shadow.clean_promotion_manifest_routes.clone(),
        promoted_packages,
        quarantined_candidates,
        boundary: "clean handoff manifest only: promoted packages are verifier-bound and false_accepts=0; local_accept remains disabled until a separate verifier/live gate enables it",
    };
    super::super::write_json_file(manifest_path, &manifest)
}

pub(super) fn live_store_candidate_runtime_parity_mismatches(
    candidate: &PhaseStreamLiveStoreFutureShadowCandidateReport,
) -> usize {
    candidate
        .runtime_margin_parity_mismatches
        .saturating_add(candidate.runtime_decision_parity_mismatches)
}

pub(super) fn live_store_frozen_candidate_runtime_quarantined(
    candidate: &LiveStoreFrozenCandidate,
) -> bool {
    candidate.future_false_accepts > 0
        || candidate.future_runtime_margin_parity_mismatches > 0
        || candidate.future_runtime_decision_parity_mismatches > 0
}

fn live_store_call_token_candidate_allowed(
    candidate: &PhaseStreamLiveStoreFutureShadowCandidateReport,
) -> bool {
    candidate.false_accepts == 0
        && live_store_candidate_runtime_parity_mismatches(candidate) == 0
        && candidate.unique_cpu_accepts_over_exact_cache > 0
        && candidate.tokens_saved > 0
}

fn live_store_call_token_candidate_allowed_with_quarantine(
    candidate: &PhaseStreamLiveStoreFutureShadowCandidateReport,
    quarantined_profile_ids: &BTreeSet<u32>,
) -> bool {
    live_store_call_token_candidate_allowed(candidate)
        && !quarantined_profile_ids.contains(&candidate.bucket_id)
}

fn live_store_call_token_candidate_blocker(
    candidate: &PhaseStreamLiveStoreFutureShadowCandidateReport,
) -> &'static str {
    if candidate.false_accepts > 0 {
        "call_token_manifest_false_accepts"
    } else if live_store_candidate_runtime_parity_mismatches(candidate) > 0 {
        "call_token_manifest_runtime_parity_mismatch"
    } else if candidate.unique_cpu_accepts_over_exact_cache == 0 {
        "call_token_manifest_no_unique_accepts_over_exact_cache"
    } else if candidate.tokens_saved == 0 {
        "call_token_manifest_no_tokens_saved"
    } else {
        "none"
    }
}

fn live_store_call_token_candidate_manifest_blocker(
    candidate: &PhaseStreamLiveStoreFutureShadowCandidateReport,
    promoted_profile_ids: &BTreeSet<u32>,
    quarantined_profile_ids: &BTreeSet<u32>,
) -> &'static str {
    if quarantined_profile_ids.contains(&candidate.bucket_id) {
        "call_token_manifest_profile_quarantined"
    } else if live_store_call_token_candidate_allowed(candidate)
        && !promoted_profile_ids.contains(&candidate.bucket_id)
    {
        "call_token_manifest_not_in_promoted_route_set"
    } else {
        live_store_call_token_candidate_blocker(candidate)
    }
}

fn live_store_call_token_promotion_manifest_blocker(
    future_shadow: &PhaseStreamLiveStoreFutureShadowReport,
) -> &'static str {
    if future_shadow.call_token_promotion_manifest_promoted_candidates == 0 {
        "call_token_promotion_manifest_no_promoted_candidates"
    } else if future_shadow.call_token_promotion_manifest_false_accepts > 0 {
        "call_token_promotion_manifest_false_accepts"
    } else if future_shadow.call_token_promotion_manifest_runtime_parity_mismatches > 0 {
        "call_token_promotion_manifest_runtime_parity_mismatch"
    } else if !future_shadow.call_token_promotion_manifest_exact_cache_overlap_excluded {
        "call_token_promotion_manifest_exact_cache_overlap_not_excluded"
    } else if future_shadow.call_token_promotion_manifest_tokens_saved == 0 {
        "call_token_promotion_manifest_missing_token_denominator"
    } else if future_shadow.call_token_promotion_manifest_local_accept_enabled {
        "call_token_promotion_manifest_local_accept_enabled"
    } else if future_shadow.call_token_promotion_manifest_market_money_claim_allowed {
        "call_token_promotion_manifest_market_money_claim_enabled"
    } else if future_shadow
        .call_token_promotion_manifest_routes
        .is_empty()
    {
        "call_token_promotion_manifest_empty_route_manifest"
    } else {
        "none"
    }
}

fn live_store_clean_survivor_call_token_blocker(
    promoted_candidate_count: usize,
    false_accepts: usize,
    runtime_parity_mismatches: usize,
    exact_cache_overlap_excluded: bool,
    tokens_saved: u64,
    local_accept_enabled: bool,
    market_money_claim_allowed: bool,
    route_count: usize,
) -> &'static str {
    if promoted_candidate_count == 0 {
        "call_token_promotion_manifest_no_promoted_candidates"
    } else if false_accepts > 0 {
        "call_token_promotion_manifest_false_accepts"
    } else if runtime_parity_mismatches > 0 {
        "call_token_promotion_manifest_runtime_parity_mismatch"
    } else if !exact_cache_overlap_excluded {
        "call_token_promotion_manifest_exact_cache_overlap_not_excluded"
    } else if tokens_saved == 0 {
        "call_token_promotion_manifest_missing_token_denominator"
    } else if local_accept_enabled {
        "call_token_promotion_manifest_local_accept_enabled"
    } else if market_money_claim_allowed {
        "call_token_promotion_manifest_market_money_claim_enabled"
    } else if route_count == 0 {
        "call_token_promotion_manifest_empty_route_manifest"
    } else {
        "none"
    }
}

fn live_store_clean_survivor_candidate_blocker(
    store: &PhaseCenterLiveOperatorStore,
    bucket_id: u32,
    selected_profile_ids: &BTreeSet<u32>,
    quarantined_profile_ids: &BTreeSet<u32>,
    frozen_candidates: &BTreeMap<u32, LiveStoreFrozenCandidate>,
) -> &'static str {
    if quarantined_profile_ids.contains(&bucket_id) {
        "call_token_manifest_profile_quarantined"
    } else if !selected_profile_ids.contains(&bucket_id) {
        "call_token_manifest_not_in_promoted_route_set"
    } else if !frozen_candidates.contains_key(&bucket_id) {
        "call_token_manifest_missing_frozen_nwpc"
    } else {
        let Some(bucket) = store.miner().bucket(bucket_id) else {
            return "call_token_manifest_missing_bucket";
        };
        if bucket.false_accepts > 0 {
            "call_token_manifest_false_accepts"
        } else if bucket.unique_cpu_accepts_over_exact_cache == 0 {
            "call_token_manifest_no_unique_accepts_over_exact_cache"
        } else if bucket.tokens_saved == 0 {
            "call_token_manifest_no_tokens_saved"
        } else {
            "none"
        }
    }
}

pub(super) fn write_live_store_clean_survivor_call_token_promotion_manifest(
    manifest_path: &Path,
    promoted_package_dir: &Path,
    store: &PhaseCenterLiveOperatorStore,
    frozen_candidates: &BTreeMap<u32, LiveStoreFrozenCandidate>,
    quarantined_profile_ids: &BTreeSet<u32>,
    known_profile_kinds: &BTreeMap<u32, &'static str>,
    future_shadow: &mut PhaseStreamLiveStoreFutureShadowReport,
) -> Result<bool, String> {
    std::fs::create_dir_all(promoted_package_dir).map_err(|error| {
        format!(
            "failed to create clean-survivor call/token package dir '{}': {error}",
            promoted_package_dir.display()
        )
    })?;

    let priority_bucket_ids = live_store_product_hot_subcenter_priority_bucket_ids(
        store,
        known_profile_kinds,
        quarantined_profile_ids,
        &[],
    );
    let excluded_profile_ids = live_store_product_hot_excluded_profile_ids(
        quarantined_profile_ids,
        known_profile_kinds,
        !priority_bucket_ids.is_empty(),
    );
    let selected_profile_ids_vec = store.candidate_bucket_ids_limited_excluding_prioritized(
        store.memory_config().max_hot_profiles_per_worker,
        &excluded_profile_ids,
        &priority_bucket_ids,
    );
    let selected_profile_ids = selected_profile_ids_vec
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();

    let mut route_indexes = BTreeMap::<u32, usize>::new();
    let mut routes = Vec::<PhaseStreamLiveStoreRegistryRouteReport>::new();
    let mut promoted_packages = Vec::<PhaseStreamLiveStorePromotedPackageManifestEntry>::new();
    let mut unique_cpu_accepts_over_exact_cache = 0usize;
    let mut tokens_saved = 0u64;
    let mut cost_saved_microusd = 0u64;
    let mut false_accepts = 0usize;
    let runtime_parity_mismatches = 0usize;
    let mut hot_bytes_estimate = 0usize;

    for bucket_id in selected_profile_ids_vec {
        let Some(bucket) = store.miner().bucket(bucket_id) else {
            continue;
        };
        if bucket.false_accepts > 0
            || bucket.unique_cpu_accepts_over_exact_cache == 0
            || bucket.tokens_saved == 0
        {
            continue;
        }
        let Some(frozen) = frozen_candidates.get(&bucket_id) else {
            continue;
        };
        let Some(route_id) = store.route_id_for_bucket(bucket_id) else {
            continue;
        };
        let package_path = promoted_package_dir.join(format!(
            "bucket-{:08x}-{:016x}.nwpc",
            frozen.package.bucket_id, frozen.package.package_info.fingerprint64
        ));
        std::fs::write(&package_path, &frozen.package.package_bytes).map_err(|error| {
            format!(
                "failed to write clean-survivor call/token package '{}': {error}",
                package_path.display()
            )
        })?;
        let next_route_index = route_indexes.len();
        let route_index = *route_indexes.entry(route_id).or_insert(next_route_index);
        routes.push(PhaseStreamLiveStoreRegistryRouteReport {
            route_id,
            route_index,
            profile_id: bucket_id,
            package_fingerprint64: frozen.package.package_info.fingerprint64,
        });
        promoted_packages.push(PhaseStreamLiveStorePromotedPackageManifestEntry {
            bucket_id,
            route_id,
            profile_id: bucket_id,
            threshold_micro: frozen.package.threshold_micro,
            package_path: package_path.display().to_string(),
            package_fingerprint64: frozen.package.package_info.fingerprint64,
            unique_cpu_accepts_over_exact_cache: bucket.unique_cpu_accepts_over_exact_cache,
            tokens_saved: bucket.tokens_saved,
            cost_saved_microusd: bucket.cost_saved_microusd,
        });
        unique_cpu_accepts_over_exact_cache = unique_cpu_accepts_over_exact_cache
            .saturating_add(bucket.unique_cpu_accepts_over_exact_cache);
        tokens_saved = tokens_saved.saturating_add(bucket.tokens_saved);
        cost_saved_microusd = cost_saved_microusd.saturating_add(bucket.cost_saved_microusd);
        false_accepts = false_accepts.saturating_add(bucket.false_accepts);
        hot_bytes_estimate = hot_bytes_estimate.saturating_add(frozen.package.package_bytes.len());
    }

    let promoted_profile_ids = promoted_packages
        .iter()
        .map(|package| package.profile_id)
        .collect::<BTreeSet<_>>();
    let quarantined_candidates = frozen_candidates
        .iter()
        .filter(|(bucket_id, _)| !promoted_profile_ids.contains(bucket_id))
        .map(
            |(bucket_id, frozen)| PhaseStreamLiveStoreQuarantinedCandidateManifestEntry {
                bucket_id: *bucket_id,
                route_id: store
                    .route_id_for_bucket(*bucket_id)
                    .unwrap_or(frozen.route_id),
                blocker: live_store_clean_survivor_candidate_blocker(
                    store,
                    *bucket_id,
                    &selected_profile_ids,
                    quarantined_profile_ids,
                    frozen_candidates,
                ),
                false_accepts: store
                    .miner()
                    .bucket(*bucket_id)
                    .map_or(frozen.future_false_accepts, |bucket| bucket.false_accepts),
                runtime_parity_mismatches: frozen
                    .future_runtime_margin_parity_mismatches
                    .saturating_add(frozen.future_runtime_decision_parity_mismatches),
            },
        )
        .collect::<Vec<_>>();

    let exact_cache_overlap_excluded = unique_cpu_accepts_over_exact_cache > 0;
    let blocker = live_store_clean_survivor_call_token_blocker(
        promoted_packages.len(),
        false_accepts,
        runtime_parity_mismatches,
        exact_cache_overlap_excluded,
        tokens_saved,
        false,
        false,
        routes.len(),
    );
    let allowed = blocker == "none";

    future_shadow.call_token_promotion_manifest_kind =
        "clean_survivor_false_accept_zero_call_token_nwpc_handoff_v1";
    future_shadow.call_token_promotion_manifest_promoted_candidates = promoted_packages.len();
    future_shadow.call_token_promotion_manifest_quarantined_candidates =
        quarantined_candidates.len();
    future_shadow.call_token_promotion_manifest_hot_route_count = route_indexes.len();
    future_shadow.call_token_promotion_manifest_hot_profile_count = promoted_packages.len();
    future_shadow.call_token_promotion_manifest_hot_route_profile_edges = routes.len();
    future_shadow.call_token_promotion_manifest_hot_bytes_estimate = hot_bytes_estimate;
    future_shadow.call_token_promotion_manifest_unique_cpu_accepts_over_exact_cache =
        unique_cpu_accepts_over_exact_cache;
    future_shadow.call_token_promotion_manifest_tokens_saved = tokens_saved;
    future_shadow.call_token_promotion_manifest_cost_saved_microusd = cost_saved_microusd;
    future_shadow.call_token_promotion_manifest_false_accepts = false_accepts;
    future_shadow.call_token_promotion_manifest_runtime_parity_mismatches =
        runtime_parity_mismatches;
    future_shadow.call_token_promotion_manifest_exact_cache_overlap_excluded =
        exact_cache_overlap_excluded;
    future_shadow.call_token_promotion_manifest_local_accept_enabled = false;
    future_shadow.call_token_promotion_manifest_market_money_claim_allowed = false;
    future_shadow.call_token_promotion_manifest_routes = routes.clone();
    future_shadow.call_token_promotion_manifest_blocker = blocker;
    future_shadow.call_token_promotion_manifest_allowed = allowed;

    let manifest = PhaseStreamLiveStoreCleanPromotionManifest {
        report_kind: "phase_stream_live_store_call_token_promotion_manifest_v1",
        manifest_kind: future_shadow.call_token_promotion_manifest_kind,
        allowed,
        blocker,
        promoted_candidate_count: promoted_packages.len(),
        quarantined_candidate_count: quarantined_candidates.len(),
        hot_route_count: route_indexes.len(),
        hot_profile_count: promoted_packages.len(),
        hot_route_profile_edges: routes.len(),
        hot_bytes_estimate,
        unique_cpu_accepts_over_exact_cache,
        tokens_saved,
        cost_saved_microusd,
        false_accepts,
        runtime_parity_mismatches,
        exact_cache_overlap_excluded,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        routes,
        promoted_packages,
        quarantined_candidates,
        boundary: "clean-survivor call/token handoff only: packages are verifier-bound .nwpc candidates selected from non-quarantined phase-center survivors; local_accept and market money claim remain disabled",
    };
    super::super::write_json_file(manifest_path, &manifest)?;
    Ok(allowed)
}

pub(super) fn refresh_live_store_call_token_promotion_manifest_summary(
    future_shadow: &mut PhaseStreamLiveStoreFutureShadowReport,
    frozen_candidates: &BTreeMap<u32, LiveStoreFrozenCandidate>,
) {
    let quarantined_profile_ids = BTreeSet::new();
    refresh_live_store_call_token_promotion_manifest_summary_with_quarantine(
        future_shadow,
        frozen_candidates,
        &quarantined_profile_ids,
    );
}

pub(super) fn refresh_live_store_call_token_promotion_manifest_summary_with_quarantine(
    future_shadow: &mut PhaseStreamLiveStoreFutureShadowReport,
    frozen_candidates: &BTreeMap<u32, LiveStoreFrozenCandidate>,
    quarantined_profile_ids: &BTreeSet<u32>,
) {
    let mut routes = Vec::new();
    let mut unique_cpu_accepts_over_exact_cache = 0usize;
    let mut tokens_saved = 0u64;
    let mut cost_saved_microusd = 0u64;
    let mut false_accepts = 0usize;
    let mut runtime_parity_mismatches = 0usize;
    let mut hot_bytes_estimate = 0usize;

    let mut route_indexes = BTreeMap::<u32, usize>::new();
    for candidate in future_shadow.candidates.iter().filter(|candidate| {
        live_store_call_token_candidate_allowed_with_quarantine(candidate, quarantined_profile_ids)
    }) {
        let Some(frozen) = frozen_candidates.get(&candidate.bucket_id) else {
            continue;
        };
        let next_route_index = route_indexes.len();
        let route_index = *route_indexes
            .entry(candidate.route_id)
            .or_insert(next_route_index);
        routes.push(PhaseStreamLiveStoreRegistryRouteReport {
            route_id: candidate.route_id,
            route_index,
            profile_id: candidate.bucket_id,
            package_fingerprint64: frozen.package.package_info.fingerprint64,
        });
        unique_cpu_accepts_over_exact_cache = unique_cpu_accepts_over_exact_cache
            .saturating_add(candidate.unique_cpu_accepts_over_exact_cache);
        tokens_saved = tokens_saved.saturating_add(candidate.tokens_saved);
        cost_saved_microusd = cost_saved_microusd.saturating_add(candidate.cost_saved_microusd);
        false_accepts = false_accepts.saturating_add(candidate.false_accepts);
        runtime_parity_mismatches = runtime_parity_mismatches
            .saturating_add(live_store_candidate_runtime_parity_mismatches(candidate));
        hot_bytes_estimate = hot_bytes_estimate.saturating_add(frozen.package.package_bytes.len());
    }

    future_shadow.call_token_promotion_manifest_kind =
        "verifier_bound_false_accept_zero_call_token_nwpc_handoff_v1";
    future_shadow.call_token_promotion_manifest_promoted_candidates = routes.len();
    future_shadow.call_token_promotion_manifest_quarantined_candidates = future_shadow
        .frozen_candidate_count
        .saturating_sub(future_shadow.call_token_promotion_manifest_promoted_candidates);
    future_shadow.call_token_promotion_manifest_hot_route_count = routes.len();
    future_shadow.call_token_promotion_manifest_hot_profile_count = routes.len();
    future_shadow.call_token_promotion_manifest_hot_route_profile_edges = routes.len();
    future_shadow.call_token_promotion_manifest_hot_bytes_estimate = hot_bytes_estimate;
    future_shadow.call_token_promotion_manifest_unique_cpu_accepts_over_exact_cache =
        unique_cpu_accepts_over_exact_cache;
    future_shadow.call_token_promotion_manifest_tokens_saved = tokens_saved;
    future_shadow.call_token_promotion_manifest_cost_saved_microusd = cost_saved_microusd;
    future_shadow.call_token_promotion_manifest_false_accepts = false_accepts;
    future_shadow.call_token_promotion_manifest_runtime_parity_mismatches =
        runtime_parity_mismatches;
    future_shadow.call_token_promotion_manifest_exact_cache_overlap_excluded =
        unique_cpu_accepts_over_exact_cache > 0;
    future_shadow.call_token_promotion_manifest_local_accept_enabled = false;
    future_shadow.call_token_promotion_manifest_market_money_claim_allowed = false;
    future_shadow.call_token_promotion_manifest_routes = routes;
    future_shadow.call_token_promotion_manifest_blocker =
        live_store_call_token_promotion_manifest_blocker(future_shadow);
    future_shadow.call_token_promotion_manifest_allowed =
        future_shadow.call_token_promotion_manifest_blocker == "none";
}

pub(super) fn write_live_store_call_token_promotion_manifest(
    manifest_path: &Path,
    promoted_package_dir: &Path,
    future_shadow: &PhaseStreamLiveStoreFutureShadowReport,
    frozen_candidates: &BTreeMap<u32, LiveStoreFrozenCandidate>,
) -> Result<(), String> {
    let quarantined_profile_ids = BTreeSet::new();
    write_live_store_call_token_promotion_manifest_with_quarantine(
        manifest_path,
        promoted_package_dir,
        future_shadow,
        frozen_candidates,
        &quarantined_profile_ids,
    )
}

pub(super) fn write_live_store_call_token_promotion_manifest_with_quarantine(
    manifest_path: &Path,
    promoted_package_dir: &Path,
    future_shadow: &PhaseStreamLiveStoreFutureShadowReport,
    frozen_candidates: &BTreeMap<u32, LiveStoreFrozenCandidate>,
    quarantined_profile_ids: &BTreeSet<u32>,
) -> Result<(), String> {
    std::fs::create_dir_all(promoted_package_dir).map_err(|error| {
        format!(
            "failed to create call/token promotion package dir '{}': {error}",
            promoted_package_dir.display()
        )
    })?;
    let route_by_profile = future_shadow
        .call_token_promotion_manifest_routes
        .iter()
        .map(|route| (route.profile_id, route))
        .collect::<BTreeMap<_, _>>();
    let promoted_profile_ids = route_by_profile.keys().copied().collect::<BTreeSet<_>>();
    let promoted_packages = future_shadow
        .candidates
        .iter()
        .filter(|candidate| {
            live_store_call_token_candidate_allowed(candidate)
                && route_by_profile.contains_key(&candidate.bucket_id)
        })
        .map(
            |candidate| -> Result<PhaseStreamLiveStorePromotedPackageManifestEntry, String> {
                let frozen = frozen_candidates.get(&candidate.bucket_id).ok_or_else(|| {
                    format!(
                        "call/token promotion candidate bucket {:08x} missing frozen package",
                        candidate.bucket_id
                    )
                })?;
                let package_path = promoted_package_dir.join(format!(
                    "bucket-{:08x}-{:016x}.nwpc",
                    frozen.package.bucket_id, frozen.package.package_info.fingerprint64
                ));
                std::fs::write(&package_path, &frozen.package.package_bytes).map_err(|error| {
                    format!(
                        "failed to write call/token promotion package '{}': {error}",
                        package_path.display()
                    )
                })?;
                let route = route_by_profile.get(&candidate.bucket_id);
                Ok(PhaseStreamLiveStorePromotedPackageManifestEntry {
                    bucket_id: candidate.bucket_id,
                    route_id: candidate.route_id,
                    profile_id: route.map_or(candidate.bucket_id, |route| route.profile_id),
                    threshold_micro: frozen.package.threshold_micro,
                    package_path: package_path.display().to_string(),
                    package_fingerprint64: frozen.package.package_info.fingerprint64,
                    unique_cpu_accepts_over_exact_cache: candidate
                        .unique_cpu_accepts_over_exact_cache,
                    tokens_saved: candidate.tokens_saved,
                    cost_saved_microusd: candidate.cost_saved_microusd,
                })
            },
        )
        .collect::<Result<Vec<_>, _>>()?;
    let quarantined_candidates = future_shadow
        .candidates
        .iter()
        .filter(|candidate| !route_by_profile.contains_key(&candidate.bucket_id))
        .map(
            |candidate| PhaseStreamLiveStoreQuarantinedCandidateManifestEntry {
                bucket_id: candidate.bucket_id,
                route_id: candidate.route_id,
                blocker: live_store_call_token_candidate_manifest_blocker(
                    candidate,
                    &promoted_profile_ids,
                    quarantined_profile_ids,
                ),
                false_accepts: candidate.false_accepts,
                runtime_parity_mismatches: live_store_candidate_runtime_parity_mismatches(
                    candidate,
                ),
            },
        )
        .collect::<Vec<_>>();
    let manifest = PhaseStreamLiveStoreCleanPromotionManifest {
        report_kind: "phase_stream_live_store_call_token_promotion_manifest_v1",
        manifest_kind: future_shadow.call_token_promotion_manifest_kind,
        allowed: future_shadow.call_token_promotion_manifest_allowed,
        blocker: future_shadow.call_token_promotion_manifest_blocker,
        promoted_candidate_count: future_shadow.call_token_promotion_manifest_promoted_candidates,
        quarantined_candidate_count: future_shadow
            .call_token_promotion_manifest_quarantined_candidates,
        hot_route_count: future_shadow.call_token_promotion_manifest_hot_route_count,
        hot_profile_count: future_shadow.call_token_promotion_manifest_hot_profile_count,
        hot_route_profile_edges: future_shadow
            .call_token_promotion_manifest_hot_route_profile_edges,
        hot_bytes_estimate: future_shadow.call_token_promotion_manifest_hot_bytes_estimate,
        unique_cpu_accepts_over_exact_cache: future_shadow
            .call_token_promotion_manifest_unique_cpu_accepts_over_exact_cache,
        tokens_saved: future_shadow.call_token_promotion_manifest_tokens_saved,
        cost_saved_microusd: future_shadow.call_token_promotion_manifest_cost_saved_microusd,
        false_accepts: future_shadow.call_token_promotion_manifest_false_accepts,
        runtime_parity_mismatches: future_shadow
            .call_token_promotion_manifest_runtime_parity_mismatches,
        exact_cache_overlap_excluded: future_shadow
            .call_token_promotion_manifest_exact_cache_overlap_excluded,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        routes: future_shadow.call_token_promotion_manifest_routes.clone(),
        promoted_packages,
        quarantined_candidates,
        boundary: "call/token handoff only: verifier-bound .nwpc packages have false_accepts=0 and runtime parity=0; money claim and local_accept remain disabled until provider billing evidence and a separate gate approve them",
    };
    super::super::write_json_file(manifest_path, &manifest)
}
