use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nando_core::{
    PhaseCenterCompiler, PhaseCenterEvalTask, PhaseCenterFlatRuntime, PhaseCenterHotRuntime,
    PhaseCenterOffloadPolicy, PhaseCenterOffloadRuntime, PhaseCenterRuntimeBudgetSnapshot,
};
use serde::Serialize;
use serde_json::Value;

use super::{
    DEFAULT_AUTO_SUBCENTER_DISCOVERY_CANDIDATES_JSONL, DEFAULT_PRICE_CONFIG,
    DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO, GenericTokenCost, ModelPriceConfig,
    PhaseAtomBinaryEvent, estimated_event_cost_microusd, json_bool, json_string, json_u64,
    margin_to_micro, parse_phase_atom_binary_event_for_action, percentile_i64, percentile_u128,
    phase_atom_action_families, phase_atom_binary_event_vector_for_task,
    phase_atom_binary_token_cost, phase_atom_bucket_selector,
    phase_atom_live_self_mining_task_name, phase_atom_state_action_bucket_key,
    phase_atom_string_vec, read_json_file, read_json_value, sanitize_file_stem, stable_fingerprint,
    write_binary_file, write_json_file,
};

const DEFAULT_ONLINE_MINER_DAEMON_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-daemon-v1.report.json";
const DEFAULT_ONLINE_MINER_VALUE_PASS_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-value-pass-v1.report.json";
const DEFAULT_ONLINE_MINER_TARGETED_SHADOW_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-targeted-shadow-v1.report.json";
const DEFAULT_ONLINE_MINER_TARGETED_REJECTION_DRILLDOWN_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-rejection-drilldown-v1.report.json";
const DEFAULT_ONLINE_MINER_TARGETED_SHADOW_DIR: &str =
    "target/nando-wave/streaming/online-miner-targeted-shadow-v1";
const DEFAULT_ONLINE_MINER_PROMOTION_REGISTRY_GATE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-promotion-registry-gate-v1.report.json";
const DEFAULT_ONLINE_MINER_DAEMON_CHECKPOINT_DIR: &str =
    "target/nando-wave/streaming/online-miner-daemon-v1";
const DEFAULT_ONLINE_MINER_PROMOTION_SHADOW_REGISTRY_DIR: &str =
    "target/nando-wave/streaming/online-miner-promotion-shadow-registry-v1";
const DEFAULT_ONLINE_MINER_PROMOTION_REGISTRY: &str =
    "target/nando-wave/streaming/online-miner-daemon-v1/product-hot-promotion-registry.shadow.json";
const DEFAULT_ONLINE_MINER_DAEMON_DECISION_LOG: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-daemon-v1.decisions.jsonl";
const DEFAULT_ONLINE_MINER_RESERVOIR_PER_LABEL: usize = 1200;
const ONLINE_MINER_AUTO_CALIBRATION_MAX_DECISIONS: usize = 1024;
const ONLINE_MINER_PRODUCT_MAX_HOT_PROFILES_PER_WORKER: usize = 8;
const ONLINE_MINER_SHADOW_AUDITION_MAX_ACTIVE_BUCKETS: usize = 32;
const ONLINE_MINER_PRODUCT_MAX_HOT_BYTES_PER_WORKER: usize = 3 * 1024 * 1024;
const ONLINE_MINER_PRODUCT_MAX_WARM_PROFILES_PER_PROCESS: usize = 256;
const ONLINE_MINER_PRODUCT_MAX_PROFILES_PER_ROUTE: usize = 4;
const ONLINE_MINER_PRODUCT_MAX_ROUTE_TOP_K: usize = 4;
const ONLINE_MINER_VALUE_PASS_DEFAULT_TOP_K: usize = 32;
const ONLINE_MINER_MAX_AUTOSUBCENTER_BUCKETS_PER_EVENT: usize = 4;
const ONLINE_MINER_AUTOSUBCENTER_SINGLE_BASIS_LIMIT: usize = 12;
const ONLINE_MINER_AUTOSUBCENTER_REQUEST_BASIS_LIMIT: usize = 6;
const ONLINE_MINER_AUTOSUBCENTER_STATE_BASIS_LIMIT: usize = 6;
const ONLINE_MINER_AUTOSUBCENTER_TOOL_BASIS_LIMIT: usize = 6;
const ONLINE_MINER_HIDDEN_STATE_BASIS_LIMIT: usize = 6;
const ONLINE_MINER_HIDDEN_STATE_MAX_CANDIDATES: usize = 16;
const ONLINE_MINER_AUTOSUBCENTER_MAX_MULTI3_CANDIDATES: usize = 24;
const ONLINE_MINER_AUTOSUBCENTER_MAX_MULTI4_CANDIDATES: usize = 24;
const ONLINE_MINER_LEARNED_SPLIT_MIN_BROAD_EVENTS: usize = 8;
const ONLINE_MINER_LEARNED_SPLIT_MIN_BROAD_LABELS: usize = 2;
const ONLINE_MINER_MODE_ENV: &str = "NANDO_ONLINE_MINER_MODE";
const ONLINE_MINER_ROW_BUDGET_ENV: &str = "NANDO_ONLINE_MINER_ROW_BUDGET";
const ONLINE_MINER_TIME_BUDGET_MS_ENV: &str = "NANDO_ONLINE_MINER_TIME_BUDGET_MS";
const ONLINE_MINER_COMPILE_BUDGET_ENV: &str = "NANDO_ONLINE_MINER_COMPILE_BUDGET";
const ONLINE_MINER_DECISION_LOG_LIMIT_ENV: &str = "NANDO_ONLINE_MINER_DECISION_LOG_LIMIT";
const ONLINE_MINER_SAMPLE_EVERY_ENV: &str = "NANDO_ONLINE_MINER_SAMPLE_EVERY";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OnlineMinerControlMode {
    Off,
    ShadowLight,
    BurstDiscovery,
    ProductHotScoreOnly,
}

impl OnlineMinerControlMode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::ShadowLight => "SHADOW_LIGHT",
            Self::BurstDiscovery => "BURST_DISCOVERY",
            Self::ProductHotScoreOnly => "PRODUCT_HOT_SCORE_ONLY",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OnlineMinerControlBudget {
    mode: OnlineMinerControlMode,
    row_budget: Option<usize>,
    time_budget: Option<Duration>,
    compile_budget: Option<usize>,
    decision_log_limit: Option<usize>,
    sample_every: usize,
}

#[derive(Clone, Debug, Serialize)]
struct OnlineMinerDaemonReport {
    report_kind: &'static str,
    mode: &'static str,
    control_mode: &'static str,
    row_budget: Option<usize>,
    time_budget_ms: Option<u64>,
    compile_budget: Option<usize>,
    compile_budget_used: usize,
    compile_budget_exhausted: bool,
    decision_log_limit: Option<usize>,
    decision_log_rows_written: usize,
    decision_log_capped: bool,
    sample_every: usize,
    sampled_out_rows: usize,
    budget_stop_reason: Option<&'static str>,
    trace_paths: Vec<String>,
    checkpoint_dir: String,
    decision_log_path: String,
    cells: usize,
    min_bucket_events: usize,
    base_margin_threshold_micro: i64,
    compile_every_rows: usize,
    requested_max_active_buckets: usize,
    max_active_buckets: usize,
    shadow_audition_max_active_buckets: usize,
    shadow_audition_cap_enforced: bool,
    product_hot_profile_cap_enforced: bool,
    product_hot_profile_cap_exceeded_by_shadow_audition: bool,
    reservoir_per_label: usize,
    total_rows: usize,
    parsed_events: usize,
    source_verifier_labeled_events: usize,
    skipped_no_action_family: usize,
    skipped_no_verifier_label: usize,
    skipped_no_phase_atoms: usize,
    online_auto_subcenter_split_enabled: bool,
    online_auto_subcenter_max_per_event: usize,
    online_auto_subcenter_multi2_enabled: bool,
    online_auto_subcenter_multi3_enabled: bool,
    online_auto_subcenter_multi4_enabled: bool,
    online_auto_subcenter_split_authority: &'static str,
    manual_class_list_used: bool,
    broad_action_bucket_updates: usize,
    state_action_bucket_updates: usize,
    auto_subcenter_bucket_updates: usize,
    learned_subcenter_bucket_updates: usize,
    learned_split_registry_action_count: usize,
    learned_split_atom_count: usize,
    learned_split_single_atom_count: usize,
    learned_split_multi2_atom_count: usize,
    learned_split_multi3_atom_count: usize,
    learned_split_multi4_atom_count: usize,
    learned_split_compound_atom_count: usize,
    learned_split_conflict_gate_enabled: bool,
    learned_split_conflict_action_count: usize,
    learned_split_atoms_blocked_without_conflict: usize,
    hidden_state_search_enabled: bool,
    hidden_state_authority: &'static str,
    hidden_state_count: usize,
    hidden_state_transition_count: usize,
    hidden_state_split_candidates: usize,
    hidden_state_bucket_count: usize,
    hidden_state_forbidden_source_leak_bucket_count: usize,
    hot_audition_replacement_enabled: bool,
    hot_audition_replacement_count: usize,
    base_bucket_count: usize,
    state_action_bucket_count: usize,
    auto_subcenter_bucket_count: usize,
    auto_subcenter_multi2_bucket_count: usize,
    auto_subcenter_multi3_bucket_count: usize,
    auto_subcenter_multi4_bucket_count: usize,
    auto_subcenter_forbidden_source_leak_bucket_count: usize,
    learned_subcenter_bucket_count: usize,
    learned_subcenter_multi2_bucket_count: usize,
    learned_subcenter_multi3_bucket_count: usize,
    learned_subcenter_multi4_bucket_count: usize,
    bucket_count: usize,
    compile_ticks: usize,
    bucket_readiness_compile_count: usize,
    compiled_checkpoint_count: usize,
    active_profile_count: usize,
    future_shadow_events: usize,
    future_shadow_safe_events: usize,
    local_operator_shadow_decisions: usize,
    fallback_shadow_decisions: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    cli_margin_floor_micro: i64,
    cli_threshold_selected_for_acceptance: bool,
    threshold_source: &'static str,
    calibration_window_before_shadow: bool,
    shadow_window_after_calibration: bool,
    per_bucket_thresholds_reported: bool,
    fixed_policy_shadow_replay: bool,
    accepted_delta_reported_against_fixed_policy: bool,
    auto_calibration_window_max_decisions: usize,
    auto_calibrated_margin_threshold_micro: i64,
    auto_calibrated_calibration_events: usize,
    auto_calibrated_shadow_events_after_calibration: usize,
    auto_calibrated_local_operator_shadow_decisions: usize,
    auto_calibrated_unique_cpu_accepts_over_exact_cache: usize,
    auto_calibrated_nando_cpu_tokens_saved: usize,
    auto_calibrated_nando_cpu_cost_saved_microusd: u64,
    auto_calibrated_false_accepts: usize,
    auto_calibrated_max_false_margin_micro: Option<i64>,
    auto_calibrated_global_unique_cpu_accepts_over_exact_cache: usize,
    auto_calibrated_global_nando_cpu_tokens_saved: usize,
    auto_calibrated_global_nando_cpu_cost_saved_microusd: u64,
    auto_calibrated_global_duplicate_accept_rows: usize,
    auto_calibrated_global_false_accepts: usize,
    active_hot_unique_cpu_accepts_over_exact_cache: usize,
    active_hot_false_accepts: usize,
    active_hot_auto_calibrated_unique_cpu_accepts_over_exact_cache: usize,
    active_hot_auto_calibrated_false_accepts: usize,
    active_hot_auto_calibrated_accepted_bucket_count: usize,
    active_hot_auto_calibrated_rejected_bucket_count: usize,
    active_hot_auto_calibrated_rejected_false_accepts: usize,
    multi_split_portfolio_enabled: bool,
    multi_split_candidate_profile_count: usize,
    multi_split_unique_cpu_accepts_over_exact_cache: usize,
    multi_split_nando_cpu_tokens_saved: usize,
    multi_split_nando_cpu_cost_saved_microusd: u64,
    multi_split_duplicate_accept_rows: usize,
    multi_split_false_accepts: usize,
    multi_split_rejected_shadow_bucket_count: usize,
    multi_split_rejected_shadow_false_accepts: usize,
    multi_split_promotion_candidate_count: usize,
    multi_split_promotion_gate_passed_count: usize,
    multi_split_promotion_gate_failed_count: usize,
    multi_split_promotion_candidates: Vec<OnlineMinerPromotionCandidateReport>,
    product_hot_portfolio_enabled: bool,
    product_hot_candidate_profile_count: usize,
    product_hot_unique_cpu_accepts_over_exact_cache: usize,
    product_hot_nando_cpu_tokens_saved: usize,
    product_hot_nando_cpu_cost_saved_microusd: u64,
    product_hot_duplicate_accept_rows: usize,
    product_hot_false_accepts: usize,
    product_hot_rejected_shadow_bucket_count: usize,
    product_hot_rejected_shadow_false_accepts: usize,
    product_hot_profile_cap: usize,
    product_hot_selected_by_budget_cap: bool,
    product_hot_package_bytes_estimate: usize,
    product_hot_shadow_registry_budget_passed: bool,
    product_hot_promotion_candidate_count: usize,
    product_hot_promotion_gate_passed_count: usize,
    product_hot_promotion_gate_failed_count: usize,
    product_hot_promotion_registry_path: String,
    product_hot_promotion_registry_written: bool,
    product_hot_promotion_registry_readback_exact: bool,
    product_hot_kind_contributions: Vec<OnlineMinerProductHotKindContributionReport>,
    product_hot_promotion_candidates: Vec<OnlineMinerPromotionCandidateReport>,
    auto_calibrated_all_accepted_buckets_false_accepts_zero: bool,
    auto_calibrated_accepted_bucket_count: usize,
    auto_calibrated_rejected_bucket_count: usize,
    auto_calibrated_rejected_false_accepts: usize,
    runtime_margin_parity_mismatches: usize,
    hot_runtime_margin_parity_checks: usize,
    hot_runtime_margin_parity_mismatches: usize,
    hot_runtime_decision_parity_mismatches: usize,
    hot_runtime_latency_p50_ns: u128,
    hot_runtime_latency_p90_ns: u128,
    hot_runtime_latency_p99_ns: u128,
    false_accepts: usize,
    wrong_wins: usize,
    min_margin_micro: i64,
    p10_margin_micro: i64,
    median_margin_micro: i64,
    latency_p50_ns: u128,
    latency_p90_ns: u128,
    latency_p99_ns: u128,
    memory_budget: OnlineMinerRuntimeBudgetReport,
    checkpoints: Vec<OnlineMinerCheckpointReport>,
    buckets: Vec<OnlineMinerBucketReport>,
    stream_contract: OnlineMinerStreamContract,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: serde_json::Value,
    verdict: &'static str,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct OnlineMinerPromotionCandidateReport {
    bucket_key: String,
    bucket_kind: &'static str,
    action_family_atom: String,
    task_name: String,
    package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    package_records: usize,
    package_file_exists: bool,
    package_read_bytes: usize,
    package_reload_verified: bool,
    package_fingerprint_verified: bool,
    package_records_verified: bool,
    promotion_gate_passed: bool,
    safe_accept_margin_threshold_micro: i64,
    auto_calibrated_margin_threshold_micro: i64,
    auto_calibration_events: usize,
    shadow_events_after_calibration: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    false_accepts: usize,
    verifier_bound: bool,
    quarantine_nwpc: bool,
    shadow_only: bool,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    promotion_status: &'static str,
}

#[derive(Clone, Debug, Default, Serialize)]
struct OnlineMinerProductHotKindContributionReport {
    bucket_kind: &'static str,
    selected_profile_count: usize,
    marginal_unique_cpu_accepts_over_exact_cache: usize,
    marginal_nando_cpu_tokens_saved: usize,
    marginal_nando_cpu_cost_saved_microusd: u64,
    marginal_false_accepts: usize,
}

#[derive(Clone, Debug, Serialize)]
struct OnlineMinerValuePassReport {
    report_kind: &'static str,
    mode: &'static str,
    trace_paths: Vec<String>,
    model_price_config_path: String,
    total_rows: usize,
    parsed_events: usize,
    source_verifier_labeled_events: usize,
    exact_cache_hits: usize,
    non_exact_rows: usize,
    total_tokens_seen: usize,
    total_cost_microusd_seen: u64,
    estimated_total_cost_microusd_seen: u64,
    token_denominator_present: bool,
    cost_denominator_present: bool,
    estimated_cost_denominator_present: bool,
    token_cost_denominator_present: bool,
    skipped_no_action_family: usize,
    skipped_no_verifier_label: usize,
    skipped_no_phase_atoms: usize,
    bucket_count: usize,
    broad_action_bucket_count: usize,
    state_action_bucket_count: usize,
    auto_subcenter_bucket_count: usize,
    learned_subcenter_bucket_count: usize,
    hidden_state_bucket_count: usize,
    hidden_state_forbidden_source_leak_bucket_count: usize,
    learned_split_registry_action_count: usize,
    learned_split_atom_count: usize,
    learned_split_compound_atom_count: usize,
    learned_split_conflict_gate_enabled: bool,
    learned_split_conflict_action_count: usize,
    learned_split_atoms_blocked_without_conflict: usize,
    product_hot_candidate_upper_bound_profile_count: usize,
    product_hot_candidate_upper_bound_unique_accepts_over_exact_cache: usize,
    product_hot_candidate_upper_bound_tokens_saved: usize,
    product_hot_candidate_upper_bound_cost_saved_microusd: u64,
    product_hot_candidate_upper_bound_estimated_cost_saved_microusd: u64,
    product_hot_candidate_upper_bound_duplicate_accept_rows: usize,
    product_hot_candidate_upper_bound_calls_saved_milli_over_total_rows: usize,
    product_hot_candidate_upper_bound_calls_saved_milli_over_labeled_events: usize,
    product_hot_candidate_upper_bound_calls_saved_milli_over_non_exact_rows: usize,
    product_hot_candidate_upper_bound_tokens_saved_milli_over_total_tokens: usize,
    product_hot_candidate_upper_bound_cost_saved_milli_over_total_cost: usize,
    product_hot_candidate_upper_bound_estimated_cost_saved_milli_over_estimated_total_cost: usize,
    product_hot_kind_contributions: Vec<OnlineMinerProductHotKindContributionReport>,
    selected_product_hot_candidates: Vec<OnlineMinerValuePassCandidateReport>,
    top_candidates: Vec<OnlineMinerValuePassCandidateReport>,
    compile_required_for_runtime_proof: bool,
    runtime_false_accepts_measured: bool,
    local_accept_enabled: bool,
    market_money_claim_allowed: bool,
    market_money_claim_blocker: &'static str,
    estimated_money_claim_allowed: bool,
    estimated_money_claim_blocker: &'static str,
    estimated_cost_method: &'static str,
    price_config_schema_version: String,
    price_config_source: String,
    forbidden_flags: serde_json::Value,
    verdict: &'static str,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct OnlineMinerTargetedShadowReport {
    report_kind: &'static str,
    mode: &'static str,
    trace_paths: Vec<String>,
    checkpoint_dir: String,
    decision_log_path: String,
    cells: usize,
    requested_top_k: usize,
    train_permille: usize,
    total_rows: usize,
    parsed_events: usize,
    selected_candidate_count: usize,
    selected_candidates: Vec<OnlineMinerValuePassCandidateReport>,
    compiled_candidate_count: usize,
    skipped_selected_candidate_count: usize,
    future_shadow_events: usize,
    promotion_candidate_count: usize,
    promotion_gate_passed_count: usize,
    promotion_gate_failed_count: usize,
    targeted_clean_unique_cpu_accepts_over_exact_cache: usize,
    targeted_clean_nando_cpu_tokens_saved: usize,
    targeted_clean_nando_cpu_cost_saved_microusd: u64,
    targeted_clean_duplicate_accept_rows: usize,
    targeted_clean_false_accepts: usize,
    product_hot_candidate_profile_count: usize,
    product_hot_unique_cpu_accepts_over_exact_cache: usize,
    product_hot_nando_cpu_tokens_saved: usize,
    product_hot_nando_cpu_cost_saved_microusd: u64,
    product_hot_duplicate_accept_rows: usize,
    product_hot_false_accepts: usize,
    product_hot_kind_contributions: Vec<OnlineMinerProductHotKindContributionReport>,
    runtime_margin_parity_mismatches: usize,
    hot_runtime_margin_parity_checks: usize,
    hot_runtime_margin_parity_mismatches: usize,
    hot_runtime_decision_parity_mismatches: usize,
    hot_runtime_latency_p50_ns: u128,
    hot_runtime_latency_p90_ns: u128,
    hot_runtime_latency_p99_ns: u128,
    raw_shadow_false_accepts: usize,
    rejected_shadow_bucket_count: usize,
    rejected_shadow_false_accepts: usize,
    false_accepts: usize,
    latency_p50_ns: u128,
    latency_p90_ns: u128,
    latency_p99_ns: u128,
    packages: Vec<OnlineMinerPromotionCandidateReport>,
    product_hot_packages: Vec<OnlineMinerPromotionCandidateReport>,
    runtime_false_accepts_measured: bool,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: serde_json::Value,
    verdict: &'static str,
    boundary: &'static str,
}

#[derive(Clone, Debug, Default)]
struct OnlineMinerTargetedDecisionDrilldownStats {
    future_rows: usize,
    raw_shadow_accepts: usize,
    raw_unique_accepts_over_exact_cache: usize,
    raw_tokens_saved: usize,
    raw_false_accepts: usize,
    exact_cache_hits: usize,
    reference_runtime_parity_mismatches: usize,
    max_false_margin_micro: Option<i64>,
    min_margin_micro: Option<i64>,
    max_margin_micro: Option<i64>,
    threshold_micro: Option<i64>,
}

#[derive(Debug)]
struct OnlineMinerValuePassCollection {
    buckets: BTreeMap<String, OnlineMinerValuePassBucketState>,
    split_atom_stats_by_action: BTreeMap<String, BTreeMap<String, OnlineMinerSplitAtomStats>>,
    total_rows: usize,
    parsed_events: usize,
    source_verifier_labeled_events: usize,
    exact_cache_hits: usize,
    non_exact_rows: usize,
    total_tokens_seen: usize,
    total_cost_microusd_seen: u64,
    skipped_no_action_family: usize,
    skipped_no_verifier_label: usize,
    skipped_no_phase_atoms: usize,
    learned_split_atoms_blocked_without_conflict: usize,
}

#[derive(Clone, Debug, Serialize)]
struct OnlineMinerValuePassCandidateReport {
    bucket_key: String,
    bucket_kind: &'static str,
    action_family_atom: String,
    events_seen: usize,
    positive_events: usize,
    negative_events: usize,
    non_exact_positive_events: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
    value_score: u128,
    selected_for_product_hot_candidate: bool,
}

#[derive(Clone, Debug, Serialize)]
struct OnlineMinerPromotionRegistryReport {
    registry_kind: &'static str,
    mode: &'static str,
    source_report_path: String,
    trace_paths: Vec<String>,
    checkpoint_dir: String,
    cells: usize,
    total_rows: usize,
    promotion_candidate_count: usize,
    promotion_gate_passed_count: usize,
    promotion_gate_failed_count: usize,
    global_unique_cpu_accepts_over_exact_cache: usize,
    duplicate_accept_rows: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    false_accepts: usize,
    rejected_shadow_bucket_count: usize,
    rejected_shadow_false_accepts: usize,
    profile_cap: usize,
    selected_by_budget_cap: bool,
    package_bytes_estimate: usize,
    shadow_registry_budget_passed: bool,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    market_money_claim_allowed: bool,
    promotion_status: &'static str,
    forbidden_flags: serde_json::Value,
    candidates: Vec<OnlineMinerPromotionCandidateReport>,
}

#[derive(Clone, Debug, Serialize)]
struct OnlineMinerPromotionRegistryGateReport {
    report_kind: &'static str,
    mode: &'static str,
    source_registry_path: String,
    shadow_registry_dir: String,
    source_registry_kind: String,
    source_mode: String,
    input_candidate_count: usize,
    promoted_candidate_count: usize,
    blocked_candidate_count: usize,
    promotion_gate_passed_count: usize,
    promotion_gate_failed_count: usize,
    global_unique_cpu_accepts_over_exact_cache: usize,
    duplicate_accept_rows: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    false_accepts: usize,
    registry_global_gate_clear: bool,
    shadow_registry_mutated: bool,
    promoted_packages: Vec<OnlineMinerPromotionRegistryGatePackageReport>,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: serde_json::Value,
    verdict: &'static str,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct OnlineMinerPromotionRegistryGatePackageReport {
    bucket_key: String,
    task_name: String,
    source_package_path: String,
    registry_package_path: String,
    source_package_fingerprint64: u64,
    inspected_package_fingerprint64: u64,
    source_package_bytes: usize,
    inspected_package_bytes: usize,
    source_package_records: usize,
    inspected_package_records: usize,
    package_file_exists: bool,
    package_readback_exact: bool,
    package_reload_verified: bool,
    package_fingerprint_verified: bool,
    package_records_verified: bool,
    source_promotion_gate_passed: bool,
    false_accepts: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    accepted_for_shadow_registry: bool,
    blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct OnlineMinerStreamContract {
    append_only_input: bool,
    score_before_train: bool,
    future_only_shadow_scoring: bool,
    incremental_bucket_updates: bool,
    bucket_readiness_quarantine_compile: bool,
    positive_negative_reservoirs: bool,
    periodic_quarantine_nwpc_compile: bool,
    compatible_denominator_delta_in_same_pass: bool,
}

#[derive(Clone, Debug, Serialize)]
struct OnlineMinerCheckpointReport {
    bucket_key: String,
    bucket_kind: &'static str,
    action_family_atom: String,
    task_name: String,
    compiled_after_row: usize,
    events_seen_at_compile: usize,
    positive_events_at_compile: usize,
    negative_events_at_compile: usize,
    safe_accept_margin_threshold_micro: i64,
    train_safe_accept_max_false_margin_micro: Option<i64>,
    train_safe_accept_min_true_margin_micro: Option<i64>,
    package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    package_records: usize,
    active_runtime_bytes_estimate: usize,
    package_readback_exact: bool,
    reservoir_positive_events: usize,
    reservoir_negative_events: usize,
}

#[derive(Clone, Debug, Serialize)]
struct OnlineMinerBucketReport {
    bucket_key: String,
    bucket_kind: &'static str,
    action_family_atom: String,
    task_name: String,
    events_seen: usize,
    positive_events: usize,
    negative_events: usize,
    exact_cache_hits: usize,
    non_exact_positive_events: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
    value_score: u128,
    active_checkpoint: bool,
    active_runtime_bytes_estimate: usize,
    reservoir_events: usize,
    checkpoints_compiled: usize,
    last_compiled_after_row: usize,
    safe_accept_margin_threshold_micro: i64,
    future_shadow_events: usize,
    local_operator_shadow_decisions: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    auto_calibration_events: usize,
    auto_calibrated_shadow_events_after_calibration: usize,
    auto_calibrated_margin_threshold_micro: i64,
    auto_calibrated_local_operator_shadow_decisions: usize,
    auto_calibrated_unique_cpu_accepts_over_exact_cache: usize,
    auto_calibrated_nando_cpu_tokens_saved: usize,
    auto_calibrated_nando_cpu_cost_saved_microusd: u64,
    auto_calibrated_false_accepts: usize,
    auto_calibrated_max_false_margin_micro: Option<i64>,
    auto_calibrated_bucket_accepted: bool,
    auto_calibrated_bucket_rejected: bool,
    product_hot_candidate: bool,
    product_hot_rejected_by_auto_calibration: bool,
    runtime_margin_parity_mismatches: usize,
    false_accepts: usize,
    wrong_wins: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
}

#[derive(Clone, Debug, Serialize)]
struct OnlineMinerRuntimeBudgetReport {
    snapshot_kind: &'static str,
    max_hot_profiles_per_worker: usize,
    max_hot_bytes_per_worker: usize,
    max_warm_profiles_per_process: usize,
    max_profiles_per_route: usize,
    max_route_top_k: usize,
    warm_route_count: usize,
    warm_profile_count: usize,
    warm_metadata_bytes_estimate: usize,
    warm_runtime_bytes_estimate: usize,
    warm_bytes_estimate: usize,
    hot_route_count: usize,
    hot_profile_count: usize,
    hot_route_profile_edges: usize,
    hot_runtime_bytes_estimate: usize,
    hot_route_table_bytes_estimate: usize,
    hot_bytes_estimate: usize,
    reservoir_events: usize,
    reservoir_bytes_estimate: usize,
    checkpoint_package_bytes_estimate: usize,
    decision_buffer_events: usize,
    warm_profile_budget_passed: bool,
    hot_profile_budget_passed: bool,
    hot_byte_budget_passed: bool,
    warm_budget_passed: bool,
    hot_budget_passed: bool,
    product_runtime_budget_passed: bool,
    product_residency_claim_allowed: bool,
}

#[derive(Clone)]
struct OnlineMinerBucketState {
    bucket_key: String,
    bucket_kind: &'static str,
    action_family_atom: String,
    task_name: String,
    compiler: PhaseCenterCompiler,
    positive_reservoir: Vec<PhaseAtomBinaryEvent>,
    negative_reservoir: Vec<PhaseAtomBinaryEvent>,
    events_seen: usize,
    positive_events: usize,
    negative_events: usize,
    exact_cache_hits: usize,
    non_exact_positive_events: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
    active_runtime: Option<PhaseCenterOffloadRuntime>,
    active_reference_runtime: Option<PhaseCenterFlatRuntime>,
    active_hot_runtime: Option<PhaseCenterHotRuntime>,
    future_decisions: Vec<OnlineMinerFutureDecisionSample>,
    active_package_path: String,
    active_package_fingerprint64: u64,
    package_bytes: usize,
    package_records: usize,
    safe_accept_margin_threshold_micro: i64,
    train_safe_accept_max_false_margin_micro: Option<i64>,
    train_safe_accept_min_true_margin_micro: Option<i64>,
    checkpoints_compiled: usize,
    last_compiled_after_row: usize,
    future_shadow_events: usize,
    local_operator_shadow_decisions: usize,
    fallback_shadow_decisions: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    runtime_margin_parity_mismatches: usize,
    hot_runtime_margin_parity_checks: usize,
    hot_runtime_margin_parity_mismatches: usize,
    hot_runtime_decision_parity_mismatches: usize,
    false_accepts: usize,
    wrong_wins: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
}

#[derive(Clone)]
struct OnlineMinerFutureDecisionSample {
    request_fingerprint: String,
    margin_micro: i64,
    verified_safe_accept: bool,
    exact_cache_hit: bool,
    total_tokens: usize,
    total_cost_microusd: u64,
}

#[derive(Clone, Debug)]
struct OnlineMinerValuePassBucketState {
    bucket_key: String,
    bucket_kind: &'static str,
    action_family_atom: String,
    events_seen: usize,
    positive_events: usize,
    negative_events: usize,
    exact_cache_hits: usize,
    non_exact_positive_events: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
    positive_non_exact_samples: Vec<OnlineMinerValuePassAcceptSample>,
}

impl OnlineMinerValuePassBucketState {
    fn new(bucket_key: String, bucket_kind: &'static str, action_family_atom: String) -> Self {
        Self {
            bucket_key,
            bucket_kind,
            action_family_atom,
            events_seen: 0,
            positive_events: 0,
            negative_events: 0,
            exact_cache_hits: 0,
            non_exact_positive_events: 0,
            total_tokens: 0,
            total_cost_microusd: 0,
            positive_non_exact_samples: Vec::new(),
        }
    }

    fn observe(
        &mut self,
        request_fingerprint: String,
        verified_safe_accept: bool,
        exact_cache_hit: bool,
        token_cost: GenericTokenCost,
    ) {
        self.events_seen = self.events_seen.saturating_add(1);
        self.exact_cache_hits = self
            .exact_cache_hits
            .saturating_add(usize::from(exact_cache_hit));
        self.total_tokens = self.total_tokens.saturating_add(token_cost.total_tokens);
        self.total_cost_microusd = self
            .total_cost_microusd
            .saturating_add(token_cost.total_cost_microusd);
        if verified_safe_accept {
            self.positive_events = self.positive_events.saturating_add(1);
            if !exact_cache_hit {
                self.non_exact_positive_events = self.non_exact_positive_events.saturating_add(1);
                self.positive_non_exact_samples
                    .push(OnlineMinerValuePassAcceptSample {
                        request_fingerprint,
                        total_tokens: token_cost.total_tokens,
                        total_cost_microusd: token_cost.total_cost_microusd,
                    });
            }
        } else {
            self.negative_events = self.negative_events.saturating_add(1);
        }
    }

    fn value_score(&self) -> u128 {
        if !self.eligible_for_candidate() {
            return 0;
        }
        (self.non_exact_positive_events as u128)
            .saturating_mul(self.total_tokens as u128)
            .saturating_mul(self.events_seen as u128)
            .saturating_add(self.positive_events as u128)
    }

