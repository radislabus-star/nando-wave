use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use nando_core::PhaseCenterLiveOperatorStore;
use nando_core::wave::PhaseCenterOnlineBucket;

use super::defaults::DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_CANDIDATE_FRONTIER_LIMIT;
use super::operator_power::{
    live_store_operator_power_allows_product_hot, live_store_operator_power_report,
};
use super::state::LiveStoreProductHotRegistryRuntimeBundle;

pub(super) fn live_store_clean_candidate_frontier(
    store: &PhaseCenterLiveOperatorStore,
    quarantined_profile_ids: &BTreeSet<u32>,
) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let clean_candidate_profile_ids = store.candidate_bucket_ids_limited(
        DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_CANDIDATE_FRONTIER_LIMIT,
    );
    let clean_candidate_quarantined_profile_ids = clean_candidate_profile_ids
        .iter()
        .copied()
        .filter(|profile_id| quarantined_profile_ids.contains(profile_id))
        .collect::<Vec<_>>();
    let clean_candidate_exportable_profile_ids = clean_candidate_profile_ids
        .iter()
        .copied()
        .filter(|profile_id| !quarantined_profile_ids.contains(profile_id))
        .collect::<Vec<_>>();
    (
        clean_candidate_profile_ids,
        clean_candidate_quarantined_profile_ids,
        clean_candidate_exportable_profile_ids,
    )
}

pub(super) fn live_store_clean_candidate_value_reports(
    store: &PhaseCenterLiveOperatorStore,
    profile_ids: &[u32],
    quarantined_profile_ids: &BTreeSet<u32>,
    known_profile_kinds: &BTreeMap<u32, &'static str>,
    final_hot_profile_ids: &[u32],
    min_bucket_events: usize,
) -> Vec<serde_json::Value> {
    profile_ids
        .iter()
        .filter_map(|profile_id| {
            let bucket = store.miner().bucket(*profile_id)?;
            let route_id = store.route_id_for_bucket(*profile_id);
            let quarantined = quarantined_profile_ids.contains(profile_id);
            let active = bucket.is_active(min_bucket_events);
            let shadow_ready = bucket.is_shadow_ready(min_bucket_events, min_bucket_events);
            let final_hot = final_hot_profile_ids.contains(profile_id);
            let kind = known_profile_kinds
                .get(profile_id)
                .copied()
                .unwrap_or("unknown");
            let operator_power = live_store_operator_power_report(bucket, kind, min_bucket_events);
            let exportable = !quarantined && operator_power.portable_operator_ready;
            let auto_recovery_running = active
                && !final_hot
                && (quarantined
                    || !operator_power.portable_operator_ready
                    || operator_power.blocker != "none");
            Some(serde_json::json!({
                "profile_id": profile_id,
                "kind": kind,
                "route_id": route_id,
                "quarantined": quarantined,
                "exportable": exportable,
                "final_hot": final_hot,
                "candidate": bucket.is_candidate(),
                "active": active,
                "shadow_ready": shadow_ready,
                "auto_recovery_running": auto_recovery_running,
                "promotion_blocker": live_store_candidate_promotion_blocker(
                    bucket,
                    operator_power.blocker,
                    quarantined,
                    active,
                    shadow_ready,
                    exportable,
                    final_hot,
                ),
                "next_auto_action": live_store_candidate_next_auto_action(
                    bucket,
                    operator_power.next_auto_action,
                    quarantined,
                    active,
                    shadow_ready,
                    exportable,
                    final_hot,
                ),
                "best_split_candidate": live_store_candidate_best_split_candidate(bucket),
                "operator_power_score_milli": operator_power.score_milli,
                "operator_power_class": operator_power.class,
                "operator_power_blocker": operator_power.blocker,
                "operator_power_next_auto_action": operator_power.next_auto_action,
                "operator_power_negative_memory": operator_power.negative_memory_status,
                "operator_power_portable_ready": operator_power.portable_operator_ready,
                "recovery_retry_after_events": live_store_candidate_recovery_retry_after_events(
                    bucket,
                    active,
                    shadow_ready,
                    min_bucket_events,
                ),
                "events_seen": bucket.events_seen,
                "positive_events": bucket.positive_events,
                "negative_events": bucket.negative_events,
                "scored_events": bucket.scored_events,
                "calibration_events_seen": bucket.calibration_events_seen,
                "learned_threshold_micro": bucket.learned_threshold_micro,
                "max_calibration_false_margin_micro": bucket.max_calibration_false_margin_micro,
                "unique_cpu_accepts_over_exact_cache": bucket.unique_cpu_accepts_over_exact_cache,
                "tokens_saved": bucket.tokens_saved,
                "cost_saved_microusd": bucket.cost_saved_microusd,
                "false_accepts": bucket.false_accepts,
                "rejected": bucket.rejected,
                "trust_quality_micro": bucket.trust_quality_micro,
                "trust_false_risk_micro": bucket.trust_false_risk_micro,
                "trust_drift_micro": bucket.trust_drift_micro,
                "trust_token_value_micro": bucket.trust_token_value_micro
            }))
        })
        .collect()
}

