use nando_core::wave::PhaseCenterOnlineBucket;

#[derive(Clone, Copy, Debug)]
pub(super) struct LiveStoreOperatorPowerReport {
    pub(super) score_milli: u16,
    pub(super) class: &'static str,
    pub(super) blocker: &'static str,
    pub(super) next_auto_action: &'static str,
    pub(super) negative_memory_status: &'static str,
    pub(super) portable_operator_ready: bool,
}

pub(super) fn live_store_operator_power_report(
    bucket: &PhaseCenterOnlineBucket,
    kind: &'static str,
    min_bucket_events: usize,
) -> LiveStoreOperatorPowerReport {
    let score_milli = live_store_operator_power_score_milli(bucket, kind);
    let negative_memory_status = if bucket.negative_events > 0 {
        "present"
    } else {
        "missing_watch"
    };
    let blocker = live_store_operator_power_blocker(bucket, kind, min_bucket_events, score_milli);
    let class = live_store_operator_power_class(bucket, kind, score_milli, blocker);
    LiveStoreOperatorPowerReport {
        score_milli,
        class,
        blocker,
        next_auto_action: live_store_operator_power_next_auto_action(bucket, kind, blocker),
        negative_memory_status,
        portable_operator_ready: blocker == "none",
    }
}

pub(super) fn live_store_operator_power_allows_product_hot(
    bucket: &PhaseCenterOnlineBucket,
    kind: &'static str,
    min_bucket_events: usize,
) -> bool {
    live_store_operator_power_report(bucket, kind, min_bucket_events).portable_operator_ready
}

fn live_store_operator_power_score_milli(
    bucket: &PhaseCenterOnlineBucket,
    kind: &'static str,
) -> u16 {
    let accept_support = (bucket.unique_cpu_accepts_over_exact_cache.min(80) as u32) * 5;
    let event_support = (bucket.events_seen.min(160) as u32) * 2;
    let token_support = if bucket.tokens_saved == 0 {
        0
    } else {
        (bucket.tokens_saved.ilog2().min(18) + 1) * 18
    };
    let negative_memory_bonus = if bucket.negative_events > 0 { 90 } else { 0 };
    let kind_bonus = match kind {
        "hidden_state" => 130,
        "observable_subcenter" => 100,
        "observable_primary" => 0,
        _ => 20,
    };
    let trust_quality = bucket.trust_quality_micro.clamp(0, 1_000_000) as u32 / 5_000;
    let drift_stability = if bucket.trust_drift_micro <= 50_000 {
        120
    } else if bucket.trust_drift_micro <= 100_000 {
        80
    } else if bucket.trust_drift_micro <= 200_000 {
        30
    } else {
        0
    };
    let risk_penalty = if bucket.rejected || bucket.false_accepts > 0 {
        1_000
    } else {
        (bucket.trust_false_risk_micro.max(0) as u32 / 2_000)
            .saturating_add((bucket.trust_drift_micro.max(0) as u32 / 12_000).min(120))
    };
    accept_support
        .saturating_add(event_support)
        .saturating_add(token_support)
        .saturating_add(negative_memory_bonus)
        .saturating_add(kind_bonus)
        .saturating_add(trust_quality)
        .saturating_add(drift_stability)
        .saturating_sub(risk_penalty)
        .min(1_000) as u16
}

fn live_store_operator_power_blocker(
    bucket: &PhaseCenterOnlineBucket,
    kind: &'static str,
    min_bucket_events: usize,
    score_milli: u16,
) -> &'static str {
    if bucket.rejected || bucket.false_accepts > 0 {
        "operator_power_false_accept_or_rejected"
    } else if kind == "observable_primary" {
        "operator_power_broad_parent_requires_subcenter"
    } else if !matches!(kind, "hidden_state" | "observable_subcenter") {
        "operator_power_unknown_profile_kind"
    } else if !bucket.is_candidate() {
        "operator_power_not_candidate"
    } else if !bucket.is_shadow_ready(min_bucket_events, min_bucket_events) {
        "operator_power_shadow_not_ready"
    } else if bucket.trust_false_risk_micro > 0 {
        "operator_power_false_risk_nonzero"
    } else if bucket.unique_cpu_accepts_over_exact_cache == 0 || bucket.tokens_saved == 0 {
        "operator_power_no_unique_value"
    } else if score_milli < 300 {
        "operator_power_thin_reflex_watch"
    } else {
        "none"
    }
}

fn live_store_operator_power_class(
    bucket: &PhaseCenterOnlineBucket,
    kind: &'static str,
    score_milli: u16,
    blocker: &'static str,
) -> &'static str {
    if blocker == "operator_power_false_accept_or_rejected"
        || blocker == "operator_power_broad_parent_requires_subcenter"
        || bucket.trust_false_risk_micro > 0
    {
        "dangerous_broad_or_unclean"
    } else if blocker != "none" {
        "weak_or_unproven"
    } else if score_milli >= 700
        && bucket.unique_cpu_accepts_over_exact_cache >= 20
        && bucket.negative_events > 0
        && matches!(kind, "hidden_state" | "observable_subcenter")
    {
        "rich_transfer_operator"
    } else if score_milli >= 500 && bucket.unique_cpu_accepts_over_exact_cache >= 5 {
        "useful_transfer_operator"
    } else {
        "thin_reflex_or_low_evidence"
    }
}

fn live_store_operator_power_next_auto_action(
    bucket: &PhaseCenterOnlineBucket,
    kind: &'static str,
    blocker: &'static str,
) -> &'static str {
    match blocker {
        "none" => {
            if bucket.negative_events == 0 {
                "keep_hot_and_collect_negative_memory"
            } else {
                "keep_hot_and_monitor_drift"
            }
        }
        "operator_power_broad_parent_requires_subcenter" => {
            "split_hidden_observable_subcenters_then_retest"
        }
        "operator_power_false_accept_or_rejected" => "isolate_false_accept_atoms_and_split_deeper",
        "operator_power_shadow_not_ready" | "operator_power_not_candidate" => {
            "collect_future_stream"
        }
        "operator_power_false_risk_nonzero" => "tighten_shadow_and_hold_quarantine",
        "operator_power_thin_reflex_watch" => {
            if kind == "hidden_state" {
                "pair_with_observable_subcenter_or_hold_watch"
            } else {
                "pair_with_hidden_state_or_hold_watch"
            }
        }
        _ => "hold_watch",
    }
}