    fn eligible_for_candidate(&self) -> bool {
        self.positive_events > 0
            && self.negative_events > 0
            && self.non_exact_positive_events > 0
            && online_miner_value_pass_bucket_is_candidate_kind(self.bucket_kind)
    }
}

#[derive(Clone, Debug)]
struct OnlineMinerValuePassAcceptSample {
    request_fingerprint: String,
    total_tokens: usize,
    total_cost_microusd: u64,
}

#[derive(Clone, Debug, Default)]
struct OnlineMinerSplitAtomStats {
    positive_events: usize,
    negative_events: usize,
    non_exact_positive_events: usize,
    total_tokens: usize,
}

impl OnlineMinerSplitAtomStats {
    fn observe(
        &mut self,
        verified_safe_accept: bool,
        exact_cache_hit: bool,
        token_cost: GenericTokenCost,
    ) {
        if verified_safe_accept {
            self.positive_events += 1;
            if !exact_cache_hit {
                self.non_exact_positive_events += 1;
            }
        } else {
            self.negative_events += 1;
        }
        self.total_tokens = self.total_tokens.saturating_add(token_cost.total_tokens);
    }

    fn eligible_for_split(&self) -> bool {
        self.positive_events > 0 && self.negative_events > 0 && self.non_exact_positive_events > 0
    }