fn live_store_candidate_promotion_blocker(
    bucket: &PhaseCenterOnlineBucket,
    operator_power_blocker: &'static str,
    quarantined: bool,
    active: bool,
    shadow_ready: bool,
    exportable: bool,
    final_hot: bool,
) -> &'static str {
    if final_hot {
        "none_final_hot"
    } else if operator_power_blocker != "none" {
        operator_power_blocker
    } else if bucket.rejected || bucket.false_accepts > 0 {
        "false_accept_or_rejected"
    } else if !active {
        "min_events_not_met"
    } else if !shadow_ready {
        "shadow_ready_not_met"
    } else if bucket.trust_false_risk_micro > 0 {
        "trust_false_risk_nonzero"
    } else if bucket.trust_drift_micro > 100_000 {
        "trust_drift_high"
    } else if bucket.negative_events > 0 {
        "mixed_positive_negative_evidence"
    } else if bucket.unique_cpu_accepts_over_exact_cache == 0 || bucket.tokens_saved == 0 {
        "no_unique_value_over_exact_cache"
    } else if quarantined {
        "awaiting_clean_child_promotion"
    } else if exportable {
        "none_exportable"
    } else {
        "watch"
    }
}

fn live_store_candidate_next_auto_action(
    bucket: &PhaseCenterOnlineBucket,
    operator_power_next_auto_action: &'static str,
    quarantined: bool,
    active: bool,
    shadow_ready: bool,
    exportable: bool,
    final_hot: bool,
) -> &'static str {
    if final_hot {
        "none"
    } else if operator_power_next_auto_action != "keep_hot_and_monitor_drift"
        && operator_power_next_auto_action != "keep_hot_and_collect_negative_memory"
    {
        operator_power_next_auto_action
    } else if exportable && !quarantined {
        "none"
    } else if bucket.rejected || bucket.false_accepts > 0 {
        "isolate_false_accept_atoms_and_split_deeper"
    } else if !active || !shadow_ready {
        "collect_future_stream"
    } else if bucket.trust_false_risk_micro > 0 {
        "tighten_shadow_and_hold_quarantine"
    } else if bucket.trust_drift_micro > 100_000 {
        "spawn_bounded_pair_triple_recovery_subcenters"
    } else if bucket.negative_events > 0 {
        "split_positive_negative_evidence"
    } else if bucket.unique_cpu_accepts_over_exact_cache == 0 || bucket.tokens_saved == 0 {
        "wait_for_unique_value"
    } else {
        "promotion_audit"
    }
}

fn live_store_candidate_best_split_candidate(bucket: &PhaseCenterOnlineBucket) -> &'static str {
    if bucket.rejected || bucket.false_accepts > 0 {
        "false_accept_trace_atoms"
    } else if bucket.trust_drift_micro > 100_000 {
        "bounded_pair_or_triple_safe_atoms"
    } else if bucket.negative_events > 0 {
        "negative_evidence_atoms"
    } else {
        "safe_subcenter_atoms"
    }
}

