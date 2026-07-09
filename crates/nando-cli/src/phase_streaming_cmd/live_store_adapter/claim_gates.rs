use nando_core::{PhaseCenterOperatorAdmissionBlocker, PhaseCenterPromotionBlocker};

use super::reports::PhaseStreamLiveStoreCleanPromotionManifestInput;
use super::worker_path::LiveStorePreparedHotPackEval;

pub(super) fn live_store_hot_path_benchmark_blocker(
    prepared_unique_rows: usize,
    timed_score_iterations: usize,
    eval: &LiveStorePreparedHotPackEval,
    hot_path_p99_latency_ns: u128,
    runtime_margin_parity_mismatches: usize,
    runtime_decision_parity_mismatches: usize,
    runtime_budget_passed: bool,
) -> &'static str {
    if prepared_unique_rows == 0 {
        "hot_path_no_prepared_future_rows"
    } else if timed_score_iterations == 0 {
        "hot_path_no_timed_iterations"
    } else if eval.false_accepts != 0 {
        "hot_path_false_accepts_nonzero"
    } else if eval.unique_cpu_accepts_over_exact_cache == 0 {
        "hot_path_unique_accepts_zero"
    } else if runtime_margin_parity_mismatches > 0 || runtime_decision_parity_mismatches > 0 {
        "hot_path_runtime_parity_mismatch"
    } else if hot_path_p99_latency_ns > 1_000 {
        "hot_path_score_p99_budget_exceeded"
    } else if !runtime_budget_passed {
        "hot_path_runtime_budget_failed"
    } else {
        "none"
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn live_store_clean_manifest_shadow_blocker(
    manifest: &PhaseStreamLiveStoreCleanPromotionManifestInput,
    loaded_record_count: usize,
    score_events: usize,
    score_candidate_events: usize,
    verifier_required_events: usize,
    local_accept_events: usize,
    false_accepts: usize,
    runtime_margin_parity_mismatches: usize,
    runtime_decision_parity_mismatches: usize,
    route_manifest_index_mismatches: usize,
) -> &'static str {
    if !manifest.allowed {
        "manifest_not_allowed"
    } else if manifest.blocker != "none" {
        "manifest_blocker_not_none"
    } else if manifest.promoted_packages.is_empty() || loaded_record_count == 0 {
        "no_loaded_promoted_package"
    } else if manifest.routes.is_empty() {
        "empty_route_manifest"
    } else if manifest.false_accepts > 0 {
        "manifest_false_accepts"
    } else if manifest.runtime_parity_mismatches > 0 {
        "manifest_runtime_parity_mismatch"
    } else if !manifest.exact_cache_overlap_excluded {
        "manifest_exact_cache_overlap_not_excluded"
    } else if manifest.local_accept_enabled {
        "manifest_local_accept_enabled"
    } else if manifest.market_money_claim_allowed {
        "manifest_market_money_claim_allowed"
    } else if route_manifest_index_mismatches > 0 {
        "route_manifest_index_mismatch"
    } else if score_events == 0 {
        "no_score_events"
    } else if score_candidate_events == 0 {
        "no_score_candidates"
    } else if verifier_required_events != score_candidate_events {
        "score_candidate_without_verifier_required"
    } else if local_accept_events > 0 {
        "local_accept_enabled"
    } else if false_accepts > 0 {
        "false_accepts"
    } else if runtime_margin_parity_mismatches > 0 || runtime_decision_parity_mismatches > 0 {
        "runtime_parity_mismatch"
    } else {
        "none"
    }
}