    fn value_score(&self) -> u128 {
        if !self.eligible_for_split() {
            return 0;
        }
        (self.non_exact_positive_events as u128)
            .saturating_mul(self.total_tokens as u128)
            .saturating_mul((self.positive_events + self.negative_events) as u128)
            .saturating_add(self.positive_events as u128)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct OnlineMinerLearnedSplitAtomCounts {
    single: usize,
    multi2: usize,
    multi3: usize,
    multi4: usize,
}

impl OnlineMinerLearnedSplitAtomCounts {
    fn observe(&mut self, atom: &str) {
        if atom.starts_with("multi2:") {
            self.multi2 += 1;
        } else if atom.starts_with("multi3:") {
            self.multi3 += 1;
        } else if atom.starts_with("multi4:") {
            self.multi4 += 1;
        } else {
            self.single += 1;
        }
    }

    fn compound(self) -> usize {
        self.multi2
            .saturating_add(self.multi3)
            .saturating_add(self.multi4)
    }

    fn total(self) -> usize {
        self.single.saturating_add(self.compound())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct OnlineMinerHotSlotDecision {
    allowed: bool,
    replaced_existing_hot_bucket: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct OnlineMinerHotAuditionRank {
    weighted_value_score: u128,
    value_score: u128,
    kind_priority: u8,
    non_exact_positive_events: usize,
    events_seen: usize,
}

#[derive(Clone, Copy, Default)]
struct OnlineMinerAutoCalibratedStats {
    threshold_micro: i64,
    calibration_events: usize,
    shadow_events_after_calibration: usize,
    local_operator_shadow_decisions: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    false_accepts: usize,
    max_false_margin_micro: Option<i64>,
    accepted_bucket_count: usize,
    rejected_bucket_count: usize,
    rejected_false_accepts: usize,
}

#[derive(Clone, Copy, Default)]
struct OnlineMinerGlobalAutoCalibratedStats {
    unique_cpu_accepts_over_exact_cache: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    duplicate_accept_rows: usize,
    false_accepts: usize,
}

#[derive(Clone, Copy, Default)]
struct OnlineMinerPromotionPackageEvidence {
    file_exists: bool,
    read_bytes: usize,
    reload_verified: bool,
    fingerprint_verified: bool,
    records_verified: bool,
}

#[derive(Clone, Copy)]
struct OnlineMinerCompileConfig<'a> {
    checkpoint_dir: &'a Path,
    cells: usize,
    min_bucket_events: usize,
    base_margin_threshold_micro: i64,
    compiled_after_row: usize,
    max_active_buckets: usize,
}

impl OnlineMinerBucketState {
    fn new(
        bucket_key: String,
        bucket_kind: &'static str,
        action_family_atom: String,
        cells: usize,
    ) -> Result<Self, String> {
        let task_name = phase_atom_live_self_mining_task_name(&bucket_key);
        Ok(Self {
            bucket_key,
            bucket_kind,
            action_family_atom,
            task_name,
            compiler: PhaseCenterCompiler::new(cells, 1)
                .map_err(|error| format!("online miner compiler init error: {error:?}"))?,
            positive_reservoir: Vec::new(),
            negative_reservoir: Vec::new(),
            events_seen: 0,
            positive_events: 0,
            negative_events: 0,
            exact_cache_hits: 0,
            non_exact_positive_events: 0,
            total_tokens: 0,
            total_cost_microusd: 0,
            active_runtime: None,
            active_reference_runtime: None,
            active_hot_runtime: None,
            future_decisions: Vec::new(),
            active_package_path: String::new(),
            active_package_fingerprint64: 0,
            package_bytes: 0,
            package_records: 0,
            safe_accept_margin_threshold_micro: DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO,
            train_safe_accept_max_false_margin_micro: None,
            train_safe_accept_min_true_margin_micro: None,
            checkpoints_compiled: 0,
            last_compiled_after_row: 0,
            future_shadow_events: 0,
            local_operator_shadow_decisions: 0,
            fallback_shadow_decisions: 0,
            unique_cpu_accepts_over_exact_cache: 0,
            runtime_margin_parity_mismatches: 0,
            hot_runtime_margin_parity_checks: 0,
            hot_runtime_margin_parity_mismatches: 0,
            hot_runtime_decision_parity_mismatches: 0,
            false_accepts: 0,
            wrong_wins: 0,
            nando_cpu_tokens_saved: 0,
            nando_cpu_cost_saved_microusd: 0,
        })
    }

    fn value_score(&self) -> u128 {
        if self.positive_events == 0 || self.negative_events == 0 {
            return 0;
        }
        (self.non_exact_positive_events as u128)
            .saturating_mul(self.total_tokens as u128)
            .saturating_mul(self.events_seen as u128)
            .saturating_add(self.positive_events as u128)
    }

    fn active_runtime_bytes_estimate(&self) -> usize {
        self.active_runtime
            .as_ref()
            .map_or(0, PhaseCenterOffloadRuntime::bytes_estimate)
    }

    fn reservoir_event_count(&self) -> usize {
        self.positive_reservoir
            .len()
            .saturating_add(self.negative_reservoir.len())
    }
}

fn online_miner_runtime_budget_report(
    buckets: &BTreeMap<String, OnlineMinerBucketState>,
    checkpoints: &[OnlineMinerCheckpointReport],
) -> OnlineMinerRuntimeBudgetReport {
    let warm_route_count = buckets.len();
    let warm_profile_count = buckets.len();
    let hot_profile_count = buckets
        .values()
        .filter(|bucket| bucket.active_runtime.is_some())
        .count();
    let hot_runtime_bytes_estimate = buckets
        .values()
        .map(OnlineMinerBucketState::active_runtime_bytes_estimate)
        .sum::<usize>();
    let reservoir_events = buckets
        .values()
        .map(OnlineMinerBucketState::reservoir_event_count)
        .sum::<usize>();
    let reservoir_bytes_estimate =
        reservoir_events.saturating_mul(std::mem::size_of::<PhaseAtomBinaryEvent>());
    let decision_buffer_events = buckets
        .values()
        .map(|bucket| bucket.future_decisions.len())
        .sum::<usize>();
    let warm_metadata_bytes_estimate = buckets
        .values()
        .map(online_miner_bucket_metadata_bytes_estimate)
        .sum::<usize>()
        .saturating_add(reservoir_bytes_estimate)
        .saturating_add(
            decision_buffer_events
                .saturating_mul(std::mem::size_of::<OnlineMinerFutureDecisionSample>()),
        );
    let checkpoint_package_bytes_estimate = checkpoints
        .iter()
        .map(|checkpoint| checkpoint.package_bytes)
        .sum::<usize>();
    let snapshot = PhaseCenterRuntimeBudgetSnapshot {
        max_hot_profiles_per_worker: ONLINE_MINER_PRODUCT_MAX_HOT_PROFILES_PER_WORKER,
        max_hot_bytes_per_worker: ONLINE_MINER_PRODUCT_MAX_HOT_BYTES_PER_WORKER,
        max_warm_profiles_per_process: ONLINE_MINER_PRODUCT_MAX_WARM_PROFILES_PER_PROCESS,
        max_profiles_per_route: ONLINE_MINER_PRODUCT_MAX_PROFILES_PER_ROUTE,
        max_route_top_k: ONLINE_MINER_PRODUCT_MAX_ROUTE_TOP_K,
        warm_route_count,
        warm_profile_count,
        warm_metadata_bytes_estimate,
        warm_runtime_bytes_estimate: 0,
        warm_bytes_estimate: warm_metadata_bytes_estimate,
        hot_route_count: hot_profile_count,
        hot_profile_count,
        hot_route_profile_edges: hot_profile_count,
        hot_runtime_bytes_estimate,
        hot_route_table_bytes_estimate: 0,
        hot_bytes_estimate: hot_runtime_bytes_estimate,
        warm_profile_budget_passed: warm_profile_count
            <= ONLINE_MINER_PRODUCT_MAX_WARM_PROFILES_PER_PROCESS,
        hot_profile_budget_passed: hot_profile_count
            <= ONLINE_MINER_PRODUCT_MAX_HOT_PROFILES_PER_WORKER,
        hot_byte_budget_passed: hot_runtime_bytes_estimate
            <= ONLINE_MINER_PRODUCT_MAX_HOT_BYTES_PER_WORKER,
    };
    OnlineMinerRuntimeBudgetReport {
        snapshot_kind: "online_miner_daemon_mutable_bucket_memory",
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
        reservoir_events,
        reservoir_bytes_estimate,
        checkpoint_package_bytes_estimate,
        decision_buffer_events,
        warm_profile_budget_passed: snapshot.warm_profile_budget_passed,
        hot_profile_budget_passed: snapshot.hot_profile_budget_passed,
        hot_byte_budget_passed: snapshot.hot_byte_budget_passed,
        warm_budget_passed: snapshot.warm_budget_passed(),
        hot_budget_passed: snapshot.hot_budget_passed(),
        product_runtime_budget_passed: snapshot.product_runtime_budget_passed(),
        product_residency_claim_allowed: false,
    }
}

fn online_miner_bucket_metadata_bytes_estimate(bucket: &OnlineMinerBucketState) -> usize {
    std::mem::size_of::<OnlineMinerBucketState>()
        .saturating_add(bucket.bucket_key.len())
        .saturating_add(bucket.action_family_atom.len())
        .saturating_add(bucket.task_name.len())
        .saturating_add(bucket.active_package_path.len())
}

fn online_miner_control_budget_from_env() -> Result<OnlineMinerControlBudget, String> {
    let mode = match std::env::var(ONLINE_MINER_MODE_ENV) {
        Ok(value) => match value.trim().to_ascii_uppercase().as_str() {
            "" | "BURST_DISCOVERY" | "BURST" => OnlineMinerControlMode::BurstDiscovery,
            "OFF" => OnlineMinerControlMode::Off,
            "SHADOW_LIGHT" | "LIGHT" => OnlineMinerControlMode::ShadowLight,
            "PRODUCT_HOT_SCORE_ONLY" | "SCORE_ONLY" => OnlineMinerControlMode::ProductHotScoreOnly,
            other => {
                return Err(format!(
                    "invalid {ONLINE_MINER_MODE_ENV}='{other}', expected OFF/SHADOW_LIGHT/BURST_DISCOVERY/PRODUCT_HOT_SCORE_ONLY"
                ));
            }
        },
        Err(std::env::VarError::NotPresent) => OnlineMinerControlMode::BurstDiscovery,
        Err(error) => return Err(format!("failed to read {ONLINE_MINER_MODE_ENV}: {error}")),
    };
    let default_row_budget = match mode {
        OnlineMinerControlMode::Off | OnlineMinerControlMode::ProductHotScoreOnly => Some(0),
        OnlineMinerControlMode::ShadowLight | OnlineMinerControlMode::BurstDiscovery => None,
    };
    let default_compile_budget = match mode {
        OnlineMinerControlMode::Off | OnlineMinerControlMode::ProductHotScoreOnly => Some(0),
        OnlineMinerControlMode::ShadowLight => Some(1),
        OnlineMinerControlMode::BurstDiscovery => None,
    };
    let default_decision_log_limit = match mode {
        OnlineMinerControlMode::Off | OnlineMinerControlMode::ProductHotScoreOnly => Some(0),
        OnlineMinerControlMode::ShadowLight => Some(1000),
        OnlineMinerControlMode::BurstDiscovery => None,
    };
    let default_sample_every = match mode {
        OnlineMinerControlMode::ShadowLight => 10,
        OnlineMinerControlMode::Off
        | OnlineMinerControlMode::BurstDiscovery
        | OnlineMinerControlMode::ProductHotScoreOnly => 1,
    };
    let sample_every =
        online_miner_env_usize(ONLINE_MINER_SAMPLE_EVERY_ENV)?.unwrap_or(default_sample_every);
    if sample_every == 0 {
        return Err(format!("{ONLINE_MINER_SAMPLE_EVERY_ENV} must be > 0"));
    }
    let time_budget = online_miner_env_usize(ONLINE_MINER_TIME_BUDGET_MS_ENV)?
        .map(|value| Duration::from_millis(value as u64));
    Ok(OnlineMinerControlBudget {
        mode,
        row_budget: online_miner_env_usize(ONLINE_MINER_ROW_BUDGET_ENV)?.or(default_row_budget),
        time_budget,
        compile_budget: online_miner_env_usize(ONLINE_MINER_COMPILE_BUDGET_ENV)?
            .or(default_compile_budget),
        decision_log_limit: online_miner_env_usize(ONLINE_MINER_DECISION_LOG_LIMIT_ENV)?
            .or(default_decision_log_limit),
        sample_every,
    })
}

fn online_miner_env_usize(name: &str) -> Result<Option<usize>, String> {
    match std::env::var(name) {
        Ok(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                trimmed
                    .parse::<usize>()
                    .map(Some)
                    .map_err(|error| format!("invalid {name}='{trimmed}': {error}"))
            }
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("failed to read {name}: {error}")),
    }
}

fn online_miner_compile_budget_allows(compile_budget: Option<usize>, used: usize) -> bool {
    compile_budget.is_none_or(|budget| used < budget)
}

pub(crate) fn run_phase_stream_online_miner_daemon_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_DAEMON_REPORT));
    let checkpoint_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_DAEMON_CHECKPOINT_DIR));
    let decision_log_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_DAEMON_DECISION_LOG));
    let cells = parse_optional_usize(args.next(), "cells")?.unwrap_or(32);
    let min_bucket_events = parse_optional_usize(args.next(), "min_bucket_events")?.unwrap_or(20);
    let base_margin_threshold_micro =
        parse_optional_i64(args.next(), "base_margin_threshold_micro")?
            .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
    let compile_every_rows =
        parse_optional_usize(args.next(), "compile_every_rows")?.unwrap_or(2000);
    let requested_max_active_buckets =
        parse_optional_usize(args.next(), "max_active_buckets")?.unwrap_or(64);
    let max_active_buckets =
        requested_max_active_buckets.min(ONLINE_MINER_SHADOW_AUDITION_MAX_ACTIVE_BUCKETS);
    let reservoir_per_label = parse_optional_usize(args.next(), "reservoir_per_label")?
        .unwrap_or(DEFAULT_ONLINE_MINER_RESERVOIR_PER_LABEL);
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    if min_bucket_events == 0 {
        return Err("min_bucket_events must be > 0".to_owned());
    }
    if compile_every_rows == 0 {
        return Err("compile_every_rows must be > 0".to_owned());
    }
    if requested_max_active_buckets == 0 {
        return Err("max_active_buckets must be > 0".to_owned());
    }
    if reservoir_per_label == 0 {
        return Err("reservoir_per_label must be > 0".to_owned());
    }
    let control_budget = online_miner_control_budget_from_env()?;
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(
                DEFAULT_AUTO_SUBCENTER_DISCOVERY_CANDIDATES_JSONL,
            )]
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("online miner daemon needs at least one trace path".to_owned());
    }
    std::fs::create_dir_all(&checkpoint_dir).map_err(|error| {
        format!(
            "failed to create online miner checkpoint dir '{}': {error}",
            checkpoint_dir.display()
        )
    })?;
    if let Some(parent) = decision_log_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create online miner decision log dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let mut decision_log = std::fs::File::create(&decision_log_path).map_err(|error| {
        format!(
            "failed to create online miner decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;

    let mut buckets = BTreeMap::<String, OnlineMinerBucketState>::new();
    let mut split_atom_stats_by_action =
        BTreeMap::<String, BTreeMap<String, OnlineMinerSplitAtomStats>>::new();
    let mut checkpoints = Vec::<OnlineMinerCheckpointReport>::new();
    let mut seen_exact_cache_keys = BTreeSet::<String>::new();
    let mut total_rows = 0usize;
    let mut parsed_events = 0usize;
    let mut source_verifier_labeled_events = 0usize;
    let mut skipped_no_action_family = 0usize;
    let mut skipped_no_verifier_label = 0usize;
    let mut skipped_no_phase_atoms = 0usize;
    let mut broad_action_bucket_updates = 0usize;
    let mut state_action_bucket_updates = 0usize;
    let mut auto_subcenter_bucket_updates = 0usize;
    let mut learned_subcenter_bucket_updates = 0usize;
    let mut hidden_state_transition_count = 0usize;
    let mut learned_split_atoms_blocked_without_conflict = 0usize;
    let mut hot_audition_replacement_count = 0usize;
    let mut compile_ticks = 0usize;
    let mut compile_budget_used = 0usize;
    let mut compile_budget_exhausted = false;
    let mut decision_log_rows_written = 0usize;
    let mut decision_log_capped = false;
    let mut sampled_out_rows = 0usize;
    let mut budget_stop_reason = None::<&'static str>;
    let mut bucket_readiness_compile_count = 0usize;
    let mut margins = Vec::<i64>::new();
    let mut latencies = Vec::<u128>::new();
    let mut hot_runtime_latencies = Vec::<u128>::new();

    let daemon_started = Instant::now();
    'trace_paths: for trace_path in &trace_paths {
        let file = std::fs::File::open(trace_path).map_err(|error| {
            format!(
                "failed to open online miner trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in BufReader::new(file).lines().enumerate() {
            if control_budget
                .row_budget
                .is_some_and(|budget| total_rows >= budget)
            {
                budget_stop_reason = Some("row_budget_reached");
                break 'trace_paths;
            }
            if control_budget
                .time_budget
                .is_some_and(|budget| daemon_started.elapsed() >= budget)
            {
                budget_stop_reason = Some("time_budget_reached");
                break 'trace_paths;
            }
            let line = line.map_err(|error| {
                format!(
                    "failed to read online miner trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows += 1;
            if control_budget.sample_every > 1
                && !total_rows.is_multiple_of(control_budget.sample_every)
            {
                sampled_out_rows = sampled_out_rows.saturating_add(1);
                continue;
            }
            if total_rows == 1 || total_rows.is_multiple_of(1000) {
                println!(
                    "online_miner_daemon_progress: rows={} buckets={} checkpoints={} future_shadow_events={}",
                    total_rows,
                    buckets.len(),
                    checkpoints.len(),
                    buckets
                        .values()
                        .map(|bucket| bucket.future_shadow_events)
                        .sum::<usize>()
                );
            }
            if total_rows > 1 && total_rows.is_multiple_of(compile_every_rows) {
                if online_miner_compile_budget_allows(
                    control_budget.compile_budget,
                    compile_budget_used,
                ) {
                    compile_ticks += 1;
                    compile_budget_used = compile_budget_used.saturating_add(1);
                    let checkpoints_before = checkpoints.len();
                    let compiled_after_row = total_rows.saturating_sub(1);
                    println!(
                        "online_miner_daemon_compile_tick_start: rows={} compiled_after_row={} buckets={} checkpoints={} compile_budget_used={}",
                        total_rows,
                        compiled_after_row,
                        buckets.len(),
                        checkpoints_before,
                        compile_budget_used
                    );
                    compile_online_miner_checkpoints(
                        &mut buckets,
                        OnlineMinerCompileConfig {
                            checkpoint_dir: &checkpoint_dir,
                            cells,
                            min_bucket_events,
                            base_margin_threshold_micro,
                            compiled_after_row,
                            max_active_buckets,
                        },
                        &mut checkpoints,
                        &mut hot_audition_replacement_count,
                    )?;
                    println!(
                        "online_miner_daemon_compile_tick_done: rows={} new_checkpoints={} checkpoints={}",
                        total_rows,
                        checkpoints.len().saturating_sub(checkpoints_before),
                        checkpoints.len()
                    );
                } else {
                    compile_budget_exhausted = true;
                }
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse online miner trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            if row
                .get("verified_safe_accept")
                .and_then(serde_json::Value::as_bool)
                .is_none()
            {
                skipped_no_verifier_label += 1;
                continue;
            }
            source_verifier_labeled_events += 1;
            let action_atoms = phase_atom_string_vec(&row, "action_atoms");
            let action_families = phase_atom_action_families(&action_atoms);
            if action_families.is_empty() {
                skipped_no_action_family += 1;
                continue;
            }
            let request_atoms = phase_atom_string_vec(&row, "request_atoms");
            let state_atoms = phase_atom_string_vec(&row, "state_atoms");
            let tool_atoms = phase_atom_string_vec(&row, "tool_atoms");
            let route_hint_atoms = phase_atom_string_vec(&row, "route_hint_atoms");
            if request_atoms.is_empty()
                && state_atoms.is_empty()
                && tool_atoms.is_empty()
                && route_hint_atoms.is_empty()
            {
                skipped_no_phase_atoms += 1;
                continue;
            }
            let exact_cache_key = json_string(&row, &["exact_cache_key"])
                .or_else(|| json_string(&row, &["request_fingerprint"]))
                .unwrap_or_else(|| format!("online_miner_row:{total_rows}"));
            let exact_cache_hit = !seen_exact_cache_keys.insert(exact_cache_key);
            let token_cost = phase_atom_binary_token_cost(&row);
            for action_family in action_families {
                let source_event_index = parsed_events;
                let base_task_name = phase_atom_live_self_mining_task_name(&action_family);
                let Some(event) = parse_phase_atom_binary_event_for_action(
                    &row,
                    source_event_index,
                    &action_family,
                    &base_task_name,
                ) else {
                    skipped_no_phase_atoms += 1;
                    continue;
                };
                let event_split_atoms = online_miner_event_split_atoms(
                    &request_atoms,
                    &state_atoms,
                    &tool_atoms,
                    &route_hint_atoms,
                );
                let learned_split_atoms = online_miner_learned_split_atoms_for_action(
                    &split_atom_stats_by_action,
                    &action_family,
                    ONLINE_MINER_MAX_AUTOSUBCENTER_BUCKETS_PER_EVENT,
                    online_miner_action_family_has_learned_split_pressure(&buckets, &action_family),
                );
                if learned_split_atoms.is_empty()
                    && online_miner_action_family_has_candidate_split_atoms(
                        &split_atom_stats_by_action,
                        &action_family,
                    )
                    && !online_miner_action_family_has_learned_split_pressure(
                        &buckets,
                        &action_family,
                    )
                {
                    learned_split_atoms_blocked_without_conflict =
                        learned_split_atoms_blocked_without_conflict.saturating_add(1);
                }
                let bucket_specs = online_miner_event_bucket_specs(
                    &action_family,
                    &request_atoms,
                    &state_atoms,
                    &tool_atoms,
                    &route_hint_atoms,
                    &learned_split_atoms,
                );
                for (bucket_kind, bucket_key) in bucket_specs {
                    parsed_events += 1;
                    match bucket_kind {
                        "broad_action" => broad_action_bucket_updates += 1,
                        "state_action_signature" => state_action_bucket_updates += 1,
                        "auto_subcenter" => auto_subcenter_bucket_updates += 1,
                        "learned_auto_subcenter" => learned_subcenter_bucket_updates += 1,
                        "hidden_state_split" => hidden_state_transition_count += 1,
                        _ => {}
                    }
                    let active_profile_count = buckets
                        .values()
                        .filter(|bucket| bucket.active_runtime.is_some())
                        .count();
                    let bucket = buckets.entry(bucket_key.clone()).or_insert_with(|| {
                        OnlineMinerBucketState::new(
                            bucket_key.clone(),
                            bucket_kind,
                            action_family.clone(),
                            cells,
                        )
                        .expect("valid online miner bucket")
                    });
                    score_future_event_before_update(
                        bucket,
                        &event,
                        exact_cache_hit,
                        total_rows,
                        &mut decision_log,
                        &mut decision_log_rows_written,
                        control_budget.decision_log_limit,
                        &mut decision_log_capped,
                        &mut margins,
                        &mut latencies,
                        &mut hot_runtime_latencies,
                    )?;
                    update_online_bucket(
                        bucket,
                        event.clone(),
                        exact_cache_hit,
                        token_cost,
                        cells,
                        reservoir_per_label,
                    )?;
                    let bucket_ready_for_first_checkpoint = bucket.active_runtime.is_none()
                        && online_miner_bucket_immediate_checkpoint_allowed(bucket)
                        && online_miner_bucket_checkpoint_eligible(bucket, min_bucket_events);
                    if bucket_ready_for_first_checkpoint
                        && active_profile_count < max_active_buckets
                    {
                        if online_miner_compile_budget_allows(
                            control_budget.compile_budget,
                            compile_budget_used,
                        ) {
                            compile_budget_used = compile_budget_used.saturating_add(1);
                            bucket_readiness_compile_count += 1;
                            let checkpoints_before = checkpoints.len();
                            println!(
                                "online_miner_daemon_bucket_ready_compile_start: rows={} bucket_key={} checkpoints={} compile_budget_used={}",
                                total_rows, bucket_key, checkpoints_before, compile_budget_used
                            );
                            compile_online_miner_checkpoint_for_bucket(
                                bucket,
                                OnlineMinerCompileConfig {
                                    checkpoint_dir: &checkpoint_dir,
                                    cells,
                                    min_bucket_events,
                                    base_margin_threshold_micro,
                                    compiled_after_row: total_rows,
                                    max_active_buckets,
                                },
                                &mut checkpoints,
                            )?;
                            println!(
                                "online_miner_daemon_bucket_ready_compile_done: rows={} bucket_key={} new_checkpoints={} checkpoints={}",
                                total_rows,
                                bucket_key,
                                checkpoints.len().saturating_sub(checkpoints_before),
                                checkpoints.len()
                            );
                        } else {
                            compile_budget_exhausted = true;
                        }
                    }
                }
                observe_online_miner_split_atoms(
                    &mut split_atom_stats_by_action,
                    &action_family,
                    &event_split_atoms,
                    event.verified_safe_accept,
                    exact_cache_hit,
                    token_cost,
                );
            }
        }
    }
    if online_miner_compile_budget_allows(control_budget.compile_budget, compile_budget_used) {
        compile_ticks += 1;
        compile_budget_used = compile_budget_used.saturating_add(1);
        compile_online_miner_checkpoints(
            &mut buckets,
            OnlineMinerCompileConfig {
                checkpoint_dir: &checkpoint_dir,
                cells,
                min_bucket_events,
                base_margin_threshold_micro,
                compiled_after_row: total_rows,
                max_active_buckets,
            },
            &mut checkpoints,
            &mut hot_audition_replacement_count,
        )?;
    } else {
        compile_budget_exhausted = true;
    }
    decision_log.flush().map_err(|error| {
        format!(
            "failed to flush online miner decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;

    let mut bucket_reports = buckets
        .values()
        .map(online_miner_bucket_report)
        .collect::<Vec<_>>();
    bucket_reports.sort_by(|left, right| {
        right
            .unique_cpu_accepts_over_exact_cache
            .cmp(&left.unique_cpu_accepts_over_exact_cache)
            .then_with(|| right.value_score.cmp(&left.value_score))
            .then_with(|| left.bucket_key.cmp(&right.bucket_key))
    });
    bucket_reports.truncate(96);
    margins.sort_unstable();
    latencies.sort_unstable();
    hot_runtime_latencies.sort_unstable();
    let active_profile_count = buckets
        .values()
        .filter(|bucket| bucket.active_runtime.is_some())
        .count();
    let future_shadow_events = buckets
        .values()
        .map(|bucket| bucket.future_shadow_events)
        .sum::<usize>();
    let future_shadow_safe_events = buckets
        .values()
        .map(|bucket| bucket.local_operator_shadow_decisions + bucket.fallback_shadow_decisions)
        .sum::<usize>();
    let local_operator_shadow_decisions = buckets
        .values()
        .map(|bucket| bucket.local_operator_shadow_decisions)
        .sum::<usize>();
    let fallback_shadow_decisions = buckets
        .values()
        .map(|bucket| bucket.fallback_shadow_decisions)
        .sum::<usize>();
    let unique_cpu_accepts_over_exact_cache = buckets
        .values()
        .map(|bucket| bucket.unique_cpu_accepts_over_exact_cache)
        .sum::<usize>();
    let nando_cpu_tokens_saved = buckets
        .values()
        .map(|bucket| bucket.nando_cpu_tokens_saved)
        .sum::<usize>();
    let nando_cpu_cost_saved_microusd = buckets
        .values()
        .map(|bucket| bucket.nando_cpu_cost_saved_microusd)
        .sum::<u64>();
    let auto_calibrated = online_miner_auto_calibrated_total(buckets.values().map(|bucket| {
        online_miner_auto_calibrated_stats(bucket, bucket.safe_accept_margin_threshold_micro)
    }));
    let active_hot_auto_calibrated =
        online_miner_auto_calibrated_total(buckets.values().filter_map(|bucket| {
            bucket.active_runtime.as_ref()?;
            Some(online_miner_auto_calibrated_stats(
                bucket,
                bucket.safe_accept_margin_threshold_micro,
            ))
        }));
    let multi_split_auto_calibrated =
        online_miner_auto_calibrated_total(buckets.values().filter_map(|bucket| {
            if !online_miner_bucket_is_multi_split(bucket) {
                return None;
            }
            bucket.active_runtime.as_ref()?;
            Some(online_miner_auto_calibrated_stats(
                bucket,
                bucket.safe_accept_margin_threshold_micro,
            ))
        }));
    let multi_split_global_auto_calibrated =
        online_miner_multi_split_global_auto_calibrated_stats(&buckets);
    let multi_split_promotion_candidates = online_miner_multi_split_promotion_candidates(&buckets);
    let multi_split_promotion_candidate_count = multi_split_promotion_candidates.len();
    let multi_split_promotion_gate_passed_count = multi_split_promotion_candidates
        .iter()
        .filter(|candidate| candidate.promotion_gate_passed)
        .count();
    let multi_split_promotion_gate_failed_count = multi_split_promotion_candidate_count
        .saturating_sub(multi_split_promotion_gate_passed_count);
    let product_hot_promotion_candidates = online_miner_product_hot_budget_candidates_with_buckets(
        Some(&buckets),
        &multi_split_promotion_candidates,
    );
    let product_hot_kind_contributions =
        online_miner_product_hot_kind_contributions(&buckets, &product_hot_promotion_candidates);
    let product_hot_promotion_candidate_count = product_hot_promotion_candidates.len();
    let product_hot_candidate_profile_count = product_hot_promotion_candidate_count;
    let product_hot_global_auto_calibrated =
        online_miner_global_auto_calibrated_stats_for_candidates(
            &buckets,
            &product_hot_promotion_candidates,
        );
    let product_hot_unique_cpu_accepts_over_exact_cache =
        product_hot_global_auto_calibrated.unique_cpu_accepts_over_exact_cache;
    let product_hot_nando_cpu_tokens_saved =
        product_hot_global_auto_calibrated.nando_cpu_tokens_saved;
    let product_hot_nando_cpu_cost_saved_microusd =
        product_hot_global_auto_calibrated.nando_cpu_cost_saved_microusd;
    let product_hot_duplicate_accept_rows =
        product_hot_global_auto_calibrated.duplicate_accept_rows;
    let product_hot_false_accepts = product_hot_global_auto_calibrated.false_accepts;
    let product_hot_rejected_shadow_bucket_count =
        multi_split_auto_calibrated.rejected_bucket_count;
    let product_hot_rejected_shadow_false_accepts =
        multi_split_auto_calibrated.rejected_false_accepts;
    let product_hot_profile_cap = ONLINE_MINER_PRODUCT_MAX_HOT_PROFILES_PER_WORKER;
    let product_hot_selected_by_budget_cap =
        multi_split_promotion_candidate_count > product_hot_promotion_candidate_count;
    let product_hot_package_bytes_estimate = product_hot_promotion_candidates
        .iter()
        .map(|candidate| candidate.package_bytes)
        .sum::<usize>();
    let product_hot_shadow_registry_budget_passed = product_hot_promotion_candidate_count
        <= product_hot_profile_cap
        && product_hot_package_bytes_estimate <= ONLINE_MINER_PRODUCT_MAX_HOT_BYTES_PER_WORKER;
    let product_hot_promotion_gate_passed_count = product_hot_promotion_candidates
        .iter()
        .filter(|candidate| candidate.promotion_gate_passed)
        .count();
    let product_hot_promotion_gate_failed_count = product_hot_promotion_candidate_count
        .saturating_sub(product_hot_promotion_gate_passed_count);
    let global_auto_calibrated = online_miner_global_auto_calibrated_stats(&buckets);
    let active_hot_unique_cpu_accepts_over_exact_cache = buckets
        .values()
        .filter(|bucket| bucket.active_runtime.is_some())
        .map(|bucket| bucket.unique_cpu_accepts_over_exact_cache)
        .sum::<usize>();
    let active_hot_false_accepts = buckets
        .values()
        .filter(|bucket| bucket.active_runtime.is_some())
        .map(|bucket| bucket.false_accepts)
        .sum::<usize>();
    let false_accepts = buckets
        .values()
        .map(|bucket| bucket.false_accepts)
        .sum::<usize>();
    let runtime_margin_parity_mismatches = buckets
        .values()
        .map(|bucket| bucket.runtime_margin_parity_mismatches)
        .sum::<usize>();
    let hot_runtime_margin_parity_checks = buckets
        .values()
        .map(|bucket| bucket.hot_runtime_margin_parity_checks)
        .sum::<usize>();
    let hot_runtime_margin_parity_mismatches = buckets
        .values()
        .map(|bucket| bucket.hot_runtime_margin_parity_mismatches)
        .sum::<usize>();
    let hot_runtime_decision_parity_mismatches = buckets
        .values()
        .map(|bucket| bucket.hot_runtime_decision_parity_mismatches)
        .sum::<usize>();
    let wrong_wins = buckets
        .values()
        .map(|bucket| bucket.wrong_wins)
        .sum::<usize>();
    let memory_budget = online_miner_runtime_budget_report(&buckets, &checkpoints);
    let base_bucket_count = buckets
        .values()
        .filter(|bucket| bucket.bucket_kind == "broad_action")
        .count();
    let state_action_bucket_count = buckets
        .values()
        .filter(|bucket| bucket.bucket_kind == "state_action_signature")
        .count();
    let auto_subcenter_bucket_count = buckets
        .values()
        .filter(|bucket| bucket.bucket_kind == "auto_subcenter")
        .count();
    let auto_subcenter_multi2_bucket_count =
        online_miner_bucket_key_count(&buckets, "auto_subcenter", |key| {
            key.contains("::auto_subcenter:multi2:")
        });
    let auto_subcenter_multi3_bucket_count =
        online_miner_bucket_key_count(&buckets, "auto_subcenter", |key| {
            key.contains("::auto_subcenter:multi3:")
        });
    let auto_subcenter_multi4_bucket_count =
        online_miner_bucket_key_count(&buckets, "auto_subcenter", |key| {
            key.contains("::auto_subcenter:multi4:")
        });
    let auto_subcenter_forbidden_source_leak_bucket_count =
        online_miner_bucket_key_count(&buckets, "auto_subcenter", |key| {
            key.contains("_cwd_kind:")
                || key.contains("route_hint:")
                || key.contains("route_key:")
                || key.contains("request_route_family:")
                || key.contains("tool_mention:")
        });
    let learned_subcenter_bucket_count = buckets
        .values()
        .filter(|bucket| bucket.bucket_kind == "learned_auto_subcenter")
        .count();
    let learned_subcenter_multi2_bucket_count =
        online_miner_bucket_key_count(&buckets, "learned_auto_subcenter", |key| {
            key.contains("::learned_auto_subcenter:multi2:")
        });
    let learned_subcenter_multi3_bucket_count =
        online_miner_bucket_key_count(&buckets, "learned_auto_subcenter", |key| {
            key.contains("::learned_auto_subcenter:multi3:")
        });
    let learned_subcenter_multi4_bucket_count =
        online_miner_bucket_key_count(&buckets, "learned_auto_subcenter", |key| {
            key.contains("::learned_auto_subcenter:multi4:")
        });
    let learned_split_atom_counts =
        online_miner_learned_split_atom_counts(&split_atom_stats_by_action);
    let learned_split_atom_count = learned_split_atom_counts.total();
    let hidden_state_count = online_miner_hidden_state_atom_count(&split_atom_stats_by_action);
    let hidden_state_bucket_count = buckets
        .values()
        .filter(|bucket| bucket.bucket_kind == "hidden_state_split")
        .count();
    let hidden_state_forbidden_source_leak_bucket_count =
        online_miner_bucket_key_count(&buckets, "hidden_state_split", |key| {
            online_miner_hidden_state_bucket_has_forbidden_source_leak(key)
        });
    let learned_split_conflict_action_count = split_atom_stats_by_action
        .keys()
        .filter(|action_family| {
            online_miner_action_family_has_learned_split_pressure(&buckets, action_family)
        })
        .count();
    let product_hot_promotion_registry_path =
        checkpoint_dir.join("product-hot-promotion-registry.shadow.json");
    let product_hot_promotion_registry = OnlineMinerPromotionRegistryReport {
        registry_kind: "phase_stream_product_hot_promotion_registry_v1",
        mode: "shadow_quarantine_review_only",
        source_report_path: report_path.display().to_string(),
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        checkpoint_dir: checkpoint_dir.display().to_string(),
        cells,
        total_rows,
        promotion_candidate_count: product_hot_promotion_candidate_count,
        promotion_gate_passed_count: product_hot_promotion_gate_passed_count,
        promotion_gate_failed_count: product_hot_promotion_gate_failed_count,
        global_unique_cpu_accepts_over_exact_cache: product_hot_unique_cpu_accepts_over_exact_cache,
        duplicate_accept_rows: product_hot_duplicate_accept_rows,
        nando_cpu_tokens_saved: product_hot_nando_cpu_tokens_saved,
        nando_cpu_cost_saved_microusd: product_hot_nando_cpu_cost_saved_microusd,
        false_accepts: product_hot_false_accepts,
        rejected_shadow_bucket_count: product_hot_rejected_shadow_bucket_count,
        rejected_shadow_false_accepts: product_hot_rejected_shadow_false_accepts,
        profile_cap: product_hot_profile_cap,
        selected_by_budget_cap: product_hot_selected_by_budget_cap,
        package_bytes_estimate: product_hot_package_bytes_estimate,
        shadow_registry_budget_passed: product_hot_shadow_registry_budget_passed,
        local_accept_enabled: false,
        auto_promote_enabled: false,
        market_money_claim_allowed: false,
        promotion_status: "quarantine_review_only",
        forbidden_flags: serde_json::json!({
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false
        }),
        candidates: product_hot_promotion_candidates.clone(),
    };
    write_json_file(
        &product_hot_promotion_registry_path,
        &product_hot_promotion_registry,
    )?;
    let product_hot_promotion_registry_expected = format!(
        "{}\n",
        serde_json::to_string_pretty(&product_hot_promotion_registry)
            .map_err(|error| format!("failed to serialize promotion registry: {error}"))?
    );
    let product_hot_promotion_registry_readback_exact =
        std::fs::read_to_string(&product_hot_promotion_registry_path)
            .map(|readback| readback == product_hot_promotion_registry_expected)
            .unwrap_or(false);
    let product_hot_shadow_gate_clear = product_hot_false_accepts == 0
        && product_hot_unique_cpu_accepts_over_exact_cache > 0
        && product_hot_promotion_candidate_count > 0
        && product_hot_promotion_gate_failed_count == 0
        && product_hot_shadow_registry_budget_passed
        && product_hot_promotion_registry_readback_exact;
    let verdict = if checkpoints.is_empty() {
        "PHASE_STREAM_ONLINE_MINER_DAEMON_V1_WATCH_NO_CHECKPOINTS"
    } else if runtime_margin_parity_mismatches > 0 {
        "PHASE_STREAM_ONLINE_MINER_DAEMON_V1_WATCH_RUNTIME_PARITY"
    } else if product_hot_shadow_gate_clear {
        "PHASE_STREAM_ONLINE_MINER_DAEMON_V1_PASS_PRODUCT_HOT_COMPACT_SHADOW_REGISTRY_READY"
    } else if multi_split_global_auto_calibrated.false_accepts == 0
        && multi_split_global_auto_calibrated.unique_cpu_accepts_over_exact_cache > 0
        && multi_split_promotion_candidate_count > 0
    {
        "PHASE_STREAM_ONLINE_MINER_DAEMON_V1_WATCH_MULTI_SPLIT_SHADOW_READY_PRODUCT_HOT_BUDGET_NOT_PROVEN"
    } else if product_hot_false_accepts == 0
        && product_hot_unique_cpu_accepts_over_exact_cache > 0
        && product_hot_promotion_candidate_count > 0
    {
        "PHASE_STREAM_ONLINE_MINER_DAEMON_V1_WATCH_PRODUCT_HOT_PACKAGE_GATE"
    } else if auto_calibrated.false_accepts == 0
        && auto_calibrated.unique_cpu_accepts_over_exact_cache > 0
    {
        "PHASE_STREAM_ONLINE_MINER_DAEMON_V1_WATCH_HISTORICAL_ACCEPTS_NOT_CURRENT_HOT"
    } else if false_accepts > 0 {
        "PHASE_STREAM_ONLINE_MINER_DAEMON_V1_WATCH_FALSE_ACCEPTS"
    } else if unique_cpu_accepts_over_exact_cache == 0 {
        "PHASE_STREAM_ONLINE_MINER_DAEMON_V1_WATCH_NO_UNIQUE_ACCEPTS"
    } else {
        "PHASE_STREAM_ONLINE_MINER_DAEMON_V1_PASS_SHADOW_ONLY"
    };
    let report = OnlineMinerDaemonReport {
        report_kind: "phase_stream_online_miner_daemon_v1",
        mode: "bounded_append_only_online_phase_center_miner_shadow_only",
        control_mode: control_budget.mode.as_str(),
        row_budget: control_budget.row_budget,
        time_budget_ms: control_budget
            .time_budget
            .map(|duration| duration.as_millis() as u64),
        compile_budget: control_budget.compile_budget,
        compile_budget_used,
        compile_budget_exhausted,
        decision_log_limit: control_budget.decision_log_limit,
        decision_log_rows_written,
        decision_log_capped,
        sample_every: control_budget.sample_every,
        sampled_out_rows,
        budget_stop_reason,
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        checkpoint_dir: checkpoint_dir.display().to_string(),
        decision_log_path: decision_log_path.display().to_string(),
        cells,
        min_bucket_events,
        base_margin_threshold_micro,
        compile_every_rows,
        requested_max_active_buckets,
        max_active_buckets,
        shadow_audition_max_active_buckets: max_active_buckets,
        shadow_audition_cap_enforced: requested_max_active_buckets
            > ONLINE_MINER_SHADOW_AUDITION_MAX_ACTIVE_BUCKETS,
        product_hot_profile_cap_enforced: max_active_buckets
            <= ONLINE_MINER_PRODUCT_MAX_HOT_PROFILES_PER_WORKER,
        product_hot_profile_cap_exceeded_by_shadow_audition: max_active_buckets
            > ONLINE_MINER_PRODUCT_MAX_HOT_PROFILES_PER_WORKER,
        reservoir_per_label,
        total_rows,
        parsed_events,
        source_verifier_labeled_events,
        skipped_no_action_family,
        skipped_no_verifier_label,
        skipped_no_phase_atoms,
        online_auto_subcenter_split_enabled: true,
        online_auto_subcenter_max_per_event: ONLINE_MINER_MAX_AUTOSUBCENTER_BUCKETS_PER_EVENT,
        online_auto_subcenter_multi2_enabled: true,
        online_auto_subcenter_multi3_enabled: true,
        online_auto_subcenter_multi4_enabled: true,
        online_auto_subcenter_split_authority: "ranked observable request/state/tool atoms and bounded multi2/multi3/multi4 compounds; route hints and manual class lists are not split authority",
        manual_class_list_used: false,
        broad_action_bucket_updates,
        state_action_bucket_updates,
        auto_subcenter_bucket_updates,
        learned_subcenter_bucket_updates,
        learned_split_registry_action_count: split_atom_stats_by_action.len(),
        learned_split_atom_count,
        learned_split_single_atom_count: learned_split_atom_counts.single,
        learned_split_multi2_atom_count: learned_split_atom_counts.multi2,
        learned_split_multi3_atom_count: learned_split_atom_counts.multi3,
        learned_split_multi4_atom_count: learned_split_atom_counts.multi4,
        learned_split_compound_atom_count: learned_split_atom_counts.compound(),
        learned_split_conflict_gate_enabled: true,
        learned_split_conflict_action_count,
        learned_split_atoms_blocked_without_conflict,
        hidden_state_search_enabled: true,
        hidden_state_authority: "inferred hidden_state atoms from source-neutral observable request/state/tool intersections; no route hints, provider ids, exact-cache ids, target/proof labels, output hashes, manual class lists, or future answers",
        hidden_state_count,
        hidden_state_transition_count,
        hidden_state_split_candidates: hidden_state_bucket_count,
        hidden_state_bucket_count,
        hidden_state_forbidden_source_leak_bucket_count,
        hot_audition_replacement_enabled: true,
        hot_audition_replacement_count,
        base_bucket_count,
        state_action_bucket_count,
        auto_subcenter_bucket_count,
        auto_subcenter_multi2_bucket_count,
        auto_subcenter_multi3_bucket_count,
        auto_subcenter_multi4_bucket_count,
        auto_subcenter_forbidden_source_leak_bucket_count,
        learned_subcenter_bucket_count,
        learned_subcenter_multi2_bucket_count,
        learned_subcenter_multi3_bucket_count,
        learned_subcenter_multi4_bucket_count,
        bucket_count: buckets.len(),
        compile_ticks,
        bucket_readiness_compile_count,
        compiled_checkpoint_count: checkpoints.len(),
        active_profile_count,
        future_shadow_events,
        future_shadow_safe_events,
        local_operator_shadow_decisions,
        fallback_shadow_decisions,
        unique_cpu_accepts_over_exact_cache,
        nando_cpu_tokens_saved,
        nando_cpu_cost_saved_microusd,
        cli_margin_floor_micro: base_margin_threshold_micro,
        cli_threshold_selected_for_acceptance: false,
        threshold_source: "automatic_train_false_margin_plus_stream_calibration_window",
        calibration_window_before_shadow: true,
        shadow_window_after_calibration: true,
        per_bucket_thresholds_reported: true,
        fixed_policy_shadow_replay: true,
        accepted_delta_reported_against_fixed_policy: true,
        auto_calibration_window_max_decisions: ONLINE_MINER_AUTO_CALIBRATION_MAX_DECISIONS,
        auto_calibrated_margin_threshold_micro: auto_calibrated.threshold_micro,
        auto_calibrated_calibration_events: auto_calibrated.calibration_events,
        auto_calibrated_shadow_events_after_calibration: auto_calibrated
            .shadow_events_after_calibration,
        auto_calibrated_local_operator_shadow_decisions: auto_calibrated
            .local_operator_shadow_decisions,
        auto_calibrated_unique_cpu_accepts_over_exact_cache: auto_calibrated
            .unique_cpu_accepts_over_exact_cache,
        auto_calibrated_nando_cpu_tokens_saved: auto_calibrated.nando_cpu_tokens_saved,
        auto_calibrated_nando_cpu_cost_saved_microusd: auto_calibrated
            .nando_cpu_cost_saved_microusd,
        auto_calibrated_false_accepts: auto_calibrated.false_accepts,
        auto_calibrated_max_false_margin_micro: auto_calibrated.max_false_margin_micro,
        auto_calibrated_global_unique_cpu_accepts_over_exact_cache: global_auto_calibrated
            .unique_cpu_accepts_over_exact_cache,
        auto_calibrated_global_nando_cpu_tokens_saved: global_auto_calibrated
            .nando_cpu_tokens_saved,
        auto_calibrated_global_nando_cpu_cost_saved_microusd: global_auto_calibrated
            .nando_cpu_cost_saved_microusd,
        auto_calibrated_global_duplicate_accept_rows: global_auto_calibrated.duplicate_accept_rows,
        auto_calibrated_global_false_accepts: global_auto_calibrated.false_accepts,
        active_hot_unique_cpu_accepts_over_exact_cache,
        active_hot_false_accepts,
        active_hot_auto_calibrated_unique_cpu_accepts_over_exact_cache: active_hot_auto_calibrated
            .unique_cpu_accepts_over_exact_cache,
        active_hot_auto_calibrated_false_accepts: active_hot_auto_calibrated.false_accepts,
        active_hot_auto_calibrated_accepted_bucket_count: active_hot_auto_calibrated
            .accepted_bucket_count,
        active_hot_auto_calibrated_rejected_bucket_count: active_hot_auto_calibrated
            .rejected_bucket_count,
        active_hot_auto_calibrated_rejected_false_accepts: active_hot_auto_calibrated
            .rejected_false_accepts,
        multi_split_portfolio_enabled: true,
        multi_split_candidate_profile_count: multi_split_auto_calibrated.accepted_bucket_count,
        multi_split_unique_cpu_accepts_over_exact_cache: multi_split_global_auto_calibrated
            .unique_cpu_accepts_over_exact_cache,
        multi_split_nando_cpu_tokens_saved: multi_split_global_auto_calibrated
            .nando_cpu_tokens_saved,
        multi_split_nando_cpu_cost_saved_microusd: multi_split_global_auto_calibrated
            .nando_cpu_cost_saved_microusd,
        multi_split_duplicate_accept_rows: multi_split_global_auto_calibrated.duplicate_accept_rows,
        multi_split_false_accepts: multi_split_global_auto_calibrated.false_accepts,
        multi_split_rejected_shadow_bucket_count: multi_split_auto_calibrated.rejected_bucket_count,
        multi_split_rejected_shadow_false_accepts: multi_split_auto_calibrated
            .rejected_false_accepts,
        multi_split_promotion_candidate_count,
        multi_split_promotion_gate_passed_count,
        multi_split_promotion_gate_failed_count,
        multi_split_promotion_candidates,
        product_hot_portfolio_enabled: true,
        product_hot_candidate_profile_count,
        product_hot_unique_cpu_accepts_over_exact_cache,
        product_hot_nando_cpu_tokens_saved,
        product_hot_nando_cpu_cost_saved_microusd,
        product_hot_duplicate_accept_rows,
        product_hot_false_accepts,
        product_hot_rejected_shadow_bucket_count,
        product_hot_rejected_shadow_false_accepts,
        product_hot_profile_cap,
        product_hot_selected_by_budget_cap,
        product_hot_package_bytes_estimate,
        product_hot_shadow_registry_budget_passed,
        product_hot_promotion_candidate_count,
        product_hot_promotion_gate_passed_count,
        product_hot_promotion_gate_failed_count,
        product_hot_promotion_registry_path: product_hot_promotion_registry_path
            .display()
            .to_string(),
        product_hot_promotion_registry_written: true,
        product_hot_promotion_registry_readback_exact,
        product_hot_kind_contributions,
        product_hot_promotion_candidates,
        auto_calibrated_all_accepted_buckets_false_accepts_zero: auto_calibrated.false_accepts == 0,
        auto_calibrated_accepted_bucket_count: auto_calibrated.accepted_bucket_count,
        auto_calibrated_rejected_bucket_count: auto_calibrated.rejected_bucket_count,
        auto_calibrated_rejected_false_accepts: auto_calibrated.rejected_false_accepts,
        runtime_margin_parity_mismatches,
        hot_runtime_margin_parity_checks,
        hot_runtime_margin_parity_mismatches,
        hot_runtime_decision_parity_mismatches,
        hot_runtime_latency_p50_ns: percentile_u128(&hot_runtime_latencies, 50),
        hot_runtime_latency_p90_ns: percentile_u128(&hot_runtime_latencies, 90),
        hot_runtime_latency_p99_ns: percentile_u128(&hot_runtime_latencies, 99),
        false_accepts,
        wrong_wins,
        min_margin_micro: margins.first().copied().unwrap_or(0),
        p10_margin_micro: percentile_i64(&margins, 10),
        median_margin_micro: percentile_i64(&margins, 50),
        latency_p50_ns: percentile_u128(&latencies, 50),
        latency_p90_ns: percentile_u128(&latencies, 90),
        latency_p99_ns: percentile_u128(&latencies, 99),
        memory_budget,
        checkpoints,
        buckets: bucket_reports,
        stream_contract: OnlineMinerStreamContract {
            append_only_input: true,
            score_before_train: true,
            future_only_shadow_scoring: true,
            incremental_bucket_updates: true,
            bucket_readiness_quarantine_compile: true,
            positive_negative_reservoirs: true,
            periodic_quarantine_nwpc_compile: true,
            compatible_denominator_delta_in_same_pass: true,
        },
        local_accept_enabled: false,
        auto_promote_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: serde_json::json!({
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false
        }),
        verdict,
        boundary: "online miner daemon shadow only: scans append-only trace rows in order, scores each event before updating the bucket, automatically fans out broad/state-action/source-neutral subcenter buckets, compiles quarantine .nwpc checkpoints from past events only, and never promotes, enables local_accept, claims market money, revives .nwrb, uses lookup, target/proof authority, concrete_x_lookup, or manual local_out_t",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_daemon_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  checkpoint_dir: {}", checkpoint_dir.display());
    println!("  decision_log_path: {}", decision_log_path.display());
    println!("  control_mode: {}", report.control_mode);
    println!("  row_budget: {:?}", report.row_budget);
    println!("  time_budget_ms: {:?}", report.time_budget_ms);
    println!("  compile_budget: {:?}", report.compile_budget);
    println!("  compile_budget_used: {}", report.compile_budget_used);
    println!(
        "  compile_budget_exhausted: {}",
        report.compile_budget_exhausted
    );
    println!("  sample_every: {}", report.sample_every);
    println!("  sampled_out_rows: {}", report.sampled_out_rows);
    println!(
        "  decision_log_rows_written: {}",
        report.decision_log_rows_written
    );
    println!("  decision_log_capped: {}", report.decision_log_capped);
    println!("  budget_stop_reason: {:?}", report.budget_stop_reason);
    println!("  total_rows: {total_rows}");
    println!("  parsed_events: {parsed_events}");
    println!("  source_verifier_labeled_events: {source_verifier_labeled_events}");
    println!("  requested_max_active_buckets: {requested_max_active_buckets}");
    println!("  effective_max_active_buckets: {max_active_buckets}");
    println!("  bucket_count: {}", report.bucket_count);
    println!("  broad_action_bucket_count: {}", report.base_bucket_count);
    println!(
        "  state_action_bucket_count: {}",
        report.state_action_bucket_count
    );
    println!(
        "  auto_subcenter_bucket_count: {}",
        report.auto_subcenter_bucket_count
    );
    println!(
        "  auto_subcenter_multi2_bucket_count: {}",
        report.auto_subcenter_multi2_bucket_count
    );
    println!(
        "  auto_subcenter_multi3_bucket_count: {}",
        report.auto_subcenter_multi3_bucket_count
    );
    println!(
        "  auto_subcenter_multi4_bucket_count: {}",
        report.auto_subcenter_multi4_bucket_count
    );
    println!(
        "  auto_subcenter_forbidden_source_leak_bucket_count: {}",
        report.auto_subcenter_forbidden_source_leak_bucket_count
    );
    println!(
        "  learned_subcenter_bucket_count: {}",
        report.learned_subcenter_bucket_count
    );
    println!(
        "  learned_subcenter_multi2_bucket_count: {}",
        report.learned_subcenter_multi2_bucket_count
    );
    println!(
        "  learned_subcenter_multi3_bucket_count: {}",
        report.learned_subcenter_multi3_bucket_count
    );
    println!(
        "  learned_subcenter_multi4_bucket_count: {}",
        report.learned_subcenter_multi4_bucket_count
    );
    println!(
        "  learned_split_atom_count: {}",
        report.learned_split_atom_count
    );
    println!(
        "  learned_split_compound_atom_count: {}",
        report.learned_split_compound_atom_count
    );
    println!("  hidden_state_count: {}", report.hidden_state_count);
    println!(
        "  hidden_state_transition_count: {}",
        report.hidden_state_transition_count
    );
    println!(
        "  hidden_state_bucket_count: {}",
        report.hidden_state_bucket_count
    );
    println!(
        "  hidden_state_forbidden_source_leak_bucket_count: {}",
        report.hidden_state_forbidden_source_leak_bucket_count
    );
    println!(
        "  hot_audition_replacement_count: {}",
        report.hot_audition_replacement_count
    );
    println!(
        "  compiled_checkpoint_count: {}",
        report.compiled_checkpoint_count
    );
    println!("  future_shadow_events: {future_shadow_events}");
    println!("  unique_cpu_accepts_over_exact_cache: {unique_cpu_accepts_over_exact_cache}");
    println!("  false_accepts: {false_accepts}");
    println!(
        "  auto_calibrated_unique_cpu_accepts_over_exact_cache: {}",
        report.auto_calibrated_unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  auto_calibrated_global_unique_cpu_accepts_over_exact_cache: {}",
        report.auto_calibrated_global_unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  auto_calibrated_global_duplicate_accept_rows: {}",
        report.auto_calibrated_global_duplicate_accept_rows
    );
    println!(
        "  active_hot_auto_calibrated_unique_cpu_accepts_over_exact_cache: {}",
        report.active_hot_auto_calibrated_unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  active_hot_auto_calibrated_false_accepts: {}",
        report.active_hot_auto_calibrated_false_accepts
    );
    println!(
        "  active_hot_auto_calibrated_accepted_bucket_count: {}",
        report.active_hot_auto_calibrated_accepted_bucket_count
    );
    println!(
        "  active_hot_auto_calibrated_rejected_bucket_count: {}",
        report.active_hot_auto_calibrated_rejected_bucket_count
    );
    println!(
        "  active_hot_auto_calibrated_rejected_false_accepts: {}",
        report.active_hot_auto_calibrated_rejected_false_accepts
    );
    println!(
        "  multi_split_unique_cpu_accepts_over_exact_cache: {}",
        report.multi_split_unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  multi_split_false_accepts: {}",
        report.multi_split_false_accepts
    );
    println!(
        "  multi_split_promotion_candidate_count: {}",
        report.multi_split_promotion_candidate_count
    );
    println!(
        "  product_hot_candidate_profile_count: {}",
        report.product_hot_candidate_profile_count
    );
    println!(
        "  product_hot_unique_cpu_accepts_over_exact_cache: {}",
        report.product_hot_unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  product_hot_duplicate_accept_rows: {}",
        report.product_hot_duplicate_accept_rows
    );
    println!(
        "  product_hot_false_accepts: {}",
        report.product_hot_false_accepts
    );
    println!(
        "  product_hot_rejected_shadow_bucket_count: {}",
        report.product_hot_rejected_shadow_bucket_count
    );
    println!(
        "  product_hot_rejected_shadow_false_accepts: {}",
        report.product_hot_rejected_shadow_false_accepts
    );
    println!(
        "  product_hot_profile_cap: {}",
        report.product_hot_profile_cap
    );
    println!(
        "  product_hot_selected_by_budget_cap: {}",
        report.product_hot_selected_by_budget_cap
    );
    println!(
        "  product_hot_package_bytes_estimate: {}",
        report.product_hot_package_bytes_estimate
    );
    println!(
        "  product_hot_shadow_registry_budget_passed: {}",
        report.product_hot_shadow_registry_budget_passed
    );
    println!(
        "  product_hot_promotion_candidate_count: {}",
        report.product_hot_promotion_candidate_count
    );
    println!(
        "  product_hot_promotion_gate_passed_count: {}",
        report.product_hot_promotion_gate_passed_count
    );
    println!(
        "  product_hot_promotion_gate_failed_count: {}",
        report.product_hot_promotion_gate_failed_count
    );
    println!(
        "  product_hot_promotion_registry_path: {}",
        report.product_hot_promotion_registry_path
    );
    println!(
        "  product_hot_promotion_registry_readback_exact: {}",
        report.product_hot_promotion_registry_readback_exact
    );
    let hidden_state_product_hot_contribution = report
        .product_hot_kind_contributions
        .iter()
        .find(|contribution| contribution.bucket_kind == "hidden_state_split");
    println!(
        "  product_hot_hidden_state_selected_profile_count: {}",
        hidden_state_product_hot_contribution
            .map_or(0, |contribution| { contribution.selected_profile_count })
    );
    println!(
        "  product_hot_hidden_state_unique_cpu_accepts_over_exact_cache: {}",
        hidden_state_product_hot_contribution.map_or(0, |contribution| {
            contribution.marginal_unique_cpu_accepts_over_exact_cache
        })
    );
    println!(
        "  auto_calibrated_false_accepts: {}",
        report.auto_calibrated_false_accepts
    );
    println!(
        "  auto_calibrated_accepted_bucket_count: {}",
        report.auto_calibrated_accepted_bucket_count
    );
    println!(
        "  auto_calibrated_rejected_bucket_count: {}",
        report.auto_calibrated_rejected_bucket_count
    );
    println!(
        "  hot_bytes_estimate: {}",
        report.memory_budget.hot_bytes_estimate
    );
    println!(
        "  hot_budget_passed: {}",
        report.memory_budget.hot_budget_passed
    );
    println!("  local_accept_enabled: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn online_miner_value_pass_milli_usize(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    ((numerator as u128).saturating_mul(1000) / denominator as u128).min(usize::MAX as u128)
        as usize
}

fn online_miner_value_pass_milli_u64(numerator: u64, denominator: u64) -> usize {
    if denominator == 0 {
        return 0;
    }
    ((numerator as u128).saturating_mul(1000) / denominator as u128).min(usize::MAX as u128)
        as usize
}

fn online_miner_value_pass_exact_cache_key(row: &Value, row_index: usize) -> String {
    json_string(row, &["exact_cache_key"])
        .or_else(|| json_string(row, &["request_fingerprint"]))
        .unwrap_or_else(|| format!("online_miner_value_row:{row_index}"))
}

fn collect_online_miner_value_pass(
    trace_paths: &[PathBuf],
    progress_label: &str,
) -> Result<OnlineMinerValuePassCollection, String> {
    let mut buckets = BTreeMap::<String, OnlineMinerValuePassBucketState>::new();
    let mut split_atom_stats_by_action =
        BTreeMap::<String, BTreeMap<String, OnlineMinerSplitAtomStats>>::new();
    let mut seen_exact_cache_keys = BTreeSet::<String>::new();
    let mut denominator_seen_exact_cache_keys = BTreeSet::<String>::new();
    let mut total_rows = 0usize;
    let mut parsed_events = 0usize;
    let mut source_verifier_labeled_events = 0usize;
    let mut exact_cache_hits = 0usize;
    let mut non_exact_rows = 0usize;
    let mut total_tokens_seen = 0usize;
    let mut total_cost_microusd_seen = 0u64;
    let mut skipped_no_action_family = 0usize;
    let mut skipped_no_verifier_label = 0usize;
    let mut skipped_no_phase_atoms = 0usize;
    let mut learned_split_atoms_blocked_without_conflict = 0usize;

    for trace_path in trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read online miner value trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows = total_rows.saturating_add(1);
            if total_rows == 1 || total_rows.is_multiple_of(5000) {
                println!(
                    "{progress_label}: rows={} buckets={}",
                    total_rows,
                    buckets.len()
                );
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse online miner value trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            let denominator_token_cost = phase_atom_binary_token_cost(&row);
            total_tokens_seen =
                total_tokens_seen.saturating_add(denominator_token_cost.total_tokens);
            total_cost_microusd_seen =
                total_cost_microusd_seen.saturating_add(denominator_token_cost.total_cost_microusd);
            if denominator_seen_exact_cache_keys
                .insert(online_miner_value_pass_exact_cache_key(&row, total_rows))
            {
                non_exact_rows = non_exact_rows.saturating_add(1);
            } else {
                exact_cache_hits = exact_cache_hits.saturating_add(1);
            }
            let Some(verified_safe_accept) = row
                .get("verified_safe_accept")
                .and_then(serde_json::Value::as_bool)
            else {
                skipped_no_verifier_label = skipped_no_verifier_label.saturating_add(1);
                continue;
            };
            source_verifier_labeled_events = source_verifier_labeled_events.saturating_add(1);
            let action_atoms = phase_atom_string_vec(&row, "action_atoms");
            let action_families = phase_atom_action_families(&action_atoms);
            if action_families.is_empty() {
                skipped_no_action_family = skipped_no_action_family.saturating_add(1);
                continue;
            }
            let request_atoms = phase_atom_string_vec(&row, "request_atoms");
            let state_atoms = phase_atom_string_vec(&row, "state_atoms");
            let tool_atoms = phase_atom_string_vec(&row, "tool_atoms");
            let route_hint_atoms = phase_atom_string_vec(&row, "route_hint_atoms");
            if request_atoms.is_empty()
                && state_atoms.is_empty()
                && tool_atoms.is_empty()
                && route_hint_atoms.is_empty()
            {
                skipped_no_phase_atoms = skipped_no_phase_atoms.saturating_add(1);
                continue;
            }
            let exact_cache_key = online_miner_value_pass_exact_cache_key(&row, total_rows);
            let request_fingerprint = json_string(&row, &["request_fingerprint"])
                .unwrap_or_else(|| format!("online_miner_value_fingerprint:{total_rows}"));
            let exact_cache_hit = !seen_exact_cache_keys.insert(exact_cache_key);
            let token_cost = phase_atom_binary_token_cost(&row);
            let event_split_atoms = online_miner_event_split_atoms(
                &request_atoms,
                &state_atoms,
                &tool_atoms,
                &route_hint_atoms,
            );
            for action_family in action_families {
                let split_pressure =
                    online_miner_value_pass_action_has_split_pressure(&buckets, &action_family);
                let learned_split_atoms = online_miner_learned_split_atoms_for_action(
                    &split_atom_stats_by_action,
                    &action_family,
                    ONLINE_MINER_MAX_AUTOSUBCENTER_BUCKETS_PER_EVENT,
                    split_pressure,
                );
                if learned_split_atoms.is_empty()
                    && online_miner_action_family_has_candidate_split_atoms(
                        &split_atom_stats_by_action,
                        &action_family,
                    )
                    && !split_pressure
                {
                    learned_split_atoms_blocked_without_conflict =
                        learned_split_atoms_blocked_without_conflict.saturating_add(1);
                }
                let bucket_specs = online_miner_event_bucket_specs(
                    &action_family,
                    &request_atoms,
                    &state_atoms,
                    &tool_atoms,
                    &route_hint_atoms,
                    &learned_split_atoms,
                );
                for (bucket_kind, bucket_key) in bucket_specs {
                    parsed_events = parsed_events.saturating_add(1);
                    let bucket = buckets.entry(bucket_key.clone()).or_insert_with(|| {
                        OnlineMinerValuePassBucketState::new(
                            bucket_key,
                            bucket_kind,
                            action_family.clone(),
                        )
                    });
                    bucket.observe(
                        request_fingerprint.clone(),
                        verified_safe_accept,
                        exact_cache_hit,
                        token_cost,
                    );
                }
                observe_online_miner_split_atoms(
                    &mut split_atom_stats_by_action,
                    &action_family,
                    &event_split_atoms,
                    verified_safe_accept,
                    exact_cache_hit,
                    token_cost,
                );
            }
        }
    }

    Ok(OnlineMinerValuePassCollection {
        buckets,
        split_atom_stats_by_action,
        total_rows,
        parsed_events,
        source_verifier_labeled_events,
        exact_cache_hits,
        non_exact_rows,
        total_tokens_seen,
        total_cost_microusd_seen,
        skipped_no_action_family,
        skipped_no_verifier_label,
        skipped_no_phase_atoms,
        learned_split_atoms_blocked_without_conflict,
    })
}

pub(crate) fn run_phase_stream_online_miner_value_pass_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_VALUE_PASS_REPORT));
    let requested_top_k = parse_optional_usize(args.next(), "top_k")?
        .unwrap_or(ONLINE_MINER_VALUE_PASS_DEFAULT_TOP_K);
    if requested_top_k == 0 {
        return Err("top_k must be > 0".to_owned());
    }
    let mut price_config_path = PathBuf::from(DEFAULT_PRICE_CONFIG);
    let mut trace_paths = Vec::<PathBuf>::new();
    let mut rest = args.collect::<Vec<_>>().into_iter();
    while let Some(arg) = rest.next() {
        if arg == "--price-config" {
            let Some(path) = rest.next() else {
                return Err("--price-config requires a path".to_owned());
            };
            price_config_path = PathBuf::from(path);
        } else if let Some(path) = arg.strip_prefix("--price-config=") {
            if path.is_empty() {
                return Err("--price-config= requires a non-empty path".to_owned());
            }
            price_config_path = PathBuf::from(path);
        } else {
            trace_paths.push(PathBuf::from(arg));
        }
    }
    if trace_paths.is_empty() {
        trace_paths.push(PathBuf::from(
            DEFAULT_AUTO_SUBCENTER_DISCOVERY_CANDIDATES_JSONL,
        ));
    }
    if trace_paths.is_empty() {
        return Err("online miner value pass needs at least one trace path".to_owned());
    }
    let price_config = read_json_file::<ModelPriceConfig>(&price_config_path)?;

    let collection =
        collect_online_miner_value_pass(&trace_paths, "online_miner_value_pass_progress")?;
    let buckets = collection.buckets;
    let split_atom_stats_by_action = collection.split_atom_stats_by_action;
    let total_rows = collection.total_rows;
    let parsed_events = collection.parsed_events;
    let source_verifier_labeled_events = collection.source_verifier_labeled_events;
    let exact_cache_hits = collection.exact_cache_hits;
    let non_exact_rows = collection.non_exact_rows;
    let total_tokens_seen = collection.total_tokens_seen;
    let total_cost_microusd_seen = collection.total_cost_microusd_seen;
    let skipped_no_action_family = collection.skipped_no_action_family;
    let skipped_no_verifier_label = collection.skipped_no_verifier_label;
    let skipped_no_phase_atoms = collection.skipped_no_phase_atoms;
    let learned_split_atoms_blocked_without_conflict =
        collection.learned_split_atoms_blocked_without_conflict;

    let selected_candidates =
        online_miner_value_pass_product_hot_candidates(&buckets, requested_top_k);
    let top_candidates = online_miner_value_pass_top_candidates(&buckets, requested_top_k);
    let upper_bound = online_miner_value_pass_global_stats_for_candidates(
        &buckets,
        &selected_candidates
            .iter()
            .map(|candidate| candidate.bucket_key.as_str())
            .collect::<Vec<_>>(),
    );
    let product_hot_kind_contributions =
        online_miner_value_pass_kind_contributions(&buckets, &selected_candidates);
    let learned_split_atom_counts =
        online_miner_learned_split_atom_counts(&split_atom_stats_by_action);
    let learned_split_conflict_action_count = split_atom_stats_by_action
        .keys()
        .filter(|action_family| {
            online_miner_value_pass_action_has_split_pressure(&buckets, action_family)
        })
        .count();
    let hidden_state_bucket_count = buckets
        .values()
        .filter(|bucket| bucket.bucket_kind == "hidden_state_split")
        .count();
    let hidden_state_forbidden_source_leak_bucket_count = buckets
        .values()
        .filter(|bucket| {
            bucket.bucket_kind == "hidden_state_split"
                && online_miner_hidden_state_bucket_has_forbidden_source_leak(&bucket.bucket_key)
        })
        .count();
    let token_denominator_present = total_tokens_seen > 0;
    let cost_denominator_present = total_cost_microusd_seen > 0;
    let estimated_total_cost_microusd_seen =
        estimated_event_cost_microusd(total_tokens_seen, 0, &price_config);
    let estimated_cost_denominator_present = estimated_total_cost_microusd_seen > 0;
    let product_hot_candidate_upper_bound_estimated_cost_saved_microusd =
        estimated_event_cost_microusd(upper_bound.nando_cpu_tokens_saved, 0, &price_config);
    let market_money_claim_blocker = if !cost_denominator_present {
        if estimated_cost_denominator_present {
            "provider_cost_missing_estimate_only"
        } else {
            "cost_denominator_missing"
        }
    } else if selected_candidates.is_empty() {
        "no_selected_candidates"
    } else if upper_bound.nando_cpu_cost_saved_microusd == 0 {
        "no_cost_saved_upper_bound"
    } else {
        "compile_and_shadow_replay_required"
    };
    let estimated_money_claim_blocker = if !estimated_cost_denominator_present {
        "estimated_cost_denominator_missing"
    } else if selected_candidates.is_empty() {
        "no_selected_candidates"
    } else {
        "estimate_only_not_market_claim"
    };
    let verdict = if selected_candidates.is_empty() {
        "PHASE_STREAM_ONLINE_MINER_VALUE_PASS_V1_WATCH_NO_CANDIDATES"
    } else if hidden_state_forbidden_source_leak_bucket_count > 0 {
        "PHASE_STREAM_ONLINE_MINER_VALUE_PASS_V1_FAIL_HIDDEN_STATE_SOURCE_LEAK"
    } else {
        "PHASE_STREAM_ONLINE_MINER_VALUE_PASS_V1_PASS_CANDIDATE_SELECTOR_READY"
    };
    let report = OnlineMinerValuePassReport {
        report_kind: "phase_stream_online_miner_value_pass_v1",
        mode: "no_compile_streaming_candidate_value_pass",
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        model_price_config_path: price_config_path.display().to_string(),
        total_rows,
        parsed_events,
        source_verifier_labeled_events,
        exact_cache_hits,
        non_exact_rows,
        total_tokens_seen,
        total_cost_microusd_seen,
        estimated_total_cost_microusd_seen,
        token_denominator_present,
        cost_denominator_present,
        estimated_cost_denominator_present,
        token_cost_denominator_present: token_denominator_present && cost_denominator_present,
        skipped_no_action_family,
        skipped_no_verifier_label,
        skipped_no_phase_atoms,
        bucket_count: buckets.len(),
        broad_action_bucket_count: online_miner_value_pass_bucket_kind_count(
            &buckets,
            "broad_action",
        ),
        state_action_bucket_count: online_miner_value_pass_bucket_kind_count(
            &buckets,
            "state_action_signature",
        ),
        auto_subcenter_bucket_count: online_miner_value_pass_bucket_kind_count(
            &buckets,
            "auto_subcenter",
        ),
        learned_subcenter_bucket_count: online_miner_value_pass_bucket_kind_count(
            &buckets,
            "learned_auto_subcenter",
        ),
        hidden_state_bucket_count,
        hidden_state_forbidden_source_leak_bucket_count,
        learned_split_registry_action_count: split_atom_stats_by_action.len(),
        learned_split_atom_count: learned_split_atom_counts.total(),
        learned_split_compound_atom_count: learned_split_atom_counts.compound(),
        learned_split_conflict_gate_enabled: true,
        learned_split_conflict_action_count,
        learned_split_atoms_blocked_without_conflict,
        product_hot_candidate_upper_bound_profile_count: selected_candidates.len(),
        product_hot_candidate_upper_bound_unique_accepts_over_exact_cache: upper_bound
            .unique_cpu_accepts_over_exact_cache,
        product_hot_candidate_upper_bound_tokens_saved: upper_bound.nando_cpu_tokens_saved,
        product_hot_candidate_upper_bound_cost_saved_microusd: upper_bound
            .nando_cpu_cost_saved_microusd,
        product_hot_candidate_upper_bound_estimated_cost_saved_microusd,
        product_hot_candidate_upper_bound_duplicate_accept_rows: upper_bound.duplicate_accept_rows,
        product_hot_candidate_upper_bound_calls_saved_milli_over_total_rows:
            online_miner_value_pass_milli_usize(
                upper_bound.unique_cpu_accepts_over_exact_cache,
                total_rows,
            ),
        product_hot_candidate_upper_bound_calls_saved_milli_over_labeled_events:
            online_miner_value_pass_milli_usize(
                upper_bound.unique_cpu_accepts_over_exact_cache,
                source_verifier_labeled_events,
            ),
        product_hot_candidate_upper_bound_calls_saved_milli_over_non_exact_rows:
            online_miner_value_pass_milli_usize(
                upper_bound.unique_cpu_accepts_over_exact_cache,
                non_exact_rows,
            ),
        product_hot_candidate_upper_bound_tokens_saved_milli_over_total_tokens:
            online_miner_value_pass_milli_usize(
                upper_bound.nando_cpu_tokens_saved,
                total_tokens_seen,
            ),
        product_hot_candidate_upper_bound_cost_saved_milli_over_total_cost:
            online_miner_value_pass_milli_u64(
                upper_bound.nando_cpu_cost_saved_microusd,
                total_cost_microusd_seen,
            ),
        product_hot_candidate_upper_bound_estimated_cost_saved_milli_over_estimated_total_cost:
            online_miner_value_pass_milli_u64(
                product_hot_candidate_upper_bound_estimated_cost_saved_microusd,
                estimated_total_cost_microusd_seen,
            ),
        product_hot_kind_contributions,
        selected_product_hot_candidates: selected_candidates,
        top_candidates,
        compile_required_for_runtime_proof: true,
        runtime_false_accepts_measured: false,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        market_money_claim_blocker,
        estimated_money_claim_allowed: false,
        estimated_money_claim_blocker,
        estimated_cost_method: "total_saved_tokens_as_input_token_floor_from_model_price_config",
        price_config_schema_version: price_config.schema_version,
        price_config_source: price_config.price_source,
        forbidden_flags: serde_json::json!({
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "local_accept_without_verifier_used": false
        }),
        verdict,
        boundary: "value-pass selector only: scans labeled real traces once, ranks verifier-labeled candidate buckets without compiling .nwpc packages, and cannot prove runtime accepts, false_accepts, local_accept, promotion, or market money until selected top candidates are compiled and shadow-replayed",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_value_pass_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  total_rows: {total_rows}");
    println!("  parsed_events: {parsed_events}");
    println!("  exact_cache_hits: {exact_cache_hits}");
    println!("  non_exact_rows: {non_exact_rows}");
    println!("  total_tokens_seen: {total_tokens_seen}");
    println!("  total_cost_microusd_seen: {total_cost_microusd_seen}");
    println!("  estimated_total_cost_microusd_seen: {estimated_total_cost_microusd_seen}");
    println!("  token_denominator_present: {token_denominator_present}");
    println!("  cost_denominator_present: {cost_denominator_present}");
    println!("  estimated_cost_denominator_present: {estimated_cost_denominator_present}");
    println!("  bucket_count: {}", report.bucket_count);
    println!("  hidden_state_bucket_count: {hidden_state_bucket_count}");
    println!(
        "  product_hot_candidate_upper_bound_profile_count: {}",
        report.product_hot_candidate_upper_bound_profile_count
    );
    println!(
        "  product_hot_candidate_upper_bound_unique_accepts_over_exact_cache: {}",
        report.product_hot_candidate_upper_bound_unique_accepts_over_exact_cache
    );
    println!(
        "  product_hot_candidate_upper_bound_calls_saved_milli_over_total_rows: {}",
        report.product_hot_candidate_upper_bound_calls_saved_milli_over_total_rows
    );
    println!(
        "  product_hot_candidate_upper_bound_tokens_saved_milli_over_total_tokens: {}",
        report.product_hot_candidate_upper_bound_tokens_saved_milli_over_total_tokens
    );
    println!(
        "  product_hot_candidate_upper_bound_cost_saved_milli_over_total_cost: {}",
        report.product_hot_candidate_upper_bound_cost_saved_milli_over_total_cost
    );
    println!(
        "  product_hot_candidate_upper_bound_estimated_cost_saved_microusd: {}",
        report.product_hot_candidate_upper_bound_estimated_cost_saved_microusd
    );
    println!(
        "  product_hot_candidate_upper_bound_estimated_cost_saved_milli_over_estimated_total_cost: {}",
        report
            .product_hot_candidate_upper_bound_estimated_cost_saved_milli_over_estimated_total_cost
    );
    println!("  compile_required_for_runtime_proof: true");
    println!("  runtime_false_accepts_measured: false");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  market_money_claim_blocker: {market_money_claim_blocker}");
    println!("  estimated_money_claim_allowed: false");
    println!("  estimated_money_claim_blocker: {estimated_money_claim_blocker}");
    println!("  verdict: {verdict}");
    Ok(())
}

pub(crate) fn run_phase_stream_online_miner_targeted_shadow_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_TARGETED_SHADOW_REPORT));
    let checkpoint_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_TARGETED_SHADOW_DIR));
    let cells = parse_optional_usize(args.next(), "cells")?.unwrap_or(32);
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    let requested_top_k = parse_optional_usize(args.next(), "top_k")?
        .unwrap_or(ONLINE_MINER_VALUE_PASS_DEFAULT_TOP_K);
    if requested_top_k == 0 {
        return Err("top_k must be > 0".to_owned());
    }
    let train_permille = parse_optional_usize(args.next(), "train_permille")?.unwrap_or(500);
    if !(1..=999).contains(&train_permille) {
        return Err("train_permille must be in 1..=999".to_owned());
    }
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(
                DEFAULT_AUTO_SUBCENTER_DISCOVERY_CANDIDATES_JSONL,
            )]
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("online miner targeted shadow needs at least one trace path".to_owned());
    }
    std::fs::create_dir_all(&checkpoint_dir).map_err(|error| {
        format!(
            "failed to create online miner targeted shadow dir '{}': {error}",
            checkpoint_dir.display()
        )
    })?;
    let decision_log_path = checkpoint_dir.join("targeted-shadow.decisions.jsonl");
    let mut decision_log = std::fs::File::create(&decision_log_path).map_err(|error| {
        format!(
            "failed to create online miner targeted shadow decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    let mut decision_log_rows_written = 0usize;
    let mut decision_log_capped = false;

    let value_collection = collect_online_miner_value_pass(
        &trace_paths,
        "online_miner_targeted_shadow_selector_progress",
    )?;
    let selected_candidates =
        online_miner_value_pass_product_hot_candidates(&value_collection.buckets, requested_top_k);
    let selected_keys = selected_candidates
        .iter()
        .map(|candidate| candidate.bucket_key.clone())
        .collect::<BTreeSet<_>>();
    let selected_train_targets = selected_candidates
        .iter()
        .map(|candidate| {
            (
                candidate.bucket_key.clone(),
                online_miner_targeted_train_target(candidate.events_seen, train_permille),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut buckets = BTreeMap::<String, OnlineMinerBucketState>::new();
    let mut selected_seen_counts = BTreeMap::<String, usize>::new();
    let mut seen_exact_cache_keys = BTreeSet::<String>::new();
    let mut checkpoints = Vec::<OnlineMinerCheckpointReport>::new();
    let mut margins = Vec::<i64>::new();
    let mut latencies = Vec::<u128>::new();
    let mut hot_runtime_latencies = Vec::<u128>::new();
    let mut total_rows = 0usize;
    let mut parsed_events = 0usize;

    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read online miner targeted shadow trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows = total_rows.saturating_add(1);
            if total_rows == 1 || total_rows.is_multiple_of(5000) {
                println!(
                    "online_miner_targeted_shadow_progress: rows={} active_selected_buckets={}",
                    total_rows,
                    buckets.len()
                );
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse online miner targeted shadow trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            let Some(_verified_safe_accept) = row
                .get("verified_safe_accept")
                .and_then(serde_json::Value::as_bool)
            else {
                continue;
            };
            let action_atoms = phase_atom_string_vec(&row, "action_atoms");
            let action_families = phase_atom_action_families(&action_atoms);
            if action_families.is_empty() {
                continue;
            }
            let request_atoms = phase_atom_string_vec(&row, "request_atoms");
            let state_atoms = phase_atom_string_vec(&row, "state_atoms");
            let tool_atoms = phase_atom_string_vec(&row, "tool_atoms");
            let route_hint_atoms = phase_atom_string_vec(&row, "route_hint_atoms");
            if request_atoms.is_empty()
                && state_atoms.is_empty()
                && tool_atoms.is_empty()
                && route_hint_atoms.is_empty()
            {
                continue;
            }
            let exact_cache_key = json_string(&row, &["exact_cache_key"])
                .or_else(|| json_string(&row, &["request_fingerprint"]))
                .unwrap_or_else(|| format!("online_miner_targeted_row:{total_rows}"));
            let exact_cache_hit = !seen_exact_cache_keys.insert(exact_cache_key);
            let token_cost = phase_atom_binary_token_cost(&row);
            for action_family in action_families {
                let split_pressure = online_miner_value_pass_action_has_split_pressure(
                    &value_collection.buckets,
                    &action_family,
                );
                let learned_split_atoms = online_miner_learned_split_atoms_for_action(
                    &value_collection.split_atom_stats_by_action,
                    &action_family,
                    ONLINE_MINER_MAX_AUTOSUBCENTER_BUCKETS_PER_EVENT,
                    split_pressure,
                );
                for (bucket_kind, bucket_key) in online_miner_event_bucket_specs(
                    &action_family,
                    &request_atoms,
                    &state_atoms,
                    &tool_atoms,
                    &route_hint_atoms,
                    &learned_split_atoms,
                ) {
                    if !selected_keys.contains(&bucket_key) {
                        continue;
                    }
                    parsed_events = parsed_events.saturating_add(1);
                    let task_name = phase_atom_live_self_mining_task_name(&bucket_key);
                    let Some(event) = parse_phase_atom_binary_event_for_action(
                        &row,
                        total_rows,
                        &action_family,
                        &task_name,
                    ) else {
                        continue;
                    };
                    let seen_for_bucket =
                        selected_seen_counts.entry(bucket_key.clone()).or_default();
                    *seen_for_bucket = seen_for_bucket.saturating_add(1);
                    let train_target = selected_train_targets
                        .get(&bucket_key)
                        .copied()
                        .unwrap_or(1);
                    let bucket = buckets.entry(bucket_key.clone()).or_insert_with(|| {
                        OnlineMinerBucketState::new(
                            bucket_key.clone(),
                            bucket_kind,
                            action_family.clone(),
                            cells,
                        )
                        .expect("selected online miner bucket state initializes")
                    });

                    if bucket.active_runtime.is_some() {
                        score_future_event_before_update(
                            bucket,
                            &event,
                            exact_cache_hit,
                            total_rows,
                            &mut decision_log,
                            &mut decision_log_rows_written,
                            None,
                            &mut decision_log_capped,
                            &mut margins,
                            &mut latencies,
                            &mut hot_runtime_latencies,
                        )?;
                        continue;
                    }

                    update_online_bucket(
                        bucket,
                        event,
                        exact_cache_hit,
                        token_cost,
                        cells,
                        DEFAULT_ONLINE_MINER_RESERVOIR_PER_LABEL,
                    )?;
                    if *seen_for_bucket >= train_target
                        && online_miner_bucket_checkpoint_eligible(
                            bucket,
                            ONLINE_MINER_LEARNED_SPLIT_MIN_BROAD_EVENTS,
                        )
                    {
                        compile_online_miner_checkpoint_for_bucket(
                            bucket,
                            OnlineMinerCompileConfig {
                                checkpoint_dir: &checkpoint_dir,
                                cells,
                                min_bucket_events: ONLINE_MINER_LEARNED_SPLIT_MIN_BROAD_EVENTS,
                                base_margin_threshold_micro:
                                    DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO,
                                compiled_after_row: total_rows,
                                max_active_buckets: selected_keys.len().max(1),
                            },
                            &mut checkpoints,
                        )?;
                    }
                }
            }
        }
    }

    latencies.sort_unstable();
    hot_runtime_latencies.sort_unstable();
    let promotion_candidates = online_miner_multi_split_promotion_candidates(&buckets);
    let promotion_candidate_count = promotion_candidates.len();
    let promotion_gate_passed_count = promotion_candidates
        .iter()
        .filter(|candidate| candidate.promotion_gate_passed)
        .count();
    let promotion_gate_failed_count =
        promotion_candidate_count.saturating_sub(promotion_gate_passed_count);
    let targeted_clean =
        online_miner_global_auto_calibrated_stats_for_candidates(&buckets, &promotion_candidates);
    let product_hot_candidates = online_miner_product_hot_budget_candidates_with_buckets(
        Some(&buckets),
        &promotion_candidates,
    );
    let product_hot =
        online_miner_global_auto_calibrated_stats_for_candidates(&buckets, &product_hot_candidates);
    let product_hot_kind_contributions =
        online_miner_product_hot_kind_contributions(&buckets, &product_hot_candidates);
    let compiled_candidate_count = buckets
        .values()
        .filter(|bucket| bucket.active_runtime.is_some())
        .count();
    let future_shadow_events = buckets
        .values()
        .map(|bucket| bucket.future_shadow_events)
        .sum::<usize>();
    let runtime_margin_parity_mismatches = buckets
        .values()
        .map(|bucket| bucket.runtime_margin_parity_mismatches)
        .sum::<usize>();
    let hot_runtime_margin_parity_checks = buckets
        .values()
        .map(|bucket| bucket.hot_runtime_margin_parity_checks)
        .sum::<usize>();
    let hot_runtime_margin_parity_mismatches = buckets
        .values()
        .map(|bucket| bucket.hot_runtime_margin_parity_mismatches)
        .sum::<usize>();
    let hot_runtime_decision_parity_mismatches = buckets
        .values()
        .map(|bucket| bucket.hot_runtime_decision_parity_mismatches)
        .sum::<usize>();
    let raw_shadow_false_accepts = buckets
        .values()
        .map(|bucket| bucket.false_accepts)
        .sum::<usize>();
    let rejected_shadow_bucket_count = buckets
        .values()
        .filter(|bucket| {
            bucket.active_runtime.is_some()
                && online_miner_auto_calibrated_stats(
                    bucket,
                    bucket.safe_accept_margin_threshold_micro,
                )
                .false_accepts
                    > 0
        })
        .count();
    let rejected_shadow_false_accepts = buckets
        .values()
        .map(|bucket| {
            online_miner_auto_calibrated_stats(bucket, bucket.safe_accept_margin_threshold_micro)
                .false_accepts
        })
        .sum::<usize>();
    let false_accepts = product_hot.false_accepts;
    let skipped_selected_candidate_count = selected_candidates
        .len()
        .saturating_sub(compiled_candidate_count);
    let verdict = if selected_candidates.is_empty() {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_SHADOW_V1_WATCH_NO_SELECTED_CANDIDATES"
    } else if product_hot.false_accepts > 0 || targeted_clean.false_accepts > 0 {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_SHADOW_V1_FAIL_FALSE_ACCEPTS"
    } else if promotion_candidate_count == 0 {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_SHADOW_V1_WATCH_NO_RUNTIME_ACCEPTS"
    } else {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_SHADOW_V1_PASS_SHADOW_NWPC_READY"
    };
    let report = OnlineMinerTargetedShadowReport {
        report_kind: "phase_stream_online_miner_targeted_shadow_v1",
        mode: "selected_value_pass_candidates_to_quarantine_nwpc_future_shadow",
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        checkpoint_dir: checkpoint_dir.display().to_string(),
        decision_log_path: decision_log_path.display().to_string(),
        cells,
        requested_top_k,
        train_permille,
        total_rows,
        parsed_events,
        selected_candidate_count: selected_candidates.len(),
        selected_candidates,
        compiled_candidate_count,
        skipped_selected_candidate_count,
        future_shadow_events,
        promotion_candidate_count,
        promotion_gate_passed_count,
        promotion_gate_failed_count,
        targeted_clean_unique_cpu_accepts_over_exact_cache: targeted_clean
            .unique_cpu_accepts_over_exact_cache,
        targeted_clean_nando_cpu_tokens_saved: targeted_clean.nando_cpu_tokens_saved,
        targeted_clean_nando_cpu_cost_saved_microusd: targeted_clean.nando_cpu_cost_saved_microusd,
        targeted_clean_duplicate_accept_rows: targeted_clean.duplicate_accept_rows,
        targeted_clean_false_accepts: targeted_clean.false_accepts,
        product_hot_candidate_profile_count: product_hot_candidates.len(),
        product_hot_unique_cpu_accepts_over_exact_cache: product_hot
            .unique_cpu_accepts_over_exact_cache,
        product_hot_nando_cpu_tokens_saved: product_hot.nando_cpu_tokens_saved,
        product_hot_nando_cpu_cost_saved_microusd: product_hot.nando_cpu_cost_saved_microusd,
        product_hot_duplicate_accept_rows: product_hot.duplicate_accept_rows,
        product_hot_false_accepts: product_hot.false_accepts,
        product_hot_kind_contributions,
        runtime_margin_parity_mismatches,
        hot_runtime_margin_parity_checks,
        hot_runtime_margin_parity_mismatches,
        hot_runtime_decision_parity_mismatches,
        hot_runtime_latency_p50_ns: percentile_u128(&hot_runtime_latencies, 50),
        hot_runtime_latency_p90_ns: percentile_u128(&hot_runtime_latencies, 90),
        hot_runtime_latency_p99_ns: percentile_u128(&hot_runtime_latencies, 99),
        raw_shadow_false_accepts,
        rejected_shadow_bucket_count,
        rejected_shadow_false_accepts,
        false_accepts,
        latency_p50_ns: percentile_u128(&latencies, 50),
        latency_p90_ns: percentile_u128(&latencies, 90),
        latency_p99_ns: percentile_u128(&latencies, 99),
        packages: promotion_candidates,
        product_hot_packages: product_hot_candidates,
        runtime_false_accepts_measured: true,
        local_accept_enabled: false,
        auto_promote_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: serde_json::json!({
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "local_accept_without_verifier_used": false
        }),
        verdict,
        boundary: "targeted shadow only: automatically selected value-pass buckets are stream-accumulated from an early verifier-labeled prefix, compiled into quarantined .nwpc packages, and evaluated on future rows; single-pass stream accumulation, no repeated full-pass learning, no full-stream checkpoint comb, no local_accept, no auto_promote, no market money claim",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_targeted_shadow_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  checkpoint_dir: {}", checkpoint_dir.display());
    println!("  total_rows: {total_rows}");
    println!(
        "  selected_candidate_count: {}",
        report.selected_candidate_count
    );
    println!("  compiled_candidate_count: {compiled_candidate_count}");
    println!("  future_shadow_events: {future_shadow_events}");
    println!(
        "  targeted_clean_unique_cpu_accepts_over_exact_cache: {}",
        report.targeted_clean_unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  product_hot_unique_cpu_accepts_over_exact_cache: {}",
        report.product_hot_unique_cpu_accepts_over_exact_cache
    );
    println!("  raw_shadow_false_accepts: {raw_shadow_false_accepts}");
    println!("  false_accepts: {false_accepts}");
    println!("  local_accept_enabled: false");
    println!("  verdict: {verdict}");
    Ok(())
}

pub(crate) fn run_phase_stream_online_miner_targeted_rejection_drilldown_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_TARGETED_REJECTION_DRILLDOWN_REPORT));
    let value_pass_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_VALUE_PASS_REPORT));
    let targeted_shadow_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_TARGETED_SHADOW_REPORT));
    let promotion_registry_gate_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PROMOTION_REGISTRY_GATE_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let value_pass = read_json_value(&value_pass_report_path)?;
    let targeted = read_json_value(&targeted_shadow_report_path)?;
    let promotion = read_json_value(&promotion_registry_gate_report_path)?;
    let decision_log_path = json_string(&targeted, &["decision_log_path"])
        .map(PathBuf::from)
        .ok_or_else(|| "targeted shadow report missing decision_log_path".to_owned())?;
    let decision_stats = online_miner_targeted_decision_stats_by_bucket(&decision_log_path)?;

    let selected_candidates = value_pass
        .get("selected_product_hot_candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| "value-pass report missing selected_product_hot_candidates".to_owned())?;
    let targeted_selected_keys = targeted
        .get("selected_candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|candidate| json_string(candidate, &["bucket_key"]))
        .collect::<BTreeSet<_>>();
    let packages_by_bucket = online_miner_report_array_by_bucket_key(&targeted, "packages");
    let product_hot_keys = targeted
        .get("product_hot_packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|candidate| json_string(candidate, &["bucket_key"]))
        .collect::<BTreeSet<_>>();
    let registry_copied_keys = promotion
        .get("promoted_packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|package| json_bool(package, &["accepted_for_shadow_registry"]) == Some(true))
        .filter_map(|package| json_string(package, &["bucket_key"]))
        .collect::<BTreeSet<_>>();

    let mut rows = Vec::new();
    let mut reason_counts = BTreeMap::<String, usize>::new();
    let mut compiled_rows = 0usize;
    let mut product_hot_rows = 0usize;
    let mut promotion_passed_rows = 0usize;
    let mut upper_bound_accepts_in_rows = 0usize;
    let mut upper_bound_tokens_in_rows = 0usize;
    let mut clean_accepts_in_rows = 0usize;
    let mut clean_tokens_in_rows = 0usize;
    for (rank_index, candidate) in selected_candidates.iter().enumerate() {
        let bucket_key = json_string(candidate, &["bucket_key"]).unwrap_or_default();
        let bucket_kind = json_string(candidate, &["bucket_kind"]).unwrap_or_default();
        let package = packages_by_bucket.get(&bucket_key);
        let stats = decision_stats.get(&bucket_key).cloned().unwrap_or_default();
        let product_hot_selected = product_hot_keys.contains(&bucket_key);
        let registry_copied = registry_copied_keys.contains(&bucket_key);
        let promotion_gate_passed = package
            .is_some_and(|package| json_bool(package, &["promotion_gate_passed"]) == Some(true));
        let compiled_package_path =
            package.and_then(|package| json_string(package, &["package_path"]));
        let clean_accepts = package
            .and_then(|package| {
                online_miner_json_usize(package, &["unique_cpu_accepts_over_exact_cache"])
            })
            .unwrap_or(0);
        let clean_tokens = package
            .and_then(|package| online_miner_json_usize(package, &["nando_cpu_tokens_saved"]))
            .unwrap_or(0);
        let clean_false_accepts = package
            .and_then(|package| online_miner_json_usize(package, &["false_accepts"]))
            .unwrap_or(usize::MAX);
        let train_safe_threshold = package
            .and_then(|package| json_u64(package, &["safe_accept_margin_threshold_micro"]))
            .and_then(|value| i64::try_from(value).ok());
        let final_threshold = package
            .and_then(|package| json_u64(package, &["auto_calibrated_margin_threshold_micro"]))
            .and_then(|value| i64::try_from(value).ok())
            .or(stats.threshold_micro);
        let upper_bound_accepts =
            online_miner_json_usize(candidate, &["non_exact_positive_events"]).unwrap_or(0);
        let upper_bound_tokens = online_miner_json_usize(candidate, &["total_tokens"]).unwrap_or(0);
        upper_bound_accepts_in_rows =
            upper_bound_accepts_in_rows.saturating_add(upper_bound_accepts);
        upper_bound_tokens_in_rows = upper_bound_tokens_in_rows.saturating_add(upper_bound_tokens);
        clean_accepts_in_rows = clean_accepts_in_rows.saturating_add(clean_accepts);
        clean_tokens_in_rows = clean_tokens_in_rows.saturating_add(clean_tokens);
        compiled_rows += usize::from(package.is_some());
        promotion_passed_rows += usize::from(promotion_gate_passed);
        product_hot_rows += usize::from(product_hot_selected);
        let rejection_reason = online_miner_targeted_rejection_reason(
            targeted_selected_keys.contains(&bucket_key),
            package.is_some(),
            product_hot_selected,
            promotion_gate_passed,
            clean_accepts,
            clean_false_accepts,
            stats.raw_false_accepts,
            stats.future_rows,
            stats.raw_unique_accepts_over_exact_cache,
            registry_copied,
        );
        *reason_counts
            .entry(rejection_reason.to_owned())
            .or_insert(0) += 1;
        let next_split_hint = online_miner_targeted_next_split_hint(&bucket_key, rejection_reason);
        rows.push(serde_json::json!({
            "rank": rank_index + 1,
            "bucket_key": bucket_key,
            "bucket_kind": bucket_kind,
            "events_seen": online_miner_json_usize(candidate, &["events_seen"]).unwrap_or(0),
            "positive_events": online_miner_json_usize(candidate, &["positive_events"]).unwrap_or(0),
            "negative_events": online_miner_json_usize(candidate, &["negative_events"]).unwrap_or(0),
            "non_exact_positive_events": online_miner_json_usize(candidate, &["non_exact_positive_events"]).unwrap_or(0),
            "upper_bound_accepts": upper_bound_accepts,
            "upper_bound_tokens": upper_bound_tokens,
            "compiled": package.is_some(),
            "compiled_package_path": compiled_package_path,
            "future_shadow_events": stats.future_rows,
            "raw_shadow_accepts": stats.raw_shadow_accepts,
            "raw_unique_accepts_over_exact_cache": stats.raw_unique_accepts_over_exact_cache,
            "raw_tokens_saved": stats.raw_tokens_saved,
            "raw_false_accepts": stats.raw_false_accepts,
            "future_false_margin_max": stats.max_false_margin_micro,
            "train_safe_threshold": train_safe_threshold,
            "final_threshold": final_threshold,
            "clean_accepts": clean_accepts,
            "clean_tokens": clean_tokens,
            "clean_false_accepts": clean_false_accepts,
            "promotion_gate_passed": promotion_gate_passed,
            "product_hot_selected": product_hot_selected,
            "registry_copied": registry_copied,
            "exact_cache_hits_in_future_shadow": stats.exact_cache_hits,
            "reference_runtime_parity_mismatches": stats.reference_runtime_parity_mismatches,
            "rejection_reason": rejection_reason,
            "next_split_hint": next_split_hint
        }));
    }

    let upper_bound_accepts = online_miner_json_usize(
        &value_pass,
        &["product_hot_candidate_upper_bound_unique_accepts_over_exact_cache"],
    )
    .unwrap_or(0);
    let upper_bound_tokens = online_miner_json_usize(
        &value_pass,
        &["product_hot_candidate_upper_bound_tokens_saved"],
    )
    .unwrap_or(0);
    let product_hot_accepts = online_miner_json_usize(
        &targeted,
        &["product_hot_unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or(0);
    let product_hot_tokens =
        online_miner_json_usize(&targeted, &["product_hot_nando_cpu_tokens_saved"]).unwrap_or(0);
    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_targeted_rejection_drilldown_v1",
        "mode": "audit_only_value_pass_to_targeted_shadow_gap_explainer",
        "value_pass_report_path": value_pass_report_path,
        "targeted_shadow_report_path": targeted_shadow_report_path,
        "promotion_registry_gate_report_path": promotion_registry_gate_report_path,
        "decision_log_path": decision_log_path,
        "selected_candidate_count": rows.len(),
        "compiled_candidate_count": compiled_rows,
        "promotion_gate_passed_count": promotion_passed_rows,
        "product_hot_candidate_profile_count": product_hot_rows,
        "upper_bound_accepts": upper_bound_accepts,
        "upper_bound_tokens": upper_bound_tokens,
        "targeted_product_hot_accepts": product_hot_accepts,
        "targeted_product_hot_tokens": product_hot_tokens,
        "gap_accepts": upper_bound_accepts.saturating_sub(product_hot_accepts),
        "gap_tokens": upper_bound_tokens.saturating_sub(product_hot_tokens),
        "row_sum_upper_bound_accepts": upper_bound_accepts_in_rows,
        "row_sum_upper_bound_tokens": upper_bound_tokens_in_rows,
        "row_sum_clean_accepts": clean_accepts_in_rows,
        "row_sum_clean_tokens": clean_tokens_in_rows,
        "reason_counts": reason_counts,
        "rows": rows,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        },
        "compile_allowed": false,
        "online_learn_enabled": false,
        "online_shadow_enabled": false,
        "auto_promote_enabled": false,
        "local_accept_enabled": false,
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
        "market_money_claim_allowed": false,
        "verdict": "PHASE_STREAM_ONLINE_MINER_TARGETED_REJECTION_DRILLDOWN_V1_READY",
        "boundary": "audit only: joins value-pass selector, targeted-shadow package report, promotion registry, and targeted decision log to explain the 5711->614 gap; does not compile, score new events, tune thresholds, promote, serve, enable local_accept, or revive legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_targeted_rejection_drilldown_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  selected_candidate_count: {}", selected_candidates.len());
    println!("  compiled_candidate_count: {compiled_rows}");
    println!("  product_hot_candidate_profile_count: {product_hot_rows}");
    println!(
        "  gap_accepts: {}",
        upper_bound_accepts.saturating_sub(product_hot_accepts)
    );
    println!(
        "  gap_tokens: {}",
        upper_bound_tokens.saturating_sub(product_hot_tokens)
    );
    println!("  local_accept_enabled: false");
    Ok(())
}

fn online_miner_report_array_by_bucket_key<'a>(
    report: &'a Value,
    key: &str,
) -> BTreeMap<String, &'a Value> {
    report
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| json_string(row, &["bucket_key"]).map(|bucket| (bucket, row)))
        .collect()
}

fn online_miner_targeted_decision_stats_by_bucket(
    decision_log_path: &Path,
) -> Result<BTreeMap<String, OnlineMinerTargetedDecisionDrilldownStats>, String> {
    let text = std::fs::read_to_string(decision_log_path).map_err(|error| {
        format!(
            "failed to read targeted rejection drilldown decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    let mut stats_by_bucket = BTreeMap::<String, OnlineMinerTargetedDecisionDrilldownStats>::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse targeted rejection drilldown decision log '{}' line {}: {error}",
                decision_log_path.display(),
                line_index + 1
            )
        })?;
        let Some(bucket_key) = json_string(&row, &["bucket_key"]) else {
            continue;
        };
        let margin_micro = json_u64(&row, &["margin_micro"])
            .and_then(|value| i64::try_from(value).ok())
            .or_else(|| row.get("margin_micro").and_then(Value::as_i64))
            .unwrap_or(0);
        let threshold_micro = json_u64(&row, &["margin_threshold_micro"])
            .and_then(|value| i64::try_from(value).ok())
            .or_else(|| row.get("margin_threshold_micro").and_then(Value::as_i64));
        let stats = stats_by_bucket.entry(bucket_key).or_default();
        stats.future_rows = stats.future_rows.saturating_add(1);
        stats.raw_shadow_accepts +=
            usize::from(json_bool(&row, &["local_operator_shadow_decision"]) == Some(true));
        stats.raw_unique_accepts_over_exact_cache +=
            usize::from(json_bool(&row, &["unique_cpu_accept_over_exact_cache"]) == Some(true));
        if json_bool(&row, &["unique_cpu_accept_over_exact_cache"]) == Some(true) {
            stats.raw_tokens_saved = stats.raw_tokens_saved.saturating_add(
                online_miner_json_usize(&row, &["token_cost", "total_tokens"]).unwrap_or(0),
            );
        }
        stats.raw_false_accepts += usize::from(json_bool(&row, &["false_accept"]) == Some(true));
        stats.exact_cache_hits += usize::from(json_bool(&row, &["exact_cache_hit"]) == Some(true));
        stats.reference_runtime_parity_mismatches +=
            usize::from(json_bool(&row, &["reference_runtime_parity_mismatch"]) == Some(true));
        if json_bool(&row, &["verified_safe_accept"]) == Some(false) {
            stats.max_false_margin_micro = Some(
                stats
                    .max_false_margin_micro
                    .map_or(margin_micro, |current| current.max(margin_micro)),
            );
        }
        stats.min_margin_micro = Some(
            stats
                .min_margin_micro
                .map_or(margin_micro, |current| current.min(margin_micro)),
        );
        stats.max_margin_micro = Some(
            stats
                .max_margin_micro
                .map_or(margin_micro, |current| current.max(margin_micro)),
        );
        stats.threshold_micro = stats.threshold_micro.or(threshold_micro);
    }
    Ok(stats_by_bucket)
}

fn online_miner_targeted_rejection_reason(
    selected_by_value_pass: bool,
    compiled: bool,
    product_hot_selected: bool,
    promotion_gate_passed: bool,
    clean_accepts: usize,
    clean_false_accepts: usize,
    raw_false_accepts: usize,
    future_rows: usize,
    raw_unique_accepts_over_exact_cache: usize,
    registry_copied: bool,
) -> &'static str {
    if product_hot_selected && registry_copied {
        return "product_hot_shadow_registry_ready";
    }
    if product_hot_selected {
        return "product_hot_selected_not_registry_copied";
    }
    if !selected_by_value_pass {
        return "not_selected_by_value_pass";
    }
    if raw_false_accepts > 0 || clean_false_accepts > 0 {
        return "actual_false_accept_risk";
    }
    if !compiled && future_rows > 0 && raw_unique_accepts_over_exact_cache > 0 {
        return "scored_but_not_promotion_candidate_after_calibration";
    }
    if !compiled {
        return "selected_but_no_future_runtime_score";
    }
    if clean_accepts == 0 {
        return "no_auto_calibrated_unique_accepts";
    }
    if !promotion_gate_passed {
        return "promotion_gate_failed";
    }
    "product_hot_budget_cap_or_lower_marginal_value"
}

