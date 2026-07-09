use nando_core::{
    PhaseCenterHotRouteTable, PhaseCenterHotRuntime, PhaseCenterRuntimeBudgetSnapshot,
};

use super::reports::PhaseStreamLiveStoreBudgetReport;

pub(super) fn live_store_hot_route_ids(route_table: &PhaseCenterHotRouteTable) -> Vec<u32> {
    (0..route_table.route_count())
        .filter_map(|index| route_table.route_id_at(index))
        .collect()
}

pub(super) fn live_store_hot_profile_ids(hot_runtime: &PhaseCenterHotRuntime) -> Vec<u32> {
    (0..hot_runtime.profile_count())
        .filter_map(|index| hot_runtime.profile_id_at(index))
        .collect()
}

pub(super) fn live_store_per_thousand(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        0
    } else {
        numerator.saturating_mul(1000) / denominator
    }
}

pub(super) fn live_store_per_thousand_u64(numerator: u64, denominator: u64) -> usize {
    if denominator == 0 {
        0
    } else {
        numerator.saturating_mul(1000).saturating_div(denominator) as usize
    }
}

pub(super) fn live_store_env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

pub(super) fn live_store_milli(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        0
    } else {
        numerator.saturating_mul(1000) / denominator
    }
}

pub(super) fn live_store_latency_percentile(sorted_latencies: &[u128], percentile: usize) -> u128 {
    if sorted_latencies.is_empty() {
        return 0;
    }
    let clamped = percentile.min(100);
    let index = ((sorted_latencies.len() - 1) * clamped) / 100;
    sorted_latencies[index]
}

pub(super) fn live_store_row_has_provider_billing(row: &serde_json::Value) -> bool {
    row.get("provider_cost_microusd")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
        > 0
        || row
            .get("token_cost")
            .and_then(|token_cost| token_cost.get("provider_cost_microusd"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            > 0
}

pub(super) fn live_store_row_has_estimated_cost(row: &serde_json::Value) -> bool {
    if live_store_row_has_provider_billing(row) {
        return false;
    }
    let top_level_estimate = row
        .get("estimated_total_cost_microusd")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let nested_total = row
        .get("token_cost")
        .and_then(|token_cost| token_cost.get("total_cost_microusd"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    top_level_estimate > 0 || nested_total > 0
}

pub(super) fn live_store_budget_report(
    snapshot: PhaseCenterRuntimeBudgetSnapshot,
) -> PhaseStreamLiveStoreBudgetReport {
    PhaseStreamLiveStoreBudgetReport {
        max_hot_profiles_per_worker: snapshot.max_hot_profiles_per_worker,
        max_hot_bytes_per_worker: snapshot.max_hot_bytes_per_worker,
        max_warm_profiles_per_process: snapshot.max_warm_profiles_per_process,
        max_profiles_per_route: snapshot.max_profiles_per_route,
        max_route_top_k: snapshot.max_route_top_k,
        warm_route_count: snapshot.warm_route_count,
        warm_profile_count: snapshot.warm_profile_count,
        warm_metadata_bytes_estimate: snapshot.warm_metadata_bytes_estimate,
        warm_runtime_bytes_estimate: snapshot.warm_runtime_bytes_estimate,
        warm_bytes_estimate: snapshot.warm_bytes_estimate,
        hot_route_count: snapshot.hot_route_count,
        hot_profile_count: snapshot.hot_profile_count,
        hot_route_profile_edges: snapshot.hot_route_profile_edges,
        hot_runtime_bytes_estimate: snapshot.hot_runtime_bytes_estimate,
        hot_route_table_bytes_estimate: snapshot.hot_route_table_bytes_estimate,
        hot_bytes_estimate: snapshot.hot_bytes_estimate,
        hot_budget_passed: snapshot.hot_budget_passed(),
        warm_budget_passed: snapshot.warm_budget_passed(),
        product_runtime_budget_passed: snapshot.product_runtime_budget_passed(),
    }
}