pub(super) const fn live_store_promotion_blocker_name(
    blocker: PhaseCenterPromotionBlocker,
) -> &'static str {
    match blocker {
        PhaseCenterPromotionBlocker::NoFutureShadowEvents => "no_future_shadow_events",
        PhaseCenterPromotionBlocker::NoUniqueAcceptsOverExactCache => {
            "no_unique_accepts_over_exact_cache"
        }
        PhaseCenterPromotionBlocker::MissingTokenSavings => "missing_token_savings",
        PhaseCenterPromotionBlocker::MissingCostSavings => "missing_cost_savings",
        PhaseCenterPromotionBlocker::FalseAccepts => "false_accepts",
        PhaseCenterPromotionBlocker::RuntimeParityMismatch => "runtime_parity_mismatch",
        PhaseCenterPromotionBlocker::MissingVerifierBinding => "missing_verifier_binding",
        PhaseCenterPromotionBlocker::MissingAutomaticThresholdCalibration => {
            "missing_automatic_threshold_calibration"
        }
        PhaseCenterPromotionBlocker::MissingCalibrationWindowBeforeShadow => {
            "missing_calibration_window_before_shadow"
        }
        PhaseCenterPromotionBlocker::MissingShadowWindowAfterCalibration => {
            "missing_shadow_window_after_calibration"
        }
        PhaseCenterPromotionBlocker::MissingPerBucketThresholdReport => {
            "missing_per_bucket_threshold_report"
        }
        PhaseCenterPromotionBlocker::MissingFixedThresholdPolicy => {
            "missing_fixed_threshold_policy"
        }
        PhaseCenterPromotionBlocker::ExactCacheOverlapNotExcluded => {
            "exact_cache_overlap_not_excluded"
        }
        PhaseCenterPromotionBlocker::MissingTokenCostDenominator => {
            "missing_token_cost_denominator"
        }
        PhaseCenterPromotionBlocker::LocalAcceptAlreadyEnabled => "local_accept_already_enabled",
    }
}

pub(super) const fn live_store_admission_blocker_name(
    blocker: PhaseCenterOperatorAdmissionBlocker,
) -> &'static str {
    match blocker {
        PhaseCenterOperatorAdmissionBlocker::InvalidBudget => "invalid_budget",
        PhaseCenterOperatorAdmissionBlocker::PromotionBlocked(promotion_blocker) => {
            live_store_promotion_blocker_name(promotion_blocker)
        }
        PhaseCenterOperatorAdmissionBlocker::BelowMinTokensSaved => "below_min_tokens_saved",
        PhaseCenterOperatorAdmissionBlocker::BelowMinAcceptRate => "below_min_accept_rate",
        PhaseCenterOperatorAdmissionBlocker::FalseAccepts => "false_accepts",
        PhaseCenterOperatorAdmissionBlocker::EvictedByWarmBudget => "evicted_by_warm_budget",
        PhaseCenterOperatorAdmissionBlocker::EvictedByRouteBudget => "evicted_by_route_budget",
    }
}

pub(super) fn live_store_append_compression_claim_blocker(
    append_parsed_rows: usize,
    false_accepts: usize,
    local_accept_events: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    tokens_saved: u64,
    token_cost_denominator_present: bool,
    final_hot_runtime_available: bool,
    runtime_source_claim_ready: bool,
    post_quarantine_false_accepts: usize,
    min_rows: usize,
) -> &'static str {
    if append_parsed_rows == 0 {
        "append_no_rows"
    } else if false_accepts != 0 {
        "append_false_accepts_nonzero"
    } else if local_accept_events != 0 {
        "append_local_accept_enabled"
    } else if post_quarantine_false_accepts != 0 {
        "append_post_quarantine_false_accepts_nonzero"
    } else if !final_hot_runtime_available {
        "append_no_final_hot_runtime"
    } else if !runtime_source_claim_ready {
        "append_runtime_source_not_claim_ready"
    } else if append_parsed_rows < min_rows {
        "append_window_below_min_rows"
    } else if unique_cpu_accepts_over_exact_cache == 0 {
        "append_unique_accepts_zero"
    } else if tokens_saved == 0 {
        "append_tokens_saved_zero"
    } else if !token_cost_denominator_present {
        "append_token_cost_denominator_missing"
    } else {
        "none"
    }
}

pub(super) fn live_store_product_hot_runtime_source_claim_ready(source: &str) -> bool {
    matches!(
        source,
        "call_token_active_manifest"
            | "call_token_promotion_manifest"
            | "product_hot_registry"
            | "live_store_clean_candidate_survivors"
            | "live_store_clean_candidate_survivors_route_refreshed"
    )
}