fn online_miner_targeted_next_split_hint(bucket_key: &str, reason: &str) -> &'static str {
    match reason {
        "product_hot_shadow_registry_ready" => "none_current_product_hot_clean",
        "actual_false_accept_risk" if bucket_key.contains("learned_auto_subcenter") => {
            "split broad learned shell/tool family into smaller observable multi-atoms with balanced train/future evidence"
        }
        "selected_but_no_future_runtime_score" => {
            "inspect train prefix balance and generate finer source-neutral request/state/tool split for this bucket"
        }
        "scored_but_not_promotion_candidate_after_calibration" => {
            "inspect train prefix balance and generate finer source-neutral request/state/tool split for this bucket"
        }
        "actual_false_accept_risk" => {
            "run false-accept split audit and add source-neutral atom split; do not lower threshold"
        }
        "no_auto_calibrated_unique_accepts" => {
            "keep bucket in shadow; needs separability split or more future safe evidence before promotion"
        }
        "product_hot_budget_cap_or_lower_marginal_value" => {
            "budget-cap stop; only widen product-hot cap after marginal-value report and provider evidence"
        }
        "promotion_gate_failed" => {
            "inspect package reload/parity/threshold evidence before any promotion"
        }
        _ => "no_safe_next_split_hint_available",
    }
}