fn live_store_candidate_recovery_retry_after_events(
    bucket: &PhaseCenterOnlineBucket,
    active: bool,
    shadow_ready: bool,
    min_bucket_events: usize,
) -> usize {
    if !active {
        min_bucket_events.saturating_sub(bucket.events_seen)
    } else if !shadow_ready {
        min_bucket_events.saturating_sub(bucket.calibration_events_seen)
    } else if bucket.rejected || bucket.false_accepts > 0 {
        min_bucket_events
    } else {
        1
    }
}

pub(super) fn live_store_trusted_clean_candidate_profile_ids(
    store: &PhaseCenterLiveOperatorStore,
    profile_ids: &[u32],
    min_bucket_events: usize,
) -> Vec<u32> {
    profile_ids
        .iter()
        .copied()
        .filter(|profile_id| {
            live_store_product_hot_profile_phase_trusted(store, *profile_id, min_bucket_events)
        })
        .collect()
}

pub(super) fn live_store_clean_candidate_survivor_runtime_from_store(
    store: &PhaseCenterLiveOperatorStore,
    cells: usize,
    registry_path: &Path,
    quarantined_profile_ids: &BTreeSet<u32>,
    priority_bucket_ids: &[u32],
    known_profile_kinds: &BTreeMap<u32, &'static str>,
    min_bucket_events: usize,
) -> Result<Option<LiveStoreProductHotRegistryRuntimeBundle>, String> {
    let subcenter_priority_bucket_ids = live_store_product_hot_subcenter_priority_bucket_ids(
        store,
        known_profile_kinds,
        quarantined_profile_ids,
        priority_bucket_ids,
        min_bucket_events,
    );
    let excluded_profile_ids = live_store_product_hot_excluded_profile_ids(
        quarantined_profile_ids,
        known_profile_kinds,
        !subcenter_priority_bucket_ids.is_empty(),
    );
    let Some((hot_runtime, route_table)) = store
        .candidate_hot_runtime_and_route_table_excluding_prioritized(
            &excluded_profile_ids,
            &subcenter_priority_bucket_ids,
        )
        .map_err(|error| {
            format!("failed to build clean candidate survivor hot runtime: {error:?}")
        })?
    else {
        return Ok(None);
    };
    let package_bytes = hot_runtime
        .bytes_estimate()
        .saturating_add(route_table.bytes_estimate());
    Ok(Some(LiveStoreProductHotRegistryRuntimeBundle {
        registry_path: registry_path.to_path_buf(),
        hot_runtime,
        route_table,
        cells,
        package_bytes,
    }))
}

pub(super) fn live_store_product_hot_subcenter_priority_bucket_ids(
    store: &PhaseCenterLiveOperatorStore,
    known_profile_kinds: &BTreeMap<u32, &'static str>,
    quarantined_profile_ids: &BTreeSet<u32>,
    priority_bucket_ids: &[u32],
    min_bucket_events: usize,
) -> Vec<u32> {
    let mut ids = Vec::new();
    for bucket_id in priority_bucket_ids {
        if live_store_product_hot_subcenter_candidate_allowed(
            store,
            known_profile_kinds,
            quarantined_profile_ids,
            *bucket_id,
            min_bucket_events,
        ) {
            ids.push(*bucket_id);
        }
    }
    let mut known_subcenters = known_profile_kinds
        .iter()
        .filter(|(_, kind)| **kind == "observable_subcenter" || **kind == "hidden_state")
        .filter_map(|(bucket_id, _)| {
            live_store_product_hot_subcenter_candidate_allowed(
                store,
                known_profile_kinds,
                quarantined_profile_ids,
                *bucket_id,
                min_bucket_events,
            )
            .then_some(*bucket_id)
        })
        .collect::<Vec<_>>();
    known_subcenters.sort_by(|left, right| {
        let left_bucket = store.miner().bucket(*left);
        let right_bucket = store.miner().bucket(*right);
        right_bucket
            .map_or(0, |bucket| bucket.tokens_saved)
            .cmp(&left_bucket.map_or(0, |bucket| bucket.tokens_saved))
            .then_with(|| {
                right_bucket
                    .map_or(0, |bucket| bucket.unique_cpu_accepts_over_exact_cache)
                    .cmp(
                        &left_bucket.map_or(0, |bucket| bucket.unique_cpu_accepts_over_exact_cache),
                    )
            })
            .then_with(|| left.cmp(right))
    });
    ids.extend(known_subcenters);
    let mut ordered_ids = Vec::with_capacity(ids.len());
    for bucket_id in ids {
        if !ordered_ids.contains(&bucket_id) {
            ordered_ids.push(bucket_id);
        }
    }
    ordered_ids
}

