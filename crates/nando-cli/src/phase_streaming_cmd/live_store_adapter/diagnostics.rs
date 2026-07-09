use std::collections::{BTreeMap, BTreeSet};

use super::reports::{PhaseStreamLiveStoreBucketReport, PhaseStreamLiveStoreRouteReport};
use super::source_events::LiveStoreParsedAtomEvent;

#[derive(Clone, Debug, Default)]
pub(super) struct LiveStoreRouteAccumulator {
    route_id: u32,
    route_key: String,
    events_seen: usize,
    scored_events: usize,
    exact_cache_hits: usize,
    verified_safe_accepts: usize,
    local_operator_shadow_decisions: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    false_accepts: usize,
    tokens_seen: u64,
    cost_seen_microusd: u64,
}

#[derive(Clone, Debug, Default)]
pub(super) struct LiveStoreBucketAccumulator {
    pub(super) route_id: u32,
    bucket_id: u32,
    route_key: String,
    bucket_key: String,
    events_seen: usize,
    scored_events: usize,
    exact_cache_hits: usize,
    verified_safe_accepts: usize,
    local_operator_shadow_decisions: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    false_accepts: usize,
    tokens_seen: u64,
    cost_seen_microusd: u64,
    bucket_refinement_depth: usize,
    selected_bucket_atoms: BTreeSet<String>,
}

pub(super) fn record_live_store_adapter_diagnostics(
    routes: &mut BTreeMap<u32, LiveStoreRouteAccumulator>,
    buckets: &mut BTreeMap<u32, LiveStoreBucketAccumulator>,
    event: &LiveStoreParsedAtomEvent,
    decision: nando_core::PhaseCenterOnlineDecision,
) {
    let route = routes
        .entry(event.route_id)
        .or_insert_with(|| LiveStoreRouteAccumulator {
            route_id: event.route_id,
            route_key: event.route_key.clone(),
            ..LiveStoreRouteAccumulator::default()
        });
    route.events_seen += 1;
    route.scored_events += usize::from(decision.active_before_update);
    route.exact_cache_hits += usize::from(event.exact_cache_hit);
    route.verified_safe_accepts += usize::from(event.verified_safe_accept);
    route.local_operator_shadow_decisions += usize::from(decision.local_operator_shadow_decision);
    route.unique_cpu_accepts_over_exact_cache +=
        usize::from(decision.unique_cpu_accept_over_exact_cache);
    route.false_accepts += usize::from(decision.false_accept);
    route.tokens_seen = route.tokens_seen.saturating_add(event.tokens);
    route.cost_seen_microusd = route.cost_seen_microusd.saturating_add(event.cost_microusd);

    let bucket = buckets
        .entry(event.bucket_id)
        .or_insert_with(|| LiveStoreBucketAccumulator {
            route_id: event.route_id,
            bucket_id: event.bucket_id,
            route_key: event.route_key.clone(),
            bucket_key: event.bucket_key.clone(),
            bucket_refinement_depth: event.bucket_refinement_depth,
            ..LiveStoreBucketAccumulator::default()
        });
    bucket.events_seen += 1;
    bucket.scored_events += usize::from(decision.active_before_update);
    bucket.exact_cache_hits += usize::from(event.exact_cache_hit);
    bucket.verified_safe_accepts += usize::from(event.verified_safe_accept);
    bucket.local_operator_shadow_decisions += usize::from(decision.local_operator_shadow_decision);
    bucket.unique_cpu_accepts_over_exact_cache +=
        usize::from(decision.unique_cpu_accept_over_exact_cache);
    bucket.false_accepts += usize::from(decision.false_accept);
    bucket.tokens_seen = bucket.tokens_seen.saturating_add(event.tokens);
    bucket.cost_seen_microusd = bucket
        .cost_seen_microusd
        .saturating_add(event.cost_microusd);
    bucket
        .selected_bucket_atoms
        .extend(event.selected_bucket_atoms.iter().cloned());
}

pub(super) fn live_store_route_reports(
    routes: BTreeMap<u32, LiveStoreRouteAccumulator>,
) -> Vec<PhaseStreamLiveStoreRouteReport> {
    let mut reports = routes
        .into_values()
        .map(|route| PhaseStreamLiveStoreRouteReport {
            route_id: route.route_id,
            route_key: route.route_key,
            events_seen: route.events_seen,
            scored_events: route.scored_events,
            exact_cache_hits: route.exact_cache_hits,
            verified_safe_accepts: route.verified_safe_accepts,
            local_operator_shadow_decisions: route.local_operator_shadow_decisions,
            unique_cpu_accepts_over_exact_cache: route.unique_cpu_accepts_over_exact_cache,
            false_accepts: route.false_accepts,
            tokens_seen: route.tokens_seen,
            cost_seen_microusd: route.cost_seen_microusd,
        })
        .collect::<Vec<_>>();
    reports.sort_by(|a, b| {
        b.false_accepts
            .cmp(&a.false_accepts)
            .then_with(|| {
                b.unique_cpu_accepts_over_exact_cache
                    .cmp(&a.unique_cpu_accepts_over_exact_cache)
            })
            .then_with(|| a.route_key.cmp(&b.route_key))
    });
    reports
}

pub(super) fn live_store_bucket_reports(
    buckets: BTreeMap<u32, LiveStoreBucketAccumulator>,
) -> Vec<PhaseStreamLiveStoreBucketReport> {
    let mut reports = buckets
        .into_values()
        .map(|bucket| {
            let rejected_by_shadow_safety = bucket.false_accepts > 0;
            PhaseStreamLiveStoreBucketReport {
                route_id: bucket.route_id,
                bucket_id: bucket.bucket_id,
                route_key: bucket.route_key,
                bucket_key: bucket.bucket_key,
                events_seen: bucket.events_seen,
                scored_events: bucket.scored_events,
                exact_cache_hits: bucket.exact_cache_hits,
                verified_safe_accepts: bucket.verified_safe_accepts,
                local_operator_shadow_decisions: bucket.local_operator_shadow_decisions,
                unique_cpu_accepts_over_exact_cache: bucket.unique_cpu_accepts_over_exact_cache,
                false_accepts: bucket.false_accepts,
                rejected_by_shadow_safety,
                product_candidate_allowed: !rejected_by_shadow_safety
                    && bucket.unique_cpu_accepts_over_exact_cache > 0,
                tokens_seen: bucket.tokens_seen,
                cost_seen_microusd: bucket.cost_seen_microusd,
                bucket_refinement_depth: bucket.bucket_refinement_depth,
                selected_bucket_atoms: bucket.selected_bucket_atoms.into_iter().take(24).collect(),
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
            .then_with(|| b.events_seen.cmp(&a.events_seen))
            .then_with(|| a.bucket_key.cmp(&b.bucket_key))
    });
    reports
}