pub(crate) fn run_phase_stream_online_miner_promotion_registry_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PROMOTION_REGISTRY_GATE_REPORT));
    let shadow_registry_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PROMOTION_SHADOW_REGISTRY_DIR));
    let source_registry_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PROMOTION_REGISTRY));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let source_registry = read_online_miner_json_value(&source_registry_path)?;
    let source_registry_kind =
        json_string(&source_registry, &["registry_kind"]).unwrap_or_default();
    let source_mode = json_string(&source_registry, &["mode"]).unwrap_or_default();
    let input_candidate_count =
        online_miner_json_usize(&source_registry, &["promotion_candidate_count"]).unwrap_or(0);
    let promotion_gate_passed_count =
        online_miner_json_usize(&source_registry, &["promotion_gate_passed_count"]).unwrap_or(0);
    let promotion_gate_failed_count =
        online_miner_json_usize(&source_registry, &["promotion_gate_failed_count"])
            .unwrap_or(usize::MAX);
    let global_unique_cpu_accepts_over_exact_cache = online_miner_json_usize(
        &source_registry,
        &["global_unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or(0);
    let duplicate_accept_rows =
        online_miner_json_usize(&source_registry, &["duplicate_accept_rows"]).unwrap_or(0);
    let nando_cpu_tokens_saved =
        online_miner_json_usize(&source_registry, &["nando_cpu_tokens_saved"]).unwrap_or(0);
    let nando_cpu_cost_saved_microusd =
        json_u64(&source_registry, &["nando_cpu_cost_saved_microusd"]).unwrap_or(0);
    let false_accepts =
        online_miner_json_usize(&source_registry, &["false_accepts"]).unwrap_or(usize::MAX);
    let input_local_accept_enabled =
        json_bool(&source_registry, &["local_accept_enabled"]).unwrap_or(true);
    let input_auto_promote_enabled =
        json_bool(&source_registry, &["auto_promote_enabled"]).unwrap_or(true);
    let input_market_money_claim_allowed =
        json_bool(&source_registry, &["market_money_claim_allowed"]).unwrap_or(true);
    let forbidden_flags_clear = source_registry
        .get("forbidden_flags")
        .is_some_and(online_miner_forbidden_flags_all_false);
    let registry_global_gate_clear = source_registry_kind
        == "phase_stream_product_hot_promotion_registry_v1"
        && source_mode == "shadow_quarantine_review_only"
        && input_candidate_count > 0
        && promotion_gate_passed_count == input_candidate_count
        && promotion_gate_failed_count == 0
        && global_unique_cpu_accepts_over_exact_cache > 0
        && false_accepts == 0
        && !input_local_accept_enabled
        && !input_auto_promote_enabled
        && !input_market_money_claim_allowed
        && forbidden_flags_clear;

    let mut promoted_packages = Vec::new();
    for candidate in source_registry
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        promoted_packages.push(online_miner_audit_promotion_registry_candidate(
            candidate,
            &shadow_registry_dir,
            registry_global_gate_clear,
        )?);
    }
    let promoted_candidate_count = promoted_packages
        .iter()
        .filter(|package| package.accepted_for_shadow_registry)
        .count();
    let blocked_candidate_count = promoted_packages
        .len()
        .saturating_sub(promoted_candidate_count);
    let verdict = if !registry_global_gate_clear {
        "PHASE_STREAM_ONLINE_MINER_PROMOTION_REGISTRY_GATE_V1_BLOCKED_INPUT_GATE"
    } else if promoted_candidate_count > 0 && blocked_candidate_count == 0 {
        "PHASE_STREAM_ONLINE_MINER_PROMOTION_REGISTRY_GATE_V1_PASS_SHADOW_REGISTRY_READY"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PROMOTION_REGISTRY_GATE_V1_WATCH_BLOCKED_PACKAGE"
    };
    let report = OnlineMinerPromotionRegistryGateReport {
        report_kind: "phase_stream_online_miner_promotion_registry_gate_v1",
        mode: "shadow_registry_review_only",
        source_registry_path: source_registry_path.display().to_string(),
        shadow_registry_dir: shadow_registry_dir.display().to_string(),
        source_registry_kind,
        source_mode,
        input_candidate_count,
        promoted_candidate_count,
        blocked_candidate_count,
        promotion_gate_passed_count,
        promotion_gate_failed_count,
        global_unique_cpu_accepts_over_exact_cache,
        duplicate_accept_rows,
        nando_cpu_tokens_saved,
        nando_cpu_cost_saved_microusd,
        false_accepts,
        registry_global_gate_clear,
        shadow_registry_mutated: promoted_candidate_count > 0,
        promoted_packages,
        local_accept_enabled: false,
        auto_promote_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: serde_json::json!({
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "local_accept_without_verifier_used": false
        }),
        verdict,
        boundary: "shadow promotion registry gate only: validates verifier-bound online-miner .nwpc candidates from a quarantine registry and copies accepted packages into a shadow registry; it never mutates serving registry, enables local_accept, auto-promotes, claims market money, or revives legacy nwrb/role-binding paths",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_promotion_registry_gate_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  shadow_registry_dir: {}", shadow_registry_dir.display());
    println!("  promoted_candidate_count: {promoted_candidate_count}");
    println!("  blocked_candidate_count: {blocked_candidate_count}");
    println!(
        "  global_unique_cpu_accepts_over_exact_cache: {global_unique_cpu_accepts_over_exact_cache}"
    );
    println!("  local_accept_enabled: false");
    println!("  auto_promote_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn parse_optional_usize(value: Option<String>, name: &str) -> Result<Option<usize>, String> {
    value
        .map(|raw| {
            raw.parse::<usize>()
                .map_err(|error| format!("invalid {name} '{raw}': {error}"))
        })
        .transpose()
}

fn parse_optional_i64(value: Option<String>, name: &str) -> Result<Option<i64>, String> {
    value
        .map(|raw| {
            raw.parse::<i64>()
                .map_err(|error| format!("invalid {name} '{raw}': {error}"))
        })
        .transpose()
}

pub(super) fn online_miner_event_bucket_specs(
    action_family: &str,
    request_atoms: &[String],
    state_atoms: &[String],
    tool_atoms: &[String],
    route_hint_atoms: &[String],
    learned_split_atoms: &[String],
) -> Vec<(&'static str, String)> {
    let mut specs = Vec::<(&'static str, String)>::new();
    let mut seen = BTreeSet::<String>::new();
    let broad_key = format!("{action_family}::broad_action");
    if seen.insert(broad_key.clone()) {
        specs.push(("broad_action", broad_key));
    }
    let state_action_key = phase_atom_state_action_bucket_key(
        action_family,
        request_atoms,
        state_atoms,
        tool_atoms,
        route_hint_atoms,
    );
    if seen.insert(state_action_key.clone()) {
        specs.push(("state_action_signature", state_action_key));
    }
    for split_atom in
        online_miner_auto_subcenter_atoms(request_atoms, state_atoms, tool_atoms, route_hint_atoms)
    {
        let key = format!("{action_family}::auto_subcenter:{split_atom}");
        if seen.insert(key.clone()) {
            specs.push(("auto_subcenter", key));
        }
    }
    for hidden_atom in online_miner_hidden_state_atoms(request_atoms, state_atoms, tool_atoms) {
        let key = format!("{action_family}::hidden_state:{hidden_atom}");
        if seen.insert(key.clone()) {
            specs.push(("hidden_state_split", key));
        }
    }
    let row_split_atoms =
        online_miner_event_split_atoms(request_atoms, state_atoms, tool_atoms, route_hint_atoms)
            .into_iter()
            .collect::<BTreeSet<_>>();
    for split_atom in learned_split_atoms {
        if !row_split_atoms.contains(split_atom) {
            continue;
        }
        let key = format!("{action_family}::learned_auto_subcenter:{split_atom}");
        if seen.insert(key.clone()) {
            specs.push(("learned_auto_subcenter", key));
        }
    }
    specs
}

fn online_miner_event_split_atoms(
    request_atoms: &[String],
    state_atoms: &[String],
    tool_atoms: &[String],
    route_hint_atoms: &[String],
) -> Vec<String> {
    let mut atoms = request_atoms
        .iter()
        .chain(state_atoms)
        .chain(tool_atoms)
        .chain(route_hint_atoms)
        .filter(|atom| online_miner_source_neutral_split_atom_allowed(atom))
        .cloned()
        .collect::<BTreeSet<_>>();
    for compound_atom in
        online_miner_auto_subcenter_atoms(request_atoms, state_atoms, tool_atoms, route_hint_atoms)
    {
        if online_miner_learned_split_atom_allowed(&compound_atom) {
            atoms.insert(compound_atom);
        }
    }
    for hidden_atom in online_miner_hidden_state_atoms(request_atoms, state_atoms, tool_atoms) {
        if online_miner_learned_split_atom_allowed(&hidden_atom) {
            atoms.insert(hidden_atom);
        }
    }
    atoms.into_iter().collect()
}

fn observe_online_miner_split_atoms(
    stats_by_action: &mut BTreeMap<String, BTreeMap<String, OnlineMinerSplitAtomStats>>,
    action_family: &str,
    atoms: &[String],
    verified_safe_accept: bool,
    exact_cache_hit: bool,
    token_cost: GenericTokenCost,
) {
    let stats_by_atom = stats_by_action.entry(action_family.to_owned()).or_default();
    for atom in atoms {
        stats_by_atom.entry(atom.clone()).or_default().observe(
            verified_safe_accept,
            exact_cache_hit,
            token_cost,
        );
    }
}

fn online_miner_learned_split_atoms_for_action(
    stats_by_action: &BTreeMap<String, BTreeMap<String, OnlineMinerSplitAtomStats>>,
    action_family: &str,
    limit: usize,
    split_pressure: bool,
) -> Vec<String> {
    if !split_pressure {
        return Vec::new();
    }
    let Some(stats_by_atom) = stats_by_action.get(action_family) else {
        return Vec::new();
    };
    let mut candidates = stats_by_atom
        .iter()
        .filter(|(_, stats)| stats.eligible_for_split())
        .filter(|(atom, _)| online_miner_learned_split_atom_allowed(atom))
        .map(|(atom, stats)| (atom.clone(), stats.value_score()))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    candidates
        .into_iter()
        .map(|(atom, _)| atom)
        .take(limit)
        .collect()
}

fn online_miner_action_family_has_candidate_split_atoms(
    stats_by_action: &BTreeMap<String, BTreeMap<String, OnlineMinerSplitAtomStats>>,
    action_family: &str,
) -> bool {
    stats_by_action
        .get(action_family)
        .is_some_and(|stats_by_atom| {
            stats_by_atom.iter().any(|(atom, stats)| {
                stats.eligible_for_split() && online_miner_learned_split_atom_allowed(atom)
            })
        })
}

fn online_miner_action_family_has_learned_split_pressure(
    buckets: &BTreeMap<String, OnlineMinerBucketState>,
    action_family: &str,
) -> bool {
    let broad_key = format!("{action_family}::broad_action");
    buckets
        .get(&broad_key)
        .is_some_and(online_miner_bucket_has_learned_split_pressure)
}

fn online_miner_bucket_has_learned_split_pressure(bucket: &OnlineMinerBucketState) -> bool {
    if bucket.bucket_kind != "broad_action"
        || bucket.events_seen < ONLINE_MINER_LEARNED_SPLIT_MIN_BROAD_EVENTS
        || bucket.positive_events < ONLINE_MINER_LEARNED_SPLIT_MIN_BROAD_LABELS
        || bucket.negative_events < ONLINE_MINER_LEARNED_SPLIT_MIN_BROAD_LABELS
        || bucket.non_exact_positive_events == 0
    {
        return false;
    }
    if bucket.false_accepts > 0 || bucket.wrong_wins > 0 {
        return true;
    }
    let stats =
        online_miner_auto_calibrated_stats(bucket, bucket.safe_accept_margin_threshold_micro);
    stats.false_accepts > 0
        || (bucket.future_decisions.len() >= 4
            && stats.shadow_events_after_calibration > 0
            && stats.unique_cpu_accepts_over_exact_cache == 0)
}

fn online_miner_learned_split_atom_counts(
    stats_by_action: &BTreeMap<String, BTreeMap<String, OnlineMinerSplitAtomStats>>,
) -> OnlineMinerLearnedSplitAtomCounts {
    let mut counts = OnlineMinerLearnedSplitAtomCounts::default();
    for atom in stats_by_action
        .values()
        .flat_map(|stats_by_atom| stats_by_atom.iter())
        .filter(|(atom, stats)| {
            stats.eligible_for_split() && online_miner_learned_split_atom_allowed(atom)
        })
        .map(|(atom, _)| atom)
    {
        counts.observe(atom);
    }
    counts
}

fn online_miner_bucket_key_count(
    buckets: &BTreeMap<String, OnlineMinerBucketState>,
    bucket_kind: &str,
    predicate: impl Fn(&str) -> bool,
) -> usize {
    buckets
        .values()
        .filter(|bucket| bucket.bucket_kind == bucket_kind && predicate(&bucket.bucket_key))
        .count()
}

fn online_miner_hidden_state_atom_count(
    stats_by_action: &BTreeMap<String, BTreeMap<String, OnlineMinerSplitAtomStats>>,
) -> usize {
    stats_by_action
        .values()
        .flat_map(|stats_by_atom| stats_by_atom.keys())
        .filter(|atom| online_miner_hidden_state_atom_allowed(atom))
        .collect::<BTreeSet<_>>()
        .len()
}

fn online_miner_hidden_state_atoms(
    request_atoms: &[String],
    state_atoms: &[String],
    tool_atoms: &[String],
) -> Vec<String> {
    let request_basis =
        online_miner_ranked_subcenter_basis(request_atoms, ONLINE_MINER_HIDDEN_STATE_BASIS_LIMIT);
    let state_basis =
        online_miner_ranked_subcenter_basis(state_atoms, ONLINE_MINER_HIDDEN_STATE_BASIS_LIMIT);
    let tool_basis =
        online_miner_ranked_subcenter_basis(tool_atoms, ONLINE_MINER_HIDDEN_STATE_BASIS_LIMIT);
    let mut candidates = Vec::<(u128, String)>::new();
    let mut seen_candidates = BTreeSet::<String>::new();

    online_miner_push_hidden_state_cross_layer_candidates(
        &mut candidates,
        &mut seen_candidates,
        "request_state",
        &request_basis,
        &state_basis,
    );
    online_miner_push_hidden_state_cross_layer_candidates(
        &mut candidates,
        &mut seen_candidates,
        "state_tool",
        &state_basis,
        &tool_basis,
    );
    online_miner_push_hidden_state_cross_layer_candidates(
        &mut candidates,
        &mut seen_candidates,
        "request_tool",
        &request_basis,
        &tool_basis,
    );

    let mut emitted = 0usize;
    'outer: for request_atom in &request_basis {
        for state_atom in &state_basis {
            for tool_atom in &tool_basis {
                if emitted >= ONLINE_MINER_HIDDEN_STATE_MAX_CANDIDATES {
                    break 'outer;
                }
                emitted += usize::from(online_miner_push_hidden_state_candidate(
                    &mut candidates,
                    &mut seen_candidates,
                    "request_state_tool",
                    &[
                        request_atom.as_str(),
                        state_atom.as_str(),
                        tool_atom.as_str(),
                    ],
                ));
            }
        }
    }

    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    candidates
        .into_iter()
        .map(|(_, atom)| atom)
        .take(ONLINE_MINER_MAX_AUTOSUBCENTER_BUCKETS_PER_EVENT)
        .collect()
}

fn online_miner_push_hidden_state_cross_layer_candidates(
    candidates: &mut Vec<(u128, String)>,
    seen_candidates: &mut BTreeSet<String>,
    transition_kind: &str,
    left_basis: &[String],
    right_basis: &[String],
) {
    let mut emitted = 0usize;
    for left in left_basis {
        for right in right_basis {
            if emitted >= ONLINE_MINER_HIDDEN_STATE_MAX_CANDIDATES {
                return;
            }
            emitted += usize::from(online_miner_push_hidden_state_candidate(
                candidates,
                seen_candidates,
                transition_kind,
                &[left.as_str(), right.as_str()],
            ));
        }
    }
}

fn online_miner_push_hidden_state_candidate(
    candidates: &mut Vec<(u128, String)>,
    seen_candidates: &mut BTreeSet<String>,
    transition_kind: &str,
    parts: &[&str],
) -> bool {
    let Some(atom) = online_miner_hidden_state_atom(transition_kind, parts) else {
        return false;
    };
    let score = parts
        .iter()
        .map(|part| online_miner_subcenter_atom_score(part))
        .fold(0u128, u128::saturating_add)
        .saturating_mul(128)
        .saturating_add(stable_fingerprint(["online_miner_hidden_state", &atom]) as u128);
    online_miner_push_subcenter_candidate(candidates, seen_candidates, score, atom)
}

fn online_miner_hidden_state_atom(transition_kind: &str, parts: &[&str]) -> Option<String> {
    if parts.len() < 2
        || !transition_kind
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch == '_')
        || !parts
            .iter()
            .all(|part| online_miner_source_neutral_visible_atom_allowed(part))
    {
        return None;
    }
    let compact_parts = parts
        .iter()
        .map(|part| online_miner_hidden_state_part(part))
        .collect::<Option<Vec<_>>>()?;
    let atom = format!("hidden_state:{transition_kind}:{}", compact_parts.join("+"));
    online_miner_hidden_state_atom_allowed(&atom).then_some(atom)
}

fn online_miner_hidden_state_part(atom: &str) -> Option<String> {
    let (family, _) = atom.split_once(':')?;
    if family.is_empty()
        || !family
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch == '_' || ch.is_ascii_digit())
    {
        return None;
    }
    let fingerprint = stable_fingerprint(["online_miner_hidden_state_part", atom]) & 0xffff;
    Some(format!("{family}_{fingerprint:04x}"))
}

fn online_miner_hidden_state_atom_allowed(atom: &str) -> bool {
    atom.starts_with("hidden_state:")
        && !online_miner_hidden_state_bucket_has_forbidden_source_leak(atom)
        && atom.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, ':' | '_' | '+')
        })
}

fn online_miner_hidden_state_bucket_has_forbidden_source_leak(text: &str) -> bool {
    text.contains("route_hint")
        || text.contains("route_key")
        || text.contains("request_route_family")
        || text.contains("_cwd_kind")
        || text.contains("tool_mention")
        || text.contains("profile_id")
        || text.contains("proof_rule_id")
        || text.contains("target_id")
        || text.contains("output_hash")
        || text.contains("provider_request_id")
        || text.contains("provider_response_id")
        || text.contains("exact_cache")
        || text.contains("billing_evidence")
}

fn online_miner_auto_subcenter_atoms(
    request_atoms: &[String],
    state_atoms: &[String],
    tool_atoms: &[String],
    _route_hint_atoms: &[String],
) -> Vec<String> {
    let request_basis = online_miner_ranked_subcenter_basis(
        request_atoms,
        ONLINE_MINER_AUTOSUBCENTER_REQUEST_BASIS_LIMIT,
    );
    let state_basis = online_miner_ranked_subcenter_basis(
        state_atoms,
        ONLINE_MINER_AUTOSUBCENTER_STATE_BASIS_LIMIT,
    );
    let tool_basis = online_miner_ranked_subcenter_basis(
        tool_atoms,
        ONLINE_MINER_AUTOSUBCENTER_TOOL_BASIS_LIMIT,
    );

    let mut singles = request_basis
        .iter()
        .chain(state_basis.iter())
        .chain(tool_basis.iter())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    online_miner_sort_subcenter_basis(&mut singles);
    singles.truncate(ONLINE_MINER_AUTOSUBCENTER_SINGLE_BASIS_LIMIT);

    let mut candidates = Vec::<(u128, String)>::new();
    let mut seen_candidates = BTreeSet::<String>::new();
    for atom in &singles {
        online_miner_push_subcenter_candidate(
            &mut candidates,
            &mut seen_candidates,
            online_miner_subcenter_atom_score(atom),
            atom.clone(),
        );
    }
    for (left_index, left) in singles.iter().enumerate().take(8) {
        for right in singles.iter().skip(left_index + 1).take(8) {
            online_miner_push_compound_subcenter_candidate(
                &mut candidates,
                &mut seen_candidates,
                "multi2",
                4,
                &[left.as_str(), right.as_str()],
            );
        }
    }
    online_miner_push_generic_compound_subcenters(
        &mut candidates,
        &mut seen_candidates,
        &singles,
        3,
        ONLINE_MINER_AUTOSUBCENTER_MAX_MULTI3_CANDIDATES,
    );
    online_miner_push_generic_compound_subcenters(
        &mut candidates,
        &mut seen_candidates,
        &singles,
        4,
        ONLINE_MINER_AUTOSUBCENTER_MAX_MULTI4_CANDIDATES,
    );

    let mut detail_basis = tool_basis.clone();
    if detail_basis.is_empty() {
        detail_basis = singles.clone();
    }
    online_miner_sort_subcenter_basis(&mut detail_basis);
    detail_basis.truncate(8);

    let mut multi3_candidates = 0usize;
    for request_atom in &request_basis {
        for state_atom in &state_basis {
            for detail_atom in &detail_basis {
                if multi3_candidates >= ONLINE_MINER_AUTOSUBCENTER_MAX_MULTI3_CANDIDATES {
                    break;
                }
                let inserted = online_miner_push_compound_subcenter_candidate(
                    &mut candidates,
                    &mut seen_candidates,
                    "multi3",
                    16,
                    &[
                        request_atom.as_str(),
                        state_atom.as_str(),
                        detail_atom.as_str(),
                    ],
                );
                multi3_candidates += usize::from(inserted);
            }
            if multi3_candidates >= ONLINE_MINER_AUTOSUBCENTER_MAX_MULTI3_CANDIDATES {
                break;
            }
        }
        if multi3_candidates >= ONLINE_MINER_AUTOSUBCENTER_MAX_MULTI3_CANDIDATES {
            break;
        }
    }

    let mut extra_basis = request_basis
        .iter()
        .chain(state_basis.iter())
        .chain(tool_basis.iter())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    online_miner_sort_subcenter_basis(&mut extra_basis);
    extra_basis.truncate(8);

    let mut multi4_candidates = 0usize;
    for request_atom in &request_basis {
        for state_atom in &state_basis {
            for tool_atom in &tool_basis {
                for extra_atom in &extra_basis {
                    if multi4_candidates >= ONLINE_MINER_AUTOSUBCENTER_MAX_MULTI4_CANDIDATES {
                        break;
                    }
                    let inserted = online_miner_push_compound_subcenter_candidate(
                        &mut candidates,
                        &mut seen_candidates,
                        "multi4",
                        64,
                        &[
                            request_atom.as_str(),
                            state_atom.as_str(),
                            tool_atom.as_str(),
                            extra_atom.as_str(),
                        ],
                    );
                    multi4_candidates += usize::from(inserted);
                }
                if multi4_candidates >= ONLINE_MINER_AUTOSUBCENTER_MAX_MULTI4_CANDIDATES {
                    break;
                }
            }
            if multi4_candidates >= ONLINE_MINER_AUTOSUBCENTER_MAX_MULTI4_CANDIDATES {
                break;
            }
        }
        if multi4_candidates >= ONLINE_MINER_AUTOSUBCENTER_MAX_MULTI4_CANDIDATES {
            break;
        }
    }
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    candidates
        .into_iter()
        .map(|(_, atom)| atom)
        .take(ONLINE_MINER_MAX_AUTOSUBCENTER_BUCKETS_PER_EVENT)
        .collect()
}