pub(super) fn live_store_power_allowed_hot_profile_count(
    store: &PhaseCenterLiveOperatorStore,
    hot_runtime: &nando_core::PhaseCenterHotRuntime,
    known_profile_kinds: &BTreeMap<u32, &'static str>,
    quarantined_profile_ids: &BTreeSet<u32>,
    min_bucket_events: usize,
) -> usize {
    (0..hot_runtime.profile_count())
        .filter_map(|index| hot_runtime.profile_id_at(index))
        .filter(|profile_id| {
            live_store_product_hot_score_candidate_allowed(
                store,
                known_profile_kinds,
                quarantined_profile_ids,
                *profile_id,
                min_bucket_events,
            )
        })
        .count()
}

pub(super) fn live_store_product_hot_subcenter_candidate_allowed(
    store: &PhaseCenterLiveOperatorStore,
    known_profile_kinds: &BTreeMap<u32, &'static str>,
    quarantined_profile_ids: &BTreeSet<u32>,
    bucket_id: u32,
    min_bucket_events: usize,
) -> bool {
    live_store_product_hot_score_candidate_allowed(
        store,
        known_profile_kinds,
        quarantined_profile_ids,
        bucket_id,
        min_bucket_events,
    )
}

pub(super) fn live_store_product_hot_score_candidate_allowed(
    store: &PhaseCenterLiveOperatorStore,
    known_profile_kinds: &BTreeMap<u32, &'static str>,
    quarantined_profile_ids: &BTreeSet<u32>,
    bucket_id: u32,
    min_bucket_events: usize,
) -> bool {
    let Some(kind) = known_profile_kinds.get(&bucket_id).copied() else {
        return false;
    };
    if !matches!(kind, "observable_subcenter" | "hidden_state") {
        return false;
    }
    let Some(bucket) = store.miner().bucket(bucket_id) else {
        return false;
    };
    !quarantined_profile_ids.contains(&bucket_id)
        && live_store_product_hot_profile_phase_trusted(store, bucket_id, min_bucket_events)
        && live_store_operator_power_allows_product_hot(bucket, kind, min_bucket_events)
}

pub(super) fn live_store_product_hot_profile_phase_trusted(
    store: &PhaseCenterLiveOperatorStore,
    bucket_id: u32,
    min_bucket_events: usize,
) -> bool {
    let Some(bucket) = store.miner().bucket(bucket_id) else {
        return false;
    };
    bucket.is_candidate()
        && bucket.is_shadow_ready(min_bucket_events, min_bucket_events)
        && !bucket.rejected
        && bucket.false_accepts == 0
        && bucket.trust_false_risk_micro == 0
        && bucket.trust_quality_micro > 0
        && bucket.tokens_saved > 0
        && bucket.unique_cpu_accepts_over_exact_cache > 0
}

pub(super) fn live_store_product_hot_excluded_profile_ids(
    quarantined_profile_ids: &BTreeSet<u32>,
    known_profile_kinds: &BTreeMap<u32, &'static str>,
    subcenter_candidates_available: bool,
) -> Vec<u32> {
    let mut excluded = quarantined_profile_ids.iter().copied().collect::<Vec<_>>();
    if subcenter_candidates_available {
        excluded.extend(
            known_profile_kinds
                .iter()
                .filter(|(_, kind)| **kind == "observable_primary")
                .map(|(bucket_id, _)| *bucket_id),
        );
    }
    excluded.sort_unstable();
    excluded.dedup();
    excluded
}
