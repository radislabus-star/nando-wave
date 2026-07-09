use std::collections::BTreeMap;

use nando_core::{
    PhaseCenterCell, PhaseCenterFlatRuntime, PhaseCenterHotRouteTable, PhaseCenterHotRuntime,
    PhaseCenterHotScratch, PhaseCenterLiveOperatorStore, PhaseCenterOnlineCandidatePackage,
    PhaseCenterVerifierBinding,
};

use super::diagnostics::LiveStoreBucketAccumulator;

pub(super) struct LiveStoreFrozenCandidate {
    pub(super) bucket_id: u32,
    pub(super) route_id: u32,
    pub(super) package: PhaseCenterOnlineCandidatePackage,
    pub(super) flat_runtime: PhaseCenterFlatRuntime,
    pub(super) hot_runtime: PhaseCenterHotRuntime,
    pub(super) route_table: PhaseCenterHotRouteTable,
    pub(super) route_index: usize,
    pub(super) scratch: PhaseCenterHotScratch,
    pub(super) future_scored_events: usize,
    pub(super) future_score_candidate_events: usize,
    pub(super) future_runtime_margin_parity_checks: usize,
    pub(super) future_runtime_margin_parity_mismatches: usize,
    pub(super) future_runtime_decision_parity_mismatches: usize,
    pub(super) future_unique_cpu_accepts_over_exact_cache: usize,
    pub(super) future_false_accepts: usize,
    pub(super) future_tokens_saved: u64,
    pub(super) future_cost_saved_microusd: u64,
    pub(super) future_events: Vec<LiveStoreCandidateFutureEvent>,
}

#[derive(Clone, Debug)]
pub(super) struct LiveStoreCandidateFutureEvent {
    pub(super) phase_vector: Vec<PhaseCenterCell>,
    pub(super) verified_safe_accept: bool,
    pub(super) exact_cache_hit: bool,
    pub(super) request_fingerprint: Option<String>,
    pub(super) exact_cache_key: Option<String>,
    pub(super) trace_id: Option<String>,
    pub(super) input_trace_path: Option<String>,
    pub(super) event_timestamp: Option<String>,
    pub(super) tokens: u64,
    pub(super) cost_microusd: u64,
}

pub(super) fn freeze_new_live_store_candidates(
    store: &PhaseCenterLiveOperatorStore,
    verifier_binding: PhaseCenterVerifierBinding,
    buckets: &BTreeMap<u32, LiveStoreBucketAccumulator>,
    frozen_candidates: &mut BTreeMap<u32, LiveStoreFrozenCandidate>,
) -> Result<(), String> {
    let mut packages = Vec::new();
    store
        .candidate_packages_into_with_verifier(verifier_binding, &mut packages)
        .map_err(|error| format!("failed to collect live candidate packages: {error:?}"))?;
    for package in packages {
        if frozen_candidates.contains_key(&package.bucket_id) {
            continue;
        }
        let Some(bucket) = buckets.get(&package.bucket_id) else {
            continue;
        };
        frozen_candidates.insert(
            package.bucket_id,
            live_store_frozen_candidate_from_package(bucket.route_id, package)?,
        );
    }
    Ok(())
}

pub(super) fn freeze_new_live_store_candidates_from_store(
    store: &PhaseCenterLiveOperatorStore,
    verifier_binding: PhaseCenterVerifierBinding,
    frozen_candidates: &mut BTreeMap<u32, LiveStoreFrozenCandidate>,
) -> Result<(), String> {
    let mut packages = Vec::new();
    store
        .candidate_packages_into_with_verifier(verifier_binding, &mut packages)
        .map_err(|error| format!("failed to collect live-tail candidate packages: {error:?}"))?;
    for package in packages {
        if frozen_candidates.contains_key(&package.bucket_id) {
            continue;
        }
        let Some(route_id) = store.route_id_for_bucket(package.bucket_id) else {
            continue;
        };
        frozen_candidates.insert(
            package.bucket_id,
            live_store_frozen_candidate_from_package(route_id, package)?,
        );
    }
    Ok(())
}

fn live_store_frozen_candidate_from_package(
    route_id: u32,
    package: PhaseCenterOnlineCandidatePackage,
) -> Result<LiveStoreFrozenCandidate, String> {
    let flat = PhaseCenterFlatRuntime::from_bytes(&package.package_bytes)
        .map_err(|error| format!("failed to load candidate .nwpc bytes: {error:?}"))?;
    let hot_runtime = PhaseCenterHotRuntime::from_flat_runtime(
        &flat,
        &[package.bucket_id],
        &[package.threshold_micro],
    )
    .map_err(|error| format!("failed to build candidate hot runtime: {error:?}"))?;
    let plan = hot_runtime
        .route_plan_from_profile_ids(route_id, [package.bucket_id])
        .map_err(|error| format!("failed to build candidate route plan: {error:?}"))?
        .ok_or_else(|| "candidate route plan unexpectedly empty".to_owned())?;
    let route_table = PhaseCenterHotRouteTable::from_plans([plan])
        .map_err(|error| format!("failed to build candidate route table: {error:?}"))?;
    let route_index = route_table
        .resolve_route_index(route_id)
        .ok_or_else(|| "candidate route index missing after route table build".to_owned())?;
    let scratch = PhaseCenterHotScratch::new(flat.cells(), 1)
        .map_err(|error| format!("failed to build candidate hot scratch: {error:?}"))?;
    Ok(LiveStoreFrozenCandidate {
        bucket_id: package.bucket_id,
        route_id,
        package,
        flat_runtime: flat,
        hot_runtime,
        route_table,
        route_index,
        scratch,
        future_scored_events: 0,
        future_score_candidate_events: 0,
        future_runtime_margin_parity_checks: 0,
        future_runtime_margin_parity_mismatches: 0,
        future_runtime_decision_parity_mismatches: 0,
        future_unique_cpu_accepts_over_exact_cache: 0,
        future_false_accepts: 0,
        future_tokens_saved: 0,
        future_cost_saved_microusd: 0,
        future_events: Vec::new(),
    })
}