fn online_miner_ranked_subcenter_basis(atoms: &[String], limit: usize) -> Vec<String> {
    let mut basis = atoms
        .iter()
        .filter(|atom| online_miner_source_neutral_split_atom_allowed(atom))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    online_miner_sort_subcenter_basis(&mut basis);
    basis.truncate(limit);
    basis
}

fn online_miner_sort_subcenter_basis(atoms: &mut [String]) {
    atoms.sort_by(|left, right| {
        online_miner_subcenter_atom_score(right)
            .cmp(&online_miner_subcenter_atom_score(left))
            .then_with(|| left.cmp(right))
    });
}

fn online_miner_push_subcenter_candidate(
    candidates: &mut Vec<(u128, String)>,
    seen_candidates: &mut BTreeSet<String>,
    score: u128,
    atom: String,
) -> bool {
    if seen_candidates.insert(atom.clone()) {
        candidates.push((score, atom));
        true
    } else {
        false
    }
}

fn online_miner_push_compound_subcenter_candidate(
    candidates: &mut Vec<(u128, String)>,
    seen_candidates: &mut BTreeSet<String>,
    prefix: &str,
    weight: u128,
    parts: &[&str],
) -> bool {
    if !online_miner_compound_subcenter_parts_allowed(parts) {
        return false;
    }
    let atom = format!("{prefix}:{}", parts.join("|"));
    let score = parts
        .iter()
        .map(|part| online_miner_subcenter_atom_score(part))
        .fold(0u128, u128::saturating_add)
        .saturating_mul(weight);
    online_miner_push_subcenter_candidate(candidates, seen_candidates, score, atom)
}

fn online_miner_push_generic_compound_subcenters(
    candidates: &mut Vec<(u128, String)>,
    seen_candidates: &mut BTreeSet<String>,
    basis: &[String],
    arity: usize,
    limit: usize,
) {
    if !(3..=4).contains(&arity) || basis.len() < arity {
        return;
    }
    let prefix = if arity == 3 { "multi3" } else { "multi4" };
    let weight = if arity == 3 { 16 } else { 64 };
    let mut emitted = 0usize;
    let bounded = basis.iter().take(8).collect::<Vec<_>>();
    for first in 0..bounded.len() {
        for second in first + 1..bounded.len() {
            for third in second + 1..bounded.len() {
                if arity == 3 {
                    let inserted = online_miner_push_compound_subcenter_candidate(
                        candidates,
                        seen_candidates,
                        prefix,
                        weight,
                        &[
                            bounded[first].as_str(),
                            bounded[second].as_str(),
                            bounded[third].as_str(),
                        ],
                    );
                    emitted += usize::from(inserted);
                    if emitted >= limit {
                        return;
                    }
                    continue;
                }
                for fourth in third + 1..bounded.len() {
                    let inserted = online_miner_push_compound_subcenter_candidate(
                        candidates,
                        seen_candidates,
                        prefix,
                        weight,
                        &[
                            bounded[first].as_str(),
                            bounded[second].as_str(),
                            bounded[third].as_str(),
                            bounded[fourth].as_str(),
                        ],
                    );
                    emitted += usize::from(inserted);
                    if emitted >= limit {
                        return;
                    }
                }
            }
        }
    }
}

fn online_miner_compound_subcenter_parts_allowed(parts: &[&str]) -> bool {
    let mut atoms = BTreeSet::new();
    let mut families = BTreeSet::new();
    for part in parts {
        if !atoms.insert(*part) {
            return false;
        }
        families.insert(online_miner_split_atom_family(part));
    }
    families.len() >= parts.len().min(3)
}

fn online_miner_learned_split_atom_allowed(atom: &str) -> bool {
    if let Some((prefix, rest)) = atom.split_once(':')
        && matches!(prefix, "multi2" | "multi3" | "multi4")
    {
        let parts = rest.split('|').collect::<Vec<_>>();
        let expected_arity = match prefix {
            "multi2" => 2,
            "multi3" => 3,
            "multi4" => 4,
            _ => unreachable!(),
        };
        return parts.len() == expected_arity
            && parts
                .iter()
                .all(|part| online_miner_source_neutral_split_atom_allowed(part))
            && online_miner_compound_subcenter_parts_allowed(&parts);
    }
    online_miner_source_neutral_split_atom_allowed(atom)
}

fn online_miner_source_neutral_split_atom_allowed(atom: &str) -> bool {
    if online_miner_hidden_state_atom_allowed(atom) {
        return true;
    }
    online_miner_source_neutral_visible_atom_allowed(atom)
}

fn online_miner_source_neutral_visible_atom_allowed(atom: &str) -> bool {
    phase_atom_bucket_selector(atom)
        && !atom.starts_with("route_hint:")
        && !atom.starts_with("route_key:")
        && !atom.starts_with("request_route_family:")
        && !atom.starts_with("request_char_band:")
        && !atom.starts_with("request_line_count_band:")
        && !atom.starts_with("request_word_count_band:")
        && !atom.starts_with("request_has_question:")
        && !atom.starts_with("request_has_json_shape:")
        && !atom.starts_with("request_has_code_fence:")
        && !atom.starts_with("request_has_shadow_request:")
        && !atom.starts_with("request_command_arg_band:")
        && !atom.starts_with("state_session_turn_band:")
        && !atom.starts_with("shadow_active_fringe_len_band:")
        && !atom.starts_with("shadow_slot_count_band:")
        && !atom.contains("_cwd_kind:")
        && !atom.starts_with("tool_mention:")
        && !atom.starts_with("profile_id:")
        && !atom.starts_with("proof_rule_id:")
        && !atom.starts_with("target_id:")
        && !atom.starts_with("output_hash")
        && !atom.contains("provider_request_id")
        && !atom.contains("provider_response_id")
        && !atom.contains("billing_evidence")
}

fn online_miner_subcenter_atom_score(atom: &str) -> u128 {
    let length_score = (atom.len() as u128).min(512);
    let hash_score = stable_fingerprint(["online_miner_auto_subcenter", atom]) as u128;
    length_score
        .saturating_mul(1_000_000_000_000)
        .saturating_add(hash_score)
}

fn online_miner_split_atom_family(atom: &str) -> &str {
    atom.split_once(':').map_or(atom, |(family, _)| family)
}

fn score_future_event_before_update(
    bucket: &mut OnlineMinerBucketState,
    event: &PhaseAtomBinaryEvent,
    exact_cache_hit: bool,
    denominator_row_index: usize,
    decision_log: &mut std::fs::File,
    decision_log_rows_written: &mut usize,
    decision_log_limit: Option<usize>,
    decision_log_capped: &mut bool,
    margins: &mut Vec<i64>,
    latencies: &mut Vec<u128>,
    hot_runtime_latencies: &mut Vec<u128>,
) -> Result<(), String> {
    let (Some(reference_runtime), Some(offload_runtime), Some(hot_runtime)) = (
        &bucket.active_reference_runtime,
        &bucket.active_runtime,
        &bucket.active_hot_runtime,
    ) else {
        return Ok(());
    };
    if bucket.last_compiled_after_row >= denominator_row_index {
        return Ok(());
    }
    let safe_accept_vec = phase_atom_binary_event_vector_for_task(
        event,
        true,
        offload_runtime.cells(),
        &bucket.task_name,
    );
    let zero = vec![nando_core::PhaseCenterCell::default(); offload_runtime.cells()];
    let task = PhaseCenterEvalTask {
        center_index: 0,
        correct_vec: safe_accept_vec.into_boxed_slice(),
        wrong_vec: zero.into_boxed_slice(),
    };
    let started = Instant::now();
    let decision = offload_runtime
        .offload_decision(&task)
        .map_err(|error| format!("online miner offload decision error: {error:?}"))?;
    latencies.push(started.elapsed().as_nanos());
    let reference_micro = margin_to_micro(
        reference_runtime
            .margin(&task)
            .map_err(|error| format!("online miner reference margin error: {error:?}"))?,
    )?;
    let runtime_micro = margin_to_micro(
        offload_runtime
            .runtime()
            .margin(&task)
            .map_err(|error| format!("online miner runtime margin error: {error:?}"))?,
    )?;
    let hot_started = Instant::now();
    let hot_decision = hot_runtime
        .score_profile(0, task.correct_vec.as_ref())
        .map_err(|error| format!("online miner hot runtime score error: {error:?}"))?;
    hot_runtime_latencies.push(hot_started.elapsed().as_nanos());
    let hot_runtime_micro = hot_decision.margin_micro;
    let parity_mismatch = reference_micro != runtime_micro;
    let hot_margin_parity_mismatch =
        reference_micro != hot_runtime_micro || runtime_micro != hot_runtime_micro;
    let signed_margin = if event.verified_safe_accept {
        decision.margin_micro
    } else {
        decision.margin_micro.saturating_neg()
    };
    margins.push(signed_margin);
    let local_operator = decision.is_local_operator();
    let hot_decision_parity_mismatch = local_operator != hot_decision.local_operator;
    let false_accept = local_operator && !event.verified_safe_accept;
    let unique_accept = local_operator && event.verified_safe_accept && !exact_cache_hit;
    let classifier_wrong = if event.verified_safe_accept {
        decision.margin_micro <= 0
    } else {
        decision.margin_micro >= 0
    };
    bucket
        .future_decisions
        .push(OnlineMinerFutureDecisionSample {
            request_fingerprint: event.request_fingerprint.clone(),
            margin_micro: decision.margin_micro,
            verified_safe_accept: event.verified_safe_accept,
            exact_cache_hit,
            total_tokens: event.token_cost.total_tokens,
            total_cost_microusd: event.token_cost.total_cost_microusd,
        });
    bucket.future_shadow_events += 1;
    bucket.wrong_wins += usize::from(classifier_wrong);
    bucket.runtime_margin_parity_mismatches += usize::from(parity_mismatch);
    bucket.hot_runtime_margin_parity_checks =
        bucket.hot_runtime_margin_parity_checks.saturating_add(1);
    bucket.hot_runtime_margin_parity_mismatches += usize::from(hot_margin_parity_mismatch);
    bucket.hot_runtime_decision_parity_mismatches += usize::from(hot_decision_parity_mismatch);
    bucket.false_accepts += usize::from(false_accept);
    if local_operator && event.verified_safe_accept {
        bucket.local_operator_shadow_decisions += 1;
        if unique_accept {
            bucket.unique_cpu_accepts_over_exact_cache += 1;
            bucket.nando_cpu_tokens_saved = bucket
                .nando_cpu_tokens_saved
                .saturating_add(event.token_cost.total_tokens);
            bucket.nando_cpu_cost_saved_microusd = bucket
                .nando_cpu_cost_saved_microusd
                .saturating_add(event.token_cost.total_cost_microusd);
        }
    } else {
        bucket.fallback_shadow_decisions += 1;
    }
    let decision_row = serde_json::json!({
        "schema_version": "phase_stream_online_miner_daemon_decision_v1",
        "bucket_key": bucket.bucket_key,
        "bucket_kind": bucket.bucket_kind,
        "action_family_atom": bucket.action_family_atom,
        "task_name": bucket.task_name,
        "denominator_row_index": denominator_row_index,
        "compiled_after_row": bucket.last_compiled_after_row,
        "future_only_shadow_scoring": bucket.last_compiled_after_row < denominator_row_index,
        "request_fingerprint": event.request_fingerprint,
        "external_provider_correlation_keys": event.external_provider_correlation_keys.clone(),
        "provider_correlation_ready": !event.external_provider_correlation_keys.is_empty(),
        "exact_cache_key": event.exact_cache_key,
        "exact_cache_hit": exact_cache_hit,
        "verified_safe_accept": event.verified_safe_accept,
        "margin_micro": decision.margin_micro,
        "margin_threshold_micro": decision.margin_threshold_micro,
        "reference_runtime_parity_mismatch": parity_mismatch,
        "hot_runtime_margin_micro": hot_runtime_micro,
        "hot_runtime_margin_parity_mismatch": hot_margin_parity_mismatch,
        "hot_runtime_decision_parity_mismatch": hot_decision_parity_mismatch,
        "local_operator_shadow_decision": local_operator && event.verified_safe_accept,
        "fallback_shadow_decision": !(local_operator && event.verified_safe_accept),
        "false_accept": false_accept,
        "unique_cpu_accept_over_exact_cache": unique_accept,
        "token_cost": event.token_cost,
        "package_fingerprint64": bucket.active_package_fingerprint64,
        "local_accept_enabled": false
    });
    if decision_log_limit.is_none_or(|limit| *decision_log_rows_written < limit) {
        writeln!(
            decision_log,
            "{}",
            serde_json::to_string(&decision_row)
                .map_err(|error| format!("online miner decision serialization error: {error}"))?
        )
        .map_err(|error| format!("failed to write online miner decision log: {error}"))?;
        *decision_log_rows_written = decision_log_rows_written.saturating_add(1);
    } else {
        *decision_log_capped = true;
    }
    Ok(())
}

fn update_online_bucket(
    bucket: &mut OnlineMinerBucketState,
    event: PhaseAtomBinaryEvent,
    exact_cache_hit: bool,
    token_cost: GenericTokenCost,
    cells: usize,
    reservoir_per_label: usize,
) -> Result<(), String> {
    let safe_accept_vec =
        phase_atom_binary_event_vector_for_task(&event, true, cells, &bucket.task_name);
    if event.verified_safe_accept {
        bucket
            .compiler
            .add_positive_vector(0, &safe_accept_vec)
            .map_err(|error| format!("online miner positive update error: {error:?}"))?;
        bucket.positive_events += 1;
        if !exact_cache_hit {
            bucket.non_exact_positive_events += 1;
        }
        push_online_reservoir(
            &mut bucket.positive_reservoir,
            event,
            reservoir_per_label,
            "positive",
        );
    } else {
        bucket
            .compiler
            .add_negative_vector(0, &safe_accept_vec)
            .map_err(|error| format!("online miner negative update error: {error:?}"))?;
        bucket.negative_events += 1;
        push_online_reservoir(
            &mut bucket.negative_reservoir,
            event,
            reservoir_per_label,
            "negative",
        );
    }
    bucket.events_seen += 1;
    bucket.exact_cache_hits += usize::from(exact_cache_hit);
    bucket.total_tokens = bucket.total_tokens.saturating_add(token_cost.total_tokens);
    bucket.total_cost_microusd = bucket
        .total_cost_microusd
        .saturating_add(token_cost.total_cost_microusd);
    Ok(())
}

fn push_online_reservoir(
    reservoir: &mut Vec<PhaseAtomBinaryEvent>,
    event: PhaseAtomBinaryEvent,
    limit: usize,
    salt: &str,
) {
    if reservoir.len() < limit {
        reservoir.push(event);
    } else {
        let replace_index = (stable_fingerprint([
            salt,
            event.event_timestamp.as_str(),
            event.request_fingerprint.as_str(),
            event.exact_cache_key.as_str(),
        ]) as usize)
            % limit;
        reservoir[replace_index] = event;
    }
}

fn compile_online_miner_checkpoints(
    buckets: &mut BTreeMap<String, OnlineMinerBucketState>,
    config: OnlineMinerCompileConfig<'_>,
    checkpoints: &mut Vec<OnlineMinerCheckpointReport>,
    hot_audition_replacement_count: &mut usize,
) -> Result<(), String> {
    let mut eligible = buckets
        .iter()
        .filter(|(_, bucket)| {
            bucket.events_seen >= config.min_bucket_events
                && bucket.positive_events > 0
                && bucket.negative_events > 0
                && bucket.value_score() > 0
                && bucket.last_compiled_after_row < config.compiled_after_row
        })
        .map(|(key, bucket)| (key.clone(), online_miner_hot_audition_rank(bucket)))
        .collect::<Vec<_>>();
    eligible.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    for (bucket_key, _) in eligible {
        let hot_slot = online_miner_prepare_hot_slot_for_bucket(
            buckets,
            &bucket_key,
            config.max_active_buckets,
        );
        if !hot_slot.allowed {
            continue;
        }
        *hot_audition_replacement_count = hot_audition_replacement_count
            .saturating_add(usize::from(hot_slot.replaced_existing_hot_bucket));
        let bucket = buckets
            .get_mut(&bucket_key)
            .ok_or_else(|| format!("online miner missing bucket '{bucket_key}'"))?;
        compile_online_miner_checkpoint_for_bucket(bucket, config, checkpoints)?;
    }
    Ok(())
}

fn online_miner_checkpoint_kind_priority(bucket_kind: &str) -> u8 {
    match bucket_kind {
        "hidden_state_split" => 7,
        "learned_auto_subcenter" => 6,
        "auto_subcenter" => 5,
        "state_action_signature" => 3,
        "broad_action" => 1,
        _ => 0,
    }
}

fn online_miner_prepare_hot_slot_for_bucket(
    buckets: &mut BTreeMap<String, OnlineMinerBucketState>,
    candidate_key: &str,
    max_active_buckets: usize,
) -> OnlineMinerHotSlotDecision {
    let Some(candidate) = buckets.get(candidate_key) else {
        return OnlineMinerHotSlotDecision::default();
    };
    if candidate.active_runtime.is_some() {
        return OnlineMinerHotSlotDecision {
            allowed: true,
            replaced_existing_hot_bucket: false,
        };
    }
    if buckets
        .values()
        .filter(|bucket| bucket.active_runtime.is_some())
        .count()
        < max_active_buckets
    {
        return OnlineMinerHotSlotDecision {
            allowed: true,
            replaced_existing_hot_bucket: false,
        };
    }

    let candidate_rank = online_miner_hot_audition_rank(candidate);
    let Some((worst_key, worst_rank)) = buckets
        .iter()
        .filter(|(key, bucket)| key.as_str() != candidate_key && bucket.active_runtime.is_some())
        .map(|(key, bucket)| (key.clone(), online_miner_hot_audition_rank(bucket)))
        .min_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
    else {
        return OnlineMinerHotSlotDecision::default();
    };
    if candidate_rank <= worst_rank {
        return OnlineMinerHotSlotDecision::default();
    }
    if let Some(worst) = buckets.get_mut(&worst_key) {
        worst.active_runtime = None;
        worst.active_reference_runtime = None;
        worst.active_hot_runtime = None;
    }
    OnlineMinerHotSlotDecision {
        allowed: true,
        replaced_existing_hot_bucket: true,
    }
}

fn online_miner_hot_audition_rank(bucket: &OnlineMinerBucketState) -> OnlineMinerHotAuditionRank {
    let value_score = bucket.value_score();
    OnlineMinerHotAuditionRank {
        weighted_value_score: value_score
            .saturating_mul(online_miner_hot_audition_kind_weight(bucket.bucket_kind)),
        value_score,
        kind_priority: online_miner_checkpoint_kind_priority(bucket.bucket_kind),
        non_exact_positive_events: bucket.non_exact_positive_events,
        events_seen: bucket.events_seen,
    }
}

fn online_miner_hot_audition_kind_weight(bucket_kind: &str) -> u128 {
    match bucket_kind {
        "hidden_state_split" => 8,
        "learned_auto_subcenter" => 8,
        _ => 1,
    }
}

fn online_miner_bucket_checkpoint_eligible(
    bucket: &OnlineMinerBucketState,
    min_bucket_events: usize,
) -> bool {
    bucket.events_seen >= min_bucket_events
        && bucket.positive_events > 0
        && bucket.negative_events > 0
        && bucket.value_score() > 0
}

fn online_miner_bucket_immediate_checkpoint_allowed(bucket: &OnlineMinerBucketState) -> bool {
    bucket.bucket_kind == "broad_action"
}

fn compile_online_miner_checkpoint_for_bucket(
    bucket: &mut OnlineMinerBucketState,
    config: OnlineMinerCompileConfig<'_>,
    checkpoints: &mut Vec<OnlineMinerCheckpointReport>,
) -> Result<(), String> {
    if !online_miner_bucket_checkpoint_eligible(bucket, config.min_bucket_events)
        || bucket.last_compiled_after_row >= config.compiled_after_row
    {
        return Ok(());
    }
    let reference_runtime = bucket
        .compiler
        .clone()
        .compile()
        .map_err(|error| format!("online miner checkpoint compile error: {error:?}"))?;
    let (threshold, max_false, min_true) = calibrate_online_threshold(
        bucket,
        &reference_runtime,
        config.cells,
        config.base_margin_threshold_micro,
    )?;
    let package_bytes = reference_runtime
        .to_bytes()
        .map_err(|error| format!("online miner package serialization error: {error:?}"))?;
    let package_path = config.checkpoint_dir.join(format!(
        "{}-row{:08}.candidate.nwpc",
        sanitize_file_stem(&bucket.task_name),
        config.compiled_after_row
    ));
    write_binary_file(&package_path, &package_bytes)?;
    let read_back = std::fs::read(&package_path).map_err(|error| {
        format!(
            "failed to read online miner package '{}': {error}",
            package_path.display()
        )
    })?;
    let package_readback_exact = read_back == package_bytes;
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&read_back)
        .map_err(|error| format!("online miner package inspect error: {error:?}"))?;
    let offload_runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &read_back,
        PhaseCenterOffloadPolicy::new(threshold)
            .map_err(|error| format!("online miner invalid policy: {error:?}"))?,
    )
    .map_err(|error| format!("online miner package load error: {error:?}"))?;
    let hot_runtime =
        PhaseCenterHotRuntime::from_flat_runtime(&reference_runtime, &[0], &[threshold])
            .map_err(|error| format!("online miner hot runtime build error: {error:?}"))?;
    let active_runtime_bytes_estimate = offload_runtime.bytes_estimate();
    bucket.active_runtime = Some(offload_runtime);
    bucket.active_reference_runtime = Some(reference_runtime);
    bucket.active_hot_runtime = Some(hot_runtime);
    bucket.active_package_path = package_path.display().to_string();
    bucket.active_package_fingerprint64 = package_info.fingerprint64;
    bucket.package_bytes = read_back.len();
    bucket.package_records = package_info.record_count;
    bucket.safe_accept_margin_threshold_micro = threshold;
    bucket.train_safe_accept_max_false_margin_micro = max_false;
    bucket.train_safe_accept_min_true_margin_micro = min_true;
    bucket.last_compiled_after_row = config.compiled_after_row;
    bucket.checkpoints_compiled += 1;
    checkpoints.push(OnlineMinerCheckpointReport {
        bucket_key: bucket.bucket_key.clone(),
        bucket_kind: bucket.bucket_kind,
        action_family_atom: bucket.action_family_atom.clone(),
        task_name: bucket.task_name.clone(),
        compiled_after_row: config.compiled_after_row,
        events_seen_at_compile: bucket.events_seen,
        positive_events_at_compile: bucket.positive_events,
        negative_events_at_compile: bucket.negative_events,
        safe_accept_margin_threshold_micro: threshold,
        train_safe_accept_max_false_margin_micro: max_false,
        train_safe_accept_min_true_margin_micro: min_true,
        package_path: bucket.active_package_path.clone(),
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: read_back.len(),
        package_records: package_info.record_count,
        active_runtime_bytes_estimate,
        package_readback_exact,
        reservoir_positive_events: bucket.positive_reservoir.len(),
        reservoir_negative_events: bucket.negative_reservoir.len(),
    });
    Ok(())
}

fn calibrate_online_threshold(
    bucket: &OnlineMinerBucketState,
    runtime: &PhaseCenterFlatRuntime,
    cells: usize,
    base_margin_threshold_micro: i64,
) -> Result<(i64, Option<i64>, Option<i64>), String> {
    let mut false_margins = Vec::new();
    let mut true_margins = Vec::new();
    for event in &bucket.negative_reservoir {
        false_margins.push(online_event_margin_micro(
            runtime,
            event,
            cells,
            &bucket.task_name,
        )?);
    }
    for event in &bucket.positive_reservoir {
        true_margins.push(online_event_margin_micro(
            runtime,
            event,
            cells,
            &bucket.task_name,
        )?);
    }
    let max_false = false_margins.iter().max().copied();
    let min_true = true_margins.iter().min().copied();
    let threshold = max_false
        .map_or(base_margin_threshold_micro, |value| value.saturating_add(1))
        .max(1);
    Ok((threshold, max_false, min_true))
}

fn online_event_margin_micro(
    runtime: &PhaseCenterFlatRuntime,
    event: &PhaseAtomBinaryEvent,
    cells: usize,
    task_name: &str,
) -> Result<i64, String> {
    let safe_accept_vec = phase_atom_binary_event_vector_for_task(event, true, cells, task_name);
    let zero = vec![nando_core::PhaseCenterCell::default(); cells];
    let task = PhaseCenterEvalTask {
        center_index: 0,
        correct_vec: safe_accept_vec.into_boxed_slice(),
        wrong_vec: zero.into_boxed_slice(),
    };
    margin_to_micro(
        runtime
            .margin(&task)
            .map_err(|error| format!("online miner calibration margin error: {error:?}"))?,
    )
}

fn online_miner_auto_calibrated_stats(
    bucket: &OnlineMinerBucketState,
    base_margin_threshold_micro: i64,
) -> OnlineMinerAutoCalibratedStats {
    let calibration_events = online_miner_auto_calibration_len(bucket.future_decisions.len());
    let max_false_margin_micro = bucket
        .future_decisions
        .iter()
        .take(calibration_events)
        .filter(|sample| !sample.verified_safe_accept)
        .map(|sample| sample.margin_micro)
        .max();
    let threshold_micro = max_false_margin_micro.map_or(base_margin_threshold_micro, |margin| {
        base_margin_threshold_micro.max(margin.saturating_add(1))
    });
    let mut stats = OnlineMinerAutoCalibratedStats {
        threshold_micro,
        calibration_events,
        max_false_margin_micro,
        ..OnlineMinerAutoCalibratedStats::default()
    };
    for sample in bucket.future_decisions.iter().skip(calibration_events) {
        stats.shadow_events_after_calibration += 1;
        if sample.margin_micro < threshold_micro {
            continue;
        }
        if sample.verified_safe_accept {
            stats.local_operator_shadow_decisions += 1;
            if !sample.exact_cache_hit {
                stats.unique_cpu_accepts_over_exact_cache += 1;
                stats.nando_cpu_tokens_saved = stats
                    .nando_cpu_tokens_saved
                    .saturating_add(sample.total_tokens);
                stats.nando_cpu_cost_saved_microusd = stats
                    .nando_cpu_cost_saved_microusd
                    .saturating_add(sample.total_cost_microusd);
            }
        } else {
            stats.false_accepts += 1;
        }
    }
    stats
}

fn online_miner_auto_calibration_len(future_decision_count: usize) -> usize {
    if future_decision_count < 4 {
        return future_decision_count;
    }
    (future_decision_count / 2)
        .min(ONLINE_MINER_AUTO_CALIBRATION_MAX_DECISIONS)
        .max(1)
        .min(future_decision_count.saturating_sub(1))
}

fn online_miner_targeted_train_target(events_seen: usize, train_permille: usize) -> usize {
    if events_seen < 2 {
        return events_seen;
    }
    (events_seen.saturating_mul(train_permille) / 1000).clamp(1, events_seen.saturating_sub(1))
}

fn online_miner_auto_calibrated_total<I>(stats_iter: I) -> OnlineMinerAutoCalibratedStats
where
    I: IntoIterator<Item = OnlineMinerAutoCalibratedStats>,
{
    let mut total = OnlineMinerAutoCalibratedStats::default();
    for stats in stats_iter {
        if stats.false_accepts > 0 {
            total.rejected_bucket_count += 1;
            total.rejected_false_accepts += stats.false_accepts;
            total.max_false_margin_micro =
                match (total.max_false_margin_micro, stats.max_false_margin_micro) {
                    (Some(left), Some(right)) => Some(left.max(right)),
                    (Some(value), None) | (None, Some(value)) => Some(value),
                    (None, None) => None,
                };
            continue;
        }
        if stats.unique_cpu_accepts_over_exact_cache == 0 {
            continue;
        }
        total.accepted_bucket_count += 1;
        total.threshold_micro = total.threshold_micro.max(stats.threshold_micro);
        total.calibration_events += stats.calibration_events;
        total.shadow_events_after_calibration += stats.shadow_events_after_calibration;
        total.local_operator_shadow_decisions += stats.local_operator_shadow_decisions;
        total.unique_cpu_accepts_over_exact_cache += stats.unique_cpu_accepts_over_exact_cache;
        total.nando_cpu_tokens_saved = total
            .nando_cpu_tokens_saved
            .saturating_add(stats.nando_cpu_tokens_saved);
        total.nando_cpu_cost_saved_microusd = total
            .nando_cpu_cost_saved_microusd
            .saturating_add(stats.nando_cpu_cost_saved_microusd);
        total.false_accepts += stats.false_accepts;
        total.max_false_margin_micro =
            match (total.max_false_margin_micro, stats.max_false_margin_micro) {
                (Some(left), Some(right)) => Some(left.max(right)),
                (Some(value), None) | (None, Some(value)) => Some(value),
                (None, None) => None,
            };
    }
    total
}

fn online_miner_global_auto_calibrated_stats(
    buckets: &BTreeMap<String, OnlineMinerBucketState>,
) -> OnlineMinerGlobalAutoCalibratedStats {
    let mut accepted_fingerprints = BTreeSet::<String>::new();
    let mut stats = OnlineMinerGlobalAutoCalibratedStats::default();
    for bucket in buckets.values() {
        let calibration_events = online_miner_auto_calibration_len(bucket.future_decisions.len());
        let threshold_micro =
            online_miner_auto_calibrated_stats(bucket, bucket.safe_accept_margin_threshold_micro)
                .threshold_micro;
        for sample in bucket.future_decisions.iter().skip(calibration_events) {
            if sample.margin_micro < threshold_micro {
                continue;
            }
            if !sample.verified_safe_accept {
                stats.false_accepts += 1;
                continue;
            }
            if sample.exact_cache_hit {
                continue;
            }
            if accepted_fingerprints.insert(sample.request_fingerprint.clone()) {
                stats.unique_cpu_accepts_over_exact_cache += 1;
                stats.nando_cpu_tokens_saved = stats
                    .nando_cpu_tokens_saved
                    .saturating_add(sample.total_tokens);
                stats.nando_cpu_cost_saved_microusd = stats
                    .nando_cpu_cost_saved_microusd
                    .saturating_add(sample.total_cost_microusd);
            } else {
                stats.duplicate_accept_rows += 1;
            }
        }
    }
    stats
}

fn online_miner_multi_split_global_auto_calibrated_stats(
    buckets: &BTreeMap<String, OnlineMinerBucketState>,
) -> OnlineMinerGlobalAutoCalibratedStats {
    let mut accepted_fingerprints = BTreeSet::<String>::new();
    let mut stats = OnlineMinerGlobalAutoCalibratedStats::default();
    for bucket in buckets.values().filter(|bucket| {
        bucket.active_runtime.is_some() && online_miner_bucket_is_multi_split(bucket)
    }) {
        let bucket_stats =
            online_miner_auto_calibrated_stats(bucket, bucket.safe_accept_margin_threshold_micro);
        if bucket_stats.false_accepts > 0 || bucket_stats.unique_cpu_accepts_over_exact_cache == 0 {
            continue;
        }
        let calibration_events = online_miner_auto_calibration_len(bucket.future_decisions.len());
        for sample in bucket.future_decisions.iter().skip(calibration_events) {
            if sample.margin_micro < bucket_stats.threshold_micro {
                continue;
            }
            if !sample.verified_safe_accept {
                stats.false_accepts += 1;
                continue;
            }
            if sample.exact_cache_hit {
                continue;
            }
            if accepted_fingerprints.insert(sample.request_fingerprint.clone()) {
                stats.unique_cpu_accepts_over_exact_cache += 1;
                stats.nando_cpu_tokens_saved = stats
                    .nando_cpu_tokens_saved
                    .saturating_add(sample.total_tokens);
                stats.nando_cpu_cost_saved_microusd = stats
                    .nando_cpu_cost_saved_microusd
                    .saturating_add(sample.total_cost_microusd);
            } else {
                stats.duplicate_accept_rows += 1;
            }
        }
    }
    stats
}

fn online_miner_global_auto_calibrated_stats_for_candidates(
    buckets: &BTreeMap<String, OnlineMinerBucketState>,
    candidates: &[OnlineMinerPromotionCandidateReport],
) -> OnlineMinerGlobalAutoCalibratedStats {
    let candidate_keys = candidates
        .iter()
        .map(|candidate| candidate.bucket_key.as_str())
        .collect::<BTreeSet<_>>();
    let mut accepted_fingerprints = BTreeSet::<String>::new();
    let mut stats = OnlineMinerGlobalAutoCalibratedStats::default();
    for bucket in buckets
        .values()
        .filter(|bucket| candidate_keys.contains(bucket.bucket_key.as_str()))
    {
        let bucket_stats =
            online_miner_auto_calibrated_stats(bucket, bucket.safe_accept_margin_threshold_micro);
        if bucket_stats.false_accepts > 0 || bucket_stats.unique_cpu_accepts_over_exact_cache == 0 {
            continue;
        }
        let calibration_events = online_miner_auto_calibration_len(bucket.future_decisions.len());
        for sample in bucket.future_decisions.iter().skip(calibration_events) {
            if sample.margin_micro < bucket_stats.threshold_micro {
                continue;
            }
            if !sample.verified_safe_accept {
                stats.false_accepts += 1;
                continue;
            }
            if sample.exact_cache_hit {
                continue;
            }
            if accepted_fingerprints.insert(sample.request_fingerprint.clone()) {
                stats.unique_cpu_accepts_over_exact_cache += 1;
                stats.nando_cpu_tokens_saved = stats
                    .nando_cpu_tokens_saved
                    .saturating_add(sample.total_tokens);
                stats.nando_cpu_cost_saved_microusd = stats
                    .nando_cpu_cost_saved_microusd
                    .saturating_add(sample.total_cost_microusd);
            } else {
                stats.duplicate_accept_rows += 1;
            }
        }
    }
    stats
}

fn online_miner_product_hot_promotion_candidates(
    buckets: &BTreeMap<String, OnlineMinerBucketState>,
) -> Vec<OnlineMinerPromotionCandidateReport> {
    let mut candidates = buckets
        .values()
        .filter_map(|bucket| {
            bucket.active_runtime.as_ref()?;
            let stats = online_miner_auto_calibrated_stats(
                bucket,
                bucket.safe_accept_margin_threshold_micro,
            );
            if stats.false_accepts > 0 || stats.unique_cpu_accepts_over_exact_cache == 0 {
                return None;
            }
            let package_evidence = online_miner_promotion_package_evidence(bucket);
            let promotion_gate_passed = package_evidence.file_exists
                && package_evidence.reload_verified
                && package_evidence.fingerprint_verified
                && package_evidence.records_verified
                && stats.false_accepts == 0
                && stats.unique_cpu_accepts_over_exact_cache > 0;
            Some(OnlineMinerPromotionCandidateReport {
                bucket_key: bucket.bucket_key.clone(),
                bucket_kind: bucket.bucket_kind,
                action_family_atom: bucket.action_family_atom.clone(),
                task_name: bucket.task_name.clone(),
                package_path: bucket.active_package_path.clone(),
                package_fingerprint64: bucket.active_package_fingerprint64,
                package_bytes: bucket.package_bytes,
                package_records: bucket.package_records,
                package_file_exists: package_evidence.file_exists,
                package_read_bytes: package_evidence.read_bytes,
                package_reload_verified: package_evidence.reload_verified,
                package_fingerprint_verified: package_evidence.fingerprint_verified,
                package_records_verified: package_evidence.records_verified,
                promotion_gate_passed,
                safe_accept_margin_threshold_micro: bucket.safe_accept_margin_threshold_micro,
                auto_calibrated_margin_threshold_micro: stats.threshold_micro,
                auto_calibration_events: stats.calibration_events,
                shadow_events_after_calibration: stats.shadow_events_after_calibration,
                unique_cpu_accepts_over_exact_cache: stats.unique_cpu_accepts_over_exact_cache,
                nando_cpu_tokens_saved: stats.nando_cpu_tokens_saved,
                nando_cpu_cost_saved_microusd: stats.nando_cpu_cost_saved_microusd,
                false_accepts: stats.false_accepts,
                verifier_bound: true,
                quarantine_nwpc: true,
                shadow_only: true,
                local_accept_enabled: false,
                auto_promote_enabled: false,
                promotion_status: "quarantine_review_only",
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .unique_cpu_accepts_over_exact_cache
            .cmp(&left.unique_cpu_accepts_over_exact_cache)
            .then_with(|| {
                right
                    .nando_cpu_tokens_saved
                    .cmp(&left.nando_cpu_tokens_saved)
            })
            .then_with(|| left.bucket_key.cmp(&right.bucket_key))
    });
    candidates
}

fn online_miner_product_hot_budget_candidates_with_buckets(
    buckets: Option<&BTreeMap<String, OnlineMinerBucketState>>,
    candidates: &[OnlineMinerPromotionCandidateReport],
) -> Vec<OnlineMinerPromotionCandidateReport> {
    let Some(buckets) = buckets else {
        return online_miner_product_hot_budget_candidates_by_individual_value(candidates);
    };
    let mut remaining = candidates.to_vec();
    let mut selected = Vec::<OnlineMinerPromotionCandidateReport>::new();
    let mut accepted_fingerprints = BTreeSet::<String>::new();
    while selected.len() < ONLINE_MINER_PRODUCT_MAX_HOT_PROFILES_PER_WORKER && !remaining.is_empty()
    {
        let Some((best_index, best_gain)) = remaining
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                (
                    index,
                    online_miner_product_hot_marginal_gain(
                        buckets,
                        candidate,
                        &accepted_fingerprints,
                    ),
                )
            })
            .filter(|(_, gain)| gain.unique_accepts > 0 && gain.false_accepts == 0)
            .max_by(|(left_index, left), (right_index, right)| {
                left.cmp(right).then_with(|| {
                    remaining[*right_index]
                        .bucket_key
                        .cmp(&remaining[*left_index].bucket_key)
                })
            })
        else {
            break;
        };
        let candidate = remaining.remove(best_index);
        for fingerprint in
            online_miner_product_hot_candidate_accept_fingerprints(buckets, &candidate)
        {
            accepted_fingerprints.insert(fingerprint);
        }
        debug_assert!(best_gain.unique_accepts > 0);
        selected.push(candidate);
    }
    selected
}

fn online_miner_product_hot_budget_candidates_by_individual_value(
    candidates: &[OnlineMinerPromotionCandidateReport],
) -> Vec<OnlineMinerPromotionCandidateReport> {
    let mut selected = candidates.to_vec();
    selected.sort_by(|left, right| {
        right
            .unique_cpu_accepts_over_exact_cache
            .cmp(&left.unique_cpu_accepts_over_exact_cache)
            .then_with(|| {
                right
                    .nando_cpu_tokens_saved
                    .cmp(&left.nando_cpu_tokens_saved)
            })
            .then_with(|| left.bucket_key.cmp(&right.bucket_key))
    });
    selected.truncate(ONLINE_MINER_PRODUCT_MAX_HOT_PROFILES_PER_WORKER);
    selected
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct OnlineMinerProductHotMarginalGain {
    unique_accepts: usize,
    tokens_saved: usize,
    cost_saved_microusd: u64,
    false_accepts: usize,
}

impl Ord for OnlineMinerProductHotMarginalGain {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.unique_accepts
            .cmp(&other.unique_accepts)
            .then_with(|| self.tokens_saved.cmp(&other.tokens_saved))
            .then_with(|| self.cost_saved_microusd.cmp(&other.cost_saved_microusd))
            .then_with(|| other.false_accepts.cmp(&self.false_accepts))
    }
}

impl PartialOrd for OnlineMinerProductHotMarginalGain {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn online_miner_product_hot_marginal_gain(
    buckets: &BTreeMap<String, OnlineMinerBucketState>,
    candidate: &OnlineMinerPromotionCandidateReport,
    accepted_fingerprints: &BTreeSet<String>,
) -> OnlineMinerProductHotMarginalGain {
    let Some(bucket) = buckets.get(&candidate.bucket_key) else {
        return OnlineMinerProductHotMarginalGain::default();
    };
    let bucket_stats =
        online_miner_auto_calibrated_stats(bucket, bucket.safe_accept_margin_threshold_micro);
    if bucket_stats.false_accepts > 0 || bucket_stats.unique_cpu_accepts_over_exact_cache == 0 {
        return OnlineMinerProductHotMarginalGain {
            false_accepts: bucket_stats.false_accepts,
            ..OnlineMinerProductHotMarginalGain::default()
        };
    }
    let calibration_events = online_miner_auto_calibration_len(bucket.future_decisions.len());
    let mut gain = OnlineMinerProductHotMarginalGain::default();
    for sample in bucket.future_decisions.iter().skip(calibration_events) {
        if sample.margin_micro < bucket_stats.threshold_micro {
            continue;
        }
        if !sample.verified_safe_accept {
            gain.false_accepts = gain.false_accepts.saturating_add(1);
            continue;
        }
        if sample.exact_cache_hit || accepted_fingerprints.contains(&sample.request_fingerprint) {
            continue;
        }
        gain.unique_accepts = gain.unique_accepts.saturating_add(1);
        gain.tokens_saved = gain.tokens_saved.saturating_add(sample.total_tokens);
        gain.cost_saved_microusd = gain
            .cost_saved_microusd
            .saturating_add(sample.total_cost_microusd);
    }
    gain
}

fn online_miner_product_hot_candidate_accept_fingerprints(
    buckets: &BTreeMap<String, OnlineMinerBucketState>,
    candidate: &OnlineMinerPromotionCandidateReport,
) -> Vec<String> {
    let Some(bucket) = buckets.get(&candidate.bucket_key) else {
        return Vec::new();
    };
    let bucket_stats =
        online_miner_auto_calibrated_stats(bucket, bucket.safe_accept_margin_threshold_micro);
    if bucket_stats.false_accepts > 0 || bucket_stats.unique_cpu_accepts_over_exact_cache == 0 {
        return Vec::new();
    }
    let calibration_events = online_miner_auto_calibration_len(bucket.future_decisions.len());
    bucket
        .future_decisions
        .iter()
        .skip(calibration_events)
        .filter(|sample| {
            sample.margin_micro >= bucket_stats.threshold_micro
                && sample.verified_safe_accept
                && !sample.exact_cache_hit
        })
        .map(|sample| sample.request_fingerprint.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn online_miner_product_hot_kind_contributions(
    buckets: &BTreeMap<String, OnlineMinerBucketState>,
    selected_candidates: &[OnlineMinerPromotionCandidateReport],
) -> Vec<OnlineMinerProductHotKindContributionReport> {
    let mut accepted_fingerprints = BTreeSet::<String>::new();
    let mut by_kind = BTreeMap::<&'static str, OnlineMinerProductHotKindContributionReport>::new();
    for candidate in selected_candidates {
        let gain =
            online_miner_product_hot_marginal_gain(buckets, candidate, &accepted_fingerprints);
        let entry = by_kind.entry(candidate.bucket_kind).or_insert(
            OnlineMinerProductHotKindContributionReport {
                bucket_kind: candidate.bucket_kind,
                ..OnlineMinerProductHotKindContributionReport::default()
            },
        );
        entry.selected_profile_count = entry.selected_profile_count.saturating_add(1);
        entry.marginal_unique_cpu_accepts_over_exact_cache = entry
            .marginal_unique_cpu_accepts_over_exact_cache
            .saturating_add(gain.unique_accepts);
        entry.marginal_nando_cpu_tokens_saved = entry
            .marginal_nando_cpu_tokens_saved
            .saturating_add(gain.tokens_saved);
        entry.marginal_nando_cpu_cost_saved_microusd = entry
            .marginal_nando_cpu_cost_saved_microusd
            .saturating_add(gain.cost_saved_microusd);
        entry.marginal_false_accepts = entry
            .marginal_false_accepts
            .saturating_add(gain.false_accepts);
        for fingerprint in
            online_miner_product_hot_candidate_accept_fingerprints(buckets, candidate)
        {
            accepted_fingerprints.insert(fingerprint);
        }
    }
    let mut contributions = by_kind.into_values().collect::<Vec<_>>();
    contributions.sort_by(|left, right| {
        right
            .marginal_unique_cpu_accepts_over_exact_cache
            .cmp(&left.marginal_unique_cpu_accepts_over_exact_cache)
            .then_with(|| {
                right
                    .marginal_nando_cpu_tokens_saved
                    .cmp(&left.marginal_nando_cpu_tokens_saved)
            })
            .then_with(|| left.bucket_kind.cmp(right.bucket_kind))
    });
    contributions
}

fn online_miner_value_pass_action_has_split_pressure(
    buckets: &BTreeMap<String, OnlineMinerValuePassBucketState>,
    action_family: &str,
) -> bool {
    let broad_key = format!("{action_family}::broad_action");
    buckets.get(&broad_key).is_some_and(|bucket| {
        bucket.events_seen >= ONLINE_MINER_LEARNED_SPLIT_MIN_BROAD_EVENTS
            && bucket.positive_events >= ONLINE_MINER_LEARNED_SPLIT_MIN_BROAD_LABELS
            && bucket.negative_events >= ONLINE_MINER_LEARNED_SPLIT_MIN_BROAD_LABELS
            && bucket.non_exact_positive_events > 0
    })
}

fn online_miner_value_pass_bucket_is_candidate_kind(bucket_kind: &str) -> bool {
    matches!(
        bucket_kind,
        "auto_subcenter" | "learned_auto_subcenter" | "hidden_state_split"
    )
}

fn online_miner_value_pass_bucket_kind_count(
    buckets: &BTreeMap<String, OnlineMinerValuePassBucketState>,
    bucket_kind: &str,
) -> usize {
    buckets
        .values()
        .filter(|bucket| bucket.bucket_kind == bucket_kind)
        .count()
}

fn online_miner_value_pass_top_candidates(
    buckets: &BTreeMap<String, OnlineMinerValuePassBucketState>,
    limit: usize,
) -> Vec<OnlineMinerValuePassCandidateReport> {
    let mut candidates = buckets
        .values()
        .filter(|bucket| bucket.eligible_for_candidate())
        .map(|bucket| online_miner_value_pass_candidate_report(bucket, false))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .value_score
            .cmp(&left.value_score)
            .then_with(|| {
                right
                    .non_exact_positive_events
                    .cmp(&left.non_exact_positive_events)
            })
            .then_with(|| left.bucket_key.cmp(&right.bucket_key))
    });
    candidates.truncate(limit);
    candidates
}

fn online_miner_value_pass_product_hot_candidates(
    buckets: &BTreeMap<String, OnlineMinerValuePassBucketState>,
    limit: usize,
) -> Vec<OnlineMinerValuePassCandidateReport> {
    let mut remaining = buckets
        .values()
        .filter(|bucket| bucket.eligible_for_candidate())
        .collect::<Vec<_>>();
    remaining.sort_by(|left, right| {
        right
            .value_score()
            .cmp(&left.value_score())
            .then_with(|| left.bucket_key.cmp(&right.bucket_key))
    });
    let mut selected = Vec::<OnlineMinerValuePassCandidateReport>::new();
    let mut accepted_fingerprints = BTreeSet::<String>::new();
    while selected.len() < limit && !remaining.is_empty() {
        let Some((best_index, best_gain)) = remaining
            .iter()
            .enumerate()
            .map(|(index, bucket)| {
                (
                    index,
                    online_miner_value_pass_marginal_gain(bucket, &accepted_fingerprints),
                )
            })
            .filter(|(_, gain)| gain.unique_accepts > 0)
            .max_by(|(left_index, left), (right_index, right)| {
                left.cmp(right).then_with(|| {
                    remaining[*right_index]
                        .bucket_key
                        .cmp(&remaining[*left_index].bucket_key)
                })
            })
        else {
            break;
        };
        let bucket = remaining.remove(best_index);
        for sample in &bucket.positive_non_exact_samples {
            accepted_fingerprints.insert(sample.request_fingerprint.clone());
        }
        debug_assert!(best_gain.unique_accepts > 0);
        selected.push(online_miner_value_pass_candidate_report(bucket, true));
    }
    selected
}

fn online_miner_value_pass_candidate_report(
    bucket: &OnlineMinerValuePassBucketState,
    selected_for_product_hot_candidate: bool,
) -> OnlineMinerValuePassCandidateReport {
    OnlineMinerValuePassCandidateReport {
        bucket_key: bucket.bucket_key.clone(),
        bucket_kind: bucket.bucket_kind,
        action_family_atom: bucket.action_family_atom.clone(),
        events_seen: bucket.events_seen,
        positive_events: bucket.positive_events,
        negative_events: bucket.negative_events,
        non_exact_positive_events: bucket.non_exact_positive_events,
        total_tokens: bucket.total_tokens,
        total_cost_microusd: bucket.total_cost_microusd,
        value_score: bucket.value_score(),
        selected_for_product_hot_candidate,
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct OnlineMinerValuePassMarginalGain {
    unique_accepts: usize,
    tokens_saved: usize,
    cost_saved_microusd: u64,
}

impl Ord for OnlineMinerValuePassMarginalGain {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.unique_accepts
            .cmp(&other.unique_accepts)
            .then_with(|| self.tokens_saved.cmp(&other.tokens_saved))
            .then_with(|| self.cost_saved_microusd.cmp(&other.cost_saved_microusd))
    }
}

impl PartialOrd for OnlineMinerValuePassMarginalGain {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn online_miner_value_pass_marginal_gain(
    bucket: &OnlineMinerValuePassBucketState,
    accepted_fingerprints: &BTreeSet<String>,
) -> OnlineMinerValuePassMarginalGain {
    let mut gain = OnlineMinerValuePassMarginalGain::default();
    for sample in &bucket.positive_non_exact_samples {
        if accepted_fingerprints.contains(&sample.request_fingerprint) {
            continue;
        }
        gain.unique_accepts = gain.unique_accepts.saturating_add(1);
        gain.tokens_saved = gain.tokens_saved.saturating_add(sample.total_tokens);
        gain.cost_saved_microusd = gain
            .cost_saved_microusd
            .saturating_add(sample.total_cost_microusd);
    }
    gain
}

fn online_miner_value_pass_global_stats_for_candidates(
    buckets: &BTreeMap<String, OnlineMinerValuePassBucketState>,
    candidate_keys: &[&str],
) -> OnlineMinerGlobalAutoCalibratedStats {
    let mut accepted_fingerprints = BTreeSet::<String>::new();
    let mut stats = OnlineMinerGlobalAutoCalibratedStats::default();
    for key in candidate_keys {
        let Some(bucket) = buckets.get(*key) else {
            continue;
        };
        for sample in &bucket.positive_non_exact_samples {
            if accepted_fingerprints.insert(sample.request_fingerprint.clone()) {
                stats.unique_cpu_accepts_over_exact_cache =
                    stats.unique_cpu_accepts_over_exact_cache.saturating_add(1);
                stats.nando_cpu_tokens_saved = stats
                    .nando_cpu_tokens_saved
                    .saturating_add(sample.total_tokens);
                stats.nando_cpu_cost_saved_microusd = stats
                    .nando_cpu_cost_saved_microusd
                    .saturating_add(sample.total_cost_microusd);
            } else {
                stats.duplicate_accept_rows = stats.duplicate_accept_rows.saturating_add(1);
            }
        }
    }
    stats
}

fn online_miner_value_pass_kind_contributions(
    buckets: &BTreeMap<String, OnlineMinerValuePassBucketState>,
    selected_candidates: &[OnlineMinerValuePassCandidateReport],
) -> Vec<OnlineMinerProductHotKindContributionReport> {
    let mut accepted_fingerprints = BTreeSet::<String>::new();
    let mut by_kind = BTreeMap::<&'static str, OnlineMinerProductHotKindContributionReport>::new();
    for candidate in selected_candidates {
        let Some(bucket) = buckets.get(&candidate.bucket_key) else {
            continue;
        };
        let gain = online_miner_value_pass_marginal_gain(bucket, &accepted_fingerprints);
        let entry = by_kind.entry(candidate.bucket_kind).or_insert(
            OnlineMinerProductHotKindContributionReport {
                bucket_kind: candidate.bucket_kind,
                ..OnlineMinerProductHotKindContributionReport::default()
            },
        );
        entry.selected_profile_count = entry.selected_profile_count.saturating_add(1);
        entry.marginal_unique_cpu_accepts_over_exact_cache = entry
            .marginal_unique_cpu_accepts_over_exact_cache
            .saturating_add(gain.unique_accepts);
        entry.marginal_nando_cpu_tokens_saved = entry
            .marginal_nando_cpu_tokens_saved
            .saturating_add(gain.tokens_saved);
        entry.marginal_nando_cpu_cost_saved_microusd = entry
            .marginal_nando_cpu_cost_saved_microusd
            .saturating_add(gain.cost_saved_microusd);
        for sample in &bucket.positive_non_exact_samples {
            accepted_fingerprints.insert(sample.request_fingerprint.clone());
        }
    }
    let mut contributions = by_kind.into_values().collect::<Vec<_>>();
    contributions.sort_by(|left, right| {
        right
            .marginal_unique_cpu_accepts_over_exact_cache
            .cmp(&left.marginal_unique_cpu_accepts_over_exact_cache)
            .then_with(|| {
                right
                    .marginal_nando_cpu_tokens_saved
                    .cmp(&left.marginal_nando_cpu_tokens_saved)
            })
            .then_with(|| left.bucket_kind.cmp(right.bucket_kind))
    });
    contributions
}

fn online_miner_multi_split_promotion_candidates(
    buckets: &BTreeMap<String, OnlineMinerBucketState>,
) -> Vec<OnlineMinerPromotionCandidateReport> {
    let mut candidates = online_miner_product_hot_promotion_candidates(buckets)
        .into_iter()
        .filter(|candidate| {
            candidate.bucket_kind == "auto_subcenter"
                || candidate.bucket_kind == "learned_auto_subcenter"
                || candidate.bucket_kind == "hidden_state_split"
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .unique_cpu_accepts_over_exact_cache
            .cmp(&left.unique_cpu_accepts_over_exact_cache)
            .then_with(|| {
                right
                    .nando_cpu_tokens_saved
                    .cmp(&left.nando_cpu_tokens_saved)
            })
            .then_with(|| left.bucket_key.cmp(&right.bucket_key))
    });
    candidates
}

fn online_miner_bucket_is_multi_split(bucket: &OnlineMinerBucketState) -> bool {
    bucket.bucket_kind == "auto_subcenter"
        || bucket.bucket_kind == "learned_auto_subcenter"
        || bucket.bucket_kind == "hidden_state_split"
}

fn online_miner_promotion_package_evidence(
    bucket: &OnlineMinerBucketState,
) -> OnlineMinerPromotionPackageEvidence {
    let Ok(bytes) = std::fs::read(&bucket.active_package_path) else {
        return OnlineMinerPromotionPackageEvidence::default();
    };
    let mut evidence = OnlineMinerPromotionPackageEvidence {
        file_exists: true,
        read_bytes: bytes.len(),
        ..OnlineMinerPromotionPackageEvidence::default()
    };
    let Ok(package_info) = PhaseCenterOffloadRuntime::inspect_package_bytes(&bytes) else {
        return evidence;
    };
    evidence.fingerprint_verified =
        package_info.fingerprint64 == bucket.active_package_fingerprint64;
    evidence.records_verified = package_info.record_count == bucket.package_records;
    let Ok(policy) = PhaseCenterOffloadPolicy::new(bucket.safe_accept_margin_threshold_micro)
    else {
        return evidence;
    };
    evidence.reload_verified =
        PhaseCenterOffloadRuntime::from_package_bytes(&bytes, policy).is_ok();
    evidence
}

fn online_miner_audit_promotion_registry_candidate(
    candidate: &Value,
    shadow_registry_dir: &Path,
    registry_global_gate_clear: bool,
) -> Result<OnlineMinerPromotionRegistryGatePackageReport, String> {
    let bucket_key = json_string(candidate, &["bucket_key"]).unwrap_or_default();
    let task_name =
        json_string(candidate, &["task_name"]).unwrap_or_else(|| sanitize_file_stem(&bucket_key));
    let source_package_path = json_string(candidate, &["package_path"]).unwrap_or_default();
    let source_package_fingerprint64 =
        json_u64(candidate, &["package_fingerprint64"]).unwrap_or_default();
    let source_package_bytes =
        online_miner_json_usize(candidate, &["package_bytes"]).unwrap_or_default();
    let source_package_records =
        online_miner_json_usize(candidate, &["package_records"]).unwrap_or_default();
    let threshold_micro =
        online_miner_json_i64(candidate, &["safe_accept_margin_threshold_micro"]).unwrap_or(1);
    let source_promotion_gate_passed =
        json_bool(candidate, &["promotion_gate_passed"]).unwrap_or(false);
    let false_accepts =
        online_miner_json_usize(candidate, &["false_accepts"]).unwrap_or(usize::MAX);
    let unique_cpu_accepts_over_exact_cache =
        online_miner_json_usize(candidate, &["unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let nando_cpu_tokens_saved =
        online_miner_json_usize(candidate, &["nando_cpu_tokens_saved"]).unwrap_or(0);
    let nando_cpu_cost_saved_microusd =
        json_u64(candidate, &["nando_cpu_cost_saved_microusd"]).unwrap_or(0);
    let mut blockers = Vec::new();
    if !registry_global_gate_clear {
        blockers.push("registry_global_gate_not_clear".to_owned());
    }
    if !source_promotion_gate_passed {
        blockers.push("source_promotion_gate_not_passed".to_owned());
    }
    if false_accepts != 0 {
        blockers.push("false_accepts_nonzero".to_owned());
    }
    if unique_cpu_accepts_over_exact_cache == 0 {
        blockers.push("no_unique_accepts_over_exact_cache".to_owned());
    }
    if json_bool(candidate, &["local_accept_enabled"]).unwrap_or(true) {
        blockers.push("candidate_local_accept_enabled".to_owned());
    }
    if json_bool(candidate, &["auto_promote_enabled"]).unwrap_or(true) {
        blockers.push("candidate_auto_promote_enabled".to_owned());
    }

    let source_path = PathBuf::from(&source_package_path);
    let package_bytes = match std::fs::read(&source_path) {
        Ok(bytes) => bytes,
        Err(_) => {
            blockers.push("package_read_failed".to_owned());
            Vec::new()
        }
    };
    let package_file_exists = !package_bytes.is_empty();
    let mut inspected_package_fingerprint64 = 0u64;
    let mut inspected_package_records = 0usize;
    let mut package_fingerprint_verified = false;
    let mut package_records_verified = false;
    let mut package_reload_verified = false;
    if package_file_exists {
        match PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes) {
            Ok(package_info) => {
                inspected_package_fingerprint64 = package_info.fingerprint64;
                inspected_package_records = package_info.record_count;
                package_fingerprint_verified =
                    package_info.fingerprint64 == source_package_fingerprint64;
                package_records_verified = package_info.record_count == source_package_records;
                if !package_fingerprint_verified {
                    blockers.push("package_fingerprint_mismatch".to_owned());
                }
                if !package_records_verified {
                    blockers.push("package_records_mismatch".to_owned());
                }
            }
            Err(_) => blockers.push("package_inspect_failed".to_owned()),
        }
        match PhaseCenterOffloadPolicy::new(threshold_micro)
            .ok()
            .and_then(|policy| {
                PhaseCenterOffloadRuntime::from_package_bytes(&package_bytes, policy).ok()
            }) {
            Some(_) => package_reload_verified = true,
            None => blockers.push("package_reload_failed".to_owned()),
        }
    }
    let registry_package_path = shadow_registry_dir.join(format!(
        "{}-{}.nwpc",
        sanitize_file_stem(&task_name),
        source_package_fingerprint64
    ));
    let prelim_accept = blockers.is_empty()
        && package_file_exists
        && package_fingerprint_verified
        && package_records_verified
        && package_reload_verified;
    let mut package_readback_exact = false;
    if prelim_accept {
        write_binary_file(&registry_package_path, &package_bytes)?;
        package_readback_exact = std::fs::read(&registry_package_path)
            .map(|readback| readback == package_bytes)
            .unwrap_or(false);
        if !package_readback_exact {
            blockers.push("shadow_registry_copy_mismatch".to_owned());
        }
    }
    Ok(OnlineMinerPromotionRegistryGatePackageReport {
        bucket_key,
        task_name,
        source_package_path,
        registry_package_path: registry_package_path.display().to_string(),
        source_package_fingerprint64,
        inspected_package_fingerprint64,
        source_package_bytes,
        inspected_package_bytes: package_bytes.len(),
        source_package_records,
        inspected_package_records,
        package_file_exists,
        package_readback_exact,
        package_reload_verified,
        package_fingerprint_verified,
        package_records_verified,
        source_promotion_gate_passed,
        false_accepts,
        unique_cpu_accepts_over_exact_cache,
        nando_cpu_tokens_saved,
        nando_cpu_cost_saved_microusd,
        accepted_for_shadow_registry: prelim_accept && package_readback_exact,
        blockers,
    })
}

fn read_online_miner_json_value(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse JSON '{}': {error}", path.display()))
}

fn online_miner_json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    online_miner_json_at(value, path)
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
        })
        .and_then(|value| usize::try_from(value).ok())
}

fn online_miner_json_i64(value: &Value, path: &[&str]) -> Option<i64> {
    online_miner_json_at(value, path).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
    })
}

fn online_miner_json_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn online_miner_forbidden_flags_all_false(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|object| object.values().all(|value| value.as_bool() == Some(false)))
}

fn online_miner_bucket_report(bucket: &OnlineMinerBucketState) -> OnlineMinerBucketReport {
    let auto_calibrated =
        online_miner_auto_calibrated_stats(bucket, bucket.safe_accept_margin_threshold_micro);
    let auto_calibrated_bucket_accepted = auto_calibrated.false_accepts == 0
        && auto_calibrated.unique_cpu_accepts_over_exact_cache > 0;
    let auto_calibrated_bucket_rejected = auto_calibrated.false_accepts > 0;
    let product_hot_candidate = bucket.active_runtime.is_some() && auto_calibrated_bucket_accepted;
    let product_hot_rejected_by_auto_calibration =
        bucket.active_runtime.is_some() && auto_calibrated_bucket_rejected;
    OnlineMinerBucketReport {
        bucket_key: bucket.bucket_key.clone(),
        bucket_kind: bucket.bucket_kind,
        action_family_atom: bucket.action_family_atom.clone(),
        task_name: bucket.task_name.clone(),
        events_seen: bucket.events_seen,
        positive_events: bucket.positive_events,
        negative_events: bucket.negative_events,
        exact_cache_hits: bucket.exact_cache_hits,
        non_exact_positive_events: bucket.non_exact_positive_events,
        total_tokens: bucket.total_tokens,
        total_cost_microusd: bucket.total_cost_microusd,
        value_score: bucket.value_score(),
        active_checkpoint: bucket.active_runtime.is_some(),
        active_runtime_bytes_estimate: bucket.active_runtime_bytes_estimate(),
        reservoir_events: bucket.reservoir_event_count(),
        checkpoints_compiled: bucket.checkpoints_compiled,
        last_compiled_after_row: bucket.last_compiled_after_row,
        safe_accept_margin_threshold_micro: bucket.safe_accept_margin_threshold_micro,
        future_shadow_events: bucket.future_shadow_events,
        local_operator_shadow_decisions: bucket.local_operator_shadow_decisions,
        unique_cpu_accepts_over_exact_cache: bucket.unique_cpu_accepts_over_exact_cache,
        auto_calibration_events: auto_calibrated.calibration_events,
        auto_calibrated_shadow_events_after_calibration: auto_calibrated
            .shadow_events_after_calibration,
        auto_calibrated_margin_threshold_micro: auto_calibrated.threshold_micro,
        auto_calibrated_local_operator_shadow_decisions: auto_calibrated
            .local_operator_shadow_decisions,
        auto_calibrated_unique_cpu_accepts_over_exact_cache: auto_calibrated
            .unique_cpu_accepts_over_exact_cache,
        auto_calibrated_nando_cpu_tokens_saved: auto_calibrated.nando_cpu_tokens_saved,
        auto_calibrated_nando_cpu_cost_saved_microusd: auto_calibrated
            .nando_cpu_cost_saved_microusd,
        auto_calibrated_false_accepts: auto_calibrated.false_accepts,
        auto_calibrated_max_false_margin_micro: auto_calibrated.max_false_margin_micro,
        auto_calibrated_bucket_accepted,
        auto_calibrated_bucket_rejected,
        product_hot_candidate,
        product_hot_rejected_by_auto_calibration,
        runtime_margin_parity_mismatches: bucket.runtime_margin_parity_mismatches,
        false_accepts: bucket.false_accepts,
        wrong_wins: bucket.wrong_wins,
        nando_cpu_tokens_saved: bucket.nando_cpu_tokens_saved,
        nando_cpu_cost_saved_microusd: bucket.nando_cpu_cost_saved_microusd,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token_cost() -> GenericTokenCost {
        GenericTokenCost {
            total_tokens: 15,
            total_cost_microusd: 45,
            evidence_missing: false,
            token_evidence_missing: false,
            cost_evidence_missing: false,
        }
    }

    #[test]
    fn event_split_atoms_include_learnable_compound_subcenters() {
        let request_atoms = vec![
            "topic:git".to_owned(),
            "domain_family:repo".to_owned(),
            "request_mentions_ext:rs".to_owned(),
        ];
        let state_atoms = vec![
            "state_followup_marker:after_command".to_owned(),
            "state_stop_marker:false".to_owned(),
            "state_exit_code_band:zero".to_owned(),
            "state_output_line_band:8_15".to_owned(),
            "state_output_has_error_marker:false".to_owned(),
            "state_output_marker:passed".to_owned(),
        ];
        let tool_atoms = vec![
            "tool_command_kind:cargo".to_owned(),
            "tool_check_kind:test".to_owned(),
        ];
        let route_hint_atoms = vec!["route_hint:manual_should_not_learn".to_owned()];

        let atoms = online_miner_event_split_atoms(
            &request_atoms,
            &state_atoms,
            &tool_atoms,
            &route_hint_atoms,
        );

        assert!(atoms.iter().any(|atom| {
            atom.starts_with("multi2:")
                || atom.starts_with("multi3:")
                || atom.starts_with("multi4:")
        }));
        assert!(
            atoms
                .iter()
                .any(|atom| atom == "state_output_marker:passed")
        );
        assert!(atoms.iter().any(|atom| atom.starts_with("hidden_state:")));
        assert!(!atoms.iter().any(|atom| atom.contains("route_hint:")));
    }

    #[test]
    fn hidden_state_atoms_are_source_neutral_cross_layer_inference() {
        let request_atoms = vec![
            "topic:git".to_owned(),
            "request_route_family:manual_forbidden".to_owned(),
            "target_id:answer_leak".to_owned(),
        ];
        let state_atoms = vec![
            "state_followup_marker:after_command".to_owned(),
            "state_cwd_kind:repo_forbidden".to_owned(),
        ];
        let tool_atoms = vec![
            "tool_command_kind:cargo".to_owned(),
            "tool_mention:codex_forbidden".to_owned(),
            "proof_rule_id:test_leak".to_owned(),
        ];

        let hidden_atoms =
            online_miner_hidden_state_atoms(&request_atoms, &state_atoms, &tool_atoms);
        let bucket_specs = online_miner_event_bucket_specs(
            "action_family:run_check",
            &request_atoms,
            &state_atoms,
            &tool_atoms,
            &[],
            &[],
        );

        assert!(!hidden_atoms.is_empty());
        assert!(
            hidden_atoms
                .iter()
                .all(|atom| online_miner_hidden_state_atom_allowed(atom))
        );
        assert!(
            hidden_atoms
                .iter()
                .all(|atom| { !online_miner_hidden_state_bucket_has_forbidden_source_leak(atom) })
        );
        assert!(
            bucket_specs
                .iter()
                .any(|(kind, key)| *kind == "hidden_state_split"
                    && key.contains("::hidden_state:hidden_state:"))
        );
    }

    #[test]
    fn learned_split_selector_can_promote_hidden_state_atoms_only_under_pressure() {
        let action_family = "action_family:run_check";
        let request_atoms = vec!["topic:git".to_owned()];
        let state_atoms = vec!["state_followup_marker:after_command".to_owned()];
        let tool_atoms = vec!["tool_command_kind:cargo".to_owned()];
        let hidden_atom =
            online_miner_hidden_state_atoms(&request_atoms, &state_atoms, &tool_atoms)
                .into_iter()
                .next()
                .expect("hidden state atom");

        let mut stats_by_action =
            BTreeMap::<String, BTreeMap<String, OnlineMinerSplitAtomStats>>::new();
        for verified_safe_accept in [true, false, true, false] {
            observe_online_miner_split_atoms(
                &mut stats_by_action,
                action_family,
                std::slice::from_ref(&hidden_atom),
                verified_safe_accept,
                false,
                token_cost(),
            );
        }

        let blocked =
            online_miner_learned_split_atoms_for_action(&stats_by_action, action_family, 8, false);
        let selected =
            online_miner_learned_split_atoms_for_action(&stats_by_action, action_family, 8, true);

        assert!(blocked.is_empty());
        assert!(selected.contains(&hidden_atom));
        assert_eq!(online_miner_hidden_state_atom_count(&stats_by_action), 1);
    }

    #[test]
    fn learned_split_selector_can_promote_compound_atoms_from_stream_stats() {
        let action_family = "action_family:run_check";
        let request_atoms = vec![
            "topic:git".to_owned(),
            "domain_family:repo".to_owned(),
            "request_mentions_ext:rs".to_owned(),
        ];
        let state_atoms = vec![
            "state_followup_marker:after_command".to_owned(),
            "state_stop_marker:false".to_owned(),
        ];
        let tool_atoms = vec![
            "tool_command_kind:cargo".to_owned(),
            "tool_check_kind:test".to_owned(),
        ];
        let split_atoms =
            online_miner_event_split_atoms(&request_atoms, &state_atoms, &tool_atoms, &[]);
        let compound_atom = split_atoms
            .iter()
            .find(|atom| atom.starts_with("multi3:") || atom.starts_with("multi4:"))
            .expect("compound split atom is generated")
            .clone();

        let mut stats_by_action =
            BTreeMap::<String, BTreeMap<String, OnlineMinerSplitAtomStats>>::new();
        for verified_safe_accept in [true, false, true, false] {
            observe_online_miner_split_atoms(
                &mut stats_by_action,
                action_family,
                std::slice::from_ref(&compound_atom),
                verified_safe_accept,
                false,
                token_cost(),
            );
        }

        let blocked =
            online_miner_learned_split_atoms_for_action(&stats_by_action, action_family, 8, false);
        let selected =
            online_miner_learned_split_atoms_for_action(&stats_by_action, action_family, 8, true);
        let counts = online_miner_learned_split_atom_counts(&stats_by_action);

        assert!(blocked.is_empty());
        assert!(online_miner_action_family_has_candidate_split_atoms(
            &stats_by_action,
            action_family
        ));
        assert!(selected.contains(&compound_atom));
        assert!(counts.compound() > 0);
        assert_eq!(counts.total(), 1);
    }

    #[test]
    fn learned_split_pressure_comes_from_conflicted_broad_bucket() {
        let action_family = "action_family:run_check";
        let broad_key = format!("{action_family}::broad_action");
        let clean_bucket = OnlineMinerBucketState {
            positive_events: 4,
            negative_events: 4,
            non_exact_positive_events: 3,
            events_seen: 8,
            wrong_wins: 0,
            false_accepts: 0,
            safe_accept_margin_threshold_micro: 1,
            future_decisions: vec![
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "false-calibration".to_owned(),
                    margin_micro: 10,
                    verified_safe_accept: false,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-calibration".to_owned(),
                    margin_micro: 11,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-shadow".to_owned(),
                    margin_micro: 12,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-shadow-2".to_owned(),
                    margin_micro: 13,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
            ],
            ..OnlineMinerBucketState::new(
                broad_key.clone(),
                "broad_action",
                action_family.to_owned(),
                4,
            )
            .expect("clean broad bucket")
        };
        assert!(!online_miner_bucket_has_learned_split_pressure(
            &clean_bucket
        ));

        let conflicted_bucket = OnlineMinerBucketState {
            wrong_wins: 1,
            ..clean_bucket.clone()
        };
        assert!(online_miner_bucket_has_learned_split_pressure(
            &conflicted_bucket
        ));

        let mut buckets = BTreeMap::new();
        buckets.insert(broad_key, conflicted_bucket);
        assert!(online_miner_action_family_has_learned_split_pressure(
            &buckets,
            action_family
        ));
    }

    #[test]
    fn learned_subcenter_checkpoint_priority_beats_raw_auto_fanout() {
        assert!(
            online_miner_checkpoint_kind_priority("learned_auto_subcenter")
                > online_miner_checkpoint_kind_priority("auto_subcenter")
        );
        assert!(
            online_miner_checkpoint_kind_priority("auto_subcenter")
                > online_miner_checkpoint_kind_priority("state_action_signature")
        );
    }

    #[test]
    fn hot_slot_replacement_prefers_stronger_learned_subcenter() {
        let mut buckets = BTreeMap::<String, OnlineMinerBucketState>::new();
        let weak_key = "action_family:x::auto_subcenter:topic:weak".to_owned();
        let strong_key =
            "action_family:x::learned_auto_subcenter:multi2:topic:a|domain_family:b".to_owned();
        let mut weak = OnlineMinerBucketState::new(
            weak_key.clone(),
            "auto_subcenter",
            "action_family:x".to_owned(),
            4,
        )
        .expect("weak bucket");
        weak.positive_events = 4;
        weak.negative_events = 2;
        weak.non_exact_positive_events = 1;
        weak.events_seen = 6;
        weak.total_tokens = 10;
        weak.active_runtime = Some(test_offload_runtime());
        let mut strong = OnlineMinerBucketState::new(
            strong_key.clone(),
            "learned_auto_subcenter",
            "action_family:x".to_owned(),
            4,
        )
        .expect("strong bucket");
        strong.positive_events = 4;
        strong.negative_events = 2;
        strong.non_exact_positive_events = 1;
        strong.events_seen = 6;
        strong.total_tokens = 10;

        buckets.insert(weak_key.clone(), weak);
        buckets.insert(strong_key.clone(), strong);

        let decision = online_miner_prepare_hot_slot_for_bucket(&mut buckets, &strong_key, 1);

        assert!(decision.allowed);
        assert!(decision.replaced_existing_hot_bucket);
        assert!(
            buckets
                .get(&weak_key)
                .expect("weak still present")
                .active_runtime
                .is_none()
        );
    }

    #[test]
    fn auto_calibration_never_lowers_train_safe_floor() {
        let bucket = OnlineMinerBucketState {
            safe_accept_margin_threshold_micro: 300_000,
            future_decisions: vec![
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "false-low".to_owned(),
                    margin_micro: 41,
                    verified_safe_accept: false,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-high".to_owned(),
                    margin_micro: 42,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 10,
                    total_cost_microusd: 30,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-shadow".to_owned(),
                    margin_micro: 42,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 10,
                    total_cost_microusd: 30,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-shadow-2".to_owned(),
                    margin_micro: 43,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 11,
                    total_cost_microusd: 33,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-shadow-3".to_owned(),
                    margin_micro: 10,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 12,
                    total_cost_microusd: 36,
                },
            ],
            ..OnlineMinerBucketState::new(
                "action_family:x::broad_action".to_owned(),
                "broad_action",
                "action_family:x".to_owned(),
                4,
            )
            .expect("bucket")
        };

        let stats = online_miner_auto_calibrated_stats(&bucket, 300_000);

        assert_eq!(stats.threshold_micro, 300_000);
        assert_eq!(stats.unique_cpu_accepts_over_exact_cache, 0);
        assert_eq!(stats.false_accepts, 0);
    }

    #[test]
    fn product_hot_candidate_excludes_rejected_active_bucket() {
        let base = OnlineMinerBucketState::new(
            "action_family:x::learned_auto_subcenter:topic:safe".to_owned(),
            "learned_auto_subcenter",
            "action_family:x".to_owned(),
            4,
        )
        .expect("bucket");
        let safe_bucket = OnlineMinerBucketState {
            active_runtime: Some(test_offload_runtime()),
            safe_accept_margin_threshold_micro: 1,
            future_decisions: vec![
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "false-calibration".to_owned(),
                    margin_micro: 10,
                    verified_safe_accept: false,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-calibration".to_owned(),
                    margin_micro: 10,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 10,
                    total_cost_microusd: 30,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-calibration-2".to_owned(),
                    margin_micro: 10,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-shadow".to_owned(),
                    margin_micro: 11,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 10,
                    total_cost_microusd: 30,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-shadow-2".to_owned(),
                    margin_micro: 12,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 11,
                    total_cost_microusd: 33,
                },
            ],
            ..base
        };
        let rejected_bucket = OnlineMinerBucketState {
            bucket_key: "action_family:x::learned_auto_subcenter:topic:unsafe".to_owned(),
            active_runtime: Some(test_offload_runtime()),
            safe_accept_margin_threshold_micro: 1,
            future_decisions: vec![
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "false-calibration".to_owned(),
                    margin_micro: 10,
                    verified_safe_accept: false,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-calibration".to_owned(),
                    margin_micro: 11,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 10,
                    total_cost_microusd: 30,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "false-shadow".to_owned(),
                    margin_micro: 12,
                    verified_safe_accept: false,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-shadow".to_owned(),
                    margin_micro: 13,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 10,
                    total_cost_microusd: 30,
                },
            ],
            ..OnlineMinerBucketState::new(
                "action_family:x::learned_auto_subcenter:topic:unsafe".to_owned(),
                "learned_auto_subcenter",
                "action_family:x".to_owned(),
                4,
            )
            .expect("bucket")
        };

        let safe_report = online_miner_bucket_report(&safe_bucket);
        let rejected_report = online_miner_bucket_report(&rejected_bucket);

        assert!(safe_report.product_hot_candidate);
        assert!(!safe_report.product_hot_rejected_by_auto_calibration);
        assert!(!rejected_report.product_hot_candidate);
        assert!(rejected_report.product_hot_rejected_by_auto_calibration);
    }

    #[test]
    fn candidate_scoped_global_stats_dedupes_accept_fingerprints() {
        let mut buckets = BTreeMap::<String, OnlineMinerBucketState>::new();
        for suffix in ["a", "b"] {
            let key = format!("action_family:x::learned_auto_subcenter:topic:{suffix}");
            let bucket = OnlineMinerBucketState {
                active_runtime: Some(test_offload_runtime()),
                safe_accept_margin_threshold_micro: 1,
                future_decisions: vec![
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: format!("false-calibration-{suffix}"),
                        margin_micro: 10,
                        verified_safe_accept: false,
                        exact_cache_hit: false,
                        total_tokens: 1,
                        total_cost_microusd: 1,
                    },
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: format!("true-calibration-{suffix}"),
                        margin_micro: 10,
                        verified_safe_accept: true,
                        exact_cache_hit: false,
                        total_tokens: 1,
                        total_cost_microusd: 1,
                    },
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: "same-request".to_owned(),
                        margin_micro: 11,
                        verified_safe_accept: true,
                        exact_cache_hit: false,
                        total_tokens: 10,
                        total_cost_microusd: 30,
                    },
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: format!("exact-cache-{suffix}"),
                        margin_micro: 12,
                        verified_safe_accept: true,
                        exact_cache_hit: true,
                        total_tokens: 100,
                        total_cost_microusd: 300,
                    },
                ],
                ..OnlineMinerBucketState::new(
                    key.clone(),
                    "learned_auto_subcenter",
                    "action_family:x".to_owned(),
                    4,
                )
                .expect("bucket")
            };
            buckets.insert(key, bucket);
        }

        let candidates = online_miner_multi_split_promotion_candidates(&buckets);
        let stats = online_miner_global_auto_calibrated_stats_for_candidates(&buckets, &candidates);

        assert_eq!(candidates.len(), 2);
        assert_eq!(stats.unique_cpu_accepts_over_exact_cache, 1);
        assert_eq!(stats.duplicate_accept_rows, 1);
        assert_eq!(stats.nando_cpu_tokens_saved, 10);
        assert_eq!(stats.false_accepts, 0);
    }

    #[test]
    fn multi_split_portfolio_excludes_clean_broad_bucket() {
        let mut buckets = BTreeMap::<String, OnlineMinerBucketState>::new();
        let package_bytes = test_offload_package_bytes();
        let package_info =
            PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes).expect("package info");
        let package_dir = "target/nando-wave/test";
        std::fs::create_dir_all(package_dir).expect("test package dir");
        let package_path = format!(
            "{package_dir}/multi-split-excludes-broad-{}-{}.candidate.nwpc",
            std::process::id(),
            stable_fingerprint(["multi_split_portfolio_excludes_clean_broad_bucket"])
        );
        std::fs::write(&package_path, &package_bytes).expect("write package");

        for (bucket_kind, suffix, fingerprint, tokens) in [
            ("broad_action", "broad", "broad-shadow", 100usize),
            ("learned_auto_subcenter", "split", "split-shadow", 10usize),
        ] {
            let key = format!("action_family:x::{bucket_kind}:{suffix}");
            let bucket = OnlineMinerBucketState {
                active_runtime: Some(test_offload_runtime()),
                active_package_path: package_path.clone(),
                active_package_fingerprint64: package_info.fingerprint64,
                package_bytes: package_bytes.len(),
                package_records: package_info.record_count,
                safe_accept_margin_threshold_micro: 1,
                future_decisions: vec![
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: format!("{suffix}-false-calibration"),
                        margin_micro: 10,
                        verified_safe_accept: false,
                        exact_cache_hit: false,
                        total_tokens: 1,
                        total_cost_microusd: 1,
                    },
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: format!("{suffix}-true-calibration"),
                        margin_micro: 10,
                        verified_safe_accept: true,
                        exact_cache_hit: false,
                        total_tokens: 1,
                        total_cost_microusd: 1,
                    },
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: fingerprint.to_owned(),
                        margin_micro: 11,
                        verified_safe_accept: true,
                        exact_cache_hit: false,
                        total_tokens: tokens,
                        total_cost_microusd: tokens as u64,
                    },
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: format!("{suffix}-exact-cache-shadow"),
                        margin_micro: 12,
                        verified_safe_accept: true,
                        exact_cache_hit: true,
                        total_tokens: 999,
                        total_cost_microusd: 999,
                    },
                ],
                ..OnlineMinerBucketState::new(
                    key.clone(),
                    bucket_kind,
                    "action_family:x".to_owned(),
                    4,
                )
                .expect("bucket")
            };
            buckets.insert(key, bucket);
        }

        let stats = online_miner_multi_split_global_auto_calibrated_stats(&buckets);
        let candidates = online_miner_multi_split_promotion_candidates(&buckets);

        assert_eq!(stats.unique_cpu_accepts_over_exact_cache, 1);
        assert_eq!(stats.nando_cpu_tokens_saved, 10);
        assert_eq!(stats.false_accepts, 0);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].bucket_kind, "learned_auto_subcenter");
        assert_eq!(candidates[0].unique_cpu_accepts_over_exact_cache, 1);
    }

    #[test]
    fn product_hot_budget_candidates_keep_bounded_multi_split_profiles() {
        let mut buckets = BTreeMap::<String, OnlineMinerBucketState>::new();
        let package_bytes = test_offload_package_bytes();
        let package_info =
            PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes).expect("package info");
        let package_dir = "target/nando-wave/test";
        std::fs::create_dir_all(package_dir).expect("test package dir");
        let package_path = format!(
            "{package_dir}/product-hot-budget-bounded-{}-{}.candidate.nwpc",
            std::process::id(),
            stable_fingerprint(["product_hot_budget_candidates_keep_bounded_multi_split_profiles"])
        );
        std::fs::write(&package_path, &package_bytes).expect("write package");

        for index in 0..6usize {
            let key = format!("action_family:x::learned_auto_subcenter:topic:{index}");
            let bucket = OnlineMinerBucketState {
                active_runtime: Some(test_offload_runtime()),
                active_package_path: package_path.clone(),
                active_package_fingerprint64: package_info.fingerprint64,
                package_bytes: package_bytes.len(),
                package_records: package_info.record_count,
                safe_accept_margin_threshold_micro: 1,
                future_decisions: vec![
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: format!("{index}-false-calibration"),
                        margin_micro: 10,
                        verified_safe_accept: false,
                        exact_cache_hit: false,
                        total_tokens: 1,
                        total_cost_microusd: 1,
                    },
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: format!("{index}-true-calibration"),
                        margin_micro: 10,
                        verified_safe_accept: true,
                        exact_cache_hit: false,
                        total_tokens: 1,
                        total_cost_microusd: 1,
                    },
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: format!("shadow-{index}"),
                        margin_micro: 11,
                        verified_safe_accept: true,
                        exact_cache_hit: false,
                        total_tokens: index + 1,
                        total_cost_microusd: (index + 1) as u64,
                    },
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: format!("shadow-{index}-cache"),
                        margin_micro: 12,
                        verified_safe_accept: true,
                        exact_cache_hit: true,
                        total_tokens: 999,
                        total_cost_microusd: 999,
                    },
                ],
                ..OnlineMinerBucketState::new(
                    key.clone(),
                    "learned_auto_subcenter",
                    "action_family:x".to_owned(),
                    4,
                )
                .expect("bucket")
            };
            buckets.insert(key, bucket);
        }

        let multi_split_candidates = online_miner_multi_split_promotion_candidates(&buckets);
        let product_hot_candidates = online_miner_product_hot_budget_candidates_with_buckets(
            Some(&buckets),
            &multi_split_candidates,
        );
        let stats = online_miner_global_auto_calibrated_stats_for_candidates(
            &buckets,
            &product_hot_candidates,
        );

        assert_eq!(multi_split_candidates.len(), 6);
        assert_eq!(
            product_hot_candidates.len(),
            6.min(ONLINE_MINER_PRODUCT_MAX_HOT_PROFILES_PER_WORKER)
        );
        assert_eq!(stats.unique_cpu_accepts_over_exact_cache, 6);
        assert_eq!(stats.nando_cpu_tokens_saved, 6 + 5 + 4 + 3 + 2 + 1);
        assert_eq!(stats.false_accepts, 0);
    }

    #[test]
    fn product_hot_budget_candidates_use_marginal_unique_accepts() {
        let mut buckets = BTreeMap::<String, OnlineMinerBucketState>::new();
        let package_bytes = test_offload_package_bytes();
        let package_info =
            PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes).expect("package info");
        let package_dir = "target/nando-wave/test";
        std::fs::create_dir_all(package_dir).expect("test package dir");
        let package_path = format!(
            "{package_dir}/product-hot-budget-marginal-{}-{}.candidate.nwpc",
            std::process::id(),
            stable_fingerprint(["product_hot_budget_candidates_use_marginal_unique_accepts"])
        );
        std::fs::write(&package_path, &package_bytes).expect("write package");

        for index in 0..6usize {
            let (bucket_kind, shadow_fingerprint, shadow_tokens) = if index < 4 {
                ("learned_auto_subcenter", "shared-shadow", 100 - index)
            } else {
                (
                    "hidden_state_split",
                    if index == 4 {
                        "hidden-shadow-a"
                    } else {
                        "hidden-shadow-b"
                    },
                    10 + index,
                )
            };
            let key = format!("action_family:x::{bucket_kind}:topic:{index}");
            let bucket = OnlineMinerBucketState {
                active_runtime: Some(test_offload_runtime()),
                active_package_path: package_path.clone(),
                active_package_fingerprint64: package_info.fingerprint64,
                package_bytes: package_bytes.len(),
                package_records: package_info.record_count,
                safe_accept_margin_threshold_micro: 1,
                future_decisions: vec![
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: format!("{index}-false-calibration"),
                        margin_micro: 10,
                        verified_safe_accept: false,
                        exact_cache_hit: false,
                        total_tokens: 1,
                        total_cost_microusd: 1,
                    },
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: format!("{index}-true-calibration"),
                        margin_micro: 10,
                        verified_safe_accept: true,
                        exact_cache_hit: false,
                        total_tokens: 1,
                        total_cost_microusd: 1,
                    },
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: shadow_fingerprint.to_owned(),
                        margin_micro: 11,
                        verified_safe_accept: true,
                        exact_cache_hit: false,
                        total_tokens: shadow_tokens,
                        total_cost_microusd: shadow_tokens as u64,
                    },
                    OnlineMinerFutureDecisionSample {
                        request_fingerprint: format!("{shadow_fingerprint}-tail"),
                        margin_micro: 12,
                        verified_safe_accept: true,
                        exact_cache_hit: true,
                        total_tokens: 999,
                        total_cost_microusd: 999,
                    },
                ],
                ..OnlineMinerBucketState::new(
                    key.clone(),
                    bucket_kind,
                    "action_family:x".to_owned(),
                    4,
                )
                .expect("bucket")
            };
            buckets.insert(key, bucket);
        }

        let multi_split_candidates = online_miner_multi_split_promotion_candidates(&buckets);
        let old_top =
            online_miner_product_hot_budget_candidates_by_individual_value(&multi_split_candidates);
        let old_stats =
            online_miner_global_auto_calibrated_stats_for_candidates(&buckets, &old_top);
        let marginal_top = online_miner_product_hot_budget_candidates_with_buckets(
            Some(&buckets),
            &multi_split_candidates,
        );
        let marginal_stats =
            online_miner_global_auto_calibrated_stats_for_candidates(&buckets, &marginal_top);
        let contributions = online_miner_product_hot_kind_contributions(&buckets, &marginal_top);
        let hidden_contribution = contributions
            .iter()
            .find(|contribution| contribution.bucket_kind == "hidden_state_split")
            .expect("hidden_state contribution is reported");
        let total_contribution_unique = contributions
            .iter()
            .map(|contribution| contribution.marginal_unique_cpu_accepts_over_exact_cache)
            .sum::<usize>();

        assert_eq!(
            old_top.len(),
            6.min(ONLINE_MINER_PRODUCT_MAX_HOT_PROFILES_PER_WORKER)
        );
        assert_eq!(old_stats.unique_cpu_accepts_over_exact_cache, 3);
        assert_eq!(old_stats.duplicate_accept_rows, 3);
        assert_eq!(marginal_top.len(), 3);
        assert_eq!(marginal_stats.unique_cpu_accepts_over_exact_cache, 3);
        assert_eq!(marginal_stats.duplicate_accept_rows, 0);
        assert!(
            marginal_top
                .iter()
                .any(|candidate| candidate.bucket_kind == "hidden_state_split")
        );
        assert_eq!(
            total_contribution_unique,
            marginal_stats.unique_cpu_accepts_over_exact_cache
        );
        assert_eq!(hidden_contribution.selected_profile_count, 2);
        assert_eq!(
            hidden_contribution.marginal_unique_cpu_accepts_over_exact_cache,
            2
        );
        assert_eq!(marginal_stats.false_accepts, 0);
    }

    #[test]
    fn value_pass_selector_reports_hidden_state_marginal_contribution() {
        let mut buckets = BTreeMap::<String, OnlineMinerValuePassBucketState>::new();
        for (bucket_kind, key, fingerprint, tokens) in [
            (
                "learned_auto_subcenter",
                "action_family:x::learned_auto_subcenter:shared_a",
                "shared",
                100usize,
            ),
            (
                "learned_auto_subcenter",
                "action_family:x::learned_auto_subcenter:shared_b",
                "shared",
                90usize,
            ),
            (
                "hidden_state_split",
                "action_family:x::hidden_state:hidden_state:request_tool:topic_abcd+tool_command_kind_1234",
                "hidden_unique",
                20usize,
            ),
        ] {
            let mut bucket = OnlineMinerValuePassBucketState::new(
                key.to_owned(),
                bucket_kind,
                "action_family:x".to_owned(),
            );
            bucket.observe(
                format!("{key}:negative"),
                false,
                false,
                GenericTokenCost {
                    total_tokens: 1,
                    total_cost_microusd: 1,
                    evidence_missing: false,
                    token_evidence_missing: false,
                    cost_evidence_missing: false,
                },
            );
            bucket.observe(
                fingerprint.to_owned(),
                true,
                false,
                GenericTokenCost {
                    total_tokens: tokens,
                    total_cost_microusd: tokens as u64,
                    evidence_missing: false,
                    token_evidence_missing: false,
                    cost_evidence_missing: false,
                },
            );
            buckets.insert(key.to_owned(), bucket);
        }

        let selected = online_miner_value_pass_product_hot_candidates(&buckets, 4);
        let selected_keys = selected
            .iter()
            .map(|candidate| candidate.bucket_key.as_str())
            .collect::<Vec<_>>();
        let stats = online_miner_value_pass_global_stats_for_candidates(&buckets, &selected_keys);
        let contributions = online_miner_value_pass_kind_contributions(&buckets, &selected);
        let hidden = contributions
            .iter()
            .find(|contribution| contribution.bucket_kind == "hidden_state_split")
            .expect("hidden state value contribution");

        assert_eq!(selected.len(), 2);
        assert_eq!(stats.unique_cpu_accepts_over_exact_cache, 2);
        assert_eq!(stats.duplicate_accept_rows, 0);
        assert_eq!(hidden.selected_profile_count, 1);
        assert_eq!(hidden.marginal_unique_cpu_accepts_over_exact_cache, 1);
        assert!(
            selected
                .iter()
                .any(|candidate| candidate.bucket_kind == "hidden_state_split")
        );
    }

    #[test]
    fn promotion_candidates_include_only_safe_quarantine_nwpc_buckets() {
        let mut buckets = BTreeMap::<String, OnlineMinerBucketState>::new();
        let safe_key = "action_family:x::learned_auto_subcenter:topic:safe".to_owned();
        let rejected_key = "action_family:x::learned_auto_subcenter:topic:unsafe".to_owned();
        let package_bytes = test_offload_package_bytes();
        let package_info =
            PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes).expect("package info");
        let package_dir = "target/nando-wave/test";
        std::fs::create_dir_all(package_dir).expect("test package dir");
        let safe_package_path = format!(
            "{package_dir}/safe-{}-{}.candidate.nwpc",
            std::process::id(),
            stable_fingerprint(["promotion_candidates_include_only_safe_quarantine_nwpc_buckets"])
        );
        std::fs::write(&safe_package_path, &package_bytes).expect("write safe package");
        let safe_bucket = OnlineMinerBucketState {
            active_runtime: Some(test_offload_runtime()),
            active_package_path: safe_package_path.clone(),
            active_package_fingerprint64: package_info.fingerprint64,
            package_bytes: package_bytes.len(),
            package_records: package_info.record_count,
            safe_accept_margin_threshold_micro: 1,
            future_decisions: vec![
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "false-calibration".to_owned(),
                    margin_micro: 10,
                    verified_safe_accept: false,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-calibration".to_owned(),
                    margin_micro: 10,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-calibration-2".to_owned(),
                    margin_micro: 10,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-shadow".to_owned(),
                    margin_micro: 11,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 10,
                    total_cost_microusd: 30,
                },
            ],
            ..OnlineMinerBucketState::new(
                safe_key.clone(),
                "learned_auto_subcenter",
                "action_family:x".to_owned(),
                4,
            )
            .expect("bucket")
        };
        let rejected_bucket = OnlineMinerBucketState {
            active_runtime: Some(test_offload_runtime()),
            active_package_path: "target/test/unsafe.candidate.nwpc".to_owned(),
            active_package_fingerprint64: 8,
            package_bytes: 128,
            package_records: 1,
            safe_accept_margin_threshold_micro: 1,
            future_decisions: vec![
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "false-calibration".to_owned(),
                    margin_micro: 10,
                    verified_safe_accept: false,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-calibration".to_owned(),
                    margin_micro: 10,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "true-calibration-2".to_owned(),
                    margin_micro: 10,
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
                OnlineMinerFutureDecisionSample {
                    request_fingerprint: "false-shadow".to_owned(),
                    margin_micro: 11,
                    verified_safe_accept: false,
                    exact_cache_hit: false,
                    total_tokens: 1,
                    total_cost_microusd: 1,
                },
            ],
            ..OnlineMinerBucketState::new(
                rejected_key.clone(),
                "learned_auto_subcenter",
                "action_family:x".to_owned(),
                4,
            )
            .expect("bucket")
        };
        buckets.insert(safe_key.clone(), safe_bucket);
        buckets.insert(rejected_key, rejected_bucket);

        let candidates = online_miner_product_hot_promotion_candidates(&buckets);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].bucket_key, safe_key);
        assert_eq!(candidates[0].package_path, safe_package_path);
        assert_eq!(
            candidates[0].package_fingerprint64,
            package_info.fingerprint64
        );
        assert!(candidates[0].package_file_exists);
        assert_eq!(candidates[0].package_read_bytes, package_bytes.len());
        assert!(candidates[0].package_reload_verified);
        assert!(candidates[0].package_fingerprint_verified);
        assert!(candidates[0].package_records_verified);
        assert!(candidates[0].promotion_gate_passed);
        assert_eq!(candidates[0].false_accepts, 0);
        assert!(candidates[0].verifier_bound);
        assert!(candidates[0].quarantine_nwpc);
        assert!(candidates[0].shadow_only);
        assert!(!candidates[0].local_accept_enabled);
        assert!(!candidates[0].auto_promote_enabled);
    }

    fn test_offload_runtime() -> PhaseCenterOffloadRuntime {
        let bytes = test_offload_package_bytes();
        PhaseCenterOffloadRuntime::from_package_bytes(
            &bytes,
            PhaseCenterOffloadPolicy::new(1).expect("policy"),
        )
        .expect("runtime")
    }

    fn test_offload_package_bytes() -> Vec<u8> {
        let mut compiler = PhaseCenterCompiler::new(4, 1).expect("compiler");
        let positive = nando_core::phase_vector_from_atoms(["positive"], 4);
        let negative = nando_core::phase_vector_from_atoms(["negative"], 4);
        compiler
            .add_positive_vector(0, &positive)
            .expect("positive");
        compiler
            .add_negative_vector(0, &negative)
            .expect("negative");
        let flat = compiler.compile().expect("flat");
        flat.to_bytes().expect("bytes")
    }
}
