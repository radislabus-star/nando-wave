use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

use nando_core::{
    PhaseCenterCell, PhaseCenterCompiler, PhaseCenterEvalTask, PhaseCenterFlatRuntime,
    PhaseCenterHotRouteTable, PhaseCenterHotRuntime, PhaseCenterHotScratch,
    PhaseCenterOffloadPolicy, PhaseCenterOffloadRuntime, PhaseCenterPreparedHotRequest,
    PhaseCenterRuntimeBudgetSnapshot, phase_vector_from_atoms,
};
use serde::{Deserialize, Serialize};

mod agent_continue;
mod auto_subcenter;
mod automatic_continuation_split;
mod automatic_discovery_chain_gate;
mod constrained_split_miner;
mod defaults;
mod live_store_adapter;
mod live_store_clean_manifest_admission_gate;
mod live_store_clean_manifest_shadow_registry_billing_request;
mod live_store_clean_manifest_shadow_registry_handoff;
mod live_store_clean_manifest_shadow_registry_replay;
mod live_store_prepared_hot_pack_correlation_sidecar;
mod online_miner_daemon;
mod online_miner_promotion_billing_request;
mod online_miner_targeted_aggregate_admission_gate;
mod online_miner_targeted_aggregate_billing_request;
mod online_miner_targeted_aggregate_gate;
mod online_miner_targeted_aggregate_provider_export_acquisition_pack;
mod online_miner_targeted_aggregate_provider_export_admission;
mod online_miner_targeted_aggregate_provider_export_attestation;
mod online_miner_targeted_aggregate_provider_export_autoscan;
mod online_miner_targeted_split_refinement;
mod online_portfolio_admission_gate;
mod online_portfolio_billing_evidence_contract;
mod online_portfolio_billing_evidence_gate;
mod online_portfolio_billing_request;
mod online_portfolio_billing_request_provider_correlation_backfill;
mod online_portfolio_clean_subset_manifest;
mod online_portfolio_evidence_chain_audit;
mod online_portfolio_np_rescue;
mod online_portfolio_promotion_manifest;
mod online_portfolio_provider_correlation_audit;
mod online_portfolio_provider_export_admission;
mod online_portfolio_provider_export_autoscan;
mod online_portfolio_provider_export_normalize;
mod online_portfolio_provider_export_watch;
mod online_portfolio_runtime_replay;
mod online_portfolio_selector;
mod online_portfolio_selector_billing_request;
mod opportunity_board;
mod phase_atom_live_capture_readiness;
mod phase_atom_trace_sample;
mod provider_billing_request;
mod provider_boundary_append_sink;
mod provider_boundary_billing_capture_contract;
mod provider_boundary_capture_coverage_gate;
mod provider_boundary_capture_request;
mod provider_boundary_codex_token_backfill;
mod provider_boundary_correlation_join;
mod provider_boundary_export_ingest;
mod provider_boundary_live_chain;
mod provider_boundary_live_np_chain;
mod provider_boundary_match_readiness;
mod provider_boundary_np_chain;
mod provider_boundary_phase_atom_capture;
mod provider_boundary_realtrace_token_cost_backfill;
mod provider_export_acquisition_pack;
mod provider_export_evidence_chain;
mod selected_split_nwpc;
mod selected_split_nwpc_admission_gate;
mod selected_split_nwpc_billing_request;
mod selected_split_nwpc_evidence_chain_audit;
mod selected_split_nwpc_loss_audit;
mod selected_split_nwpc_portfolio_select;
mod selected_split_nwpc_promotion_gate;
mod selected_split_nwpc_provider_export_admission;
mod selected_split_nwpc_provider_export_attestation;
mod selected_split_nwpc_provider_export_autoscan;
mod selected_split_nwpc_shadow_replay;
mod selected_split_nwpc_stage_filter;
mod verifier_evidence_join;

pub(crate) use automatic_continuation_split::run_phase_stream_automatic_continuation_split_v1;
pub(crate) use automatic_discovery_chain_gate::run_phase_stream_automatic_discovery_chain_gate_v1;
pub(crate) use constrained_split_miner::run_phase_stream_constrained_split_miner_v1;
use defaults::*;
pub(crate) use live_store_clean_manifest_admission_gate::{
    run_phase_stream_live_store_clean_manifest_admission_gate_v1,
    run_phase_stream_live_store_clean_manifest_live_policy_shadow_review_v1,
    run_phase_stream_live_store_clean_manifest_live_policy_stage_v1,
    run_phase_stream_live_store_clean_manifest_prepared_policy_shadow_review_v1,
};
pub(crate) use live_store_clean_manifest_shadow_registry_billing_request::run_phase_stream_live_store_clean_manifest_shadow_registry_billing_request_v1;
pub(crate) use live_store_clean_manifest_shadow_registry_handoff::run_phase_stream_live_store_clean_manifest_shadow_registry_handoff_v1;
pub(crate) use live_store_clean_manifest_shadow_registry_replay::run_phase_stream_live_store_clean_manifest_shadow_registry_replay_v1;
pub(crate) use live_store_prepared_hot_pack_correlation_sidecar::run_phase_stream_live_store_prepared_hot_pack_correlation_sidecar_v1;
pub(crate) use online_miner_targeted_aggregate_admission_gate::run_phase_stream_online_miner_targeted_aggregate_admission_gate_v1;
pub(crate) use online_miner_targeted_aggregate_billing_request::run_phase_stream_online_miner_targeted_aggregate_billing_request_v1;
pub(crate) use online_miner_targeted_aggregate_gate::run_phase_stream_online_miner_targeted_aggregate_gate_v1;
pub(crate) use online_miner_targeted_aggregate_provider_export_acquisition_pack::run_phase_stream_online_miner_targeted_aggregate_provider_export_acquisition_pack_v1;
pub(crate) use online_miner_targeted_aggregate_provider_export_admission::run_phase_stream_online_miner_targeted_aggregate_provider_export_admission_v1;
pub(crate) use online_miner_targeted_aggregate_provider_export_attestation::run_phase_stream_online_miner_targeted_aggregate_provider_export_attestation_contract_v1;
pub(crate) use online_miner_targeted_aggregate_provider_export_autoscan::run_phase_stream_online_miner_targeted_aggregate_provider_export_autoscan_v1;
pub(crate) use online_miner_targeted_split_refinement::run_phase_stream_online_miner_targeted_split_refinement_v1;
pub(crate) use online_portfolio_admission_gate::run_phase_stream_online_miner_portfolio_admission_gate_v1;
pub(crate) use online_portfolio_billing_evidence_contract::run_phase_stream_online_miner_portfolio_billing_evidence_contract_v1;
pub(crate) use online_portfolio_billing_evidence_gate::run_phase_stream_online_miner_portfolio_billing_evidence_gate_v1;
pub(crate) use online_portfolio_billing_request::run_phase_stream_online_miner_portfolio_billing_request_v1;
pub(crate) use online_portfolio_billing_request_provider_correlation_backfill::run_phase_stream_online_miner_portfolio_billing_request_provider_correlation_backfill_v1;
pub(crate) use online_portfolio_clean_subset_manifest::run_phase_stream_online_miner_portfolio_clean_subset_manifest_v1;
pub(crate) use online_portfolio_evidence_chain_audit::run_phase_stream_online_miner_portfolio_evidence_chain_audit_v1;
pub(crate) use online_portfolio_np_rescue::{
    run_phase_stream_online_miner_portfolio_np_rescue_runtime_replay_v1,
    run_phase_stream_online_miner_portfolio_np_rescue_v1,
};
pub(crate) use online_portfolio_promotion_manifest::run_phase_stream_online_miner_portfolio_promotion_manifest_v1;
pub(crate) use online_portfolio_provider_correlation_audit::run_phase_stream_online_miner_portfolio_provider_correlation_audit_v1;
pub(crate) use online_portfolio_provider_export_admission::run_phase_stream_online_miner_portfolio_provider_export_admission_v1;
pub(crate) use online_portfolio_provider_export_autoscan::run_phase_stream_online_miner_portfolio_provider_export_autoscan_v1;
pub(crate) use online_portfolio_provider_export_normalize::run_phase_stream_online_miner_portfolio_provider_export_normalize_v1;
pub(crate) use online_portfolio_provider_export_watch::run_phase_stream_online_miner_portfolio_provider_export_watch_v1;
pub(crate) use online_portfolio_runtime_replay::{
    run_phase_stream_online_miner_portfolio_future_tail_billing_request_v1,
    run_phase_stream_online_miner_portfolio_future_tail_replay_v1,
    run_phase_stream_online_miner_portfolio_live_tail_billing_request_v1,
    run_phase_stream_online_miner_portfolio_live_tail_score_only_v1,
    run_phase_stream_online_miner_portfolio_runtime_replay_v1,
};
pub(crate) use online_portfolio_selector::run_phase_stream_online_miner_portfolio_selector_v1;
pub(crate) use online_portfolio_selector_billing_request::run_phase_stream_online_miner_portfolio_selector_billing_request_v1;
pub(crate) use opportunity_board::run_phase_stream_opportunity_board_v1;
pub(crate) use phase_atom_live_capture_readiness::run_phase_stream_phase_atom_live_capture_readiness_v1;
pub(crate) use phase_atom_trace_sample::run_phase_stream_phase_atom_trace_sample_v1;
pub(crate) use provider_billing_request::run_phase_stream_phase_atom_frontier_billing_request_v1;
pub(crate) use provider_boundary_append_sink::run_phase_stream_provider_boundary_append_sink_v1;
pub(crate) use provider_boundary_billing_capture_contract::{
    run_phase_stream_provider_boundary_billing_capture_chain_v1,
    run_phase_stream_provider_boundary_billing_capture_contract_v1,
    run_phase_stream_provider_boundary_billing_capture_evidence_gate_v1,
};
pub(crate) use provider_boundary_capture_coverage_gate::run_phase_stream_provider_boundary_capture_coverage_gate_v1;
pub(crate) use provider_boundary_capture_request::run_phase_stream_provider_boundary_capture_request_v1;
pub(crate) use provider_boundary_codex_token_backfill::run_phase_stream_provider_boundary_codex_token_backfill_v1;
pub(crate) use provider_boundary_correlation_join::run_phase_stream_provider_boundary_correlation_join_v1;
pub(crate) use provider_boundary_export_ingest::run_phase_stream_provider_boundary_export_ingest_v1;
pub(crate) use provider_boundary_live_chain::run_phase_stream_provider_boundary_live_chain_v1;
pub(crate) use provider_boundary_live_np_chain::run_phase_stream_provider_boundary_live_np_chain_v1;
pub(crate) use provider_boundary_match_readiness::run_phase_stream_provider_boundary_match_readiness_v1;
pub(crate) use provider_boundary_np_chain::{
    run_phase_stream_provider_boundary_np_chain_from_phase_trace_v1,
    run_phase_stream_provider_boundary_np_chain_v1,
};
pub(crate) use provider_boundary_phase_atom_capture::run_phase_stream_provider_boundary_phase_atom_trace_v1;
pub(crate) use provider_boundary_realtrace_token_cost_backfill::run_phase_stream_provider_boundary_realtrace_token_cost_backfill_v1;
pub(crate) use provider_export_acquisition_pack::run_phase_stream_provider_export_acquisition_pack_v1;
pub(crate) use provider_export_evidence_chain::run_phase_stream_provider_export_evidence_chain_v1;
pub(crate) use selected_split_nwpc::run_phase_stream_selected_split_nwpc_quarantine_v1;
pub(crate) use selected_split_nwpc_admission_gate::run_phase_stream_selected_split_nwpc_admission_gate_v1;
pub(crate) use selected_split_nwpc_billing_request::run_phase_stream_selected_split_nwpc_billing_request_v1;
pub(crate) use selected_split_nwpc_evidence_chain_audit::run_phase_stream_selected_split_nwpc_evidence_chain_audit_v1;
pub(crate) use selected_split_nwpc_loss_audit::run_phase_stream_selected_split_nwpc_loss_audit_v1;
pub(crate) use selected_split_nwpc_portfolio_select::run_phase_stream_selected_split_nwpc_portfolio_select_v1;
pub(crate) use selected_split_nwpc_promotion_gate::run_phase_stream_selected_split_nwpc_promotion_gate_v1;
pub(crate) use selected_split_nwpc_provider_export_admission::run_phase_stream_selected_split_nwpc_provider_export_admission_v1;
pub(crate) use selected_split_nwpc_provider_export_attestation::run_phase_stream_selected_split_nwpc_provider_export_attestation_contract_v1;
pub(crate) use selected_split_nwpc_provider_export_autoscan::run_phase_stream_selected_split_nwpc_provider_export_autoscan_v1;
pub(crate) use selected_split_nwpc_shadow_replay::run_phase_stream_selected_split_nwpc_shadow_replay_v1;
pub(crate) use selected_split_nwpc_stage_filter::run_phase_stream_selected_split_nwpc_stage_filter_v1;
pub(crate) use verifier_evidence_join::run_phase_stream_verifier_evidence_join_v1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum TestOutputLabel {
    Pass,
    Fail,
    CompileError,
    RuntimePanic,
}

impl TestOutputLabel {
    const ALL: [Self; 4] = [
        Self::Pass,
        Self::Fail,
        Self::CompileError,
        Self::RuntimePanic,
    ];

    const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::CompileError => "compile_error",
            Self::RuntimePanic => "runtime_panic",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TestOutputTraceRow {
    event_id: Option<String>,
    trace_id: Option<String>,
    traffic_source: Option<String>,
    command: Option<String>,
    stdout: Option<String>,
    stderr: Option<String>,
    exit_code: Option<i32>,
    source: Option<String>,
    verification_source: Option<String>,
    tool_call_fingerprints: Option<Vec<String>>,
    request_fingerprint: Option<String>,
    provider: Option<String>,
    model_id: Option<String>,
    input_tokens: Option<usize>,
    output_tokens: Option<usize>,
    cached_input_tokens: Option<usize>,
    provider_cost_microusd: Option<u64>,
    exact_cache_hit: Option<bool>,
    synthetic_source: Option<bool>,
    notes: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct RawLogTraceBuildReport {
    report_kind: &'static str,
    profile: &'static str,
    mode: &'static str,
    trace_path: String,
    source: &'static str,
    logs_read: usize,
    rows_written: usize,
    skipped_unclassified_logs: usize,
    raw_output_classified_events: usize,
    verifier_metadata_classified_events: usize,
    synthetic_events: usize,
    label_counts: BTreeMap<String, usize>,
    source_logs: Vec<RawLogSourceReport>,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct RawLogSourceReport {
    path: String,
    label: Option<String>,
    bytes: usize,
    written: bool,
    reason: String,
    fingerprint64: u64,
}

#[derive(Clone, Debug)]
struct ParsedTestOutputEvent {
    command: String,
    traffic_source: String,
    verification_source: String,
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    tool_call_fingerprint_count: usize,
    request_fingerprint: String,
    notes: String,
    provider: Option<String>,
    model_id: Option<String>,
    input_tokens: Option<usize>,
    output_tokens: Option<usize>,
    cached_input_tokens: Option<usize>,
    provider_cost_microusd: Option<u64>,
    explicit_exact_cache_hit: Option<bool>,
    synthetic_source: bool,
    label: TestOutputLabel,
    verifier_evidence: Vec<String>,
    command_signal: Option<String>,
    raw_output_available: bool,
    metadata_verifier_used: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct ModelPriceConfig {
    schema_version: String,
    default_provider: String,
    default_model_id: String,
    input_cost_microusd_per_1k_tokens: u64,
    output_cost_microusd_per_1k_tokens: u64,
    price_source: String,
}

#[derive(Clone, Copy, Debug)]
struct EventTokenCost {
    input_tokens: usize,
    output_tokens: usize,
    cached_input_tokens: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
    token_estimate_used: bool,
    cost_estimate_used: bool,
}

#[derive(Clone, Debug, Serialize)]
struct OnlinePhaseCenterShadowReport {
    report_kind: &'static str,
    profile: &'static str,
    verdict: &'static str,
    mode: &'static str,
    cells: usize,
    proof_scope: String,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    phase_center_runtime_changed: bool,
    nwpc_schema_changed: bool,
    package_written: bool,
    compiler_path: &'static str,
    runtime_path: &'static str,
    verifier: VerifierReport,
    trace: TraceReport,
    candidate: CandidateReport,
    candidate_package: CandidatePackageReport,
    shadow: ShadowReport,
    forbidden_flags: ForbiddenFlags,
    label_counts: BTreeMap<String, usize>,
    verifier_evidence_counts: BTreeMap<String, usize>,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct VerifierReport {
    verifier_name: &'static str,
    verifier_version: &'static str,
    verifier_input_kind: &'static str,
    verifier_evidence_source: &'static str,
    accept_rule: &'static str,
    false_accept_threshold: usize,
}

#[derive(Clone, Debug, Serialize)]
struct TraceReport {
    trace_path: Option<String>,
    generated_default_trace_used: bool,
    total_events: usize,
    parsed_events: usize,
    skipped_unclassified_events: usize,
    raw_output_classified_events: usize,
    verifier_metadata_classified_events: usize,
    synthetic_events: usize,
    train_events: usize,
    heldout_events: usize,
    heldout_uncovered_events: usize,
    explicit_exact_cache_field_available: bool,
    explicit_exact_cache_hits: usize,
    fingerprint_exact_cache_comparison_available: bool,
    fingerprint_exact_cache_hits: usize,
}

#[derive(Clone, Debug, Serialize)]
struct CandidateReport {
    candidate_count: usize,
    candidate_labels: Vec<String>,
    positive_updates: usize,
    negative_updates: usize,
    runtime_record_count: usize,
    runtime_bytes_estimate: usize,
    runtime_serialized_len: usize,
    verifier_bound: bool,
    training_window_fingerprint: u64,
    shadow_report_fingerprint: u64,
}

#[derive(Clone, Debug, Serialize)]
struct CandidatePackageReport {
    package_kind: &'static str,
    package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    inspected_cells: usize,
    inspected_record_count: usize,
    inspected_serialized_len: usize,
    inspected_payload_bytes: usize,
    inspect_matches_runtime: bool,
    load_roundtrip_matches: bool,
    runtime_margin_parity_mismatches: usize,
    quarantine_only: bool,
    serving_profile_artifact: bool,
    promoted: bool,
}

#[derive(Clone, Debug, Serialize)]
struct ShadowReport {
    shadow_events: usize,
    metadata_status_shadow_pass: bool,
    raw_output_shadow_pass: bool,
    metadata_status_heldout_events: usize,
    raw_output_heldout_events: usize,
    metadata_status_verified_accepts: usize,
    raw_output_verified_accepts: usize,
    metadata_status_wrong_wins: usize,
    raw_output_wrong_wins: usize,
    pairwise_wrong_comparisons: usize,
    wrong_wins: usize,
    false_accepts: usize,
    shadow_local_accepts: usize,
    exact_cache_comparison_reported_separately: bool,
    median_margin_micro: i64,
    p10_margin_micro: i64,
    min_margin_micro: i64,
}

#[derive(Clone, Debug, Serialize)]
struct ForbiddenFlags {
    target_id_used: bool,
    proof_rule_id_authority_used: bool,
    concrete_x_lookup_used: bool,
    manual_local_out_t_used: bool,
    hidden_frame_id_or_bind_x_used: bool,
    legacy_backend_used: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseStreamPromotionAuditReport {
    report_kind: &'static str,
    profile: &'static str,
    verdict: &'static str,
    mode: &'static str,
    trace_path: String,
    shadow_report_path: String,
    candidate_package_path: String,
    model_price_config_path: String,
    proof_scope: String,
    metadata_status_claim_allowed: bool,
    raw_output_claim_allowed: bool,
    margin_threshold_micro: i64,
    package: PromotionPackageAudit,
    shadow_gate: PromotionShadowGateAudit,
    evaluation: PromotionEvaluationAudit,
    token_cost_meter: PromotionTokenCostMeter,
    forbidden_flags: ForbiddenFlags,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    local_accept_enabled: bool,
    promoted: bool,
    promotion_eligible: bool,
    billing_evidence_real: bool,
    money_estimate_available: bool,
    market_money_claim_allowed: bool,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct OnlinePhaseCenterDiscoveryReport {
    report_kind: &'static str,
    profile: &'static str,
    mode: &'static str,
    cells: usize,
    trace_paths: Vec<String>,
    candidate_package_dir: String,
    total_rows: usize,
    parsed_events: usize,
    skipped_unclassified_events: usize,
    bucket_count: usize,
    candidate_count: usize,
    accepted_candidate_count: usize,
    total_unique_cpu_accepts_over_exact_cache: usize,
    total_nando_cpu_tokens_saved: usize,
    total_nando_cpu_cost_saved_microusd: u64,
    total_combined_cost_saved_microusd: u64,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    candidates: Vec<DiscoveryCandidateReport>,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct DiscoveryCandidateReport {
    bucket_key: String,
    proof_scope: String,
    package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    events: usize,
    train_events: usize,
    heldout_events: usize,
    candidate_labels: Vec<String>,
    raw_output_events: usize,
    metadata_status_events: usize,
    false_accepts: usize,
    wrong_wins: usize,
    heldout_uncovered_events: usize,
    runtime_margin_parity_mismatches: usize,
    min_margin_micro: i64,
    median_margin_micro: i64,
    p10_margin_micro: i64,
    exact_cache_hits_in_heldout: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    combined_cost_saved_microusd: u64,
    verifier_bound: bool,
    quarantine_only: bool,
    promoted: bool,
    accepted_for_offline_review: bool,
    rejection_reason: String,
}

#[derive(Clone, Debug, Serialize)]
struct OnlinePhaseCenterStreamingDiscoveryReport {
    report_kind: &'static str,
    profile: &'static str,
    mode: &'static str,
    cells: usize,
    min_bucket_events: usize,
    margin_threshold_micro: i64,
    trace_paths: Vec<String>,
    candidate_package_dir: String,
    total_rows: usize,
    parsed_events: usize,
    skipped_unclassified_events: usize,
    bucket_count: usize,
    compiled_bucket_count: usize,
    accepted_bucket_count: usize,
    stream_shadow_events: usize,
    stream_shadow_accepts: usize,
    stream_false_accepts: usize,
    total_unique_cpu_accepts_over_exact_cache: usize,
    total_nando_cpu_tokens_saved: usize,
    total_nando_cpu_cost_saved_microusd: u64,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    buckets: Vec<OnlineDiscoveryBucketReport>,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct OnlineDiscoveryBucketReport {
    bucket_key: String,
    proof_scope: String,
    package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    events_seen: usize,
    precompile_events: usize,
    compiled_after_global_event_index: Option<usize>,
    candidate_labels: Vec<String>,
    raw_output_events: usize,
    metadata_status_events: usize,
    shadow_events: usize,
    shadow_accepts: usize,
    false_accepts: usize,
    wrong_wins: usize,
    shadow_uncovered_events: usize,
    runtime_margin_parity_mismatches: usize,
    min_margin_micro: i64,
    median_margin_micro: i64,
    p10_margin_micro: i64,
    exact_cache_hits_in_shadow: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    verifier_bound: bool,
    quarantine_only: bool,
    promoted: bool,
    accepted_for_online_shadow_review: bool,
    rejection_reason: String,
}

#[derive(Clone, Debug)]
struct OnlineDiscoveryBucketState {
    bucket_key: String,
    proof_scope: String,
    event_indices: Vec<usize>,
    raw_output_events: usize,
    metadata_status_events: usize,
    compiled: Option<OnlineCompiledBucket>,
    shadow_events: usize,
    shadow_accepts: usize,
    false_accepts: usize,
    wrong_wins: usize,
    shadow_uncovered_events: usize,
    runtime_margin_parity_mismatches: usize,
    margins: Vec<i64>,
    exact_cache_hits_in_shadow: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
}

#[derive(Clone, Debug)]
struct OnlineCompiledBucket {
    package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    compiled_after_global_event_index: usize,
    precompile_events: usize,
    candidate_labels: Vec<String>,
    label_to_index: BTreeMap<TestOutputLabel, usize>,
    reference_runtime: PhaseCenterFlatRuntime,
    runtime: PhaseCenterOffloadRuntime,
}

#[derive(Clone, Debug, Serialize)]
struct GenericRealTrafficOnlineDiscoveryReport {
    report_kind: &'static str,
    mode: &'static str,
    bucket_mode: &'static str,
    cells: usize,
    min_bucket_events: usize,
    margin_threshold_micro: i64,
    trace_paths: Vec<String>,
    candidate_package_dir: String,
    total_rows: usize,
    parsed_candidate_events: usize,
    skipped_no_shadow_request: usize,
    skipped_no_verifier_label: usize,
    skipped_legacy_profile_events: usize,
    bucket_count: usize,
    compiled_bucket_count: usize,
    accepted_bucket_count: usize,
    stream_shadow_events: usize,
    stream_shadow_safe_events: usize,
    stream_shadow_accepts: usize,
    stream_false_accepts: usize,
    total_unique_cpu_accepts_over_exact_cache: usize,
    total_nando_cpu_tokens_saved: usize,
    total_nando_cpu_cost_saved_microusd: u64,
    token_cost_evidence_missing_events: usize,
    token_evidence_missing_events: usize,
    cost_evidence_missing_events: usize,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    buckets: Vec<GenericOnlineBucketReport>,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct GenericOnlineBucketReport {
    bucket_key: String,
    route_key: String,
    profile_id: String,
    package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    events_seen: usize,
    precompile_events: usize,
    compiled_after_global_event_index: Option<usize>,
    verifier_true_events: usize,
    verifier_false_events: usize,
    shadow_events: usize,
    shadow_safe_events: usize,
    shadow_accepts: usize,
    false_accepts: usize,
    missed_safe_accepts: usize,
    runtime_margin_parity_mismatches: usize,
    min_margin_micro: i64,
    median_margin_micro: i64,
    p10_margin_micro: i64,
    exact_cache_hits_in_shadow: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    unique_accepts: Vec<GenericAcceptedEventReport>,
    token_cost_evidence_missing_events: usize,
    token_evidence_missing_events: usize,
    cost_evidence_missing_events: usize,
    verifier_bound: bool,
    quarantine_only: bool,
    promoted: bool,
    accepted_for_online_shadow_review: bool,
    rejection_reason: String,
}

#[derive(Clone, Debug, Serialize)]
struct GenericAcceptedEventReport {
    request_fingerprint: String,
    total_tokens: usize,
    total_cost_microusd: u64,
    token_evidence_missing: bool,
    cost_evidence_missing: bool,
}

#[derive(Clone, Debug, Serialize)]
struct GenericFrontierUnionReport {
    report_kind: &'static str,
    mode: &'static str,
    input_report_paths: Vec<String>,
    input_reports: Vec<GenericFrontierInputReport>,
    input_report_count: usize,
    safe_input_report_count: usize,
    excluded_input_report_count: usize,
    combined_unique_cpu_accepts_over_exact_cache: usize,
    combined_nando_cpu_tokens_saved: usize,
    combined_nando_cpu_cost_saved_microusd: u64,
    duplicate_request_fingerprint_count: usize,
    duplicate_token_cost_mismatch_count: usize,
    token_evidence_missing_events: usize,
    cost_evidence_missing_events: usize,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct GenericFrontierInputReport {
    path: String,
    report_kind: String,
    bucket_mode: String,
    cells: usize,
    margin_threshold_micro: i64,
    stream_false_accepts: usize,
    accepted_bucket_count: usize,
    unique_accepts_in_report: usize,
    tokens_saved_in_report: usize,
    cost_saved_microusd_in_report: u64,
    included_in_union: bool,
    exclusion_reason: String,
}

#[derive(Clone, Debug, Serialize)]
struct GenericCpu10GapAuditReport {
    report_kind: &'static str,
    mode: &'static str,
    target_cpu_accepts_over_exact_cache: usize,
    current_frontier_report_path: String,
    current_safe_accepts_over_exact_cache: usize,
    current_safe_tokens_saved: usize,
    current_safe_cost_saved_microusd: u64,
    remaining_accept_gap_to_cpu10: usize,
    trace_paths: Vec<String>,
    total_rows: usize,
    rows_without_shadow_request: usize,
    shadow_request_rows: usize,
    legacy_shadow_request_rows: usize,
    nonlegacy_shadow_request_rows: usize,
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    verifier_missing_rows: usize,
    exact_cache_hits_in_nonlegacy_shadow: usize,
    verifier_true_over_exact_cache_ceiling: usize,
    verifier_true_token_ceiling_over_exact_cache: usize,
    verifier_true_cost_ceiling_microusd_over_exact_cache: u64,
    trace_pool_ceiling_shortfall_to_cpu10: usize,
    additional_verifier_true_over_exact_cache_needed_for_cpu10: usize,
    current_frontier_capture_rate_milli_of_true_ceiling: usize,
    current_trace_pool_can_reach_cpu10_by_scoring_only: bool,
    frontier_reaches_cpu10_accept_target: bool,
    frontier_accepts_exceed_trace_pool_ceiling: bool,
    routes: Vec<GenericCpu10RouteGapReport>,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct GenericCpu10RouteGapReport {
    bucket_key: String,
    route_key: String,
    profile_id: String,
    nonlegacy_shadow_request_rows: usize,
    verifier_true_rows: usize,
    verifier_true_over_exact_cache_ceiling: usize,
    verifier_false_rows: usize,
    verifier_missing_rows: usize,
    exact_cache_hits: usize,
    verifier_true_token_ceiling_over_exact_cache: usize,
    verifier_true_cost_ceiling_microusd_over_exact_cache: u64,
    traffic_share_milli_of_nonlegacy_shadow: usize,
    true_ceiling_share_milli_of_total_true_ceiling: usize,
    recommended_next_action: &'static str,
}

#[derive(Clone, Debug)]
struct GenericCpu10TraceEvent {
    bucket_key: String,
    route_key: String,
    profile_id: String,
    request_fingerprint: String,
    exact_cache_key: String,
    explicit_provider_cache_hit: Option<bool>,
    verified_safe_accept: Option<bool>,
    token_cost: GenericTokenCost,
    legacy: bool,
}

#[derive(Clone, Debug, Default)]
struct GenericCpu10RouteGapState {
    bucket_key: String,
    route_key: String,
    profile_id: String,
    nonlegacy_shadow_request_rows: usize,
    verifier_true_rows: usize,
    verifier_true_over_exact_cache_ceiling: usize,
    verifier_false_rows: usize,
    verifier_missing_rows: usize,
    exact_cache_hits: usize,
    verifier_true_token_ceiling_over_exact_cache: usize,
    verifier_true_cost_ceiling_microusd_over_exact_cache: u64,
}

#[derive(Clone, Debug, Serialize)]
struct GenericShadowRequestGapAuditReport {
    report_kind: &'static str,
    mode: &'static str,
    trace_paths: Vec<String>,
    total_rows: usize,
    distinct_request_fingerprints: usize,
    shadow_request_rows: usize,
    missing_shadow_request_rows: usize,
    missing_shadow_with_token_or_cost_rows: usize,
    missing_shadow_token_ceiling: usize,
    missing_shadow_cost_ceiling_microusd: u64,
    missing_shadow_not_route_candidate_rows: usize,
    missing_shadow_rejected_candidate_rows: usize,
    missing_shadow_builder_rejected_request_side_features_rows: usize,
    missing_shadow_missing_request_signal_rows: usize,
    missing_shadow_missing_context_signal_rows: usize,
    missing_shadow_missing_evidence_signal_rows: usize,
    missing_shadow_missing_verifier_signal_rows: usize,
    scoreable_shadow_rows: usize,
    scoreable_verifier_true_rows: usize,
    scoreable_verifier_false_rows: usize,
    scoreable_verifier_missing_rows: usize,
    scoreable_verifier_true_token_ceiling: usize,
    scoreable_verifier_true_cost_ceiling_microusd: u64,
    route_reports: Vec<GenericShadowRequestGapRouteReport>,
    file_reports: Vec<GenericShadowRequestGapFileReport>,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct GenericShadowRequestGapRouteReport {
    bucket_key: String,
    route_key: String,
    profile_id: String,
    trace_rows: usize,
    shadow_request_rows: usize,
    missing_shadow_request_rows: usize,
    missing_shadow_with_token_or_cost_rows: usize,
    missing_shadow_token_ceiling: usize,
    missing_shadow_cost_ceiling_microusd: u64,
    missing_shadow_not_route_candidate_rows: usize,
    missing_shadow_rejected_candidate_rows: usize,
    missing_shadow_builder_rejected_request_side_features_rows: usize,
    missing_shadow_missing_request_signal_rows: usize,
    missing_shadow_missing_context_signal_rows: usize,
    missing_shadow_missing_evidence_signal_rows: usize,
    missing_shadow_missing_verifier_signal_rows: usize,
    scoreable_verifier_true_rows: usize,
    scoreable_verifier_false_rows: usize,
    scoreable_verifier_missing_rows: usize,
    scoreable_verifier_true_token_ceiling: usize,
    scoreable_verifier_true_cost_ceiling_microusd: u64,
    missing_cost_share_milli_of_total_missing_cost: usize,
    scoreable_true_cost_share_milli_of_total_true_cost: usize,
    recommended_next_action: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct GenericShadowRequestGapFileReport {
    path: String,
    inferred_route_key: String,
    inferred_profile_id: String,
    total_rows: usize,
    shadow_request_rows: usize,
    missing_shadow_request_rows: usize,
    missing_shadow_with_token_or_cost_rows: usize,
    missing_shadow_token_ceiling: usize,
    missing_shadow_cost_ceiling_microusd: u64,
    missing_shadow_not_route_candidate_rows: usize,
    missing_shadow_rejected_candidate_rows: usize,
    missing_shadow_builder_rejected_request_side_features_rows: usize,
    missing_shadow_missing_request_signal_rows: usize,
    missing_shadow_missing_context_signal_rows: usize,
    missing_shadow_missing_evidence_signal_rows: usize,
    missing_shadow_missing_verifier_signal_rows: usize,
    scoreable_verifier_true_rows: usize,
    scoreable_verifier_false_rows: usize,
    scoreable_verifier_missing_rows: usize,
}

#[derive(Clone, Debug, Default)]
struct GenericShadowRequestGapState {
    bucket_key: String,
    route_key: String,
    profile_id: String,
    trace_rows: usize,
    shadow_request_rows: usize,
    missing_shadow_request_rows: usize,
    missing_shadow_with_token_or_cost_rows: usize,
    missing_shadow_token_ceiling: usize,
    missing_shadow_cost_ceiling_microusd: u64,
    missing_shadow_not_route_candidate_rows: usize,
    missing_shadow_rejected_candidate_rows: usize,
    missing_shadow_builder_rejected_request_side_features_rows: usize,
    missing_shadow_missing_request_signal_rows: usize,
    missing_shadow_missing_context_signal_rows: usize,
    missing_shadow_missing_evidence_signal_rows: usize,
    missing_shadow_missing_verifier_signal_rows: usize,
    scoreable_verifier_true_rows: usize,
    scoreable_verifier_false_rows: usize,
    scoreable_verifier_missing_rows: usize,
    scoreable_verifier_true_token_ceiling: usize,
    scoreable_verifier_true_cost_ceiling_microusd: u64,
}

#[derive(Clone, Debug, Serialize)]
struct GenericMiningInputReadinessReport {
    report_kind: &'static str,
    mode: &'static str,
    trace_paths: Vec<String>,
    total_rows: usize,
    shadow_request_rows: usize,
    missing_shadow_request_rows: usize,
    llm_call_object_rows: usize,
    llm_call_string_rows: usize,
    llm_call_boolean_rows: usize,
    llm_call_null_rows: usize,
    tool_fingerprint_rows: usize,
    missing_shadow_rows_with_request_side_atoms: usize,
    missing_shadow_rows_with_only_boolean_llm_call: usize,
    route_family_mining_ready_now: bool,
    required_next_artifact: &'static str,
    file_reports: Vec<GenericMiningInputReadinessFileReport>,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct GenericMiningInputReadinessFileReport {
    path: String,
    total_rows: usize,
    shadow_request_rows: usize,
    missing_shadow_request_rows: usize,
    llm_call_object_rows: usize,
    llm_call_string_rows: usize,
    llm_call_boolean_rows: usize,
    llm_call_null_rows: usize,
    tool_fingerprint_rows: usize,
    missing_shadow_rows_with_request_side_atoms: usize,
    missing_shadow_rows_with_only_boolean_llm_call: usize,
}

#[derive(Clone, Debug, Default)]
struct GenericMiningInputReadinessState {
    total_rows: usize,
    shadow_request_rows: usize,
    missing_shadow_request_rows: usize,
    llm_call_object_rows: usize,
    llm_call_string_rows: usize,
    llm_call_boolean_rows: usize,
    llm_call_null_rows: usize,
    tool_fingerprint_rows: usize,
    missing_shadow_rows_with_request_side_atoms: usize,
    missing_shadow_rows_with_only_boolean_llm_call: usize,
}

#[derive(Clone, Debug, Serialize)]
struct GenericPhaseAtomTraceBuildReport {
    report_kind: &'static str,
    mode: &'static str,
    input_trace_paths: Vec<String>,
    output_trace_path: String,
    total_rows: usize,
    output_rows: usize,
    rows_with_shadow_request: usize,
    rows_with_verifier_label: usize,
    rows_with_token_or_cost: usize,
    rows_with_explicit_request_atoms: usize,
    rows_with_explicit_state_atoms: usize,
    rows_with_explicit_action_atoms: usize,
    rows_with_explicit_tool_atoms: usize,
    rows_with_shadow_payload_atoms: usize,
    rows_with_provider_correlation_keys: usize,
    metadata_only_rows: usize,
    rows_ready_for_route_family_mining: usize,
    rows_ready_for_existing_shadow_scoring: usize,
    rows_missing_state_or_request_atoms: usize,
    rows_missing_action_atoms: usize,
    rows_missing_verifier_label: usize,
    output_atoms_written: usize,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Default)]
struct GenericPhaseAtomTraceBuildState {
    total_rows: usize,
    output_rows: usize,
    rows_with_shadow_request: usize,
    rows_with_verifier_label: usize,
    rows_with_token_or_cost: usize,
    rows_with_explicit_request_atoms: usize,
    rows_with_explicit_state_atoms: usize,
    rows_with_explicit_action_atoms: usize,
    rows_with_explicit_tool_atoms: usize,
    rows_with_shadow_payload_atoms: usize,
    rows_with_provider_correlation_keys: usize,
    metadata_only_rows: usize,
    rows_ready_for_route_family_mining: usize,
    rows_ready_for_existing_shadow_scoring: usize,
    rows_missing_state_or_request_atoms: usize,
    rows_missing_action_atoms: usize,
    rows_missing_verifier_label: usize,
    output_atoms_written: usize,
}

#[derive(Clone, Debug, Serialize)]
struct CodexHistoryPhaseAtomTraceReport {
    report_kind: &'static str,
    mode: &'static str,
    history_path: String,
    output_trace_path: String,
    max_rows: usize,
    history_rows_seen: usize,
    sampled_rows: usize,
    output_rows: usize,
    rows_with_request_atoms: usize,
    rows_with_state_atoms: usize,
    rows_with_action_atoms: usize,
    rows_with_tool_atoms: usize,
    rows_with_verifier_label: usize,
    rows_ready_for_route_family_mining: usize,
    rows_ready_for_action_family_clustering: usize,
    estimated_total_tokens: usize,
    top_action_families: Vec<AtomCountReport>,
    top_tool_atoms: Vec<AtomCountReport>,
    raw_text_written: bool,
    raw_response_text_written: bool,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct AtomCountReport {
    atom: String,
    count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomVerifierNeededRankingReport {
    report_kind: &'static str,
    mode: &'static str,
    input_trace_paths: Vec<String>,
    total_rows: usize,
    rows_with_action_family: usize,
    rows_with_verifier_label: usize,
    exact_cache_hits: usize,
    exact_cache_misses_over_cache: usize,
    exact_cache_overlap_milli: usize,
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    verifier_true_over_exact_cache_ceiling: usize,
    verifier_true_tokens_over_exact_cache_ceiling: usize,
    verifier_true_cost_microusd_over_exact_cache_ceiling: u64,
    rows_missing_verifier_label_over_exact_cache: usize,
    rows_with_shadow_request: usize,
    rows_missing_shadow_request: usize,
    rows_with_result_atoms: usize,
    rows_missing_result_atoms: usize,
    rows_ready_for_action_family_clustering: usize,
    rows_ready_for_route_family_mining: usize,
    rows_ready_for_existing_shadow_scoring: usize,
    estimated_total_tokens: usize,
    estimated_total_cost_microusd: u64,
    token_events: usize,
    provider_cost_events: usize,
    estimated_cost_events: usize,
    cpu10_target_unique_accepts: usize,
    remaining_verifier_true_accept_gap_to_cpu10: usize,
    current_labeled_pool_can_reach_cpu10: bool,
    action_family_count: usize,
    state_action_bucket_count: usize,
    top_action_families: Vec<PhaseAtomActionFamilyRankingReport>,
    top_value_action_families: Vec<PhaseAtomActionFamilyRankingReport>,
    top_state_action_buckets: Vec<PhaseAtomStateActionBucketReport>,
    compile_allowed: bool,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomActionFamilyRankingReport {
    action_family: String,
    rows: usize,
    traffic_share_milli: usize,
    estimated_tokens: usize,
    estimated_cost_microusd: u64,
    exact_cache_hits: usize,
    exact_cache_misses_over_cache: usize,
    exact_cache_overlap_milli: usize,
    rows_with_verifier_label: usize,
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    verifier_true_over_exact_cache_ceiling: usize,
    expected_unique_cpu_accepts_over_exact_cache: usize,
    expected_tokens_saved_over_exact_cache: usize,
    expected_cost_saved_microusd_over_exact_cache: u64,
    rows_missing_verifier_label: usize,
    rows_missing_verifier_label_over_exact_cache: usize,
    token_events: usize,
    provider_cost_events: usize,
    estimated_cost_events: usize,
    rows_with_shadow_request: usize,
    rows_missing_shadow_request: usize,
    rows_with_result_atoms: usize,
    rows_missing_result_atoms: usize,
    rows_ready_for_action_family_clustering: usize,
    rows_ready_for_route_family_mining: usize,
    rows_ready_for_existing_shadow_scoring: usize,
    rows_with_tool_atoms: usize,
    distinct_state_action_buckets: usize,
    top_route_hints: Vec<AtomCountReport>,
    top_tool_atoms: Vec<AtomCountReport>,
    recommended_verifier_capture: &'static str,
    recommended_next_action: &'static str,
    false_accept_risk: &'static str,
    daemon_next_action: &'static str,
    compile_allowed: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomStateActionBucketReport {
    bucket_key: String,
    action_family: String,
    rows: usize,
    estimated_tokens: usize,
    estimated_cost_microusd: u64,
    exact_cache_hits: usize,
    exact_cache_misses_over_cache: usize,
    exact_cache_overlap_milli: usize,
    rows_with_verifier_label: usize,
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    verifier_true_over_exact_cache_ceiling: usize,
    expected_tokens_saved_over_exact_cache: usize,
    expected_cost_saved_microusd_over_exact_cache: u64,
    rows_missing_verifier_label: usize,
    rows_missing_verifier_label_over_exact_cache: usize,
    token_events: usize,
    provider_cost_events: usize,
    estimated_cost_events: usize,
    rows_with_shadow_request: usize,
    rows_missing_shadow_request: usize,
    rows_with_result_atoms: usize,
    rows_missing_result_atoms: usize,
    top_route_hints: Vec<AtomCountReport>,
    recommended_next_action: &'static str,
    false_accept_risk: &'static str,
    daemon_next_action: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomActionFamilySeparabilityAuditReport {
    report_kind: &'static str,
    mode: &'static str,
    input_trace_paths: Vec<String>,
    action_family_filter: String,
    base_action_family: String,
    task_name: String,
    total_rows: usize,
    matched_rows: usize,
    positive_rows: usize,
    negative_rows: usize,
    rows_missing_verifier_label: usize,
    distinct_base_atoms: usize,
    top_positive_atoms: Vec<AtomCountReport>,
    top_negative_atoms: Vec<AtomCountReport>,
    top_positive_enriched_atoms: Vec<PhaseAtomLabelEnrichmentReport>,
    top_negative_enriched_atoms: Vec<PhaseAtomLabelEnrichmentReport>,
    max_positive_delta_milli: i64,
    max_negative_delta_milli: i64,
    label_balance_milli: usize,
    compile_allowed: bool,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    recommended_next_action: &'static str,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomLabelEnrichmentReport {
    atom: String,
    positive_count: usize,
    negative_count: usize,
    positive_rate_milli: usize,
    negative_rate_milli: usize,
    delta_milli: i64,
}

#[derive(Clone, Debug, Default)]
struct PhaseAtomActionFamilyState {
    action_family: String,
    rows: usize,
    estimated_tokens: usize,
    estimated_cost_microusd: u64,
    exact_cache_hits: usize,
    token_events: usize,
    provider_cost_events: usize,
    estimated_cost_events: usize,
    rows_with_verifier_label: usize,
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    verifier_true_over_exact_cache_ceiling: usize,
    verifier_true_tokens_over_exact_cache_ceiling: usize,
    verifier_true_cost_microusd_over_exact_cache_ceiling: u64,
    rows_missing_verifier_label_over_exact_cache: usize,
    rows_with_shadow_request: usize,
    rows_with_result_atoms: usize,
    rows_ready_for_existing_shadow_scoring: usize,
    rows_ready_for_action_family_clustering: usize,
    rows_ready_for_route_family_mining: usize,
    rows_with_tool_atoms: usize,
    route_hint_counts: BTreeMap<String, usize>,
    tool_atom_counts: BTreeMap<String, usize>,
    state_action_buckets: BTreeSet<String>,
}

#[derive(Clone, Debug, Default)]
struct PhaseAtomStateActionBucketState {
    bucket_key: String,
    action_family: String,
    rows: usize,
    estimated_tokens: usize,
    estimated_cost_microusd: u64,
    exact_cache_hits: usize,
    token_events: usize,
    provider_cost_events: usize,
    estimated_cost_events: usize,
    rows_with_verifier_label: usize,
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    verifier_true_over_exact_cache_ceiling: usize,
    verifier_true_tokens_over_exact_cache_ceiling: usize,
    verifier_true_cost_microusd_over_exact_cache_ceiling: u64,
    rows_missing_verifier_label_over_exact_cache: usize,
    rows_with_shadow_request: usize,
    rows_with_result_atoms: usize,
    route_hint_counts: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Serialize)]
struct CodexSessionRunCheckVerifierTraceReport {
    report_kind: &'static str,
    mode: &'static str,
    sessions_dir: String,
    output_trace_path: String,
    max_events: usize,
    session_files_seen: usize,
    session_files_scanned: usize,
    json_rows_seen: usize,
    exec_command_end_events_seen: usize,
    run_check_events_seen: usize,
    rows_written: usize,
    pass_rows: usize,
    fail_rows: usize,
    compile_error_rows: usize,
    runtime_panic_rows: usize,
    unknown_failure_rows: usize,
    rows_ready_for_route_family_mining: usize,
    rows_with_shadow_request: usize,
    rows_ready_for_existing_shadow_scoring: usize,
    rows_ready_for_action_family_clustering: usize,
    raw_tool_output_written: bool,
    raw_request_text_written: bool,
    raw_response_text_written: bool,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct CodexSessionToolStatusVerifierTraceReport {
    report_kind: &'static str,
    mode: &'static str,
    sessions_dir: String,
    output_trace_path: String,
    max_events: usize,
    selection_policy: &'static str,
    session_files_seen: usize,
    session_files_scanned: usize,
    json_rows_seen: usize,
    exec_command_end_events_seen: usize,
    response_item_tool_call_events_seen: usize,
    response_item_tool_output_events_seen: usize,
    tool_status_events_seen: usize,
    rows_written: usize,
    pass_rows: usize,
    fail_rows: usize,
    compile_error_rows: usize,
    runtime_panic_rows: usize,
    unknown_failure_rows: usize,
    rows_ready_for_route_family_mining: usize,
    rows_with_shadow_request: usize,
    rows_ready_for_existing_shadow_scoring: usize,
    rows_ready_for_action_family_clustering: usize,
    raw_tool_output_written: bool,
    raw_request_text_written: bool,
    raw_response_text_written: bool,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct CodexSessionLiveAppendReport {
    report_kind: &'static str,
    mode: &'static str,
    snapshot_in_progress: bool,
    session_path: String,
    append_trace_path: String,
    poll_ms: u64,
    max_idle_ms: u64,
    max_rows: usize,
    idle_elapsed_ms: u64,
    start_at_end: bool,
    json_rows_seen: usize,
    session_meta_events_seen: usize,
    function_call_events_seen: usize,
    custom_tool_call_events_seen: usize,
    function_call_output_events_seen: usize,
    custom_tool_call_output_events_seen: usize,
    exec_command_end_events_seen: usize,
    tool_status_events_seen: usize,
    rows_written: usize,
    pass_rows: usize,
    fail_rows: usize,
    compile_error_rows: usize,
    runtime_panic_rows: usize,
    unknown_failure_rows: usize,
    skipped_no_payload: usize,
    skipped_unhandled_payload: usize,
    skipped_unlabeled_event: usize,
    last_offset: u64,
    raw_tool_output_written: bool,
    raw_request_text_written: bool,
    raw_response_text_written: bool,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    verdict: &'static str,
    blocker: String,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct CodexSessionsLiveAppendReport {
    report_kind: &'static str,
    mode: &'static str,
    snapshot_in_progress: bool,
    sessions_dir: String,
    append_trace_path: String,
    poll_ms: u64,
    max_idle_ms: u64,
    max_rows: usize,
    max_recent_files: usize,
    idle_elapsed_ms: u64,
    start_at_end: bool,
    session_files_seen: usize,
    active_session_files: usize,
    json_rows_seen: usize,
    session_meta_events_seen: usize,
    function_call_events_seen: usize,
    custom_tool_call_events_seen: usize,
    function_call_output_events_seen: usize,
    custom_tool_call_output_events_seen: usize,
    exec_command_end_events_seen: usize,
    tool_status_events_seen: usize,
    rows_written: usize,
    pass_rows: usize,
    fail_rows: usize,
    compile_error_rows: usize,
    runtime_panic_rows: usize,
    unknown_failure_rows: usize,
    skipped_no_payload: usize,
    skipped_unhandled_payload: usize,
    skipped_unlabeled_event: usize,
    raw_tool_output_written: bool,
    raw_request_text_written: bool,
    raw_response_text_written: bool,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    verdict: &'static str,
    blocker: String,
    boundary: &'static str,
}

#[derive(Clone, Debug)]
struct CodexSessionLiveTailState {
    offset: u64,
    session_id: String,
    tool_call_meta_by_id: BTreeMap<String, SessionToolCallMeta>,
}

#[derive(Clone, Debug, Serialize)]
struct CodexSessionPlanningVerifierTraceReport {
    report_kind: &'static str,
    mode: &'static str,
    sessions_dir: String,
    output_trace_path: String,
    max_events: usize,
    selection_policy: &'static str,
    session_files_seen: usize,
    session_files_scanned: usize,
    json_rows_seen: usize,
    update_plan_call_events_seen: usize,
    update_plan_output_events_seen: usize,
    planning_events_seen: usize,
    rows_written: usize,
    success_rows: usize,
    failure_rows: usize,
    invalid_plan_rows: usize,
    rows_ready_for_route_family_mining: usize,
    rows_with_shadow_request: usize,
    rows_ready_for_existing_shadow_scoring: usize,
    rows_ready_for_action_family_clustering: usize,
    raw_plan_text_written: bool,
    raw_request_text_written: bool,
    raw_response_text_written: bool,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug)]
struct SessionFileEntry {
    path: PathBuf,
    modified_ms: u128,
}

#[derive(Clone, Debug)]
struct SessionRunCheckEvent {
    session_id: String,
    turn_id: String,
    timestamp: String,
    path: PathBuf,
    command: String,
    cwd: String,
    output: String,
    exit_code: i64,
    label: TestOutputLabel,
    evidence: Vec<String>,
    unknown_failure: bool,
}

#[derive(Clone, Debug)]
struct SessionToolCallMeta {
    turn_id: String,
    command: String,
    cwd: String,
}

#[derive(Clone, Debug)]
struct SessionPlanningCallMeta {
    turn_id: String,
    arguments: String,
    plan_shape: PlanningPlanShape,
}

#[derive(Clone, Debug, Default)]
struct PlanningPlanShape {
    step_count: usize,
    pending_count: usize,
    in_progress_count: usize,
    completed_count: usize,
    other_status_count: usize,
    has_explanation: bool,
    valid_schema: bool,
}

#[derive(Clone, Debug)]
struct SessionPlanningEvent {
    session_id: String,
    turn_id: String,
    timestamp: String,
    path: PathBuf,
    arguments: String,
    output: String,
    plan_shape: PlanningPlanShape,
    verified_safe_accept: bool,
    evidence: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomRunCheckDiscoveryReport {
    report_kind: &'static str,
    mode: &'static str,
    action_family: String,
    input_trace_paths: Vec<String>,
    package_path: String,
    cells: usize,
    margin_threshold_micro: i64,
    total_rows: usize,
    parsed_verifier_events: usize,
    positive_pass_events: usize,
    negative_events: usize,
    split_granularity: &'static str,
    train_heldout_time_order_ok: bool,
    train_time_min: String,
    train_time_max: String,
    heldout_time_min: String,
    heldout_time_max: String,
    train_events: usize,
    heldout_events: usize,
    package_fingerprint64: u64,
    package_bytes: usize,
    package_records: usize,
    heldout_accuracy_milli: usize,
    heldout_local_operator_calls: usize,
    heldout_fallback_calls: usize,
    false_accepts: usize,
    wrong_wins: usize,
    runtime_margin_parity_mismatches: usize,
    min_margin_micro: i64,
    median_margin_micro: i64,
    p10_margin_micro: i64,
    exact_cache_hits_in_heldout: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    unique_accepts: Vec<GenericAcceptedEventReport>,
    verifier_bound: bool,
    quarantine_only: bool,
    promoted: bool,
    serving_profile_artifact: bool,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    accepted_for_offline_review: bool,
    rejection_reason: String,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomRunCheckTimeSplitPromotionAuditReport {
    report_kind: &'static str,
    mode: &'static str,
    action_family: String,
    discovery_report_path: String,
    candidate_package_path: String,
    model_price_config_path: String,
    margin_threshold_micro: i64,
    package: PhaseAtomRunCheckTimeSplitPackageAudit,
    discovery_gate: PhaseAtomRunCheckTimeSplitDiscoveryGate,
    unique_accepts: Vec<GenericAcceptedEventReport>,
    economics: PhaseAtomRunCheckTimeSplitEconomicsAudit,
    forbidden_flags: ForbiddenFlags,
    promotion_candidate_allowed: bool,
    product_promotion_allowed: bool,
    local_accept_enabled: bool,
    promoted: bool,
    serving_profile_artifact: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    rejection_reason: String,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomRunCheckTimeSplitPackageAudit {
    package_kind: &'static str,
    package_magic_ok: bool,
    package_fingerprint64: u64,
    package_bytes: usize,
    inspected_cells: usize,
    inspected_record_count: usize,
    report_package_fingerprint64: u64,
    report_package_bytes: usize,
    report_package_records: usize,
    inspect_matches_discovery_report: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomRunCheckTimeSplitDiscoveryGate {
    discovery_report_kind: String,
    discovery_mode: String,
    action_family: String,
    split_granularity: String,
    train_heldout_time_order_ok: bool,
    verifier_bound: bool,
    accepted_for_offline_review: bool,
    quarantine_only: bool,
    discovery_promoted: bool,
    discovery_serving_profile_artifact: bool,
    discovery_local_accept_enabled: bool,
    train_events: usize,
    heldout_events: usize,
    heldout_accuracy_milli: usize,
    heldout_local_operator_calls: usize,
    heldout_fallback_calls: usize,
    false_accepts: usize,
    wrong_wins: usize,
    runtime_margin_parity_mismatches: usize,
    min_margin_micro: i64,
    p10_margin_micro: i64,
    median_margin_micro: i64,
    exact_cache_hits_in_heldout: usize,
    unique_cpu_accepts_over_exact_cache: usize,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomRunCheckTimeSplitEconomicsAudit {
    token_evidence_present: bool,
    provider_cost_evidence_present: bool,
    explicit_model_price_estimate_used: bool,
    price_config_schema_version: String,
    provider: String,
    model_id: String,
    price_source: String,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    estimated_nando_cpu_cost_saved_microusd: u64,
    estimated_cost_method: String,
    projected_nando_calls_saved_milli: usize,
    projected_combined_calls_saved_milli: usize,
    money_claim_blocker: String,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomServingAdmissionAuditReport {
    report_kind: &'static str,
    mode: &'static str,
    action_family: String,
    promotion_audit_report_path: String,
    discovery_report_path: String,
    candidate_package_path: String,
    model_price_config_path: String,
    replay_trace_paths: Vec<String>,
    margin_threshold_micro: i64,
    package: PhaseAtomRunCheckTimeSplitPackageAudit,
    promotion_gate: PhaseAtomServingAdmissionPromotionGate,
    replay: PhaseAtomServingAdmissionReplayAudit,
    economics: PhaseAtomRunCheckTimeSplitEconomicsAudit,
    forbidden_flags: ForbiddenFlags,
    serving_admission_candidate_allowed: bool,
    product_promotion_allowed: bool,
    local_accept_enabled: bool,
    promoted: bool,
    serving_profile_artifact: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    rejection_reason: String,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomServingAdmissionPromotionGate {
    promotion_report_kind: String,
    promotion_mode: String,
    promotion_candidate_allowed: bool,
    promotion_product_promotion_allowed: bool,
    promotion_local_accept_enabled: bool,
    promotion_promoted: bool,
    promotion_serving_profile_artifact: bool,
    promotion_product_runtime_changed: bool,
    promotion_serving_runtime_changed: bool,
    promotion_market_money_claim_allowed: bool,
    promotion_rejection_reason: String,
    promotion_unique_cpu_accepts_over_exact_cache: usize,
    promotion_tokens_saved: usize,
    promotion_estimated_cost_saved_microusd: u64,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomServingAdmissionReplayAudit {
    runtime_package_loaded: bool,
    runtime_cells: usize,
    runtime_record_count: usize,
    runtime_bytes_estimate: usize,
    replay_total_rows: usize,
    replay_parsed_verifier_events: usize,
    replay_train_events: usize,
    replay_heldout_events: usize,
    replay_train_heldout_time_order_ok: bool,
    replay_heldout_accuracy_milli: usize,
    replay_local_operator_calls: usize,
    replay_fallback_calls: usize,
    replay_false_accepts: usize,
    replay_wrong_wins: usize,
    replay_margin_parity_mismatches: usize,
    replay_min_margin_micro: i64,
    replay_median_margin_micro: i64,
    replay_p10_margin_micro: i64,
    replay_exact_cache_hits_in_heldout: usize,
    replay_unique_cpu_accepts_over_exact_cache: usize,
    replay_nando_cpu_tokens_saved: usize,
    replay_nando_cpu_cost_saved_microusd: u64,
    replay_latency_p50_ns: u128,
    replay_latency_p90_ns: u128,
    replay_latency_p99_ns: u128,
    replay_latency_max_ns: u128,
    replay_matches_promotion_accept_count: bool,
    replay_matches_promotion_token_count: bool,
    replay_matches_promotion_cost_or_estimate: bool,
    unique_accepts: Vec<GenericAcceptedEventReport>,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomServingShadowReplayReport {
    report_kind: &'static str,
    mode: &'static str,
    shadow_runtime_kind: &'static str,
    replay_trace_path: String,
    append_watermark_trace_path: Option<String>,
    append_watermark_max_timestamp: Option<String>,
    admission_report_paths: Vec<String>,
    profile_count: usize,
    loaded_profile_count: usize,
    full_trace_replay: bool,
    append_window_replay: bool,
    training_overlap_excluded: bool,
    market_savings_count_allowed: bool,
    progress_output_enabled: bool,
    runtime_budget: PhaseAtomServingRuntimeBudgetReport,
    profiles: Vec<PhaseAtomServingShadowProfileReport>,
    replay: PhaseAtomServingShadowReplayAudit,
    economics: PhaseAtomRunCheckTimeSplitEconomicsAudit,
    forbidden_flags: ForbiddenFlags,
    serving_shadow_replay_allowed: bool,
    product_promotion_allowed: bool,
    local_accept_enabled: bool,
    promoted: bool,
    serving_profile_artifact: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    rejection_reason: String,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomServingShadowProfileReport {
    action_family: String,
    admission_report_path: String,
    candidate_package_path: String,
    admission_candidate_allowed: bool,
    margin_threshold_micro: i64,
    replay_train_events: usize,
    replay_heldout_events: usize,
    package_fingerprint64: u64,
    package_bytes: usize,
    runtime_cells: usize,
    runtime_record_count: usize,
    runtime_bytes_estimate: usize,
    runtime_budget: PhaseAtomServingRuntimeBudgetReport,
    forbidden_flags_clear: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomServingRuntimeBudgetReport {
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
    warm_profile_budget_passed: bool,
    hot_profile_budget_passed: bool,
    hot_byte_budget_passed: bool,
    warm_budget_passed: bool,
    hot_budget_passed: bool,
    product_runtime_budget_passed: bool,
    product_residency_claim_allowed: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomServingShadowReplayAudit {
    total_rows: usize,
    parsed_routable_events: usize,
    append_watermark_routable_events: usize,
    append_watermark_excluded_events: usize,
    excluded_training_overlap_events: usize,
    routed_events: usize,
    unrouted_events: usize,
    local_operator_shadow_decisions: usize,
    fallback_shadow_decisions: usize,
    wrong_wins: usize,
    false_accepts: usize,
    exact_cache_hits_in_routed_events: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    min_margin_micro: i64,
    p10_margin_micro: i64,
    median_margin_micro: i64,
    latency_p50_ns: u128,
    latency_p90_ns: u128,
    latency_p99_ns: u128,
    latency_max_ns: u128,
    unique_accepts: Vec<GenericAcceptedEventReport>,
}

struct PhaseAtomServingShadowRuntimeProfile {
    action_family: String,
    task_name: String,
    admission_candidate_allowed: bool,
    forbidden_flags_clear: bool,
    replay_train_events: usize,
    replay_heldout_events: usize,
    hot_runtime: PhaseCenterHotRuntime,
    hot_routes: PhaseCenterHotRouteTable,
    hot_scratch: PhaseCenterHotScratch,
    runtime_cells: usize,
    runtime_record_count: usize,
}

fn phase_atom_serving_runtime_budget_report(
    snapshot_kind: &'static str,
    snapshot: PhaseCenterRuntimeBudgetSnapshot,
) -> PhaseAtomServingRuntimeBudgetReport {
    PhaseAtomServingRuntimeBudgetReport {
        snapshot_kind,
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
        warm_profile_budget_passed: snapshot.warm_profile_budget_passed,
        hot_profile_budget_passed: snapshot.hot_profile_budget_passed,
        hot_byte_budget_passed: snapshot.hot_byte_budget_passed,
        warm_budget_passed: snapshot.warm_budget_passed(),
        hot_budget_passed: snapshot.hot_budget_passed(),
        product_runtime_budget_passed: snapshot.product_runtime_budget_passed(),
        product_residency_claim_allowed: false,
    }
}

fn phase_atom_serving_budget_snapshot(
    warm_route_count: usize,
    warm_profile_count: usize,
    warm_metadata_bytes_estimate: usize,
    warm_runtime_bytes_estimate: usize,
    hot_route_count: usize,
    hot_profile_count: usize,
    hot_route_profile_edges: usize,
    hot_runtime_bytes_estimate: usize,
    hot_route_table_bytes_estimate: usize,
) -> PhaseCenterRuntimeBudgetSnapshot {
    let warm_bytes_estimate =
        warm_metadata_bytes_estimate.saturating_add(warm_runtime_bytes_estimate);
    let hot_bytes_estimate =
        hot_runtime_bytes_estimate.saturating_add(hot_route_table_bytes_estimate);
    PhaseCenterRuntimeBudgetSnapshot {
        max_hot_profiles_per_worker: DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_PROFILES_PER_WORKER,
        max_hot_bytes_per_worker: DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
        max_warm_profiles_per_process: DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
        max_profiles_per_route: DEFAULT_PHASE_CENTER_SHADOW_MAX_PROFILES_PER_ROUTE,
        max_route_top_k: DEFAULT_PHASE_CENTER_SHADOW_MAX_ROUTE_TOP_K,
        warm_route_count,
        warm_profile_count,
        warm_metadata_bytes_estimate,
        warm_runtime_bytes_estimate,
        warm_bytes_estimate,
        hot_route_count,
        hot_profile_count,
        hot_route_profile_edges,
        hot_runtime_bytes_estimate,
        hot_route_table_bytes_estimate,
        hot_bytes_estimate,
        warm_profile_budget_passed: warm_profile_count
            <= DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS,
        hot_profile_budget_passed: hot_profile_count
            <= DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_PROFILES_PER_WORKER,
        hot_byte_budget_passed: hot_bytes_estimate
            <= DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER,
    }
}

fn phase_atom_serving_shadow_profile_metadata_bytes_estimate(
    action_family: &str,
    task_name: &str,
) -> usize {
    std::mem::size_of::<PhaseAtomServingShadowRuntimeProfile>()
        .saturating_add(action_family.len())
        .saturating_add(task_name.len())
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomLiveAdmissionManifestReport {
    report_kind: &'static str,
    mode: &'static str,
    admission_report_path: String,
    shadow_replay_report_path: String,
    action_family: String,
    candidate_package_path: String,
    margin_threshold_micro: i64,
    package: PhaseAtomLiveAdmissionPackageReport,
    evidence_gate: PhaseAtomLiveAdmissionEvidenceGate,
    economics: PhaseAtomRunCheckTimeSplitEconomicsAudit,
    forbidden_flags: ForbiddenFlags,
    live_accept_eligible: bool,
    live_accept_recommendation: &'static str,
    product_promotion_allowed: bool,
    local_accept_enabled: bool,
    promoted: bool,
    serving_profile_artifact: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    rejection_reason: String,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomLiveAdmissionPackageReport {
    package_kind: &'static str,
    package_magic_ok: bool,
    package_fingerprint64: u64,
    package_bytes: usize,
    inspected_cells: usize,
    inspected_record_count: usize,
    admission_package_fingerprint64: u64,
    shadow_package_fingerprint64: u64,
    package_matches_admission_report: bool,
    package_matches_shadow_report: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomLiveAdmissionEvidenceGate {
    admission_candidate_allowed: bool,
    shadow_replay_allowed: bool,
    shadow_append_or_future_only: bool,
    shadow_market_savings_count_allowed: bool,
    verifier_bound_profile_loaded: bool,
    routed_events: usize,
    exact_cache_hits_in_routed_events: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    projected_nando_calls_saved_milli: usize,
    projected_combined_calls_saved_milli: usize,
    nando_cpu_tokens_saved: usize,
    provider_cost_evidence_present: bool,
    estimated_cost_evidence_present: bool,
    false_accepts: usize,
    wrong_wins: usize,
    p99_latency_ns: u128,
    admission_local_accept_disabled: bool,
    shadow_local_accept_disabled: bool,
    forbidden_flags_clear: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomLiveAdmissionPolicySmokeReport {
    report_kind: &'static str,
    mode: &'static str,
    manifest_report_path: String,
    action_family: String,
    candidate_package_path: String,
    package_fingerprint64: u64,
    package_file_matches_manifest: bool,
    live_accept_eligible_from_manifest: bool,
    policy_decision: &'static str,
    would_local_accepts_over_exact_cache: usize,
    would_tokens_saved: usize,
    would_estimated_cost_saved_microusd: u64,
    false_accepts: usize,
    wrong_wins: usize,
    p99_latency_ns: u128,
    provider_cost_evidence_present: bool,
    estimated_cost_evidence_present: bool,
    market_money_claim_allowed: bool,
    local_accept_enabled: bool,
    serving_runtime_changed: bool,
    product_runtime_changed: bool,
    promoted: bool,
    forbidden_flags: ForbiddenFlags,
    policy_accept_guard: PhaseAtomLiveAdmissionPolicyGuard,
    rejection_reason: String,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomLiveAdmissionPolicyGuard {
    manifest_live_accept_eligible: bool,
    package_file_matches_manifest: bool,
    verifier_bound_profile_loaded: bool,
    package_matches_admission_report: bool,
    package_matches_shadow_report: bool,
    false_accepts_zero: bool,
    wrong_wins_zero: bool,
    unique_cpu_accepts_positive: bool,
    tokens_saved_positive: bool,
    provider_cost_missing_blocks_money_claim: bool,
    local_accept_stays_disabled: bool,
    product_promotion_stays_disabled: bool,
    runtime_stays_unchanged: bool,
    forbidden_flags_clear: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomLiveDaemonShadowGateReport {
    report_kind: &'static str,
    mode: &'static str,
    coverage_scope: String,
    manifest_report_path: String,
    live_trace_path: String,
    exact_cache_watermark_trace_path: String,
    decision_log_path: String,
    profile: PhaseAtomLiveDaemonShadowProfileReport,
    audit: PhaseAtomLiveDaemonShadowAudit,
    fallback_probe: PhaseAtomLiveDaemonFallbackProbe,
    economics: PhaseAtomRunCheckTimeSplitEconomicsAudit,
    forbidden_flags: ForbiddenFlags,
    live_daemon_shadow_gate_passed: bool,
    product_promotion_allowed: bool,
    local_accept_enabled: bool,
    promoted: bool,
    serving_profile_artifact: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    rejection_reason: String,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomLiveDaemonShadowProfileReport {
    action_family: String,
    task_name: String,
    candidate_package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    package_record_count: usize,
    package_file_matches_manifest: bool,
    manifest_live_accept_eligible: bool,
    policy_decision: String,
    runtime_cells: usize,
    runtime_record_count: usize,
    runtime_bytes_estimate: usize,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomLiveDaemonShadowAudit {
    total_rows: usize,
    watermark_routable_events: usize,
    routed_events: usize,
    unrouted_events: usize,
    decision_log_rows: usize,
    local_operator_shadow_decisions: usize,
    fallback_shadow_decisions: usize,
    natural_fallback_rows_observed: bool,
    exact_cache_hits_in_routed_events: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    false_accepts: usize,
    wrong_wins: usize,
    min_margin_micro: i64,
    p10_margin_micro: i64,
    median_margin_micro: i64,
    latency_p50_ns: u128,
    latency_p90_ns: u128,
    latency_p99_ns: u128,
    latency_max_ns: u128,
    process_rss_kib_before_load: Option<usize>,
    process_rss_kib_after_load: Option<usize>,
    process_rss_kib_after_score: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomLiveDaemonFallbackProbe {
    explicit_probe_ran: bool,
    probe_kind: &'static str,
    probe_decision: String,
    probe_margin_micro: i64,
    probe_fell_back: bool,
    natural_fallback_rows_observed: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomLiveDaemonDecisionLogRow {
    row_index: usize,
    event_timestamp: String,
    action_family: String,
    package_fingerprint64: u64,
    decision: String,
    margin_micro: i64,
    verified_safe_accept: bool,
    exact_cache_hit: bool,
    unique_cpu_accept_over_exact_cache: bool,
    total_tokens: usize,
    total_cost_microusd: u64,
    token_evidence_missing: bool,
    cost_evidence_missing: bool,
    request_fingerprint: String,
    local_accept_enabled: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomLiveSelfMiningLoopReport {
    report_kind: &'static str,
    mode: &'static str,
    input_trace_paths: Vec<String>,
    candidate_dir: String,
    cells: usize,
    min_class_events: usize,
    top_n: usize,
    selection_policy: &'static str,
    max_compiled_per_base_action_family: usize,
    margin_threshold_micro: i64,
    train_permille: usize,
    total_rows: usize,
    parsed_verifier_events: usize,
    action_families_seen: usize,
    high_value_classes: usize,
    compiled_quarantine_candidates: usize,
    selected_base_action_families: usize,
    shadow_accepted_candidates: usize,
    aggregate_heldout_local_operator_calls: usize,
    aggregate_heldout_fallback_calls: usize,
    natural_fallback_rows_observed: bool,
    aggregate_unique_cpu_accepts_over_exact_cache: usize,
    aggregate_nando_cpu_tokens_saved: usize,
    aggregate_nando_cpu_cost_saved_microusd: u64,
    process_rss_kib_before: Option<usize>,
    process_rss_kib_after: Option<usize>,
    classes: Vec<PhaseAtomLiveSelfMiningClassReport>,
    economics: PhaseAtomRunCheckTimeSplitEconomicsAudit,
    forbidden_flags: ForbiddenFlags,
    online_learn_enabled: bool,
    online_shadow_enabled: bool,
    auto_promote_enabled: bool,
    local_accept_enabled: bool,
    product_promotion_allowed: bool,
    promoted: bool,
    serving_profile_artifact: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomLiveSelfMiningClassReport {
    action_family: String,
    task_name: String,
    events_seen: usize,
    positive_events: usize,
    negative_events: usize,
    exact_cache_hits: usize,
    exact_cache_overlap_milli: usize,
    non_exact_events: usize,
    total_tokens: usize,
    non_exact_token_ceiling: usize,
    total_cost_microusd: u64,
    verifier_bound: bool,
    high_value_candidate: bool,
    value_score: u128,
    candidate_package_path: String,
    compiled_quarantine_candidate: bool,
    package_fingerprint64: u64,
    package_bytes: usize,
    package_records: usize,
    train_events: usize,
    train_positive_events: usize,
    train_negative_events: usize,
    background_negative_train_events_used: usize,
    background_negative_heldout_events_used: usize,
    heldout_events: usize,
    heldout_positive_events: usize,
    heldout_negative_events: usize,
    heldout_non_exact_positive_events: usize,
    heldout_non_exact_negative_events: usize,
    train_heldout_time_order_ok: bool,
    heldout_accuracy_milli: usize,
    heldout_local_operator_calls: usize,
    heldout_fallback_calls: usize,
    false_accepts: usize,
    wrong_wins: usize,
    runtime_margin_parity_mismatches: usize,
    safe_accept_margin_threshold_micro: i64,
    train_safe_accept_max_false_margin_micro: Option<i64>,
    train_safe_accept_min_true_margin_micro: Option<i64>,
    train_safe_accept_margin_separation_micro: Option<i64>,
    min_margin_micro: i64,
    p10_margin_micro: i64,
    median_margin_micro: i64,
    heldout_missed_safe_accepts: usize,
    exact_cache_hits_in_heldout: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    accepted_heldout_decisions: Vec<PhaseAtomAcceptedDecisionRow>,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    token_evidence_missing_events: usize,
    cost_evidence_missing_events: usize,
    accepted_for_shadow_review: bool,
    recommended_verifier: &'static str,
    recommended_next_action: String,
    rejection_reason: String,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseAtomAcceptedDecisionRow {
    heldout_position: usize,
    event_timestamp: String,
    request_fingerprint: String,
    exact_cache_key: String,
    exact_cache_hit: bool,
    margin_micro: i64,
    token_cost: GenericTokenCost,
}

#[derive(Clone, Copy, Debug)]
struct PhaseAtomLiveSelfMiningClassConfig<'a> {
    candidate_dir: &'a Path,
    cells: usize,
    min_class_events: usize,
    margin_threshold_micro: i64,
    train_permille: usize,
    base_background_events: &'a [PhaseAtomBinaryEvent],
}

#[derive(Clone, Debug)]
struct PhaseAtomBinaryEvent {
    event_timestamp: String,
    request_fingerprint: String,
    external_provider_correlation_keys: Vec<String>,
    verified_safe_accept: bool,
    base_atoms: Vec<String>,
    exact_cache_key: String,
    token_cost: GenericTokenCost,
}

#[derive(Clone, Debug, Serialize)]
struct GenericSeparatorAuditReport {
    report_kind: &'static str,
    mode: &'static str,
    trace_paths: Vec<String>,
    min_true_over_exact: usize,
    top_n: usize,
    parsed_verifier_labeled_events: usize,
    skipped_no_shadow_request: usize,
    skipped_no_verifier_label: usize,
    skipped_legacy_profile_events: usize,
    exact_cache_hits: usize,
    route_summaries: Vec<GenericSeparatorRouteSummaryReport>,
    static_clean_candidate_count: usize,
    top_candidates: Vec<GenericSeparatorCandidateReport>,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct GenericSeparatorRouteSummaryReport {
    bucket_key: String,
    route_key: String,
    profile_id: String,
    events: usize,
    verifier_true_events: usize,
    verifier_false_events: usize,
    exact_cache_hits: usize,
    static_clean_candidate_count: usize,
    best_true_over_exact: usize,
    best_candidate_atom: String,
}

#[derive(Clone, Debug, Serialize)]
struct GenericSeparatorCandidateReport {
    route_key: String,
    profile_id: String,
    atom_family: String,
    atom: String,
    events: usize,
    verifier_true_events: usize,
    verifier_false_events: usize,
    true_over_exact_cache_events: usize,
    exact_cache_hits: usize,
    token_ceiling_over_exact_cache: usize,
    cost_ceiling_microusd_over_exact_cache: u64,
    false_rate_milli: usize,
    static_clean_on_current_labeled_set: bool,
    shortcut_risk: &'static str,
    recommended_next_action: &'static str,
}

#[derive(Clone, Debug, Default)]
struct GenericSeparatorCandidateState {
    route_key: String,
    profile_id: String,
    atom_family: String,
    atom: String,
    events: usize,
    verifier_true_events: usize,
    verifier_false_events: usize,
    true_over_exact_cache_events: usize,
    exact_cache_hits: usize,
    token_ceiling_over_exact_cache: usize,
    cost_ceiling_microusd_over_exact_cache: u64,
}

#[derive(Clone, Debug, Default)]
struct GenericSeparatorRouteState {
    bucket_key: String,
    route_key: String,
    profile_id: String,
    events: usize,
    verifier_true_events: usize,
    verifier_false_events: usize,
    exact_cache_hits: usize,
    static_clean_candidate_count: usize,
    best_true_over_exact: usize,
    best_candidate_atom: String,
}

#[derive(Clone, Debug, Serialize)]
struct GenericGuardedSeparatorShadowReport {
    report_kind: &'static str,
    mode: &'static str,
    bucket_mode: &'static str,
    selector_report_path: String,
    max_guards: usize,
    selected_guard_count: usize,
    selected_guards: Vec<GenericSeparatorGuardSpecReport>,
    cells: usize,
    min_bucket_events: usize,
    margin_threshold_micro: i64,
    trace_paths: Vec<String>,
    candidate_package_dir: String,
    total_rows: usize,
    parsed_candidate_events: usize,
    skipped_no_shadow_request: usize,
    skipped_no_verifier_label: usize,
    skipped_legacy_profile_events: usize,
    bucket_count: usize,
    compiled_bucket_count: usize,
    accepted_bucket_count: usize,
    stream_shadow_events: usize,
    stream_shadow_safe_events: usize,
    stream_shadow_accepts: usize,
    stream_false_accepts: usize,
    total_unique_cpu_accepts_over_exact_cache: usize,
    total_nando_cpu_tokens_saved: usize,
    total_nando_cpu_cost_saved_microusd: u64,
    token_cost_evidence_missing_events: usize,
    token_evidence_missing_events: usize,
    cost_evidence_missing_events: usize,
    buckets: Vec<GenericOnlineBucketReport>,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct GenericGuardedSeparatorSplitShadowReport {
    report_kind: &'static str,
    mode: &'static str,
    bucket_mode: &'static str,
    split_granularity: &'static str,
    global_contiguous_windows: bool,
    selector_source: &'static str,
    selector_permille: usize,
    train_permille: usize,
    selector_event_start: usize,
    selector_event_end: usize,
    train_event_start: usize,
    train_event_end: usize,
    shadow_event_start: usize,
    shadow_event_end: usize,
    selector_events: usize,
    train_events: usize,
    shadow_events_window: usize,
    selector_train_shadow_disjoint: bool,
    shadow_window_independent: bool,
    max_guards: usize,
    selected_guard_count: usize,
    selected_guards: Vec<GenericSeparatorGuardSpecReport>,
    cells: usize,
    min_bucket_events: usize,
    margin_threshold_micro: i64,
    trace_paths: Vec<String>,
    candidate_package_dir: String,
    total_rows: usize,
    parsed_candidate_events: usize,
    skipped_no_shadow_request: usize,
    skipped_no_verifier_label: usize,
    skipped_legacy_profile_events: usize,
    route_train_bucket_count: usize,
    route_compiled_bucket_count: usize,
    bucket_count: usize,
    compiled_bucket_count: usize,
    accepted_bucket_count: usize,
    stream_shadow_events: usize,
    stream_shadow_safe_events: usize,
    stream_shadow_accepts: usize,
    stream_false_accepts: usize,
    total_unique_cpu_accepts_over_exact_cache: usize,
    total_nando_cpu_tokens_saved: usize,
    total_nando_cpu_cost_saved_microusd: u64,
    token_cost_evidence_missing_events: usize,
    token_evidence_missing_events: usize,
    cost_evidence_missing_events: usize,
    buckets: Vec<GenericOnlineBucketReport>,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct GenericGuardedSeparatorCalibratedSplitShadowReport {
    report_kind: &'static str,
    mode: &'static str,
    bucket_mode: &'static str,
    split_granularity: &'static str,
    selector_permille: usize,
    compile_permille: usize,
    calibration_permille: usize,
    selector_events: usize,
    compile_events: usize,
    calibration_events: usize,
    shadow_events_window: usize,
    selector_compile_calibration_shadow_disjoint: bool,
    shadow_window_independent: bool,
    max_guards: usize,
    selected_guard_count: usize,
    selected_guards: Vec<GenericSeparatorGuardSpecReport>,
    cells: usize,
    min_bucket_events: usize,
    calibration_margin_floor_micro: i64,
    calibration_margin_guard_micro: i64,
    trace_paths: Vec<String>,
    candidate_package_dir: String,
    total_rows: usize,
    parsed_candidate_events: usize,
    skipped_no_shadow_request: usize,
    skipped_no_verifier_label: usize,
    skipped_legacy_profile_events: usize,
    route_compile_bucket_count: usize,
    route_compiled_bucket_count: usize,
    bucket_count: usize,
    compiled_bucket_count: usize,
    calibrated_bucket_count: usize,
    accepted_bucket_count: usize,
    stream_shadow_events: usize,
    stream_shadow_safe_events: usize,
    stream_shadow_accepts: usize,
    stream_false_accepts: usize,
    total_unique_cpu_accepts_over_exact_cache: usize,
    total_nando_cpu_tokens_saved: usize,
    total_nando_cpu_cost_saved_microusd: u64,
    token_cost_evidence_missing_events: usize,
    token_evidence_missing_events: usize,
    cost_evidence_missing_events: usize,
    buckets: Vec<GenericCalibratedSplitBucketReport>,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct GenericCalibratedSplitBucketReport {
    #[serde(flatten)]
    bucket: GenericOnlineBucketReport,
    calibrated_margin_threshold_micro: i64,
    calibration_events: usize,
    calibration_safe_events: usize,
    calibration_false_events: usize,
    calibration_accepts: usize,
    calibration_false_accepts: usize,
    calibration_max_false_margin_micro: Option<i64>,
    calibration_min_safe_margin_micro: Option<i64>,
    calibration_threshold_source: &'static str,
}

#[derive(Clone, Debug)]
struct GenericCalibratedBucketState {
    state: GenericOnlineBucketState,
    calibrated_margin_threshold_micro: i64,
    calibration_margins: Vec<(i64, bool)>,
    calibration_events: usize,
    calibration_safe_events: usize,
    calibration_false_events: usize,
    calibration_accepts: usize,
    calibration_false_accepts: usize,
    calibration_max_false_margin_micro: Option<i64>,
    calibration_min_safe_margin_micro: Option<i64>,
    calibration_threshold_source: &'static str,
}

#[derive(Clone, Debug, Default)]
struct GenericRouteLocalFourWaySplit {
    selector_indices: BTreeSet<usize>,
    compile_indices: BTreeSet<usize>,
    calibration_indices: BTreeSet<usize>,
    shadow_indices: BTreeSet<usize>,
    disjoint: bool,
}

#[derive(Clone, Debug, Serialize)]
struct GenericSeparatorGuardSpecReport {
    route_key: String,
    profile_id: String,
    atom_family: String,
    atom: String,
    source_true_over_exact_cache_events: usize,
    source_verifier_false_events: usize,
    source_shortcut_risk: String,
    source_recommended_next_action: String,
}

#[derive(Clone, Debug)]
struct GenericSeparatorGuardSpec {
    route_key: String,
    profile_id: String,
    atom_family: String,
    atom: String,
    source_true_over_exact_cache_events: usize,
    source_verifier_false_events: usize,
    source_shortcut_risk: String,
    source_recommended_next_action: String,
}

#[derive(Clone, Debug, Serialize)]
struct GenericCostEvidenceAuditReport {
    report_kind: &'static str,
    mode: &'static str,
    trace_paths: Vec<String>,
    total_rows: usize,
    shadow_request_rows: usize,
    skipped_legacy_profile_events: usize,
    nonlegacy_candidate_rows: usize,
    no_verifier_label_rows: usize,
    verifier_true_events: usize,
    verifier_false_events: usize,
    provider_cost_events: usize,
    estimated_cost_events: usize,
    token_events: usize,
    token_or_cost_events: usize,
    verifier_bound_token_or_cost_events: usize,
    compile_ready_bucket_count: usize,
    money_proof_candidate_bucket_count: usize,
    buckets: Vec<GenericCostEvidenceBucketReport>,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct GenericCostEvidenceBucketReport {
    bucket_key: String,
    route_key: String,
    profile_id: String,
    candidate_rows: usize,
    verifier_true_events: usize,
    verifier_false_events: usize,
    no_verifier_label_events: usize,
    provider_cost_events: usize,
    estimated_cost_events: usize,
    token_events: usize,
    token_or_cost_events: usize,
    verifier_true_token_or_cost_events: usize,
    verifier_false_token_or_cost_events: usize,
    verifier_true_cost_events: usize,
    verifier_false_cost_events: usize,
    can_compile_phase_center: bool,
    can_measure_money: bool,
    recommended_next_action: &'static str,
}

#[derive(Clone, Debug, Default)]
struct GenericCostEvidenceBucketState {
    bucket_key: String,
    route_key: String,
    profile_id: String,
    candidate_rows: usize,
    verifier_true_events: usize,
    verifier_false_events: usize,
    no_verifier_label_events: usize,
    provider_cost_events: usize,
    estimated_cost_events: usize,
    token_events: usize,
    token_or_cost_events: usize,
    verifier_true_token_or_cost_events: usize,
    verifier_false_token_or_cost_events: usize,
    verifier_true_cost_events: usize,
    verifier_false_cost_events: usize,
}

#[derive(Clone, Debug, Serialize)]
struct GenericTraceTokenCostEnrichmentReport {
    report_kind: &'static str,
    mode: &'static str,
    readiness_report_path: String,
    output_dir: String,
    input_trace_paths: Vec<String>,
    readiness_rows: usize,
    readiness_rows_with_fingerprint: usize,
    readiness_rows_with_tokens: usize,
    readiness_rows_with_cost: usize,
    input_rows: usize,
    output_rows: usize,
    rows_with_shadow_request: usize,
    matched_rows: usize,
    rows_enriched_tokens: usize,
    rows_enriched_cost: usize,
    output_files: Vec<GenericTraceTokenCostEnrichmentFileReport>,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct GenericTraceTokenCostEnrichmentFileReport {
    input_path: String,
    output_path: String,
    input_rows: usize,
    output_rows: usize,
    rows_with_shadow_request: usize,
    matched_rows: usize,
    rows_enriched_tokens: usize,
    rows_enriched_cost: usize,
}

#[derive(Clone, Debug, Serialize)]
struct ProviderBillingEvidenceJoinReport {
    report_kind: &'static str,
    mode: &'static str,
    provider_billing_evidence_path: String,
    output_dir: String,
    input_trace_paths: Vec<String>,
    billing_rows: usize,
    billing_rows_with_match_key: usize,
    billing_rows_with_provider_cost: usize,
    duplicate_billing_keys: usize,
    input_rows: usize,
    output_rows: usize,
    rows_with_shadow_request: usize,
    matched_rows: usize,
    rows_enriched_provider_cost: usize,
    rows_enriched_tokens: usize,
    output_files: Vec<ProviderBillingEvidenceJoinFileReport>,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct ProviderBillingEvidenceJoinFileReport {
    input_path: String,
    output_path: String,
    input_rows: usize,
    output_rows: usize,
    rows_with_shadow_request: usize,
    matched_rows: usize,
    rows_enriched_provider_cost: usize,
    rows_enriched_tokens: usize,
}

#[derive(Clone, Debug)]
struct ProviderBillingEvidence {
    billing_evidence_id: String,
    billing_source: String,
    provider: Option<String>,
    model_id: Option<String>,
    input_tokens: Option<usize>,
    output_tokens: Option<usize>,
    cached_input_tokens: Option<usize>,
    total_tokens: Option<usize>,
    provider_cost_microusd: Option<u64>,
}

#[derive(Clone, Debug, Default)]
struct ProviderBillingEvidenceMap {
    by_key: BTreeMap<String, ProviderBillingEvidence>,
    billing_rows: usize,
    billing_rows_with_match_key: usize,
    billing_rows_with_provider_cost: usize,
    duplicate_billing_keys: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct ReadinessTokenCostEvidence {
    estimated_total_tokens: Option<usize>,
    estimated_total_cost_microusd: Option<u64>,
    token_cost_estimate_used: Option<bool>,
}

#[derive(Clone, Copy, Debug, Default)]
struct ReadinessTokenCostSummary {
    readiness_rows: usize,
    readiness_rows_with_fingerprint: usize,
    readiness_rows_with_tokens: usize,
    readiness_rows_with_cost: usize,
}

#[derive(Clone, Debug)]
struct GenericRealTrafficEvent {
    route_key: String,
    profile_id: String,
    traffic_source: String,
    verification_source: String,
    request_fingerprint: String,
    exact_cache_key: String,
    explicit_provider_cache_hit: Option<bool>,
    verified_safe_accept: bool,
    expect_local_operator: Option<bool>,
    active_fringe: Vec<(u64, i64)>,
    slot_summary: Vec<String>,
    tool_call_fingerprint_count: usize,
    input_tokens: Option<usize>,
    output_tokens: Option<usize>,
    cached_input_tokens: Option<usize>,
    estimated_total_tokens: Option<usize>,
    provider_cost_microusd: Option<u64>,
    estimated_total_cost_microusd: Option<u64>,
}

#[derive(Clone, Debug)]
struct GenericOnlineBucketState {
    bucket_key: String,
    route_key: String,
    profile_id: String,
    event_indices: Vec<usize>,
    verifier_true_events: usize,
    verifier_false_events: usize,
    compiled: Option<GenericCompiledBucket>,
    shadow_events: usize,
    shadow_safe_events: usize,
    shadow_accepts: usize,
    false_accepts: usize,
    missed_safe_accepts: usize,
    runtime_margin_parity_mismatches: usize,
    margins: Vec<i64>,
    exact_cache_hits_in_shadow: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    unique_accepts: BTreeMap<String, GenericAcceptedEventReport>,
    token_cost_evidence_missing_events: usize,
    token_evidence_missing_events: usize,
    cost_evidence_missing_events: usize,
}

#[derive(Clone, Debug)]
struct GenericCompiledBucket {
    package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    compiled_after_global_event_index: usize,
    precompile_events: usize,
    reference_runtime: PhaseCenterFlatRuntime,
    runtime: PhaseCenterOffloadRuntime,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct GenericTokenCost {
    total_tokens: usize,
    total_cost_microusd: u64,
    evidence_missing: bool,
    token_evidence_missing: bool,
    cost_evidence_missing: bool,
}

#[derive(Clone, Debug)]
enum GenericParseResult {
    Event(Box<GenericRealTrafficEvent>),
    NoShadowRequest,
    NoVerifierLabel,
    LegacyProfile,
}

#[derive(Clone, Copy, Debug)]
enum GenericBucketMode {
    Route,
    RequestShape,
    ActionFamily,
    StateAction,
}

impl GenericBucketMode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Route => "route",
            Self::RequestShape => "request_shape_v1",
            Self::ActionFamily => "action_family_v1",
            Self::StateAction => "state_action_v1",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct PromotionPackageAudit {
    package_kind: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    inspected_cells: usize,
    inspected_record_count: usize,
    inspect_matches_shadow_report: bool,
    quarantine_only: bool,
    serving_profile_artifact: bool,
    promoted: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PromotionShadowGateAudit {
    shadow_verdict: String,
    proof_scope: String,
    metadata_status_shadow_pass: bool,
    raw_output_shadow_pass: bool,
    metadata_status_verified_accepts: usize,
    raw_output_verified_accepts: usize,
    generated_default_trace_used: bool,
    synthetic_events: usize,
    verifier_bound: bool,
    false_accepts: usize,
    wrong_wins: usize,
    heldout_uncovered_events: usize,
    runtime_margin_parity_mismatches: usize,
    local_accept_enabled: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PromotionEvaluationAudit {
    evaluation_scope: &'static str,
    parsed_events: usize,
    train_events: usize,
    heldout_events: usize,
    heldout_uncovered_events: usize,
    exact_cache_hits_in_heldout: usize,
    verified_shadow_accepts: usize,
    unique_cpu_accepts_over_exact_cache: usize,
    audit_false_accepts: usize,
    median_margin_micro: i64,
    p10_margin_micro: i64,
    min_margin_micro: i64,
    projected_nando_calls_saved_milli: usize,
    projected_combined_calls_saved_milli: usize,
}

#[derive(Clone, Debug, Serialize)]
struct PromotionTokenCostMeter {
    token_cost_estimate_used: bool,
    token_source: String,
    price_config_schema_version: String,
    provider: String,
    model_id: String,
    price_source: String,
    input_cost_microusd_per_1k_tokens: u64,
    output_cost_microusd_per_1k_tokens: u64,
    real_token_rows: usize,
    estimated_token_rows: usize,
    provider_cost_rows: usize,
    estimated_cost_rows: usize,
    total_baseline_input_tokens: usize,
    total_baseline_output_tokens: usize,
    total_cached_input_tokens: usize,
    total_baseline_tokens: usize,
    total_baseline_cost_microusd: u64,
    exact_cache_tokens_saved: usize,
    exact_cache_cost_saved_microusd: u64,
    nando_cpu_tokens_saved: usize,
    nando_cpu_cost_saved_microusd: u64,
    combined_tokens_saved: usize,
    combined_cost_saved_microusd: u64,
    nando_tokens_saved_milli: usize,
    nando_cost_saved_milli: usize,
    combined_tokens_saved_milli: usize,
    combined_cost_saved_milli: usize,
}

pub(crate) fn run_phase_stream_test_output_parse_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let trace_path = args.next().map(PathBuf::from);
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REPORT));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CELLS);
    let candidate_package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_candidate_package_path(&report_path));
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let (rows, generated_default_trace_used) = match trace_path.as_deref() {
        Some(path) => (read_trace_rows(path)?, false),
        None => (default_trace_rows(), true),
    };
    let mut parsed_events = Vec::new();
    let mut skipped_unclassified_events = 0usize;
    for (index, row) in rows.iter().enumerate() {
        if let Some(event) = parse_trace_row(index, row) {
            parsed_events.push(event);
        } else {
            skipped_unclassified_events += 1;
        }
    }
    if parsed_events.len() < 4 {
        return Err(format!(
            "need at least 4 verifier-classified events, got {}",
            parsed_events.len()
        ));
    }

    let (train_indices, heldout_indices) = stratified_train_heldout_indices(&parsed_events);
    if train_indices.is_empty() || heldout_indices.is_empty() {
        return Err("train/heldout split produced an empty side".to_owned());
    }

    let mut label_to_index = BTreeMap::new();
    for &event_index in &train_indices {
        let event = &parsed_events[event_index];
        let next_index = label_to_index.len();
        label_to_index.entry(event.label).or_insert(next_index);
    }
    let mut compiler = PhaseCenterCompiler::new(cells, label_to_index.len())
        .map_err(|error| format!("phase-center compiler error: {error:?}"))?;
    let mut positive_updates = 0usize;
    let mut negative_updates = 0usize;
    for &event_index in &train_indices {
        let event = &parsed_events[event_index];
        let program_index = label_to_index[&event.label];
        let positive_vec = event_vector(event, event.label, cells);
        compiler
            .add_positive_vector(program_index, &positive_vec)
            .map_err(|error| format!("phase-center positive update error: {error:?}"))?;
        positive_updates += 1;
        for wrong_label in TestOutputLabel::ALL {
            if wrong_label == event.label {
                continue;
            }
            let wrong_vec = event_vector(event, wrong_label, cells);
            compiler
                .add_negative_vector(program_index, &wrong_vec)
                .map_err(|error| format!("phase-center negative update error: {error:?}"))?;
            negative_updates += 1;
        }
    }
    let runtime = compiler
        .compile()
        .map_err(|error| format!("phase-center compile error: {error:?}"))?;
    let runtime_bytes = runtime
        .to_bytes()
        .map_err(|error| format!("phase-center package serialization error: {error:?}"))?;
    write_binary_file(&candidate_package_path, &runtime_bytes)?;
    let read_package = std::fs::read(&candidate_package_path).map_err(|error| {
        format!(
            "failed to read candidate package '{}': {error}",
            candidate_package_path.display()
        )
    })?;
    if read_package != runtime_bytes {
        return Err(format!(
            "candidate package '{}' readback mismatch",
            candidate_package_path.display()
        ));
    }
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&read_package)
        .map_err(|error| format!("phase-center package inspect error: {error:?}"))?;
    let offload_runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &read_package,
        PhaseCenterOffloadPolicy::default_conservative(),
    )
    .map_err(|error| format!("phase-center package load error: {error:?}"))?;

    let mut margins = Vec::new();
    let mut wrong_wins = 0usize;
    let mut comparisons = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut heldout_uncovered_events = 0usize;
    let mut metadata_status_heldout_events = 0usize;
    let mut raw_output_heldout_events = 0usize;
    let mut metadata_status_verified_accepts = 0usize;
    let mut raw_output_verified_accepts = 0usize;
    let mut metadata_status_wrong_wins = 0usize;
    let mut raw_output_wrong_wins = 0usize;
    for &event_index in &heldout_indices {
        let event = &parsed_events[event_index];
        let Some(program_index) = label_to_index.get(&event.label).copied() else {
            heldout_uncovered_events += 1;
            continue;
        };
        let correct_vec = event_vector(event, event.label, cells);
        let mut event_wrong_wins = 0usize;
        for wrong_label in TestOutputLabel::ALL {
            if wrong_label == event.label {
                continue;
            }
            let wrong_vec = event_vector(event, wrong_label, cells);
            let task = PhaseCenterEvalTask {
                center_index: program_index,
                correct_vec: correct_vec.clone().into_boxed_slice(),
                wrong_vec: wrong_vec.into_boxed_slice(),
            };
            let margin = runtime
                .margin(&task)
                .map_err(|error| format!("phase-center shadow margin error: {error:?}"))?;
            let margin_micro = margin_to_micro(margin)?;
            let package_margin = offload_runtime
                .runtime()
                .margin(&task)
                .map_err(|error| format!("phase-center package parity margin error: {error:?}"))?;
            let package_margin_micro = margin_to_micro(package_margin)?;
            if package_margin_micro != margin_micro {
                runtime_margin_parity_mismatches += 1;
            }
            margins.push(margin_micro);
            comparisons += 1;
            if margin_micro <= 0 {
                wrong_wins += 1;
                event_wrong_wins += 1;
            }
        }
        if event.metadata_verifier_used {
            metadata_status_heldout_events += 1;
            metadata_status_wrong_wins += event_wrong_wins;
            if event_wrong_wins == 0 {
                metadata_status_verified_accepts += 1;
            }
        }
        if event.raw_output_available {
            raw_output_heldout_events += 1;
            raw_output_wrong_wins += event_wrong_wins;
            if event_wrong_wins == 0 {
                raw_output_verified_accepts += 1;
            }
        }
    }
    margins.sort_unstable();
    let label_counts = count_labels(&parsed_events);
    let verifier_evidence_counts = count_evidence(&parsed_events);
    let explicit_exact_cache_hits = parsed_events
        .iter()
        .filter(|event| event.explicit_exact_cache_hit == Some(true))
        .count();
    let explicit_exact_cache_field_available = parsed_events
        .iter()
        .any(|event| event.explicit_exact_cache_hit.is_some());
    let fingerprint_exact_cache_hits = fingerprint_exact_cache_hits(&parsed_events);
    let training_window_fingerprint = stable_fingerprint(
        train_indices
            .iter()
            .map(|event_index| parsed_events[*event_index].request_fingerprint.as_str()),
    );
    let raw_output_classified_events = parsed_events
        .iter()
        .filter(|event| event.raw_output_available)
        .count();
    let verifier_metadata_classified_events = parsed_events
        .iter()
        .filter(|event| event.metadata_verifier_used)
        .count();
    let synthetic_events = parsed_events
        .iter()
        .filter(|event| event.synthetic_source)
        .count();

    let mut candidate_labels = label_to_index
        .keys()
        .map(|label| label.as_str().to_owned())
        .collect::<Vec<_>>();
    candidate_labels.sort();
    let mut report = OnlinePhaseCenterShadowReport {
        report_kind: "online_phase_center_test_output_parse_shadow_v1",
        profile: PROFILE,
        verdict: "ONLINE_PHASE_CENTER_TEST_OUTPUT_PARSE_SHADOW_REVIEW",
        mode: "shadow_only",
        cells,
        proof_scope: if raw_output_verified_accepts > 0 && raw_output_wrong_wins == 0 {
            "raw_output_parse"
        } else if metadata_status_verified_accepts > 0 && metadata_status_wrong_wins == 0 {
            "tool_output_state_metadata_parse"
        } else {
            "unproven_shadow_scope"
        }
        .to_owned(),
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        phase_center_runtime_changed: false,
        nwpc_schema_changed: false,
        package_written: true,
        compiler_path: "nando_core::PhaseCenterCompiler",
        runtime_path: "nando_core::PhaseCenterFlatRuntime",
        verifier: VerifierReport {
            verifier_name: VERIFIER_NAME,
            verifier_version: VERIFIER_VERSION,
            verifier_input_kind: VERIFIER_INPUT_KIND,
            verifier_evidence_source: VERIFIER_EVIDENCE_SOURCE,
            accept_rule: ACCEPT_RULE,
            false_accept_threshold: 0,
        },
        trace: TraceReport {
            trace_path: trace_path.as_ref().map(|path| path.display().to_string()),
            generated_default_trace_used,
            total_events: rows.len(),
            parsed_events: parsed_events.len(),
            skipped_unclassified_events,
            raw_output_classified_events,
            verifier_metadata_classified_events,
            synthetic_events,
            train_events: train_indices.len(),
            heldout_events: heldout_indices.len(),
            heldout_uncovered_events,
            explicit_exact_cache_field_available,
            explicit_exact_cache_hits,
            fingerprint_exact_cache_comparison_available: true,
            fingerprint_exact_cache_hits,
        },
        candidate: CandidateReport {
            candidate_count: candidate_labels.len(),
            candidate_labels,
            positive_updates,
            negative_updates,
            runtime_record_count: runtime.record_count(),
            runtime_bytes_estimate: runtime.bytes_estimate(),
            runtime_serialized_len: runtime.serialized_len(),
            verifier_bound: true,
            training_window_fingerprint,
            shadow_report_fingerprint: 0,
        },
        candidate_package: CandidatePackageReport {
            package_kind: "quarantine_candidate_package",
            package_path: candidate_package_path.display().to_string(),
            package_fingerprint64: package_info.fingerprint64,
            package_bytes: read_package.len(),
            inspected_cells: package_info.cells,
            inspected_record_count: package_info.record_count,
            inspected_serialized_len: package_info.serialized_len,
            inspected_payload_bytes: package_info.payload_bytes,
            inspect_matches_runtime: package_info.cells == cells
                && package_info.record_count == runtime.record_count()
                && package_info.serialized_len == runtime_bytes.len(),
            load_roundtrip_matches: offload_runtime.package_info() == package_info,
            runtime_margin_parity_mismatches,
            quarantine_only: true,
            serving_profile_artifact: false,
            promoted: false,
        },
        shadow: ShadowReport {
            shadow_events: heldout_indices.len(),
            metadata_status_shadow_pass: metadata_status_heldout_events > 0
                && metadata_status_verified_accepts == metadata_status_heldout_events
                && metadata_status_wrong_wins == 0,
            raw_output_shadow_pass: raw_output_heldout_events > 0
                && raw_output_verified_accepts == raw_output_heldout_events
                && raw_output_wrong_wins == 0,
            metadata_status_heldout_events,
            raw_output_heldout_events,
            metadata_status_verified_accepts,
            raw_output_verified_accepts,
            metadata_status_wrong_wins,
            raw_output_wrong_wins,
            pairwise_wrong_comparisons: comparisons,
            wrong_wins,
            false_accepts: wrong_wins,
            shadow_local_accepts: 0,
            exact_cache_comparison_reported_separately: true,
            median_margin_micro: percentile_i64(&margins, 50),
            p10_margin_micro: percentile_i64(&margins, 10),
            min_margin_micro: margins.first().copied().unwrap_or(0),
        },
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        label_counts,
        verifier_evidence_counts,
        boundary: "shadow-only online phase-center mining report plus verifier-bound candidate .nwpc package; no promotion, no product local_accept, no serving/runtime/schema change, no legacy backend",
    };
    if report.shadow.false_accepts == 0
        && report.trace.heldout_uncovered_events == 0
        && report.candidate.candidate_count > 0
        && report.candidate_package.inspect_matches_runtime
        && report.candidate_package.load_roundtrip_matches
        && report.candidate_package.runtime_margin_parity_mismatches == 0
        && report.candidate_package.quarantine_only
        && !report.candidate_package.serving_profile_artifact
        && !report.candidate_package.promoted
        && !report.local_accept_enabled
    {
        report.verdict = "ONLINE_PHASE_CENTER_TEST_OUTPUT_PARSE_SHADOW_PASS";
    }
    let report_fingerprint = stable_fingerprint([
        report.report_kind,
        report.profile,
        report.verdict,
        &runtime_bytes.len().to_string(),
        &report.candidate_package.package_fingerprint64.to_string(),
        &report.shadow.false_accepts.to_string(),
        &report.shadow.pairwise_wrong_comparisons.to_string(),
    ]);
    report.candidate.shadow_report_fingerprint = report_fingerprint;

    write_json_file(&report_path, &report)?;
    println!("online_phase_center_test_output_parse_shadow_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  verdict: {}", report.verdict);
    println!("  generated_default_trace_used: {generated_default_trace_used}");
    println!("  parsed_events: {}", report.trace.parsed_events);
    println!("  candidate_count: {}", report.candidate.candidate_count);
    println!(
        "  candidate_package_path: {}",
        report.candidate_package.package_path
    );
    println!(
        "  runtime_margin_parity_mismatches: {}",
        report.candidate_package.runtime_margin_parity_mismatches
    );
    println!("  pairwise_wrong_comparisons: {}", comparisons);
    println!("  false_accepts: {}", report.shadow.false_accepts);
    println!("  proof_scope: {}", report.proof_scope);
    println!(
        "  metadata_status_shadow_pass: {}",
        report.shadow.metadata_status_shadow_pass
    );
    println!(
        "  raw_output_shadow_pass: {}",
        report.shadow.raw_output_shadow_pass
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    Ok(())
}

pub(crate) fn run_phase_stream_test_output_raw_log_trace_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let trace_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(
            "target/nando-wave/real-traffic-shadow/test-output-parse-raw-log-v1.trace.jsonl",
        )
    });
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("target/nando-wave/streaming/test-output-parse-raw-log-v1.trace-report.json")
    });
    let log_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_raw_log_paths()
        } else {
            rest
        }
    };
    if log_paths.is_empty() {
        return Err("no raw log paths provided".to_owned());
    }

    let mut rows = Vec::new();
    let mut source_logs = Vec::new();
    let mut skipped_unclassified_logs = 0usize;
    for path in &log_paths {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => {
                source_logs.push(RawLogSourceReport {
                    path: path.display().to_string(),
                    label: None,
                    bytes: 0,
                    written: false,
                    reason: format!("read_error:{error}"),
                    fingerprint64: 0,
                });
                skipped_unclassified_logs += 1;
                continue;
            }
        };
        let lower = text.to_ascii_lowercase();
        let fingerprint = stable_fingerprint([path.display().to_string().as_str(), &text]);
        let Some((label, _evidence)) = verify_test_output(&lower) else {
            source_logs.push(RawLogSourceReport {
                path: path.display().to_string(),
                label: None,
                bytes: text.len(),
                written: false,
                reason: "raw_output_unclassified".to_owned(),
                fingerprint64: fingerprint,
            });
            skipped_unclassified_logs += 1;
            continue;
        };
        let row_index = rows.len();
        let row = TestOutputTraceRow {
            event_id: Some(format!("raw_log_{row_index:04}_{}", label.as_str())),
            trace_id: Some(format!("raw-log-{fingerprint:016x}")),
            traffic_source: Some("local_raw_log_artifact".to_owned()),
            command: Some(format!("cargo test/check log artifact {}", path.display())),
            stdout: Some(text.clone()),
            stderr: Some(String::new()),
            exit_code: Some(if label == TestOutputLabel::Pass {
                0
            } else {
                101
            }),
            source: Some("existing_raw_log_artifact".to_owned()),
            verification_source: Some("raw stdout/stderr log artifact".to_owned()),
            tool_call_fingerprints: Some(vec![format!("{fingerprint:016x}")]),
            request_fingerprint: Some(format!("{fingerprint:016x}")),
            provider: Some("local_trace".to_owned()),
            model_id: Some("no_provider_raw_log_artifact".to_owned()),
            input_tokens: None,
            output_tokens: None,
            cached_input_tokens: Some(0),
            provider_cost_microusd: None,
            exact_cache_hit: Some(false),
            synthetic_source: Some(false),
            notes: Some(format!(
                "raw_log_artifact=true;raw_log_path={};raw_label={}",
                path.display(),
                label.as_str()
            )),
        };
        rows.push(row);
        source_logs.push(RawLogSourceReport {
            path: path.display().to_string(),
            label: Some(label.as_str().to_owned()),
            bytes: text.len(),
            written: true,
            reason: "raw_output_classified".to_owned(),
            fingerprint64: fingerprint,
        });
    }
    if rows.len() < 4 {
        return Err(format!(
            "need at least 4 raw-output classified log rows, got {}",
            rows.len()
        ));
    }
    let label_kinds = rows
        .iter()
        .filter_map(|row| {
            let stdout = row.stdout.as_deref().unwrap_or_default();
            verify_test_output(&stdout.to_ascii_lowercase()).map(|(label, _)| label)
        })
        .collect::<BTreeSet<_>>();
    if label_kinds.len() < 2 {
        return Err(format!(
            "need at least 2 raw-output labels for heldout pressure, got {}",
            label_kinds.len()
        ));
    }

    write_trace_jsonl(&trace_path, &rows)?;

    let mut label_counts = BTreeMap::new();
    for row in &rows {
        if let Some((label, _)) = verify_test_output(
            &row.stdout
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase(),
        ) {
            *label_counts.entry(label.as_str().to_owned()).or_insert(0) += 1;
        }
    }
    let report = RawLogTraceBuildReport {
        report_kind: "online_phase_center_test_output_parse_raw_log_trace_v1",
        profile: PROFILE,
        mode: "trace_build_only",
        trace_path: trace_path.display().to_string(),
        source: "existing raw stdout/stderr log artifacts",
        logs_read: log_paths.len(),
        rows_written: rows.len(),
        skipped_unclassified_logs,
        raw_output_classified_events: rows.len(),
        verifier_metadata_classified_events: 0,
        synthetic_events: 0,
        label_counts,
        source_logs,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "trace-build only from existing raw stdout/stderr logs; no compiler change, no product local_accept, no serving promotion, no market claim",
    };
    write_json_file(&report_path, &report)?;
    println!("online_phase_center_test_output_parse_raw_log_trace_v1:");
    println!("  trace_path: {}", trace_path.display());
    println!("  report_path: {}", report_path.display());
    println!("  logs_read: {}", report.logs_read);
    println!("  rows_written: {}", report.rows_written);
    println!(
        "  raw_output_classified_events: {}",
        report.raw_output_classified_events
    );
    println!(
        "  verifier_metadata_classified_events: {}",
        report.verifier_metadata_classified_events
    );
    println!("  synthetic_events: {}", report.synthetic_events);
    Ok(())
}

pub(crate) fn run_phase_stream_discovery_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DISCOVERY_REPORT));
    let package_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DISCOVERY_PACKAGE_DIR));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CELLS);
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    let price_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PRICE_CONFIG));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_discovery_trace_paths()
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no trace paths provided".to_owned());
    }
    let price_config = read_json_file::<ModelPriceConfig>(&price_config_path)?;

    let mut total_rows = 0usize;
    let mut parsed_events = Vec::new();
    let mut skipped_unclassified_events = 0usize;
    for trace_path in &trace_paths {
        let rows = read_trace_rows(trace_path)?;
        total_rows += rows.len();
        for row in &rows {
            if let Some(event) = parse_trace_row(parsed_events.len(), row) {
                parsed_events.push(event);
            } else {
                skipped_unclassified_events += 1;
            }
        }
    }
    if parsed_events.is_empty() {
        return Err("no verifier-classified events found in discovery traces".to_owned());
    }

    let mut buckets: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, event) in parsed_events.iter().enumerate() {
        buckets
            .entry(discovery_bucket_key(event))
            .or_default()
            .push(index);
    }

    let mut candidates = Vec::new();
    for (bucket_key, indices) in &buckets {
        let package_path = package_dir.join(format!("{}.nwpc", sanitize_file_stem(bucket_key)));
        let candidate = build_discovery_candidate(
            bucket_key,
            &parsed_events,
            indices,
            cells,
            &price_config,
            &package_path,
        )?;
        candidates.push(candidate);
    }
    let accepted_candidate_count = candidates
        .iter()
        .filter(|candidate| candidate.accepted_for_offline_review)
        .count();
    let total_unique_cpu_accepts_over_exact_cache = candidates
        .iter()
        .filter(|candidate| candidate.accepted_for_offline_review)
        .map(|candidate| candidate.unique_cpu_accepts_over_exact_cache)
        .sum();
    let total_nando_cpu_tokens_saved = candidates
        .iter()
        .filter(|candidate| candidate.accepted_for_offline_review)
        .map(|candidate| candidate.nando_cpu_tokens_saved)
        .sum();
    let total_nando_cpu_cost_saved_microusd = candidates
        .iter()
        .filter(|candidate| candidate.accepted_for_offline_review)
        .map(|candidate| candidate.nando_cpu_cost_saved_microusd)
        .sum();
    let total_combined_cost_saved_microusd = candidates
        .iter()
        .filter(|candidate| candidate.accepted_for_offline_review)
        .map(|candidate| candidate.combined_cost_saved_microusd)
        .sum();
    let report = OnlinePhaseCenterDiscoveryReport {
        report_kind: "online_phase_center_discovery_v1",
        profile: PROFILE,
        mode: "offline_shadow_discovery_only",
        cells,
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        candidate_package_dir: package_dir.display().to_string(),
        total_rows,
        parsed_events: parsed_events.len(),
        skipped_unclassified_events,
        bucket_count: buckets.len(),
        candidate_count: candidates.len(),
        accepted_candidate_count,
        total_unique_cpu_accepts_over_exact_cache,
        total_nando_cpu_tokens_saved,
        total_nando_cpu_cost_saved_microusd,
        total_combined_cost_saved_microusd,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        candidates,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "offline online-discovery registry over verifier-bound phase-center buckets; writes quarantine .nwpc candidates only; no product local_accept, serving promotion, legacy backend, or market money claim",
    };
    write_json_file(&report_path, &report)?;
    println!("online_phase_center_discovery_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  parsed_events: {}", report.parsed_events);
    println!("  bucket_count: {}", report.bucket_count);
    println!("  candidate_count: {}", report.candidate_count);
    println!(
        "  accepted_candidate_count: {}",
        report.accepted_candidate_count
    );
    println!(
        "  total_unique_cpu_accepts_over_exact_cache: {}",
        report.total_unique_cpu_accepts_over_exact_cache
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_online_discovery_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_DISCOVERY_REPORT));
    let package_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_DISCOVERY_PACKAGE_DIR));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CELLS);
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    let min_bucket_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid min bucket events '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    if min_bucket_events < 2 {
        return Err("min bucket events must be >= 2".to_owned());
    }
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin threshold '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
    if margin_threshold_micro <= 0 {
        return Err("margin threshold must be > 0".to_owned());
    }
    let price_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PRICE_CONFIG));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_discovery_trace_paths()
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no trace paths provided".to_owned());
    }
    let price_config = read_json_file::<ModelPriceConfig>(&price_config_path)?;

    let mut total_rows = 0usize;
    let mut parsed_events = Vec::new();
    let mut skipped_unclassified_events = 0usize;
    for trace_path in &trace_paths {
        let rows = read_trace_rows(trace_path)?;
        total_rows += rows.len();
        for row in &rows {
            if let Some(event) = parse_trace_row(parsed_events.len(), row) {
                parsed_events.push(event);
            } else {
                skipped_unclassified_events += 1;
            }
        }
    }
    if parsed_events.is_empty() {
        return Err("no verifier-classified events found in online discovery traces".to_owned());
    }

    let exact_cache_flags = exact_cache_hit_flags(&parsed_events);
    let mut buckets: BTreeMap<String, OnlineDiscoveryBucketState> = BTreeMap::new();
    for (event_index, event) in parsed_events.iter().enumerate() {
        let bucket_key = discovery_bucket_key(event);
        if !buckets.contains_key(&bucket_key) {
            buckets.insert(
                bucket_key.clone(),
                OnlineDiscoveryBucketState {
                    bucket_key: bucket_key.clone(),
                    proof_scope: event_proof_scope(event).to_owned(),
                    event_indices: Vec::new(),
                    raw_output_events: 0,
                    metadata_status_events: 0,
                    compiled: None,
                    shadow_events: 0,
                    shadow_accepts: 0,
                    false_accepts: 0,
                    wrong_wins: 0,
                    shadow_uncovered_events: 0,
                    runtime_margin_parity_mismatches: 0,
                    margins: Vec::new(),
                    exact_cache_hits_in_shadow: 0,
                    unique_cpu_accepts_over_exact_cache: 0,
                    nando_cpu_tokens_saved: 0,
                    nando_cpu_cost_saved_microusd: 0,
                },
            );
        }

        let compile_request = {
            let state = buckets
                .get_mut(&bucket_key)
                .expect("bucket inserted before use");
            if event.raw_output_available && !event.metadata_verifier_used {
                state.raw_output_events += 1;
            }
            if event.metadata_verifier_used {
                state.metadata_status_events += 1;
            }

            if let Some(compiled) = state.compiled.clone() {
                score_online_shadow_event(
                    state,
                    &compiled,
                    event,
                    exact_cache_flags[event_index],
                    &price_config,
                    cells,
                    margin_threshold_micro,
                )?;
                None
            } else {
                state.event_indices.push(event_index);
                if state.event_indices.len() >= min_bucket_events
                    && labels_for_indices(&parsed_events, &state.event_indices).len() >= 2
                {
                    Some((state.bucket_key.clone(), state.event_indices.clone()))
                } else {
                    None
                }
            }
        };

        if let Some((compile_bucket_key, event_indices)) = compile_request {
            let package_path = package_dir.join(format!(
                "{}.stream.nwpc",
                sanitize_file_stem(&compile_bucket_key)
            ));
            let compiled = compile_online_bucket(
                &compile_bucket_key,
                &parsed_events,
                &event_indices,
                cells,
                margin_threshold_micro,
                &package_path,
                event_index,
            )?;
            buckets
                .get_mut(&bucket_key)
                .expect("bucket exists for compiled state")
                .compiled = Some(compiled);
        }
    }

    let mut reports = buckets
        .values()
        .map(online_bucket_report)
        .collect::<Vec<_>>();
    reports.sort_by(|left, right| left.bucket_key.cmp(&right.bucket_key));
    let compiled_bucket_count = reports
        .iter()
        .filter(|bucket| bucket.compiled_after_global_event_index.is_some())
        .count();
    let accepted_bucket_count = reports
        .iter()
        .filter(|bucket| bucket.accepted_for_online_shadow_review)
        .count();
    let stream_shadow_events = reports.iter().map(|bucket| bucket.shadow_events).sum();
    let stream_shadow_accepts = reports.iter().map(|bucket| bucket.shadow_accepts).sum();
    let stream_false_accepts = reports.iter().map(|bucket| bucket.false_accepts).sum();
    let total_unique_cpu_accepts_over_exact_cache = reports
        .iter()
        .filter(|bucket| bucket.accepted_for_online_shadow_review)
        .map(|bucket| bucket.unique_cpu_accepts_over_exact_cache)
        .sum();
    let total_nando_cpu_tokens_saved = reports
        .iter()
        .filter(|bucket| bucket.accepted_for_online_shadow_review)
        .map(|bucket| bucket.nando_cpu_tokens_saved)
        .sum();
    let total_nando_cpu_cost_saved_microusd = reports
        .iter()
        .filter(|bucket| bucket.accepted_for_online_shadow_review)
        .map(|bucket| bucket.nando_cpu_cost_saved_microusd)
        .sum();

    let report = OnlinePhaseCenterStreamingDiscoveryReport {
        report_kind: "online_phase_center_streaming_discovery_v1",
        profile: PROFILE,
        mode: "online_shadow_discovery_only",
        cells,
        min_bucket_events,
        margin_threshold_micro,
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        candidate_package_dir: package_dir.display().to_string(),
        total_rows,
        parsed_events: parsed_events.len(),
        skipped_unclassified_events,
        bucket_count: reports.len(),
        compiled_bucket_count,
        accepted_bucket_count,
        stream_shadow_events,
        stream_shadow_accepts,
        stream_false_accepts,
        total_unique_cpu_accepts_over_exact_cache,
        total_nando_cpu_tokens_saved,
        total_nando_cpu_cost_saved_microusd,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        buckets: reports,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "online-order shadow discovery only; compiles .nwpc after verifier-bound bucket threshold and scores only future events; no product local_accept, serving promotion, legacy backend, or market money claim",
    };
    write_json_file(&report_path, &report)?;
    println!("online_phase_center_streaming_discovery_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  parsed_events: {}", report.parsed_events);
    println!("  bucket_count: {}", report.bucket_count);
    println!("  compiled_bucket_count: {}", report.compiled_bucket_count);
    println!("  accepted_bucket_count: {}", report.accepted_bucket_count);
    println!("  stream_shadow_events: {}", report.stream_shadow_events);
    println!("  stream_shadow_accepts: {}", report.stream_shadow_accepts);
    println!(
        "  total_unique_cpu_accepts_over_exact_cache: {}",
        report.total_unique_cpu_accepts_over_exact_cache
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_real_traffic_online_discovery_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    run_phase_stream_real_traffic_online_discovery_impl(args, GenericBucketMode::Route)
}

pub(crate) fn run_phase_stream_real_traffic_refined_online_discovery_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    run_phase_stream_real_traffic_online_discovery_impl(args, GenericBucketMode::RequestShape)
}

pub(crate) fn run_phase_stream_real_traffic_action_family_online_discovery_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    run_phase_stream_real_traffic_online_discovery_impl(args, GenericBucketMode::ActionFamily)
}

pub(crate) fn run_phase_stream_real_traffic_state_action_online_discovery_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    run_phase_stream_real_traffic_online_discovery_impl(args, GenericBucketMode::StateAction)
}

pub(crate) fn run_phase_stream_real_traffic_frontier_union_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_FRONTIER_UNION_REPORT));
    let input_report_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_frontier_union_report_paths()
        } else {
            rest
        }
    };
    if input_report_paths.is_empty() {
        return Err("no phase-center discovery reports provided".to_owned());
    }

    let mut input_reports = Vec::new();
    let mut unique_accepts = BTreeMap::<String, GenericAcceptedEventReport>::new();
    let mut duplicate_request_fingerprint_count = 0usize;
    let mut duplicate_token_cost_mismatch_count = 0usize;

    for input_path in &input_report_paths {
        let report = read_json_file::<serde_json::Value>(input_path)?;
        let report_kind = report
            .get("report_kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let mut bucket_mode = report
            .get("bucket_mode")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let mut cells = json_usize(report.get("cells")).unwrap_or(0);
        let margin_threshold_micro = report
            .get("margin_threshold_micro")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        let mut stream_false_accepts = json_usize(report.get("stream_false_accepts")).unwrap_or(0);
        let mut accepted_bucket_count =
            json_usize(report.get("accepted_bucket_count")).unwrap_or(0);
        let local_accept_enabled = report
            .get("local_accept_enabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let product_runtime_changed = report
            .get("product_runtime_changed")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let serving_runtime_changed = report
            .get("serving_runtime_changed")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let market_money_claim_allowed = report
            .get("market_money_claim_allowed")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let forbidden_flags_ok = report
            .get("forbidden_flags")
            .is_some_and(forbidden_flags_value_all_false);
        let supported_phase_atom_promotion_audit = report_kind
            == "phase_atom_run_check_time_split_promotion_audit_v1"
            || report_kind == "phase_atom_action_family_time_split_promotion_audit_v1";
        if supported_phase_atom_promotion_audit {
            bucket_mode = json_string(&report, &["action_family"])
                .or_else(|| json_string(&report, &["discovery_gate", "action_family"]))
                .unwrap_or_else(|| "phase_atom_time_split".to_owned());
            cells = json_usize(json_at(&report, &["package", "inspected_cells"])).unwrap_or(0);
            stream_false_accepts =
                json_usize(json_at(&report, &["discovery_gate", "false_accepts"]))
                    .unwrap_or(usize::MAX);
            accepted_bucket_count =
                usize::from(json_bool(&report, &["promotion_candidate_allowed"]).unwrap_or(false));
        }

        let mut report_accept_map = BTreeMap::<String, GenericAcceptedEventReport>::new();
        if let Some(buckets) = report.get("buckets").and_then(serde_json::Value::as_array) {
            for bucket in buckets {
                if bucket
                    .get("accepted_for_online_shadow_review")
                    .and_then(serde_json::Value::as_bool)
                    != Some(true)
                {
                    continue;
                }
                if json_usize(bucket.get("false_accepts")).unwrap_or(usize::MAX) != 0 {
                    continue;
                }
                let Some(accepted_events) = bucket
                    .get("unique_accepts")
                    .and_then(serde_json::Value::as_array)
                else {
                    continue;
                };
                for accepted in accepted_events {
                    let Some(request_fingerprint) = accepted
                        .get("request_fingerprint")
                        .and_then(serde_json::Value::as_str)
                    else {
                        continue;
                    };
                    let event = GenericAcceptedEventReport {
                        request_fingerprint: request_fingerprint.to_owned(),
                        total_tokens: json_usize(accepted.get("total_tokens")).unwrap_or(0),
                        total_cost_microusd: accepted
                            .get("total_cost_microusd")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0),
                        token_evidence_missing: accepted
                            .get("token_evidence_missing")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(true),
                        cost_evidence_missing: accepted
                            .get("cost_evidence_missing")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(true),
                    };
                    report_accept_map
                        .entry(event.request_fingerprint.clone())
                        .or_insert(event);
                }
            }
        }
        if supported_phase_atom_promotion_audit
            && let Some(accepted_events) = report
                .get("unique_accepts")
                .and_then(serde_json::Value::as_array)
        {
            for accepted in accepted_events {
                let Some(request_fingerprint) = accepted
                    .get("request_fingerprint")
                    .and_then(serde_json::Value::as_str)
                else {
                    continue;
                };
                let event = GenericAcceptedEventReport {
                    request_fingerprint: request_fingerprint.to_owned(),
                    total_tokens: json_usize(accepted.get("total_tokens")).unwrap_or(0),
                    total_cost_microusd: accepted
                        .get("total_cost_microusd")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0),
                    token_evidence_missing: accepted
                        .get("token_evidence_missing")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(true),
                    cost_evidence_missing: accepted
                        .get("cost_evidence_missing")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(true),
                };
                report_accept_map
                    .entry(event.request_fingerprint.clone())
                    .or_insert(event);
            }
        }
        let unique_accepts_in_report = report_accept_map.len();
        let tokens_saved_in_report = report_accept_map
            .values()
            .map(|event| event.total_tokens)
            .sum::<usize>();
        let cost_saved_microusd_in_report = report_accept_map
            .values()
            .map(|event| event.total_cost_microusd)
            .sum::<u64>();
        let report_accepts = report_accept_map.into_values().collect::<Vec<_>>();

        let mut exclusion_reasons = Vec::new();
        let supported_online_report_kind =
            report_kind == "generic_real_traffic_phase_center_online_discovery_v1";
        let supported_split_report_kind =
            report_kind == "generic_real_traffic_phase_center_guarded_separator_split_shadow_v1";
        let supported_calibrated_split_report_kind = report_kind
            == "generic_real_traffic_phase_center_guarded_separator_calibrated_split_shadow_v1";
        let split_shadow_independent = report
            .get("shadow_window_independent")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let selector_train_shadow_disjoint = report
            .get("selector_train_shadow_disjoint")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let selector_compile_calibration_shadow_disjoint = report
            .get("selector_compile_calibration_shadow_disjoint")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if !supported_online_report_kind
            && !supported_split_report_kind
            && !supported_calibrated_split_report_kind
            && !supported_phase_atom_promotion_audit
        {
            exclusion_reasons.push("unsupported_report_kind");
        }
        if supported_phase_atom_promotion_audit {
            if !json_bool(&report, &["promotion_candidate_allowed"]).unwrap_or(false) {
                exclusion_reasons.push("phase_atom_promotion_candidate_not_allowed");
            }
            if json_bool(&report, &["product_promotion_allowed"]).unwrap_or(true) {
                exclusion_reasons.push("phase_atom_product_promotion_allowed");
            }
            if !json_bool(&report, &["package", "inspect_matches_discovery_report"])
                .unwrap_or(false)
            {
                exclusion_reasons.push("phase_atom_package_inspect_mismatch");
            }
            if json_usize(json_at(&report, &["discovery_gate", "wrong_wins"])).unwrap_or(usize::MAX)
                != 0
            {
                exclusion_reasons.push("phase_atom_wrong_wins_nonzero");
            }
            if json_usize(json_at(
                &report,
                &["discovery_gate", "runtime_margin_parity_mismatches"],
            ))
            .unwrap_or(usize::MAX)
                != 0
            {
                exclusion_reasons.push("phase_atom_runtime_parity_mismatch");
            }
        }
        if supported_split_report_kind && !split_shadow_independent {
            exclusion_reasons.push("split_shadow_not_independent");
        }
        if supported_split_report_kind && !selector_train_shadow_disjoint {
            exclusion_reasons.push("selector_train_shadow_not_disjoint");
        }
        if supported_calibrated_split_report_kind && !split_shadow_independent {
            exclusion_reasons.push("calibrated_split_shadow_not_independent");
        }
        if supported_calibrated_split_report_kind && !selector_compile_calibration_shadow_disjoint {
            exclusion_reasons.push("calibrated_split_windows_not_disjoint");
        }
        if stream_false_accepts > 0 {
            exclusion_reasons.push("report_stream_false_accepts_nonzero");
        }
        if local_accept_enabled {
            exclusion_reasons.push("local_accept_enabled");
        }
        if product_runtime_changed {
            exclusion_reasons.push("product_runtime_changed");
        }
        if serving_runtime_changed {
            exclusion_reasons.push("serving_runtime_changed");
        }
        if market_money_claim_allowed {
            exclusion_reasons.push("market_money_claim_allowed");
        }
        if !forbidden_flags_ok {
            exclusion_reasons.push("forbidden_flags_not_all_false");
        }
        if unique_accepts_in_report == 0 {
            exclusion_reasons.push("no_unique_accepts");
        }
        let included_in_union = exclusion_reasons.is_empty();

        if included_in_union {
            for event in report_accepts {
                if let Some(existing) = unique_accepts.get(&event.request_fingerprint) {
                    duplicate_request_fingerprint_count += 1;
                    if existing.total_tokens != event.total_tokens
                        || existing.total_cost_microusd != event.total_cost_microusd
                        || existing.token_evidence_missing != event.token_evidence_missing
                        || existing.cost_evidence_missing != event.cost_evidence_missing
                    {
                        duplicate_token_cost_mismatch_count += 1;
                    }
                } else {
                    unique_accepts.insert(event.request_fingerprint.clone(), event);
                }
            }
        }

        input_reports.push(GenericFrontierInputReport {
            path: input_path.display().to_string(),
            report_kind,
            bucket_mode,
            cells,
            margin_threshold_micro,
            stream_false_accepts,
            accepted_bucket_count,
            unique_accepts_in_report,
            tokens_saved_in_report,
            cost_saved_microusd_in_report,
            included_in_union,
            exclusion_reason: if exclusion_reasons.is_empty() {
                "included".to_owned()
            } else {
                exclusion_reasons.join(",")
            },
        });
    }

    let combined_unique_cpu_accepts_over_exact_cache = unique_accepts.len();
    let combined_nando_cpu_tokens_saved = unique_accepts
        .values()
        .map(|event| event.total_tokens)
        .sum::<usize>();
    let combined_nando_cpu_cost_saved_microusd = unique_accepts
        .values()
        .map(|event| event.total_cost_microusd)
        .sum::<u64>();
    let token_evidence_missing_events = unique_accepts
        .values()
        .filter(|event| event.token_evidence_missing)
        .count();
    let cost_evidence_missing_events = unique_accepts
        .values()
        .filter(|event| event.cost_evidence_missing)
        .count();
    let safe_input_report_count = input_reports
        .iter()
        .filter(|report| report.included_in_union)
        .count();
    let report = GenericFrontierUnionReport {
        report_kind: "generic_real_traffic_phase_center_frontier_union_v1",
        mode: "shadow_report_union_audit_only",
        input_report_paths: input_report_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        input_report_count: input_reports.len(),
        safe_input_report_count,
        excluded_input_report_count: input_reports.len().saturating_sub(safe_input_report_count),
        input_reports,
        combined_unique_cpu_accepts_over_exact_cache,
        combined_nando_cpu_tokens_saved,
        combined_nando_cpu_cost_saved_microusd,
        duplicate_request_fingerprint_count,
        duplicate_token_cost_mismatch_count,
        token_evidence_missing_events,
        cost_evidence_missing_events,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "frontier union audit only: dedupes accepted verifier-bound shadow reports by request_fingerprint; does not compile, promote, serve, local-accept, or claim market money",
    };
    write_json_file(&report_path, &report)?;
    println!("generic_real_traffic_phase_center_frontier_union_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  input_report_count: {}", report.input_report_count);
    println!(
        "  safe_input_report_count: {}",
        report.safe_input_report_count
    );
    println!(
        "  combined_unique_cpu_accepts_over_exact_cache: {}",
        report.combined_unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  combined_nando_cpu_tokens_saved: {}",
        report.combined_nando_cpu_tokens_saved
    );
    println!(
        "  combined_nando_cpu_cost_saved_microusd: {}",
        report.combined_nando_cpu_cost_saved_microusd
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_real_traffic_cpu10_gap_audit_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_CPU10_GAP_AUDIT_REPORT));
    let frontier_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_FRONTIER_UNION_REPORT));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_enriched_trace_paths()
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no real-traffic trace paths provided".to_owned());
    }

    let frontier = read_json_file::<serde_json::Value>(&frontier_report_path)?;
    let current_safe_accepts_over_exact_cache =
        json_usize(frontier.get("combined_unique_cpu_accepts_over_exact_cache")).unwrap_or(0);
    let current_safe_tokens_saved =
        json_usize(frontier.get("combined_nando_cpu_tokens_saved")).unwrap_or(0);
    let current_safe_cost_saved_microusd = frontier
        .get("combined_nando_cpu_cost_saved_microusd")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    let mut total_rows = 0usize;
    let mut rows_without_shadow_request = 0usize;
    let mut shadow_request_rows = 0usize;
    let mut legacy_shadow_request_rows = 0usize;
    let mut events = Vec::new();
    for trace_path in &trace_paths {
        let (rows, missing_shadow, parsed_events) = read_cpu10_trace_events(trace_path)?;
        total_rows += rows;
        rows_without_shadow_request += missing_shadow;
        shadow_request_rows += parsed_events.len();
        legacy_shadow_request_rows += parsed_events.iter().filter(|event| event.legacy).count();
        events.extend(parsed_events);
    }

    let exact_cache_hits = exact_cache_hit_flags_cpu10(&events);
    let mut route_states = BTreeMap::<String, GenericCpu10RouteGapState>::new();
    let mut verifier_true_rows = 0usize;
    let mut verifier_false_rows = 0usize;
    let mut verifier_missing_rows = 0usize;
    let mut exact_cache_hits_in_nonlegacy_shadow = 0usize;
    let mut verifier_true_over_exact_cache_ceiling = 0usize;
    let mut verifier_true_token_ceiling_over_exact_cache = 0usize;
    let mut verifier_true_cost_ceiling_microusd_over_exact_cache = 0u64;

    for (event, exact_hit) in events.iter().zip(exact_cache_hits.iter().copied()) {
        if event.legacy {
            continue;
        }
        let state = route_states
            .entry(event.bucket_key.clone())
            .or_insert_with(|| GenericCpu10RouteGapState {
                bucket_key: event.bucket_key.clone(),
                route_key: event.route_key.clone(),
                profile_id: event.profile_id.clone(),
                ..Default::default()
            });
        state.nonlegacy_shadow_request_rows += 1;
        if exact_hit {
            exact_cache_hits_in_nonlegacy_shadow += 1;
            state.exact_cache_hits += 1;
        }
        match event.verified_safe_accept {
            Some(true) => {
                verifier_true_rows += 1;
                state.verifier_true_rows += 1;
                if !exact_hit {
                    verifier_true_over_exact_cache_ceiling += 1;
                    state.verifier_true_over_exact_cache_ceiling += 1;
                    verifier_true_token_ceiling_over_exact_cache += event.token_cost.total_tokens;
                    state.verifier_true_token_ceiling_over_exact_cache +=
                        event.token_cost.total_tokens;
                    verifier_true_cost_ceiling_microusd_over_exact_cache =
                        verifier_true_cost_ceiling_microusd_over_exact_cache
                            .saturating_add(event.token_cost.total_cost_microusd);
                    state.verifier_true_cost_ceiling_microusd_over_exact_cache = state
                        .verifier_true_cost_ceiling_microusd_over_exact_cache
                        .saturating_add(event.token_cost.total_cost_microusd);
                }
            }
            Some(false) => {
                verifier_false_rows += 1;
                state.verifier_false_rows += 1;
            }
            None => {
                verifier_missing_rows += 1;
                state.verifier_missing_rows += 1;
            }
        }
    }

    let nonlegacy_shadow_request_rows =
        shadow_request_rows.saturating_sub(legacy_shadow_request_rows);
    let mut routes = route_states
        .into_values()
        .map(|state| {
            let recommended_next_action =
                cpu10_route_recommended_next_action(&state, verifier_true_over_exact_cache_ceiling);
            GenericCpu10RouteGapReport {
                bucket_key: state.bucket_key,
                route_key: state.route_key,
                profile_id: state.profile_id,
                nonlegacy_shadow_request_rows: state.nonlegacy_shadow_request_rows,
                verifier_true_rows: state.verifier_true_rows,
                verifier_true_over_exact_cache_ceiling: state
                    .verifier_true_over_exact_cache_ceiling,
                verifier_false_rows: state.verifier_false_rows,
                verifier_missing_rows: state.verifier_missing_rows,
                exact_cache_hits: state.exact_cache_hits,
                verifier_true_token_ceiling_over_exact_cache: state
                    .verifier_true_token_ceiling_over_exact_cache,
                verifier_true_cost_ceiling_microusd_over_exact_cache: state
                    .verifier_true_cost_ceiling_microusd_over_exact_cache,
                traffic_share_milli_of_nonlegacy_shadow: per_thousand(
                    state.nonlegacy_shadow_request_rows,
                    nonlegacy_shadow_request_rows,
                ),
                true_ceiling_share_milli_of_total_true_ceiling: per_thousand(
                    state.verifier_true_over_exact_cache_ceiling,
                    verifier_true_over_exact_cache_ceiling,
                ),
                recommended_next_action,
            }
        })
        .collect::<Vec<_>>();
    routes.sort_by(|left, right| {
        right
            .verifier_true_over_exact_cache_ceiling
            .cmp(&left.verifier_true_over_exact_cache_ceiling)
            .then_with(|| {
                right
                    .verifier_true_cost_ceiling_microusd_over_exact_cache
                    .cmp(&left.verifier_true_cost_ceiling_microusd_over_exact_cache)
            })
            .then_with(|| left.bucket_key.cmp(&right.bucket_key))
    });

    let target_cpu_accepts_over_exact_cache = 500usize;
    let remaining_accept_gap_to_cpu10 =
        target_cpu_accepts_over_exact_cache.saturating_sub(current_safe_accepts_over_exact_cache);
    let trace_pool_ceiling_shortfall_to_cpu10 =
        target_cpu_accepts_over_exact_cache.saturating_sub(verifier_true_over_exact_cache_ceiling);
    let frontier_reaches_cpu10_accept_target =
        current_safe_accepts_over_exact_cache >= target_cpu_accepts_over_exact_cache;
    let frontier_accepts_exceed_trace_pool_ceiling =
        current_safe_accepts_over_exact_cache > verifier_true_over_exact_cache_ceiling;
    let report = GenericCpu10GapAuditReport {
        report_kind: "generic_real_traffic_phase_center_cpu10_gap_audit_v1",
        mode: "audit_only_trace_pool_ceiling",
        target_cpu_accepts_over_exact_cache,
        current_frontier_report_path: frontier_report_path.display().to_string(),
        current_safe_accepts_over_exact_cache,
        current_safe_tokens_saved,
        current_safe_cost_saved_microusd,
        remaining_accept_gap_to_cpu10,
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        total_rows,
        rows_without_shadow_request,
        shadow_request_rows,
        legacy_shadow_request_rows,
        nonlegacy_shadow_request_rows,
        verifier_true_rows,
        verifier_false_rows,
        verifier_missing_rows,
        exact_cache_hits_in_nonlegacy_shadow,
        verifier_true_over_exact_cache_ceiling,
        verifier_true_token_ceiling_over_exact_cache,
        verifier_true_cost_ceiling_microusd_over_exact_cache,
        trace_pool_ceiling_shortfall_to_cpu10,
        additional_verifier_true_over_exact_cache_needed_for_cpu10: remaining_accept_gap_to_cpu10,
        current_frontier_capture_rate_milli_of_true_ceiling: per_thousand(
            current_safe_accepts_over_exact_cache,
            verifier_true_over_exact_cache_ceiling,
        ),
        current_trace_pool_can_reach_cpu10_by_scoring_only: verifier_true_over_exact_cache_ceiling
            >= target_cpu_accepts_over_exact_cache,
        frontier_reaches_cpu10_accept_target,
        frontier_accepts_exceed_trace_pool_ceiling,
        routes,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "audit only: measures current verifier-ready default trace ceiling and combined frontier CPU10 gap; combined frontier may include external verifier-bound phase-atom audit streams, but this does not compile, promote, serve, local-accept, or claim market money",
    };

    write_json_file(&report_path, &report)?;
    println!("generic_real_traffic_phase_center_cpu10_gap_audit_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  current_safe_accepts_over_exact_cache: {}",
        report.current_safe_accepts_over_exact_cache
    );
    println!(
        "  verifier_true_over_exact_cache_ceiling: {}",
        report.verifier_true_over_exact_cache_ceiling
    );
    println!(
        "  additional_verifier_true_over_exact_cache_needed_for_cpu10: {}",
        report.additional_verifier_true_over_exact_cache_needed_for_cpu10
    );
    println!(
        "  trace_pool_ceiling_shortfall_to_cpu10: {}",
        report.trace_pool_ceiling_shortfall_to_cpu10
    );
    println!(
        "  frontier_reaches_cpu10_accept_target: {}",
        report.frontier_reaches_cpu10_accept_target
    );
    println!(
        "  frontier_accepts_exceed_trace_pool_ceiling: {}",
        report.frontier_accepts_exceed_trace_pool_ceiling
    );
    println!(
        "  current_trace_pool_can_reach_cpu10_by_scoring_only: {}",
        report.current_trace_pool_can_reach_cpu10_by_scoring_only
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_real_traffic_shadow_request_gap_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_SHADOW_REQUEST_GAP_AUDIT_REPORT));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_enriched_trace_paths()
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no real-traffic trace paths provided".to_owned());
    }

    let mut total_rows = 0usize;
    let mut distinct_request_fingerprints = BTreeSet::new();
    let mut shadow_request_rows = 0usize;
    let mut missing_shadow_request_rows = 0usize;
    let mut missing_shadow_with_token_or_cost_rows = 0usize;
    let mut missing_shadow_token_ceiling = 0usize;
    let mut missing_shadow_cost_ceiling_microusd = 0u64;
    let mut missing_shadow_not_route_candidate_rows = 0usize;
    let mut missing_shadow_rejected_candidate_rows = 0usize;
    let mut missing_shadow_builder_rejected_request_side_features_rows = 0usize;
    let mut missing_shadow_missing_request_signal_rows = 0usize;
    let mut missing_shadow_missing_context_signal_rows = 0usize;
    let mut missing_shadow_missing_evidence_signal_rows = 0usize;
    let mut missing_shadow_missing_verifier_signal_rows = 0usize;
    let mut scoreable_shadow_rows = 0usize;
    let mut scoreable_verifier_true_rows = 0usize;
    let mut scoreable_verifier_false_rows = 0usize;
    let mut scoreable_verifier_missing_rows = 0usize;
    let mut scoreable_verifier_true_token_ceiling = 0usize;
    let mut scoreable_verifier_true_cost_ceiling_microusd = 0u64;
    let mut route_states = BTreeMap::<String, GenericShadowRequestGapState>::new();
    let mut file_reports = Vec::new();

    for trace_path in &trace_paths {
        let (inferred_route_key, inferred_profile_id) =
            infer_shadow_gap_route_from_path(trace_path);
        let mut file_state = GenericShadowRequestGapState {
            bucket_key: format!("{inferred_profile_id}::{inferred_route_key}"),
            route_key: inferred_route_key.clone(),
            profile_id: inferred_profile_id.clone(),
            ..Default::default()
        };
        let text = std::fs::read_to_string(trace_path)
            .map_err(|error| format!("failed to read trace '{}': {error}", trace_path.display()))?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            if !row.is_object() {
                continue;
            }
            total_rows += 1;
            file_state.trace_rows += 1;
            if let Some(fingerprint) = row
                .get("request_fingerprint")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty())
            {
                distinct_request_fingerprints.insert(fingerprint.to_owned());
            }

            let token_cost = generic_token_cost_from_row(&row);
            if let Some(request) = row
                .get("nando_shadow_request")
                .and_then(serde_json::Value::as_object)
            {
                shadow_request_rows += 1;
                file_state.shadow_request_rows += 1;
                let route_key = json_field_string(request.get("route_key"))
                    .unwrap_or(inferred_route_key.clone());
                let profile_id = json_field_string(request.get("profile_id"))
                    .unwrap_or(inferred_profile_id.clone());
                let state = shadow_gap_route_state(&mut route_states, &route_key, &profile_id);
                state.trace_rows += 1;
                state.shadow_request_rows += 1;
                scoreable_shadow_rows += 1;
                match row
                    .get("verified_safe_accept")
                    .and_then(serde_json::Value::as_bool)
                {
                    Some(true) => {
                        scoreable_verifier_true_rows += 1;
                        file_state.scoreable_verifier_true_rows += 1;
                        state.scoreable_verifier_true_rows += 1;
                        scoreable_verifier_true_token_ceiling += token_cost.total_tokens;
                        state.scoreable_verifier_true_token_ceiling += token_cost.total_tokens;
                        scoreable_verifier_true_cost_ceiling_microusd =
                            scoreable_verifier_true_cost_ceiling_microusd
                                .saturating_add(token_cost.total_cost_microusd);
                        state.scoreable_verifier_true_cost_ceiling_microusd = state
                            .scoreable_verifier_true_cost_ceiling_microusd
                            .saturating_add(token_cost.total_cost_microusd);
                    }
                    Some(false) => {
                        scoreable_verifier_false_rows += 1;
                        file_state.scoreable_verifier_false_rows += 1;
                        state.scoreable_verifier_false_rows += 1;
                    }
                    None => {
                        scoreable_verifier_missing_rows += 1;
                        file_state.scoreable_verifier_missing_rows += 1;
                        state.scoreable_verifier_missing_rows += 1;
                    }
                }
                continue;
            }

            missing_shadow_request_rows += 1;
            file_state.missing_shadow_request_rows += 1;
            let state = shadow_gap_route_state(
                &mut route_states,
                &inferred_route_key,
                &inferred_profile_id,
            );
            state.trace_rows += 1;
            state.missing_shadow_request_rows += 1;
            let has_token_or_cost =
                token_cost.total_tokens > 0 || token_cost.total_cost_microusd > 0;
            if has_token_or_cost {
                missing_shadow_with_token_or_cost_rows += 1;
                file_state.missing_shadow_with_token_or_cost_rows += 1;
                state.missing_shadow_with_token_or_cost_rows += 1;
            }
            missing_shadow_token_ceiling += token_cost.total_tokens;
            file_state.missing_shadow_token_ceiling += token_cost.total_tokens;
            state.missing_shadow_token_ceiling += token_cost.total_tokens;
            missing_shadow_cost_ceiling_microusd =
                missing_shadow_cost_ceiling_microusd.saturating_add(token_cost.total_cost_microusd);
            file_state.missing_shadow_cost_ceiling_microusd = file_state
                .missing_shadow_cost_ceiling_microusd
                .saturating_add(token_cost.total_cost_microusd);
            state.missing_shadow_cost_ceiling_microusd = state
                .missing_shadow_cost_ceiling_microusd
                .saturating_add(token_cost.total_cost_microusd);

            let notes = row
                .get("notes")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if notes.contains("not ") && notes.contains("route-gap candidate") {
                missing_shadow_not_route_candidate_rows += 1;
                file_state.missing_shadow_not_route_candidate_rows += 1;
                state.missing_shadow_not_route_candidate_rows += 1;
            }
            if notes.contains("candidate rejected by") {
                missing_shadow_rejected_candidate_rows += 1;
                file_state.missing_shadow_rejected_candidate_rows += 1;
                state.missing_shadow_rejected_candidate_rows += 1;
            }
            if notes.contains("builder_rejected_request_side_features") {
                missing_shadow_builder_rejected_request_side_features_rows += 1;
                file_state.missing_shadow_builder_rejected_request_side_features_rows += 1;
                state.missing_shadow_builder_rejected_request_side_features_rows += 1;
            }
            if notes.contains("missing_request_signal") {
                missing_shadow_missing_request_signal_rows += 1;
                file_state.missing_shadow_missing_request_signal_rows += 1;
                state.missing_shadow_missing_request_signal_rows += 1;
            }
            if notes.contains("missing_context_signal") {
                missing_shadow_missing_context_signal_rows += 1;
                file_state.missing_shadow_missing_context_signal_rows += 1;
                state.missing_shadow_missing_context_signal_rows += 1;
            }
            if notes.contains("missing_evidence_signal") {
                missing_shadow_missing_evidence_signal_rows += 1;
                file_state.missing_shadow_missing_evidence_signal_rows += 1;
                state.missing_shadow_missing_evidence_signal_rows += 1;
            }
            if notes.contains("missing_verifier_signal") {
                missing_shadow_missing_verifier_signal_rows += 1;
                file_state.missing_shadow_missing_verifier_signal_rows += 1;
                state.missing_shadow_missing_verifier_signal_rows += 1;
            }
        }

        file_reports.push(GenericShadowRequestGapFileReport {
            path: trace_path.display().to_string(),
            inferred_route_key,
            inferred_profile_id,
            total_rows: file_state.trace_rows,
            shadow_request_rows: file_state.shadow_request_rows,
            missing_shadow_request_rows: file_state.missing_shadow_request_rows,
            missing_shadow_with_token_or_cost_rows: file_state
                .missing_shadow_with_token_or_cost_rows,
            missing_shadow_token_ceiling: file_state.missing_shadow_token_ceiling,
            missing_shadow_cost_ceiling_microusd: file_state.missing_shadow_cost_ceiling_microusd,
            missing_shadow_not_route_candidate_rows: file_state
                .missing_shadow_not_route_candidate_rows,
            missing_shadow_rejected_candidate_rows: file_state
                .missing_shadow_rejected_candidate_rows,
            missing_shadow_builder_rejected_request_side_features_rows: file_state
                .missing_shadow_builder_rejected_request_side_features_rows,
            missing_shadow_missing_request_signal_rows: file_state
                .missing_shadow_missing_request_signal_rows,
            missing_shadow_missing_context_signal_rows: file_state
                .missing_shadow_missing_context_signal_rows,
            missing_shadow_missing_evidence_signal_rows: file_state
                .missing_shadow_missing_evidence_signal_rows,
            missing_shadow_missing_verifier_signal_rows: file_state
                .missing_shadow_missing_verifier_signal_rows,
            scoreable_verifier_true_rows: file_state.scoreable_verifier_true_rows,
            scoreable_verifier_false_rows: file_state.scoreable_verifier_false_rows,
            scoreable_verifier_missing_rows: file_state.scoreable_verifier_missing_rows,
        });
    }

    let mut route_reports = route_states
        .into_values()
        .map(|state| {
            let recommended_next_action = shadow_gap_recommended_next_action(&state);
            GenericShadowRequestGapRouteReport {
                bucket_key: state.bucket_key,
                route_key: state.route_key,
                profile_id: state.profile_id,
                trace_rows: state.trace_rows,
                shadow_request_rows: state.shadow_request_rows,
                missing_shadow_request_rows: state.missing_shadow_request_rows,
                missing_shadow_with_token_or_cost_rows: state
                    .missing_shadow_with_token_or_cost_rows,
                missing_shadow_token_ceiling: state.missing_shadow_token_ceiling,
                missing_shadow_cost_ceiling_microusd: state.missing_shadow_cost_ceiling_microusd,
                missing_shadow_not_route_candidate_rows: state
                    .missing_shadow_not_route_candidate_rows,
                missing_shadow_rejected_candidate_rows: state
                    .missing_shadow_rejected_candidate_rows,
                missing_shadow_builder_rejected_request_side_features_rows: state
                    .missing_shadow_builder_rejected_request_side_features_rows,
                missing_shadow_missing_request_signal_rows: state
                    .missing_shadow_missing_request_signal_rows,
                missing_shadow_missing_context_signal_rows: state
                    .missing_shadow_missing_context_signal_rows,
                missing_shadow_missing_evidence_signal_rows: state
                    .missing_shadow_missing_evidence_signal_rows,
                missing_shadow_missing_verifier_signal_rows: state
                    .missing_shadow_missing_verifier_signal_rows,
                scoreable_verifier_true_rows: state.scoreable_verifier_true_rows,
                scoreable_verifier_false_rows: state.scoreable_verifier_false_rows,
                scoreable_verifier_missing_rows: state.scoreable_verifier_missing_rows,
                scoreable_verifier_true_token_ceiling: state.scoreable_verifier_true_token_ceiling,
                scoreable_verifier_true_cost_ceiling_microusd: state
                    .scoreable_verifier_true_cost_ceiling_microusd,
                missing_cost_share_milli_of_total_missing_cost: per_thousand_u64(
                    state.missing_shadow_cost_ceiling_microusd,
                    missing_shadow_cost_ceiling_microusd,
                ),
                scoreable_true_cost_share_milli_of_total_true_cost: per_thousand_u64(
                    state.scoreable_verifier_true_cost_ceiling_microusd,
                    scoreable_verifier_true_cost_ceiling_microusd,
                ),
                recommended_next_action,
            }
        })
        .collect::<Vec<_>>();
    route_reports.sort_by(|left, right| {
        right
            .missing_shadow_cost_ceiling_microusd
            .cmp(&left.missing_shadow_cost_ceiling_microusd)
            .then_with(|| {
                right
                    .scoreable_verifier_true_cost_ceiling_microusd
                    .cmp(&left.scoreable_verifier_true_cost_ceiling_microusd)
            })
            .then_with(|| {
                right
                    .missing_shadow_request_rows
                    .cmp(&left.missing_shadow_request_rows)
            })
            .then_with(|| left.bucket_key.cmp(&right.bucket_key))
    });

    let report = GenericShadowRequestGapAuditReport {
        report_kind: "generic_real_traffic_phase_center_shadow_request_gap_audit_v1",
        mode: "audit_only_missing_shadow_request_adapter_gap",
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        total_rows,
        distinct_request_fingerprints: distinct_request_fingerprints.len(),
        shadow_request_rows,
        missing_shadow_request_rows,
        missing_shadow_with_token_or_cost_rows,
        missing_shadow_token_ceiling,
        missing_shadow_cost_ceiling_microusd,
        missing_shadow_not_route_candidate_rows,
        missing_shadow_rejected_candidate_rows,
        missing_shadow_builder_rejected_request_side_features_rows,
        missing_shadow_missing_request_signal_rows,
        missing_shadow_missing_context_signal_rows,
        missing_shadow_missing_evidence_signal_rows,
        missing_shadow_missing_verifier_signal_rows,
        scoreable_shadow_rows,
        scoreable_verifier_true_rows,
        scoreable_verifier_false_rows,
        scoreable_verifier_missing_rows,
        scoreable_verifier_true_token_ceiling,
        scoreable_verifier_true_cost_ceiling_microusd,
        route_reports,
        file_reports,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "audit only: ranks missing nando_shadow_request gaps in real-traffic dry-run traces; does not build payloads, compile .nwpc, promote, serve, local-accept, use target/proof authority, or revive legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("generic_real_traffic_phase_center_shadow_request_gap_audit_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  total_rows: {}", report.total_rows);
    println!("  shadow_request_rows: {}", report.shadow_request_rows);
    println!(
        "  missing_shadow_request_rows: {}",
        report.missing_shadow_request_rows
    );
    println!(
        "  missing_shadow_rejected_candidate_rows: {}",
        report.missing_shadow_rejected_candidate_rows
    );
    println!(
        "  missing_shadow_not_route_candidate_rows: {}",
        report.missing_shadow_not_route_candidate_rows
    );
    println!(
        "  scoreable_verifier_true_rows: {}",
        report.scoreable_verifier_true_rows
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_real_traffic_mining_input_readiness_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_MINING_INPUT_READINESS_REPORT));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_enriched_trace_paths()
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no real-traffic trace paths provided".to_owned());
    }

    let mut total = GenericMiningInputReadinessState::default();
    let mut file_reports = Vec::new();
    for trace_path in &trace_paths {
        let mut state = GenericMiningInputReadinessState::default();
        let text = std::fs::read_to_string(trace_path)
            .map_err(|error| format!("failed to read trace '{}': {error}", trace_path.display()))?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            if !row.is_object() {
                continue;
            }
            mining_input_readiness_observe_row(&mut state, &row);
        }
        merge_mining_input_readiness(&mut total, &state);
        file_reports.push(GenericMiningInputReadinessFileReport {
            path: trace_path.display().to_string(),
            total_rows: state.total_rows,
            shadow_request_rows: state.shadow_request_rows,
            missing_shadow_request_rows: state.missing_shadow_request_rows,
            llm_call_object_rows: state.llm_call_object_rows,
            llm_call_string_rows: state.llm_call_string_rows,
            llm_call_boolean_rows: state.llm_call_boolean_rows,
            llm_call_null_rows: state.llm_call_null_rows,
            tool_fingerprint_rows: state.tool_fingerprint_rows,
            missing_shadow_rows_with_request_side_atoms: state
                .missing_shadow_rows_with_request_side_atoms,
            missing_shadow_rows_with_only_boolean_llm_call: state
                .missing_shadow_rows_with_only_boolean_llm_call,
        });
    }

    let route_family_mining_ready_now = total.missing_shadow_rows_with_request_side_atoms > 0;
    let report = GenericMiningInputReadinessReport {
        report_kind: "generic_real_traffic_phase_center_mining_input_readiness_v1",
        mode: "audit_only_request_side_atom_availability",
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        total_rows: total.total_rows,
        shadow_request_rows: total.shadow_request_rows,
        missing_shadow_request_rows: total.missing_shadow_request_rows,
        llm_call_object_rows: total.llm_call_object_rows,
        llm_call_string_rows: total.llm_call_string_rows,
        llm_call_boolean_rows: total.llm_call_boolean_rows,
        llm_call_null_rows: total.llm_call_null_rows,
        tool_fingerprint_rows: total.tool_fingerprint_rows,
        missing_shadow_rows_with_request_side_atoms: total
            .missing_shadow_rows_with_request_side_atoms,
        missing_shadow_rows_with_only_boolean_llm_call: total
            .missing_shadow_rows_with_only_boolean_llm_call,
        route_family_mining_ready_now,
        required_next_artifact: "real_traffic_phase_atom_trace_v1: request-side state/action/tool atoms plus verifier availability, no raw response, no target/proof labels",
        file_reports,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "audit only: checks whether current real-traffic traces contain request-side atoms needed for route-family phase-center mining; does not compile .nwpc, promote, serve, local-accept, inspect response text, or revive legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("generic_real_traffic_phase_center_mining_input_readiness_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  total_rows: {}", report.total_rows);
    println!("  shadow_request_rows: {}", report.shadow_request_rows);
    println!(
        "  missing_shadow_request_rows: {}",
        report.missing_shadow_request_rows
    );
    println!("  llm_call_boolean_rows: {}", report.llm_call_boolean_rows);
    println!(
        "  missing_shadow_rows_with_request_side_atoms: {}",
        report.missing_shadow_rows_with_request_side_atoms
    );
    println!(
        "  route_family_mining_ready_now: {}",
        report.route_family_mining_ready_now
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_real_traffic_phase_atom_trace_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_PHASE_ATOM_TRACE_REPORT));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_enriched_trace_paths()
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no real-traffic trace paths provided".to_owned());
    }

    let mut state = GenericPhaseAtomTraceBuildState::default();
    let mut output = String::new();
    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path)
            .map_err(|error| format!("failed to read trace '{}': {error}", trace_path.display()))?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            if !row.is_object() {
                continue;
            }
            state.total_rows += 1;
            let atom_row = build_phase_atom_trace_row(trace_path, &row, &mut state);
            output.push_str(
                &serde_json::to_string(&atom_row)
                    .map_err(|error| format!("failed to serialize phase atom row: {error}"))?,
            );
            output.push('\n');
            state.output_rows += 1;
        }
    }

    if let Some(parent) = output_trace_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create phase atom trace directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(&output_trace_path, output).map_err(|error| {
        format!(
            "failed to write phase atom trace '{}': {error}",
            output_trace_path.display()
        )
    })?;
    let report = GenericPhaseAtomTraceBuildReport {
        report_kind: "generic_real_traffic_phase_atom_trace_v1",
        mode: "request_side_phase_atom_trace_builder",
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        output_trace_path: output_trace_path.display().to_string(),
        total_rows: state.total_rows,
        output_rows: state.output_rows,
        rows_with_shadow_request: state.rows_with_shadow_request,
        rows_with_verifier_label: state.rows_with_verifier_label,
        rows_with_token_or_cost: state.rows_with_token_or_cost,
        rows_with_explicit_request_atoms: state.rows_with_explicit_request_atoms,
        rows_with_explicit_state_atoms: state.rows_with_explicit_state_atoms,
        rows_with_explicit_action_atoms: state.rows_with_explicit_action_atoms,
        rows_with_explicit_tool_atoms: state.rows_with_explicit_tool_atoms,
        rows_with_shadow_payload_atoms: state.rows_with_shadow_payload_atoms,
        rows_with_provider_correlation_keys: state.rows_with_provider_correlation_keys,
        metadata_only_rows: state.metadata_only_rows,
        rows_ready_for_route_family_mining: state.rows_ready_for_route_family_mining,
        rows_ready_for_existing_shadow_scoring: state.rows_ready_for_existing_shadow_scoring,
        rows_missing_state_or_request_atoms: state.rows_missing_state_or_request_atoms,
        rows_missing_action_atoms: state.rows_missing_action_atoms,
        rows_missing_verifier_label: state.rows_missing_verifier_label,
        output_atoms_written: state.output_atoms_written,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "builder only: writes request-side phase atom trace rows from existing telemetry; does not inspect raw response text, use target/proof labels, compile .nwpc, promote, serve, local-accept, or revive legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("generic_real_traffic_phase_atom_trace_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  output_trace_path: {}", output_trace_path.display());
    println!("  total_rows: {}", report.total_rows);
    println!(
        "  rows_ready_for_route_family_mining: {}",
        report.rows_ready_for_route_family_mining
    );
    println!(
        "  rows_ready_for_existing_shadow_scoring: {}",
        report.rows_ready_for_existing_shadow_scoring
    );
    println!("  metadata_only_rows: {}", report.metadata_only_rows);
    println!(
        "  rows_with_provider_correlation_keys: {}",
        report.rows_with_provider_correlation_keys
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_codex_history_phase_atom_trace_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_PHASE_ATOM_TRACE_REPORT));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_PHASE_ATOM_TRACE_JSONL));
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_PATH));
    let max_rows = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_rows value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CODEX_HISTORY_PHASE_ATOM_MAX_ROWS);
    if max_rows == 0 {
        return Err("max_rows must be > 0".to_owned());
    }

    let text = std::fs::read_to_string(&history_path).map_err(|error| {
        format!(
            "failed to read Codex history '{}': {error}",
            history_path.display()
        )
    })?;
    let mut rows = Vec::new();
    let mut history_rows_seen = 0usize;
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse Codex history '{}' line {}: {error}",
                history_path.display(),
                line_index + 1
            )
        })?;
        let Some(prompt) = row.get("text").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let session_id = row
            .get("session_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown_session")
            .to_owned();
        let ts = row
            .get("ts")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        rows.push((session_id, ts, prompt.to_owned()));
        history_rows_seen += 1;
    }

    let start = rows.len().saturating_sub(max_rows);
    let sampled = &rows[start..];
    let mut output = String::new();
    let mut session_turn_counts = BTreeMap::<String, usize>::new();
    let mut top_action_counts = BTreeMap::<String, usize>::new();
    let mut top_tool_counts = BTreeMap::<String, usize>::new();
    let mut output_rows = 0usize;
    let mut rows_with_request_atoms = 0usize;
    let mut rows_with_state_atoms = 0usize;
    let mut rows_with_action_atoms = 0usize;
    let mut rows_with_tool_atoms = 0usize;
    let rows_with_verifier_label = 0usize;
    let mut rows_ready_for_action_family_clustering = 0usize;
    let rows_ready_for_route_family_mining = 0usize;
    let mut estimated_total_tokens = 0usize;

    for (sample_index, (session_id, ts, prompt)) in sampled.iter().enumerate() {
        let session_turn = session_turn_counts.entry(session_id.clone()).or_insert(0);
        *session_turn += 1;
        let ts_text = ts.to_string();
        let text_hash = stable_fingerprint([session_id.as_str(), ts_text.as_str(), prompt]);
        let request_hash = stable_fingerprint([prompt.as_str()]);
        let request_fingerprint = format!("codex_history_phase:{text_hash:016x}");
        let exact_cache_key = format!("codex_history_request:{request_hash:016x}");
        let estimated_tokens = prompt.chars().count().saturating_add(3) / 4;
        estimated_total_tokens = estimated_total_tokens.saturating_add(estimated_tokens);

        let request_atoms = codex_history_request_atoms(prompt);
        let state_atoms = codex_history_state_atoms(session_id, *session_turn, prompt);
        let action_atoms = codex_history_action_atoms(prompt);
        let tool_atoms = codex_history_tool_atoms(prompt);
        rows_with_request_atoms += usize::from(!request_atoms.is_empty());
        rows_with_state_atoms += usize::from(!state_atoms.is_empty());
        rows_with_action_atoms += usize::from(!action_atoms.is_empty());
        rows_with_tool_atoms += usize::from(!tool_atoms.is_empty());
        rows_ready_for_action_family_clustering +=
            usize::from(!request_atoms.is_empty() && !action_atoms.is_empty());

        for atom in action_atoms
            .iter()
            .filter(|atom| atom.starts_with("action_family:"))
        {
            *top_action_counts.entry(atom.clone()).or_insert(0) += 1;
        }
        for atom in &tool_atoms {
            *top_tool_counts.entry(atom.clone()).or_insert(0) += 1;
        }

        let metadata_atoms = vec![
            "traffic_source_kind:codex_history".to_owned(),
            "verification_source_kind:none".to_owned(),
            "llm_call_kind:boolean".to_owned(),
            "has_shadow_request:false".to_owned(),
            "has_verifier_label:false".to_owned(),
            "synthetic_source:false".to_owned(),
            format!("token_band:{}", generic_count_band(estimated_tokens)),
            "cost_band:0".to_owned(),
        ];
        let route_hint_atoms = codex_history_route_hint_atoms(&action_atoms);
        let output_atoms_written = metadata_atoms.len()
            + route_hint_atoms.len()
            + request_atoms.len()
            + state_atoms.len()
            + action_atoms.len()
            + tool_atoms.len();

        let atom_row = serde_json::json!({
            "schema_version": "real_traffic_phase_atom_trace_v1",
            "source_schema_version": "codex_history_phase_atom_ingest_v1",
            "input_trace_path": history_path.display().to_string(),
            "trace_id": format!("codex-history-phase-{sample_index:06}"),
            "time_ms": ts.saturating_mul(1000),
            "request_fingerprint": request_fingerprint,
            "exact_cache_key": exact_cache_key,
            "traffic_source": "codex_history_phase_atom_ingest_v1",
            "verification_source_kind": "none",
            "verified_safe_accept": serde_json::Value::Null,
            "has_shadow_request": false,
            "ready_for_route_family_mining": false,
            "ready_for_existing_shadow_scoring": false,
            "ready_for_action_family_clustering": !request_atoms.is_empty() && !action_atoms.is_empty(),
            "metadata_only": false,
            "missing_state_or_request_atoms": false,
            "missing_action_atoms": action_atoms.is_empty(),
            "missing_verifier_label": true,
            "token_cost": {
                "total_tokens": estimated_tokens,
                "total_cost_microusd": 0,
                "token_evidence_missing": false,
                "cost_evidence_missing": true,
                "token_cost_estimate_used": true
            },
            "request_atoms": request_atoms,
            "state_atoms": state_atoms,
            "action_atoms": action_atoms,
            "tool_atoms": tool_atoms,
            "atom_groups": {
                "metadata_atoms": metadata_atoms,
                "route_hint_atoms": route_hint_atoms,
                "request_atoms": request_atoms,
                "state_atoms": state_atoms,
                "action_atoms": action_atoms,
                "tool_atoms": tool_atoms,
                "derived_tool_atoms": [],
                "shadow_payload_atoms": []
            },
            "output_atoms_written": output_atoms_written,
            "forbidden_fields_absent": {
                "raw_request_text": true,
                "raw_response_text": true,
                "target_id": true,
                "proof_rule_id": true,
                "concrete_x_lookup": true,
                "manual_local_out_t": true
            }
        });
        output.push_str(
            &serde_json::to_string(&atom_row)
                .map_err(|error| format!("failed to serialize Codex phase atom row: {error}"))?,
        );
        output.push('\n');
        output_rows += 1;
    }

    if let Some(parent) = output_trace_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create Codex phase atom trace directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(&output_trace_path, output).map_err(|error| {
        format!(
            "failed to write Codex phase atom trace '{}': {error}",
            output_trace_path.display()
        )
    })?;

    let report = CodexHistoryPhaseAtomTraceReport {
        report_kind: "codex_history_phase_atom_trace_v1",
        mode: "request_side_phase_atom_trace_ingest",
        history_path: history_path.display().to_string(),
        output_trace_path: output_trace_path.display().to_string(),
        max_rows,
        history_rows_seen,
        sampled_rows: sampled.len(),
        output_rows,
        rows_with_request_atoms,
        rows_with_state_atoms,
        rows_with_action_atoms,
        rows_with_tool_atoms,
        rows_with_verifier_label,
        rows_ready_for_route_family_mining,
        rows_ready_for_action_family_clustering,
        estimated_total_tokens,
        top_action_families: atom_count_reports(top_action_counts, 16),
        top_tool_atoms: atom_count_reports(top_tool_counts, 16),
        raw_text_written: false,
        raw_response_text_written: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "ingest only: reads local Codex request history at analysis time and writes only phase atoms/fingerprints; no raw request text, raw response text, verifier labels, .nwpc compile, promotion, serving, local_accept, market claim, or legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("codex_history_phase_atom_trace_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  output_trace_path: {}", output_trace_path.display());
    println!("  history_rows_seen: {}", report.history_rows_seen);
    println!("  output_rows: {}", report.output_rows);
    println!(
        "  rows_ready_for_action_family_clustering: {}",
        report.rows_ready_for_action_family_clustering
    );
    println!(
        "  rows_ready_for_route_family_mining: {}",
        report.rows_ready_for_route_family_mining
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_verifier_needed_ranking_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_VERIFIER_NEEDED_RANKING_REPORT));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(DEFAULT_CODEX_HISTORY_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no phase atom trace paths provided".to_owned());
    }

    let mut total_rows = 0usize;
    let mut rows_with_action_family = 0usize;
    let mut rows_with_verifier_label = 0usize;
    let mut exact_cache_hits = 0usize;
    let mut verifier_true_rows = 0usize;
    let mut verifier_false_rows = 0usize;
    let mut verifier_true_over_exact_cache_ceiling = 0usize;
    let mut verifier_true_tokens_over_exact_cache_ceiling = 0usize;
    let mut verifier_true_cost_microusd_over_exact_cache_ceiling = 0u64;
    let mut rows_missing_verifier_label_over_exact_cache = 0usize;
    let mut rows_with_shadow_request = 0usize;
    let mut rows_missing_shadow_request = 0usize;
    let mut rows_with_result_atoms = 0usize;
    let mut rows_missing_result_atoms = 0usize;
    let mut rows_ready_for_action_family_clustering = 0usize;
    let mut rows_ready_for_route_family_mining = 0usize;
    let mut rows_ready_for_existing_shadow_scoring = 0usize;
    let mut estimated_total_tokens = 0usize;
    let mut estimated_total_cost_microusd = 0u64;
    let mut token_events = 0usize;
    let mut provider_cost_events = 0usize;
    let mut estimated_cost_events = 0usize;
    let mut seen_exact_cache_keys = BTreeSet::<String>::new();
    let mut action_states = BTreeMap::<String, PhaseAtomActionFamilyState>::new();
    let mut bucket_states = BTreeMap::<String, PhaseAtomStateActionBucketState>::new();

    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read phase atom trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse phase atom trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            if !row.is_object() {
                continue;
            }
            total_rows += 1;
            let exact_cache_key = json_string(&row, &["exact_cache_key"])
                .unwrap_or_else(|| format!("phase-atom-ranking-row:{total_rows:08}"));
            let exact_cache_hit = !seen_exact_cache_keys.insert(exact_cache_key);
            exact_cache_hits += usize::from(exact_cache_hit);
            let token_cost = GenericTokenCost {
                total_tokens: json_usize(json_at(&row, &["token_cost", "total_tokens"]))
                    .unwrap_or(0),
                total_cost_microusd: json_at(&row, &["token_cost", "total_cost_microusd"])
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
                evidence_missing: false,
                token_evidence_missing: false,
                cost_evidence_missing: false,
            };
            estimated_total_tokens = estimated_total_tokens.saturating_add(token_cost.total_tokens);
            estimated_total_cost_microusd =
                estimated_total_cost_microusd.saturating_add(token_cost.total_cost_microusd);
            let has_tokens = token_cost.total_tokens > 0;
            let has_provider_cost = row
                .get("provider_cost_microusd")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                > 0;
            let has_estimated_cost = token_cost.total_cost_microusd > 0 && !has_provider_cost;
            token_events += usize::from(has_tokens);
            provider_cost_events += usize::from(has_provider_cost);
            estimated_cost_events += usize::from(has_estimated_cost);
            let verifier_label = row
                .get("verified_safe_accept")
                .and_then(serde_json::Value::as_bool);
            let has_verifier_label = verifier_label.is_some();
            rows_with_verifier_label += usize::from(has_verifier_label);
            verifier_true_rows += usize::from(verifier_label == Some(true));
            verifier_false_rows += usize::from(verifier_label == Some(false));
            verifier_true_over_exact_cache_ceiling +=
                usize::from(verifier_label == Some(true) && !exact_cache_hit);
            if verifier_label == Some(true) && !exact_cache_hit {
                verifier_true_tokens_over_exact_cache_ceiling =
                    verifier_true_tokens_over_exact_cache_ceiling
                        .saturating_add(token_cost.total_tokens);
                verifier_true_cost_microusd_over_exact_cache_ceiling =
                    verifier_true_cost_microusd_over_exact_cache_ceiling
                        .saturating_add(token_cost.total_cost_microusd);
            }
            rows_missing_verifier_label_over_exact_cache +=
                usize::from(verifier_label.is_none() && !exact_cache_hit);
            let has_shadow_request = json_bool(&row, &["has_shadow_request"])
                .or_else(|| Some(row.get("nando_shadow_request").is_some()))
                .unwrap_or(false);
            rows_with_shadow_request += usize::from(has_shadow_request);
            rows_missing_shadow_request += usize::from(!has_shadow_request);
            let has_result_atoms = row
                .get("result_atoms")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|items| !items.is_empty());
            rows_with_result_atoms += usize::from(has_result_atoms);
            rows_missing_result_atoms += usize::from(!has_result_atoms);
            let ready_for_action_family_clustering = row
                .get("ready_for_action_family_clustering")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let ready_for_route_family_mining = row
                .get("ready_for_route_family_mining")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let ready_for_existing_shadow_scoring = row
                .get("ready_for_existing_shadow_scoring")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            rows_ready_for_action_family_clustering +=
                usize::from(ready_for_action_family_clustering);
            rows_ready_for_route_family_mining += usize::from(ready_for_route_family_mining);
            rows_ready_for_existing_shadow_scoring +=
                usize::from(ready_for_existing_shadow_scoring);

            let request_atoms = phase_atom_string_vec(&row, "request_atoms");
            let state_atoms = phase_atom_string_vec(&row, "state_atoms");
            let action_atoms = phase_atom_string_vec(&row, "action_atoms");
            let tool_atoms = phase_atom_string_vec(&row, "tool_atoms");
            let route_hint_atoms = phase_atom_string_vec(&row, "route_hint_atoms");
            let action_families = phase_atom_action_families(&action_atoms);
            rows_with_action_family += usize::from(!action_families.is_empty());
            for action_family in action_families {
                let bucket_key = phase_atom_state_action_bucket_key(
                    &action_family,
                    &request_atoms,
                    &state_atoms,
                    &tool_atoms,
                    &route_hint_atoms,
                );
                let action_state =
                    action_states
                        .entry(action_family.clone())
                        .or_insert_with(|| PhaseAtomActionFamilyState {
                            action_family: action_family.clone(),
                            ..Default::default()
                        });
                action_state.rows += 1;
                action_state.estimated_tokens = action_state
                    .estimated_tokens
                    .saturating_add(token_cost.total_tokens);
                action_state.estimated_cost_microusd = action_state
                    .estimated_cost_microusd
                    .saturating_add(token_cost.total_cost_microusd);
                action_state.exact_cache_hits += usize::from(exact_cache_hit);
                action_state.token_events += usize::from(has_tokens);
                action_state.provider_cost_events += usize::from(has_provider_cost);
                action_state.estimated_cost_events += usize::from(has_estimated_cost);
                action_state.rows_with_verifier_label += usize::from(has_verifier_label);
                action_state.verifier_true_rows += usize::from(verifier_label == Some(true));
                action_state.verifier_false_rows += usize::from(verifier_label == Some(false));
                action_state.verifier_true_over_exact_cache_ceiling +=
                    usize::from(verifier_label == Some(true) && !exact_cache_hit);
                if verifier_label == Some(true) && !exact_cache_hit {
                    action_state.verifier_true_tokens_over_exact_cache_ceiling = action_state
                        .verifier_true_tokens_over_exact_cache_ceiling
                        .saturating_add(token_cost.total_tokens);
                    action_state.verifier_true_cost_microusd_over_exact_cache_ceiling =
                        action_state
                            .verifier_true_cost_microusd_over_exact_cache_ceiling
                            .saturating_add(token_cost.total_cost_microusd);
                }
                action_state.rows_missing_verifier_label_over_exact_cache +=
                    usize::from(verifier_label.is_none() && !exact_cache_hit);
                action_state.rows_with_shadow_request += usize::from(has_shadow_request);
                action_state.rows_with_result_atoms += usize::from(has_result_atoms);
                action_state.rows_ready_for_existing_shadow_scoring +=
                    usize::from(ready_for_existing_shadow_scoring);
                action_state.rows_ready_for_action_family_clustering +=
                    usize::from(ready_for_action_family_clustering);
                action_state.rows_ready_for_route_family_mining +=
                    usize::from(ready_for_route_family_mining);
                action_state.rows_with_tool_atoms += usize::from(!tool_atoms.is_empty());
                action_state.state_action_buckets.insert(bucket_key.clone());
                for atom in &route_hint_atoms {
                    *action_state
                        .route_hint_counts
                        .entry(atom.clone())
                        .or_insert(0) += 1;
                }
                for atom in &tool_atoms {
                    *action_state
                        .tool_atom_counts
                        .entry(atom.clone())
                        .or_insert(0) += 1;
                }

                let bucket_state = bucket_states.entry(bucket_key.clone()).or_insert_with(|| {
                    PhaseAtomStateActionBucketState {
                        bucket_key: bucket_key.clone(),
                        action_family: action_family.clone(),
                        ..Default::default()
                    }
                });
                bucket_state.rows += 1;
                bucket_state.estimated_tokens = bucket_state
                    .estimated_tokens
                    .saturating_add(token_cost.total_tokens);
                bucket_state.estimated_cost_microusd = bucket_state
                    .estimated_cost_microusd
                    .saturating_add(token_cost.total_cost_microusd);
                bucket_state.exact_cache_hits += usize::from(exact_cache_hit);
                bucket_state.token_events += usize::from(has_tokens);
                bucket_state.provider_cost_events += usize::from(has_provider_cost);
                bucket_state.estimated_cost_events += usize::from(has_estimated_cost);
                bucket_state.rows_with_verifier_label += usize::from(has_verifier_label);
                bucket_state.verifier_true_rows += usize::from(verifier_label == Some(true));
                bucket_state.verifier_false_rows += usize::from(verifier_label == Some(false));
                bucket_state.verifier_true_over_exact_cache_ceiling +=
                    usize::from(verifier_label == Some(true) && !exact_cache_hit);
                if verifier_label == Some(true) && !exact_cache_hit {
                    bucket_state.verifier_true_tokens_over_exact_cache_ceiling = bucket_state
                        .verifier_true_tokens_over_exact_cache_ceiling
                        .saturating_add(token_cost.total_tokens);
                    bucket_state.verifier_true_cost_microusd_over_exact_cache_ceiling =
                        bucket_state
                            .verifier_true_cost_microusd_over_exact_cache_ceiling
                            .saturating_add(token_cost.total_cost_microusd);
                }
                bucket_state.rows_missing_verifier_label_over_exact_cache +=
                    usize::from(verifier_label.is_none() && !exact_cache_hit);
                bucket_state.rows_with_shadow_request += usize::from(has_shadow_request);
                bucket_state.rows_with_result_atoms += usize::from(has_result_atoms);
                for atom in &route_hint_atoms {
                    *bucket_state
                        .route_hint_counts
                        .entry(atom.clone())
                        .or_insert(0) += 1;
                }
            }
        }
    }

    let action_family_count = action_states.len();
    let state_action_bucket_count = bucket_states.len();
    let action_family_reports = action_states
        .into_values()
        .map(|state| {
            let recommended_next_action = verifier_needed_recommended_next_action(&state);
            let false_accept_risk = phase_atom_false_accept_risk(
                state.verifier_true_rows,
                state.verifier_false_rows,
                state.rows.saturating_sub(state.rows_with_verifier_label),
                state.rows_ready_for_route_family_mining > 0,
            );
            let daemon_next_action = phase_atom_daemon_next_action(PhaseAtomDaemonActionInput {
                action_family: &state.action_family,
                rows_with_verifier_label: state.rows_with_verifier_label,
                verifier_true_rows: state.verifier_true_rows,
                verifier_false_rows: state.verifier_false_rows,
                verifier_true_over_exact_cache_ceiling: state
                    .verifier_true_over_exact_cache_ceiling,
                expected_tokens_saved_over_exact_cache: state
                    .verifier_true_tokens_over_exact_cache_ceiling,
                expected_cost_saved_microusd_over_exact_cache: state
                    .verifier_true_cost_microusd_over_exact_cache_ceiling,
                provider_cost_events: state.provider_cost_events,
                estimated_cost_events: state.estimated_cost_events,
                rows_with_shadow_request: state.rows_with_shadow_request,
                rows_with_result_atoms: state.rows_with_result_atoms,
                rows_ready_for_route_family_mining: state.rows_ready_for_route_family_mining,
            });
            PhaseAtomActionFamilyRankingReport {
                action_family: state.action_family.clone(),
                rows: state.rows,
                traffic_share_milli: per_thousand(state.rows, total_rows),
                estimated_tokens: state.estimated_tokens,
                estimated_cost_microusd: state.estimated_cost_microusd,
                exact_cache_hits: state.exact_cache_hits,
                exact_cache_misses_over_cache: state.rows.saturating_sub(state.exact_cache_hits),
                exact_cache_overlap_milli: per_thousand(state.exact_cache_hits, state.rows),
                rows_with_verifier_label: state.rows_with_verifier_label,
                verifier_true_rows: state.verifier_true_rows,
                verifier_false_rows: state.verifier_false_rows,
                verifier_true_over_exact_cache_ceiling: state
                    .verifier_true_over_exact_cache_ceiling,
                expected_unique_cpu_accepts_over_exact_cache: state
                    .verifier_true_over_exact_cache_ceiling,
                expected_tokens_saved_over_exact_cache: state
                    .verifier_true_tokens_over_exact_cache_ceiling,
                expected_cost_saved_microusd_over_exact_cache: state
                    .verifier_true_cost_microusd_over_exact_cache_ceiling,
                rows_missing_verifier_label: state
                    .rows
                    .saturating_sub(state.rows_with_verifier_label),
                rows_missing_verifier_label_over_exact_cache: state
                    .rows_missing_verifier_label_over_exact_cache,
                token_events: state.token_events,
                provider_cost_events: state.provider_cost_events,
                estimated_cost_events: state.estimated_cost_events,
                rows_with_shadow_request: state.rows_with_shadow_request,
                rows_missing_shadow_request: state
                    .rows
                    .saturating_sub(state.rows_with_shadow_request),
                rows_with_result_atoms: state.rows_with_result_atoms,
                rows_missing_result_atoms: state.rows.saturating_sub(state.rows_with_result_atoms),
                rows_ready_for_action_family_clustering: state
                    .rows_ready_for_action_family_clustering,
                rows_ready_for_route_family_mining: state.rows_ready_for_route_family_mining,
                rows_ready_for_existing_shadow_scoring: state
                    .rows_ready_for_existing_shadow_scoring,
                rows_with_tool_atoms: state.rows_with_tool_atoms,
                distinct_state_action_buckets: state.state_action_buckets.len(),
                top_route_hints: atom_count_reports(state.route_hint_counts, 8),
                top_tool_atoms: atom_count_reports(state.tool_atom_counts, 8),
                recommended_verifier_capture: recommended_verifier_capture_for_action_family(
                    &state.action_family,
                ),
                recommended_next_action,
                false_accept_risk,
                daemon_next_action,
                compile_allowed: false,
            }
        })
        .collect::<Vec<_>>();
    let mut top_action_families = action_family_reports.clone();
    top_action_families.sort_by(|left, right| {
        right
            .rows
            .cmp(&left.rows)
            .then_with(|| right.estimated_tokens.cmp(&left.estimated_tokens))
            .then_with(|| left.action_family.cmp(&right.action_family))
    });
    top_action_families.truncate(32);
    let mut top_value_action_families = action_family_reports;
    top_value_action_families.sort_by(|left, right| {
        right
            .expected_cost_saved_microusd_over_exact_cache
            .cmp(&left.expected_cost_saved_microusd_over_exact_cache)
            .then_with(|| {
                right
                    .expected_tokens_saved_over_exact_cache
                    .cmp(&left.expected_tokens_saved_over_exact_cache)
            })
            .then_with(|| {
                right
                    .expected_unique_cpu_accepts_over_exact_cache
                    .cmp(&left.expected_unique_cpu_accepts_over_exact_cache)
            })
            .then_with(|| right.rows.cmp(&left.rows))
            .then_with(|| left.action_family.cmp(&right.action_family))
    });
    top_value_action_families.truncate(32);

    let mut top_state_action_buckets = bucket_states
        .into_values()
        .map(|state| {
            let recommended_next_action =
                if state.action_family == "action_family:dialogue_or_unknown" {
                    "split_unknown_bucket_before_verifier_capture"
                } else if state.rows_with_verifier_label == 0 {
                    "attach_verifier_or_result_capture"
                } else {
                    "eligible_for_shadow_phase_center_review"
                };
            let false_accept_risk = phase_atom_false_accept_risk(
                state.verifier_true_rows,
                state.verifier_false_rows,
                state.rows.saturating_sub(state.rows_with_verifier_label),
                state.rows_with_verifier_label > 0,
            );
            let daemon_next_action = phase_atom_daemon_next_action(PhaseAtomDaemonActionInput {
                action_family: &state.action_family,
                rows_with_verifier_label: state.rows_with_verifier_label,
                verifier_true_rows: state.verifier_true_rows,
                verifier_false_rows: state.verifier_false_rows,
                verifier_true_over_exact_cache_ceiling: state
                    .verifier_true_over_exact_cache_ceiling,
                expected_tokens_saved_over_exact_cache: state
                    .verifier_true_tokens_over_exact_cache_ceiling,
                expected_cost_saved_microusd_over_exact_cache: state
                    .verifier_true_cost_microusd_over_exact_cache_ceiling,
                provider_cost_events: state.provider_cost_events,
                estimated_cost_events: state.estimated_cost_events,
                rows_with_shadow_request: state.rows_with_shadow_request,
                rows_with_result_atoms: state.rows_with_result_atoms,
                rows_ready_for_route_family_mining: state.rows_with_verifier_label,
            });
            PhaseAtomStateActionBucketReport {
                bucket_key: state.bucket_key,
                action_family: state.action_family,
                rows: state.rows,
                estimated_tokens: state.estimated_tokens,
                estimated_cost_microusd: state.estimated_cost_microusd,
                exact_cache_hits: state.exact_cache_hits,
                exact_cache_misses_over_cache: state.rows.saturating_sub(state.exact_cache_hits),
                exact_cache_overlap_milli: per_thousand(state.exact_cache_hits, state.rows),
                rows_with_verifier_label: state.rows_with_verifier_label,
                verifier_true_rows: state.verifier_true_rows,
                verifier_false_rows: state.verifier_false_rows,
                verifier_true_over_exact_cache_ceiling: state
                    .verifier_true_over_exact_cache_ceiling,
                expected_tokens_saved_over_exact_cache: state
                    .verifier_true_tokens_over_exact_cache_ceiling,
                expected_cost_saved_microusd_over_exact_cache: state
                    .verifier_true_cost_microusd_over_exact_cache_ceiling,
                rows_missing_verifier_label: state
                    .rows
                    .saturating_sub(state.rows_with_verifier_label),
                rows_missing_verifier_label_over_exact_cache: state
                    .rows_missing_verifier_label_over_exact_cache,
                token_events: state.token_events,
                provider_cost_events: state.provider_cost_events,
                estimated_cost_events: state.estimated_cost_events,
                rows_with_shadow_request: state.rows_with_shadow_request,
                rows_missing_shadow_request: state
                    .rows
                    .saturating_sub(state.rows_with_shadow_request),
                rows_with_result_atoms: state.rows_with_result_atoms,
                rows_missing_result_atoms: state.rows.saturating_sub(state.rows_with_result_atoms),
                top_route_hints: atom_count_reports(state.route_hint_counts, 6),
                recommended_next_action,
                false_accept_risk,
                daemon_next_action,
            }
        })
        .collect::<Vec<_>>();
    top_state_action_buckets.sort_by(|left, right| {
        right
            .rows
            .cmp(&left.rows)
            .then_with(|| right.estimated_tokens.cmp(&left.estimated_tokens))
            .then_with(|| left.bucket_key.cmp(&right.bucket_key))
    });
    top_state_action_buckets.truncate(64);

    let exact_cache_misses_over_cache = total_rows.saturating_sub(exact_cache_hits);
    let cpu10_target_unique_accepts = total_rows.saturating_add(9) / 10;
    let remaining_verifier_true_accept_gap_to_cpu10 =
        cpu10_target_unique_accepts.saturating_sub(verifier_true_over_exact_cache_ceiling);
    let current_labeled_pool_can_reach_cpu10 =
        verifier_true_over_exact_cache_ceiling >= cpu10_target_unique_accepts;

    let report = PhaseAtomVerifierNeededRankingReport {
        report_kind: "phase_atom_verifier_needed_ranking_v1",
        mode: "phase_atom_action_family_value_ranking_without_promotion",
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        total_rows,
        rows_with_action_family,
        rows_with_verifier_label,
        exact_cache_hits,
        exact_cache_misses_over_cache,
        exact_cache_overlap_milli: per_thousand(exact_cache_hits, total_rows),
        verifier_true_rows,
        verifier_false_rows,
        verifier_true_over_exact_cache_ceiling,
        verifier_true_tokens_over_exact_cache_ceiling,
        verifier_true_cost_microusd_over_exact_cache_ceiling,
        rows_missing_verifier_label_over_exact_cache,
        rows_with_shadow_request,
        rows_missing_shadow_request,
        rows_with_result_atoms,
        rows_missing_result_atoms,
        rows_ready_for_action_family_clustering,
        rows_ready_for_route_family_mining,
        rows_ready_for_existing_shadow_scoring,
        estimated_total_tokens,
        estimated_total_cost_microusd,
        token_events,
        provider_cost_events,
        estimated_cost_events,
        cpu10_target_unique_accepts,
        remaining_verifier_true_accept_gap_to_cpu10,
        current_labeled_pool_can_reach_cpu10,
        action_family_count,
        state_action_bucket_count,
        top_action_families,
        top_value_action_families,
        top_state_action_buckets,
        compile_allowed: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "ranking only: reads phase atom traces, ranks action families that need verifier/result capture, and does not compile .nwpc, promote, serve, local_accept, claim savings, use target/proof authority, or revive legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_atom_verifier_needed_ranking_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  total_rows: {}", report.total_rows);
    println!(
        "  rows_with_action_family: {}",
        report.rows_with_action_family
    );
    println!(
        "  rows_with_verifier_label: {}",
        report.rows_with_verifier_label
    );
    println!("  exact_cache_hits: {}", report.exact_cache_hits);
    println!(
        "  verifier_true_over_exact_cache_ceiling: {}",
        report.verifier_true_over_exact_cache_ceiling
    );
    println!(
        "  cpu10_target_unique_accepts: {}",
        report.cpu10_target_unique_accepts
    );
    println!(
        "  remaining_verifier_true_accept_gap_to_cpu10: {}",
        report.remaining_verifier_true_accept_gap_to_cpu10
    );
    println!(
        "  rows_missing_verifier_label_over_exact_cache: {}",
        report.rows_missing_verifier_label_over_exact_cache
    );
    println!(
        "  rows_missing_shadow_request: {}",
        report.rows_missing_shadow_request
    );
    println!("  action_family_count: {}", report.action_family_count);
    println!(
        "  state_action_bucket_count: {}",
        report.state_action_bucket_count
    );
    println!("  compile_allowed: {}", report.compile_allowed);
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_action_family_separability_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let action_family_filter = args
        .next()
        .unwrap_or_else(|| "action_family:planning".to_owned());
    if !action_family_filter.starts_with("action_family:") {
        return Err(format!(
            "action_family filter must start with action_family:, got '{action_family_filter}'"
        ));
    }
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                "target/nando-wave/streaming/phase-atom-action-family-separability-audit-v1.report.json",
            )
        });
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no phase atom trace paths provided".to_owned());
    }

    let base_action_family = phase_atom_base_action_family(&action_family_filter).to_owned();
    let task_name = phase_atom_live_self_mining_task_name(&action_family_filter);
    let bucket_filter = action_family_filter.contains("::");
    let mut total_rows = 0usize;
    let mut matched_rows = 0usize;
    let mut positive_rows = 0usize;
    let mut negative_rows = 0usize;
    let mut rows_missing_verifier_label = 0usize;
    let mut positive_atom_counts = BTreeMap::<String, usize>::new();
    let mut negative_atom_counts = BTreeMap::<String, usize>::new();
    let mut atom_pair_counts = BTreeMap::<String, (usize, usize)>::new();

    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read phase atom separability trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows += 1;
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse phase atom separability trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            let action_atoms = phase_atom_string_vec(&row, "action_atoms");
            if !action_atoms.iter().any(|atom| atom == &base_action_family) {
                continue;
            }
            if bucket_filter {
                let request_atoms = phase_atom_string_vec(&row, "request_atoms");
                let state_atoms = phase_atom_string_vec(&row, "state_atoms");
                let tool_atoms = phase_atom_string_vec(&row, "tool_atoms");
                let route_hint_atoms = phase_atom_string_vec(&row, "route_hint_atoms");
                let bucket_key = phase_atom_state_action_bucket_key(
                    &base_action_family,
                    &request_atoms,
                    &state_atoms,
                    &tool_atoms,
                    &route_hint_atoms,
                );
                if bucket_key != action_family_filter {
                    continue;
                }
            }
            if row
                .get("verified_safe_accept")
                .and_then(serde_json::Value::as_bool)
                .is_none()
            {
                rows_missing_verifier_label += 1;
                continue;
            }
            let Some(event) = parse_phase_atom_binary_event_for_action(
                &row,
                matched_rows,
                &base_action_family,
                &task_name,
            ) else {
                continue;
            };
            matched_rows += 1;
            if event.verified_safe_accept {
                positive_rows += 1;
            } else {
                negative_rows += 1;
            }
            for atom in event.base_atoms {
                if atom.starts_with("phase_atom_binary_task:") {
                    continue;
                }
                let pair = atom_pair_counts.entry(atom.clone()).or_insert((0, 0));
                if event.verified_safe_accept {
                    *positive_atom_counts.entry(atom).or_insert(0) += 1;
                    pair.0 += 1;
                } else {
                    *negative_atom_counts.entry(atom).or_insert(0) += 1;
                    pair.1 += 1;
                }
            }
        }
    }

    let mut enrichment = atom_pair_counts
        .into_iter()
        .map(|(atom, (positive_count, negative_count))| {
            let positive_rate_milli = per_thousand(positive_count, positive_rows);
            let negative_rate_milli = per_thousand(negative_count, negative_rows);
            PhaseAtomLabelEnrichmentReport {
                atom,
                positive_count,
                negative_count,
                positive_rate_milli,
                negative_rate_milli,
                delta_milli: positive_rate_milli as i64 - negative_rate_milli as i64,
            }
        })
        .collect::<Vec<_>>();
    let distinct_base_atoms = enrichment.len();
    enrichment.sort_by(|left, right| {
        right
            .delta_milli
            .cmp(&left.delta_milli)
            .then_with(|| right.positive_count.cmp(&left.positive_count))
            .then_with(|| left.atom.cmp(&right.atom))
    });
    let top_positive_enriched_atoms = enrichment.iter().take(16).cloned().collect::<Vec<_>>();
    enrichment.sort_by(|left, right| {
        left.delta_milli
            .cmp(&right.delta_milli)
            .then_with(|| right.negative_count.cmp(&left.negative_count))
            .then_with(|| left.atom.cmp(&right.atom))
    });
    let top_negative_enriched_atoms = enrichment.iter().take(16).cloned().collect::<Vec<_>>();
    let max_positive_delta_milli = top_positive_enriched_atoms
        .first()
        .map_or(0, |row| row.delta_milli);
    let max_negative_delta_milli = top_negative_enriched_atoms
        .first()
        .map_or(0, |row| row.delta_milli);
    let label_balance_milli = if matched_rows == 0 {
        0
    } else {
        let minority = positive_rows.min(negative_rows);
        per_thousand(minority * 2, matched_rows)
    };
    let recommended_next_action = if matched_rows == 0 {
        "no_rows_for_action_family_or_bucket"
    } else if positive_rows == 0 || negative_rows == 0 {
        "capture_both_positive_and_negative_verifier_labels"
    } else if max_positive_delta_milli < 150 && max_negative_delta_milli > -150 {
        "add_result_or_state_atoms_before_phase_center_compile"
    } else if label_balance_milli < 250 {
        "collect_balanced_positive_negative_verifier_window"
    } else {
        "rerun_time_split_discovery_or_bucket_split_with_this_evidence"
    };

    let report = PhaseAtomActionFamilySeparabilityAuditReport {
        report_kind: "phase_atom_action_family_separability_audit_v1",
        mode: "diagnostic_only_label_conditioned_atom_separability_no_compile",
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        action_family_filter,
        base_action_family,
        task_name,
        total_rows,
        matched_rows,
        positive_rows,
        negative_rows,
        rows_missing_verifier_label,
        distinct_base_atoms,
        top_positive_atoms: atom_count_reports(positive_atom_counts, 16),
        top_negative_atoms: atom_count_reports(negative_atom_counts, 16),
        top_positive_enriched_atoms,
        top_negative_enriched_atoms,
        max_positive_delta_milli,
        max_negative_delta_milli,
        label_balance_milli,
        compile_allowed: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        recommended_next_action,
        boundary: "diagnostic only: reads phase atom traces and verifier labels to explain separability; does not compile .nwpc, promote, serve, local_accept, claim money savings, inspect raw response text, use target/proof authority, or revive legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_atom_action_family_separability_audit_v1:");
    println!("  action_family_filter: {}", report.action_family_filter);
    println!("  report_path: {}", report_path.display());
    println!("  matched_rows: {}", report.matched_rows);
    println!("  positive_rows: {}", report.positive_rows);
    println!("  negative_rows: {}", report.negative_rows);
    println!(
        "  max_positive_delta_milli: {}",
        report.max_positive_delta_milli
    );
    println!(
        "  max_negative_delta_milli: {}",
        report.max_negative_delta_milli
    );
    println!(
        "  recommended_next_action: {}",
        report.recommended_next_action
    );
    Ok(())
}

pub(crate) fn run_phase_stream_codex_session_run_check_verifier_trace_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSION_RUN_CHECK_VERIFIER_REPORT));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSION_RUN_CHECK_VERIFIER_JSONL));
    let sessions_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSIONS_DIR));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CODEX_SESSION_RUN_CHECK_MAX_EVENTS);
    if max_events == 0 {
        return Err("max_events must be > 0".to_owned());
    }

    let mut session_files = Vec::new();
    collect_session_jsonl_files(&sessions_dir, &mut session_files)?;
    session_files.sort_by(|left, right| {
        right
            .modified_ms
            .cmp(&left.modified_ms)
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut output = String::new();
    let mut session_files_scanned = 0usize;
    let mut json_rows_seen = 0usize;
    let mut exec_command_end_events_seen = 0usize;
    let mut run_check_events_seen = 0usize;
    let mut pass_rows = 0usize;
    let mut fail_rows = 0usize;
    let mut compile_error_rows = 0usize;
    let mut runtime_panic_rows = 0usize;
    let mut unknown_failure_rows = 0usize;
    let mut rows_written = 0usize;
    let mut rows_ready_for_route_family_mining = 0usize;
    let mut rows_with_shadow_request = 0usize;
    let mut rows_ready_for_existing_shadow_scoring = 0usize;
    let mut rows_ready_for_action_family_clustering = 0usize;

    for file in &session_files {
        if rows_written >= max_events {
            break;
        }
        session_files_scanned += 1;
        if session_files_scanned == 1 || session_files_scanned.is_multiple_of(50) {
            println!(
                "codex_session_run_check_scan_progress: files_scanned={} rows_written={}",
                session_files_scanned, rows_written
            );
        }
        let text = match std::fs::read_to_string(&file.path) {
            Ok(text) => text,
            Err(_) => continue,
        };
        let mut session_id = file
            .path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown_session")
            .to_owned();
        for (line_index, line) in text.lines().enumerate() {
            if rows_written >= max_events {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(row) => row,
                Err(error) => {
                    return Err(format!(
                        "failed to parse session '{}' line {}: {error}",
                        file.path.display(),
                        line_index + 1
                    ));
                }
            };
            json_rows_seen += 1;
            if row.get("type").and_then(serde_json::Value::as_str) == Some("session_meta")
                && let Some(id) = json_string(&row, &["payload", "session_id"])
                    .or_else(|| json_string(&row, &["payload", "id"]))
            {
                session_id = id;
                continue;
            }
            let Some(payload) = row.get("payload").and_then(serde_json::Value::as_object) else {
                continue;
            };
            if payload.get("type").and_then(serde_json::Value::as_str) != Some("exec_command_end") {
                continue;
            }
            exec_command_end_events_seen += 1;
            let Some(event) = parse_session_run_check_event(
                &session_id,
                row.get("timestamp").and_then(serde_json::Value::as_str),
                &file.path,
                payload,
            ) else {
                continue;
            };
            run_check_events_seen += 1;
            match event.label {
                TestOutputLabel::Pass => pass_rows += 1,
                TestOutputLabel::Fail => fail_rows += 1,
                TestOutputLabel::CompileError => compile_error_rows += 1,
                TestOutputLabel::RuntimePanic => runtime_panic_rows += 1,
            }
            unknown_failure_rows += usize::from(event.unknown_failure);
            rows_ready_for_route_family_mining += 1;
            rows_ready_for_action_family_clustering += 1;
            let atom_row = session_run_check_event_to_phase_atom_row(&event, rows_written);
            rows_with_shadow_request += usize::from(
                atom_row
                    .get("has_shadow_request")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            );
            rows_ready_for_existing_shadow_scoring += usize::from(
                atom_row
                    .get("ready_for_existing_shadow_scoring")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            );
            output.push_str(&serde_json::to_string(&atom_row).map_err(|error| {
                format!("failed to serialize session run-check atom row: {error}")
            })?);
            output.push('\n');
            rows_written += 1;
        }
    }

    if let Some(parent) = output_trace_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create run-check verifier trace directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(&output_trace_path, output).map_err(|error| {
        format!(
            "failed to write run-check verifier trace '{}': {error}",
            output_trace_path.display()
        )
    })?;
    let report = CodexSessionRunCheckVerifierTraceReport {
        report_kind: "codex_session_run_check_verifier_trace_v1",
        mode: "session_tool_output_to_phase_atom_trace",
        sessions_dir: sessions_dir.display().to_string(),
        output_trace_path: output_trace_path.display().to_string(),
        max_events,
        session_files_seen: session_files.len(),
        session_files_scanned,
        json_rows_seen,
        exec_command_end_events_seen,
        run_check_events_seen,
        rows_written,
        pass_rows,
        fail_rows,
        compile_error_rows,
        runtime_panic_rows,
        unknown_failure_rows,
        rows_ready_for_route_family_mining,
        rows_with_shadow_request,
        rows_ready_for_existing_shadow_scoring,
        rows_ready_for_action_family_clustering,
        raw_tool_output_written: false,
        raw_request_text_written: false,
        raw_response_text_written: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "verifier-trace only: reads local Codex session exec_command_end tool outputs, writes deterministic run_check phase atoms and verifier labels, writes no raw tool output/request/response text, and does not compile .nwpc, promote, serve, local_accept, claim savings, or revive legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("codex_session_run_check_verifier_trace_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  output_trace_path: {}", output_trace_path.display());
    println!("  session_files_scanned: {}", report.session_files_scanned);
    println!(
        "  exec_command_end_events_seen: {}",
        report.exec_command_end_events_seen
    );
    println!("  run_check_events_seen: {}", report.run_check_events_seen);
    println!("  rows_written: {}", report.rows_written);
    println!("  pass_rows: {}", report.pass_rows);
    println!(
        "  negative_rows: {}",
        report.fail_rows + report.compile_error_rows + report.runtime_panic_rows
    );
    println!(
        "  rows_ready_for_route_family_mining: {}",
        report.rows_ready_for_route_family_mining
    );
    println!(
        "  rows_with_shadow_request: {}",
        report.rows_with_shadow_request
    );
    println!(
        "  rows_ready_for_existing_shadow_scoring: {}",
        report.rows_ready_for_existing_shadow_scoring
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_codex_session_tool_status_verifier_trace_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSION_TOOL_STATUS_VERIFIER_REPORT));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSION_TOOL_STATUS_VERIFIER_JSONL));
    let sessions_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSIONS_DIR));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CODEX_SESSION_TOOL_STATUS_MAX_EVENTS);
    if max_events == 0 {
        return Err("max_events must be > 0".to_owned());
    }

    let mut session_files = Vec::new();
    collect_session_jsonl_files(&sessions_dir, &mut session_files)?;
    session_files.sort_by(|left, right| {
        right
            .modified_ms
            .cmp(&left.modified_ms)
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut session_files_scanned = 0usize;
    let mut json_rows_seen = 0usize;
    let mut exec_command_end_events_seen = 0usize;
    let mut response_item_tool_call_events_seen = 0usize;
    let mut response_item_tool_output_events_seen = 0usize;
    let mut tool_status_events_seen = 0usize;
    let mut pass_rows = 0usize;
    let mut fail_rows = 0usize;
    let mut compile_error_rows = 0usize;
    let mut runtime_panic_rows = 0usize;
    let mut unknown_failure_rows = 0usize;
    let mut rows_ready_for_route_family_mining = 0usize;
    let mut rows_with_shadow_request = 0usize;
    let mut rows_ready_for_existing_shadow_scoring = 0usize;
    let mut rows_ready_for_action_family_clustering = 0usize;
    let mut collected_events = Vec::<SessionRunCheckEvent>::new();

    for file in &session_files {
        session_files_scanned += 1;
        if session_files_scanned == 1 || session_files_scanned.is_multiple_of(50) {
            println!(
                "codex_session_tool_status_scan_progress: files_scanned={} events_seen={}",
                session_files_scanned, tool_status_events_seen
            );
        }
        let text = match std::fs::read_to_string(&file.path) {
            Ok(text) => text,
            Err(_) => continue,
        };
        let mut session_id = file
            .path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown_session")
            .to_owned();
        let mut tool_call_meta_by_id = BTreeMap::<String, SessionToolCallMeta>::new();
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(row) => row,
                Err(error) => {
                    return Err(format!(
                        "failed to parse session '{}' line {}: {error}",
                        file.path.display(),
                        line_index + 1
                    ));
                }
            };
            json_rows_seen += 1;
            if row.get("type").and_then(serde_json::Value::as_str) == Some("session_meta")
                && let Some(id) = json_string(&row, &["payload", "session_id"])
                    .or_else(|| json_string(&row, &["payload", "id"]))
            {
                session_id = id;
                continue;
            }
            let Some(payload) = row.get("payload").and_then(serde_json::Value::as_object) else {
                continue;
            };
            let payload_type = payload.get("type").and_then(serde_json::Value::as_str);
            if matches!(payload_type, Some("function_call" | "custom_tool_call")) {
                response_item_tool_call_events_seen += 1;
                if let Some((call_id, meta)) = parse_session_tool_call_meta(payload) {
                    tool_call_meta_by_id.insert(call_id, meta);
                }
                continue;
            }
            let event = if payload_type == Some("exec_command_end") {
                exec_command_end_events_seen += 1;
                parse_session_tool_status_event(
                    &session_id,
                    row.get("timestamp").and_then(serde_json::Value::as_str),
                    &file.path,
                    payload,
                )
            } else if matches!(
                payload_type,
                Some("function_call_output" | "custom_tool_call_output")
            ) {
                response_item_tool_output_events_seen += 1;
                let call_meta = json_field_string(payload.get("call_id"))
                    .and_then(|call_id| tool_call_meta_by_id.get(&call_id));
                parse_session_tool_status_event_from_tool_output(
                    &session_id,
                    row.get("timestamp").and_then(serde_json::Value::as_str),
                    &file.path,
                    payload,
                    call_meta,
                )
            } else {
                None
            };
            let Some(event) = event else { continue };
            tool_status_events_seen += 1;
            collected_events.push(event);
        }
    }

    collected_events.sort_by(|left, right| {
        right
            .timestamp
            .cmp(&left.timestamp)
            .then_with(|| right.session_id.cmp(&left.session_id))
            .then_with(|| right.turn_id.cmp(&left.turn_id))
            .then_with(|| right.command.cmp(&left.command))
    });
    collected_events.truncate(max_events);
    collected_events.sort_by(|left, right| {
        left.timestamp
            .cmp(&right.timestamp)
            .then_with(|| left.session_id.cmp(&right.session_id))
            .then_with(|| left.turn_id.cmp(&right.turn_id))
            .then_with(|| left.command.cmp(&right.command))
    });

    let mut output = String::new();
    for (row_index, event) in collected_events.iter().enumerate() {
        match event.label {
            TestOutputLabel::Pass => pass_rows += 1,
            TestOutputLabel::Fail => fail_rows += 1,
            TestOutputLabel::CompileError => compile_error_rows += 1,
            TestOutputLabel::RuntimePanic => runtime_panic_rows += 1,
        }
        unknown_failure_rows += usize::from(event.unknown_failure);
        rows_ready_for_route_family_mining += 1;
        rows_ready_for_action_family_clustering += 1;
        let atom_row = session_tool_status_event_to_phase_atom_row(event, row_index);
        rows_with_shadow_request += usize::from(
            atom_row
                .get("has_shadow_request")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        );
        rows_ready_for_existing_shadow_scoring += usize::from(
            atom_row
                .get("ready_for_existing_shadow_scoring")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        );
        output.push_str(&serde_json::to_string(&atom_row).map_err(|error| {
            format!("failed to serialize session tool-status atom row: {error}")
        })?);
        output.push('\n');
    }
    let rows_written = collected_events.len();

    if let Some(parent) = output_trace_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create tool-status verifier trace directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(&output_trace_path, output).map_err(|error| {
        format!(
            "failed to write tool-status verifier trace '{}': {error}",
            output_trace_path.display()
        )
    })?;
    let report = CodexSessionToolStatusVerifierTraceReport {
        report_kind: "codex_session_tool_status_verifier_trace_v1",
        mode: "session_tool_status_to_phase_atom_trace",
        sessions_dir: sessions_dir.display().to_string(),
        output_trace_path: output_trace_path.display().to_string(),
        max_events,
        selection_policy: "latest_events_by_event_timestamp_then_chronological_output",
        session_files_seen: session_files.len(),
        session_files_scanned,
        json_rows_seen,
        exec_command_end_events_seen,
        response_item_tool_call_events_seen,
        response_item_tool_output_events_seen,
        tool_status_events_seen,
        rows_written,
        pass_rows,
        fail_rows,
        compile_error_rows,
        runtime_panic_rows,
        unknown_failure_rows,
        rows_ready_for_route_family_mining,
        rows_with_shadow_request,
        rows_ready_for_existing_shadow_scoring,
        rows_ready_for_action_family_clustering,
        raw_tool_output_written: false,
        raw_request_text_written: false,
        raw_response_text_written: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "verifier-trace only: reads local Codex session exec_command_end plus response_item tool-call/output status metadata, writes deterministic tool_status phase atoms and verifier labels, writes no raw tool output/request/response text, and does not compile .nwpc, promote, serve, local_accept, claim savings, or revive legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("codex_session_tool_status_verifier_trace_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  output_trace_path: {}", output_trace_path.display());
    println!("  session_files_scanned: {}", report.session_files_scanned);
    println!(
        "  exec_command_end_events_seen: {}",
        report.exec_command_end_events_seen
    );
    println!(
        "  response_item_tool_call_events_seen: {}",
        report.response_item_tool_call_events_seen
    );
    println!(
        "  response_item_tool_output_events_seen: {}",
        report.response_item_tool_output_events_seen
    );
    println!(
        "  tool_status_events_seen: {}",
        report.tool_status_events_seen
    );
    println!("  rows_written: {}", report.rows_written);
    println!("  pass_rows: {}", report.pass_rows);
    println!(
        "  negative_rows: {}",
        report.fail_rows + report.compile_error_rows + report.runtime_panic_rows
    );
    println!(
        "  rows_ready_for_route_family_mining: {}",
        report.rows_ready_for_route_family_mining
    );
    println!(
        "  rows_with_shadow_request: {}",
        report.rows_with_shadow_request
    );
    println!(
        "  rows_ready_for_existing_shadow_scoring: {}",
        report.rows_ready_for_existing_shadow_scoring
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_codex_session_live_append_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSION_LIVE_APPEND_REPORT));
    let append_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSION_LIVE_APPEND_JSONL));
    let session_path = match args.next() {
        Some(path) => PathBuf::from(path),
        None => latest_codex_session_file(Path::new(DEFAULT_CODEX_SESSIONS_DIR))?,
    };
    let poll_ms = args
        .next()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid poll_ms value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(250);
    let max_idle_ms = args
        .next()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid max_idle_ms value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(0);
    let max_rows = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_rows value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(0);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }
    if poll_ms == 0 {
        return Err("poll_ms must be > 0".to_owned());
    }

    let session_file = std::fs::OpenOptions::new()
        .read(true)
        .open(&session_path)
        .map_err(|error| {
            format!(
                "failed to open Codex session live source '{}': {error}",
                session_path.display()
            )
        })?;
    if let Some(parent) = append_trace_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create live append trace directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    let mut append_trace = std::io::BufWriter::new(
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&append_trace_path)
            .map_err(|error| {
                format!(
                    "failed to open live append trace '{}': {error}",
                    append_trace_path.display()
                )
            })?,
    );
    let mut reader = std::io::BufReader::new(session_file);
    reader
        .seek(SeekFrom::End(0))
        .map_err(|error| format!("failed to seek Codex session live source to end: {error}"))?;

    let mut session_id = session_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown_session")
        .to_owned();
    let mut tool_call_meta_by_id = BTreeMap::<String, SessionToolCallMeta>::new();
    let mut json_rows_seen = 0usize;
    let mut session_meta_events_seen = 0usize;
    let mut function_call_events_seen = 0usize;
    let mut custom_tool_call_events_seen = 0usize;
    let mut function_call_output_events_seen = 0usize;
    let mut custom_tool_call_output_events_seen = 0usize;
    let mut exec_command_end_events_seen = 0usize;
    let mut tool_status_events_seen = 0usize;
    let mut rows_written = 0usize;
    let mut pass_rows = 0usize;
    let mut fail_rows = 0usize;
    let mut compile_error_rows = 0usize;
    let mut runtime_panic_rows = 0usize;
    let mut unknown_failure_rows = 0usize;
    let mut skipped_no_payload = 0usize;
    let mut skipped_unhandled_payload = 0usize;
    let mut skipped_unlabeled_event = 0usize;
    let mut idle_elapsed_ms = 0u64;
    let mut line = String::new();
    let mut last_heartbeat = Instant::now();

    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("failed to read Codex session live source: {error}"))?;
        if bytes == 0 {
            reader.seek(SeekFrom::Current(0)).map_err(|error| {
                format!("failed to refresh Codex session live source tail: {error}")
            })?;
            if max_rows > 0 && rows_written >= max_rows {
                break;
            }
            if max_idle_ms > 0 && idle_elapsed_ms >= max_idle_ms {
                break;
            }
            if last_heartbeat.elapsed()
                >= Duration::from_secs(DEFAULT_CODEX_SESSION_LIVE_APPEND_HEARTBEAT_SECS)
            {
                let current_offset = reader.stream_position().map_err(|error| {
                    format!("failed to read Codex session live heartbeat offset: {error}")
                })?;
                let snapshot = CodexSessionLiveAppendReport {
                    report_kind: "codex_session_live_append_v1",
                    mode: "codex_session_tail_to_live_phase_atom_append",
                    snapshot_in_progress: true,
                    session_path: session_path.display().to_string(),
                    append_trace_path: append_trace_path.display().to_string(),
                    poll_ms,
                    max_idle_ms,
                    max_rows,
                    idle_elapsed_ms,
                    start_at_end: true,
                    json_rows_seen,
                    session_meta_events_seen,
                    function_call_events_seen,
                    custom_tool_call_events_seen,
                    function_call_output_events_seen,
                    custom_tool_call_output_events_seen,
                    exec_command_end_events_seen,
                    tool_status_events_seen,
                    rows_written,
                    pass_rows,
                    fail_rows,
                    compile_error_rows,
                    runtime_panic_rows,
                    unknown_failure_rows,
                    skipped_no_payload,
                    skipped_unhandled_payload,
                    skipped_unlabeled_event,
                    last_offset: current_offset,
                    raw_tool_output_written: false,
                    raw_request_text_written: false,
                    raw_response_text_written: false,
                    local_accept_enabled: false,
                    product_runtime_changed: false,
                    serving_runtime_changed: false,
                    market_money_claim_allowed: false,
                    forbidden_flags: ForbiddenFlags {
                        target_id_used: false,
                        proof_rule_id_authority_used: false,
                        concrete_x_lookup_used: false,
                        manual_local_out_t_used: false,
                        hidden_frame_id_or_bind_x_used: false,
                        legacy_backend_used: false,
                    },
                    verdict: "CODEX_SESSION_LIVE_APPEND_RUNNING",
                    blocker: "codex_session_live_append_running".to_owned(),
                    boundary: "heartbeat snapshot only: live source adapter tails one Codex session JSONL from EOF and appends verifier-bound phase-atom rows; this snapshot does not compile .nwpc, mutate registry, promote, local_accept, claim money, or revive legacy role-binding/nwrb backend",
                };
                write_json_file(&report_path, &snapshot)?;
                println!(
                    "codex_session_live_append_heartbeat: rows_written={} json_rows_seen={} tool_status_events_seen={} idle_ms={}",
                    rows_written, json_rows_seen, tool_status_events_seen, idle_elapsed_ms
                );
                last_heartbeat = Instant::now();
            }
            std::thread::sleep(Duration::from_millis(poll_ms));
            idle_elapsed_ms = idle_elapsed_ms.saturating_add(poll_ms);
            continue;
        }
        idle_elapsed_ms = 0;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse Codex session live source '{}' row {}: {error}",
                session_path.display(),
                json_rows_seen + 1
            )
        })?;
        json_rows_seen += 1;
        if row.get("type").and_then(serde_json::Value::as_str) == Some("session_meta") {
            session_meta_events_seen += 1;
            if let Some(id) = json_string(&row, &["payload", "session_id"])
                .or_else(|| json_string(&row, &["payload", "id"]))
            {
                session_id = id;
            }
            continue;
        }
        let Some(payload) = row.get("payload").and_then(serde_json::Value::as_object) else {
            skipped_no_payload += 1;
            continue;
        };
        let payload_type = payload.get("type").and_then(serde_json::Value::as_str);
        let event = match payload_type {
            Some("function_call") => {
                function_call_events_seen += 1;
                if let Some((call_id, meta)) = parse_session_tool_call_meta(payload) {
                    tool_call_meta_by_id.insert(call_id, meta);
                }
                None
            }
            Some("custom_tool_call") => {
                custom_tool_call_events_seen += 1;
                if let Some((call_id, meta)) = parse_session_tool_call_meta(payload) {
                    tool_call_meta_by_id.insert(call_id, meta);
                }
                None
            }
            Some("function_call_output") => {
                function_call_output_events_seen += 1;
                let call_meta = json_field_string(payload.get("call_id"))
                    .and_then(|call_id| tool_call_meta_by_id.get(&call_id));
                parse_session_tool_status_event_from_tool_output(
                    &session_id,
                    row.get("timestamp").and_then(serde_json::Value::as_str),
                    &session_path,
                    payload,
                    call_meta,
                )
            }
            Some("custom_tool_call_output") => {
                custom_tool_call_output_events_seen += 1;
                let call_meta = json_field_string(payload.get("call_id"))
                    .and_then(|call_id| tool_call_meta_by_id.get(&call_id));
                parse_session_tool_status_event_from_tool_output(
                    &session_id,
                    row.get("timestamp").and_then(serde_json::Value::as_str),
                    &session_path,
                    payload,
                    call_meta,
                )
            }
            Some("exec_command_end") => {
                exec_command_end_events_seen += 1;
                parse_session_tool_status_event(
                    &session_id,
                    row.get("timestamp").and_then(serde_json::Value::as_str),
                    &session_path,
                    payload,
                )
            }
            Some(_) => {
                skipped_unhandled_payload += 1;
                None
            }
            None => {
                skipped_unhandled_payload += 1;
                None
            }
        };
        let Some(event) = event else {
            continue;
        };
        tool_status_events_seen += 1;
        match event.label {
            TestOutputLabel::Pass => pass_rows += 1,
            TestOutputLabel::Fail => fail_rows += 1,
            TestOutputLabel::CompileError => compile_error_rows += 1,
            TestOutputLabel::RuntimePanic => runtime_panic_rows += 1,
        }
        unknown_failure_rows += usize::from(event.unknown_failure);
        let atom_row = session_tool_status_event_to_phase_atom_row(&event, rows_written);
        if atom_row
            .get("verified_safe_accept")
            .and_then(serde_json::Value::as_bool)
            .is_none()
        {
            skipped_unlabeled_event += 1;
            continue;
        }
        serde_json::to_writer(&mut append_trace, &atom_row)
            .map_err(|error| format!("failed to write live append phase atom row: {error}"))?;
        append_trace
            .write_all(b"\n")
            .map_err(|error| format!("failed live append newline: {error}"))?;
        append_trace
            .flush()
            .map_err(|error| format!("failed live append flush: {error}"))?;
        rows_written += 1;
        println!(
            "codex_session_live_append_row: rows_written={} tool_status_events_seen={} label={}",
            rows_written,
            tool_status_events_seen,
            event.label.as_str()
        );
        if max_rows > 0 && rows_written >= max_rows {
            break;
        }
    }

    let last_offset = reader
        .stream_position()
        .map_err(|error| format!("failed to read final Codex session offset: {error}"))?;
    let blocker = if rows_written > 0 {
        "none".to_owned()
    } else if json_rows_seen == 0 {
        "codex_session_live_append_no_new_rows".to_owned()
    } else if tool_status_events_seen == 0 {
        "codex_session_live_append_no_tool_status_events".to_owned()
    } else {
        "codex_session_live_append_no_rows_written".to_owned()
    };
    let report = CodexSessionLiveAppendReport {
        report_kind: "codex_session_live_append_v1",
        mode: "codex_session_tail_to_live_phase_atom_append",
        snapshot_in_progress: false,
        session_path: session_path.display().to_string(),
        append_trace_path: append_trace_path.display().to_string(),
        poll_ms,
        max_idle_ms,
        max_rows,
        idle_elapsed_ms,
        start_at_end: true,
        json_rows_seen,
        session_meta_events_seen,
        function_call_events_seen,
        custom_tool_call_events_seen,
        function_call_output_events_seen,
        custom_tool_call_output_events_seen,
        exec_command_end_events_seen,
        tool_status_events_seen,
        rows_written,
        pass_rows,
        fail_rows,
        compile_error_rows,
        runtime_panic_rows,
        unknown_failure_rows,
        skipped_no_payload,
        skipped_unhandled_payload,
        skipped_unlabeled_event,
        last_offset,
        raw_tool_output_written: false,
        raw_request_text_written: false,
        raw_response_text_written: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        verdict: if rows_written > 0 {
            "CODEX_SESSION_LIVE_APPEND_PASS"
        } else {
            "CODEX_SESSION_LIVE_APPEND_WATCH"
        },
        blocker,
        boundary: "live source adapter only: tails one Codex session JSONL from EOF, converts new tool status events into verifier-bound phase-atom rows, appends and flushes them to the live phase-atom source, writes no raw tool output/request/response text, does not compile .nwpc, mutate registry, promote, local_accept, claim money, or revive legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("codex_session_live_append_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  session_path: {}", report.session_path);
    println!("  append_trace_path: {}", report.append_trace_path);
    println!("  json_rows_seen: {}", report.json_rows_seen);
    println!(
        "  tool_status_events_seen: {}",
        report.tool_status_events_seen
    );
    println!("  rows_written: {}", report.rows_written);
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_codex_sessions_live_append_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("target/nando-wave/streaming/codex-sessions-live-append-v1.report.json")
    });
    let append_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSION_LIVE_APPEND_JSONL));
    let sessions_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSIONS_DIR));
    let poll_ms = args
        .next()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid poll_ms value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(250);
    let max_idle_ms = args
        .next()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid max_idle_ms value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(0);
    let max_rows = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_rows value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(0);
    let max_recent_files = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_recent_files value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(16);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }
    if poll_ms == 0 {
        return Err("poll_ms must be > 0".to_owned());
    }
    if max_recent_files == 0 {
        return Err("max_recent_files must be > 0".to_owned());
    }
    if let Some(parent) = append_trace_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create multi-session live append trace directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    let mut append_trace = std::io::BufWriter::new(
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&append_trace_path)
            .map_err(|error| {
                format!(
                    "failed to open multi-session live append trace '{}': {error}",
                    append_trace_path.display()
                )
            })?,
    );

    let mut states = BTreeMap::<PathBuf, CodexSessionLiveTailState>::new();
    let mut session_files_seen;
    let mut json_rows_seen = 0usize;
    let mut session_meta_events_seen = 0usize;
    let mut function_call_events_seen = 0usize;
    let mut custom_tool_call_events_seen = 0usize;
    let mut function_call_output_events_seen = 0usize;
    let mut custom_tool_call_output_events_seen = 0usize;
    let mut exec_command_end_events_seen = 0usize;
    let mut tool_status_events_seen = 0usize;
    let mut rows_written = 0usize;
    let mut pass_rows = 0usize;
    let mut fail_rows = 0usize;
    let mut compile_error_rows = 0usize;
    let mut runtime_panic_rows = 0usize;
    let mut unknown_failure_rows = 0usize;
    let mut skipped_no_payload = 0usize;
    let mut skipped_unhandled_payload = 0usize;
    let mut skipped_unlabeled_event = 0usize;
    let mut idle_elapsed_ms = 0u64;
    let mut last_heartbeat = Instant::now();

    loop {
        let mut session_files = Vec::new();
        collect_session_jsonl_files(&sessions_dir, &mut session_files)?;
        session_files.sort_by(|left, right| {
            right
                .modified_ms
                .cmp(&left.modified_ms)
                .then_with(|| left.path.cmp(&right.path))
        });
        session_files_seen = session_files.len();
        session_files.truncate(max_recent_files);
        let mut tick_rows_written = 0usize;

        for entry in &session_files {
            if max_rows > 0 && rows_written >= max_rows {
                break;
            }
            let metadata = match std::fs::metadata(&entry.path) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            let file_len = metadata.len();
            let active_session_files_for_log = states.len();
            let state =
                states
                    .entry(entry.path.clone())
                    .or_insert_with(|| CodexSessionLiveTailState {
                        offset: file_len,
                        session_id: entry
                            .path
                            .file_stem()
                            .and_then(|value| value.to_str())
                            .unwrap_or("unknown_session")
                            .to_owned(),
                        tool_call_meta_by_id: BTreeMap::new(),
                    });
            if file_len <= state.offset {
                continue;
            }
            let file = std::fs::OpenOptions::new()
                .read(true)
                .open(&entry.path)
                .map_err(|error| {
                    format!(
                        "failed to open multi-session Codex source '{}': {error}",
                        entry.path.display()
                    )
                })?;
            let mut reader = std::io::BufReader::new(file);
            reader
                .seek(SeekFrom::Start(state.offset))
                .map_err(|error| {
                    format!(
                        "failed to seek multi-session Codex source '{}': {error}",
                        entry.path.display()
                    )
                })?;
            let mut line_bytes = Vec::new();
            loop {
                line_bytes.clear();
                let bytes = reader.read_until(b'\n', &mut line_bytes).map_err(|error| {
                    format!(
                        "failed to read multi-session Codex source '{}': {error}",
                        entry.path.display()
                    )
                })?;
                if bytes == 0 {
                    break;
                }
                let line = String::from_utf8_lossy(&line_bytes);
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                    format!(
                        "failed to parse multi-session Codex source '{}' row {}: {error}",
                        entry.path.display(),
                        json_rows_seen + 1
                    )
                })?;
                json_rows_seen += 1;
                if row.get("type").and_then(serde_json::Value::as_str) == Some("session_meta") {
                    session_meta_events_seen += 1;
                    if let Some(id) = json_string(&row, &["payload", "session_id"])
                        .or_else(|| json_string(&row, &["payload", "id"]))
                    {
                        state.session_id = id;
                    }
                    continue;
                }
                let Some(payload) = row.get("payload").and_then(serde_json::Value::as_object)
                else {
                    skipped_no_payload += 1;
                    continue;
                };
                let payload_type = payload.get("type").and_then(serde_json::Value::as_str);
                let event = match payload_type {
                    Some("function_call") => {
                        function_call_events_seen += 1;
                        if let Some((call_id, meta)) = parse_session_tool_call_meta(payload) {
                            state.tool_call_meta_by_id.insert(call_id, meta);
                        }
                        None
                    }
                    Some("custom_tool_call") => {
                        custom_tool_call_events_seen += 1;
                        if let Some((call_id, meta)) = parse_session_tool_call_meta(payload) {
                            state.tool_call_meta_by_id.insert(call_id, meta);
                        }
                        None
                    }
                    Some("function_call_output") => {
                        function_call_output_events_seen += 1;
                        let call_meta = json_field_string(payload.get("call_id"))
                            .and_then(|call_id| state.tool_call_meta_by_id.get(&call_id));
                        parse_session_tool_status_event_from_tool_output(
                            &state.session_id,
                            row.get("timestamp").and_then(serde_json::Value::as_str),
                            &entry.path,
                            payload,
                            call_meta,
                        )
                    }
                    Some("custom_tool_call_output") => {
                        custom_tool_call_output_events_seen += 1;
                        let call_meta = json_field_string(payload.get("call_id"))
                            .and_then(|call_id| state.tool_call_meta_by_id.get(&call_id));
                        parse_session_tool_status_event_from_tool_output(
                            &state.session_id,
                            row.get("timestamp").and_then(serde_json::Value::as_str),
                            &entry.path,
                            payload,
                            call_meta,
                        )
                    }
                    Some("exec_command_end") => {
                        exec_command_end_events_seen += 1;
                        parse_session_tool_status_event(
                            &state.session_id,
                            row.get("timestamp").and_then(serde_json::Value::as_str),
                            &entry.path,
                            payload,
                        )
                    }
                    Some(_) | None => {
                        skipped_unhandled_payload += 1;
                        None
                    }
                };
                let Some(event) = event else {
                    continue;
                };
                tool_status_events_seen += 1;
                match event.label {
                    TestOutputLabel::Pass => pass_rows += 1,
                    TestOutputLabel::Fail => fail_rows += 1,
                    TestOutputLabel::CompileError => compile_error_rows += 1,
                    TestOutputLabel::RuntimePanic => runtime_panic_rows += 1,
                }
                unknown_failure_rows += usize::from(event.unknown_failure);
                let atom_row = session_tool_status_event_to_phase_atom_row(&event, rows_written);
                if atom_row
                    .get("verified_safe_accept")
                    .and_then(serde_json::Value::as_bool)
                    .is_none()
                {
                    skipped_unlabeled_event += 1;
                    continue;
                }
                serde_json::to_writer(&mut append_trace, &atom_row).map_err(|error| {
                    format!("failed to write multi-session live append phase atom row: {error}")
                })?;
                append_trace.write_all(b"\n").map_err(|error| {
                    format!("failed multi-session live append newline: {error}")
                })?;
                rows_written += 1;
                tick_rows_written += 1;
                println!(
                    "codex_sessions_live_append_row: rows_written={} session_files={} tool_status_events_seen={} label={}",
                    rows_written,
                    active_session_files_for_log,
                    tool_status_events_seen,
                    event.label.as_str()
                );
                if max_rows > 0 && rows_written >= max_rows {
                    break;
                }
            }
            state.offset = reader.stream_position().map_err(|error| {
                format!(
                    "failed to read multi-session Codex source offset '{}': {error}",
                    entry.path.display()
                )
            })?;
        }

        append_trace
            .flush()
            .map_err(|error| format!("failed multi-session live append flush: {error}"))?;
        if max_rows > 0 && rows_written >= max_rows {
            break;
        }
        if tick_rows_written > 0 {
            idle_elapsed_ms = 0;
        } else if max_idle_ms > 0 && idle_elapsed_ms >= max_idle_ms {
            break;
        }
        if last_heartbeat.elapsed()
            >= Duration::from_secs(DEFAULT_CODEX_SESSION_LIVE_APPEND_HEARTBEAT_SECS)
        {
            let snapshot = CodexSessionsLiveAppendReport {
                report_kind: "codex_sessions_live_append_v1",
                mode: "codex_sessions_dir_tail_to_live_phase_atom_append",
                snapshot_in_progress: true,
                sessions_dir: sessions_dir.display().to_string(),
                append_trace_path: append_trace_path.display().to_string(),
                poll_ms,
                max_idle_ms,
                max_rows,
                max_recent_files,
                idle_elapsed_ms,
                start_at_end: true,
                session_files_seen,
                active_session_files: states.len(),
                json_rows_seen,
                session_meta_events_seen,
                function_call_events_seen,
                custom_tool_call_events_seen,
                function_call_output_events_seen,
                custom_tool_call_output_events_seen,
                exec_command_end_events_seen,
                tool_status_events_seen,
                rows_written,
                pass_rows,
                fail_rows,
                compile_error_rows,
                runtime_panic_rows,
                unknown_failure_rows,
                skipped_no_payload,
                skipped_unhandled_payload,
                skipped_unlabeled_event,
                raw_tool_output_written: false,
                raw_request_text_written: false,
                raw_response_text_written: false,
                local_accept_enabled: false,
                product_runtime_changed: false,
                serving_runtime_changed: false,
                market_money_claim_allowed: false,
                forbidden_flags: ForbiddenFlags {
                    target_id_used: false,
                    proof_rule_id_authority_used: false,
                    concrete_x_lookup_used: false,
                    manual_local_out_t_used: false,
                    hidden_frame_id_or_bind_x_used: false,
                    legacy_backend_used: false,
                },
                verdict: "CODEX_SESSIONS_LIVE_APPEND_RUNNING",
                blocker: "codex_sessions_live_append_running".to_owned(),
                boundary: "heartbeat snapshot only: source adapter tails recent Codex session JSONL files from EOF and appends verifier-bound phase-atom rows; this snapshot does not compile .nwpc, mutate registry, promote, local_accept, claim money, or revive legacy role-binding/nwrb backend",
            };
            write_json_file(&report_path, &snapshot)?;
            println!(
                "codex_sessions_live_append_heartbeat: rows_written={} active_session_files={} json_rows_seen={} tool_status_events_seen={} idle_ms={}",
                rows_written,
                states.len(),
                json_rows_seen,
                tool_status_events_seen,
                idle_elapsed_ms
            );
            last_heartbeat = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(poll_ms));
        idle_elapsed_ms = idle_elapsed_ms.saturating_add(poll_ms);
    }

    append_trace
        .flush()
        .map_err(|error| format!("failed multi-session final live append flush: {error}"))?;
    let blocker = if rows_written > 0 {
        "none".to_owned()
    } else if session_files_seen == 0 {
        "codex_sessions_live_append_no_session_files".to_owned()
    } else if json_rows_seen == 0 {
        "codex_sessions_live_append_no_new_rows".to_owned()
    } else if tool_status_events_seen == 0 {
        "codex_sessions_live_append_no_tool_status_events".to_owned()
    } else {
        "codex_sessions_live_append_no_rows_written".to_owned()
    };
    let report = CodexSessionsLiveAppendReport {
        report_kind: "codex_sessions_live_append_v1",
        mode: "codex_sessions_dir_tail_to_live_phase_atom_append",
        snapshot_in_progress: false,
        sessions_dir: sessions_dir.display().to_string(),
        append_trace_path: append_trace_path.display().to_string(),
        poll_ms,
        max_idle_ms,
        max_rows,
        max_recent_files,
        idle_elapsed_ms,
        start_at_end: true,
        session_files_seen,
        active_session_files: states.len(),
        json_rows_seen,
        session_meta_events_seen,
        function_call_events_seen,
        custom_tool_call_events_seen,
        function_call_output_events_seen,
        custom_tool_call_output_events_seen,
        exec_command_end_events_seen,
        tool_status_events_seen,
        rows_written,
        pass_rows,
        fail_rows,
        compile_error_rows,
        runtime_panic_rows,
        unknown_failure_rows,
        skipped_no_payload,
        skipped_unhandled_payload,
        skipped_unlabeled_event,
        raw_tool_output_written: false,
        raw_request_text_written: false,
        raw_response_text_written: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        verdict: if rows_written > 0 {
            "CODEX_SESSIONS_LIVE_APPEND_PASS"
        } else {
            "CODEX_SESSIONS_LIVE_APPEND_WATCH"
        },
        blocker,
        boundary: "live source adapter only: tails recent Codex session JSONL files from EOF, converts new tool status events into verifier-bound phase-atom rows, appends and flushes them to the live phase-atom source, writes no raw tool output/request/response text, does not compile .nwpc, mutate registry, promote, local_accept, claim money, or revive legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("codex_sessions_live_append_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  sessions_dir: {}", report.sessions_dir);
    println!("  append_trace_path: {}", report.append_trace_path);
    println!("  active_session_files: {}", report.active_session_files);
    println!("  json_rows_seen: {}", report.json_rows_seen);
    println!(
        "  tool_status_events_seen: {}",
        report.tool_status_events_seen
    );
    println!("  rows_written: {}", report.rows_written);
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    println!("  verdict: {}", report.verdict);
    println!("  blocker: {}", report.blocker);
    Ok(())
}

pub(crate) fn run_phase_stream_codex_session_planning_verifier_trace_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSION_PLANNING_VERIFIER_REPORT));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSION_PLANNING_VERIFIER_JSONL));
    let sessions_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSIONS_DIR));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CODEX_SESSION_PLANNING_MAX_EVENTS);
    if max_events == 0 {
        return Err("max_events must be > 0".to_owned());
    }

    let mut session_files = Vec::new();
    collect_session_jsonl_files(&sessions_dir, &mut session_files)?;
    session_files.sort_by(|left, right| {
        right
            .modified_ms
            .cmp(&left.modified_ms)
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut session_files_scanned = 0usize;
    let mut json_rows_seen = 0usize;
    let mut update_plan_call_events_seen = 0usize;
    let mut update_plan_output_events_seen = 0usize;
    let mut planning_events_seen = 0usize;
    let mut collected_events = Vec::<SessionPlanningEvent>::new();

    for file in &session_files {
        session_files_scanned += 1;
        if session_files_scanned == 1 || session_files_scanned.is_multiple_of(50) {
            println!(
                "codex_session_planning_scan_progress: files_scanned={} events_seen={}",
                session_files_scanned, planning_events_seen
            );
        }
        let text = match std::fs::read_to_string(&file.path) {
            Ok(text) => text,
            Err(_) => continue,
        };
        let mut session_id = file
            .path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown_session")
            .to_owned();
        let mut planning_call_meta_by_id = BTreeMap::<String, SessionPlanningCallMeta>::new();
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(row) => row,
                Err(error) => {
                    return Err(format!(
                        "failed to parse session '{}' line {}: {error}",
                        file.path.display(),
                        line_index + 1
                    ));
                }
            };
            json_rows_seen += 1;
            if row.get("type").and_then(serde_json::Value::as_str) == Some("session_meta")
                && let Some(id) = json_string(&row, &["payload", "session_id"])
                    .or_else(|| json_string(&row, &["payload", "id"]))
            {
                session_id = id;
                continue;
            }
            let Some(payload) = row.get("payload").and_then(serde_json::Value::as_object) else {
                continue;
            };
            let payload_type = payload.get("type").and_then(serde_json::Value::as_str);
            if payload_type == Some("function_call") {
                if let Some((call_id, meta)) = parse_session_planning_call_meta(payload) {
                    update_plan_call_events_seen += 1;
                    planning_call_meta_by_id.insert(call_id, meta);
                }
                continue;
            }
            if payload_type != Some("function_call_output") {
                continue;
            }
            let Some(call_id) = json_field_string(payload.get("call_id")) else {
                continue;
            };
            let Some(call_meta) = planning_call_meta_by_id.get(&call_id) else {
                continue;
            };
            update_plan_output_events_seen += 1;
            let event = parse_session_planning_event_from_tool_output(
                &session_id,
                row.get("timestamp").and_then(serde_json::Value::as_str),
                &file.path,
                payload,
                call_meta,
            );
            planning_events_seen += 1;
            collected_events.push(event);
        }
    }

    collected_events.sort_by(|left, right| {
        right
            .timestamp
            .cmp(&left.timestamp)
            .then_with(|| right.session_id.cmp(&left.session_id))
            .then_with(|| right.turn_id.cmp(&left.turn_id))
            .then_with(|| right.arguments.cmp(&left.arguments))
    });
    collected_events.truncate(max_events);
    collected_events.sort_by(|left, right| {
        left.timestamp
            .cmp(&right.timestamp)
            .then_with(|| left.session_id.cmp(&right.session_id))
            .then_with(|| left.turn_id.cmp(&right.turn_id))
            .then_with(|| left.arguments.cmp(&right.arguments))
    });

    let mut output = String::new();
    let mut success_rows = 0usize;
    let mut failure_rows = 0usize;
    let mut invalid_plan_rows = 0usize;
    let mut rows_ready_for_route_family_mining = 0usize;
    let mut rows_with_shadow_request = 0usize;
    let mut rows_ready_for_existing_shadow_scoring = 0usize;
    let mut rows_ready_for_action_family_clustering = 0usize;
    for (row_index, event) in collected_events.iter().enumerate() {
        success_rows += usize::from(event.verified_safe_accept);
        failure_rows += usize::from(!event.verified_safe_accept);
        invalid_plan_rows += usize::from(!event.plan_shape.valid_schema);
        rows_ready_for_route_family_mining += 1;
        rows_with_shadow_request += 1;
        rows_ready_for_existing_shadow_scoring += 1;
        rows_ready_for_action_family_clustering += 1;
        let atom_row = session_planning_event_to_phase_atom_row(event, row_index);
        output.push_str(
            &serde_json::to_string(&atom_row).map_err(|error| {
                format!("failed to serialize session planning atom row: {error}")
            })?,
        );
        output.push('\n');
    }
    let rows_written = collected_events.len();

    if let Some(parent) = output_trace_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create planning verifier trace directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(&output_trace_path, output).map_err(|error| {
        format!(
            "failed to write planning verifier trace '{}': {error}",
            output_trace_path.display()
        )
    })?;
    let report = CodexSessionPlanningVerifierTraceReport {
        report_kind: "codex_session_planning_verifier_trace_v1",
        mode: "session_update_plan_to_phase_atom_trace",
        sessions_dir: sessions_dir.display().to_string(),
        output_trace_path: output_trace_path.display().to_string(),
        max_events,
        selection_policy: "latest_update_plan_tool_outputs_then_chronological_output",
        session_files_seen: session_files.len(),
        session_files_scanned,
        json_rows_seen,
        update_plan_call_events_seen,
        update_plan_output_events_seen,
        planning_events_seen,
        rows_written,
        success_rows,
        failure_rows,
        invalid_plan_rows,
        rows_ready_for_route_family_mining,
        rows_with_shadow_request,
        rows_ready_for_existing_shadow_scoring,
        rows_ready_for_action_family_clustering,
        raw_plan_text_written: false,
        raw_request_text_written: false,
        raw_response_text_written: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "verifier-trace only: reads local Codex session update_plan tool-call/output metadata, writes structural planning phase atoms and tool-success verifier labels, writes no raw plan/request/response text, and does not compile .nwpc, promote, serve, local_accept, claim savings, or revive legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("codex_session_planning_verifier_trace_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  output_trace_path: {}", report.output_trace_path);
    println!("  session_files_scanned: {}", report.session_files_scanned);
    println!(
        "  update_plan_call_events_seen: {}",
        report.update_plan_call_events_seen
    );
    println!(
        "  update_plan_output_events_seen: {}",
        report.update_plan_output_events_seen
    );
    println!("  rows_written: {}", report.rows_written);
    println!("  success_rows: {}", report.success_rows);
    println!("  failure_rows: {}", report.failure_rows);
    println!("  invalid_plan_rows: {}", report.invalid_plan_rows);
    println!(
        "  rows_ready_for_route_family_mining: {}",
        report.rows_ready_for_route_family_mining
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_agent_continue_active_turn_state_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    agent_continue::run_phase_stream_agent_continue_active_turn_state_v1(args)
}

pub(crate) fn run_phase_stream_agent_continue_command_result_followup_pack_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    agent_continue::run_phase_stream_agent_continue_command_result_followup_pack_v1(args)
}

pub(crate) fn run_phase_stream_auto_subcenter_discovery_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    auto_subcenter::run_phase_stream_auto_subcenter_discovery_v1(args)
}

pub(crate) fn run_phase_stream_agent_continue_subroute_scoreboard_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    agent_continue::run_phase_stream_agent_continue_subroute_scoreboard_v1(args)
}

pub(crate) fn run_phase_stream_phase_atom_run_check_discovery_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_RUN_CHECK_DISCOVERY_REPORT));
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_RUN_CHECK_DISCOVERY_PACKAGE));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CELLS);
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin threshold '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(
                DEFAULT_CODEX_SESSION_RUN_CHECK_VERIFIER_JSONL,
            )]
        } else {
            rest
        }
    };

    let mut total_rows = 0usize;
    let mut events = Vec::new();
    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read run-check phase atom trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse run-check phase atom trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            total_rows += 1;
            if let Some(event) = parse_phase_atom_binary_event(&row, events.len()) {
                events.push(event);
            }
        }
    }

    let positive_pass_events = events
        .iter()
        .filter(|event| event.verified_safe_accept)
        .count();
    let negative_events = events.len().saturating_sub(positive_pass_events);
    if positive_pass_events == 0 || negative_events == 0 {
        return Err(format!(
            "run-check discovery needs positive and negative verifier events, got pass={} negative={}",
            positive_pass_events, negative_events
        ));
    }
    let (train_indices, heldout_indices) = phase_atom_binary_train_heldout_indices(&events);
    if train_indices.is_empty() || heldout_indices.is_empty() {
        return Err("run-check discovery needs non-empty train and heldout splits".to_owned());
    }

    let mut compiler = PhaseCenterCompiler::new(cells, 2)
        .map_err(|error| format!("phase atom run-check compiler error: {error:?}"))?;
    for &event_index in &train_indices {
        let event = &events[event_index];
        let program_index = usize::from(!event.verified_safe_accept);
        let positive_vec = phase_atom_binary_event_vector(event, event.verified_safe_accept, cells);
        let negative_vec =
            phase_atom_binary_event_vector(event, !event.verified_safe_accept, cells);
        compiler
            .add_positive_vector(program_index, &positive_vec)
            .map_err(|error| format!("run-check positive update error: {error:?}"))?;
        compiler
            .add_negative_vector(program_index, &negative_vec)
            .map_err(|error| format!("run-check negative update error: {error:?}"))?;
    }
    let reference_runtime = compiler
        .compile()
        .map_err(|error| format!("run-check phase-center compile error: {error:?}"))?;
    let package_bytes = reference_runtime
        .to_bytes()
        .map_err(|error| format!("run-check package serialization error: {error:?}"))?;
    write_binary_file(&package_path, &package_bytes)?;
    let read_package = std::fs::read(&package_path).map_err(|error| {
        format!(
            "failed to read run-check package '{}': {error}",
            package_path.display()
        )
    })?;
    if read_package != package_bytes {
        return Err(format!(
            "run-check package '{}' readback mismatch",
            package_path.display()
        ));
    }
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&read_package)
        .map_err(|error| format!("run-check package inspect error: {error:?}"))?;
    let offload_runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &read_package,
        PhaseCenterOffloadPolicy::new(margin_threshold_micro)
            .map_err(|error| format!("invalid run-check policy: {error:?}"))?,
    )
    .map_err(|error| format!("run-check package load error: {error:?}"))?;

    let exact_cache_flags = exact_cache_hit_flags_phase_atom_binary(&events);
    let mut margins = Vec::new();
    let mut correct_rows = 0usize;
    let mut wrong_wins = 0usize;
    let mut false_accepts = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut heldout_local_operator_calls = 0usize;
    let mut heldout_fallback_calls = 0usize;
    let mut exact_cache_hits_in_heldout = 0usize;
    let mut unique_cpu_accepts_over_exact_cache = 0usize;
    let mut nando_cpu_tokens_saved = 0usize;
    let mut nando_cpu_cost_saved_microusd = 0u64;
    let mut unique_accepts = Vec::new();
    for &event_index in &heldout_indices {
        let event = &events[event_index];
        let program_index = usize::from(!event.verified_safe_accept);
        let correct_vec = phase_atom_binary_event_vector(event, event.verified_safe_accept, cells);
        let wrong_vec = phase_atom_binary_event_vector(event, !event.verified_safe_accept, cells);
        let task = PhaseCenterEvalTask {
            center_index: program_index,
            correct_vec: correct_vec.clone().into_boxed_slice(),
            wrong_vec: wrong_vec.clone().into_boxed_slice(),
        };
        let reference_margin = reference_runtime
            .margin(&task)
            .map_err(|error| format!("run-check reference margin error: {error:?}"))?;
        let runtime_margin = offload_runtime
            .runtime()
            .margin(&task)
            .map_err(|error| format!("run-check runtime margin error: {error:?}"))?;
        let reference_micro = margin_to_micro(reference_margin)?;
        let runtime_micro = margin_to_micro(runtime_margin)?;
        runtime_margin_parity_mismatches += usize::from(reference_micro != runtime_micro);
        margins.push(runtime_micro);
        correct_rows += usize::from(runtime_micro > 0);
        wrong_wins += usize::from(runtime_micro <= 0);
        let decision = offload_runtime
            .offload_decision(&task)
            .map_err(|error| format!("run-check offload decision error: {error:?}"))?;
        if decision.action == nando_core::PhaseCenterOffloadAction::LocalOperator {
            heldout_local_operator_calls += 1;
            false_accepts += usize::from(runtime_micro <= 0);
            if !exact_cache_flags[event_index] {
                unique_cpu_accepts_over_exact_cache += 1;
                nando_cpu_tokens_saved =
                    nando_cpu_tokens_saved.saturating_add(event.token_cost.total_tokens);
                nando_cpu_cost_saved_microusd = nando_cpu_cost_saved_microusd
                    .saturating_add(event.token_cost.total_cost_microusd);
                unique_accepts.push(GenericAcceptedEventReport {
                    request_fingerprint: format!("phase_atom_run_check:{}", event.exact_cache_key),
                    total_tokens: event.token_cost.total_tokens,
                    total_cost_microusd: event.token_cost.total_cost_microusd,
                    token_evidence_missing: event.token_cost.token_evidence_missing,
                    cost_evidence_missing: event.token_cost.cost_evidence_missing,
                });
            }
        } else {
            heldout_fallback_calls += 1;
        }
        exact_cache_hits_in_heldout += usize::from(exact_cache_flags[event_index]);
    }
    margins.sort_unstable();
    let accepted_for_offline_review = !heldout_indices.is_empty()
        && wrong_wins == 0
        && false_accepts == 0
        && runtime_margin_parity_mismatches == 0
        && package_info.record_count == reference_runtime.record_count()
        && package_info.serialized_len == read_package.len();
    let rejection_reason = if accepted_for_offline_review {
        "accepted_for_offline_shadow_review".to_owned()
    } else if wrong_wins > 0 {
        "wrong_wins_detected".to_owned()
    } else if false_accepts > 0 {
        "false_accepts_detected".to_owned()
    } else if runtime_margin_parity_mismatches > 0 {
        "runtime_margin_parity_mismatches".to_owned()
    } else {
        "package_shape_mismatch".to_owned()
    };
    let report = PhaseAtomRunCheckDiscoveryReport {
        report_kind: "phase_atom_run_check_discovery_v1",
        mode: "verifier_bound_phase_atom_to_quarantine_nwpc_candidate",
        action_family: "action_family:run_check".to_owned(),
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        package_path: package_path.display().to_string(),
        cells,
        margin_threshold_micro,
        total_rows,
        parsed_verifier_events: events.len(),
        positive_pass_events,
        negative_events,
        split_granularity: "label_stratified_every_5_by_label",
        train_heldout_time_order_ok: false,
        train_time_min: String::new(),
        train_time_max: String::new(),
        heldout_time_min: String::new(),
        heldout_time_max: String::new(),
        train_events: train_indices.len(),
        heldout_events: heldout_indices.len(),
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: read_package.len(),
        package_records: package_info.record_count,
        heldout_accuracy_milli: per_thousand(correct_rows, heldout_indices.len()),
        heldout_local_operator_calls,
        heldout_fallback_calls,
        false_accepts,
        wrong_wins,
        runtime_margin_parity_mismatches,
        min_margin_micro: margins.first().copied().unwrap_or(0),
        median_margin_micro: percentile_i64(&margins, 50),
        p10_margin_micro: percentile_i64(&margins, 10),
        exact_cache_hits_in_heldout,
        unique_cpu_accepts_over_exact_cache,
        nando_cpu_tokens_saved,
        nando_cpu_cost_saved_microusd,
        unique_accepts,
        verifier_bound: true,
        quarantine_only: true,
        promoted: false,
        serving_profile_artifact: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        accepted_for_offline_review,
        rejection_reason,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "quarantine candidate only: compiles verifier-bound run_check phase atoms into .nwpc for offline shadow review; no promotion, no serving profile, no product local_accept, no market money claim, no target/proof authority, and no legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_atom_run_check_discovery_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  package_path: {}", package_path.display());
    println!(
        "  parsed_verifier_events: {}",
        report.parsed_verifier_events
    );
    println!("  train_events: {}", report.train_events);
    println!("  heldout_events: {}", report.heldout_events);
    println!(
        "  heldout_accuracy_milli: {}",
        report.heldout_accuracy_milli
    );
    println!("  wrong_wins: {}", report.wrong_wins);
    println!("  false_accepts: {}", report.false_accepts);
    println!(
        "  local_operator_calls: {}",
        report.heldout_local_operator_calls
    );
    println!("  quarantine_only: {}", report.quarantine_only);
    println!("  promoted: {}", report.promoted);
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_run_check_time_split_discovery_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_DISCOVERY_REPORT));
    let package_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_DISCOVERY_PACKAGE)
    });
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CELLS);
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin threshold '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
    let train_permille = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid train_permille '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_TRAIN_PERMILLE);
    if !(1..=999).contains(&train_permille) {
        return Err("train_permille must be in 1..=999".to_owned());
    }
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(
                DEFAULT_CODEX_SESSION_RUN_CHECK_VERIFIER_JSONL,
            )]
        } else {
            rest
        }
    };

    let mut total_rows = 0usize;
    let mut events = Vec::new();
    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read run-check time-split phase atom trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse run-check time-split phase atom trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            total_rows += 1;
            if let Some(event) = parse_phase_atom_binary_event(&row, events.len()) {
                events.push(event);
            }
        }
    }

    let positive_pass_events = events
        .iter()
        .filter(|event| event.verified_safe_accept)
        .count();
    let negative_events = events.len().saturating_sub(positive_pass_events);
    if positive_pass_events == 0 || negative_events == 0 {
        return Err(format!(
            "run-check time-split discovery needs positive and negative verifier events, got pass={} negative={}",
            positive_pass_events, negative_events
        ));
    }
    let (train_indices, heldout_indices) =
        phase_atom_binary_time_split_indices(&events, train_permille);
    if train_indices.is_empty() || heldout_indices.is_empty() {
        return Err(
            "run-check time-split discovery needs non-empty train and heldout splits".to_owned(),
        );
    }
    if !phase_atom_binary_split_has_both_labels(&events, &train_indices) {
        return Err(
            "run-check time-split train window does not contain both pass and not_pass labels"
                .to_owned(),
        );
    }
    if !phase_atom_binary_split_has_both_labels(&events, &heldout_indices) {
        return Err(
            "run-check time-split shadow window does not contain both pass and not_pass labels"
                .to_owned(),
        );
    }

    let (train_time_min, train_time_max) = phase_atom_binary_time_range(&events, &train_indices);
    let (heldout_time_min, heldout_time_max) =
        phase_atom_binary_time_range(&events, &heldout_indices);
    let train_heldout_time_order_ok =
        phase_atom_binary_time_order_ok(&events, &train_indices, &heldout_indices);

    let mut compiler = PhaseCenterCompiler::new(cells, 2)
        .map_err(|error| format!("phase atom run-check time-split compiler error: {error:?}"))?;
    for &event_index in &train_indices {
        let event = &events[event_index];
        let program_index = usize::from(!event.verified_safe_accept);
        let positive_vec = phase_atom_binary_event_vector(event, event.verified_safe_accept, cells);
        let negative_vec =
            phase_atom_binary_event_vector(event, !event.verified_safe_accept, cells);
        compiler
            .add_positive_vector(program_index, &positive_vec)
            .map_err(|error| format!("run-check time-split positive update error: {error:?}"))?;
        compiler
            .add_negative_vector(program_index, &negative_vec)
            .map_err(|error| format!("run-check time-split negative update error: {error:?}"))?;
    }
    let reference_runtime = compiler
        .compile()
        .map_err(|error| format!("run-check time-split phase-center compile error: {error:?}"))?;
    let package_bytes = reference_runtime
        .to_bytes()
        .map_err(|error| format!("run-check time-split package serialization error: {error:?}"))?;
    write_binary_file(&package_path, &package_bytes)?;
    let read_package = std::fs::read(&package_path).map_err(|error| {
        format!(
            "failed to read run-check time-split package '{}': {error}",
            package_path.display()
        )
    })?;
    if read_package != package_bytes {
        return Err(format!(
            "run-check time-split package '{}' readback mismatch",
            package_path.display()
        ));
    }
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&read_package)
        .map_err(|error| format!("run-check time-split package inspect error: {error:?}"))?;
    let offload_runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &read_package,
        PhaseCenterOffloadPolicy::new(margin_threshold_micro)
            .map_err(|error| format!("invalid run-check time-split policy: {error:?}"))?,
    )
    .map_err(|error| format!("run-check time-split package load error: {error:?}"))?;

    let exact_cache_flags = exact_cache_hit_flags_phase_atom_binary(&events);
    let mut margins = Vec::new();
    let mut correct_rows = 0usize;
    let mut wrong_wins = 0usize;
    let mut false_accepts = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut heldout_local_operator_calls = 0usize;
    let mut heldout_fallback_calls = 0usize;
    let mut exact_cache_hits_in_heldout = 0usize;
    let mut unique_cpu_accepts_over_exact_cache = 0usize;
    let mut nando_cpu_tokens_saved = 0usize;
    let mut nando_cpu_cost_saved_microusd = 0u64;
    let mut unique_accepts = Vec::new();
    for &event_index in &heldout_indices {
        let event = &events[event_index];
        let program_index = usize::from(!event.verified_safe_accept);
        let correct_vec = phase_atom_binary_event_vector(event, event.verified_safe_accept, cells);
        let wrong_vec = phase_atom_binary_event_vector(event, !event.verified_safe_accept, cells);
        let task = PhaseCenterEvalTask {
            center_index: program_index,
            correct_vec: correct_vec.clone().into_boxed_slice(),
            wrong_vec: wrong_vec.clone().into_boxed_slice(),
        };
        let reference_margin = reference_runtime
            .margin(&task)
            .map_err(|error| format!("run-check time-split reference margin error: {error:?}"))?;
        let runtime_margin = offload_runtime
            .runtime()
            .margin(&task)
            .map_err(|error| format!("run-check time-split runtime margin error: {error:?}"))?;
        let reference_micro = margin_to_micro(reference_margin)?;
        let runtime_micro = margin_to_micro(runtime_margin)?;
        runtime_margin_parity_mismatches += usize::from(reference_micro != runtime_micro);
        margins.push(runtime_micro);
        correct_rows += usize::from(runtime_micro > 0);
        wrong_wins += usize::from(runtime_micro <= 0);
        let decision = offload_runtime
            .offload_decision(&task)
            .map_err(|error| format!("run-check time-split offload decision error: {error:?}"))?;
        if decision.action == nando_core::PhaseCenterOffloadAction::LocalOperator {
            heldout_local_operator_calls += 1;
            false_accepts += usize::from(runtime_micro <= 0);
            if !exact_cache_flags[event_index] {
                unique_cpu_accepts_over_exact_cache += 1;
                nando_cpu_tokens_saved =
                    nando_cpu_tokens_saved.saturating_add(event.token_cost.total_tokens);
                nando_cpu_cost_saved_microusd = nando_cpu_cost_saved_microusd
                    .saturating_add(event.token_cost.total_cost_microusd);
                unique_accepts.push(GenericAcceptedEventReport {
                    request_fingerprint: format!("phase_atom_run_check:{}", event.exact_cache_key),
                    total_tokens: event.token_cost.total_tokens,
                    total_cost_microusd: event.token_cost.total_cost_microusd,
                    token_evidence_missing: event.token_cost.token_evidence_missing,
                    cost_evidence_missing: event.token_cost.cost_evidence_missing,
                });
            }
        } else {
            heldout_fallback_calls += 1;
        }
        exact_cache_hits_in_heldout += usize::from(exact_cache_flags[event_index]);
    }
    margins.sort_unstable();
    let accepted_for_offline_review = !heldout_indices.is_empty()
        && train_heldout_time_order_ok
        && wrong_wins == 0
        && false_accepts == 0
        && runtime_margin_parity_mismatches == 0
        && package_info.record_count == reference_runtime.record_count()
        && package_info.serialized_len == read_package.len();
    let rejection_reason = if accepted_for_offline_review {
        "accepted_for_time_split_offline_shadow_review".to_owned()
    } else if !train_heldout_time_order_ok {
        "train_heldout_time_order_failed".to_owned()
    } else if wrong_wins > 0 {
        "wrong_wins_detected".to_owned()
    } else if false_accepts > 0 {
        "false_accepts_detected".to_owned()
    } else if runtime_margin_parity_mismatches > 0 {
        "runtime_margin_parity_mismatches".to_owned()
    } else {
        "package_shape_mismatch".to_owned()
    };
    let report = PhaseAtomRunCheckDiscoveryReport {
        report_kind: "phase_atom_run_check_time_split_discovery_v1",
        mode: "verifier_bound_time_split_phase_atom_to_quarantine_nwpc_candidate",
        action_family: "action_family:run_check".to_owned(),
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        package_path: package_path.display().to_string(),
        cells,
        margin_threshold_micro,
        total_rows,
        parsed_verifier_events: events.len(),
        positive_pass_events,
        negative_events,
        split_granularity: "event_timestamp_older_train_newer_shadow",
        train_heldout_time_order_ok,
        train_time_min,
        train_time_max,
        heldout_time_min,
        heldout_time_max,
        train_events: train_indices.len(),
        heldout_events: heldout_indices.len(),
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: read_package.len(),
        package_records: package_info.record_count,
        heldout_accuracy_milli: per_thousand(correct_rows, heldout_indices.len()),
        heldout_local_operator_calls,
        heldout_fallback_calls,
        false_accepts,
        wrong_wins,
        runtime_margin_parity_mismatches,
        min_margin_micro: margins.first().copied().unwrap_or(0),
        median_margin_micro: percentile_i64(&margins, 50),
        p10_margin_micro: percentile_i64(&margins, 10),
        exact_cache_hits_in_heldout,
        unique_cpu_accepts_over_exact_cache,
        nando_cpu_tokens_saved,
        nando_cpu_cost_saved_microusd,
        unique_accepts,
        verifier_bound: true,
        quarantine_only: true,
        promoted: false,
        serving_profile_artifact: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        accepted_for_offline_review,
        rejection_reason,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "time-split quarantine candidate only: compiles older verifier-bound run_check phase atoms into .nwpc and shadows newer events; no promotion, no serving profile, no product local_accept, no market money claim, no target/proof authority, and no legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_atom_run_check_time_split_discovery_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  package_path: {}", package_path.display());
    println!(
        "  parsed_verifier_events: {}",
        report.parsed_verifier_events
    );
    println!("  train_events: {}", report.train_events);
    println!("  heldout_events: {}", report.heldout_events);
    println!(
        "  train_heldout_time_order_ok: {}",
        report.train_heldout_time_order_ok
    );
    println!(
        "  heldout_accuracy_milli: {}",
        report.heldout_accuracy_milli
    );
    println!("  wrong_wins: {}", report.wrong_wins);
    println!("  false_accepts: {}", report.false_accepts);
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  quarantine_only: {}", report.quarantine_only);
    println!("  promoted: {}", report.promoted);
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_action_family_time_split_discovery_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let action_family = args
        .next()
        .unwrap_or_else(|| "action_family:metrics_report".to_owned());
    if !action_family.starts_with("action_family:") {
        return Err("action_family must start with 'action_family:'".to_owned());
    }
    let task_name = action_family
        .strip_prefix("action_family:")
        .unwrap_or(action_family.as_str())
        .replace(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_', "_");
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_PHASE_ATOM_ACTION_FAMILY_TIME_SPLIT_DISCOVERY_REPORT)
    });
    let package_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_PHASE_ATOM_ACTION_FAMILY_TIME_SPLIT_DISCOVERY_PACKAGE)
    });
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CELLS);
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin threshold '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
    let train_permille = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid train_permille '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_TRAIN_PERMILLE);
    if !(1..=999).contains(&train_permille) {
        return Err("train_permille must be in 1..=999".to_owned());
    }
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
        } else {
            rest
        }
    };

    let mut total_rows = 0usize;
    let mut events = Vec::new();
    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read action-family time-split phase atom trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse action-family time-split phase atom trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            total_rows += 1;
            if let Some(event) = parse_phase_atom_binary_event_for_action(
                &row,
                events.len(),
                &action_family,
                &task_name,
            ) {
                events.push(event);
            }
        }
    }

    let positive_pass_events = events
        .iter()
        .filter(|event| event.verified_safe_accept)
        .count();
    let negative_events = events.len().saturating_sub(positive_pass_events);
    if positive_pass_events == 0 || negative_events == 0 {
        return Err(format!(
            "action-family time-split discovery for {action_family} needs positive and negative verifier events, got pass={} negative={}",
            positive_pass_events, negative_events
        ));
    }
    let (train_indices, heldout_indices) =
        phase_atom_binary_time_split_indices(&events, train_permille);
    if train_indices.is_empty() || heldout_indices.is_empty() {
        return Err(
            "action-family time-split discovery needs non-empty train and heldout splits"
                .to_owned(),
        );
    }
    if !phase_atom_binary_split_has_both_labels(&events, &train_indices) {
        return Err(
            "action-family time-split train window does not contain both labels".to_owned(),
        );
    }
    if !phase_atom_binary_split_has_both_labels(&events, &heldout_indices) {
        return Err(
            "action-family time-split shadow window does not contain both labels".to_owned(),
        );
    }

    let (train_time_min, train_time_max) = phase_atom_binary_time_range(&events, &train_indices);
    let (heldout_time_min, heldout_time_max) =
        phase_atom_binary_time_range(&events, &heldout_indices);
    let train_heldout_time_order_ok =
        phase_atom_binary_time_order_ok(&events, &train_indices, &heldout_indices);

    let mut compiler = PhaseCenterCompiler::new(cells, 2).map_err(|error| {
        format!("action-family time-split compiler error for {action_family}: {error:?}")
    })?;
    for &event_index in &train_indices {
        let event = &events[event_index];
        let program_index = usize::from(!event.verified_safe_accept);
        let positive_vec = phase_atom_binary_event_vector_for_task(
            event,
            event.verified_safe_accept,
            cells,
            &task_name,
        );
        let negative_vec = phase_atom_binary_event_vector_for_task(
            event,
            !event.verified_safe_accept,
            cells,
            &task_name,
        );
        compiler
            .add_positive_vector(program_index, &positive_vec)
            .map_err(|error| format!("action-family positive update error: {error:?}"))?;
        compiler
            .add_negative_vector(program_index, &negative_vec)
            .map_err(|error| format!("action-family negative update error: {error:?}"))?;
    }
    let reference_runtime = compiler
        .compile()
        .map_err(|error| format!("action-family time-split compile error: {error:?}"))?;
    let package_bytes = reference_runtime
        .to_bytes()
        .map_err(|error| format!("action-family package serialization error: {error:?}"))?;
    write_binary_file(&package_path, &package_bytes)?;
    let read_package = std::fs::read(&package_path).map_err(|error| {
        format!(
            "failed to read action-family package '{}': {error}",
            package_path.display()
        )
    })?;
    if read_package != package_bytes {
        return Err(format!(
            "action-family package '{}' readback mismatch",
            package_path.display()
        ));
    }
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&read_package)
        .map_err(|error| format!("action-family package inspect error: {error:?}"))?;
    let offload_runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &read_package,
        PhaseCenterOffloadPolicy::new(margin_threshold_micro)
            .map_err(|error| format!("invalid action-family policy: {error:?}"))?,
    )
    .map_err(|error| format!("action-family package load error: {error:?}"))?;

    let exact_cache_flags = exact_cache_hit_flags_phase_atom_binary(&events);
    let mut margins = Vec::new();
    let mut correct_rows = 0usize;
    let mut wrong_wins = 0usize;
    let mut false_accepts = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut heldout_local_operator_calls = 0usize;
    let mut heldout_fallback_calls = 0usize;
    let mut exact_cache_hits_in_heldout = 0usize;
    let mut unique_cpu_accepts_over_exact_cache = 0usize;
    let mut nando_cpu_tokens_saved = 0usize;
    let mut nando_cpu_cost_saved_microusd = 0u64;
    let mut unique_accepts = Vec::new();
    for &event_index in &heldout_indices {
        let event = &events[event_index];
        let program_index = usize::from(!event.verified_safe_accept);
        let correct_vec = phase_atom_binary_event_vector_for_task(
            event,
            event.verified_safe_accept,
            cells,
            &task_name,
        );
        let wrong_vec = phase_atom_binary_event_vector_for_task(
            event,
            !event.verified_safe_accept,
            cells,
            &task_name,
        );
        let task = PhaseCenterEvalTask {
            center_index: program_index,
            correct_vec: correct_vec.clone().into_boxed_slice(),
            wrong_vec: wrong_vec.clone().into_boxed_slice(),
        };
        let reference_micro = margin_to_micro(
            reference_runtime
                .margin(&task)
                .map_err(|error| format!("action-family reference margin error: {error:?}"))?,
        )?;
        let runtime_micro = margin_to_micro(
            offload_runtime
                .runtime()
                .margin(&task)
                .map_err(|error| format!("action-family runtime margin error: {error:?}"))?,
        )?;
        runtime_margin_parity_mismatches += usize::from(reference_micro != runtime_micro);
        margins.push(runtime_micro);
        correct_rows += usize::from(runtime_micro > 0);
        wrong_wins += usize::from(runtime_micro <= 0);
        let decision = offload_runtime
            .offload_decision(&task)
            .map_err(|error| format!("action-family offload decision error: {error:?}"))?;
        if decision.action == nando_core::PhaseCenterOffloadAction::LocalOperator {
            heldout_local_operator_calls += 1;
            false_accepts += usize::from(runtime_micro <= 0);
            if !exact_cache_flags[event_index] {
                unique_cpu_accepts_over_exact_cache += 1;
                nando_cpu_tokens_saved =
                    nando_cpu_tokens_saved.saturating_add(event.token_cost.total_tokens);
                nando_cpu_cost_saved_microusd = nando_cpu_cost_saved_microusd
                    .saturating_add(event.token_cost.total_cost_microusd);
                unique_accepts.push(GenericAcceptedEventReport {
                    request_fingerprint: format!(
                        "phase_atom_{task_name}:{}",
                        event.exact_cache_key
                    ),
                    total_tokens: event.token_cost.total_tokens,
                    total_cost_microusd: event.token_cost.total_cost_microusd,
                    token_evidence_missing: event.token_cost.token_evidence_missing,
                    cost_evidence_missing: event.token_cost.cost_evidence_missing,
                });
            }
        } else {
            heldout_fallback_calls += 1;
        }
        exact_cache_hits_in_heldout += usize::from(exact_cache_flags[event_index]);
    }
    margins.sort_unstable();
    let accepted_for_offline_review = !heldout_indices.is_empty()
        && train_heldout_time_order_ok
        && wrong_wins == 0
        && false_accepts == 0
        && runtime_margin_parity_mismatches == 0
        && package_info.record_count == reference_runtime.record_count()
        && package_info.serialized_len == read_package.len();
    let rejection_reason = if accepted_for_offline_review {
        "accepted_for_time_split_offline_shadow_review".to_owned()
    } else if !train_heldout_time_order_ok {
        "train_heldout_time_order_failed".to_owned()
    } else if wrong_wins > 0 {
        "wrong_wins_detected".to_owned()
    } else if false_accepts > 0 {
        "false_accepts_detected".to_owned()
    } else if runtime_margin_parity_mismatches > 0 {
        "runtime_margin_parity_mismatches".to_owned()
    } else {
        "package_shape_mismatch".to_owned()
    };
    let report = PhaseAtomRunCheckDiscoveryReport {
        report_kind: "phase_atom_action_family_time_split_discovery_v1",
        mode: "verifier_bound_time_split_phase_atom_action_family_to_quarantine_nwpc_candidate",
        action_family: action_family.clone(),
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        package_path: package_path.display().to_string(),
        cells,
        margin_threshold_micro,
        total_rows,
        parsed_verifier_events: events.len(),
        positive_pass_events,
        negative_events,
        split_granularity: "event_timestamp_older_train_newer_shadow",
        train_heldout_time_order_ok,
        train_time_min,
        train_time_max,
        heldout_time_min,
        heldout_time_max,
        train_events: train_indices.len(),
        heldout_events: heldout_indices.len(),
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: read_package.len(),
        package_records: package_info.record_count,
        heldout_accuracy_milli: per_thousand(correct_rows, heldout_indices.len()),
        heldout_local_operator_calls,
        heldout_fallback_calls,
        false_accepts,
        wrong_wins,
        runtime_margin_parity_mismatches,
        min_margin_micro: margins.first().copied().unwrap_or(0),
        median_margin_micro: percentile_i64(&margins, 50),
        p10_margin_micro: percentile_i64(&margins, 10),
        exact_cache_hits_in_heldout,
        unique_cpu_accepts_over_exact_cache,
        nando_cpu_tokens_saved,
        nando_cpu_cost_saved_microusd,
        unique_accepts,
        verifier_bound: true,
        quarantine_only: true,
        promoted: false,
        serving_profile_artifact: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        accepted_for_offline_review,
        rejection_reason,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "time-split quarantine candidate only: compiles older verifier-bound action-family phase atoms into .nwpc and shadows newer events; no promotion, no serving profile, no product local_accept, no market money claim, no target/proof authority, and no legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_atom_action_family_time_split_discovery_v1:");
    println!("  action_family: {action_family}");
    println!("  report_path: {}", report_path.display());
    println!("  package_path: {}", package_path.display());
    println!(
        "  parsed_verifier_events: {}",
        report.parsed_verifier_events
    );
    println!("  train_events: {}", report.train_events);
    println!("  heldout_events: {}", report.heldout_events);
    println!(
        "  train_heldout_time_order_ok: {}",
        report.train_heldout_time_order_ok
    );
    println!(
        "  heldout_accuracy_milli: {}",
        report.heldout_accuracy_milli
    );
    println!("  wrong_wins: {}", report.wrong_wins);
    println!("  false_accepts: {}", report.false_accepts);
    println!(
        "  local_operator_calls: {}",
        report.heldout_local_operator_calls
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.unique_cpu_accepts_over_exact_cache
    );
    println!("  quarantine_only: {}", report.quarantine_only);
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_run_check_time_split_promotion_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let discovery_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_DISCOVERY_REPORT));
    let discovery = read_json_value(&discovery_report_path)?;
    let candidate_package_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        json_string(&discovery, &["package_path"])
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_DISCOVERY_PACKAGE)
            })
    });
    let audit_report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_PROMOTION_AUDIT_REPORT)
    });
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin threshold '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
    if margin_threshold_micro <= 0 {
        return Err("margin threshold must be > 0".to_owned());
    }
    let price_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PRICE_CONFIG));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let price_config = read_json_file::<ModelPriceConfig>(&price_config_path)?;
    let package_bytes = std::fs::read(&candidate_package_path).map_err(|error| {
        format!(
            "failed to read candidate package '{}': {error}",
            candidate_package_path.display()
        )
    })?;
    let package_magic_ok = package_bytes.starts_with(b"NWPCF001");
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes)
        .map_err(|error| format!("run-check time-split package inspect error: {error:?}"))?;

    let report_kind = json_string(&discovery, &["report_kind"]).unwrap_or_default();
    let discovery_mode = json_string(&discovery, &["mode"]).unwrap_or_default();
    let discovery_action_family =
        json_string(&discovery, &["action_family"]).unwrap_or_else(|| {
            if report_kind == "phase_atom_run_check_time_split_discovery_v1" {
                "action_family:run_check".to_owned()
            } else {
                "action_family:unknown".to_owned()
            }
        });
    let split_granularity = json_string(&discovery, &["split_granularity"]).unwrap_or_default();
    let train_heldout_time_order_ok =
        json_bool(&discovery, &["train_heldout_time_order_ok"]).unwrap_or(false);
    let verifier_bound = json_bool(&discovery, &["verifier_bound"]).unwrap_or(false);
    let accepted_for_offline_review =
        json_bool(&discovery, &["accepted_for_offline_review"]).unwrap_or(false);
    let quarantine_only = json_bool(&discovery, &["quarantine_only"]).unwrap_or(false);
    let discovery_promoted = json_bool(&discovery, &["promoted"]).unwrap_or(true);
    let discovery_serving_profile_artifact =
        json_bool(&discovery, &["serving_profile_artifact"]).unwrap_or(true);
    let discovery_local_accept_enabled =
        json_bool(&discovery, &["local_accept_enabled"]).unwrap_or(true);
    let product_runtime_changed =
        json_bool(&discovery, &["product_runtime_changed"]).unwrap_or(true);
    let serving_runtime_changed =
        json_bool(&discovery, &["serving_runtime_changed"]).unwrap_or(true);
    let discovery_market_money_claim =
        json_bool(&discovery, &["market_money_claim_allowed"]).unwrap_or(true);

    let train_events = json_u64(&discovery, &["train_events"]).unwrap_or_default() as usize;
    let heldout_events = json_u64(&discovery, &["heldout_events"]).unwrap_or_default() as usize;
    let heldout_accuracy_milli =
        json_u64(&discovery, &["heldout_accuracy_milli"]).unwrap_or_default() as usize;
    let heldout_local_operator_calls =
        json_u64(&discovery, &["heldout_local_operator_calls"]).unwrap_or_default() as usize;
    let heldout_fallback_calls =
        json_u64(&discovery, &["heldout_fallback_calls"]).unwrap_or_default() as usize;
    let false_accepts =
        json_u64(&discovery, &["false_accepts"]).unwrap_or(usize::MAX as u64) as usize;
    let wrong_wins = json_u64(&discovery, &["wrong_wins"]).unwrap_or(usize::MAX as u64) as usize;
    let runtime_margin_parity_mismatches =
        json_u64(&discovery, &["runtime_margin_parity_mismatches"]).unwrap_or(usize::MAX as u64)
            as usize;
    let min_margin_micro = json_u64(&discovery, &["min_margin_micro"]).unwrap_or_default() as i64;
    let p10_margin_micro = json_u64(&discovery, &["p10_margin_micro"]).unwrap_or_default() as i64;
    let median_margin_micro =
        json_u64(&discovery, &["median_margin_micro"]).unwrap_or_default() as i64;
    let exact_cache_hits_in_heldout =
        json_u64(&discovery, &["exact_cache_hits_in_heldout"]).unwrap_or_default() as usize;
    let unique_cpu_accepts_over_exact_cache =
        json_u64(&discovery, &["unique_cpu_accepts_over_exact_cache"]).unwrap_or_default() as usize;
    let nando_cpu_tokens_saved =
        json_u64(&discovery, &["nando_cpu_tokens_saved"]).unwrap_or_default() as usize;
    let nando_cpu_cost_saved_microusd =
        json_u64(&discovery, &["nando_cpu_cost_saved_microusd"]).unwrap_or_default();
    let discovery_unique_accepts = json_at(&discovery, &["unique_accepts"])
        .and_then(serde_json::Value::as_array)
        .map(|accepted_events| {
            accepted_events
                .iter()
                .filter_map(|accepted| {
                    let request_fingerprint = accepted
                        .get("request_fingerprint")
                        .and_then(serde_json::Value::as_str)?;
                    let total_tokens = json_usize(accepted.get("total_tokens")).unwrap_or(0);
                    let report_cost = accepted
                        .get("total_cost_microusd")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let estimated_cost = if report_cost == 0 && total_tokens > 0 {
                        estimated_event_cost_microusd(total_tokens, 0, &price_config)
                    } else {
                        0
                    };
                    Some(GenericAcceptedEventReport {
                        request_fingerprint: request_fingerprint.to_owned(),
                        total_tokens,
                        total_cost_microusd: report_cost.max(estimated_cost),
                        token_evidence_missing: accepted
                            .get("token_evidence_missing")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(total_tokens == 0),
                        cost_evidence_missing: report_cost == 0 && estimated_cost == 0,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let report_package_fingerprint64 =
        json_u64(&discovery, &["package_fingerprint64"]).unwrap_or_default();
    let report_package_bytes =
        json_u64(&discovery, &["package_bytes"]).unwrap_or_default() as usize;
    let report_package_records =
        json_u64(&discovery, &["package_records"]).unwrap_or_default() as usize;
    let inspect_matches_discovery_report = report_package_fingerprint64
        == package_info.fingerprint64
        && report_package_bytes == package_bytes.len()
        && report_package_records == package_info.record_count;

    let forbidden_target =
        json_bool(&discovery, &["forbidden_flags", "target_id_used"]).unwrap_or(true);
    let forbidden_proof = json_bool(
        &discovery,
        &["forbidden_flags", "proof_rule_id_authority_used"],
    )
    .unwrap_or(true);
    let forbidden_lookup =
        json_bool(&discovery, &["forbidden_flags", "concrete_x_lookup_used"]).unwrap_or(true);
    let forbidden_local_out_t =
        json_bool(&discovery, &["forbidden_flags", "manual_local_out_t_used"]).unwrap_or(true);
    let forbidden_bind = json_bool(
        &discovery,
        &["forbidden_flags", "hidden_frame_id_or_bind_x_used"],
    )
    .unwrap_or(true);
    let forbidden_legacy =
        json_bool(&discovery, &["forbidden_flags", "legacy_backend_used"]).unwrap_or(true);
    let forbidden_flags_clear = !forbidden_target
        && !forbidden_proof
        && !forbidden_lookup
        && !forbidden_local_out_t
        && !forbidden_bind
        && !forbidden_legacy;

    let token_evidence_present = nando_cpu_tokens_saved > 0;
    let provider_cost_evidence_present = nando_cpu_cost_saved_microusd > 0;
    let estimated_nando_cpu_cost_saved_microusd =
        if token_evidence_present && !provider_cost_evidence_present {
            estimated_event_cost_microusd(nando_cpu_tokens_saved, 0, &price_config)
        } else {
            0
        };
    let explicit_model_price_estimate_used = estimated_nando_cpu_cost_saved_microusd > 0;
    let estimated_cost_method = if explicit_model_price_estimate_used {
        "total_saved_tokens_as_input_token_floor_from_model_price_config".to_owned()
    } else if provider_cost_evidence_present {
        "provider_cost_evidence_present_no_estimate_needed".to_owned()
    } else {
        "no_token_or_price_estimate_available".to_owned()
    };
    let supported_run_check_discovery = report_kind
        == "phase_atom_run_check_time_split_discovery_v1"
        && discovery_mode == "verifier_bound_time_split_phase_atom_to_quarantine_nwpc_candidate";
    let supported_action_family_discovery = report_kind
        == "phase_atom_action_family_time_split_discovery_v1"
        && discovery_mode
            == "verifier_bound_time_split_phase_atom_action_family_to_quarantine_nwpc_candidate";
    let supported_time_split_discovery =
        supported_run_check_discovery || supported_action_family_discovery;
    let promotion_report_kind = if supported_action_family_discovery {
        "phase_atom_action_family_time_split_promotion_audit_v1"
    } else {
        "phase_atom_run_check_time_split_promotion_audit_v1"
    };
    let promotion_candidate_allowed = supported_time_split_discovery
        && split_granularity == "event_timestamp_older_train_newer_shadow"
        && train_heldout_time_order_ok
        && verifier_bound
        && accepted_for_offline_review
        && quarantine_only
        && !discovery_promoted
        && !discovery_serving_profile_artifact
        && !discovery_local_accept_enabled
        && !product_runtime_changed
        && !serving_runtime_changed
        && !discovery_market_money_claim
        && heldout_events > 0
        && heldout_local_operator_calls > 0
        && unique_cpu_accepts_over_exact_cache > 0
        && discovery_unique_accepts.len() == unique_cpu_accepts_over_exact_cache
        && false_accepts == 0
        && wrong_wins == 0
        && runtime_margin_parity_mismatches == 0
        && min_margin_micro >= margin_threshold_micro
        && package_magic_ok
        && inspect_matches_discovery_report
        && forbidden_flags_clear;
    let product_promotion_allowed = false;
    let market_money_claim_allowed = false;
    let money_claim_blocker = if provider_cost_evidence_present {
        "product_money_claim_still_blocked_in_offline_audit_mode".to_owned()
    } else if explicit_model_price_estimate_used {
        "provider_cost_missing_internal_price_estimate_only".to_owned()
    } else {
        "provider_cost_evidence_missing_or_zero".to_owned()
    };
    let rejection_reason = if promotion_candidate_allowed {
        "accepted_for_quarantine_promotion_candidate_review_product_accept_still_disabled"
            .to_owned()
    } else if !supported_time_split_discovery {
        "wrong_or_unsupported_discovery_report_kind_or_mode".to_owned()
    } else if !train_heldout_time_order_ok {
        "train_heldout_time_order_failed".to_owned()
    } else if !accepted_for_offline_review {
        "discovery_not_accepted_for_offline_review".to_owned()
    } else if false_accepts > 0 {
        "false_accepts_detected".to_owned()
    } else if wrong_wins > 0 {
        "wrong_wins_detected".to_owned()
    } else if runtime_margin_parity_mismatches > 0 {
        "runtime_margin_parity_mismatches".to_owned()
    } else if !inspect_matches_discovery_report {
        "package_inspect_mismatch".to_owned()
    } else if !forbidden_flags_clear {
        "forbidden_flag_detected".to_owned()
    } else if min_margin_micro < margin_threshold_micro {
        "min_margin_below_audit_threshold".to_owned()
    } else if discovery_unique_accepts.len() != unique_cpu_accepts_over_exact_cache {
        "unique_accepts_list_count_mismatch".to_owned()
    } else if unique_cpu_accepts_over_exact_cache == 0 {
        "no_unique_cpu_accepts_over_exact_cache".to_owned()
    } else {
        "promotion_candidate_gate_failed".to_owned()
    };

    let report = PhaseAtomRunCheckTimeSplitPromotionAuditReport {
        report_kind: promotion_report_kind,
        mode: "offline_quarantine_promotion_candidate_audit_only",
        action_family: discovery_action_family.clone(),
        discovery_report_path: discovery_report_path.display().to_string(),
        candidate_package_path: candidate_package_path.display().to_string(),
        model_price_config_path: price_config_path.display().to_string(),
        margin_threshold_micro,
        package: PhaseAtomRunCheckTimeSplitPackageAudit {
            package_kind: "phase_center_nwpc",
            package_magic_ok,
            package_fingerprint64: package_info.fingerprint64,
            package_bytes: package_bytes.len(),
            inspected_cells: package_info.cells,
            inspected_record_count: package_info.record_count,
            report_package_fingerprint64,
            report_package_bytes,
            report_package_records,
            inspect_matches_discovery_report,
        },
        discovery_gate: PhaseAtomRunCheckTimeSplitDiscoveryGate {
            discovery_report_kind: report_kind,
            discovery_mode,
            action_family: discovery_action_family,
            split_granularity,
            train_heldout_time_order_ok,
            verifier_bound,
            accepted_for_offline_review,
            quarantine_only,
            discovery_promoted,
            discovery_serving_profile_artifact,
            discovery_local_accept_enabled,
            train_events,
            heldout_events,
            heldout_accuracy_milli,
            heldout_local_operator_calls,
            heldout_fallback_calls,
            false_accepts,
            wrong_wins,
            runtime_margin_parity_mismatches,
            min_margin_micro,
            p10_margin_micro,
            median_margin_micro,
            exact_cache_hits_in_heldout,
            unique_cpu_accepts_over_exact_cache,
        },
        unique_accepts: discovery_unique_accepts,
        economics: PhaseAtomRunCheckTimeSplitEconomicsAudit {
            token_evidence_present,
            provider_cost_evidence_present,
            explicit_model_price_estimate_used,
            price_config_schema_version: price_config.schema_version,
            provider: price_config.default_provider,
            model_id: price_config.default_model_id,
            price_source: price_config.price_source,
            nando_cpu_tokens_saved,
            nando_cpu_cost_saved_microusd,
            estimated_nando_cpu_cost_saved_microusd,
            estimated_cost_method,
            projected_nando_calls_saved_milli: per_thousand(
                unique_cpu_accepts_over_exact_cache,
                heldout_events,
            ),
            projected_combined_calls_saved_milli: per_thousand(
                exact_cache_hits_in_heldout + unique_cpu_accepts_over_exact_cache,
                heldout_events,
            ),
            money_claim_blocker,
        },
        forbidden_flags: ForbiddenFlags {
            target_id_used: forbidden_target,
            proof_rule_id_authority_used: forbidden_proof,
            concrete_x_lookup_used: forbidden_lookup,
            manual_local_out_t_used: forbidden_local_out_t,
            hidden_frame_id_or_bind_x_used: forbidden_bind,
            legacy_backend_used: forbidden_legacy,
        },
        promotion_candidate_allowed,
        product_promotion_allowed,
        local_accept_enabled: false,
        promoted: false,
        serving_profile_artifact: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed,
        rejection_reason,
        boundary: "offline promotion audit only: verifies time-split .nwpc package/report coherence and promotion-candidate eligibility; it does not write a serving profile, enable local_accept, promote product runtime, allow market money claims, or use legacy nwrb/role-binding paths",
    };
    write_json_file(&audit_report_path, &report)?;
    println!("{}:", report.report_kind);
    println!("  report_path: {}", audit_report_path.display());
    println!(
        "  promotion_candidate_allowed: {}",
        report.promotion_candidate_allowed
    );
    println!(
        "  product_promotion_allowed: {}",
        report.product_promotion_allowed
    );
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.discovery_gate.unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  nando_cpu_tokens_saved: {}",
        report.economics.nando_cpu_tokens_saved
    );
    println!(
        "  nando_cpu_cost_saved_microusd: {}",
        report.economics.nando_cpu_cost_saved_microusd
    );
    println!(
        "  estimated_nando_cpu_cost_saved_microusd: {}",
        report.economics.estimated_nando_cpu_cost_saved_microusd
    );
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    println!("  rejection_reason: {}", report.rejection_reason);
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_action_family_serving_admission_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let promotion_audit_report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(
            "target/nando-wave/streaming/phase-atom-tool-status-time-split-promotion-audit-v1.report.json",
        )
    });
    let promotion = read_json_value(&promotion_audit_report_path)?;
    let discovery_report_path = json_string(&promotion, &["discovery_report_path"])
        .map(PathBuf::from)
        .ok_or_else(|| {
            format!(
                "promotion audit '{}' does not contain discovery_report_path",
                promotion_audit_report_path.display()
            )
        })?;
    let discovery = read_json_value(&discovery_report_path)?;
    let admission_report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_PHASE_ATOM_ACTION_FAMILY_SERVING_ADMISSION_AUDIT_REPORT)
    });
    let candidate_package_path = args
        .next()
        .map(PathBuf::from)
        .or_else(|| json_string(&promotion, &["candidate_package_path"]).map(PathBuf::from))
        .ok_or_else(|| {
            format!(
                "promotion audit '{}' does not contain candidate_package_path",
                promotion_audit_report_path.display()
            )
        })?;
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin threshold '{value}': {error}"))
        })
        .transpose()?
        .or_else(|| json_i64(&promotion, &["margin_threshold_micro"]))
        .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
    if margin_threshold_micro <= 0 {
        return Err("margin threshold must be > 0".to_owned());
    }
    let price_config_path = args
        .next()
        .map(PathBuf::from)
        .or_else(|| json_string(&promotion, &["model_price_config_path"]).map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PRICE_CONFIG));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            let from_discovery = json_string_vec(json_at(&discovery, &["input_trace_paths"]))
                .into_iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>();
            if from_discovery.is_empty() {
                vec![PathBuf::from(DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL)]
            } else {
                from_discovery
            }
        } else {
            rest
        }
    };

    let price_config = read_json_file::<ModelPriceConfig>(&price_config_path)?;
    let package_bytes = std::fs::read(&candidate_package_path).map_err(|error| {
        format!(
            "failed to read serving admission candidate package '{}': {error}",
            candidate_package_path.display()
        )
    })?;
    let package_magic_ok = package_bytes.starts_with(b"NWPCF001");
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes)
        .map_err(|error| format!("serving admission package inspect error: {error:?}"))?;
    let serving_runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &package_bytes,
        PhaseCenterOffloadPolicy::new(margin_threshold_micro)
            .map_err(|error| format!("serving admission invalid policy: {error:?}"))?,
    )
    .map_err(|error| format!("serving admission package load error: {error:?}"))?;

    let action_family = json_string(&promotion, &["action_family"])
        .or_else(|| json_string(&discovery, &["action_family"]))
        .unwrap_or_else(|| "action_family:unknown".to_owned());
    if !action_family.starts_with("action_family:") {
        return Err(format!(
            "serving admission action_family must start with action_family:, got '{action_family}'"
        ));
    }
    let task_name = action_family
        .strip_prefix("action_family:")
        .unwrap_or(action_family.as_str())
        .replace(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_', "_");

    let mut replay_total_rows = 0usize;
    let mut events = Vec::new();
    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read serving admission replay trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse serving admission replay trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            replay_total_rows += 1;
            if let Some(event) = parse_phase_atom_binary_event_for_action(
                &row,
                events.len(),
                &action_family,
                &task_name,
            ) {
                events.push(event);
            }
        }
    }
    if events.is_empty() {
        return Err(format!(
            "serving admission replay found no verifier-bound events for {action_family}"
        ));
    }
    let discovery_train_events =
        json_u64(&discovery, &["train_events"]).unwrap_or_default() as usize;
    let discovery_parsed_events =
        json_u64(&discovery, &["parsed_verifier_events"]).unwrap_or(events.len() as u64) as usize;
    let replay_train_permille = discovery_train_events
        .saturating_mul(1000)
        .checked_div(discovery_parsed_events)
        .unwrap_or(DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_TRAIN_PERMILLE)
        .clamp(1, 999);
    let (train_indices, heldout_indices) =
        phase_atom_binary_time_split_indices(&events, replay_train_permille);
    if train_indices.is_empty() || heldout_indices.is_empty() {
        return Err("serving admission replay needs non-empty train and heldout splits".to_owned());
    }
    let replay_train_heldout_time_order_ok =
        phase_atom_binary_time_order_ok(&events, &train_indices, &heldout_indices);

    let exact_cache_flags = exact_cache_hit_flags_phase_atom_binary(&events);
    let mut margins = Vec::with_capacity(heldout_indices.len());
    let mut latencies = Vec::with_capacity(heldout_indices.len());
    let mut replay_correct_rows = 0usize;
    let mut replay_wrong_wins = 0usize;
    let mut replay_false_accepts = 0usize;
    let mut replay_margin_parity_mismatches = 0usize;
    let mut replay_local_operator_calls = 0usize;
    let mut replay_fallback_calls = 0usize;
    let mut replay_exact_cache_hits_in_heldout = 0usize;
    let mut replay_unique_cpu_accepts_over_exact_cache = 0usize;
    let mut replay_nando_cpu_tokens_saved = 0usize;
    let mut replay_nando_cpu_cost_saved_microusd = 0u64;
    let mut unique_accepts = Vec::new();
    for &event_index in &heldout_indices {
        let event = &events[event_index];
        let program_index = usize::from(!event.verified_safe_accept);
        let correct_vec = phase_atom_binary_event_vector_for_task(
            event,
            event.verified_safe_accept,
            serving_runtime.cells(),
            &task_name,
        );
        let wrong_vec = phase_atom_binary_event_vector_for_task(
            event,
            !event.verified_safe_accept,
            serving_runtime.cells(),
            &task_name,
        );
        let task = PhaseCenterEvalTask {
            center_index: program_index,
            correct_vec: correct_vec.into_boxed_slice(),
            wrong_vec: wrong_vec.into_boxed_slice(),
        };
        let started = Instant::now();
        let direct_margin_micro = margin_to_micro(
            serving_runtime
                .runtime()
                .margin(&task)
                .map_err(|error| format!("serving admission direct margin error: {error:?}"))?,
        )?;
        let decision = serving_runtime
            .offload_decision(&task)
            .map_err(|error| format!("serving admission offload decision error: {error:?}"))?;
        latencies.push(started.elapsed().as_nanos());
        replay_margin_parity_mismatches +=
            usize::from(decision.margin_micro != direct_margin_micro);
        margins.push(decision.margin_micro);
        replay_correct_rows += usize::from(decision.margin_micro > 0);
        replay_wrong_wins += usize::from(decision.margin_micro <= 0);
        if decision.action == nando_core::PhaseCenterOffloadAction::LocalOperator {
            replay_local_operator_calls += 1;
            replay_false_accepts += usize::from(decision.margin_micro <= 0);
            if !exact_cache_flags[event_index] {
                replay_unique_cpu_accepts_over_exact_cache += 1;
                replay_nando_cpu_tokens_saved =
                    replay_nando_cpu_tokens_saved.saturating_add(event.token_cost.total_tokens);
                replay_nando_cpu_cost_saved_microusd = replay_nando_cpu_cost_saved_microusd
                    .saturating_add(event.token_cost.total_cost_microusd);
                unique_accepts.push(GenericAcceptedEventReport {
                    request_fingerprint: format!(
                        "serving_admission_{task_name}:{}",
                        event.exact_cache_key
                    ),
                    total_tokens: event.token_cost.total_tokens,
                    total_cost_microusd: event.token_cost.total_cost_microusd,
                    token_evidence_missing: event.token_cost.token_evidence_missing,
                    cost_evidence_missing: event.token_cost.cost_evidence_missing,
                });
            }
        } else {
            replay_fallback_calls += 1;
        }
        replay_exact_cache_hits_in_heldout += usize::from(exact_cache_flags[event_index]);
    }
    margins.sort_unstable();
    latencies.sort_unstable();

    let promotion_report_kind = json_string(&promotion, &["report_kind"]).unwrap_or_default();
    let promotion_mode = json_string(&promotion, &["mode"]).unwrap_or_default();
    let promotion_candidate_allowed =
        json_bool(&promotion, &["promotion_candidate_allowed"]).unwrap_or(false);
    let promotion_product_promotion_allowed =
        json_bool(&promotion, &["product_promotion_allowed"]).unwrap_or(true);
    let promotion_local_accept_enabled =
        json_bool(&promotion, &["local_accept_enabled"]).unwrap_or(true);
    let promotion_promoted = json_bool(&promotion, &["promoted"]).unwrap_or(true);
    let promotion_serving_profile_artifact =
        json_bool(&promotion, &["serving_profile_artifact"]).unwrap_or(true);
    let promotion_product_runtime_changed =
        json_bool(&promotion, &["product_runtime_changed"]).unwrap_or(true);
    let promotion_serving_runtime_changed =
        json_bool(&promotion, &["serving_runtime_changed"]).unwrap_or(true);
    let promotion_market_money_claim_allowed =
        json_bool(&promotion, &["market_money_claim_allowed"]).unwrap_or(true);
    let promotion_rejection_reason =
        json_string(&promotion, &["rejection_reason"]).unwrap_or_default();
    let promotion_unique_cpu_accepts_over_exact_cache = json_u64(
        &promotion,
        &["discovery_gate", "unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or_default() as usize;
    let promotion_tokens_saved =
        json_u64(&promotion, &["economics", "nando_cpu_tokens_saved"]).unwrap_or_default() as usize;
    let promotion_estimated_cost_saved_microusd = json_u64(
        &promotion,
        &["economics", "estimated_nando_cpu_cost_saved_microusd"],
    )
    .unwrap_or_default();
    let report_package_fingerprint64 =
        json_u64(&promotion, &["package", "package_fingerprint64"]).unwrap_or_default();
    let report_package_bytes =
        json_u64(&promotion, &["package", "package_bytes"]).unwrap_or_default() as usize;
    let report_package_records =
        json_u64(&promotion, &["package", "inspected_record_count"]).unwrap_or_default() as usize;
    let inspect_matches_promotion_report = package_info.fingerprint64
        == report_package_fingerprint64
        && package_bytes.len() == report_package_bytes
        && package_info.record_count == report_package_records;

    let forbidden_target =
        json_bool(&promotion, &["forbidden_flags", "target_id_used"]).unwrap_or(true);
    let forbidden_proof = json_bool(
        &promotion,
        &["forbidden_flags", "proof_rule_id_authority_used"],
    )
    .unwrap_or(true);
    let forbidden_lookup =
        json_bool(&promotion, &["forbidden_flags", "concrete_x_lookup_used"]).unwrap_or(true);
    let forbidden_local_out_t =
        json_bool(&promotion, &["forbidden_flags", "manual_local_out_t_used"]).unwrap_or(true);
    let forbidden_bind = json_bool(
        &promotion,
        &["forbidden_flags", "hidden_frame_id_or_bind_x_used"],
    )
    .unwrap_or(true);
    let forbidden_legacy =
        json_bool(&promotion, &["forbidden_flags", "legacy_backend_used"]).unwrap_or(true);
    let forbidden_flags_clear = !forbidden_target
        && !forbidden_proof
        && !forbidden_lookup
        && !forbidden_local_out_t
        && !forbidden_bind
        && !forbidden_legacy;

    let token_evidence_present = replay_nando_cpu_tokens_saved > 0;
    let provider_cost_evidence_present = replay_nando_cpu_cost_saved_microusd > 0;
    let estimated_nando_cpu_cost_saved_microusd =
        if token_evidence_present && !provider_cost_evidence_present {
            estimated_event_cost_microusd(replay_nando_cpu_tokens_saved, 0, &price_config)
        } else {
            0
        };
    let explicit_model_price_estimate_used = estimated_nando_cpu_cost_saved_microusd > 0;
    let estimated_cost_method = if explicit_model_price_estimate_used {
        "total_saved_tokens_as_input_token_floor_from_model_price_config".to_owned()
    } else if provider_cost_evidence_present {
        "provider_cost_evidence_present_no_estimate_needed".to_owned()
    } else {
        "no_token_or_price_estimate_available".to_owned()
    };
    let replay_matches_promotion_accept_count =
        replay_unique_cpu_accepts_over_exact_cache == promotion_unique_cpu_accepts_over_exact_cache;
    let replay_matches_promotion_token_count =
        replay_nando_cpu_tokens_saved == promotion_tokens_saved;
    let replay_effective_cost =
        replay_nando_cpu_cost_saved_microusd.max(estimated_nando_cpu_cost_saved_microusd);
    let replay_matches_promotion_cost_or_estimate = replay_effective_cost
        == promotion_estimated_cost_saved_microusd
        || promotion_estimated_cost_saved_microusd == 0;

    let supported_promotion = (promotion_report_kind
        == "phase_atom_action_family_time_split_promotion_audit_v1"
        || promotion_report_kind == "phase_atom_run_check_time_split_promotion_audit_v1")
        && promotion_mode == "offline_quarantine_promotion_candidate_audit_only";
    let serving_admission_candidate_allowed = supported_promotion
        && promotion_candidate_allowed
        && !promotion_product_promotion_allowed
        && !promotion_local_accept_enabled
        && !promotion_promoted
        && !promotion_serving_profile_artifact
        && !promotion_product_runtime_changed
        && !promotion_serving_runtime_changed
        && !promotion_market_money_claim_allowed
        && package_magic_ok
        && inspect_matches_promotion_report
        && replay_train_heldout_time_order_ok
        && !heldout_indices.is_empty()
        && replay_local_operator_calls > 0
        && replay_unique_cpu_accepts_over_exact_cache > 0
        && replay_wrong_wins == 0
        && replay_false_accepts == 0
        && replay_margin_parity_mismatches == 0
        && margins.first().copied().unwrap_or(0) >= margin_threshold_micro
        && replay_matches_promotion_accept_count
        && replay_matches_promotion_token_count
        && replay_matches_promotion_cost_or_estimate
        && forbidden_flags_clear;
    let rejection_reason = if serving_admission_candidate_allowed {
        "accepted_for_serving_admission_candidate_review_product_accept_still_disabled".to_owned()
    } else if !supported_promotion {
        "unsupported_or_wrong_promotion_audit_report".to_owned()
    } else if !promotion_candidate_allowed {
        "promotion_candidate_not_allowed".to_owned()
    } else if !inspect_matches_promotion_report {
        "package_inspect_mismatch_with_promotion_audit".to_owned()
    } else if replay_wrong_wins > 0 {
        "serving_replay_wrong_wins_detected".to_owned()
    } else if replay_false_accepts > 0 {
        "serving_replay_false_accepts_detected".to_owned()
    } else if replay_margin_parity_mismatches > 0 {
        "serving_replay_margin_parity_mismatches".to_owned()
    } else if margins.first().copied().unwrap_or(0) < margin_threshold_micro {
        "serving_replay_min_margin_below_threshold".to_owned()
    } else if !replay_matches_promotion_accept_count {
        "serving_replay_accept_count_differs_from_promotion_audit".to_owned()
    } else if !replay_matches_promotion_token_count {
        "serving_replay_token_count_differs_from_promotion_audit".to_owned()
    } else if !forbidden_flags_clear {
        "forbidden_flag_detected".to_owned()
    } else {
        "serving_admission_candidate_gate_failed".to_owned()
    };

    let report = PhaseAtomServingAdmissionAuditReport {
        report_kind: "phase_atom_action_family_serving_admission_audit_v1",
        mode: "serving_admission_replay_audit_only",
        action_family: action_family.clone(),
        promotion_audit_report_path: promotion_audit_report_path.display().to_string(),
        discovery_report_path: discovery_report_path.display().to_string(),
        candidate_package_path: candidate_package_path.display().to_string(),
        model_price_config_path: price_config_path.display().to_string(),
        replay_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        margin_threshold_micro,
        package: PhaseAtomRunCheckTimeSplitPackageAudit {
            package_kind: "phase_center_nwpc",
            package_magic_ok,
            package_fingerprint64: package_info.fingerprint64,
            package_bytes: package_bytes.len(),
            inspected_cells: package_info.cells,
            inspected_record_count: package_info.record_count,
            report_package_fingerprint64,
            report_package_bytes,
            report_package_records,
            inspect_matches_discovery_report: inspect_matches_promotion_report,
        },
        promotion_gate: PhaseAtomServingAdmissionPromotionGate {
            promotion_report_kind,
            promotion_mode,
            promotion_candidate_allowed,
            promotion_product_promotion_allowed,
            promotion_local_accept_enabled,
            promotion_promoted,
            promotion_serving_profile_artifact,
            promotion_product_runtime_changed,
            promotion_serving_runtime_changed,
            promotion_market_money_claim_allowed,
            promotion_rejection_reason,
            promotion_unique_cpu_accepts_over_exact_cache,
            promotion_tokens_saved,
            promotion_estimated_cost_saved_microusd,
        },
        replay: PhaseAtomServingAdmissionReplayAudit {
            runtime_package_loaded: true,
            runtime_cells: serving_runtime.cells(),
            runtime_record_count: serving_runtime.record_count(),
            runtime_bytes_estimate: serving_runtime.bytes_estimate(),
            replay_total_rows,
            replay_parsed_verifier_events: events.len(),
            replay_train_events: train_indices.len(),
            replay_heldout_events: heldout_indices.len(),
            replay_train_heldout_time_order_ok,
            replay_heldout_accuracy_milli: per_thousand(replay_correct_rows, heldout_indices.len()),
            replay_local_operator_calls,
            replay_fallback_calls,
            replay_false_accepts,
            replay_wrong_wins,
            replay_margin_parity_mismatches,
            replay_min_margin_micro: margins.first().copied().unwrap_or(0),
            replay_median_margin_micro: percentile_i64(&margins, 50),
            replay_p10_margin_micro: percentile_i64(&margins, 10),
            replay_exact_cache_hits_in_heldout,
            replay_unique_cpu_accepts_over_exact_cache,
            replay_nando_cpu_tokens_saved,
            replay_nando_cpu_cost_saved_microusd,
            replay_latency_p50_ns: percentile_u128(&latencies, 50),
            replay_latency_p90_ns: percentile_u128(&latencies, 90),
            replay_latency_p99_ns: percentile_u128(&latencies, 99),
            replay_latency_max_ns: latencies.last().copied().unwrap_or(0),
            replay_matches_promotion_accept_count,
            replay_matches_promotion_token_count,
            replay_matches_promotion_cost_or_estimate,
            unique_accepts,
        },
        economics: PhaseAtomRunCheckTimeSplitEconomicsAudit {
            token_evidence_present,
            provider_cost_evidence_present,
            explicit_model_price_estimate_used,
            price_config_schema_version: price_config.schema_version,
            provider: price_config.default_provider,
            model_id: price_config.default_model_id,
            price_source: price_config.price_source,
            nando_cpu_tokens_saved: replay_nando_cpu_tokens_saved,
            nando_cpu_cost_saved_microusd: replay_nando_cpu_cost_saved_microusd,
            estimated_nando_cpu_cost_saved_microusd,
            estimated_cost_method,
            projected_nando_calls_saved_milli: per_thousand(
                replay_unique_cpu_accepts_over_exact_cache,
                heldout_indices.len(),
            ),
            projected_combined_calls_saved_milli: per_thousand(
                replay_exact_cache_hits_in_heldout + replay_unique_cpu_accepts_over_exact_cache,
                heldout_indices.len(),
            ),
            money_claim_blocker: "serving admission is still replay-only; market money claim requires product shadow/live evidence".to_owned(),
        },
        forbidden_flags: ForbiddenFlags {
            target_id_used: forbidden_target,
            proof_rule_id_authority_used: forbidden_proof,
            concrete_x_lookup_used: forbidden_lookup,
            manual_local_out_t_used: forbidden_local_out_t,
            hidden_frame_id_or_bind_x_used: forbidden_bind,
            legacy_backend_used: forbidden_legacy,
        },
        serving_admission_candidate_allowed,
        product_promotion_allowed: false,
        local_accept_enabled: false,
        promoted: false,
        serving_profile_artifact: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        rejection_reason,
        boundary: "serving admission replay audit only: loads the quarantine .nwpc through PhaseCenterOffloadRuntime and replays verifier-bound heldout trace events with latency/economics; it does not write a serving profile, enable product local_accept, promote runtime, allow market claims, or use legacy nwrb/role-binding paths",
    };
    write_json_file(&admission_report_path, &report)?;
    println!("phase_atom_action_family_serving_admission_audit_v1:");
    println!("  report_path: {}", admission_report_path.display());
    println!("  action_family: {}", report.action_family);
    println!(
        "  serving_admission_candidate_allowed: {}",
        report.serving_admission_candidate_allowed
    );
    println!(
        "  replay_unique_cpu_accepts_over_exact_cache: {}",
        report.replay.replay_unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  replay_false_accepts: {}",
        report.replay.replay_false_accepts
    );
    println!(
        "  replay_p99_latency_ns: {}",
        report.replay.replay_latency_p99_ns
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!("  rejection_reason: {}", report.rejection_reason);
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_serving_shadow_replay_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    run_phase_stream_phase_atom_serving_shadow_replay(args, false, false)
}

pub(crate) fn run_phase_stream_phase_atom_serving_future_shadow_replay_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    run_phase_stream_phase_atom_serving_shadow_replay(args, true, false)
}

pub(crate) fn run_phase_stream_phase_atom_serving_append_shadow_replay_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    run_phase_stream_phase_atom_serving_shadow_replay(args, false, true)
}

pub(crate) fn run_phase_stream_phase_atom_live_admission_manifest_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let admission_report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_PHASE_ATOM_TOOL_STATUS_SERVING_ADMISSION_AUDIT_REPORT)
    });
    let shadow_replay_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_SERVING_APPEND_SHADOW_REPLAY_REPORT));
    let manifest_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_LIVE_ADMISSION_MANIFEST_REPORT));

    let admission = read_json_value(&admission_report_path)?;
    let shadow = read_json_value(&shadow_replay_report_path)?;

    let action_family = json_string(&admission, &["action_family"]).ok_or_else(|| {
        format!(
            "live admission admission report '{}' missing action_family",
            admission_report_path.display()
        )
    })?;
    let candidate_package_path = json_string(&admission, &["candidate_package_path"])
        .map(PathBuf::from)
        .ok_or_else(|| {
            format!(
                "live admission admission report '{}' missing candidate_package_path",
                admission_report_path.display()
            )
        })?;
    let package_bytes = std::fs::read(&candidate_package_path).map_err(|error| {
        format!(
            "failed to read live admission candidate package '{}': {error}",
            candidate_package_path.display()
        )
    })?;
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes)
        .map_err(|error| format!("live admission package inspect error: {error:?}"))?;

    let margin_threshold_micro = json_i64(&admission, &["margin_threshold_micro"])
        .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
    let admission_candidate_allowed =
        json_bool(&admission, &["serving_admission_candidate_allowed"]).unwrap_or(false);
    let admission_local_accept_disabled = !json_bool(&admission, &["local_accept_enabled"])
        .unwrap_or(true)
        && !json_bool(&admission, &["promoted"]).unwrap_or(true)
        && !json_bool(&admission, &["serving_profile_artifact"]).unwrap_or(true)
        && !json_bool(&admission, &["product_runtime_changed"]).unwrap_or(true)
        && !json_bool(&admission, &["serving_runtime_changed"]).unwrap_or(true)
        && !json_bool(&admission, &["market_money_claim_allowed"]).unwrap_or(true);

    let shadow_replay_allowed =
        json_bool(&shadow, &["serving_shadow_replay_allowed"]).unwrap_or(false);
    let shadow_append_or_future_only = json_bool(&shadow, &["append_window_replay"])
        .unwrap_or(false)
        || json_bool(&shadow, &["training_overlap_excluded"]).unwrap_or(false);
    let shadow_market_savings_count_allowed =
        json_bool(&shadow, &["market_savings_count_allowed"]).unwrap_or(false);
    let shadow_local_accept_disabled = !json_bool(&shadow, &["local_accept_enabled"])
        .unwrap_or(true)
        && !json_bool(&shadow, &["promoted"]).unwrap_or(true)
        && !json_bool(&shadow, &["serving_profile_artifact"]).unwrap_or(true)
        && !json_bool(&shadow, &["product_runtime_changed"]).unwrap_or(true)
        && !json_bool(&shadow, &["serving_runtime_changed"]).unwrap_or(true)
        && !json_bool(&shadow, &["market_money_claim_allowed"]).unwrap_or(true);

    let admission_package_fingerprint64 =
        json_u64(&admission, &["package", "package_fingerprint64"]).unwrap_or_default();
    let admission_package_bytes =
        json_u64(&admission, &["package", "package_bytes"]).unwrap_or_default() as usize;
    let admission_package_records =
        json_u64(&admission, &["package", "inspected_record_count"]).unwrap_or_default() as usize;
    let mut shadow_package_fingerprint64 = 0u64;
    let mut shadow_package_bytes = 0usize;
    let mut shadow_package_records = 0usize;
    let mut verifier_bound_profile_loaded = false;
    if let Some(profiles) = json_at(&shadow, &["profiles"]).and_then(serde_json::Value::as_array) {
        for profile in profiles {
            let profile_action = json_string(profile, &["action_family"]).unwrap_or_default();
            let profile_package =
                json_string(profile, &["candidate_package_path"]).unwrap_or_default();
            if profile_action == action_family
                && profile_package == candidate_package_path.display().to_string()
            {
                verifier_bound_profile_loaded =
                    json_bool(profile, &["admission_candidate_allowed"]).unwrap_or(false)
                        && json_bool(profile, &["forbidden_flags_clear"]).unwrap_or(false);
                shadow_package_fingerprint64 =
                    json_u64(profile, &["package_fingerprint64"]).unwrap_or_default();
                shadow_package_bytes =
                    json_u64(profile, &["package_bytes"]).unwrap_or_default() as usize;
                shadow_package_records =
                    json_u64(profile, &["runtime_record_count"]).unwrap_or_default() as usize;
                break;
            }
        }
    }
    let package_matches_admission_report = package_info.fingerprint64
        == admission_package_fingerprint64
        && package_bytes.len() == admission_package_bytes
        && package_info.record_count == admission_package_records;
    let package_matches_shadow_report = package_info.fingerprint64 == shadow_package_fingerprint64
        && package_bytes.len() == shadow_package_bytes
        && package_info.record_count == shadow_package_records;

    let forbidden_target = json_bool(&admission, &["forbidden_flags", "target_id_used"])
        .unwrap_or(true)
        || json_bool(&shadow, &["forbidden_flags", "target_id_used"]).unwrap_or(true);
    let forbidden_proof = json_bool(
        &admission,
        &["forbidden_flags", "proof_rule_id_authority_used"],
    )
    .unwrap_or(true)
        || json_bool(
            &shadow,
            &["forbidden_flags", "proof_rule_id_authority_used"],
        )
        .unwrap_or(true);
    let forbidden_lookup = json_bool(&admission, &["forbidden_flags", "concrete_x_lookup_used"])
        .unwrap_or(true)
        || json_bool(&shadow, &["forbidden_flags", "concrete_x_lookup_used"]).unwrap_or(true);
    let forbidden_local_out_t =
        json_bool(&admission, &["forbidden_flags", "manual_local_out_t_used"]).unwrap_or(true)
            || json_bool(&shadow, &["forbidden_flags", "manual_local_out_t_used"]).unwrap_or(true);
    let forbidden_bind = json_bool(
        &admission,
        &["forbidden_flags", "hidden_frame_id_or_bind_x_used"],
    )
    .unwrap_or(true)
        || json_bool(
            &shadow,
            &["forbidden_flags", "hidden_frame_id_or_bind_x_used"],
        )
        .unwrap_or(true);
    let forbidden_legacy = json_bool(&admission, &["forbidden_flags", "legacy_backend_used"])
        .unwrap_or(true)
        || json_bool(&shadow, &["forbidden_flags", "legacy_backend_used"]).unwrap_or(true);
    let forbidden_flags_clear = !forbidden_target
        && !forbidden_proof
        && !forbidden_lookup
        && !forbidden_local_out_t
        && !forbidden_bind
        && !forbidden_legacy;

    let routed_events =
        json_u64(&shadow, &["replay", "routed_events"]).unwrap_or_default() as usize;
    let exact_cache_hits_in_routed_events =
        json_u64(&shadow, &["replay", "exact_cache_hits_in_routed_events"]).unwrap_or_default()
            as usize;
    let unique_cpu_accepts_over_exact_cache =
        json_u64(&shadow, &["replay", "unique_cpu_accepts_over_exact_cache"]).unwrap_or_default()
            as usize;
    let false_accepts =
        json_u64(&shadow, &["replay", "false_accepts"]).unwrap_or(usize::MAX as u64) as usize;
    let wrong_wins =
        json_u64(&shadow, &["replay", "wrong_wins"]).unwrap_or(usize::MAX as u64) as usize;
    let p99_latency_ns =
        json_u64(&shadow, &["replay", "latency_p99_ns"]).unwrap_or_default() as u128;
    let nando_cpu_tokens_saved =
        json_u64(&shadow, &["economics", "nando_cpu_tokens_saved"]).unwrap_or_default() as usize;
    let nando_cpu_cost_saved_microusd =
        json_u64(&shadow, &["economics", "nando_cpu_cost_saved_microusd"]).unwrap_or_default();
    let estimated_nando_cpu_cost_saved_microusd = json_u64(
        &shadow,
        &["economics", "estimated_nando_cpu_cost_saved_microusd"],
    )
    .unwrap_or_default();
    let projected_nando_calls_saved_milli =
        json_u64(&shadow, &["economics", "projected_nando_calls_saved_milli"]).unwrap_or_default()
            as usize;
    let projected_combined_calls_saved_milli = json_u64(
        &shadow,
        &["economics", "projected_combined_calls_saved_milli"],
    )
    .unwrap_or_default() as usize;
    let provider_cost_evidence_present =
        json_bool(&shadow, &["economics", "provider_cost_evidence_present"]).unwrap_or(false);
    let explicit_model_price_estimate_used = json_bool(
        &shadow,
        &["economics", "explicit_model_price_estimate_used"],
    )
    .unwrap_or(false);

    let live_accept_eligible = admission_candidate_allowed
        && shadow_replay_allowed
        && shadow_append_or_future_only
        && shadow_market_savings_count_allowed
        && verifier_bound_profile_loaded
        && package_matches_admission_report
        && package_matches_shadow_report
        && routed_events > 0
        && unique_cpu_accepts_over_exact_cache > 0
        && false_accepts == 0
        && wrong_wins == 0
        && nando_cpu_tokens_saved > 0
        && admission_local_accept_disabled
        && shadow_local_accept_disabled
        && forbidden_flags_clear;

    let rejection_reason = if live_accept_eligible {
        "accepted_for_live_admission_manifest_runtime_accept_still_disabled".to_owned()
    } else if !admission_candidate_allowed {
        "serving_admission_candidate_not_allowed".to_owned()
    } else if !shadow_replay_allowed {
        "shadow_replay_not_allowed".to_owned()
    } else if !shadow_append_or_future_only {
        "shadow_replay_not_append_or_future_only".to_owned()
    } else if !shadow_market_savings_count_allowed {
        "shadow_market_savings_count_not_allowed".to_owned()
    } else if !verifier_bound_profile_loaded {
        "verifier_bound_profile_not_loaded".to_owned()
    } else if !package_matches_admission_report {
        "package_mismatch_with_admission_report".to_owned()
    } else if !package_matches_shadow_report {
        "package_mismatch_with_shadow_report".to_owned()
    } else if false_accepts > 0 {
        "false_accepts_detected".to_owned()
    } else if wrong_wins > 0 {
        "wrong_wins_detected".to_owned()
    } else if unique_cpu_accepts_over_exact_cache == 0 {
        "no_unique_cpu_accepts_over_exact_cache".to_owned()
    } else if !admission_local_accept_disabled || !shadow_local_accept_disabled {
        "upstream_report_already_enabled_local_accept_or_promotion".to_owned()
    } else if !forbidden_flags_clear {
        "forbidden_flag_detected".to_owned()
    } else {
        "live_admission_manifest_gate_failed".to_owned()
    };

    let report = PhaseAtomLiveAdmissionManifestReport {
        report_kind: "phase_atom_live_admission_manifest_v1",
        mode: "live_accept_eligibility_audit_manifest_only",
        admission_report_path: admission_report_path.display().to_string(),
        shadow_replay_report_path: shadow_replay_report_path.display().to_string(),
        action_family,
        candidate_package_path: candidate_package_path.display().to_string(),
        margin_threshold_micro,
        package: PhaseAtomLiveAdmissionPackageReport {
            package_kind: "phase_center_nwpc",
            package_magic_ok: true,
            package_fingerprint64: package_info.fingerprint64,
            package_bytes: package_bytes.len(),
            inspected_cells: package_info.cells,
            inspected_record_count: package_info.record_count,
            admission_package_fingerprint64,
            shadow_package_fingerprint64,
            package_matches_admission_report,
            package_matches_shadow_report,
        },
        evidence_gate: PhaseAtomLiveAdmissionEvidenceGate {
            admission_candidate_allowed,
            shadow_replay_allowed,
            shadow_append_or_future_only,
            shadow_market_savings_count_allowed,
            verifier_bound_profile_loaded,
            routed_events,
            exact_cache_hits_in_routed_events,
            unique_cpu_accepts_over_exact_cache,
            projected_nando_calls_saved_milli,
            projected_combined_calls_saved_milli,
            nando_cpu_tokens_saved,
            provider_cost_evidence_present,
            estimated_cost_evidence_present: estimated_nando_cpu_cost_saved_microusd > 0,
            false_accepts,
            wrong_wins,
            p99_latency_ns,
            admission_local_accept_disabled,
            shadow_local_accept_disabled,
            forbidden_flags_clear,
        },
        economics: PhaseAtomRunCheckTimeSplitEconomicsAudit {
            token_evidence_present: nando_cpu_tokens_saved > 0,
            provider_cost_evidence_present,
            explicit_model_price_estimate_used,
            price_config_schema_version: json_string(
                &shadow,
                &["economics", "price_config_schema_version"],
            )
            .unwrap_or_default(),
            provider: json_string(&shadow, &["economics", "provider"]).unwrap_or_default(),
            model_id: json_string(&shadow, &["economics", "model_id"]).unwrap_or_default(),
            price_source: json_string(&shadow, &["economics", "price_source"]).unwrap_or_default(),
            nando_cpu_tokens_saved,
            nando_cpu_cost_saved_microusd,
            estimated_nando_cpu_cost_saved_microusd,
            estimated_cost_method: json_string(&shadow, &["economics", "estimated_cost_method"])
                .unwrap_or_default(),
            projected_nando_calls_saved_milli,
            projected_combined_calls_saved_milli,
            money_claim_blocker: "live admission manifest does not grant market money claim; provider billing evidence plus product live/shadow deployment evidence required".to_owned(),
        },
        forbidden_flags: ForbiddenFlags {
            target_id_used: forbidden_target,
            proof_rule_id_authority_used: forbidden_proof,
            concrete_x_lookup_used: forbidden_lookup,
            manual_local_out_t_used: forbidden_local_out_t,
            hidden_frame_id_or_bind_x_used: forbidden_bind,
            legacy_backend_used: forbidden_legacy,
        },
        live_accept_eligible,
        live_accept_recommendation: if live_accept_eligible {
            "eligible_for_next_daemon_admission_step_only"
        } else {
            "not_eligible"
        },
        product_promotion_allowed: false,
        local_accept_enabled: false,
        promoted: false,
        serving_profile_artifact: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        rejection_reason,
        boundary: "live admission manifest only: combines verifier-bound serving admission with fresh append/future .nwpc shadow replay and may mark a profile eligible for the next daemon admission step; it does not enable local_accept, promote runtime, write serving profiles, or allow market money claims",
    };
    write_json_file(&manifest_report_path, &report)?;
    println!("phase_atom_live_admission_manifest_v1:");
    println!("  report_path: {}", manifest_report_path.display());
    println!("  action_family: {}", report.action_family);
    println!("  live_accept_eligible: {}", report.live_accept_eligible);
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.evidence_gate.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.evidence_gate.false_accepts);
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    println!("  rejection_reason: {}", report.rejection_reason);
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_live_admission_policy_smoke_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let manifest_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_LIVE_ADMISSION_MANIFEST_REPORT));
    let policy_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_LIVE_ADMISSION_POLICY_SMOKE_REPORT));

    let manifest = read_json_value(&manifest_report_path)?;
    let action_family = json_string(&manifest, &["action_family"]).ok_or_else(|| {
        format!(
            "live admission policy smoke manifest '{}' missing action_family",
            manifest_report_path.display()
        )
    })?;
    let candidate_package_path = json_string(&manifest, &["candidate_package_path"])
        .map(PathBuf::from)
        .ok_or_else(|| {
            format!(
                "live admission policy smoke manifest '{}' missing candidate_package_path",
                manifest_report_path.display()
            )
        })?;
    let package_bytes = std::fs::read(&candidate_package_path).map_err(|error| {
        format!(
            "failed to read live admission policy candidate package '{}': {error}",
            candidate_package_path.display()
        )
    })?;
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes)
        .map_err(|error| format!("live admission policy package inspect error: {error:?}"))?;
    let manifest_package_fingerprint64 =
        json_u64(&manifest, &["package", "package_fingerprint64"]).unwrap_or_default();
    let manifest_package_bytes =
        json_u64(&manifest, &["package", "package_bytes"]).unwrap_or_default() as usize;
    let manifest_package_records =
        json_u64(&manifest, &["package", "inspected_record_count"]).unwrap_or_default() as usize;
    let package_file_matches_manifest = package_info.fingerprint64
        == manifest_package_fingerprint64
        && package_bytes.len() == manifest_package_bytes
        && package_info.record_count == manifest_package_records;

    let live_accept_eligible_from_manifest =
        json_bool(&manifest, &["live_accept_eligible"]).unwrap_or(false);
    let verifier_bound_profile_loaded = json_bool(
        &manifest,
        &["evidence_gate", "verifier_bound_profile_loaded"],
    )
    .unwrap_or(false);
    let package_matches_admission_report =
        json_bool(&manifest, &["package", "package_matches_admission_report"]).unwrap_or(false);
    let package_matches_shadow_report =
        json_bool(&manifest, &["package", "package_matches_shadow_report"]).unwrap_or(false);
    let unique_cpu_accepts_over_exact_cache = json_u64(
        &manifest,
        &["evidence_gate", "unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or_default() as usize;
    let nando_cpu_tokens_saved =
        json_u64(&manifest, &["economics", "nando_cpu_tokens_saved"]).unwrap_or_default() as usize;
    let estimated_nando_cpu_cost_saved_microusd = json_u64(
        &manifest,
        &["economics", "estimated_nando_cpu_cost_saved_microusd"],
    )
    .unwrap_or_default();
    let false_accepts = json_u64(&manifest, &["evidence_gate", "false_accepts"])
        .unwrap_or(usize::MAX as u64) as usize;
    let wrong_wins =
        json_u64(&manifest, &["evidence_gate", "wrong_wins"]).unwrap_or(usize::MAX as u64) as usize;
    let p99_latency_ns =
        json_u64(&manifest, &["evidence_gate", "p99_latency_ns"]).unwrap_or_default() as u128;
    let provider_cost_evidence_present =
        json_bool(&manifest, &["economics", "provider_cost_evidence_present"]).unwrap_or(false);
    let estimated_cost_evidence_present = estimated_nando_cpu_cost_saved_microusd > 0
        || json_bool(
            &manifest,
            &["evidence_gate", "estimated_cost_evidence_present"],
        )
        .unwrap_or(false);

    let forbidden_target =
        json_bool(&manifest, &["forbidden_flags", "target_id_used"]).unwrap_or(true);
    let forbidden_proof = json_bool(
        &manifest,
        &["forbidden_flags", "proof_rule_id_authority_used"],
    )
    .unwrap_or(true);
    let forbidden_lookup =
        json_bool(&manifest, &["forbidden_flags", "concrete_x_lookup_used"]).unwrap_or(true);
    let forbidden_local_out_t =
        json_bool(&manifest, &["forbidden_flags", "manual_local_out_t_used"]).unwrap_or(true);
    let forbidden_bind = json_bool(
        &manifest,
        &["forbidden_flags", "hidden_frame_id_or_bind_x_used"],
    )
    .unwrap_or(true);
    let forbidden_legacy =
        json_bool(&manifest, &["forbidden_flags", "legacy_backend_used"]).unwrap_or(true);
    let forbidden_flags_clear = !forbidden_target
        && !forbidden_proof
        && !forbidden_lookup
        && !forbidden_local_out_t
        && !forbidden_bind
        && !forbidden_legacy;

    let local_accept_stays_disabled = !json_bool(&manifest, &["local_accept_enabled"])
        .unwrap_or(true)
        && !json_bool(&manifest, &["promoted"]).unwrap_or(true)
        && !json_bool(&manifest, &["serving_profile_artifact"]).unwrap_or(true);
    let product_promotion_stays_disabled = !json_bool(&manifest, &["product_promotion_allowed"])
        .unwrap_or(true)
        && !json_bool(&manifest, &["market_money_claim_allowed"]).unwrap_or(true);
    let runtime_stays_unchanged = !json_bool(&manifest, &["product_runtime_changed"])
        .unwrap_or(true)
        && !json_bool(&manifest, &["serving_runtime_changed"]).unwrap_or(true);
    let provider_cost_missing_blocks_money_claim = !provider_cost_evidence_present
        && !json_bool(&manifest, &["market_money_claim_allowed"]).unwrap_or(true);

    let guard = PhaseAtomLiveAdmissionPolicyGuard {
        manifest_live_accept_eligible: live_accept_eligible_from_manifest,
        package_file_matches_manifest,
        verifier_bound_profile_loaded,
        package_matches_admission_report,
        package_matches_shadow_report,
        false_accepts_zero: false_accepts == 0,
        wrong_wins_zero: wrong_wins == 0,
        unique_cpu_accepts_positive: unique_cpu_accepts_over_exact_cache > 0,
        tokens_saved_positive: nando_cpu_tokens_saved > 0,
        provider_cost_missing_blocks_money_claim,
        local_accept_stays_disabled,
        product_promotion_stays_disabled,
        runtime_stays_unchanged,
        forbidden_flags_clear,
    };
    let would_admit_shadow_only = guard.manifest_live_accept_eligible
        && guard.package_file_matches_manifest
        && guard.verifier_bound_profile_loaded
        && guard.package_matches_admission_report
        && guard.package_matches_shadow_report
        && guard.false_accepts_zero
        && guard.wrong_wins_zero
        && guard.unique_cpu_accepts_positive
        && guard.tokens_saved_positive
        && guard.provider_cost_missing_blocks_money_claim
        && guard.local_accept_stays_disabled
        && guard.product_promotion_stays_disabled
        && guard.runtime_stays_unchanged
        && guard.forbidden_flags_clear;

    let rejection_reason = if would_admit_shadow_only {
        "accepted_for_policy_smoke_shadow_only_product_accept_still_disabled".to_owned()
    } else if !guard.manifest_live_accept_eligible {
        "manifest_not_live_accept_eligible".to_owned()
    } else if !guard.package_file_matches_manifest {
        "package_file_mismatch_with_manifest".to_owned()
    } else if !guard.verifier_bound_profile_loaded {
        "verifier_bound_profile_not_loaded".to_owned()
    } else if !guard.package_matches_admission_report {
        "manifest_package_mismatch_with_admission_report".to_owned()
    } else if !guard.package_matches_shadow_report {
        "manifest_package_mismatch_with_shadow_report".to_owned()
    } else if !guard.false_accepts_zero {
        "false_accepts_detected".to_owned()
    } else if !guard.wrong_wins_zero {
        "wrong_wins_detected".to_owned()
    } else if !guard.unique_cpu_accepts_positive {
        "no_unique_cpu_accepts_over_exact_cache".to_owned()
    } else if !guard.tokens_saved_positive {
        "no_tokens_saved_evidence".to_owned()
    } else if !guard.local_accept_stays_disabled
        || !guard.product_promotion_stays_disabled
        || !guard.runtime_stays_unchanged
    {
        "manifest_already_mutates_product_or_runtime_state".to_owned()
    } else if !guard.forbidden_flags_clear {
        "forbidden_flag_detected".to_owned()
    } else {
        "live_admission_policy_smoke_gate_failed".to_owned()
    };

    let report = PhaseAtomLiveAdmissionPolicySmokeReport {
        report_kind: "phase_atom_live_admission_policy_smoke_v1",
        mode: "daemon_admission_policy_smoke_no_runtime_mutation",
        manifest_report_path: manifest_report_path.display().to_string(),
        action_family,
        candidate_package_path: candidate_package_path.display().to_string(),
        package_fingerprint64: package_info.fingerprint64,
        package_file_matches_manifest,
        live_accept_eligible_from_manifest,
        policy_decision: if would_admit_shadow_only {
            "would_admit_shadow_only"
        } else {
            "reject"
        },
        would_local_accepts_over_exact_cache: if would_admit_shadow_only {
            unique_cpu_accepts_over_exact_cache
        } else {
            0
        },
        would_tokens_saved: if would_admit_shadow_only {
            nando_cpu_tokens_saved
        } else {
            0
        },
        would_estimated_cost_saved_microusd: if would_admit_shadow_only {
            estimated_nando_cpu_cost_saved_microusd
        } else {
            0
        },
        false_accepts,
        wrong_wins,
        p99_latency_ns,
        provider_cost_evidence_present,
        estimated_cost_evidence_present,
        market_money_claim_allowed: false,
        local_accept_enabled: false,
        serving_runtime_changed: false,
        product_runtime_changed: false,
        promoted: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: forbidden_target,
            proof_rule_id_authority_used: forbidden_proof,
            concrete_x_lookup_used: forbidden_lookup,
            manual_local_out_t_used: forbidden_local_out_t,
            hidden_frame_id_or_bind_x_used: forbidden_bind,
            legacy_backend_used: forbidden_legacy,
        },
        policy_accept_guard: guard,
        rejection_reason,
        boundary: "policy smoke only: consumes the live-admission manifest and reports a shadow-only daemon admission decision; it does not enable local_accept, mutate product/serving runtime, promote packages, or allow market money claims",
    };
    write_json_file(&policy_report_path, &report)?;
    println!("phase_atom_live_admission_policy_smoke_v1:");
    println!("  report_path: {}", policy_report_path.display());
    println!("  action_family: {}", report.action_family);
    println!("  policy_decision: {}", report.policy_decision);
    println!(
        "  would_local_accepts_over_exact_cache: {}",
        report.would_local_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.false_accepts);
    println!("  wrong_wins: {}", report.wrong_wins);
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    println!("  rejection_reason: {}", report.rejection_reason);
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_live_daemon_shadow_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let manifest_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_LIVE_ADMISSION_POLICY_SMOKE_REPORT));
    let live_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSION_TOOL_STATUS_APPEND_LATEST_JSONL));
    let decision_log_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_LIVE_DAEMON_SHADOW_DECISION_LOG));
    let gate_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_LIVE_DAEMON_SHADOW_GATE_REPORT));
    let exact_cache_watermark_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSION_TOOL_STATUS_VERIFIER_JSONL));
    let process_rss_kib_before_load = current_process_rss_kib();
    let price_config = read_json_file::<ModelPriceConfig>(Path::new(DEFAULT_PRICE_CONFIG))?;
    let manifest = read_json_value(&manifest_report_path)?;

    let action_family = json_string(&manifest, &["action_family"]).ok_or_else(|| {
        format!(
            "live daemon shadow manifest '{}' missing action_family",
            manifest_report_path.display()
        )
    })?;
    if !action_family.starts_with("action_family:") {
        return Err(format!(
            "live daemon shadow action_family must start with action_family:, got '{action_family}'"
        ));
    }
    let task_name = action_family
        .strip_prefix("action_family:")
        .unwrap_or(action_family.as_str())
        .replace(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_', "_");
    let candidate_package_path = json_string(&manifest, &["candidate_package_path"])
        .map(PathBuf::from)
        .ok_or_else(|| {
            format!(
                "live daemon shadow manifest '{}' missing candidate_package_path",
                manifest_report_path.display()
            )
        })?;
    let package_bytes = std::fs::read(&candidate_package_path).map_err(|error| {
        format!(
            "failed to read live daemon candidate package '{}': {error}",
            candidate_package_path.display()
        )
    })?;
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes)
        .map_err(|error| format!("live daemon package inspect error: {error:?}"))?;
    let manifest_package_fingerprint64 = json_u64(&manifest, &["package_fingerprint64"])
        .or_else(|| json_u64(&manifest, &["package", "package_fingerprint64"]))
        .unwrap_or_default();
    let manifest_package_bytes = json_u64(&manifest, &["package", "package_bytes"])
        .unwrap_or(package_bytes.len() as u64) as usize;
    let manifest_package_records = json_u64(&manifest, &["package", "inspected_record_count"])
        .unwrap_or(package_info.record_count as u64) as usize;
    let package_file_matches_manifest = package_info.fingerprint64
        == manifest_package_fingerprint64
        && package_bytes.len() == manifest_package_bytes
        && package_info.record_count == manifest_package_records;
    let margin_threshold_micro = json_i64(&manifest, &["margin_threshold_micro"])
        .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
    let runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &package_bytes,
        PhaseCenterOffloadPolicy::new(margin_threshold_micro)
            .map_err(|error| format!("live daemon invalid policy: {error:?}"))?,
    )
    .map_err(|error| format!("live daemon package load error: {error:?}"))?;
    let runtime_bytes_estimate = runtime.bytes_estimate();
    let process_rss_kib_after_load = current_process_rss_kib();

    let manifest_live_accept_eligible =
        json_bool(&manifest, &["live_accept_eligible_from_manifest"])
            .or_else(|| json_bool(&manifest, &["live_accept_eligible"]))
            .unwrap_or(false);
    let policy_decision =
        json_string(&manifest, &["policy_decision"]).unwrap_or_else(|| "unknown".to_owned());
    let policy_shadow_only = policy_decision == "would_admit_shadow_only";

    let watermark_text =
        std::fs::read_to_string(&exact_cache_watermark_trace_path).map_err(|error| {
            format!(
                "failed to read live daemon exact-cache watermark trace '{}': {error}",
                exact_cache_watermark_trace_path.display()
            )
        })?;
    let mut cache_scope_events = Vec::<PhaseAtomBinaryEvent>::new();
    for (line_index, line) in watermark_text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse live daemon watermark trace '{}' line {}: {error}",
                exact_cache_watermark_trace_path.display(),
                line_index + 1
            )
        })?;
        if let Some(event) = parse_phase_atom_binary_event_for_action(
            &row,
            cache_scope_events.len(),
            &action_family,
            &task_name,
        ) {
            cache_scope_events.push(event);
        }
    }
    let watermark_routable_events = cache_scope_events.len();

    let live_trace_text = std::fs::read_to_string(&live_trace_path).map_err(|error| {
        format!(
            "failed to read live daemon trace '{}': {error}",
            live_trace_path.display()
        )
    })?;
    if let Some(parent) = decision_log_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    let mut decision_log = std::fs::File::create(&decision_log_path).map_err(|error| {
        format!(
            "failed to create live daemon decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;

    let mut total_rows = 0usize;
    let mut routed_events = 0usize;
    let mut decision_log_rows = 0usize;
    let mut selected_cache_indices = Vec::new();
    let mut routed_event_buffer = Vec::new();
    for (line_index, line) in live_trace_text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        total_rows += 1;
        if total_rows.is_multiple_of(1000) {
            println!("  live_daemon_trace_rows_scanned: {total_rows}");
        }
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse live daemon trace '{}' line {}: {error}",
                live_trace_path.display(),
                line_index + 1
            )
        })?;
        if let Some(event) = parse_phase_atom_binary_event_for_action(
            &row,
            routed_events,
            &action_family,
            &task_name,
        ) {
            let cache_index = cache_scope_events.len();
            cache_scope_events.push(event.clone());
            selected_cache_indices.push(cache_index);
            routed_event_buffer.push(event);
            routed_events += 1;
        }
    }

    let exact_cache_flags = exact_cache_hit_flags_phase_atom_binary(&cache_scope_events);
    let mut margins = Vec::with_capacity(routed_event_buffer.len());
    let mut latencies = Vec::with_capacity(routed_event_buffer.len());
    let mut local_operator_shadow_decisions = 0usize;
    let mut fallback_shadow_decisions = 0usize;
    let mut wrong_wins = 0usize;
    let mut false_accepts = 0usize;
    let mut exact_cache_hits_in_routed_events = 0usize;
    let mut unique_cpu_accepts_over_exact_cache = 0usize;
    let mut nando_cpu_tokens_saved = 0usize;
    let mut nando_cpu_cost_saved_microusd = 0u64;

    for (row_index, event) in routed_event_buffer.iter().enumerate() {
        if row_index > 0 && row_index % 1000 == 0 {
            println!("  live_daemon_shadow_events_scored: {row_index}");
        }
        let program_index = usize::from(!event.verified_safe_accept);
        let correct_vec = phase_atom_binary_event_vector_for_task(
            event,
            event.verified_safe_accept,
            runtime.cells(),
            &task_name,
        );
        let wrong_vec = phase_atom_binary_event_vector_for_task(
            event,
            !event.verified_safe_accept,
            runtime.cells(),
            &task_name,
        );
        let task = PhaseCenterEvalTask {
            center_index: program_index,
            correct_vec: correct_vec.into_boxed_slice(),
            wrong_vec: wrong_vec.into_boxed_slice(),
        };
        let started = Instant::now();
        let decision = runtime
            .offload_decision(&task)
            .map_err(|error| format!("live daemon offload decision error: {error:?}"))?;
        latencies.push(started.elapsed().as_nanos());
        margins.push(decision.margin_micro);
        wrong_wins += usize::from(decision.margin_micro <= 0);
        let local_operator = decision.is_local_operator();
        if local_operator {
            local_operator_shadow_decisions += 1;
            false_accepts += usize::from(decision.margin_micro <= 0);
        } else {
            fallback_shadow_decisions += 1;
        }
        let exact_hit = exact_cache_flags
            .get(selected_cache_indices[row_index])
            .copied()
            .unwrap_or(false);
        exact_cache_hits_in_routed_events += usize::from(exact_hit);
        let unique_cpu_accept_over_exact_cache = local_operator && !exact_hit;
        if unique_cpu_accept_over_exact_cache {
            unique_cpu_accepts_over_exact_cache += 1;
            nando_cpu_tokens_saved =
                nando_cpu_tokens_saved.saturating_add(event.token_cost.total_tokens);
            nando_cpu_cost_saved_microusd =
                nando_cpu_cost_saved_microusd.saturating_add(event.token_cost.total_cost_microusd);
        }
        let row = PhaseAtomLiveDaemonDecisionLogRow {
            row_index,
            event_timestamp: event.event_timestamp.clone(),
            action_family: action_family.clone(),
            package_fingerprint64: package_info.fingerprint64,
            decision: if local_operator {
                "local_operator_shadow".to_owned()
            } else {
                "fallback".to_owned()
            },
            margin_micro: decision.margin_micro,
            verified_safe_accept: event.verified_safe_accept,
            exact_cache_hit: exact_hit,
            unique_cpu_accept_over_exact_cache,
            total_tokens: event.token_cost.total_tokens,
            total_cost_microusd: event.token_cost.total_cost_microusd,
            token_evidence_missing: event.token_cost.token_evidence_missing,
            cost_evidence_missing: event.token_cost.cost_evidence_missing,
            request_fingerprint: format!("live_daemon_{}:{}", task_name, event.exact_cache_key),
            local_accept_enabled: false,
        };
        serde_json::to_writer(&mut decision_log, &row)
            .map_err(|error| format!("failed to serialize live daemon decision row: {error}"))?;
        decision_log.write_all(b"\n").map_err(|error| {
            format!(
                "failed to write live daemon decision log '{}': {error}",
                decision_log_path.display()
            )
        })?;
        decision_log_rows += 1;
    }
    decision_log.flush().map_err(|error| {
        format!(
            "failed to flush live daemon decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    let process_rss_kib_after_score = current_process_rss_kib();
    let natural_fallback_rows_observed = fallback_shadow_decisions > 0;

    let fallback_probe = if let Some(event) = routed_event_buffer.first() {
        let correct_vec = phase_atom_binary_event_vector_for_task(
            event,
            event.verified_safe_accept,
            runtime.cells(),
            &task_name,
        );
        let wrong_vec = phase_atom_binary_event_vector_for_task(
            event,
            !event.verified_safe_accept,
            runtime.cells(),
            &task_name,
        );
        let probe_task = PhaseCenterEvalTask {
            center_index: usize::from(!event.verified_safe_accept),
            correct_vec: wrong_vec.into_boxed_slice(),
            wrong_vec: correct_vec.into_boxed_slice(),
        };
        let decision = runtime
            .offload_decision(&probe_task)
            .map_err(|error| format!("live daemon fallback probe decision error: {error:?}"))?;
        PhaseAtomLiveDaemonFallbackProbe {
            explicit_probe_ran: true,
            probe_kind: "synthetic_reversed_vector_probe",
            probe_decision: if decision.is_local_operator() {
                "local_operator_shadow".to_owned()
            } else {
                "fallback".to_owned()
            },
            probe_margin_micro: decision.margin_micro,
            probe_fell_back: !decision.is_local_operator(),
            natural_fallback_rows_observed,
        }
    } else {
        PhaseAtomLiveDaemonFallbackProbe {
            explicit_probe_ran: false,
            probe_kind: "not_run",
            probe_decision: "not_run".to_owned(),
            probe_margin_micro: 0,
            probe_fell_back: false,
            natural_fallback_rows_observed,
        }
    };

    margins.sort_unstable();
    latencies.sort_unstable();
    let token_evidence_present = nando_cpu_tokens_saved > 0;
    let provider_cost_evidence_present = nando_cpu_cost_saved_microusd > 0;
    let estimated_nando_cpu_cost_saved_microusd =
        if token_evidence_present && !provider_cost_evidence_present {
            estimated_event_cost_microusd(nando_cpu_tokens_saved, 0, &price_config)
        } else {
            0
        };
    let explicit_model_price_estimate_used = estimated_nando_cpu_cost_saved_microusd > 0;
    let estimated_cost_method = if explicit_model_price_estimate_used {
        "total_saved_tokens_as_input_token_floor_from_model_price_config".to_owned()
    } else if provider_cost_evidence_present {
        "provider_cost_evidence_present_no_estimate_needed".to_owned()
    } else {
        "no_token_or_price_estimate_available".to_owned()
    };

    let forbidden_flags = ForbiddenFlags {
        target_id_used: false,
        proof_rule_id_authority_used: false,
        concrete_x_lookup_used: false,
        manual_local_out_t_used: false,
        hidden_frame_id_or_bind_x_used: false,
        legacy_backend_used: false,
    };
    let live_daemon_shadow_gate_passed = package_file_matches_manifest
        && manifest_live_accept_eligible
        && policy_shadow_only
        && routed_events > 0
        && decision_log_rows == routed_events
        && local_operator_shadow_decisions > 0
        && unique_cpu_accepts_over_exact_cache > 0
        && false_accepts == 0
        && wrong_wins == 0
        && fallback_probe.explicit_probe_ran
        && fallback_probe.probe_fell_back;
    let rejection_reason = if live_daemon_shadow_gate_passed {
        "accepted_for_live_daemon_shadow_gate_product_accept_still_disabled".to_owned()
    } else if !package_file_matches_manifest {
        "package_file_mismatch_with_manifest".to_owned()
    } else if !manifest_live_accept_eligible {
        "manifest_not_live_accept_eligible".to_owned()
    } else if !policy_shadow_only {
        "policy_decision_not_shadow_only_admit".to_owned()
    } else if routed_events == 0 {
        "no_live_trace_events_matched_profile".to_owned()
    } else if decision_log_rows != routed_events {
        "decision_log_row_count_mismatch".to_owned()
    } else if false_accepts > 0 {
        "live_daemon_false_accepts_detected".to_owned()
    } else if wrong_wins > 0 {
        "live_daemon_wrong_wins_detected".to_owned()
    } else if unique_cpu_accepts_over_exact_cache == 0 {
        "no_unique_cpu_accepts_over_exact_cache".to_owned()
    } else if !fallback_probe.probe_fell_back {
        "fallback_probe_did_not_fallback".to_owned()
    } else {
        "live_daemon_shadow_gate_failed".to_owned()
    };

    let report = PhaseAtomLiveDaemonShadowGateReport {
        report_kind: "phase_atom_live_daemon_shadow_gate_v1",
        mode: "live_daemon_shadow_gate_no_product_accept",
        coverage_scope: action_family.clone(),
        manifest_report_path: manifest_report_path.display().to_string(),
        live_trace_path: live_trace_path.display().to_string(),
        exact_cache_watermark_trace_path: exact_cache_watermark_trace_path.display().to_string(),
        decision_log_path: decision_log_path.display().to_string(),
        profile: PhaseAtomLiveDaemonShadowProfileReport {
            action_family,
            task_name,
            candidate_package_path: candidate_package_path.display().to_string(),
            package_fingerprint64: package_info.fingerprint64,
            package_bytes: package_bytes.len(),
            package_record_count: package_info.record_count,
            package_file_matches_manifest,
            manifest_live_accept_eligible,
            policy_decision,
            runtime_cells: runtime.cells(),
            runtime_record_count: runtime.record_count(),
            runtime_bytes_estimate,
        },
        audit: PhaseAtomLiveDaemonShadowAudit {
            total_rows,
            watermark_routable_events,
            routed_events,
            unrouted_events: total_rows.saturating_sub(routed_events),
            decision_log_rows,
            local_operator_shadow_decisions,
            fallback_shadow_decisions,
            natural_fallback_rows_observed,
            exact_cache_hits_in_routed_events,
            unique_cpu_accepts_over_exact_cache,
            nando_cpu_tokens_saved,
            nando_cpu_cost_saved_microusd,
            false_accepts,
            wrong_wins,
            min_margin_micro: margins.first().copied().unwrap_or(0),
            p10_margin_micro: percentile_i64(&margins, 10),
            median_margin_micro: percentile_i64(&margins, 50),
            latency_p50_ns: percentile_u128(&latencies, 50),
            latency_p90_ns: percentile_u128(&latencies, 90),
            latency_p99_ns: percentile_u128(&latencies, 99),
            latency_max_ns: latencies.last().copied().unwrap_or(0),
            process_rss_kib_before_load,
            process_rss_kib_after_load,
            process_rss_kib_after_score,
        },
        fallback_probe,
        economics: PhaseAtomRunCheckTimeSplitEconomicsAudit {
            token_evidence_present,
            provider_cost_evidence_present,
            explicit_model_price_estimate_used,
            price_config_schema_version: price_config.schema_version,
            provider: price_config.default_provider,
            model_id: price_config.default_model_id,
            price_source: price_config.price_source,
            nando_cpu_tokens_saved,
            nando_cpu_cost_saved_microusd,
            estimated_nando_cpu_cost_saved_microusd,
            estimated_cost_method,
            projected_nando_calls_saved_milli: per_thousand(
                unique_cpu_accepts_over_exact_cache,
                routed_events,
            ),
            projected_combined_calls_saved_milli: per_thousand(
                exact_cache_hits_in_routed_events + unique_cpu_accepts_over_exact_cache,
                routed_events,
            ),
            money_claim_blocker: "live daemon shadow gate still keeps product local_accept disabled; market money claim requires provider billing evidence and product shadow/live deployment policy".to_owned(),
        },
        forbidden_flags,
        live_daemon_shadow_gate_passed,
        product_promotion_allowed: false,
        local_accept_enabled: false,
        promoted: false,
        serving_profile_artifact: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        rejection_reason,
        boundary: "live daemon shadow gate only: loads the exact .nwpc fingerprint from the live-admission policy manifest, writes a decision log for live trace rows, joins verifier labels for audit, and checks an explicit fallback probe; it does not enable product local_accept, mutate serving runtime, promote packages, allow market money claims, or use legacy nwrb/role-binding paths",
    };
    write_json_file(&gate_report_path, &report)?;
    println!("phase_atom_live_daemon_shadow_gate_v1:");
    println!("  report_path: {}", gate_report_path.display());
    println!("  decision_log_path: {}", decision_log_path.display());
    println!(
        "  live_daemon_shadow_gate_passed: {}",
        report.live_daemon_shadow_gate_passed
    );
    println!("  routed_events: {}", report.audit.routed_events);
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.audit.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.audit.false_accepts);
    println!("  wrong_wins: {}", report.audit.wrong_wins);
    println!(
        "  fallback_probe_fell_back: {}",
        report.fallback_probe.probe_fell_back
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    println!("  rejection_reason: {}", report.rejection_reason);
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_live_self_mining_loop_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_LIVE_SELF_MINING_REPORT));
    let candidate_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_LIVE_SELF_MINING_CANDIDATE_DIR));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CELLS);
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    let min_class_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid min_class_events '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PHASE_ATOM_LIVE_SELF_MINING_MIN_EVENTS);
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin threshold '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
    if margin_threshold_micro <= 0 {
        return Err("margin threshold must be > 0".to_owned());
    }
    let train_permille = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid train_permille '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_TRAIN_PERMILLE);
    if !(1..=999).contains(&train_permille) {
        return Err("train_permille must be in 1..=999".to_owned());
    }
    let top_n = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid top_n '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PHASE_ATOM_LIVE_SELF_MINING_TOP_N);
    let price_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PRICE_CONFIG));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(
                DEFAULT_CODEX_SESSION_TOOL_STATUS_APPEND_LATEST_JSONL,
            )]
        } else {
            rest
        }
    };
    let process_rss_kib_before = current_process_rss_kib();
    let price_config = read_json_file::<ModelPriceConfig>(&price_config_path)?;

    let mut total_rows = 0usize;
    let mut parsed_verifier_events = 0usize;
    let mut events_by_action_family = BTreeMap::<String, Vec<PhaseAtomBinaryEvent>>::new();
    let mut events_by_base_action_family = BTreeMap::<String, Vec<PhaseAtomBinaryEvent>>::new();
    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read live self-mining phase atom trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows += 1;
            if total_rows.is_multiple_of(1000) {
                println!("  live_self_mining_rows_scanned: {total_rows}");
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse live self-mining trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            let action_atoms = phase_atom_string_vec(&row, "action_atoms");
            let request_atoms = phase_atom_string_vec(&row, "request_atoms");
            let state_atoms = phase_atom_string_vec(&row, "state_atoms");
            let tool_atoms = phase_atom_string_vec(&row, "tool_atoms");
            let route_hint_atoms = phase_atom_string_vec(&row, "route_hint_atoms");
            for action_family in phase_atom_action_families(&action_atoms) {
                let bucket_key = phase_atom_state_action_bucket_key(
                    &action_family,
                    &request_atoms,
                    &state_atoms,
                    &tool_atoms,
                    &route_hint_atoms,
                );
                let task_name = phase_atom_live_self_mining_task_name(&bucket_key);
                if let Some(event) = parse_phase_atom_binary_event_for_action(
                    &row,
                    parsed_verifier_events,
                    &action_family,
                    &task_name,
                ) {
                    events_by_base_action_family
                        .entry(phase_atom_base_action_family(&action_family).to_owned())
                        .or_default()
                        .push(event.clone());
                    events_by_action_family
                        .entry(bucket_key)
                        .or_default()
                        .push(event);
                    parsed_verifier_events += 1;
                }
            }
        }
    }

    let mut ranked = events_by_action_family
        .iter()
        .map(|(action_family, events)| {
            (
                action_family.clone(),
                phase_atom_live_self_mining_value_score(events, min_class_events),
            )
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let max_compiled_per_base_action_family = (top_n / 4).max(2);
    let mut selected_base_counts = BTreeMap::<String, usize>::new();
    let mut selected_for_compile = BTreeSet::<String>::new();
    for (action_family, _) in ranked.iter().filter(|(_, score)| *score > 0) {
        let base_action_family = phase_atom_base_action_family(action_family);
        let base_count = selected_base_counts
            .get(base_action_family)
            .copied()
            .unwrap_or(0);
        if base_count >= max_compiled_per_base_action_family {
            continue;
        }
        selected_for_compile.insert(action_family.clone());
        selected_base_counts.insert(base_action_family.to_owned(), base_count + 1);
        if selected_for_compile.len() >= top_n {
            break;
        }
    }
    println!(
        "  live_self_mining_ranked_classes: {} selected_for_compile={} selected_base_families={} max_per_base_family={}",
        ranked.len(),
        selected_for_compile.len(),
        selected_base_counts.len(),
        max_compiled_per_base_action_family
    );

    let mut classes = Vec::new();
    let ranked_len = ranked.len();
    for (rank_index, (action_family, _)) in ranked.into_iter().enumerate() {
        let events = events_by_action_family
            .get(&action_family)
            .expect("ranked action family exists");
        let base_background_events = events_by_base_action_family
            .get(phase_atom_base_action_family(&action_family))
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let compile_candidate = selected_for_compile.contains(&action_family);
        if compile_candidate {
            println!(
                "  live_self_mining_class_selected: rank={} action_family={} events={}",
                rank_index + 1,
                action_family,
                events.len()
            );
        } else if (rank_index + 1).is_multiple_of(250) {
            println!(
                "  live_self_mining_classes_scanned: {}/{}",
                rank_index + 1,
                ranked_len
            );
        }
        let class_report = phase_atom_live_self_mining_class_report(
            &action_family,
            events,
            compile_candidate,
            PhaseAtomLiveSelfMiningClassConfig {
                candidate_dir: &candidate_dir,
                cells,
                min_class_events,
                margin_threshold_micro,
                train_permille,
                base_background_events,
            },
        )?;
        classes.push(class_report);
    }

    let high_value_classes = classes
        .iter()
        .filter(|class| class.high_value_candidate)
        .count();
    let compiled_quarantine_candidates = classes
        .iter()
        .filter(|class| class.compiled_quarantine_candidate)
        .count();
    let shadow_accepted_candidates = classes
        .iter()
        .filter(|class| class.accepted_for_shadow_review)
        .count();
    let aggregate_heldout_local_operator_calls = classes
        .iter()
        .filter(|class| class.accepted_for_shadow_review)
        .map(|class| class.heldout_local_operator_calls)
        .sum::<usize>();
    let aggregate_heldout_fallback_calls = classes
        .iter()
        .filter(|class| class.accepted_for_shadow_review)
        .map(|class| class.heldout_fallback_calls)
        .sum::<usize>();
    let natural_fallback_rows_observed = aggregate_heldout_fallback_calls > 0;
    let aggregate_unique_cpu_accepts_over_exact_cache = classes
        .iter()
        .filter(|class| class.accepted_for_shadow_review)
        .map(|class| class.unique_cpu_accepts_over_exact_cache)
        .sum::<usize>();
    let aggregate_nando_cpu_tokens_saved = classes
        .iter()
        .filter(|class| class.accepted_for_shadow_review)
        .map(|class| class.nando_cpu_tokens_saved)
        .sum::<usize>();
    let aggregate_nando_cpu_cost_saved_microusd = classes
        .iter()
        .filter(|class| class.accepted_for_shadow_review)
        .map(|class| class.nando_cpu_cost_saved_microusd)
        .fold(0u64, u64::saturating_add);
    let token_evidence_present = aggregate_nando_cpu_tokens_saved > 0;
    let provider_cost_evidence_present = aggregate_nando_cpu_cost_saved_microusd > 0;
    let estimated_nando_cpu_cost_saved_microusd =
        if token_evidence_present && !provider_cost_evidence_present {
            estimated_event_cost_microusd(aggregate_nando_cpu_tokens_saved, 0, &price_config)
        } else {
            0
        };
    let explicit_model_price_estimate_used = estimated_nando_cpu_cost_saved_microusd > 0;
    let estimated_cost_method = if explicit_model_price_estimate_used {
        "aggregate_saved_tokens_as_input_token_floor_from_model_price_config".to_owned()
    } else if provider_cost_evidence_present {
        "provider_cost_evidence_present_no_estimate_needed".to_owned()
    } else {
        "no_token_or_price_estimate_available".to_owned()
    };
    let process_rss_kib_after = current_process_rss_kib();
    let report = PhaseAtomLiveSelfMiningLoopReport {
        report_kind: "phase_atom_live_self_mining_loop_v1",
        mode: "live_trace_class_stats_to_quarantine_nwpc_candidates_shadow_only",
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        candidate_dir: candidate_dir.display().to_string(),
        cells,
        min_class_events,
        top_n,
        selection_policy: "value_score_with_base_action_family_cap",
        max_compiled_per_base_action_family,
        margin_threshold_micro,
        train_permille,
        total_rows,
        parsed_verifier_events,
        action_families_seen: events_by_action_family.len(),
        high_value_classes,
        compiled_quarantine_candidates,
        selected_base_action_families: selected_base_counts.len(),
        shadow_accepted_candidates,
        aggregate_heldout_local_operator_calls,
        aggregate_heldout_fallback_calls,
        natural_fallback_rows_observed,
        aggregate_unique_cpu_accepts_over_exact_cache,
        aggregate_nando_cpu_tokens_saved,
        aggregate_nando_cpu_cost_saved_microusd,
        process_rss_kib_before,
        process_rss_kib_after,
        classes,
        economics: PhaseAtomRunCheckTimeSplitEconomicsAudit {
            token_evidence_present,
            provider_cost_evidence_present,
            explicit_model_price_estimate_used,
            price_config_schema_version: price_config.schema_version,
            provider: price_config.default_provider,
            model_id: price_config.default_model_id,
            price_source: price_config.price_source,
            nando_cpu_tokens_saved: aggregate_nando_cpu_tokens_saved,
            nando_cpu_cost_saved_microusd: aggregate_nando_cpu_cost_saved_microusd,
            estimated_nando_cpu_cost_saved_microusd,
            estimated_cost_method,
            projected_nando_calls_saved_milli: per_thousand(
                aggregate_unique_cpu_accepts_over_exact_cache,
                parsed_verifier_events,
            ),
            projected_combined_calls_saved_milli: per_thousand(
                aggregate_unique_cpu_accepts_over_exact_cache,
                parsed_verifier_events,
            ),
            money_claim_blocker: "self-mining loop is shadow/quarantine only; market money claim requires provider billing evidence and product deployment policy".to_owned(),
        },
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        online_learn_enabled: true,
        online_shadow_enabled: true,
        auto_promote_enabled: false,
        local_accept_enabled: false,
        product_promotion_allowed: false,
        promoted: false,
        serving_profile_artifact: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        boundary: "live self-mining loop only: ranks action-family traffic by frequency, token value, exact-cache overlap, and verifier availability, compiles quarantine .nwpc candidates, and shadow-scores heldout events; it does not promote, write serving profiles, enable local_accept, allow market claims, use lookup/target/proof authority, or revive legacy nwrb/role-binding paths",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_atom_live_self_mining_loop_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  candidate_dir: {}", candidate_dir.display());
    println!("  total_rows: {}", report.total_rows);
    println!("  action_families_seen: {}", report.action_families_seen);
    println!("  high_value_classes: {}", report.high_value_classes);
    println!(
        "  compiled_quarantine_candidates: {}",
        report.compiled_quarantine_candidates
    );
    println!(
        "  shadow_accepted_candidates: {}",
        report.shadow_accepted_candidates
    );
    println!(
        "  aggregate_heldout_local_operator_calls: {}",
        report.aggregate_heldout_local_operator_calls
    );
    println!(
        "  aggregate_heldout_fallback_calls: {}",
        report.aggregate_heldout_fallback_calls
    );
    println!(
        "  aggregate_unique_cpu_accepts_over_exact_cache: {}",
        report.aggregate_unique_cpu_accepts_over_exact_cache
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_online_miner_daemon_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    online_miner_daemon::run_phase_stream_online_miner_daemon_v1(args)
}

pub(crate) fn run_phase_stream_online_miner_value_pass_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    online_miner_daemon::run_phase_stream_online_miner_value_pass_v1(args)
}

pub(crate) fn run_phase_stream_online_miner_targeted_shadow_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    online_miner_daemon::run_phase_stream_online_miner_targeted_shadow_v1(args)
}

pub(crate) fn run_phase_stream_online_miner_targeted_rejection_drilldown_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    online_miner_daemon::run_phase_stream_online_miner_targeted_rejection_drilldown_v1(args)
}

pub(crate) fn run_phase_stream_online_miner_promotion_registry_gate_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    online_miner_daemon::run_phase_stream_online_miner_promotion_registry_gate_v1(args)
}

pub(crate) fn run_phase_stream_online_miner_promotion_billing_request_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    online_miner_promotion_billing_request::run_phase_stream_online_miner_promotion_billing_request_v1(args)
}

pub(crate) fn run_phase_stream_online_miner_targeted_billing_request_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    online_miner_promotion_billing_request::run_phase_stream_online_miner_targeted_billing_request_v1(args)
}

pub(crate) fn run_phase_stream_online_miner_targeted_admission_gate_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    online_miner_promotion_billing_request::run_phase_stream_online_miner_targeted_admission_gate_v1(
        args,
    )
}

pub(crate) fn run_phase_stream_online_miner_promotion_provider_capture_request_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    online_miner_promotion_billing_request::run_phase_stream_online_miner_promotion_provider_capture_request_v1(args)
}

pub(crate) fn run_phase_stream_live_store_adapter_smoke_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_live_store_adapter_smoke_v1(args)
}

pub(crate) fn run_phase_stream_live_store_clean_manifest_shadow_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_live_store_clean_manifest_shadow_v1(args)
}

pub(crate) fn run_phase_stream_live_store_prepared_hot_pack_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_live_store_prepared_hot_pack_v1(args)
}

pub(crate) fn run_phase_stream_live_worker_memory_smoke_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_live_worker_memory_smoke_v1(args)
}

pub(crate) fn run_phase_stream_live_source_adapter_worker_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_live_source_adapter_worker_v1(args)
}

pub(crate) fn run_phase_stream_live_worker_queue_smoke_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_live_worker_queue_smoke_v1(args)
}

pub(crate) fn run_phase_stream_live_worker_thread_smoke_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_live_worker_thread_smoke_v1(args)
}

pub(crate) fn run_phase_stream_live_worker_batch_thread_smoke_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_live_worker_batch_thread_smoke_v1(args)
}

pub(crate) fn run_phase_stream_live_store_direct_batch_thread_smoke_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_live_store_direct_batch_thread_smoke_v1(args)
}

pub(crate) fn run_phase_stream_hot_path_benchmark_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_hot_path_benchmark_v1(args)
}

pub(crate) fn run_phase_stream_hot_path_daemon_admission_policy_smoke_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_hot_path_daemon_admission_policy_smoke_v1(args)
}

pub(crate) fn run_phase_stream_hot_path_daemon_shadow_gate_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_hot_path_daemon_shadow_gate_v1(args)
}

pub(crate) fn run_phase_stream_hot_path_daemon_append_shadow_gate_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_hot_path_daemon_append_shadow_gate_v1(args)
}

pub(crate) fn run_phase_stream_hot_path_daemon_live_loop_budget_smoke_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_hot_path_daemon_live_loop_budget_smoke_v1(args)
}

pub(crate) fn run_phase_stream_hot_path_daemon_append_live_loop_smoke_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_hot_path_daemon_append_live_loop_smoke_v1(args)
}

pub(crate) fn run_phase_stream_hot_path_daemon_append_live_tail_v1<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_hot_path_daemon_append_live_tail_v1(args)
}

pub(crate) fn run_phase_stream_hot_path_daemon_live_loop_numeric_benchmark_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_hot_path_daemon_live_loop_numeric_benchmark_v1(args)
}

pub(crate) fn run_phase_stream_hot_path_daemon_numeric_package_shadow_audit_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_hot_path_daemon_numeric_package_shadow_audit_v1(args)
}

pub(crate) fn run_phase_stream_hot_path_daemon_numeric_future_package_audit_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_hot_path_daemon_numeric_future_package_audit_v1(args)
}

pub(crate) fn run_phase_stream_hot_path_daemon_numeric_future_portfolio_audit_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_hot_path_daemon_numeric_future_portfolio_audit_v1(args)
}

pub(crate) fn run_phase_stream_hot_path_daemon_numeric_admission_portfolio_gate_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_hot_path_daemon_numeric_admission_portfolio_gate_v1(args)
}

pub(crate) fn run_phase_stream_hot_path_daemon_numeric_admission_portfolio_runtime_replay_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_hot_path_daemon_numeric_admission_portfolio_runtime_replay_v1(args)
}

pub(crate) fn run_phase_stream_hot_path_daemon_numeric_false_accept_split_audit_v1<I>(
    args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    live_store_adapter::run_phase_stream_hot_path_daemon_numeric_false_accept_split_audit_v1(args)
}

pub(crate) fn run_phase_stream_global_denominator_compressibility_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GLOBAL_DENOMINATOR_COMPRESSIBILITY_AUDIT_REPORT));
    let current5k_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CURRENT5K_FEEDBACK_REPORT));
    let mining_report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_PHASE_ATOM_LIVE_SELF_MINING_MULTIFAMILY_V7_REPORT)
    });
    let global_phase_atom_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_PHASE_ATOM_TRACE_JSONL));

    let current5k = read_json_value(&current5k_report_path)?;
    let mining = read_json_value(&mining_report_path)?;
    let mining_trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            mining
                .get("input_trace_paths")
                .and_then(|value| value.as_array())
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|value| value.as_str())
                        .map(PathBuf::from)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            rest
        }
    };

    let global_total_rows = json_u64(&current5k, &["total_llm_calls"]).unwrap_or(0);
    let global_total_tokens = json_u64(&current5k, &["total_baseline_tokens"]).unwrap_or(0);
    let global_exact_cache_hits = json_u64(&current5k, &["exact_cache_hits"]).unwrap_or(0);
    let global_exact_cache_tokens_saved =
        json_u64(&current5k, &["exact_cache_tokens_saved"]).unwrap_or(0);
    let global_current_cpu_accepts = json_u64(
        &current5k,
        &["incremental_cpu_accept_unique_request_fingerprints"],
    )
    .unwrap_or(0);
    let global_current_tokens_saved =
        json_u64(&current5k, &["nando_cpu_tokens_saved"]).unwrap_or(0);
    let global_current_cost_saved =
        json_u64(&current5k, &["nando_cpu_cost_saved_microusd"]).unwrap_or(0);

    let mining_total_rows = json_u64(&mining, &["total_rows"]).unwrap_or(0);
    let mining_total_tokens = mining
        .get("classes")
        .and_then(|classes| classes.as_array())
        .map(|classes| {
            classes
                .iter()
                .filter_map(|class| class.get("total_tokens").and_then(|value| value.as_u64()))
                .sum::<u64>()
        })
        .unwrap_or(0);
    let mining_cpu_accepts =
        json_u64(&mining, &["aggregate_unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let mining_tokens_saved = json_u64(&mining, &["aggregate_nando_cpu_tokens_saved"]).unwrap_or(0);
    let mining_cost_saved =
        json_u64(&mining, &["aggregate_nando_cpu_cost_saved_microusd"]).unwrap_or(0);
    let mining_false_accepts = mining
        .get("classes")
        .and_then(|classes| classes.as_array())
        .map(|classes| {
            classes
                .iter()
                .filter(|class| {
                    class
                        .get("accepted_for_shadow_review")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false)
                })
                .filter_map(|class| class.get("false_accepts").and_then(|value| value.as_u64()))
                .sum::<u64>()
        })
        .unwrap_or(0);
    let mut accepted_bucket_keys = BTreeSet::new();
    let mut accepted_decision_rows = 0u64;
    let mut accepted_decision_non_exact_rows = 0u64;
    let mut accepted_decision_tokens_saved = 0u64;
    let mut accepted_decision_cost_saved = 0u64;
    let mut accepted_decision_request_fingerprints = BTreeSet::new();
    let accepted_classes = mining
        .get("classes")
        .and_then(|classes| classes.as_array())
        .map(|classes| {
            classes
                .iter()
                .filter(|class| {
                    class
                        .get("accepted_for_shadow_review")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false)
                })
                .map(|class| {
                    let action_family = class
                        .get("action_family")
                        .and_then(|value| value.as_str())
                        .unwrap_or("unknown");
                    accepted_bucket_keys.insert(action_family.to_owned());
                    let accepted_decisions = class
                        .get("accepted_heldout_decisions")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();
                    for decision in &accepted_decisions {
                        accepted_decision_rows = accepted_decision_rows.saturating_add(1);
                        let exact_hit = decision
                            .get("exact_cache_hit")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false);
                        if !exact_hit {
                            accepted_decision_non_exact_rows =
                                accepted_decision_non_exact_rows.saturating_add(1);
                            accepted_decision_tokens_saved =
                                accepted_decision_tokens_saved.saturating_add(
                                    decision
                                        .get("token_cost")
                                        .and_then(|cost| cost.get("total_tokens"))
                                        .and_then(|value| value.as_u64())
                                        .unwrap_or(0),
                                );
                            accepted_decision_cost_saved =
                                accepted_decision_cost_saved.saturating_add(
                                    decision
                                        .get("token_cost")
                                        .and_then(|cost| cost.get("total_cost_microusd"))
                                        .and_then(|value| value.as_u64())
                                        .unwrap_or(0),
                                );
                        }
                        if let Some(fingerprint) = decision
                            .get("request_fingerprint")
                            .and_then(|value| value.as_str())
                        {
                            accepted_decision_request_fingerprints.insert(fingerprint.to_owned());
                        }
                    }
                    serde_json::json!({
                        "action_family": action_family,
                        "task_name": class.get("task_name").and_then(|value| value.as_str()).unwrap_or("unknown"),
                        "candidate_package_path": class.get("candidate_package_path").and_then(|value| value.as_str()).unwrap_or(""),
                        "package_fingerprint64": class.get("package_fingerprint64").and_then(|value| value.as_u64()).unwrap_or(0),
                        "events_seen": class.get("events_seen").and_then(|value| value.as_u64()).unwrap_or(0),
                        "exact_cache_hits": class.get("exact_cache_hits").and_then(|value| value.as_u64()).unwrap_or(0),
                        "heldout_events": class.get("heldout_events").and_then(|value| value.as_u64()).unwrap_or(0),
                        "heldout_local_operator_calls": class.get("heldout_local_operator_calls").and_then(|value| value.as_u64()).unwrap_or(0),
                        "heldout_fallback_calls": class.get("heldout_fallback_calls").and_then(|value| value.as_u64()).unwrap_or(0),
                        "heldout_missed_safe_accepts": class.get("heldout_missed_safe_accepts").and_then(|value| value.as_u64()).unwrap_or(0),
                        "unique_cpu_accepts_over_exact_cache": class.get("unique_cpu_accepts_over_exact_cache").and_then(|value| value.as_u64()).unwrap_or(0),
                        "nando_cpu_tokens_saved": class.get("nando_cpu_tokens_saved").and_then(|value| value.as_u64()).unwrap_or(0),
                        "false_accepts": class.get("false_accepts").and_then(|value| value.as_u64()).unwrap_or(0),
                        "runtime_margin_parity_mismatches": class.get("runtime_margin_parity_mismatches").and_then(|value| value.as_u64()).unwrap_or(0),
                        "accepted_heldout_decisions": accepted_decisions,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let global_phase_stats =
        scan_phase_atom_trace_for_join(&global_phase_atom_trace_path, &accepted_bucket_keys)?;
    let mut mining_trace_stats = PhaseAtomJoinScan::default();
    for path in &mining_trace_paths {
        mining_trace_stats.merge(scan_phase_atom_trace_for_join(path, &accepted_bucket_keys)?);
    }
    let fingerprint_overlap_count = global_phase_stats
        .request_fingerprints
        .intersection(&mining_trace_stats.request_fingerprints)
        .count() as u64;
    let accepted_bucket_fingerprint_overlap_count = global_phase_stats
        .request_fingerprints
        .intersection(&mining_trace_stats.accepted_bucket_request_fingerprints)
        .count() as u64;
    let accepted_decision_global_fingerprint_overlap_count = global_phase_stats
        .request_fingerprints
        .intersection(&accepted_decision_request_fingerprints)
        .count() as u64;
    let compatible_agent_loop_rows = global_phase_stats
        .rows
        .saturating_add(mining_trace_stats.rows);
    let compatible_agent_loop_unique_request_fingerprints = global_phase_stats
        .request_fingerprints
        .union(&mining_trace_stats.request_fingerprints)
        .count() as u64;

    let joined_mined_accepts_counted_in_global = 0u64;
    let joined_mined_tokens_counted_in_global = 0u64;
    let joined_global_cpu_accepts =
        global_current_cpu_accepts + joined_mined_accepts_counted_in_global;
    let joined_global_tokens_saved =
        global_current_tokens_saved + joined_mined_tokens_counted_in_global;

    let report = serde_json::json!({
        "report_kind": "global_denominator_compressibility_audit_v1",
        "mode": "audit_only_no_runtime_change",
        "input_paths": {
            "current5k_feedback_report": current5k_report_path.display().to_string(),
            "phase_center_self_mining_report": mining_report_path.display().to_string(),
            "global_phase_atom_trace": global_phase_atom_trace_path.display().to_string(),
            "phase_center_self_mining_input_traces": mining_trace_paths.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
        },
        "global_current5k": {
            "total_rows": global_total_rows,
            "total_tokens": global_total_tokens,
            "exact_cache_hits": global_exact_cache_hits,
            "exact_cache_tokens_saved": global_exact_cache_tokens_saved,
            "current_cpu_accepts_over_exact_cache": global_current_cpu_accepts,
            "current_nando_tokens_saved_over_exact_cache": global_current_tokens_saved,
            "current_nando_cost_saved_microusd": global_current_cost_saved,
            "current_nando_calls_saved_pct": percent_u64(global_current_cpu_accepts, global_total_rows),
            "current_nando_calls_saved_milli": per_thousand_u64(global_current_cpu_accepts, global_total_rows),
            "current_nando_tokens_saved_pct": percent_u64(global_current_tokens_saved, global_total_tokens),
            "token_cost_estimate_used": json_bool(&current5k, &["token_cost_estimate_used"]).unwrap_or(false),
            "market_claim_allowed": json_bool(&current5k, &["market_claim_allowed"]).unwrap_or(false),
        },
        "selected_verifier_bound_phase_center_mining": {
            "denominator_scope": "selected verifier-bound phase-atom traces, not global current5k",
            "total_rows": mining_total_rows,
            "total_tokens": mining_total_tokens,
            "action_families_seen": json_u64(&mining, &["action_families_seen"]).unwrap_or(0),
            "high_value_classes": json_u64(&mining, &["high_value_classes"]).unwrap_or(0),
            "compiled_quarantine_candidates": json_u64(&mining, &["compiled_quarantine_candidates"]).unwrap_or(0),
            "shadow_accepted_candidates": json_u64(&mining, &["shadow_accepted_candidates"]).unwrap_or(0),
            "cpu_accepts_over_exact_cache": mining_cpu_accepts,
            "calls_saved_pct_on_selected_trace_only": percent_u64(mining_cpu_accepts, mining_total_rows),
            "calls_saved_milli_on_selected_trace_only": per_thousand_u64(mining_cpu_accepts, mining_total_rows),
            "tokens_saved_on_selected_trace_only": mining_tokens_saved,
            "tokens_saved_pct_on_selected_trace_only": percent_u64(mining_tokens_saved, mining_total_tokens),
            "cost_saved_microusd_on_selected_trace_only": mining_cost_saved,
            "false_accepts": mining_false_accepts,
            "local_accept_enabled": json_bool(&mining, &["local_accept_enabled"]).unwrap_or(false),
            "market_money_claim_allowed": json_bool(&mining, &["market_money_claim_allowed"]).unwrap_or(false),
            "accepted_classes": accepted_classes,
        },
        "global_join_status": {
            "joined_mined_profile_accepts_counted_in_global": joined_mined_accepts_counted_in_global,
            "joined_mined_profile_tokens_counted_in_global": joined_mined_tokens_counted_in_global,
            "global_cpu_accepts_after_join": joined_global_cpu_accepts,
            "global_nando_calls_saved_pct_after_join": percent_u64(joined_global_cpu_accepts, global_total_rows),
            "global_nando_calls_saved_milli_after_join": per_thousand_u64(joined_global_cpu_accepts, global_total_rows),
            "global_nando_tokens_saved_after_join": joined_global_tokens_saved,
            "global_nando_tokens_saved_pct_after_join": percent_u64(joined_global_tokens_saved, global_total_tokens),
            "bridge_gap": "phase-center v7 accepted classes have explicit join evidence, but the current global phase-atom denominator has no matching accepted bucket rows/fingerprints",
            "next_required_artifact": "upgrade global recorder to include tool_status/result phase-atom events, then replay accepted .nwpc candidates against that global denominator",
        },
        "row_level_join_evidence": {
            "global_phase_atom_trace": global_phase_stats.to_json(25),
            "selected_mining_traces": mining_trace_stats.to_json(25),
            "global_vs_selected_unique_request_fingerprint_overlap": fingerprint_overlap_count,
            "global_vs_selected_accepted_bucket_request_fingerprint_overlap": accepted_bucket_fingerprint_overlap_count,
            "global_vs_selected_accepted_decision_request_fingerprint_overlap": accepted_decision_global_fingerprint_overlap_count,
            "accepted_decision_rows": accepted_decision_rows,
            "accepted_decision_non_exact_rows": accepted_decision_non_exact_rows,
            "accepted_decision_unique_request_fingerprints": accepted_decision_request_fingerprints.len(),
            "accepted_bucket_rows_in_global_phase_atom_trace": global_phase_stats.accepted_bucket_rows,
            "accepted_bucket_unique_request_fingerprints_in_global_phase_atom_trace": global_phase_stats.accepted_bucket_request_fingerprints.len(),
            "accepted_bucket_rows_in_selected_mining_traces": mining_trace_stats.accepted_bucket_rows,
            "diagnosis": if global_phase_stats.accepted_bucket_rows == 0 {
                "current global phase-atom denominator has no rows matching accepted v7 tool_status buckets; direct global replay would count zero until the global recorder includes tool_status/result events"
            } else if fingerprint_overlap_count == 0 {
                "accepted bucket shape exists but request_fingerprint overlap is zero; need a shared trace window or fingerprint join export"
            } else {
                "some join surface exists; next run should replay .nwpc packages and count verified accepts"
            },
        },
        "compatible_agent_loop_denominator_projection": {
            "scope": "request-history phase-atom rows plus verifier-bound tool_status/run_check phase-atom event rows; this is an agent-loop event denominator, not the old current5k LLM-call denominator",
            "total_rows": compatible_agent_loop_rows,
            "unique_request_fingerprints": compatible_agent_loop_unique_request_fingerprints,
            "projected_cpu_accepts_over_exact_cache": accepted_decision_non_exact_rows,
            "projected_calls_saved_pct": percent_u64(accepted_decision_non_exact_rows, compatible_agent_loop_rows),
            "projected_calls_saved_milli": per_thousand_u64(accepted_decision_non_exact_rows, compatible_agent_loop_rows),
            "projected_tokens_saved": accepted_decision_tokens_saved,
            "projected_cost_saved_microusd": accepted_decision_cost_saved,
            "false_accepts": mining_false_accepts,
            "market_money_claim_allowed": false,
            "claim_boundary": "projection uses row-level v8 heldout decisions on a compatible event denominator; it is not a production local_accept deployment and not provider-billing money evidence",
        },
        "pass_condition_audit": {
            "every_global_row_accounted_for": global_total_rows > 0,
            "selected_traces_not_used_as_global_denominator": true,
            "exact_cache_counted_before_nando": true,
            "cpu_accepts_counted_only_over_exact_cache_misses": true,
            "false_accepts_zero_for_selected_mining": mining_false_accepts == 0,
            "row_level_join_evidence_emitted": true,
            "global_join_complete": global_phase_stats.accepted_bucket_rows > 0 && fingerprint_overlap_count > 0,
        },
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false,
        },
        "verdict": "GLOBAL_DENOMINATOR_COMPRESSIBILITY_AUDIT_V1_WATCH_JOIN_MISSING",
        "boundary": "This report records the v7 phase-center shadow win and preserves the current5k global denominator. It does not promote, enable local_accept, or count selected verifier-bound traces as market/global savings.",
    });

    write_json_file(&report_path, &report)?;
    println!("global_denominator_compressibility_audit_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  global_total_rows: {global_total_rows}");
    println!("  global_current_cpu_accepts: {global_current_cpu_accepts}");
    println!(
        "  global_current_nando_calls_saved_milli: {}",
        per_thousand_u64(global_current_cpu_accepts, global_total_rows)
    );
    println!("  selected_mining_total_rows: {mining_total_rows}");
    println!("  selected_mining_cpu_accepts: {mining_cpu_accepts}");
    println!("  selected_mining_false_accepts: {mining_false_accepts}");
    println!(
        "  accepted_bucket_rows_in_global_phase_atom_trace: {}",
        global_phase_stats.accepted_bucket_rows
    );
    println!(
        "  global_vs_selected_unique_request_fingerprint_overlap: {fingerprint_overlap_count}"
    );
    println!("  joined_mined_profile_accepts_counted_in_global: 0");
    println!("  verdict: GLOBAL_DENOMINATOR_COMPRESSIBILITY_AUDIT_V1_WATCH_JOIN_MISSING");
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_compatible_denominator_shadow_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_COMPATIBLE_DENOMINATOR_SHADOW_REPORT));
    let decision_log_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_PHASE_ATOM_COMPATIBLE_DENOMINATOR_SHADOW_DECISION_LOG)
    });
    let mining_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from("target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v8-row-evidence.report.json")
        });
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![
                PathBuf::from(DEFAULT_CODEX_HISTORY_PHASE_ATOM_TRACE_JSONL),
                PathBuf::from(DEFAULT_CODEX_SESSION_TOOL_STATUS_APPEND_LATEST_JSONL),
                PathBuf::from(DEFAULT_CODEX_SESSION_RUN_CHECK_VERIFIER_JSONL),
            ]
        } else {
            rest
        }
    };
    let mining = read_json_value(&mining_report_path)?;
    let price_config = read_json_file::<ModelPriceConfig>(Path::new(DEFAULT_PRICE_CONFIG))?;
    let train_permille = json_u64(&mining, &["train_permille"])
        .unwrap_or(DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_TRAIN_PERMILLE as u64)
        .clamp(1, 999) as usize;
    let mut profiles = Vec::<CompatibleDenominatorProfile>::new();
    for class in mining
        .get("classes")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter(|class| {
            class
                .get("accepted_for_shadow_review")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        })
    {
        let bucket_key = class
            .get("action_family")
            .and_then(|value| value.as_str())
            .ok_or_else(|| "accepted class missing action_family".to_owned())?
            .to_owned();
        let action_family_atom = bucket_key
            .split_once("::")
            .map(|(prefix, _)| prefix)
            .unwrap_or(bucket_key.as_str())
            .to_owned();
        let task_name = class
            .get("task_name")
            .and_then(|value| value.as_str())
            .ok_or_else(|| format!("accepted class '{bucket_key}' missing task_name"))?
            .to_owned();
        let package_path = PathBuf::from(
            class
                .get("candidate_package_path")
                .and_then(|value| value.as_str())
                .ok_or_else(|| format!("accepted class '{bucket_key}' missing package path"))?,
        );
        let package_bytes = std::fs::read(&package_path).map_err(|error| {
            format!(
                "failed to read compatible denominator package '{}': {error}",
                package_path.display()
            )
        })?;
        let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes)
            .map_err(|error| {
                format!(
                    "compatible denominator package inspect error '{}': {error:?}",
                    package_path.display()
                )
            })?;
        let expected_fingerprint64 = class
            .get("package_fingerprint64")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let safe_accept_margin_threshold_micro = class
            .get("safe_accept_margin_threshold_micro")
            .and_then(|value| value.as_i64())
            .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
        let runtime = PhaseCenterOffloadRuntime::from_package_bytes(
            &package_bytes,
            PhaseCenterOffloadPolicy::new(safe_accept_margin_threshold_micro)
                .map_err(|error| format!("compatible denominator invalid policy: {error:?}"))?,
        )
        .map_err(|error| format!("compatible denominator package load error: {error:?}"))?;
        profiles.push(CompatibleDenominatorProfile {
            bucket_key,
            action_family_atom,
            task_name,
            candidate_package_path: package_path.display().to_string(),
            expected_package_fingerprint64: expected_fingerprint64,
            package_fingerprint64: package_info.fingerprint64,
            package_bytes: package_bytes.len(),
            package_records: package_info.record_count,
            package_matches_report: expected_fingerprint64 == 0
                || expected_fingerprint64 == package_info.fingerprint64,
            safe_accept_margin_threshold_micro,
            runtime,
            events: Vec::new(),
        });
    }

    let mut total_rows = 0usize;
    let mut unique_request_fingerprints = BTreeSet::new();
    let mut exact_cache_seen = BTreeSet::<String>::new();
    let mut total_tokens = 0usize;
    let mut total_cost_microusd = 0u64;
    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read compatible denominator trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows += 1;
            if total_rows.is_multiple_of(1000) {
                println!("  compatible_denominator_rows_scanned: {total_rows}");
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse compatible denominator trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            if let Some(fingerprint) = json_string(&row, &["request_fingerprint"]) {
                unique_request_fingerprints.insert(fingerprint);
            }
            let exact_cache_key = json_string(&row, &["exact_cache_key"])
                .or_else(|| json_string(&row, &["request_fingerprint"]))
                .unwrap_or_else(|| format!("compatible_denominator_row:{total_rows}"));
            let exact_cache_hit = exact_cache_seen.contains(&exact_cache_key);
            exact_cache_seen.insert(exact_cache_key);
            let token_cost = phase_atom_binary_token_cost(&row);
            total_tokens = total_tokens.saturating_add(token_cost.total_tokens);
            total_cost_microusd =
                total_cost_microusd.saturating_add(token_cost.total_cost_microusd);

            let action_atoms = phase_atom_string_vec(&row, "action_atoms");
            let request_atoms = phase_atom_string_vec(&row, "request_atoms");
            let state_atoms = phase_atom_string_vec(&row, "state_atoms");
            let tool_atoms = phase_atom_string_vec(&row, "tool_atoms");
            let route_hint_atoms = phase_atom_string_vec(&row, "route_hint_atoms");
            for (profile_index, profile) in profiles.iter_mut().enumerate() {
                if !action_atoms
                    .iter()
                    .any(|atom| atom == &profile.action_family_atom)
                {
                    continue;
                }
                let bucket_key = phase_atom_state_action_bucket_key(
                    &profile.action_family_atom,
                    &request_atoms,
                    &state_atoms,
                    &tool_atoms,
                    &route_hint_atoms,
                );
                if bucket_key != profile.bucket_key {
                    continue;
                }
                if let Some(event) = parse_phase_atom_binary_event_for_action(
                    &row,
                    profile.events.len(),
                    &profile.action_family_atom,
                    &profile.task_name,
                ) {
                    profile.events.push(CompatibleRoutedEvent {
                        profile_index,
                        denominator_row_index: total_rows - 1,
                        source_trace_path: trace_path.display().to_string(),
                        source_line_index: line_index,
                        exact_cache_hit,
                        event,
                    });
                }
                break;
            }
        }
    }

    if let Some(parent) = decision_log_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create compatible denominator decision log dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let mut decision_log = std::fs::File::create(&decision_log_path).map_err(|error| {
        format!(
            "failed to create compatible denominator decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;

    let mut profile_reports = Vec::new();
    let mut routed_events = 0usize;
    let mut heldout_routed_events = 0usize;
    let mut local_operator_shadow_decisions = 0usize;
    let mut fallback_shadow_decisions = 0usize;
    let mut exact_cache_hits_in_routed_events = 0usize;
    let mut unique_cpu_accepts_over_exact_cache = 0usize;
    let mut nando_cpu_tokens_saved = 0usize;
    let mut nando_cpu_cost_saved_microusd = 0u64;
    let mut nando_cpu_token_evidence_missing_events = 0usize;
    let mut nando_cpu_cost_evidence_missing_events = 0usize;
    let mut false_accepts = 0usize;
    let mut wrong_wins = 0usize;
    let mut latencies = Vec::new();
    let mut margins = Vec::new();
    for profile in &profiles {
        routed_events = routed_events.saturating_add(profile.events.len());
        let event_values = profile
            .events
            .iter()
            .map(|routed| routed.event.clone())
            .collect::<Vec<_>>();
        let (train_indices, heldout_indices) =
            phase_atom_binary_time_split_indices(&event_values, train_permille);
        let mut profile_local = 0usize;
        let mut profile_unique = 0usize;
        let mut profile_false = 0usize;
        let mut profile_fallback = 0usize;
        for (heldout_position, event_index) in heldout_indices.iter().copied().enumerate() {
            let routed = &profile.events[event_index];
            let event = &routed.event;
            let safe_accept_vec = phase_atom_binary_event_vector_for_task(
                event,
                true,
                profile.runtime.cells(),
                &profile.task_name,
            );
            let zero = vec![nando_core::PhaseCenterCell::default(); profile.runtime.cells()];
            let task = PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: safe_accept_vec.into_boxed_slice(),
                wrong_vec: zero.into_boxed_slice(),
            };
            let started = Instant::now();
            let decision = profile.runtime.offload_decision(&task).map_err(|error| {
                format!("compatible denominator offload decision error: {error:?}")
            })?;
            latencies.push(started.elapsed().as_nanos());
            margins.push(decision.margin_micro);
            let local_operator = decision.is_local_operator();
            let false_accept = local_operator && !event.verified_safe_accept;
            let unique_accept =
                local_operator && event.verified_safe_accept && !routed.exact_cache_hit;
            let classifier_wrong = if event.verified_safe_accept {
                decision.margin_micro <= 0
            } else {
                decision.margin_micro >= 0
            };
            wrong_wins += usize::from(classifier_wrong);
            false_accepts += usize::from(false_accept);
            profile_false += usize::from(false_accept);
            exact_cache_hits_in_routed_events += usize::from(routed.exact_cache_hit);
            if local_operator && event.verified_safe_accept {
                local_operator_shadow_decisions += 1;
                profile_local += 1;
                if unique_accept {
                    unique_cpu_accepts_over_exact_cache += 1;
                    profile_unique += 1;
                    nando_cpu_tokens_saved =
                        nando_cpu_tokens_saved.saturating_add(event.token_cost.total_tokens);
                    nando_cpu_cost_saved_microusd = nando_cpu_cost_saved_microusd
                        .saturating_add(event.token_cost.total_cost_microusd);
                    nando_cpu_token_evidence_missing_events +=
                        usize::from(event.token_cost.token_evidence_missing);
                    nando_cpu_cost_evidence_missing_events +=
                        usize::from(event.token_cost.cost_evidence_missing);
                }
            } else {
                fallback_shadow_decisions += 1;
                profile_fallback += 1;
            }
            heldout_routed_events += 1;
            let decision_row = serde_json::json!({
                "schema_version": "phase_atom_compatible_denominator_shadow_decision_v1",
                "profile_task_name": profile.task_name,
                "profile_bucket_key": profile.bucket_key,
                "profile_index": routed.profile_index,
                "heldout_position": heldout_position,
                "denominator_row_index": routed.denominator_row_index,
                "source_trace_path": routed.source_trace_path,
                "source_line_index": routed.source_line_index,
                "request_fingerprint": event.request_fingerprint,
                "exact_cache_key": event.exact_cache_key,
                "exact_cache_hit": routed.exact_cache_hit,
                "verified_safe_accept": event.verified_safe_accept,
                "margin_micro": decision.margin_micro,
                "local_operator_shadow_decision": local_operator && event.verified_safe_accept,
                "fallback_shadow_decision": !(local_operator && event.verified_safe_accept),
                "false_accept": false_accept,
                "unique_cpu_accept_over_exact_cache": unique_accept,
                "token_cost": event.token_cost,
                "package_fingerprint64": profile.package_fingerprint64,
                "local_accept_enabled": false,
            });
            writeln!(
                decision_log,
                "{}",
                serde_json::to_string(&decision_row)
                    .map_err(|error| format!("decision log serialization error: {error}"))?
            )
            .map_err(|error| format!("failed to write compatible decision log: {error}"))?;
        }
        profile_reports.push(serde_json::json!({
            "bucket_key": profile.bucket_key,
            "action_family_atom": profile.action_family_atom,
            "task_name": profile.task_name,
            "candidate_package_path": profile.candidate_package_path,
            "expected_package_fingerprint64": profile.expected_package_fingerprint64,
            "package_fingerprint64": profile.package_fingerprint64,
            "package_matches_report": profile.package_matches_report,
            "package_bytes": profile.package_bytes,
            "package_records": profile.package_records,
            "runtime_cells": profile.runtime.cells(),
            "runtime_record_count": profile.runtime.record_count(),
            "runtime_bytes_estimate": profile.runtime.bytes_estimate(),
            "safe_accept_margin_threshold_micro": profile.safe_accept_margin_threshold_micro,
            "events_seen": profile.events.len(),
            "train_events": train_indices.len(),
            "heldout_events": heldout_indices.len(),
            "heldout_local_operator_calls": profile_local,
            "heldout_fallback_calls": profile_fallback,
            "unique_cpu_accepts_over_exact_cache": profile_unique,
            "false_accepts": profile_false,
        }));
    }
    margins.sort_unstable();
    latencies.sort_unstable();
    let all_packages_match = profiles
        .iter()
        .all(|profile| profile.package_matches_report);
    let shadow_gate_passed = !profiles.is_empty()
        && all_packages_match
        && heldout_routed_events > 0
        && unique_cpu_accepts_over_exact_cache > 0
        && false_accepts == 0;
    let denominator_report = serde_json::json!({
        "total_rows": total_rows,
        "unique_request_fingerprints": unique_request_fingerprints.len(),
        "total_tokens": total_tokens,
        "total_cost_microusd": total_cost_microusd,
    });
    let token_evidence_present = nando_cpu_tokens_saved > 0
        && nando_cpu_token_evidence_missing_events < unique_cpu_accepts_over_exact_cache;
    let provider_or_row_cost_evidence_present = nando_cpu_cost_saved_microusd > 0
        && nando_cpu_cost_evidence_missing_events < unique_cpu_accepts_over_exact_cache;
    let estimated_nando_cpu_cost_saved_microusd =
        if token_evidence_present && !provider_or_row_cost_evidence_present {
            estimated_event_cost_microusd(nando_cpu_tokens_saved, 0, &price_config)
        } else {
            0
        };
    let explicit_model_price_estimate_used = estimated_nando_cpu_cost_saved_microusd > 0;
    let estimated_cost_method = if explicit_model_price_estimate_used {
        "total_saved_tokens_as_input_token_floor_from_model_price_config"
    } else if provider_or_row_cost_evidence_present {
        "provider_or_row_cost_evidence_present_no_estimate_needed"
    } else {
        "no_token_or_price_estimate_available"
    };
    let forbidden_flags = serde_json::json!({
        "nwrb_used": false,
        "role_binding_backend_used": false,
        "lookup_used": false,
        "target_id_or_proof_rule_id_authority_used": false,
        "concrete_x_lookup_used": false,
        "manual_local_out_t_used": false,
        "local_accept_without_verifier_used": false
    });
    let verdict = if shadow_gate_passed {
        "PHASE_ATOM_COMPATIBLE_DENOMINATOR_SHADOW_V1_PASS_SHADOW_ONLY"
    } else {
        "PHASE_ATOM_COMPATIBLE_DENOMINATOR_SHADOW_V1_WATCH"
    };
    let mut report_map = serde_json::Map::new();
    report_map.insert(
        "report_kind".to_owned(),
        serde_json::Value::String("phase_atom_compatible_denominator_shadow_v1".to_owned()),
    );
    report_map.insert(
        "mode".to_owned(),
        serde_json::Value::String(
            "accepted_nwpc_packages_replayed_against_compatible_agent_loop_event_denominator_shadow_only"
                .to_owned(),
        ),
    );
    report_map.insert(
        "self_mining_report_path".to_owned(),
        serde_json::Value::String(mining_report_path.display().to_string()),
    );
    report_map.insert(
        "decision_log_path".to_owned(),
        serde_json::Value::String(decision_log_path.display().to_string()),
    );
    report_map.insert(
        "trace_paths".to_owned(),
        serde_json::Value::Array(
            trace_paths
                .iter()
                .map(|path| serde_json::Value::String(path.display().to_string()))
                .collect(),
        ),
    );
    report_map.insert(
        "train_permille".to_owned(),
        serde_json::json!(train_permille),
    );
    report_map.insert("denominator".to_owned(), denominator_report);
    report_map.insert(
        "profiles".to_owned(),
        serde_json::Value::Array(profile_reports),
    );
    report_map.insert(
        "profile_count".to_owned(),
        serde_json::json!(profiles.len()),
    );
    report_map.insert("routed_events".to_owned(), serde_json::json!(routed_events));
    report_map.insert(
        "heldout_routed_events".to_owned(),
        serde_json::json!(heldout_routed_events),
    );
    report_map.insert(
        "local_operator_shadow_decisions".to_owned(),
        serde_json::json!(local_operator_shadow_decisions),
    );
    report_map.insert(
        "fallback_shadow_decisions".to_owned(),
        serde_json::json!(fallback_shadow_decisions),
    );
    report_map.insert(
        "exact_cache_hits_in_routed_events".to_owned(),
        serde_json::json!(exact_cache_hits_in_routed_events),
    );
    report_map.insert(
        "unique_cpu_accepts_over_exact_cache".to_owned(),
        serde_json::json!(unique_cpu_accepts_over_exact_cache),
    );
    report_map.insert(
        "calls_saved_pct".to_owned(),
        serde_json::json!(percent_u64(
            unique_cpu_accepts_over_exact_cache as u64,
            total_rows as u64,
        )),
    );
    report_map.insert(
        "calls_saved_milli".to_owned(),
        serde_json::json!(per_thousand(
            unique_cpu_accepts_over_exact_cache,
            total_rows
        )),
    );
    report_map.insert(
        "nando_cpu_tokens_saved".to_owned(),
        serde_json::json!(nando_cpu_tokens_saved),
    );
    report_map.insert(
        "tokens_saved_pct".to_owned(),
        serde_json::json!(percent_u64(
            nando_cpu_tokens_saved as u64,
            total_tokens as u64
        )),
    );
    report_map.insert(
        "nando_cpu_cost_saved_microusd".to_owned(),
        serde_json::json!(nando_cpu_cost_saved_microusd),
    );
    report_map.insert(
        "estimated_nando_cpu_cost_saved_microusd".to_owned(),
        serde_json::json!(estimated_nando_cpu_cost_saved_microusd),
    );
    report_map.insert(
        "token_cost_evidence".to_owned(),
        serde_json::json!({
            "token_evidence_present": token_evidence_present,
            "provider_or_row_cost_evidence_present": provider_or_row_cost_evidence_present,
            "explicit_model_price_estimate_used": explicit_model_price_estimate_used,
            "price_config_schema_version": price_config.schema_version,
            "provider": price_config.default_provider,
            "model_id": price_config.default_model_id,
            "price_source": price_config.price_source,
            "input_cost_microusd_per_1k_tokens": price_config.input_cost_microusd_per_1k_tokens,
            "output_cost_microusd_per_1k_tokens": price_config.output_cost_microusd_per_1k_tokens,
            "unique_cpu_accepts_over_exact_cache": unique_cpu_accepts_over_exact_cache,
            "nando_cpu_tokens_saved": nando_cpu_tokens_saved,
            "nando_cpu_cost_saved_microusd": nando_cpu_cost_saved_microusd,
            "estimated_nando_cpu_cost_saved_microusd": estimated_nando_cpu_cost_saved_microusd,
            "accepted_token_evidence_missing_events": nando_cpu_token_evidence_missing_events,
            "accepted_cost_evidence_missing_events": nando_cpu_cost_evidence_missing_events,
            "estimated_cost_method": estimated_cost_method,
            "money_claim_blocker": "compatible denominator shadow is estimate-only for accepted costs unless provider billing or row-level cost evidence is present; market_money_claim_allowed remains false"
        }),
    );
    report_map.insert("false_accepts".to_owned(), serde_json::json!(false_accepts));
    report_map.insert("wrong_wins".to_owned(), serde_json::json!(wrong_wins));
    report_map.insert(
        "min_margin_micro".to_owned(),
        serde_json::json!(margins.first().copied().unwrap_or(0)),
    );
    report_map.insert(
        "p10_margin_micro".to_owned(),
        serde_json::json!(percentile_i64(&margins, 10)),
    );
    report_map.insert(
        "median_margin_micro".to_owned(),
        serde_json::json!(percentile_i64(&margins, 50)),
    );
    report_map.insert(
        "latency_p50_ns".to_owned(),
        serde_json::json!(percentile_u128(&latencies, 50)),
    );
    report_map.insert(
        "latency_p90_ns".to_owned(),
        serde_json::json!(percentile_u128(&latencies, 90)),
    );
    report_map.insert(
        "latency_p99_ns".to_owned(),
        serde_json::json!(percentile_u128(&latencies, 99)),
    );
    report_map.insert(
        "latency_max_ns".to_owned(),
        serde_json::json!(latencies.last().copied().unwrap_or(0)),
    );
    report_map.insert(
        "shadow_gate_passed".to_owned(),
        serde_json::json!(shadow_gate_passed),
    );
    report_map.insert("local_accept_enabled".to_owned(), serde_json::json!(false));
    report_map.insert("auto_promote_enabled".to_owned(), serde_json::json!(false));
    report_map.insert(
        "product_promotion_allowed".to_owned(),
        serde_json::json!(false),
    );
    report_map.insert(
        "market_money_claim_allowed".to_owned(),
        serde_json::json!(false),
    );
    report_map.insert("forbidden_flags".to_owned(), forbidden_flags);
    report_map.insert(
        "verdict".to_owned(),
        serde_json::Value::String(verdict.to_owned()),
    );
    report_map.insert(
        "boundary".to_owned(),
        serde_json::Value::String("shadow replay only: loads accepted quarantine .nwpc candidates from self-mining report and scores compatible event-denominator heldout rows; does not promote, enable local_accept, count provider-billing money, use legacy nwrb, lookup, target/proof authority, concrete_x_lookup, or manual local_out_t".to_owned()),
    );
    let report = serde_json::Value::Object(report_map);
    write_json_file(&report_path, &report)?;
    println!("phase_atom_compatible_denominator_shadow_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  decision_log_path: {}", decision_log_path.display());
    println!("  denominator_rows: {total_rows}");
    println!("  profile_count: {}", profiles.len());
    println!("  heldout_routed_events: {heldout_routed_events}");
    println!("  unique_cpu_accepts_over_exact_cache: {unique_cpu_accepts_over_exact_cache}");
    println!("  false_accepts: {false_accepts}");
    println!("  local_accept_enabled: false");
    println!(
        "  verdict: {}",
        report
            .get("verdict")
            .and_then(|value| value.as_str())
            .unwrap_or("UNKNOWN")
    );
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_market_money_claim_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_MARKET_MONEY_CLAIM_GATE_REPORT));
    let compatible_shadow_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_COMPATIBLE_DENOMINATOR_SHADOW_REPORT));
    let cost_audit_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_COST_EVIDENCE_AUDIT_REPORT));
    let price_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PRICE_CONFIG));
    let explicit_provider_billing_evidence_path = args.next().map(PathBuf::from);
    if let Some(extra) = args.next() {
        return Err(format!(
            "unexpected extra argument '{extra}' for phase-stream-phase-atom-market-money-claim-gate-v1"
        ));
    }

    let compatible = read_json_value(&compatible_shadow_report_path)?;
    let cost_audit = read_json_value(&cost_audit_report_path)?;
    let price_config = read_json_value(&price_config_path)?;

    let price_source = json_string(&price_config, &["price_source"]).unwrap_or_default();
    let price_source_lower = price_source.to_ascii_lowercase();
    let price_source_is_placeholder = price_source_lower.contains("placeholder")
        || price_source_lower.contains("replace with real")
        || price_source_lower.contains("estimate");
    let user_approved_price_config = json_bool(&price_config, &["market_claim_price_approved"])
        .or_else(|| json_bool(&price_config, &["user_approved_price_config"]))
        .unwrap_or(false);
    let approved_by = json_string(&price_config, &["approved_by"]);
    let approved_at = json_string(&price_config, &["approved_at"]);
    let price_claim_scope = json_string(&price_config, &["claim_scope"])
        .unwrap_or_else(|| "not_market_approved".to_owned());
    let config_provider_billing_evidence_path =
        json_string(&price_config, &["provider_billing_evidence_path"]).map(PathBuf::from);
    let provider_billing_evidence_path = explicit_provider_billing_evidence_path
        .clone()
        .or(config_provider_billing_evidence_path);
    let provider_billing_evidence_file_present = provider_billing_evidence_path
        .as_ref()
        .is_some_and(|path| path.exists());

    let false_accepts = json_u64(&compatible, &["false_accepts"]).unwrap_or(u64::MAX);
    let unique_cpu_accepts_over_exact_cache =
        json_u64(&compatible, &["unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let nando_cpu_tokens_saved = json_u64(&compatible, &["nando_cpu_tokens_saved"]).unwrap_or(0);
    let nando_cpu_cost_saved_microusd =
        json_u64(&compatible, &["nando_cpu_cost_saved_microusd"]).unwrap_or(0);
    let accepted_token_evidence_missing_events = json_u64(
        &compatible,
        &[
            "token_cost_evidence",
            "accepted_token_evidence_missing_events",
        ],
    )
    .unwrap_or(u64::MAX);
    let accepted_cost_evidence_missing_events = json_u64(
        &compatible,
        &[
            "token_cost_evidence",
            "accepted_cost_evidence_missing_events",
        ],
    )
    .unwrap_or(u64::MAX);
    let compatible_shadow_market_money_claim_allowed =
        json_bool(&compatible, &["market_money_claim_allowed"]).unwrap_or(false);
    let compatible_shadow_local_accept_enabled =
        json_bool(&compatible, &["local_accept_enabled"]).unwrap_or(false);
    let compatible_shadow_gate_passed =
        json_bool(&compatible, &["shadow_gate_passed"]).unwrap_or(false);

    let provider_cost_events = json_u64(&cost_audit, &["provider_cost_events"]).unwrap_or(0);
    let estimated_cost_events = json_u64(&cost_audit, &["estimated_cost_events"]).unwrap_or(0);
    let money_proof_candidate_bucket_count =
        json_u64(&cost_audit, &["money_proof_candidate_bucket_count"]).unwrap_or(0);

    let safety_gate_passed = false_accepts == 0;
    let cpu_savings_gate_passed = unique_cpu_accepts_over_exact_cache > 0;
    let token_evidence_gate_passed =
        nando_cpu_tokens_saved > 0 && accepted_token_evidence_missing_events == 0;
    let row_cost_evidence_gate_passed =
        nando_cpu_cost_saved_microusd > 0 && accepted_cost_evidence_missing_events == 0;
    let provider_row_cost_gate_passed = provider_cost_events > 0;
    let provider_billing_file_gate_passed =
        provider_billing_evidence_file_present && provider_cost_events > 0;
    let user_approved_price_gate_passed =
        user_approved_price_config && approved_by.is_some() && approved_at.is_some();
    let price_source_gate_passed = !price_source_is_placeholder;
    let money_claim_gate_passed = compatible_shadow_gate_passed
        && safety_gate_passed
        && cpu_savings_gate_passed
        && token_evidence_gate_passed
        && row_cost_evidence_gate_passed
        && price_source_gate_passed
        && (provider_row_cost_gate_passed
            || provider_billing_file_gate_passed
            || user_approved_price_gate_passed);

    let mut blockers = Vec::<String>::new();
    if !compatible_shadow_gate_passed {
        blockers.push("compatible_shadow_gate_not_passed".to_owned());
    }
    if !safety_gate_passed {
        blockers.push("false_accepts_nonzero".to_owned());
    }
    if !cpu_savings_gate_passed {
        blockers.push("no_unique_cpu_accepts_over_exact_cache".to_owned());
    }
    if !token_evidence_gate_passed {
        blockers.push("accepted_token_evidence_missing_or_zero".to_owned());
    }
    if !row_cost_evidence_gate_passed {
        blockers.push("accepted_row_cost_evidence_missing_or_zero".to_owned());
    }
    if !price_source_gate_passed {
        blockers.push("price_source_is_placeholder_or_estimate".to_owned());
    }
    if !(provider_row_cost_gate_passed
        || provider_billing_file_gate_passed
        || user_approved_price_gate_passed)
    {
        blockers.push(
            "no_provider_billing_evidence_file_provider_row_cost_or_user_approved_price_config"
                .to_owned(),
        );
    }

    let claim_status = if money_claim_gate_passed {
        "EXTERNAL_MARKET_MONEY_CLAIM_READY"
    } else if row_cost_evidence_gate_passed {
        "INTERNAL_ESTIMATE_ONLY"
    } else {
        "MONEY_CLAIM_BLOCKED"
    };
    let report = serde_json::json!({
        "report_kind": "phase_atom_market_money_claim_gate_v1",
        "mode": "claim_gate_only_no_compile_no_promote_no_local_accept",
        "compatible_shadow_report_path": compatible_shadow_report_path.display().to_string(),
        "cost_audit_report_path": cost_audit_report_path.display().to_string(),
        "price_config_path": price_config_path.display().to_string(),
        "provider_billing_evidence_path": provider_billing_evidence_path
            .as_ref()
            .map(|path| path.display().to_string()),
        "claim_status": claim_status,
        "money_claim_gate_passed": money_claim_gate_passed,
        "market_money_claim_allowed": money_claim_gate_passed,
        "local_accept_enabled": false,
        "product_promotion_allowed": false,
        "compatible_shadow": {
            "shadow_gate_passed": compatible_shadow_gate_passed,
            "local_accept_enabled": compatible_shadow_local_accept_enabled,
            "market_money_claim_allowed": compatible_shadow_market_money_claim_allowed,
            "false_accepts": false_accepts,
            "unique_cpu_accepts_over_exact_cache": unique_cpu_accepts_over_exact_cache,
            "nando_cpu_tokens_saved": nando_cpu_tokens_saved,
            "nando_cpu_cost_saved_microusd": nando_cpu_cost_saved_microusd,
            "accepted_token_evidence_missing_events": accepted_token_evidence_missing_events,
            "accepted_cost_evidence_missing_events": accepted_cost_evidence_missing_events,
        },
        "cost_audit": {
            "provider_cost_events": provider_cost_events,
            "estimated_cost_events": estimated_cost_events,
            "money_proof_candidate_bucket_count": money_proof_candidate_bucket_count,
        },
        "price_evidence": {
            "price_source": price_source,
            "price_source_is_placeholder": price_source_is_placeholder,
            "price_source_gate_passed": price_source_gate_passed,
            "user_approved_price_config": user_approved_price_config,
            "approved_by": approved_by,
            "approved_at": approved_at,
            "claim_scope": price_claim_scope,
            "user_approved_price_gate_passed": user_approved_price_gate_passed,
            "provider_billing_evidence_file_present": provider_billing_evidence_file_present,
            "provider_billing_file_gate_passed": provider_billing_file_gate_passed,
            "provider_row_cost_gate_passed": provider_row_cost_gate_passed,
        },
        "gates": {
            "compatible_shadow_gate_passed": compatible_shadow_gate_passed,
            "safety_gate_passed": safety_gate_passed,
            "cpu_savings_gate_passed": cpu_savings_gate_passed,
            "token_evidence_gate_passed": token_evidence_gate_passed,
            "row_cost_evidence_gate_passed": row_cost_evidence_gate_passed,
            "provider_row_cost_gate_passed": provider_row_cost_gate_passed,
            "provider_billing_file_gate_passed": provider_billing_file_gate_passed,
            "user_approved_price_gate_passed": user_approved_price_gate_passed,
            "price_source_gate_passed": price_source_gate_passed,
        },
        "blockers": blockers,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false
        },
        "boundary": "claim gate only: allows market money claim only when compatible shadow is safe, row cost is present, and provider billing or explicit non-placeholder user-approved price evidence is present; does not compile, promote, serve, or enable local_accept",
    });
    write_json_file(&report_path, &report)?;
    println!("phase_atom_market_money_claim_gate_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  claim_status: {claim_status}");
    println!("  money_claim_gate_passed: {money_claim_gate_passed}");
    println!("  market_money_claim_allowed: {money_claim_gate_passed}");
    println!("  local_accept_enabled: false");
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_frontier_shadow_replay_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_FRONTIER_SHADOW_REPLAY_REPORT));
    let decision_log_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_FRONTIER_SHADOW_REPLAY_DECISION_LOG));
    let frontier_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_FRONTIER_UNION_REPORT));
    let explicit_trace_paths = args.map(PathBuf::from).collect::<Vec<_>>();

    let frontier = read_json_value(&frontier_report_path)?;
    let report_paths = frontier
        .get("input_report_paths")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            format!(
                "frontier report '{}' missing input_report_paths",
                frontier_report_path.display()
            )
        })?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if report_paths.is_empty() {
        return Err("frontier report has no input reports".to_owned());
    }

    let mut profiles = Vec::<FrontierShadowReplayProfile>::new();
    let mut default_trace_paths = BTreeSet::<PathBuf>::new();
    let mut skipped_reports = Vec::new();
    for input_report_path in &report_paths {
        let promotion = read_json_value(input_report_path)?;
        let report_kind = promotion
            .get("report_kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let supported = report_kind == "phase_atom_run_check_time_split_promotion_audit_v1"
            || report_kind == "phase_atom_action_family_time_split_promotion_audit_v1";
        if !supported {
            skipped_reports.push(serde_json::json!({
                "path": input_report_path.display().to_string(),
                "report_kind": report_kind,
                "reason": "not_phase_atom_promotion_audit",
            }));
            continue;
        }
        let allowed = json_bool(&promotion, &["promotion_candidate_allowed"]).unwrap_or(false)
            && !json_bool(&promotion, &["product_promotion_allowed"]).unwrap_or(true)
            && !json_bool(&promotion, &["local_accept_enabled"]).unwrap_or(true)
            && promotion
                .get("forbidden_flags")
                .is_some_and(forbidden_flags_value_all_false)
            && json_bool(&promotion, &["package", "inspect_matches_discovery_report"])
                .unwrap_or(false)
            && json_usize(json_at(&promotion, &["discovery_gate", "false_accepts"]))
                .unwrap_or(usize::MAX)
                == 0
            && json_usize(json_at(&promotion, &["discovery_gate", "wrong_wins"]))
                .unwrap_or(usize::MAX)
                == 0
            && json_usize(json_at(
                &promotion,
                &["discovery_gate", "runtime_margin_parity_mismatches"],
            ))
            .unwrap_or(usize::MAX)
                == 0;
        if !allowed {
            skipped_reports.push(serde_json::json!({
                "path": input_report_path.display().to_string(),
                "report_kind": report_kind,
                "reason": "promotion_audit_not_safe",
            }));
            continue;
        }
        let discovery_report_path = PathBuf::from(
            json_string(&promotion, &["discovery_report_path"]).ok_or_else(|| {
                format!(
                    "promotion report '{}' missing discovery_report_path",
                    input_report_path.display()
                )
            })?,
        );
        let discovery = read_json_value(&discovery_report_path)?;
        let action_family_atom = json_string(&promotion, &["action_family"])
            .or_else(|| json_string(&promotion, &["discovery_gate", "action_family"]))
            .or_else(|| json_string(&discovery, &["action_family"]))
            .unwrap_or_else(|| "action_family:run_check".to_owned());
        let task_name = action_family_atom
            .strip_prefix("action_family:")
            .unwrap_or(action_family_atom.as_str())
            .replace(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_', "_");
        let candidate_package_path = PathBuf::from(
            json_string(&promotion, &["candidate_package_path"]).ok_or_else(|| {
                format!(
                    "promotion report '{}' missing candidate_package_path",
                    input_report_path.display()
                )
            })?,
        );
        let package_bytes = std::fs::read(&candidate_package_path).map_err(|error| {
            format!(
                "failed to read frontier package '{}': {error}",
                candidate_package_path.display()
            )
        })?;
        let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes)
            .map_err(|error| {
                format!(
                    "frontier package inspect error '{}': {error:?}",
                    candidate_package_path.display()
                )
            })?;
        let expected_package_fingerprint64 =
            json_u64(&promotion, &["package", "package_fingerprint64"]).unwrap_or(0);
        let margin_threshold_micro = json_i64(&promotion, &["margin_threshold_micro"])
            .unwrap_or_else(|| {
                json_i64(&discovery, &["margin_threshold_micro"])
                    .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO)
            });
        let runtime = PhaseCenterOffloadRuntime::from_package_bytes(
            &package_bytes,
            PhaseCenterOffloadPolicy::new(margin_threshold_micro)
                .map_err(|error| format!("frontier invalid policy: {error:?}"))?,
        )
        .map_err(|error| format!("frontier package load error: {error:?}"))?;
        let runtime_cells = runtime.cells();
        let runtime_record_count = runtime.record_count();
        let profile_ids = (0..runtime_record_count as u32).collect::<Vec<_>>();
        let thresholds = vec![margin_threshold_micro; runtime_record_count];
        let hot_runtime =
            PhaseCenterHotRuntime::from_flat_runtime(runtime.runtime(), &profile_ids, &thresholds)
                .map_err(|error| format!("frontier hot runtime build error: {error:?}"))?;
        let hot_route = hot_runtime
            .route_plan_from_profile_ids(0, profile_ids.iter().copied())
            .map_err(|error| format!("frontier hot route build error: {error:?}"))?
            .ok_or_else(|| "frontier hot route has no profiles".to_owned())?;
        let hot_routes = PhaseCenterHotRouteTable::from_plans([hot_route])
            .map_err(|error| format!("frontier hot route table error: {error:?}"))?;
        let hot_scratch = PhaseCenterHotScratch::new(runtime_cells, runtime_record_count)
            .map_err(|error| format!("frontier hot scratch error: {error:?}"))?;

        for input_trace in discovery
            .get("input_trace_paths")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
        {
            default_trace_paths.insert(PathBuf::from(input_trace));
        }
        profiles.push(FrontierShadowReplayProfile {
            input_report_path: input_report_path.display().to_string(),
            discovery_report_path: discovery_report_path.display().to_string(),
            action_family_atom,
            task_name,
            candidate_package_path: candidate_package_path.display().to_string(),
            expected_package_fingerprint64,
            package_fingerprint64: package_info.fingerprint64,
            package_bytes: package_bytes.len(),
            package_records: package_info.record_count,
            package_matches_report: expected_package_fingerprint64 == 0
                || expected_package_fingerprint64 == package_info.fingerprint64,
            margin_threshold_micro,
            train_time_max: json_string(&discovery, &["train_time_max"]).unwrap_or_default(),
            heldout_time_min: json_string(&discovery, &["heldout_time_min"]).unwrap_or_default(),
            runtime,
            hot_runtime,
            hot_routes,
            hot_scratch,
            runtime_record_count,
            events: Vec::new(),
        });
    }
    if profiles.is_empty() {
        return Err("frontier shadow replay found no safe phase-atom .nwpc profiles".to_owned());
    }
    let trace_paths = if explicit_trace_paths.is_empty() {
        default_trace_paths.into_iter().collect::<Vec<_>>()
    } else {
        explicit_trace_paths
    };
    if trace_paths.is_empty() {
        return Err("frontier shadow replay has no trace paths".to_owned());
    }

    let mut total_rows = 0usize;
    let mut unique_request_fingerprints = BTreeSet::new();
    let mut exact_cache_seen = BTreeSet::<String>::new();
    let mut total_tokens = 0usize;
    let mut total_cost_microusd = 0u64;
    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read frontier shadow trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows += 1;
            if total_rows.is_multiple_of(1000) {
                println!("  frontier_shadow_rows_scanned: {total_rows}");
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse frontier shadow trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            if let Some(fingerprint) = json_string(&row, &["request_fingerprint"]) {
                unique_request_fingerprints.insert(fingerprint);
            }
            let exact_cache_key = json_string(&row, &["exact_cache_key"])
                .or_else(|| json_string(&row, &["request_fingerprint"]))
                .unwrap_or_else(|| format!("frontier_shadow_row:{total_rows}"));
            let exact_cache_hit = exact_cache_seen.contains(&exact_cache_key);
            exact_cache_seen.insert(exact_cache_key);
            let token_cost = phase_atom_binary_token_cost(&row);
            total_tokens = total_tokens.saturating_add(token_cost.total_tokens);
            total_cost_microusd =
                total_cost_microusd.saturating_add(token_cost.total_cost_microusd);

            for (profile_index, profile) in profiles.iter_mut().enumerate() {
                let Some(event) = parse_phase_atom_binary_event_for_action(
                    &row,
                    profile.events.len(),
                    &profile.action_family_atom,
                    &profile.task_name,
                ) else {
                    continue;
                };
                if !profile.train_time_max.is_empty()
                    && event.event_timestamp.as_str() <= profile.train_time_max.as_str()
                {
                    continue;
                }
                if !profile.heldout_time_min.is_empty()
                    && event.event_timestamp.as_str() < profile.heldout_time_min.as_str()
                {
                    continue;
                }
                profile.events.push(FrontierRoutedEvent {
                    profile_index,
                    denominator_row_index: total_rows - 1,
                    source_trace_path: trace_path.display().to_string(),
                    source_line_index: line_index,
                    exact_cache_hit,
                    event,
                });
            }
        }
    }

    if let Some(parent) = decision_log_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create frontier decision log dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let mut decision_log = std::fs::File::create(&decision_log_path).map_err(|error| {
        format!(
            "failed to create frontier decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;

    let mut profile_reports = Vec::new();
    let mut routed_events = 0usize;
    let mut local_operator_shadow_decisions = 0usize;
    let mut fallback_shadow_decisions = 0usize;
    let mut exact_cache_hits_in_routed_events = 0usize;
    let mut unique_cpu_accepts_over_exact_cache = 0usize;
    let mut nando_cpu_tokens_saved = 0usize;
    let mut nando_cpu_cost_saved_microusd = 0u64;
    let mut estimated_nando_cpu_cost_saved_microusd = 0u64;
    let mut provider_cost_missing_cpu_accepts = 0usize;
    let mut verified_safe_local_decisions = 0usize;
    let mut verified_reject_local_decisions = 0usize;
    let mut false_accepts = 0usize;
    let mut wrong_wins = 0usize;
    let mut runtime_margin_parity_checks = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut runtime_decision_parity_mismatches = 0usize;
    let mut latencies = Vec::new();
    let mut margins = Vec::new();
    let mut frontier_events_scored = 0usize;
    for profile in &mut profiles {
        routed_events = routed_events.saturating_add(profile.events.len());
        let mut profile_local = 0usize;
        let mut profile_fallback = 0usize;
        let mut profile_unique = 0usize;
        let mut profile_false = 0usize;
        let mut profile_wrong = 0usize;
        let mut profile_safe_local = 0usize;
        let mut profile_reject_local = 0usize;
        let profile_events = profile.events.clone();
        for (heldout_position, routed) in profile_events.iter().enumerate() {
            frontier_events_scored += 1;
            if frontier_events_scored.is_multiple_of(1000) {
                println!("  frontier_shadow_events_scored: {frontier_events_scored}");
            }
            let event = &routed.event;
            let program_index = usize::from(!event.verified_safe_accept);
            if program_index >= profile.runtime_record_count {
                return Err(format!(
                    "frontier program_index {program_index} out of bounds for record_count {}",
                    profile.runtime_record_count
                ));
            }
            let correct_vec = phase_atom_binary_event_vector_for_task(
                event,
                event.verified_safe_accept,
                profile.runtime.cells(),
                &profile.task_name,
            );
            let wrong_vec = phase_atom_binary_event_vector_for_task(
                event,
                !event.verified_safe_accept,
                profile.runtime.cells(),
                &profile.task_name,
            );
            let task = PhaseCenterEvalTask {
                center_index: program_index,
                correct_vec: correct_vec.into_boxed_slice(),
                wrong_vec: wrong_vec.into_boxed_slice(),
            };
            let started = Instant::now();
            let reference_decision = profile
                .runtime
                .offload_decision(&task)
                .map_err(|error| format!("frontier shadow offload decision error: {error:?}"))?;
            let correct_vec = task.correct_vec.to_vec();
            let wrong_vec = task.wrong_vec.to_vec();
            let prepared_delta_vec = phase_delta_vector(&correct_vec, &wrong_vec);
            let candidates = profile
                .hot_runtime
                .score_prepared_hot_request_candidates(
                    &profile.hot_routes,
                    PhaseCenterPreparedHotRequest::new(0, &prepared_delta_vec),
                    &mut profile.hot_scratch,
                )
                .map_err(|error| {
                    format!("frontier shadow prepared hot decision error: {error:?}")
                })?;
            latencies.push(started.elapsed().as_nanos());
            let Some(decision) = candidates.get(program_index) else {
                return Err(format!(
                    "frontier shadow missing hot candidate for program_index {program_index}"
                ));
            };
            runtime_margin_parity_checks += 1;
            if decision.margin_micro != reference_decision.margin_micro {
                runtime_margin_parity_mismatches += 1;
            }
            let reference_local_operator =
                reference_decision.action == nando_core::PhaseCenterOffloadAction::LocalOperator;
            if decision.score_candidate != reference_local_operator {
                runtime_decision_parity_mismatches += 1;
            }
            margins.push(decision.margin_micro);
            let local_operator = decision.score_candidate;
            let correct = decision.margin_micro > 0;
            let false_accept = local_operator && !correct;
            let unique_accept = local_operator && correct && !routed.exact_cache_hit;
            wrong_wins += usize::from(!correct);
            profile_wrong += usize::from(!correct);
            false_accepts += usize::from(false_accept);
            profile_false += usize::from(false_accept);
            exact_cache_hits_in_routed_events += usize::from(routed.exact_cache_hit);
            if local_operator {
                local_operator_shadow_decisions += 1;
                profile_local += 1;
                if correct && event.verified_safe_accept {
                    verified_safe_local_decisions += 1;
                    profile_safe_local += 1;
                }
                if correct && !event.verified_safe_accept {
                    verified_reject_local_decisions += 1;
                    profile_reject_local += 1;
                }
                if unique_accept {
                    unique_cpu_accepts_over_exact_cache += 1;
                    profile_unique += 1;
                    nando_cpu_tokens_saved =
                        nando_cpu_tokens_saved.saturating_add(event.token_cost.total_tokens);
                    nando_cpu_cost_saved_microusd = nando_cpu_cost_saved_microusd
                        .saturating_add(event.token_cost.total_cost_microusd);
                    if event.token_cost.total_cost_microusd == 0 {
                        provider_cost_missing_cpu_accepts += 1;
                        estimated_nando_cpu_cost_saved_microusd =
                            estimated_nando_cpu_cost_saved_microusd
                                .saturating_add(event.token_cost.total_tokens as u64);
                    } else {
                        estimated_nando_cpu_cost_saved_microusd =
                            estimated_nando_cpu_cost_saved_microusd
                                .saturating_add(event.token_cost.total_cost_microusd);
                    }
                }
            } else {
                fallback_shadow_decisions += 1;
                profile_fallback += 1;
            }
            let decision_row = serde_json::json!({
                "schema_version": "phase_atom_frontier_shadow_replay_decision_v1",
                "profile_task_name": profile.task_name,
                "profile_action_family": profile.action_family_atom,
                "profile_index": routed.profile_index,
                "heldout_position": heldout_position,
                "denominator_row_index": routed.denominator_row_index,
                "source_trace_path": routed.source_trace_path,
                "source_line_index": routed.source_line_index,
                "request_fingerprint": event.request_fingerprint,
                "exact_cache_key": event.exact_cache_key,
                "exact_cache_hit": routed.exact_cache_hit,
                "verified_safe_accept": event.verified_safe_accept,
                "margin_micro": decision.margin_micro,
                "reference_margin_micro": reference_decision.margin_micro,
                "local_operator_shadow_decision": local_operator && correct,
                "fallback_shadow_decision": !local_operator,
                "false_accept": false_accept,
                "wrong_win": !correct,
                "unique_cpu_accept_over_exact_cache": unique_accept,
                "token_cost": event.token_cost,
                "package_fingerprint64": profile.package_fingerprint64,
                "runtime_margin_parity_match": decision.margin_micro == reference_decision.margin_micro,
                "runtime_decision_parity_match": decision.score_candidate == reference_local_operator,
                "local_accept_enabled": false,
            });
            writeln!(
                decision_log,
                "{}",
                serde_json::to_string(&decision_row).map_err(|error| format!(
                    "frontier decision log serialization error: {error}"
                ))?
            )
            .map_err(|error| format!("failed to write frontier decision log: {error}"))?;
        }
        profile_reports.push(serde_json::json!({
            "input_report_path": profile.input_report_path,
            "discovery_report_path": profile.discovery_report_path,
            "action_family_atom": profile.action_family_atom,
            "task_name": profile.task_name,
            "candidate_package_path": profile.candidate_package_path,
            "expected_package_fingerprint64": profile.expected_package_fingerprint64,
            "package_fingerprint64": profile.package_fingerprint64,
            "package_matches_report": profile.package_matches_report,
            "package_bytes": profile.package_bytes,
            "package_records": profile.package_records,
            "runtime_cells": profile.runtime.cells(),
            "runtime_record_count": profile.runtime.record_count(),
            "runtime_bytes_estimate": profile.runtime.bytes_estimate(),
            "margin_threshold_micro": profile.margin_threshold_micro,
            "train_time_max": profile.train_time_max,
            "heldout_time_min": profile.heldout_time_min,
            "events_seen_after_train_window": profile.events.len(),
            "local_operator_shadow_decisions": profile_local,
            "fallback_shadow_decisions": profile_fallback,
            "verified_safe_local_decisions": profile_safe_local,
            "verified_reject_local_decisions": profile_reject_local,
            "unique_cpu_accepts_over_exact_cache": profile_unique,
            "false_accepts": profile_false,
            "wrong_wins": profile_wrong,
            "runtime_margin_parity_checks": profile.events.len(),
        }));
    }
    margins.sort_unstable();
    latencies.sort_unstable();
    let all_packages_match = profiles
        .iter()
        .all(|profile| profile.package_matches_report);
    let shadow_gate_passed = all_packages_match
        && routed_events > 0
        && unique_cpu_accepts_over_exact_cache > 0
        && false_accepts == 0
        && wrong_wins == 0
        && runtime_margin_parity_checks == routed_events
        && runtime_margin_parity_mismatches == 0
        && runtime_decision_parity_mismatches == 0;
    let cpu10_reached = per_thousand(unique_cpu_accepts_over_exact_cache, total_rows) >= 100;
    let runtime_replay_passed = shadow_gate_passed;
    let manual_class_list_used = true;
    let static_topn_seed_used = false;
    let online_discovery_used = false;
    let marginal_denominator_delta_used = false;
    let portfolio_gate_passed = false;
    let product_dynamic_discovery_claim_allowed = !manual_class_list_used
        && !static_topn_seed_used
        && online_discovery_used
        && marginal_denominator_delta_used
        && portfolio_gate_passed
        && runtime_replay_passed;
    let forbidden_flags = serde_json::json!({
        "nwrb_used": false,
        "role_binding_backend_used": false,
        "lookup_used": false,
        "target_id_or_proof_rule_id_authority_used": false,
        "concrete_x_lookup_used": false,
        "manual_local_out_t_used": false,
        "local_accept_without_verifier_used": false
    });
    let denominator_report = serde_json::json!({
        "total_rows": total_rows,
        "unique_request_fingerprints": unique_request_fingerprints.len(),
        "total_tokens": total_tokens,
        "total_cost_microusd": total_cost_microusd,
    });
    let verdict = if shadow_gate_passed {
        "PHASE_ATOM_FRONTIER_SHADOW_REPLAY_V1_PASS_SHADOW_ONLY"
    } else {
        "PHASE_ATOM_FRONTIER_SHADOW_REPLAY_V1_WATCH"
    };
    let mut report_map = serde_json::Map::new();
    report_map.insert(
        "report_kind".to_owned(),
        serde_json::Value::String("phase_atom_frontier_shadow_replay_v1".to_owned()),
    );
    report_map.insert(
        "mode".to_owned(),
        serde_json::Value::String(
            "safe_phase_atom_frontier_nwpc_shadow_replay_against_compatible_trace_denominator"
                .to_owned(),
        ),
    );
    report_map.insert(
        "shadow_runtime_kind".to_owned(),
        serde_json::Value::String("phase_center_prepared_hot_runtime_registry".to_owned()),
    );
    report_map.insert(
        "frontier_report_path".to_owned(),
        serde_json::Value::String(frontier_report_path.display().to_string()),
    );
    report_map.insert(
        "decision_log_path".to_owned(),
        serde_json::Value::String(decision_log_path.display().to_string()),
    );
    report_map.insert(
        "trace_paths".to_owned(),
        serde_json::Value::Array(
            trace_paths
                .iter()
                .map(|path| serde_json::Value::String(path.display().to_string()))
                .collect(),
        ),
    );
    report_map.insert(
        "skipped_reports".to_owned(),
        serde_json::Value::Array(skipped_reports),
    );
    report_map.insert("denominator".to_owned(), denominator_report);
    report_map.insert(
        "profiles".to_owned(),
        serde_json::Value::Array(profile_reports),
    );
    report_map.insert(
        "profile_count".to_owned(),
        serde_json::json!(profiles.len()),
    );
    report_map.insert(
        "routed_events_after_train_window".to_owned(),
        serde_json::json!(routed_events),
    );
    report_map.insert(
        "local_operator_shadow_decisions".to_owned(),
        serde_json::json!(local_operator_shadow_decisions),
    );
    report_map.insert(
        "fallback_shadow_decisions".to_owned(),
        serde_json::json!(fallback_shadow_decisions),
    );
    report_map.insert(
        "verified_safe_local_decisions".to_owned(),
        serde_json::json!(verified_safe_local_decisions),
    );
    report_map.insert(
        "verified_reject_local_decisions".to_owned(),
        serde_json::json!(verified_reject_local_decisions),
    );
    report_map.insert(
        "exact_cache_hits_in_routed_events".to_owned(),
        serde_json::json!(exact_cache_hits_in_routed_events),
    );
    report_map.insert(
        "unique_cpu_accepts_over_exact_cache".to_owned(),
        serde_json::json!(unique_cpu_accepts_over_exact_cache),
    );
    report_map.insert(
        "calls_saved_pct".to_owned(),
        serde_json::json!(percent_u64(
            unique_cpu_accepts_over_exact_cache as u64,
            total_rows as u64,
        )),
    );
    report_map.insert(
        "calls_saved_milli".to_owned(),
        serde_json::json!(per_thousand(
            unique_cpu_accepts_over_exact_cache,
            total_rows
        )),
    );
    report_map.insert("cpu10_reached".to_owned(), serde_json::json!(cpu10_reached));
    report_map.insert(
        "nando_cpu_tokens_saved".to_owned(),
        serde_json::json!(nando_cpu_tokens_saved),
    );
    report_map.insert(
        "tokens_saved_pct".to_owned(),
        serde_json::json!(percent_u64(
            nando_cpu_tokens_saved as u64,
            total_tokens as u64
        )),
    );
    report_map.insert(
        "nando_cpu_cost_saved_microusd".to_owned(),
        serde_json::json!(nando_cpu_cost_saved_microusd),
    );
    report_map.insert(
        "estimated_nando_cpu_cost_saved_microusd".to_owned(),
        serde_json::json!(estimated_nando_cpu_cost_saved_microusd),
    );
    report_map.insert(
        "provider_cost_missing_cpu_accepts".to_owned(),
        serde_json::json!(provider_cost_missing_cpu_accepts),
    );
    report_map.insert(
        "money_claim_blocker".to_owned(),
        serde_json::Value::String(
            "provider_cost_missing_for_some_cpu_accepts_or_offline_shadow_only".to_owned(),
        ),
    );
    report_map.insert("false_accepts".to_owned(), serde_json::json!(false_accepts));
    report_map.insert("wrong_wins".to_owned(), serde_json::json!(wrong_wins));
    report_map.insert(
        "runtime_margin_parity_checks".to_owned(),
        serde_json::json!(runtime_margin_parity_checks),
    );
    report_map.insert(
        "runtime_margin_parity_mismatches".to_owned(),
        serde_json::json!(runtime_margin_parity_mismatches),
    );
    report_map.insert(
        "runtime_decision_parity_mismatches".to_owned(),
        serde_json::json!(runtime_decision_parity_mismatches),
    );
    report_map.insert(
        "min_margin_micro".to_owned(),
        serde_json::json!(margins.first().copied().unwrap_or(0)),
    );
    report_map.insert(
        "p10_margin_micro".to_owned(),
        serde_json::json!(percentile_i64(&margins, 10)),
    );
    report_map.insert(
        "median_margin_micro".to_owned(),
        serde_json::json!(percentile_i64(&margins, 50)),
    );
    report_map.insert(
        "latency_p50_ns".to_owned(),
        serde_json::json!(percentile_u128(&latencies, 50)),
    );
    report_map.insert(
        "latency_p90_ns".to_owned(),
        serde_json::json!(percentile_u128(&latencies, 90)),
    );
    report_map.insert(
        "latency_p99_ns".to_owned(),
        serde_json::json!(percentile_u128(&latencies, 99)),
    );
    report_map.insert(
        "latency_max_ns".to_owned(),
        serde_json::json!(latencies.last().copied().unwrap_or(0)),
    );
    report_map.insert(
        "shadow_gate_passed".to_owned(),
        serde_json::json!(shadow_gate_passed),
    );
    report_map.insert(
        "manual_class_list_used".to_owned(),
        serde_json::json!(manual_class_list_used),
    );
    report_map.insert(
        "static_topn_seed_used".to_owned(),
        serde_json::json!(static_topn_seed_used),
    );
    report_map.insert(
        "online_discovery_used".to_owned(),
        serde_json::json!(online_discovery_used),
    );
    report_map.insert(
        "marginal_denominator_delta_used".to_owned(),
        serde_json::json!(marginal_denominator_delta_used),
    );
    report_map.insert(
        "portfolio_gate_passed".to_owned(),
        serde_json::json!(portfolio_gate_passed),
    );
    report_map.insert(
        "runtime_replay_passed".to_owned(),
        serde_json::json!(runtime_replay_passed),
    );
    report_map.insert(
        "product_dynamic_discovery_claim_allowed".to_owned(),
        serde_json::json!(product_dynamic_discovery_claim_allowed),
    );
    report_map.insert(
        "discovery_mode".to_owned(),
        serde_json::json!({
            "manual_class_list_used": manual_class_list_used,
            "static_topn_seed_used": static_topn_seed_used,
            "online_discovery_used": online_discovery_used,
            "marginal_denominator_delta_used": marginal_denominator_delta_used,
            "portfolio_gate_passed": portfolio_gate_passed,
            "runtime_replay_passed": runtime_replay_passed,
            "product_dynamic_discovery_claim_allowed": product_dynamic_discovery_claim_allowed,
            "claim_boundary": "manual frontier/package list is a debug shadow, not dynamic product discovery"
        }),
    );
    report_map.insert("local_accept_enabled".to_owned(), serde_json::json!(false));
    report_map.insert("auto_promote_enabled".to_owned(), serde_json::json!(false));
    report_map.insert(
        "product_promotion_allowed".to_owned(),
        serde_json::json!(false),
    );
    report_map.insert(
        "market_money_claim_allowed".to_owned(),
        serde_json::json!(false),
    );
    report_map.insert("forbidden_flags".to_owned(), forbidden_flags);
    report_map.insert(
        "verdict".to_owned(),
        serde_json::Value::String(verdict.to_owned()),
    );
    report_map.insert(
        "boundary".to_owned(),
        serde_json::Value::String("shadow replay only: loads safe phase-atom promotion-audit .nwpc packages into a prepared-hot runtime registry, counts only rows after each package train window, writes decision logs, verifies reference parity, and never enables local_accept, promotes runtime, claims provider-billing money, revives legacy nwrb, uses lookup, target/proof authority, concrete_x_lookup, or manual local_out_t".to_owned()),
    );
    let report = serde_json::Value::Object(report_map);
    write_json_file(&report_path, &report)?;
    println!("phase_atom_frontier_shadow_replay_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  decision_log_path: {}", decision_log_path.display());
    println!("  denominator_rows: {total_rows}");
    println!("  profile_count: {}", profiles.len());
    println!("  routed_events_after_train_window: {routed_events}");
    println!("  unique_cpu_accepts_over_exact_cache: {unique_cpu_accepts_over_exact_cache}");
    println!(
        "  calls_saved_milli: {}",
        per_thousand(unique_cpu_accepts_over_exact_cache, total_rows)
    );
    println!("  false_accepts: {false_accepts}");
    println!("  wrong_wins: {wrong_wins}");
    println!("  runtime_margin_parity_mismatches: {runtime_margin_parity_mismatches}");
    println!("  runtime_decision_parity_mismatches: {runtime_decision_parity_mismatches}");
    println!("  cpu10_reached: {cpu10_reached}");
    println!("  local_accept_enabled: false");
    println!(
        "  verdict: {}",
        report
            .get("verdict")
            .and_then(|value| value.as_str())
            .unwrap_or("UNKNOWN")
    );
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_frontier_claim_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_FRONTIER_CLAIM_AUDIT_REPORT));
    let replay_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_FRONTIER_SHADOW_REPLAY_REPORT));
    let replay = read_json_value(&replay_report_path)?;

    let total_rows = json_u64(&replay, &["denominator", "total_rows"])
        .or_else(|| json_u64(&replay, &["denominator_rows"]))
        .unwrap_or(0);
    let total_tokens = json_u64(&replay, &["denominator", "total_tokens"]).unwrap_or(0);
    let unique_request_fingerprints =
        json_u64(&replay, &["denominator", "unique_request_fingerprints"]).unwrap_or(0);
    let unique_cpu_accepts_over_exact_cache =
        json_u64(&replay, &["unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let nando_cpu_tokens_saved = json_u64(&replay, &["nando_cpu_tokens_saved"]).unwrap_or(0);
    let nando_cpu_cost_saved_microusd =
        json_u64(&replay, &["nando_cpu_cost_saved_microusd"]).unwrap_or(0);
    let estimated_nando_cpu_cost_saved_microusd =
        json_u64(&replay, &["estimated_nando_cpu_cost_saved_microusd"]).unwrap_or(0);
    let provider_cost_missing_cpu_accepts =
        json_u64(&replay, &["provider_cost_missing_cpu_accepts"]).unwrap_or(0);
    let false_accepts = json_u64(&replay, &["false_accepts"]).unwrap_or(0);
    let wrong_wins = json_u64(&replay, &["wrong_wins"]).unwrap_or(0);
    let shadow_runtime_kind = json_string(&replay, &["shadow_runtime_kind"])
        .unwrap_or_else(|| "legacy_or_unspecified_shadow_runtime".to_owned());
    let runtime_margin_parity_checks =
        json_u64(&replay, &["runtime_margin_parity_checks"]).unwrap_or(0);
    let runtime_margin_parity_mismatches =
        json_u64(&replay, &["runtime_margin_parity_mismatches"]).unwrap_or(0);
    let runtime_decision_parity_mismatches =
        json_u64(&replay, &["runtime_decision_parity_mismatches"]).unwrap_or(0);
    let local_accept_enabled = json_bool(&replay, &["local_accept_enabled"]).unwrap_or(false);
    let market_money_claim_allowed =
        json_bool(&replay, &["market_money_claim_allowed"]).unwrap_or(false);
    let shadow_gate_passed = json_bool(&replay, &["shadow_gate_passed"]).unwrap_or(false);
    let replay_verdict = json_string(&replay, &["verdict"]).unwrap_or_else(|| "UNKNOWN".to_owned());
    let manual_class_list_used = json_bool(&replay, &["discovery_mode", "manual_class_list_used"])
        .or_else(|| json_bool(&replay, &["manual_class_list_used"]))
        .unwrap_or(true);
    let static_topn_seed_used = json_bool(&replay, &["discovery_mode", "static_topn_seed_used"])
        .or_else(|| json_bool(&replay, &["static_topn_seed_used"]))
        .unwrap_or(false);
    let online_discovery_used = json_bool(&replay, &["discovery_mode", "online_discovery_used"])
        .or_else(|| json_bool(&replay, &["online_discovery_used"]))
        .unwrap_or(false);
    let marginal_denominator_delta_used = json_bool(
        &replay,
        &["discovery_mode", "marginal_denominator_delta_used"],
    )
    .or_else(|| json_bool(&replay, &["marginal_denominator_delta_used"]))
    .unwrap_or(false);
    let portfolio_gate_passed = json_bool(&replay, &["discovery_mode", "portfolio_gate_passed"])
        .or_else(|| json_bool(&replay, &["portfolio_gate_passed"]))
        .unwrap_or(false);

    let forbidden_flag_paths = [
        "nwrb_used",
        "role_binding_backend_used",
        "lookup_used",
        "target_id_or_proof_rule_id_authority_used",
        "concrete_x_lookup_used",
        "manual_local_out_t_used",
        "local_accept_without_verifier_used",
    ];
    let mut forbidden_flag_rows = Vec::new();
    let mut forbidden_flags_clean = true;
    for flag in forbidden_flag_paths {
        let used = json_bool(&replay, &["forbidden_flags", flag]).unwrap_or(false);
        forbidden_flags_clean &= !used;
        forbidden_flag_rows.push(serde_json::json!({
            "flag": flag,
            "used": used,
        }));
    }

    let mut profile_rows = replay
        .get("profiles")
        .and_then(|value| value.as_array())
        .map(|profiles| {
            profiles
                .iter()
                .map(|profile| {
                    let task_name = profile
                        .get("task_name")
                        .and_then(|value| value.as_str())
                        .unwrap_or("unknown");
                    let accepts =
                        json_u64(profile, &["unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
                    let events_seen =
                        json_u64(profile, &["events_seen_after_train_window"]).unwrap_or(0);
                    let false_accepts = json_u64(profile, &["false_accepts"]).unwrap_or(0);
                    let wrong_wins = json_u64(profile, &["wrong_wins"]).unwrap_or(0);
                    serde_json::json!({
                        "task_name": task_name,
                        "events_seen_after_train_window": events_seen,
                        "unique_cpu_accepts_over_exact_cache": accepts,
                        "share_of_cpu_accepts_milli": per_thousand(
                            accepts as usize,
                            unique_cpu_accepts_over_exact_cache as usize
                        ),
                        "false_accepts": false_accepts,
                        "wrong_wins": wrong_wins,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    profile_rows.sort_by(|left, right| {
        right["unique_cpu_accepts_over_exact_cache"]
            .as_u64()
            .unwrap_or(0)
            .cmp(
                &left["unique_cpu_accepts_over_exact_cache"]
                    .as_u64()
                    .unwrap_or(0),
            )
            .then_with(|| {
                left["task_name"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(right["task_name"].as_str().unwrap_or(""))
            })
    });
    let contributing_profile_count = profile_rows
        .iter()
        .filter(|row| {
            row["unique_cpu_accepts_over_exact_cache"]
                .as_u64()
                .unwrap_or(0)
                > 0
        })
        .count();
    let top_profile_accepts = profile_rows
        .first()
        .and_then(|row| row["unique_cpu_accepts_over_exact_cache"].as_u64())
        .unwrap_or(0);
    let top_profile_name = profile_rows
        .first()
        .and_then(|row| row["task_name"].as_str())
        .unwrap_or("none")
        .to_owned();
    let top_profile_share_milli = per_thousand(
        top_profile_accepts as usize,
        unique_cpu_accepts_over_exact_cache as usize,
    );
    let non_top_profile_accepts =
        unique_cpu_accepts_over_exact_cache.saturating_sub(top_profile_accepts);

    let calls_saved_milli = per_thousand(
        unique_cpu_accepts_over_exact_cache as usize,
        total_rows as usize,
    );
    let tokens_saved_milli = per_thousand(nando_cpu_tokens_saved as usize, total_tokens as usize);
    let cpu10_gate_passed = calls_saved_milli >= 100;
    let runtime_parity_gate_passed = shadow_runtime_kind
        == "phase_center_prepared_hot_runtime_registry"
        && runtime_margin_parity_checks > 0
        && runtime_margin_parity_mismatches == 0
        && runtime_decision_parity_mismatches == 0;
    let runtime_replay_passed = json_bool(&replay, &["discovery_mode", "runtime_replay_passed"])
        .or_else(|| json_bool(&replay, &["runtime_replay_passed"]))
        .unwrap_or(runtime_parity_gate_passed);
    let safety_gate_passed = shadow_gate_passed
        && false_accepts == 0
        && wrong_wins == 0
        && forbidden_flags_clean
        && runtime_parity_gate_passed;
    let product_promotion_allowed =
        json_bool(&replay, &["product_promotion_allowed"]).unwrap_or(false);
    let shadow_only_boundary_ok = !local_accept_enabled && !product_promotion_allowed;
    let money_claim_gate_passed = market_money_claim_allowed
        && provider_cost_missing_cpu_accepts == 0
        && nando_cpu_cost_saved_microusd > 0;
    let profile_diversity_gate_passed =
        contributing_profile_count >= 3 && top_profile_share_milli <= 800;
    let product_dynamic_discovery_claim_allowed = !manual_class_list_used
        && !static_topn_seed_used
        && online_discovery_used
        && marginal_denominator_delta_used
        && portfolio_gate_passed
        && runtime_replay_passed;
    let daemon_promotion_ready = cpu10_gate_passed
        && safety_gate_passed
        && profile_diversity_gate_passed
        && money_claim_gate_passed
        && product_dynamic_discovery_claim_allowed
        && shadow_only_boundary_ok;

    let next_actions = if !product_dynamic_discovery_claim_allowed {
        vec![
            "replace manual/debug frontier selection with automatic stream discovery",
            "require marginal denominator delta and portfolio gate before product compression claim",
            "keep local_accept disabled while discovery-mode gate is red",
        ]
    } else if !profile_diversity_gate_passed {
        vec![
            "mine additional verifier-bound action families beyond the top profile",
            "rerun frontier shadow replay with diversified .nwpc candidates",
            "keep local_accept disabled until diversity, safety, and money gates pass",
        ]
    } else if !runtime_parity_gate_passed {
        vec![
            "replay frontier through prepared-hot runtime registry",
            "require runtime margin and decision parity = 0 before any promotion discussion",
        ]
    } else if !money_claim_gate_passed {
        vec![
            "attach provider billing evidence to CPU-accepted rows",
            "rerun claim audit with provider_cost_missing_cpu_accepts = 0",
            "keep market money claim blocked until real provider cost evidence exists",
        ]
    } else if !safety_gate_passed {
        vec![
            "repair safety or forbidden-path gate",
            "rerun shadow replay before any promotion discussion",
        ]
    } else {
        vec!["ready for a separate promotion-policy audit; do not auto-enable local_accept"]
    };

    let verdict = if daemon_promotion_ready {
        "PHASE_ATOM_FRONTIER_CLAIM_AUDIT_PROMOTION_READY_REVIEW_ONLY"
    } else if cpu10_gate_passed && safety_gate_passed {
        "PHASE_ATOM_FRONTIER_CLAIM_AUDIT_CPU10_SHADOW_PASS_CLAIM_BLOCKED"
    } else if safety_gate_passed {
        "PHASE_ATOM_FRONTIER_CLAIM_AUDIT_SAFE_SHADOW_NEEDS_COVERAGE"
    } else {
        "PHASE_ATOM_FRONTIER_CLAIM_AUDIT_REPAIR_REQUIRED"
    };

    let report = serde_json::json!({
        "report_kind": "phase_atom_frontier_claim_audit_v1",
        "replay_report_path": replay_report_path,
        "replay_verdict": replay_verdict,
        "denominator": {
            "total_rows": total_rows,
            "total_tokens": total_tokens,
            "unique_request_fingerprints": unique_request_fingerprints,
        },
        "score": {
            "unique_cpu_accepts_over_exact_cache": unique_cpu_accepts_over_exact_cache,
            "calls_saved_milli": calls_saved_milli,
            "calls_saved_pct": percent_u64(unique_cpu_accepts_over_exact_cache, total_rows),
            "nando_cpu_tokens_saved": nando_cpu_tokens_saved,
            "tokens_saved_milli": tokens_saved_milli,
            "tokens_saved_pct": percent_u64(nando_cpu_tokens_saved, total_tokens),
            "nando_cpu_cost_saved_microusd": nando_cpu_cost_saved_microusd,
            "estimated_nando_cpu_cost_saved_microusd": estimated_nando_cpu_cost_saved_microusd,
            "provider_cost_missing_cpu_accepts": provider_cost_missing_cpu_accepts,
        },
        "runtime": {
            "shadow_runtime_kind": shadow_runtime_kind,
            "runtime_margin_parity_checks": runtime_margin_parity_checks,
            "runtime_margin_parity_mismatches": runtime_margin_parity_mismatches,
            "runtime_decision_parity_mismatches": runtime_decision_parity_mismatches,
            "runtime_parity_gate_passed": runtime_parity_gate_passed,
        },
        "discovery_mode": {
            "manual_class_list_used": manual_class_list_used,
            "static_topn_seed_used": static_topn_seed_used,
            "online_discovery_used": online_discovery_used,
            "marginal_denominator_delta_used": marginal_denominator_delta_used,
            "portfolio_gate_passed": portfolio_gate_passed,
            "runtime_replay_passed": runtime_replay_passed,
            "product_dynamic_discovery_claim_allowed": product_dynamic_discovery_claim_allowed,
            "policy": "manual/debug class selection is WATCH and cannot count as product dynamic discovery",
        },
        "safety": {
            "shadow_gate_passed": shadow_gate_passed,
            "false_accepts": false_accepts,
            "wrong_wins": wrong_wins,
            "forbidden_flags_clean": forbidden_flags_clean,
            "forbidden_flags": forbidden_flag_rows,
            "local_accept_enabled": local_accept_enabled,
            "shadow_only_boundary_ok": shadow_only_boundary_ok,
        },
        "profile_diversity": {
            "contributing_profile_count": contributing_profile_count,
            "top_profile_name": top_profile_name,
            "top_profile_accepts": top_profile_accepts,
            "top_profile_share_milli": top_profile_share_milli,
            "non_top_profile_accepts": non_top_profile_accepts,
            "profile_diversity_gate_passed": profile_diversity_gate_passed,
            "policy": "PASS requires >=3 contributing profiles and top profile <=80% of CPU accepts",
            "profiles": profile_rows,
        },
        "gates": {
            "cpu10_gate_passed": cpu10_gate_passed,
            "safety_gate_passed": safety_gate_passed,
            "runtime_parity_gate_passed": runtime_parity_gate_passed,
            "runtime_replay_passed": runtime_replay_passed,
            "profile_diversity_gate_passed": profile_diversity_gate_passed,
            "product_dynamic_discovery_claim_allowed": product_dynamic_discovery_claim_allowed,
            "money_claim_gate_passed": money_claim_gate_passed,
            "daemon_promotion_ready": daemon_promotion_ready,
            "market_money_claim_allowed": market_money_claim_allowed,
            "product_promotion_allowed": false,
            "auto_promote_enabled": false,
            "local_accept_enabled": local_accept_enabled,
        },
        "next_actions": next_actions,
        "verdict": verdict,
        "boundary": "claim audit only: reads frontier .nwpc shadow replay reports, separates CPU10/safety/diversity/money gates, and never compiles packages, promotes runtime, enables local_accept, revives legacy nwrb, uses lookup, target/proof authority, concrete_x_lookup, or manual local_out_t",
    });
    write_json_file(&report_path, &report)?;
    println!("phase_atom_frontier_claim_audit_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  replay_report_path: {}", replay_report_path.display());
    println!("  unique_cpu_accepts_over_exact_cache: {unique_cpu_accepts_over_exact_cache}");
    println!("  calls_saved_milli: {calls_saved_milli}");
    println!("  cpu10_gate_passed: {cpu10_gate_passed}");
    println!("  profile_diversity_gate_passed: {profile_diversity_gate_passed}");
    println!("  money_claim_gate_passed: {money_claim_gate_passed}");
    println!("  false_accepts: {false_accepts}");
    println!("  wrong_wins: {wrong_wins}");
    println!(
        "  verdict: {}",
        report
            .get("verdict")
            .and_then(|value| value.as_str())
            .unwrap_or("UNKNOWN")
    );
    Ok(())
}

pub(crate) fn run_phase_stream_phase_atom_diversity_backlog_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_DIVERSITY_BACKLOG_REPORT));
    let claim_audit_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_FRONTIER_CLAIM_AUDIT_REPORT));
    let verifier_ranking_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PHASE_ATOM_VERIFIER_NEEDED_RANKING_REPORT));

    let claim = read_json_value(&claim_audit_path)?;
    let ranking = read_json_value(&verifier_ranking_path)?;

    let top_profile_name = json_string(&claim, &["profile_diversity", "top_profile_name"])
        .unwrap_or_else(|| "unknown".to_owned());
    let top_profile_accepts =
        json_u64(&claim, &["profile_diversity", "top_profile_accepts"]).unwrap_or(0);
    let current_unique_cpu_accepts =
        json_u64(&claim, &["score", "unique_cpu_accepts_over_exact_cache"]).unwrap_or(0);
    let current_non_top_accepts =
        json_u64(&claim, &["profile_diversity", "non_top_profile_accepts"])
            .unwrap_or_else(|| current_unique_cpu_accepts.saturating_sub(top_profile_accepts));
    let current_top_share_milli =
        json_u64(&claim, &["profile_diversity", "top_profile_share_milli"]).unwrap_or(0);
    let contributing_profile_count =
        json_u64(&claim, &["profile_diversity", "contributing_profile_count"]).unwrap_or(0);
    let false_accepts = json_u64(&claim, &["safety", "false_accepts"]).unwrap_or(0);
    let wrong_wins = json_u64(&claim, &["safety", "wrong_wins"]).unwrap_or(0);
    let safety_gate_passed = json_bool(&claim, &["gates", "safety_gate_passed"]).unwrap_or(false);
    let cpu10_gate_passed = json_bool(&claim, &["gates", "cpu10_gate_passed"]).unwrap_or(false);
    let money_claim_gate_passed =
        json_bool(&claim, &["gates", "money_claim_gate_passed"]).unwrap_or(false);
    let local_accept_enabled =
        json_bool(&claim, &["gates", "local_accept_enabled"]).unwrap_or(false);
    let market_money_claim_allowed =
        json_bool(&claim, &["gates", "market_money_claim_allowed"]).unwrap_or(false);

    let diversity_target_top_share_milli = 800u64;
    let min_contributing_profiles = 3u64;
    let required_total_for_top_share = if top_profile_accepts == 0 {
        0
    } else {
        ceil_div_u64(
            top_profile_accepts.saturating_mul(1000),
            diversity_target_top_share_milli,
        )
    };
    let required_non_top_accepts = required_total_for_top_share.saturating_sub(top_profile_accepts);
    let additional_non_top_accepts_needed =
        required_non_top_accepts.saturating_sub(current_non_top_accepts);
    let additional_contributing_profiles_needed =
        min_contributing_profiles.saturating_sub(contributing_profile_count);

    let mut profile_current = BTreeMap::<String, serde_json::Value>::new();
    if let Some(profiles) =
        json_at(&claim, &["profile_diversity", "profiles"]).and_then(serde_json::Value::as_array)
    {
        for profile in profiles {
            if let Some(task_name) = profile.get("task_name").and_then(serde_json::Value::as_str) {
                profile_current.insert(task_name.to_owned(), profile.clone());
            }
        }
    }

    let mut candidate_names = BTreeSet::<String>::new();
    for name in profile_current.keys() {
        candidate_names.insert(name.clone());
    }
    if let Some(families) = ranking
        .get("top_action_families")
        .and_then(serde_json::Value::as_array)
    {
        for family in families {
            if let Some(action_family) = family
                .get("action_family")
                .and_then(serde_json::Value::as_str)
            {
                candidate_names.insert(profile_name_from_action_family(action_family).to_owned());
            }
        }
    }

    let mut ranking_by_profile = BTreeMap::<String, serde_json::Value>::new();
    if let Some(families) = ranking
        .get("top_action_families")
        .and_then(serde_json::Value::as_array)
    {
        for family in families {
            if let Some(action_family) = family
                .get("action_family")
                .and_then(serde_json::Value::as_str)
            {
                ranking_by_profile.insert(
                    profile_name_from_action_family(action_family).to_owned(),
                    family.clone(),
                );
            }
        }
    }

    let mut backlog_rows = Vec::new();
    for profile_name in candidate_names {
        let current = profile_current.get(&profile_name);
        let ranked = ranking_by_profile.get(&profile_name);
        let current_accepts = current
            .and_then(|value| json_u64(value, &["unique_cpu_accepts_over_exact_cache"]))
            .unwrap_or(0);
        let events_seen_after_train_window = current
            .and_then(|value| json_u64(value, &["events_seen_after_train_window"]))
            .unwrap_or(0);
        let profile_false_accepts = current
            .and_then(|value| json_u64(value, &["false_accepts"]))
            .unwrap_or(0);
        let profile_wrong_wins = current
            .and_then(|value| json_u64(value, &["wrong_wins"]))
            .unwrap_or(0);
        let rows = ranked
            .and_then(|value| json_u64(value, &["rows"]))
            .unwrap_or(0);
        let rows_with_verifier_label = ranked
            .and_then(|value| json_u64(value, &["rows_with_verifier_label"]))
            .unwrap_or(0);
        let rows_missing_verifier_label = ranked
            .and_then(|value| json_u64(value, &["rows_missing_verifier_label"]))
            .unwrap_or(0);
        let estimated_tokens = ranked
            .and_then(|value| json_u64(value, &["estimated_tokens"]))
            .unwrap_or(0);
        let traffic_share_milli = ranked
            .and_then(|value| json_u64(value, &["traffic_share_milli"]))
            .unwrap_or(0);
        let recommended_verifier_capture = ranked
            .and_then(|value| json_string(value, &["recommended_verifier_capture"]))
            .unwrap_or_else(|| "capture_verifier_or_result_evidence".to_owned());
        let current_profile_is_top = profile_name == top_profile_name;
        let remaining_rows_after_current_accepts = rows.saturating_sub(current_accepts);
        let possible_gap_cover_milli = per_thousand_u64(
            remaining_rows_after_current_accepts.min(additional_non_top_accepts_needed),
            additional_non_top_accepts_needed,
        );
        let recommended_next_action = if current_profile_is_top {
            "freeze_top_profile_for_safety_regression_only_do_not_spend_diversity_work"
        } else if current_accepts > 0 && rows == 0 {
            "attach_this_profile_to_verifier_needed_ranking_before_scaling"
        } else if current_accepts > 0
            && rows_with_verifier_label > 0
            && rows_missing_verifier_label == 0
        {
            "expand_time_window_or_future_replay_for_existing_verifier_bound_acceptor"
        } else if current_accepts > 0 && rows_missing_verifier_label > 0 {
            "scale_verifier_capture_for_existing_non_top_winner_then_rerun_frontier"
        } else if rows_with_verifier_label > 0 && current_accepts == 0 {
            "diagnose_window_or_exact_cache_overlap_then_expand_verifier_evidence"
        } else if rows > 0 {
            "attach_deterministic_verifier_before_nwpc_compile"
        } else {
            "collect_real_trace_rows_before_profile_work"
        };
        let priority_class = if current_profile_is_top {
            "exclude_top_profile_from_diversity_push"
        } else if current_accepts > 0 && rows == 0 {
            "attach_ranking_trace_for_existing_non_top_acceptor"
        } else if current_accepts > 0 {
            "scale_existing_non_top_acceptor"
        } else if rows_with_verifier_label > 0 {
            "repair_zero_global_accept_profile"
        } else if rows > 0 {
            "capture_verifier_for_high_traffic_family"
        } else {
            "trace_needed"
        };
        backlog_rows.push(serde_json::json!({
            "profile_name": profile_name,
            "current_profile_is_top": current_profile_is_top,
            "priority_class": priority_class,
            "current_unique_cpu_accepts_over_exact_cache": current_accepts,
            "events_seen_after_train_window": events_seen_after_train_window,
            "profile_false_accepts": profile_false_accepts,
            "profile_wrong_wins": profile_wrong_wins,
            "trace_rows": rows,
            "traffic_share_milli": traffic_share_milli,
            "rows_with_verifier_label": rows_with_verifier_label,
            "rows_missing_verifier_label": rows_missing_verifier_label,
            "estimated_tokens": estimated_tokens,
            "remaining_rows_after_current_accepts": remaining_rows_after_current_accepts,
            "possible_gap_cover_milli_if_verifier_capture_succeeds": possible_gap_cover_milli,
            "recommended_verifier_capture": recommended_verifier_capture,
            "recommended_next_action": recommended_next_action,
        }));
    }
    backlog_rows.sort_by(|left, right| {
        diversity_backlog_priority_rank(left)
            .cmp(&diversity_backlog_priority_rank(right))
            .then_with(|| {
                right["current_unique_cpu_accepts_over_exact_cache"]
                    .as_u64()
                    .unwrap_or(0)
                    .cmp(
                        &left["current_unique_cpu_accepts_over_exact_cache"]
                            .as_u64()
                            .unwrap_or(0),
                    )
            })
            .then_with(|| {
                right["rows_missing_verifier_label"]
                    .as_u64()
                    .unwrap_or(0)
                    .cmp(&left["rows_missing_verifier_label"].as_u64().unwrap_or(0))
            })
            .then_with(|| {
                left["profile_name"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(right["profile_name"].as_str().unwrap_or(""))
            })
    });

    let recommended_focus = backlog_rows
        .iter()
        .filter(|row| {
            !row["current_profile_is_top"].as_bool().unwrap_or(false)
                && row["profile_false_accepts"].as_u64().unwrap_or(0) == 0
                && row["profile_wrong_wins"].as_u64().unwrap_or(0) == 0
        })
        .take(8)
        .cloned()
        .collect::<Vec<_>>();

    let diversity_backlog_blocked = additional_non_top_accepts_needed > 0
        || additional_contributing_profiles_needed > 0
        || !money_claim_gate_passed;
    let verdict = if diversity_backlog_blocked {
        "PHASE_ATOM_DIVERSITY_BACKLOG_ACTION_REQUIRED"
    } else if cpu10_gate_passed && safety_gate_passed {
        "PHASE_ATOM_DIVERSITY_BACKLOG_DIVERSITY_READY_MONEY_PENDING_OR_READY"
    } else {
        "PHASE_ATOM_DIVERSITY_BACKLOG_REPAIR_REQUIRED"
    };

    let report = serde_json::json!({
        "report_kind": "phase_atom_diversity_backlog_v1",
        "mode": "offline_backlog_only_no_compile_no_promotion",
        "claim_audit_path": claim_audit_path,
        "verifier_needed_ranking_path": verifier_ranking_path,
        "current_state": {
            "current_unique_cpu_accepts_over_exact_cache": current_unique_cpu_accepts,
            "current_non_top_accepts": current_non_top_accepts,
            "top_profile_name": top_profile_name,
            "top_profile_accepts": top_profile_accepts,
            "top_profile_share_milli": current_top_share_milli,
            "contributing_profile_count": contributing_profile_count,
            "cpu10_gate_passed": cpu10_gate_passed,
            "safety_gate_passed": safety_gate_passed,
            "money_claim_gate_passed": money_claim_gate_passed,
            "false_accepts": false_accepts,
            "wrong_wins": wrong_wins,
        },
        "diversity_target": {
            "min_contributing_profiles": min_contributing_profiles,
            "max_top_profile_share_milli": diversity_target_top_share_milli,
            "required_total_cpu_accepts_if_top_stays_constant": required_total_for_top_share,
            "required_non_top_accepts_if_top_stays_constant": required_non_top_accepts,
            "additional_non_top_accepts_needed": additional_non_top_accepts_needed,
            "additional_contributing_profiles_needed": additional_contributing_profiles_needed,
            "policy_note": "If the top profile gains more accepts, the non-top requirement rises again; spend new mining work on non-top verifier-bound families."
        },
        "business_value_gate": {
            "required_before_profile_work": [
                "real_trace_frequency",
                "exact_cache_overlap",
                "deterministic_verifier_or_external_fact",
                "expected_unique_accepts_over_exact_cache",
                "false_accept_risk",
                "tokens_or_cost_evidence"
            ],
            "do_not_optimize": [
                "top_profile_more_than_safety_regression",
                "local_heldout_accuracy_without_global_unique_accept_lift",
                "profiles_without_verifier_evidence"
            ]
        },
        "recommended_focus": recommended_focus,
        "backlog": backlog_rows,
        "gates": {
            "compile_allowed": false,
            "product_runtime_changed": false,
            "serving_runtime_changed": false,
            "local_accept_enabled": local_accept_enabled,
            "market_money_claim_allowed": market_money_claim_allowed,
            "auto_promote_enabled": false,
        },
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false,
        },
        "next_actions": [
            "collect or extract verifier/result evidence for recommended non-top families",
            "rerun time-split discovery and promotion audit only for evidence-backed families",
            "rerun frontier union, frontier shadow replay, and claim audit",
            "keep local_accept and market money claim blocked until diversity and provider-cost gates pass"
        ],
        "verdict": verdict,
        "boundary": "backlog only: reads claim audit plus verifier-needed ranking and calculates the non-top verified accept gap; it does not compile .nwpc packages, promote, enable local_accept, use legacy .nwrb/role-binding, lookup, target/proof authority, concrete_x_lookup, or manual local_out_t",
    });
    write_json_file(&report_path, &report)?;
    println!("phase_atom_diversity_backlog_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  claim_audit_path: {}", claim_audit_path.display());
    println!("  current_unique_cpu_accepts_over_exact_cache: {current_unique_cpu_accepts}");
    println!("  top_profile_name: {top_profile_name}");
    println!("  top_profile_share_milli: {current_top_share_milli}");
    println!("  additional_non_top_accepts_needed: {additional_non_top_accepts_needed}");
    println!("  false_accepts: {false_accepts}");
    println!("  wrong_wins: {wrong_wins}");
    println!("  verdict: {verdict}");
    Ok(())
}

struct CompatibleDenominatorProfile {
    bucket_key: String,
    action_family_atom: String,
    task_name: String,
    candidate_package_path: String,
    expected_package_fingerprint64: u64,
    package_fingerprint64: u64,
    package_bytes: usize,
    package_records: usize,
    package_matches_report: bool,
    safe_accept_margin_threshold_micro: i64,
    runtime: PhaseCenterOffloadRuntime,
    events: Vec<CompatibleRoutedEvent>,
}

struct CompatibleRoutedEvent {
    profile_index: usize,
    denominator_row_index: usize,
    source_trace_path: String,
    source_line_index: usize,
    exact_cache_hit: bool,
    event: PhaseAtomBinaryEvent,
}

struct FrontierShadowReplayProfile {
    input_report_path: String,
    discovery_report_path: String,
    action_family_atom: String,
    task_name: String,
    candidate_package_path: String,
    expected_package_fingerprint64: u64,
    package_fingerprint64: u64,
    package_bytes: usize,
    package_records: usize,
    package_matches_report: bool,
    margin_threshold_micro: i64,
    train_time_max: String,
    heldout_time_min: String,
    runtime: PhaseCenterOffloadRuntime,
    hot_runtime: PhaseCenterHotRuntime,
    hot_routes: PhaseCenterHotRouteTable,
    hot_scratch: PhaseCenterHotScratch,
    runtime_record_count: usize,
    events: Vec<FrontierRoutedEvent>,
}

#[derive(Clone)]
struct FrontierRoutedEvent {
    profile_index: usize,
    denominator_row_index: usize,
    source_trace_path: String,
    source_line_index: usize,
    exact_cache_hit: bool,
    event: PhaseAtomBinaryEvent,
}

#[derive(Default)]
struct PhaseAtomJoinScan {
    rows: u64,
    rows_with_request_fingerprint: u64,
    request_fingerprints: BTreeSet<String>,
    action_family_counts: BTreeMap<String, u64>,
    accepted_bucket_rows: u64,
    accepted_bucket_request_fingerprints: BTreeSet<String>,
    accepted_bucket_counts: BTreeMap<String, u64>,
}

impl PhaseAtomJoinScan {
    fn merge(&mut self, other: Self) {
        self.rows = self.rows.saturating_add(other.rows);
        self.rows_with_request_fingerprint = self
            .rows_with_request_fingerprint
            .saturating_add(other.rows_with_request_fingerprint);
        self.request_fingerprints.extend(other.request_fingerprints);
        for (key, count) in other.action_family_counts {
            *self.action_family_counts.entry(key).or_insert(0) += count;
        }
        self.accepted_bucket_rows = self
            .accepted_bucket_rows
            .saturating_add(other.accepted_bucket_rows);
        self.accepted_bucket_request_fingerprints
            .extend(other.accepted_bucket_request_fingerprints);
        for (key, count) in other.accepted_bucket_counts {
            *self.accepted_bucket_counts.entry(key).or_insert(0) += count;
        }
    }

    fn to_json(&self, top_n: usize) -> serde_json::Value {
        serde_json::json!({
            "rows": self.rows,
            "rows_with_request_fingerprint": self.rows_with_request_fingerprint,
            "unique_request_fingerprints": self.request_fingerprints.len(),
            "accepted_bucket_rows": self.accepted_bucket_rows,
            "accepted_bucket_unique_request_fingerprints": self.accepted_bucket_request_fingerprints.len(),
            "top_action_families": top_count_rows(&self.action_family_counts, top_n),
            "accepted_bucket_counts": top_count_rows(&self.accepted_bucket_counts, top_n),
        })
    }
}

fn scan_phase_atom_trace_for_join(
    trace_path: &Path,
    accepted_bucket_keys: &BTreeSet<String>,
) -> Result<PhaseAtomJoinScan, String> {
    let text = std::fs::read_to_string(trace_path).map_err(|error| {
        format!(
            "failed to read phase-atom join trace '{}': {error}",
            trace_path.display()
        )
    })?;
    let mut scan = PhaseAtomJoinScan::default();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        scan.rows = scan.rows.saturating_add(1);
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse phase-atom join trace '{}' line {}: {error}",
                trace_path.display(),
                line_index + 1
            )
        })?;
        let request_fingerprint = json_string(&row, &["request_fingerprint"]);
        if let Some(fingerprint) = request_fingerprint.as_deref() {
            scan.rows_with_request_fingerprint =
                scan.rows_with_request_fingerprint.saturating_add(1);
            scan.request_fingerprints.insert(fingerprint.to_owned());
        }
        let action_atoms = phase_atom_string_vec(&row, "action_atoms");
        let request_atoms = phase_atom_string_vec(&row, "request_atoms");
        let state_atoms = phase_atom_string_vec(&row, "state_atoms");
        let tool_atoms = phase_atom_string_vec(&row, "tool_atoms");
        let route_hint_atoms = phase_atom_string_vec(&row, "route_hint_atoms");
        for action_family in phase_atom_action_families(&action_atoms) {
            *scan
                .action_family_counts
                .entry(action_family.clone())
                .or_insert(0) += 1;
            let bucket_key = phase_atom_state_action_bucket_key(
                &action_family,
                &request_atoms,
                &state_atoms,
                &tool_atoms,
                &route_hint_atoms,
            );
            if accepted_bucket_keys.contains(&bucket_key) {
                scan.accepted_bucket_rows = scan.accepted_bucket_rows.saturating_add(1);
                *scan.accepted_bucket_counts.entry(bucket_key).or_insert(0) += 1;
                if let Some(fingerprint) = request_fingerprint.as_deref() {
                    scan.accepted_bucket_request_fingerprints
                        .insert(fingerprint.to_owned());
                }
            }
        }
    }
    Ok(scan)
}

fn top_count_rows(counts: &BTreeMap<String, u64>, limit: usize) -> Vec<serde_json::Value> {
    let mut rows = counts
        .iter()
        .map(|(key, count)| {
            serde_json::json!({
                "key": key,
                "count": count,
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right["count"]
            .as_u64()
            .unwrap_or(0)
            .cmp(&left["count"].as_u64().unwrap_or(0))
            .then_with(|| {
                left["key"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(right["key"].as_str().unwrap_or(""))
            })
    });
    rows.truncate(limit);
    rows
}

fn run_phase_stream_phase_atom_serving_shadow_replay<I>(
    mut args: I,
    future_only: bool,
    append_window: bool,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(if append_window {
            DEFAULT_PHASE_ATOM_SERVING_APPEND_SHADOW_REPLAY_REPORT
        } else if future_only {
            DEFAULT_PHASE_ATOM_SERVING_FUTURE_SHADOW_REPLAY_REPORT
        } else {
            DEFAULT_PHASE_ATOM_SERVING_SHADOW_REPLAY_REPORT
        })
    });
    let append_watermark_trace_path = if append_window {
        Some(
            args.next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSION_TOOL_STATUS_VERIFIER_JSONL)),
        )
    } else {
        None
    };
    let replay_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_SESSION_TOOL_STATUS_VERIFIER_JSONL));
    let admission_report_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(
                DEFAULT_PHASE_ATOM_TOOL_STATUS_SERVING_ADMISSION_AUDIT_REPORT,
            )]
        } else {
            rest
        }
    };
    let price_config = read_json_file::<ModelPriceConfig>(Path::new(DEFAULT_PRICE_CONFIG))?;

    let mut profiles = Vec::new();
    let mut profile_reports = Vec::new();
    for admission_report_path in &admission_report_paths {
        let admission = read_json_value(admission_report_path)?;
        let action_family = json_string(&admission, &["action_family"]).ok_or_else(|| {
            format!(
                "serving shadow admission report '{}' missing action_family",
                admission_report_path.display()
            )
        })?;
        if !action_family.starts_with("action_family:") {
            return Err(format!(
                "serving shadow action_family must start with action_family:, got '{action_family}'"
            ));
        }
        let task_name = action_family
            .strip_prefix("action_family:")
            .unwrap_or(action_family.as_str())
            .replace(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_', "_");
        let candidate_package_path = json_string(&admission, &["candidate_package_path"])
            .map(PathBuf::from)
            .ok_or_else(|| {
                format!(
                    "serving shadow admission report '{}' missing candidate_package_path",
                    admission_report_path.display()
                )
            })?;
        let margin_threshold_micro = json_i64(&admission, &["margin_threshold_micro"])
            .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
        let replay_train_events =
            json_u64(&admission, &["replay", "replay_train_events"]).unwrap_or_default() as usize;
        let replay_heldout_events =
            json_u64(&admission, &["replay", "replay_heldout_events"]).unwrap_or_default() as usize;
        let admission_candidate_allowed =
            json_bool(&admission, &["serving_admission_candidate_allowed"]).unwrap_or(false);
        let admission_local_accept =
            json_bool(&admission, &["local_accept_enabled"]).unwrap_or(true);
        let admission_promoted = json_bool(&admission, &["promoted"]).unwrap_or(true);
        let admission_serving_profile_artifact =
            json_bool(&admission, &["serving_profile_artifact"]).unwrap_or(true);
        let admission_product_runtime_changed =
            json_bool(&admission, &["product_runtime_changed"]).unwrap_or(true);
        let admission_serving_runtime_changed =
            json_bool(&admission, &["serving_runtime_changed"]).unwrap_or(true);
        let admission_market_claim =
            json_bool(&admission, &["market_money_claim_allowed"]).unwrap_or(true);
        let forbidden_target =
            json_bool(&admission, &["forbidden_flags", "target_id_used"]).unwrap_or(true);
        let forbidden_proof = json_bool(
            &admission,
            &["forbidden_flags", "proof_rule_id_authority_used"],
        )
        .unwrap_or(true);
        let forbidden_lookup =
            json_bool(&admission, &["forbidden_flags", "concrete_x_lookup_used"]).unwrap_or(true);
        let forbidden_local_out_t =
            json_bool(&admission, &["forbidden_flags", "manual_local_out_t_used"]).unwrap_or(true);
        let forbidden_bind = json_bool(
            &admission,
            &["forbidden_flags", "hidden_frame_id_or_bind_x_used"],
        )
        .unwrap_or(true);
        let forbidden_legacy =
            json_bool(&admission, &["forbidden_flags", "legacy_backend_used"]).unwrap_or(true);
        let forbidden_flags_clear = !forbidden_target
            && !forbidden_proof
            && !forbidden_lookup
            && !forbidden_local_out_t
            && !forbidden_bind
            && !forbidden_legacy
            && !admission_local_accept
            && !admission_promoted
            && !admission_serving_profile_artifact
            && !admission_product_runtime_changed
            && !admission_serving_runtime_changed
            && !admission_market_claim;
        let package_bytes = std::fs::read(&candidate_package_path).map_err(|error| {
            format!(
                "failed to read serving shadow package '{}': {error}",
                candidate_package_path.display()
            )
        })?;
        let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes)
            .map_err(|error| {
                format!(
                    "serving shadow package inspect error for '{}': {error:?}",
                    candidate_package_path.display()
                )
            })?;
        let report_package_fingerprint64 =
            json_u64(&admission, &["package", "package_fingerprint64"]).unwrap_or_default();
        let report_package_bytes =
            json_u64(&admission, &["package", "package_bytes"]).unwrap_or_default() as usize;
        let report_package_records = json_u64(&admission, &["package", "inspected_record_count"])
            .unwrap_or_default() as usize;
        let package_matches_report = report_package_fingerprint64 == package_info.fingerprint64
            && report_package_bytes == package_bytes.len()
            && report_package_records == package_info.record_count;
        let runtime = PhaseCenterOffloadRuntime::from_package_bytes(
            &package_bytes,
            PhaseCenterOffloadPolicy::new(margin_threshold_micro)
                .map_err(|error| format!("serving shadow invalid policy: {error:?}"))?,
        )
        .map_err(|error| format!("serving shadow package load error: {error:?}"))?;
        let runtime_cells = runtime.cells();
        let runtime_record_count = runtime.record_count();
        let profile_ids = (0..runtime_record_count as u32).collect::<Vec<_>>();
        let thresholds = vec![margin_threshold_micro; runtime_record_count];
        let hot_runtime =
            PhaseCenterHotRuntime::from_flat_runtime(runtime.runtime(), &profile_ids, &thresholds)
                .map_err(|error| format!("serving shadow hot runtime build error: {error:?}"))?;
        let hot_route = hot_runtime
            .route_plan_from_profile_ids(0, profile_ids.iter().copied())
            .map_err(|error| format!("serving shadow hot route build error: {error:?}"))?
            .ok_or_else(|| "serving shadow hot route has no profiles".to_owned())?;
        let hot_routes = PhaseCenterHotRouteTable::from_plans([hot_route])
            .map_err(|error| format!("serving shadow hot route table error: {error:?}"))?;
        let hot_scratch = PhaseCenterHotScratch::new(runtime_cells, runtime_record_count)
            .map_err(|error| format!("serving shadow hot scratch error: {error:?}"))?;
        let hot_runtime_bytes_estimate = hot_runtime.bytes_estimate();
        let hot_route_table_bytes_estimate = hot_routes.bytes_estimate();
        let runtime_bytes_estimate =
            hot_runtime_bytes_estimate.saturating_add(hot_route_table_bytes_estimate);
        let profile_metadata_bytes_estimate =
            phase_atom_serving_shadow_profile_metadata_bytes_estimate(&action_family, &task_name);
        let runtime_budget = phase_atom_serving_runtime_budget_report(
            "shadow_loaded_profile_hot_runtime",
            phase_atom_serving_budget_snapshot(
                1,
                runtime_record_count,
                profile_metadata_bytes_estimate,
                0,
                hot_routes.route_count(),
                hot_runtime.profile_count(),
                hot_routes.profile_edge_count(),
                hot_runtime_bytes_estimate,
                hot_route_table_bytes_estimate,
            ),
        );
        profile_reports.push(PhaseAtomServingShadowProfileReport {
            action_family: action_family.clone(),
            admission_report_path: admission_report_path.display().to_string(),
            candidate_package_path: candidate_package_path.display().to_string(),
            admission_candidate_allowed,
            margin_threshold_micro,
            replay_train_events,
            replay_heldout_events,
            package_fingerprint64: package_info.fingerprint64,
            package_bytes: package_bytes.len(),
            runtime_cells,
            runtime_record_count,
            runtime_bytes_estimate,
            runtime_budget,
            forbidden_flags_clear: forbidden_flags_clear && package_matches_report,
        });
        profiles.push(PhaseAtomServingShadowRuntimeProfile {
            action_family,
            task_name,
            admission_candidate_allowed,
            forbidden_flags_clear: forbidden_flags_clear && package_matches_report,
            replay_train_events,
            replay_heldout_events,
            hot_runtime,
            hot_routes,
            hot_scratch,
            runtime_cells,
            runtime_record_count,
        });
    }

    let mut append_watermark_routable_events = Vec::<PhaseAtomBinaryEvent>::new();
    if let Some(watermark_path) = append_watermark_trace_path.as_ref() {
        let watermark_text = std::fs::read_to_string(watermark_path).map_err(|error| {
            format!(
                "failed to read serving append watermark trace '{}': {error}",
                watermark_path.display()
            )
        })?;
        for (line_index, line) in watermark_text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse serving append watermark trace '{}' line {}: {error}",
                    watermark_path.display(),
                    line_index + 1
                )
            })?;
            for profile in &profiles {
                if let Some(event) = parse_phase_atom_binary_event_for_action(
                    &row,
                    append_watermark_routable_events.len(),
                    &profile.action_family,
                    &profile.task_name,
                ) {
                    append_watermark_routable_events.push(event);
                    break;
                }
            }
        }
    }
    let append_watermark_max_timestamp = append_watermark_routable_events
        .iter()
        .map(|event| event.event_timestamp.as_str())
        .max()
        .map(ToOwned::to_owned);

    let trace_text = std::fs::read_to_string(&replay_trace_path).map_err(|error| {
        format!(
            "failed to read serving shadow replay trace '{}': {error}",
            replay_trace_path.display()
        )
    })?;
    let mut total_rows = 0usize;
    let mut parsed_routable_events = 0usize;
    let mut excluded_training_overlap_events = 0usize;
    let mut all_routable_events = Vec::<(usize, PhaseAtomBinaryEvent)>::new();
    for (line_index, line) in trace_text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse serving shadow replay trace '{}' line {}: {error}",
                replay_trace_path.display(),
                line_index + 1
            )
        })?;
        total_rows += 1;
        if total_rows.is_multiple_of(1000) {
            println!("  serving_shadow_rows_scanned: {total_rows}");
        }
        for (profile_index, profile) in profiles.iter().enumerate() {
            if let Some(event) = parse_phase_atom_binary_event_for_action(
                &row,
                parsed_routable_events,
                &profile.action_family,
                &profile.task_name,
            ) {
                parsed_routable_events += 1;
                all_routable_events.push((profile_index, event));
                break;
            }
        }
    }
    let mut cache_scope_events = append_watermark_routable_events.clone();
    let mut selected_cache_indices = Vec::new();
    let mut append_watermark_excluded_events = 0usize;
    let routed_events = if append_window {
        let mut routed_events = Vec::new();
        for (profile_index, event) in all_routable_events.iter().cloned() {
            if append_watermark_max_timestamp
                .as_deref()
                .is_some_and(|max_timestamp| event.event_timestamp.as_str() <= max_timestamp)
            {
                append_watermark_excluded_events =
                    append_watermark_excluded_events.saturating_add(1);
                continue;
            }
            let cache_index = cache_scope_events.len();
            cache_scope_events.push(event.clone());
            selected_cache_indices.push(cache_index);
            routed_events.push((profile_index, event));
        }
        routed_events
    } else if future_only {
        cache_scope_events.extend(all_routable_events.iter().map(|(_, event)| event.clone()));
        for (profile_index, profile) in profiles.iter().enumerate() {
            let profile_global_indices = all_routable_events
                .iter()
                .enumerate()
                .filter_map(|(index, (event_profile_index, _))| {
                    (*event_profile_index == profile_index).then_some(index)
                })
                .collect::<Vec<_>>();
            if profile_global_indices.is_empty() {
                continue;
            }
            let profile_events = profile_global_indices
                .iter()
                .map(|index| all_routable_events[*index].1.clone())
                .collect::<Vec<_>>();
            let split_denominator = profile
                .replay_train_events
                .saturating_add(profile.replay_heldout_events)
                .max(profile_events.len());
            let train_permille = profile
                .replay_train_events
                .saturating_mul(1000)
                .checked_div(split_denominator)
                .unwrap_or(DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_TRAIN_PERMILLE)
                .clamp(1, 999);
            let (train_indices, heldout_indices) =
                phase_atom_binary_time_split_indices(&profile_events, train_permille);
            excluded_training_overlap_events =
                excluded_training_overlap_events.saturating_add(train_indices.len());
            selected_cache_indices.extend(
                heldout_indices
                    .into_iter()
                    .map(|local_index| profile_global_indices[local_index]),
            );
        }
        selected_cache_indices.sort_unstable();
        selected_cache_indices
            .iter()
            .map(|cache_index| all_routable_events[*cache_index].clone())
            .collect::<Vec<_>>()
    } else {
        cache_scope_events.extend(all_routable_events.iter().map(|(_, event)| event.clone()));
        selected_cache_indices.extend(0..all_routable_events.len());
        all_routable_events.clone()
    };
    let exact_cache_flags = exact_cache_hit_flags_phase_atom_binary(&cache_scope_events);
    let mut margins = Vec::with_capacity(routed_events.len());
    let mut latencies = Vec::with_capacity(routed_events.len());
    let mut local_operator_shadow_decisions = 0usize;
    let mut fallback_shadow_decisions = 0usize;
    let mut wrong_wins = 0usize;
    let mut false_accepts = 0usize;
    let mut exact_cache_hits_in_routed_events = 0usize;
    let mut unique_cpu_accepts_over_exact_cache = 0usize;
    let mut nando_cpu_tokens_saved = 0usize;
    let mut nando_cpu_cost_saved_microusd = 0u64;
    let mut unique_accepts = Vec::new();
    for (routed_index, (profile_index, event)) in routed_events.iter().enumerate() {
        if routed_index > 0 && routed_index % 1000 == 0 {
            println!("  serving_shadow_events_scored: {routed_index}");
        }
        let profile = &mut profiles[*profile_index];
        let program_index = usize::from(!event.verified_safe_accept);
        if program_index >= profile.runtime_record_count {
            return Err(format!(
                "serving shadow program_index {program_index} out of bounds for record_count {}",
                profile.runtime_record_count
            ));
        }
        let correct_vec = phase_atom_binary_event_vector_for_task(
            event,
            event.verified_safe_accept,
            profile.runtime_cells,
            &profile.task_name,
        );
        let wrong_vec = phase_atom_binary_event_vector_for_task(
            event,
            !event.verified_safe_accept,
            profile.runtime_cells,
            &profile.task_name,
        );
        let prepared_delta_vec = phase_delta_vector(&correct_vec, &wrong_vec);
        let started = Instant::now();
        let candidates = profile
            .hot_runtime
            .score_prepared_hot_request_candidates(
                &profile.hot_routes,
                PhaseCenterPreparedHotRequest::new(0, &prepared_delta_vec),
                &mut profile.hot_scratch,
            )
            .map_err(|error| format!("serving shadow prepared hot decision error: {error:?}"))?;
        latencies.push(started.elapsed().as_nanos());
        let Some(decision) = candidates.get(program_index) else {
            return Err(format!(
                "serving shadow missing candidate for program_index {program_index}"
            ));
        };
        margins.push(decision.margin_micro);
        wrong_wins += usize::from(decision.margin_micro <= 0);
        if decision.score_candidate {
            local_operator_shadow_decisions += 1;
            false_accepts += usize::from(decision.margin_micro <= 0);
            if !exact_cache_flags[selected_cache_indices[routed_index]] {
                unique_cpu_accepts_over_exact_cache += 1;
                nando_cpu_tokens_saved =
                    nando_cpu_tokens_saved.saturating_add(event.token_cost.total_tokens);
                nando_cpu_cost_saved_microusd = nando_cpu_cost_saved_microusd
                    .saturating_add(event.token_cost.total_cost_microusd);
                unique_accepts.push(GenericAcceptedEventReport {
                    request_fingerprint: format!(
                        "serving_shadow_{}:{}",
                        profile.task_name, event.exact_cache_key
                    ),
                    total_tokens: event.token_cost.total_tokens,
                    total_cost_microusd: event.token_cost.total_cost_microusd,
                    token_evidence_missing: event.token_cost.token_evidence_missing,
                    cost_evidence_missing: event.token_cost.cost_evidence_missing,
                });
            }
        } else {
            fallback_shadow_decisions += 1;
        }
        exact_cache_hits_in_routed_events +=
            usize::from(exact_cache_flags[selected_cache_indices[routed_index]]);
    }
    margins.sort_unstable();
    latencies.sort_unstable();
    let profile_count = profiles.len();
    let loaded_profile_count = profile_reports.len();
    let aggregate_hot_runtime_bytes_estimate = profiles
        .iter()
        .map(|profile| profile.hot_runtime.bytes_estimate())
        .sum::<usize>();
    let aggregate_hot_route_table_bytes_estimate = profiles
        .iter()
        .map(|profile| profile.hot_routes.bytes_estimate())
        .sum::<usize>();
    let aggregate_hot_route_count = profiles
        .iter()
        .map(|profile| profile.hot_routes.route_count())
        .sum::<usize>();
    let aggregate_hot_profile_count = profiles
        .iter()
        .map(|profile| profile.hot_runtime.profile_count())
        .sum::<usize>();
    let aggregate_hot_route_profile_edges = profiles
        .iter()
        .map(|profile| profile.hot_routes.profile_edge_count())
        .sum::<usize>();
    let aggregate_warm_metadata_bytes_estimate = profiles
        .iter()
        .map(|profile| {
            phase_atom_serving_shadow_profile_metadata_bytes_estimate(
                &profile.action_family,
                &profile.task_name,
            )
        })
        .sum::<usize>();
    let runtime_budget = phase_atom_serving_runtime_budget_report(
        "shadow_loaded_hot_runtime_registry",
        phase_atom_serving_budget_snapshot(
            aggregate_hot_route_count,
            aggregate_hot_profile_count,
            aggregate_warm_metadata_bytes_estimate,
            0,
            aggregate_hot_route_count,
            aggregate_hot_profile_count,
            aggregate_hot_route_profile_edges,
            aggregate_hot_runtime_bytes_estimate,
            aggregate_hot_route_table_bytes_estimate,
        ),
    );
    let all_profiles_admitted = profiles
        .iter()
        .all(|profile| profile.admission_candidate_allowed && profile.forbidden_flags_clear);
    let token_evidence_present = nando_cpu_tokens_saved > 0;
    let provider_cost_evidence_present = nando_cpu_cost_saved_microusd > 0;
    let estimated_nando_cpu_cost_saved_microusd =
        if token_evidence_present && !provider_cost_evidence_present {
            estimated_event_cost_microusd(nando_cpu_tokens_saved, 0, &price_config)
        } else {
            0
        };
    let explicit_model_price_estimate_used = estimated_nando_cpu_cost_saved_microusd > 0;
    let estimated_cost_method = if explicit_model_price_estimate_used {
        "total_saved_tokens_as_input_token_floor_from_model_price_config".to_owned()
    } else if provider_cost_evidence_present {
        "provider_cost_evidence_present_no_estimate_needed".to_owned()
    } else {
        "no_token_or_price_estimate_available".to_owned()
    };
    let training_overlap_excluded =
        future_only && excluded_training_overlap_events > 0 && parsed_routable_events > 0;
    let serving_shadow_replay_allowed = profile_count > 0
        && loaded_profile_count == profile_count
        && all_profiles_admitted
        && !routed_events.is_empty()
        && local_operator_shadow_decisions > 0
        && unique_cpu_accepts_over_exact_cache > 0
        && wrong_wins == 0
        && false_accepts == 0;
    let rejection_reason = if serving_shadow_replay_allowed {
        if append_window {
            "accepted_for_append_shadow_replay_product_accept_still_disabled".to_owned()
        } else if future_only {
            "accepted_for_future_shadow_replay_product_accept_still_disabled".to_owned()
        } else {
            "accepted_for_shadow_serving_replay_product_accept_still_disabled".to_owned()
        }
    } else if profile_count == 0 {
        "no_shadow_profiles_loaded".to_owned()
    } else if !all_profiles_admitted {
        "one_or_more_profiles_not_admitted_or_forbidden_flag_detected".to_owned()
    } else if append_window && parsed_routable_events > 0 && routed_events.is_empty() {
        "no_new_append_events_after_watermark".to_owned()
    } else if routed_events.is_empty() {
        "no_trace_events_matched_shadow_profiles".to_owned()
    } else if wrong_wins > 0 {
        "shadow_replay_wrong_wins_detected".to_owned()
    } else if false_accepts > 0 {
        "shadow_replay_false_accepts_detected".to_owned()
    } else if unique_cpu_accepts_over_exact_cache == 0 {
        "no_unique_cpu_accepts_over_exact_cache".to_owned()
    } else {
        "shadow_replay_gate_failed".to_owned()
    };
    let report = PhaseAtomServingShadowReplayReport {
        report_kind: if future_only {
            "phase_atom_serving_future_shadow_replay_v1"
        } else if append_window {
            "phase_atom_serving_append_shadow_replay_v1"
        } else {
            "phase_atom_serving_shadow_replay_v1"
        },
        mode: if future_only {
            "future_only_shadow_serving_runtime_replay"
        } else if append_window {
            "append_window_shadow_serving_runtime_replay"
        } else {
            "shadow_serving_runtime_replay_only"
        },
        shadow_runtime_kind: "phase_center_prepared_hot_runtime_registry",
        replay_trace_path: replay_trace_path.display().to_string(),
        append_watermark_trace_path: append_watermark_trace_path
            .as_ref()
            .map(|path| path.display().to_string()),
        append_watermark_max_timestamp,
        admission_report_paths: admission_report_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        profile_count,
        loaded_profile_count,
        full_trace_replay: !future_only && !append_window,
        append_window_replay: append_window,
        training_overlap_excluded,
        market_savings_count_allowed: ((future_only && training_overlap_excluded) || append_window)
            && serving_shadow_replay_allowed,
        progress_output_enabled: true,
        runtime_budget,
        profiles: profile_reports,
        replay: PhaseAtomServingShadowReplayAudit {
            total_rows,
            parsed_routable_events,
            append_watermark_routable_events: append_watermark_routable_events.len(),
            append_watermark_excluded_events,
            excluded_training_overlap_events,
            routed_events: routed_events.len(),
            unrouted_events: total_rows.saturating_sub(parsed_routable_events),
            local_operator_shadow_decisions,
            fallback_shadow_decisions,
            wrong_wins,
            false_accepts,
            exact_cache_hits_in_routed_events,
            unique_cpu_accepts_over_exact_cache,
            nando_cpu_tokens_saved,
            nando_cpu_cost_saved_microusd,
            min_margin_micro: margins.first().copied().unwrap_or(0),
            p10_margin_micro: percentile_i64(&margins, 10),
            median_margin_micro: percentile_i64(&margins, 50),
            latency_p50_ns: percentile_u128(&latencies, 50),
            latency_p90_ns: percentile_u128(&latencies, 90),
            latency_p99_ns: percentile_u128(&latencies, 99),
            latency_max_ns: latencies.last().copied().unwrap_or(0),
            unique_accepts,
        },
        economics: PhaseAtomRunCheckTimeSplitEconomicsAudit {
            token_evidence_present,
            provider_cost_evidence_present,
            explicit_model_price_estimate_used,
            price_config_schema_version: price_config.schema_version,
            provider: price_config.default_provider,
            model_id: price_config.default_model_id,
            price_source: price_config.price_source,
            nando_cpu_tokens_saved,
            nando_cpu_cost_saved_microusd,
            estimated_nando_cpu_cost_saved_microusd,
            estimated_cost_method,
            projected_nando_calls_saved_milli: per_thousand(
                unique_cpu_accepts_over_exact_cache,
                routed_events.len(),
            ),
            projected_combined_calls_saved_milli: per_thousand(
                exact_cache_hits_in_routed_events + unique_cpu_accepts_over_exact_cache,
                routed_events.len(),
            ),
            money_claim_blocker:
                "shadow serving replay only; product money claim requires live/shadow deployment evidence"
                    .to_owned(),
        },
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        serving_shadow_replay_allowed,
        product_promotion_allowed: false,
        local_accept_enabled: false,
        promoted: false,
        serving_profile_artifact: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        rejection_reason,
        boundary: if future_only {
            "future-only shadow serving replay: loads admitted .nwpc profiles into a prepared-hot runtime registry and shadow-scores only events after the admission train window; counts are overlap-excluded shadow evidence, but this still does not enable product local_accept, promote runtime, write serving profiles, or use legacy nwrb/role-binding paths"
        } else if append_window {
            "append-window shadow serving replay: loads admitted .nwpc profiles into a prepared-hot runtime registry, treats the watermark trace as exact-cache history, and shadow-scores only events newer than the watermark timestamp; it does not enable product local_accept, promote runtime, write serving profiles, or use legacy nwrb/role-binding paths"
        } else {
            "shadow serving replay only: loads admitted .nwpc profiles into a prepared-hot runtime registry and shadow-scores routed trace events; this default full-trace replay may include the original compile/admission window, so its accept count is not a market savings claim; it does not compile, write serving profiles, enable product local_accept, promote runtime, allow market claims, or use legacy nwrb/role-binding paths"
        },
    };
    write_json_file(&report_path, &report)?;
    println!(
        "{}:",
        if future_only {
            "phase_atom_serving_future_shadow_replay_v1"
        } else if append_window {
            "phase_atom_serving_append_shadow_replay_v1"
        } else {
            "phase_atom_serving_shadow_replay_v1"
        }
    );
    println!("  report_path: {}", report_path.display());
    println!(
        "  serving_shadow_replay_allowed: {}",
        report.serving_shadow_replay_allowed
    );
    println!("  profile_count: {}", report.profile_count);
    println!("  routed_events: {}", report.replay.routed_events);
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.replay.unique_cpu_accepts_over_exact_cache
    );
    println!("  false_accepts: {}", report.replay.false_accepts);
    println!("  p99_latency_ns: {}", report.replay.latency_p99_ns);
    println!(
        "  hot_bytes_estimate: {}",
        report.runtime_budget.hot_bytes_estimate
    );
    println!(
        "  hot_budget_passed: {}",
        report.runtime_budget.hot_budget_passed
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!("  rejection_reason: {}", report.rejection_reason);
    Ok(())
}

pub(crate) fn run_phase_stream_real_traffic_separator_audit_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_SEPARATOR_AUDIT_REPORT));
    let min_true_over_exact = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!("invalid min_true_over_exact '{value}' for separator audit: {error}")
            })
        })
        .transpose()?
        .unwrap_or(3);
    let top_n = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid top_n '{value}' for separator audit: {error}"))
        })
        .transpose()?
        .unwrap_or(80);
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_enriched_trace_paths()
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no real-traffic trace paths provided".to_owned());
    }

    let mut events = Vec::new();
    let mut skipped_no_shadow_request = 0usize;
    let mut skipped_no_verifier_label = 0usize;
    let mut skipped_legacy_profile_events = 0usize;
    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read separator audit trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse separator audit trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            match parse_generic_real_traffic_event(&row, events.len()) {
                GenericParseResult::Event(event) => events.push(*event),
                GenericParseResult::NoShadowRequest => skipped_no_shadow_request += 1,
                GenericParseResult::NoVerifierLabel => skipped_no_verifier_label += 1,
                GenericParseResult::LegacyProfile => skipped_legacy_profile_events += 1,
            }
        }
    }
    if events.is_empty() {
        return Err("no verifier-labeled non-legacy real-traffic events found".to_owned());
    }

    let exact_cache_flags = exact_cache_hit_flags_generic(&events);
    let mut route_states = BTreeMap::<String, GenericSeparatorRouteState>::new();
    let mut candidate_states = BTreeMap::<String, GenericSeparatorCandidateState>::new();
    for (event_index, event) in events.iter().enumerate() {
        let exact_hit = exact_cache_flags[event_index];
        let route_key = format!("{}::{}", event.profile_id, event.route_key);
        let route =
            route_states
                .entry(route_key.clone())
                .or_insert_with(|| GenericSeparatorRouteState {
                    bucket_key: route_key.clone(),
                    route_key: event.route_key.clone(),
                    profile_id: event.profile_id.clone(),
                    ..Default::default()
                });
        route.events += 1;
        if event.verified_safe_accept {
            route.verifier_true_events += 1;
        } else {
            route.verifier_false_events += 1;
        }
        if exact_hit {
            route.exact_cache_hits += 1;
        }

        let token_cost = generic_event_token_cost(event);
        for (atom_family, atom) in generic_separator_atoms(event) {
            let key = format!("{}::{atom_family}::{atom}", route.bucket_key);
            let state =
                candidate_states
                    .entry(key)
                    .or_insert_with(|| GenericSeparatorCandidateState {
                        route_key: event.route_key.clone(),
                        profile_id: event.profile_id.clone(),
                        atom_family: atom_family.clone(),
                        atom: atom.clone(),
                        ..Default::default()
                    });
            state.events += 1;
            if exact_hit {
                state.exact_cache_hits += 1;
            }
            if event.verified_safe_accept {
                state.verifier_true_events += 1;
                if !exact_hit {
                    state.true_over_exact_cache_events += 1;
                    state.token_ceiling_over_exact_cache += token_cost.total_tokens;
                    state.cost_ceiling_microusd_over_exact_cache = state
                        .cost_ceiling_microusd_over_exact_cache
                        .saturating_add(token_cost.total_cost_microusd);
                }
            } else {
                state.verifier_false_events += 1;
            }
        }
    }

    let mut candidates = candidate_states
        .into_values()
        .filter(|state| state.true_over_exact_cache_events >= min_true_over_exact)
        .map(|state| {
            let static_clean_on_current_labeled_set = state.verifier_false_events == 0;
            let shortcut_risk = separator_atom_shortcut_risk(&state.atom_family);
            let recommended_next_action = if !static_clean_on_current_labeled_set {
                "do_not_bucket_false_support_present"
            } else if state.true_over_exact_cache_events >= 8 {
                "run_shadow_bucket_experiment_with_future_events"
            } else {
                "collect_more_trace_before_bucket_experiment"
            };
            GenericSeparatorCandidateReport {
                route_key: state.route_key,
                profile_id: state.profile_id,
                atom_family: state.atom_family,
                atom: state.atom,
                events: state.events,
                verifier_true_events: state.verifier_true_events,
                verifier_false_events: state.verifier_false_events,
                true_over_exact_cache_events: state.true_over_exact_cache_events,
                exact_cache_hits: state.exact_cache_hits,
                token_ceiling_over_exact_cache: state.token_ceiling_over_exact_cache,
                cost_ceiling_microusd_over_exact_cache: state
                    .cost_ceiling_microusd_over_exact_cache,
                false_rate_milli: per_thousand(state.verifier_false_events, state.events),
                static_clean_on_current_labeled_set,
                shortcut_risk,
                recommended_next_action,
            }
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .static_clean_on_current_labeled_set
            .cmp(&left.static_clean_on_current_labeled_set)
            .then_with(|| {
                right
                    .true_over_exact_cache_events
                    .cmp(&left.true_over_exact_cache_events)
            })
            .then_with(|| {
                right
                    .cost_ceiling_microusd_over_exact_cache
                    .cmp(&left.cost_ceiling_microusd_over_exact_cache)
            })
            .then_with(|| left.route_key.cmp(&right.route_key))
            .then_with(|| left.atom_family.cmp(&right.atom_family))
            .then_with(|| left.atom.cmp(&right.atom))
    });

    let static_clean_candidate_count = candidates
        .iter()
        .filter(|candidate| candidate.static_clean_on_current_labeled_set)
        .count();
    let route_best = candidates
        .iter()
        .filter(|candidate| candidate.static_clean_on_current_labeled_set)
        .fold(
            BTreeMap::<String, (usize, String)>::new(),
            |mut best, candidate| {
                let key = format!("{}::{}", candidate.profile_id, candidate.route_key);
                let entry = best.entry(key).or_insert((0, String::new()));
                if candidate.true_over_exact_cache_events > entry.0 {
                    *entry = (
                        candidate.true_over_exact_cache_events,
                        format!("{}:{}", candidate.atom_family, candidate.atom),
                    );
                }
                best
            },
        );
    for route in route_states.values_mut() {
        if let Some((best_true, atom)) = route_best.get(&route.bucket_key) {
            route.best_true_over_exact = *best_true;
            route.best_candidate_atom = atom.clone();
        }
    }
    for candidate in candidates
        .iter()
        .filter(|candidate| candidate.static_clean_on_current_labeled_set)
    {
        let key = format!("{}::{}", candidate.profile_id, candidate.route_key);
        if let Some(route) = route_states.get_mut(&key) {
            route.static_clean_candidate_count += 1;
        }
    }
    let mut route_summaries = route_states
        .into_values()
        .map(|state| GenericSeparatorRouteSummaryReport {
            bucket_key: state.bucket_key,
            route_key: state.route_key,
            profile_id: state.profile_id,
            events: state.events,
            verifier_true_events: state.verifier_true_events,
            verifier_false_events: state.verifier_false_events,
            exact_cache_hits: state.exact_cache_hits,
            static_clean_candidate_count: state.static_clean_candidate_count,
            best_true_over_exact: state.best_true_over_exact,
            best_candidate_atom: state.best_candidate_atom,
        })
        .collect::<Vec<_>>();
    route_summaries.sort_by(|left, right| {
        right
            .best_true_over_exact
            .cmp(&left.best_true_over_exact)
            .then_with(|| right.verifier_true_events.cmp(&left.verifier_true_events))
            .then_with(|| left.bucket_key.cmp(&right.bucket_key))
    });
    candidates.truncate(top_n);

    let report = GenericSeparatorAuditReport {
        report_kind: "generic_real_traffic_phase_center_separator_audit_v1",
        mode: "audit_only_request_side_atom_mining",
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        min_true_over_exact,
        top_n,
        parsed_verifier_labeled_events: events.len(),
        skipped_no_shadow_request,
        skipped_no_verifier_label,
        skipped_legacy_profile_events,
        exact_cache_hits: exact_cache_flags.iter().filter(|hit| **hit).count(),
        route_summaries,
        static_clean_candidate_count,
        top_candidates: candidates,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "audit only: ranks request-side separator atoms for future phase-center bucket experiments; does not compile, promote, serve, local-accept, or claim market money",
    };

    write_json_file(&report_path, &report)?;
    println!("generic_real_traffic_phase_center_separator_audit_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  parsed_verifier_labeled_events: {}",
        report.parsed_verifier_labeled_events
    );
    println!(
        "  static_clean_candidate_count: {}",
        report.static_clean_candidate_count
    );
    println!("  top_candidates: {}", report.top_candidates.len());
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_real_traffic_guarded_separator_shadow_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_GUARDED_SEPARATOR_SHADOW_REPORT));
    let package_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_GUARDED_SEPARATOR_PACKAGE_DIR));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells '{value}' for guarded shadow: {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CELLS);
    let min_bucket_events = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!("invalid min_bucket_events '{value}' for guarded shadow: {error}")
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value.parse::<i64>().map_err(|error| {
                format!("invalid margin_threshold_micro '{value}' for guarded shadow: {error}")
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
    let max_guards = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!("invalid max_guards '{value}' for guarded shadow: {error}")
            })
        })
        .transpose()?
        .unwrap_or(16);
    let selector_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_SEPARATOR_AUDIT_REPORT));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_enriched_trace_paths()
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no real-traffic trace paths provided".to_owned());
    }
    let selected_guards = selected_separator_guards_from_report(&selector_report_path, max_guards)?;
    if selected_guards.is_empty() {
        return Err("separator report did not provide any guard candidates".to_owned());
    }

    let mut total_rows = 0usize;
    let mut parsed_events = Vec::new();
    let mut skipped_no_shadow_request = 0usize;
    let mut skipped_no_verifier_label = 0usize;
    let mut skipped_legacy_profile_events = 0usize;
    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read guarded shadow trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows += 1;
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse guarded shadow trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            match parse_generic_real_traffic_event(&row, parsed_events.len()) {
                GenericParseResult::Event(event) => parsed_events.push(*event),
                GenericParseResult::NoShadowRequest => skipped_no_shadow_request += 1,
                GenericParseResult::NoVerifierLabel => skipped_no_verifier_label += 1,
                GenericParseResult::LegacyProfile => skipped_legacy_profile_events += 1,
            }
        }
    }
    if parsed_events.is_empty() {
        return Err("no verifier-labeled non-legacy real-traffic events found".to_owned());
    }

    let exact_cache_flags = exact_cache_hit_flags_generic(&parsed_events);
    let mut route_states: BTreeMap<String, GenericOnlineBucketState> = BTreeMap::new();
    let mut guard_states: BTreeMap<String, GenericOnlineBucketState> = BTreeMap::new();
    for (event_index, event) in parsed_events.iter().enumerate() {
        let route_bucket_key = format!("{}::{}", event.profile_id, event.route_key);
        if !route_states.contains_key(&route_bucket_key) {
            route_states.insert(
                route_bucket_key.clone(),
                GenericOnlineBucketState {
                    bucket_key: route_bucket_key.clone(),
                    route_key: event.route_key.clone(),
                    profile_id: event.profile_id.clone(),
                    event_indices: Vec::new(),
                    verifier_true_events: 0,
                    verifier_false_events: 0,
                    compiled: None,
                    shadow_events: 0,
                    shadow_safe_events: 0,
                    shadow_accepts: 0,
                    false_accepts: 0,
                    missed_safe_accepts: 0,
                    runtime_margin_parity_mismatches: 0,
                    margins: Vec::new(),
                    exact_cache_hits_in_shadow: 0,
                    unique_cpu_accepts_over_exact_cache: 0,
                    nando_cpu_tokens_saved: 0,
                    nando_cpu_cost_saved_microusd: 0,
                    unique_accepts: BTreeMap::new(),
                    token_cost_evidence_missing_events: 0,
                    token_evidence_missing_events: 0,
                    cost_evidence_missing_events: 0,
                },
            );
        }

        let parent_compiled = route_states
            .get(&route_bucket_key)
            .and_then(|state| state.compiled.clone());
        let matching_guards = selected_guards
            .iter()
            .filter(|guard| separator_guard_matches_event(guard, event))
            .collect::<Vec<_>>();
        for guard in &matching_guards {
            let bucket_key = separator_guard_bucket_key(guard);
            if !guard_states.contains_key(&bucket_key) {
                guard_states.insert(
                    bucket_key.clone(),
                    GenericOnlineBucketState {
                        bucket_key: bucket_key.clone(),
                        route_key: event.route_key.clone(),
                        profile_id: event.profile_id.clone(),
                        event_indices: Vec::new(),
                        verifier_true_events: 0,
                        verifier_false_events: 0,
                        compiled: None,
                        shadow_events: 0,
                        shadow_safe_events: 0,
                        shadow_accepts: 0,
                        false_accepts: 0,
                        missed_safe_accepts: 0,
                        runtime_margin_parity_mismatches: 0,
                        margins: Vec::new(),
                        exact_cache_hits_in_shadow: 0,
                        unique_cpu_accepts_over_exact_cache: 0,
                        nando_cpu_tokens_saved: 0,
                        nando_cpu_cost_saved_microusd: 0,
                        unique_accepts: BTreeMap::new(),
                        token_cost_evidence_missing_events: 0,
                        token_evidence_missing_events: 0,
                        cost_evidence_missing_events: 0,
                    },
                );
            }
            let state = guard_states
                .get_mut(&bucket_key)
                .expect("guarded bucket inserted before use");
            if event.verified_safe_accept {
                state.verifier_true_events += 1;
            } else {
                state.verifier_false_events += 1;
            }
            if let Some(compiled) = parent_compiled.clone() {
                if state.compiled.is_none() {
                    state.compiled = Some(compiled.clone());
                }
                score_generic_shadow_event(
                    state,
                    &compiled,
                    event,
                    exact_cache_flags[event_index],
                    cells,
                    margin_threshold_micro,
                )?;
            } else {
                state.event_indices.push(event_index);
            }
        }

        let compile_request = {
            let state = route_states
                .get_mut(&route_bucket_key)
                .expect("route bucket inserted before use");
            if event.verified_safe_accept {
                state.verifier_true_events += 1;
            } else {
                state.verifier_false_events += 1;
            }
            if state.compiled.is_some() {
                None
            } else {
                state.event_indices.push(event_index);
                let (true_count, false_count) =
                    generic_label_counts_for_indices(&parsed_events, &state.event_indices);
                if state.event_indices.len() >= min_bucket_events
                    && true_count > 0
                    && false_count > 0
                {
                    Some((state.bucket_key.clone(), state.event_indices.clone()))
                } else {
                    None
                }
            }
        };

        if let Some((compile_bucket_key, event_indices)) = compile_request {
            let package_path = package_dir.join(format!(
                "{}.parent.stream.nwpc",
                sanitize_file_stem(&compile_bucket_key)
            ));
            let compiled = compile_generic_bucket(
                &compile_bucket_key,
                &parsed_events,
                &event_indices,
                cells,
                margin_threshold_micro,
                &package_path,
                event_index,
            )?;
            route_states
                .get_mut(&route_bucket_key)
                .expect("route bucket exists for compiled state")
                .compiled = Some(compiled);
        }
    }

    let mut reports = guard_states
        .values()
        .map(generic_bucket_report)
        .collect::<Vec<_>>();
    reports.sort_by(|left, right| left.bucket_key.cmp(&right.bucket_key));
    let compiled_bucket_count = reports
        .iter()
        .filter(|bucket| !bucket.package_path.is_empty())
        .count();
    let accepted_bucket_count = reports
        .iter()
        .filter(|bucket| bucket.accepted_for_online_shadow_review)
        .count();
    let stream_shadow_events = reports.iter().map(|bucket| bucket.shadow_events).sum();
    let stream_shadow_safe_events = reports.iter().map(|bucket| bucket.shadow_safe_events).sum();
    let stream_shadow_accepts = reports.iter().map(|bucket| bucket.shadow_accepts).sum();
    let stream_false_accepts = reports.iter().map(|bucket| bucket.false_accepts).sum();
    let token_cost_evidence_missing_events = reports
        .iter()
        .map(|bucket| bucket.token_cost_evidence_missing_events)
        .sum();
    let token_evidence_missing_events = reports
        .iter()
        .map(|bucket| bucket.token_evidence_missing_events)
        .sum();
    let cost_evidence_missing_events = reports
        .iter()
        .map(|bucket| bucket.cost_evidence_missing_events)
        .sum();
    let mut unique_accepts = BTreeMap::<String, GenericAcceptedEventReport>::new();
    for bucket in reports
        .iter()
        .filter(|bucket| bucket.accepted_for_online_shadow_review && bucket.false_accepts == 0)
    {
        for accepted in &bucket.unique_accepts {
            unique_accepts
                .entry(accepted.request_fingerprint.clone())
                .or_insert_with(|| accepted.clone());
        }
    }
    let total_unique_cpu_accepts_over_exact_cache = unique_accepts.len();
    let total_nando_cpu_tokens_saved = unique_accepts
        .values()
        .map(|accepted| accepted.total_tokens)
        .sum::<usize>();
    let total_nando_cpu_cost_saved_microusd = unique_accepts
        .values()
        .map(|accepted| accepted.total_cost_microusd)
        .sum::<u64>();

    let report = GenericGuardedSeparatorShadowReport {
        report_kind: "generic_real_traffic_phase_center_guarded_separator_shadow_v1",
        mode: "candidate_selected_online_shadow_review",
        bucket_mode: "separator_guard_v1",
        selector_report_path: selector_report_path.display().to_string(),
        max_guards,
        selected_guard_count: selected_guards.len(),
        selected_guards: selected_guards
            .iter()
            .map(GenericSeparatorGuardSpec::to_report)
            .collect(),
        cells,
        min_bucket_events,
        margin_threshold_micro,
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        candidate_package_dir: package_dir.display().to_string(),
        total_rows,
        parsed_candidate_events: parsed_events.len(),
        skipped_no_shadow_request,
        skipped_no_verifier_label,
        skipped_legacy_profile_events,
        bucket_count: reports.len(),
        compiled_bucket_count,
        accepted_bucket_count,
        stream_shadow_events,
        stream_shadow_safe_events,
        stream_shadow_accepts,
        stream_false_accepts,
        total_unique_cpu_accepts_over_exact_cache,
        total_nando_cpu_tokens_saved,
        total_nando_cpu_cost_saved_microusd,
        token_cost_evidence_missing_events,
        token_evidence_missing_events,
        cost_evidence_missing_events,
        buckets: reports,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "candidate-selected guarded shadow review only: separator guards come from an audit report and must not be treated as market proof; no serving, local accept, promotion, lookup, target/proof authority, or legacy backend",
    };

    write_json_file(&report_path, &report)?;
    println!("generic_real_traffic_phase_center_guarded_separator_shadow_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  selected_guard_count: {}", report.selected_guard_count);
    println!("  bucket_count: {}", report.bucket_count);
    println!("  compiled_bucket_count: {}", report.compiled_bucket_count);
    println!("  accepted_bucket_count: {}", report.accepted_bucket_count);
    println!("  stream_false_accepts: {}", report.stream_false_accepts);
    println!(
        "  total_unique_cpu_accepts_over_exact_cache: {}",
        report.total_unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  total_nando_cpu_tokens_saved: {}",
        report.total_nando_cpu_tokens_saved
    );
    println!(
        "  total_nando_cpu_cost_saved_microusd: {}",
        report.total_nando_cpu_cost_saved_microusd
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_real_traffic_guarded_separator_split_shadow_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_GUARDED_SEPARATOR_SPLIT_SHADOW_REPORT));
    let package_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_GUARDED_SEPARATOR_SPLIT_PACKAGE_DIR));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells '{value}' for split shadow: {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CELLS);
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    let min_bucket_events = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!("invalid min_bucket_events '{value}' for split shadow: {error}")
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    if min_bucket_events < 2 {
        return Err("min_bucket_events must be >= 2".to_owned());
    }
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value.parse::<i64>().map_err(|error| {
                format!("invalid margin_threshold_micro '{value}' for split shadow: {error}")
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_SPLIT_SHADOW_MARGIN_THRESHOLD_MICRO);
    if margin_threshold_micro <= 0 {
        return Err("margin_threshold_micro must be > 0".to_owned());
    }
    let max_guards = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_guards '{value}' for split shadow: {error}"))
        })
        .transpose()?
        .unwrap_or(16);
    let selector_permille = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!("invalid selector_permille '{value}' for split shadow: {error}")
            })
        })
        .transpose()?
        .unwrap_or(300);
    let train_permille = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!("invalid train_permille '{value}' for split shadow: {error}")
            })
        })
        .transpose()?
        .unwrap_or(400);
    if selector_permille == 0 || train_permille == 0 {
        return Err("selector_permille and train_permille must be > 0".to_owned());
    }
    if selector_permille.saturating_add(train_permille) >= 1000 {
        return Err("selector_permille + train_permille must be < 1000".to_owned());
    }
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_enriched_trace_paths()
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no real-traffic trace paths provided".to_owned());
    }

    let mut total_rows = 0usize;
    let mut parsed_events = Vec::new();
    let mut skipped_no_shadow_request = 0usize;
    let mut skipped_no_verifier_label = 0usize;
    let mut skipped_legacy_profile_events = 0usize;
    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read split shadow trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows += 1;
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse split shadow trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            match parse_generic_real_traffic_event(&row, parsed_events.len()) {
                GenericParseResult::Event(event) => parsed_events.push(*event),
                GenericParseResult::NoShadowRequest => skipped_no_shadow_request += 1,
                GenericParseResult::NoVerifierLabel => skipped_no_verifier_label += 1,
                GenericParseResult::LegacyProfile => skipped_legacy_profile_events += 1,
            }
        }
    }
    if parsed_events.len() < 3 {
        return Err("not enough verifier-labeled non-legacy events for split shadow".to_owned());
    }
    let exact_cache_flags = exact_cache_hit_flags_generic(&parsed_events);
    let mut route_event_indices = BTreeMap::<String, Vec<usize>>::new();
    for (event_index, event) in parsed_events.iter().enumerate() {
        route_event_indices
            .entry(format!("{}::{}", event.profile_id, event.route_key))
            .or_default()
            .push(event_index);
    }

    let mut selector_indices = BTreeSet::<usize>::new();
    let mut train_indices = BTreeSet::<usize>::new();
    let mut shadow_indices = BTreeSet::<usize>::new();
    for indices in route_event_indices.values() {
        if indices.len() < 3 {
            continue;
        }
        let selector_count = (indices.len() * selector_permille / 1000)
            .max(1)
            .min(indices.len().saturating_sub(2));
        let train_end_count = (indices.len() * (selector_permille + train_permille) / 1000)
            .max(selector_count + 1)
            .min(indices.len().saturating_sub(1));
        for &event_index in &indices[..selector_count] {
            selector_indices.insert(event_index);
        }
        for &event_index in &indices[selector_count..train_end_count] {
            train_indices.insert(event_index);
        }
        for &event_index in &indices[train_end_count..] {
            shadow_indices.insert(event_index);
        }
    }
    let selector_train_shadow_disjoint = selector_indices.is_disjoint(&train_indices)
        && selector_indices.is_disjoint(&shadow_indices)
        && train_indices.is_disjoint(&shadow_indices)
        && !selector_indices.is_empty()
        && !train_indices.is_empty()
        && !shadow_indices.is_empty();
    if !selector_train_shadow_disjoint {
        return Err(
            "route-local selector/train/shadow windows are not strictly disjoint".to_owned(),
        );
    }
    let selector_index_list = selector_indices.iter().copied().collect::<Vec<_>>();
    let selected_guards = selected_separator_guards_from_events(
        &parsed_events,
        &exact_cache_flags,
        &selector_index_list,
        max_guards,
    );
    if selected_guards.is_empty() {
        return Err("selector window did not provide any static-clean guard candidates".to_owned());
    }

    let mut route_train_indices: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for event_index in train_indices.iter().copied() {
        let event = &parsed_events[event_index];
        route_train_indices
            .entry(format!("{}::{}", event.profile_id, event.route_key))
            .or_default()
            .push(event_index);
    }
    let route_train_bucket_count = route_train_indices.len();
    let mut route_compiled = BTreeMap::<String, GenericCompiledBucket>::new();
    for (route_bucket_key, event_indices) in &route_train_indices {
        let (true_count, false_count) =
            generic_label_counts_for_indices(&parsed_events, event_indices);
        if event_indices.len() < min_bucket_events || true_count == 0 || false_count == 0 {
            continue;
        }
        let package_path = package_dir.join(format!(
            "{}.split-parent.nwpc",
            sanitize_file_stem(route_bucket_key)
        ));
        let compiled = compile_generic_bucket(
            route_bucket_key,
            &parsed_events,
            event_indices,
            cells,
            margin_threshold_micro,
            &package_path,
            train_indices.iter().next_back().copied().unwrap_or(0),
        )?;
        route_compiled.insert(route_bucket_key.clone(), compiled);
    }
    let route_compiled_bucket_count = route_compiled.len();

    let mut guard_states = BTreeMap::<String, GenericOnlineBucketState>::new();
    for guard in &selected_guards {
        let bucket_key = separator_guard_bucket_key(guard);
        let mut state = new_generic_online_bucket_state(
            bucket_key.clone(),
            guard.route_key.clone(),
            guard.profile_id.clone(),
        );
        if let Some(compiled) =
            route_compiled.get(&format!("{}::{}", guard.profile_id, guard.route_key))
        {
            state.compiled = Some(compiled.clone());
        }
        guard_states.insert(bucket_key, state);
    }

    for event_index in shadow_indices.iter().copied() {
        let event = &parsed_events[event_index];
        let matching_guards = selected_guards
            .iter()
            .filter(|guard| separator_guard_matches_event(guard, event))
            .collect::<Vec<_>>();
        for guard in matching_guards {
            let bucket_key = separator_guard_bucket_key(guard);
            let Some(state) = guard_states.get_mut(&bucket_key) else {
                continue;
            };
            if event.verified_safe_accept {
                state.verifier_true_events += 1;
            } else {
                state.verifier_false_events += 1;
            }
            let Some(compiled) = state.compiled.clone() else {
                continue;
            };
            score_generic_shadow_event(
                state,
                &compiled,
                event,
                exact_cache_flags[event_index],
                cells,
                margin_threshold_micro,
            )?;
        }
    }

    let mut reports = guard_states
        .values()
        .map(generic_bucket_report)
        .collect::<Vec<_>>();
    reports.sort_by(|left, right| left.bucket_key.cmp(&right.bucket_key));
    let compiled_bucket_count = reports
        .iter()
        .filter(|bucket| !bucket.package_path.is_empty())
        .count();
    let accepted_bucket_count = reports
        .iter()
        .filter(|bucket| bucket.accepted_for_online_shadow_review)
        .count();
    let stream_shadow_events = reports.iter().map(|bucket| bucket.shadow_events).sum();
    let stream_shadow_safe_events = reports.iter().map(|bucket| bucket.shadow_safe_events).sum();
    let stream_shadow_accepts = reports.iter().map(|bucket| bucket.shadow_accepts).sum();
    let stream_false_accepts = reports.iter().map(|bucket| bucket.false_accepts).sum();
    let token_cost_evidence_missing_events = reports
        .iter()
        .map(|bucket| bucket.token_cost_evidence_missing_events)
        .sum();
    let token_evidence_missing_events = reports
        .iter()
        .map(|bucket| bucket.token_evidence_missing_events)
        .sum();
    let cost_evidence_missing_events = reports
        .iter()
        .map(|bucket| bucket.cost_evidence_missing_events)
        .sum();
    let mut unique_accepts = BTreeMap::<String, GenericAcceptedEventReport>::new();
    for bucket in reports
        .iter()
        .filter(|bucket| bucket.accepted_for_online_shadow_review && bucket.false_accepts == 0)
    {
        for accepted in &bucket.unique_accepts {
            unique_accepts
                .entry(accepted.request_fingerprint.clone())
                .or_insert_with(|| accepted.clone());
        }
    }
    let total_unique_cpu_accepts_over_exact_cache = unique_accepts.len();
    let total_nando_cpu_tokens_saved = unique_accepts
        .values()
        .map(|accepted| accepted.total_tokens)
        .sum::<usize>();
    let total_nando_cpu_cost_saved_microusd = unique_accepts
        .values()
        .map(|accepted| accepted.total_cost_microusd)
        .sum::<u64>();

    let report = GenericGuardedSeparatorSplitShadowReport {
        report_kind: "generic_real_traffic_phase_center_guarded_separator_split_shadow_v1",
        mode: "split_window_guarded_shadow_review",
        bucket_mode: "separator_guard_split_v1",
        split_granularity: "route_local_event_order",
        global_contiguous_windows: false,
        selector_source: "route_local_first_window_static_clean_request_side_atoms",
        selector_permille,
        train_permille,
        selector_event_start: selector_indices.iter().next().copied().unwrap_or(0),
        selector_event_end: selector_indices
            .iter()
            .next_back()
            .map(|index| index + 1)
            .unwrap_or(0),
        train_event_start: train_indices.iter().next().copied().unwrap_or(0),
        train_event_end: train_indices
            .iter()
            .next_back()
            .map(|index| index + 1)
            .unwrap_or(0),
        shadow_event_start: shadow_indices.iter().next().copied().unwrap_or(0),
        shadow_event_end: shadow_indices
            .iter()
            .next_back()
            .map(|index| index + 1)
            .unwrap_or(0),
        selector_events: selector_indices.len(),
        train_events: train_indices.len(),
        shadow_events_window: shadow_indices.len(),
        selector_train_shadow_disjoint,
        shadow_window_independent: selector_train_shadow_disjoint,
        max_guards,
        selected_guard_count: selected_guards.len(),
        selected_guards: selected_guards
            .iter()
            .map(GenericSeparatorGuardSpec::to_report)
            .collect(),
        cells,
        min_bucket_events,
        margin_threshold_micro,
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        candidate_package_dir: package_dir.display().to_string(),
        total_rows,
        parsed_candidate_events: parsed_events.len(),
        skipped_no_shadow_request,
        skipped_no_verifier_label,
        skipped_legacy_profile_events,
        route_train_bucket_count,
        route_compiled_bucket_count,
        bucket_count: reports.len(),
        compiled_bucket_count,
        accepted_bucket_count,
        stream_shadow_events,
        stream_shadow_safe_events,
        stream_shadow_accepts,
        stream_false_accepts,
        total_unique_cpu_accepts_over_exact_cache,
        total_nando_cpu_tokens_saved,
        total_nando_cpu_cost_saved_microusd,
        token_cost_evidence_missing_events,
        token_evidence_missing_events,
        cost_evidence_missing_events,
        buckets: reports,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "split-window guarded shadow review only: selector, train, and shadow windows are disjoint; no serving, local accept, promotion, lookup, target/proof authority, or legacy backend",
    };

    write_json_file(&report_path, &report)?;
    println!("generic_real_traffic_phase_center_guarded_separator_split_shadow_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  selector_train_shadow_disjoint: {}",
        report.selector_train_shadow_disjoint
    );
    println!("  selected_guard_count: {}", report.selected_guard_count);
    println!(
        "  route_compiled_bucket_count: {}",
        report.route_compiled_bucket_count
    );
    println!("  bucket_count: {}", report.bucket_count);
    println!("  compiled_bucket_count: {}", report.compiled_bucket_count);
    println!("  accepted_bucket_count: {}", report.accepted_bucket_count);
    println!("  stream_false_accepts: {}", report.stream_false_accepts);
    println!(
        "  total_unique_cpu_accepts_over_exact_cache: {}",
        report.total_unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  total_nando_cpu_tokens_saved: {}",
        report.total_nando_cpu_tokens_saved
    );
    println!(
        "  total_nando_cpu_cost_saved_microusd: {}",
        report.total_nando_cpu_cost_saved_microusd
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_real_traffic_guarded_separator_calibrated_split_shadow_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_GENERIC_GUARDED_SEPARATOR_CALIBRATED_SPLIT_SHADOW_REPORT)
    });
    let package_dir = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_GENERIC_GUARDED_SEPARATOR_CALIBRATED_SPLIT_PACKAGE_DIR)
    });
    let cells = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!("invalid cells '{value}' for calibrated split shadow: {error}")
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_CELLS);
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    let min_bucket_events = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!("invalid min_bucket_events '{value}' for calibrated split shadow: {error}")
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS);
    if min_bucket_events < 2 {
        return Err("min_bucket_events must be >= 2".to_owned());
    }
    let calibration_margin_floor_micro = args
        .next()
        .map(|value| {
            value.parse::<i64>().map_err(|error| {
                format!(
                    "invalid calibration_margin_floor_micro '{value}' for calibrated split shadow: {error}"
                )
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_CALIBRATION_MARGIN_FLOOR_MICRO);
    if calibration_margin_floor_micro <= 0 {
        return Err("calibration_margin_floor_micro must be > 0".to_owned());
    }
    let calibration_margin_guard_micro = args
        .next()
        .map(|value| {
            value.parse::<i64>().map_err(|error| {
                format!(
                    "invalid calibration_margin_guard_micro '{value}' for calibrated split shadow: {error}"
                )
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_CALIBRATION_MARGIN_GUARD_MICRO);
    if calibration_margin_guard_micro < 0 {
        return Err("calibration_margin_guard_micro must be >= 0".to_owned());
    }
    let max_guards = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!("invalid max_guards '{value}' for calibrated split shadow: {error}")
            })
        })
        .transpose()?
        .unwrap_or(16);
    let selector_permille = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!("invalid selector_permille '{value}' for calibrated split shadow: {error}")
            })
        })
        .transpose()?
        .unwrap_or(200);
    let compile_permille = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!("invalid compile_permille '{value}' for calibrated split shadow: {error}")
            })
        })
        .transpose()?
        .unwrap_or(300);
    let calibration_permille = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!(
                    "invalid calibration_permille '{value}' for calibrated split shadow: {error}"
                )
            })
        })
        .transpose()?
        .unwrap_or(300);
    if selector_permille == 0 || compile_permille == 0 || calibration_permille == 0 {
        return Err("selector/compile/calibration permille values must be > 0".to_owned());
    }
    if selector_permille
        .saturating_add(compile_permille)
        .saturating_add(calibration_permille)
        >= 1000
    {
        return Err(
            "selector_permille + compile_permille + calibration_permille must be < 1000".to_owned(),
        );
    }
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_enriched_trace_paths()
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no real-traffic trace paths provided".to_owned());
    }

    let mut total_rows = 0usize;
    let mut parsed_events = Vec::new();
    let mut skipped_no_shadow_request = 0usize;
    let mut skipped_no_verifier_label = 0usize;
    let mut skipped_legacy_profile_events = 0usize;
    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read calibrated split shadow trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows += 1;
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse calibrated split shadow trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            match parse_generic_real_traffic_event(&row, parsed_events.len()) {
                GenericParseResult::Event(event) => parsed_events.push(*event),
                GenericParseResult::NoShadowRequest => skipped_no_shadow_request += 1,
                GenericParseResult::NoVerifierLabel => skipped_no_verifier_label += 1,
                GenericParseResult::LegacyProfile => skipped_legacy_profile_events += 1,
            }
        }
    }
    if parsed_events.len() < 4 {
        return Err(
            "not enough verifier-labeled non-legacy events for calibrated split shadow".to_owned(),
        );
    }

    let exact_cache_flags = exact_cache_hit_flags_generic(&parsed_events);
    let split = route_local_four_way_split(
        &parsed_events,
        selector_permille,
        compile_permille,
        calibration_permille,
    )?;
    let selector_index_list = split.selector_indices.iter().copied().collect::<Vec<_>>();
    let selected_guards = selected_separator_guards_from_events(
        &parsed_events,
        &exact_cache_flags,
        &selector_index_list,
        max_guards,
    );
    if selected_guards.is_empty() {
        return Err(
            "selector window did not provide any calibrated static-clean guard candidates"
                .to_owned(),
        );
    }

    let mut route_compile_indices: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for event_index in split.compile_indices.iter().copied() {
        let event = &parsed_events[event_index];
        route_compile_indices
            .entry(format!("{}::{}", event.profile_id, event.route_key))
            .or_default()
            .push(event_index);
    }
    let route_compile_bucket_count = route_compile_indices.len();
    let mut route_compiled = BTreeMap::<String, GenericCompiledBucket>::new();
    for (route_bucket_key, event_indices) in &route_compile_indices {
        let (true_count, false_count) =
            generic_label_counts_for_indices(&parsed_events, event_indices);
        if event_indices.len() < min_bucket_events || true_count == 0 || false_count == 0 {
            continue;
        }
        let package_path = package_dir.join(format!(
            "{}.calibrated-parent.nwpc",
            sanitize_file_stem(route_bucket_key)
        ));
        let compiled = compile_generic_bucket(
            route_bucket_key,
            &parsed_events,
            event_indices,
            cells,
            calibration_margin_floor_micro,
            &package_path,
            split
                .compile_indices
                .iter()
                .next_back()
                .copied()
                .unwrap_or(0),
        )?;
        route_compiled.insert(route_bucket_key.clone(), compiled);
    }
    let route_compiled_bucket_count = route_compiled.len();

    let mut guard_states = BTreeMap::<String, GenericCalibratedBucketState>::new();
    for guard in &selected_guards {
        let bucket_key = separator_guard_bucket_key(guard);
        let mut state = new_generic_online_bucket_state(
            bucket_key.clone(),
            guard.route_key.clone(),
            guard.profile_id.clone(),
        );
        if let Some(compiled) =
            route_compiled.get(&format!("{}::{}", guard.profile_id, guard.route_key))
        {
            state.compiled = Some(compiled.clone());
        }
        guard_states.insert(
            bucket_key,
            GenericCalibratedBucketState {
                state,
                calibrated_margin_threshold_micro: calibration_margin_floor_micro,
                calibration_margins: Vec::new(),
                calibration_events: 0,
                calibration_safe_events: 0,
                calibration_false_events: 0,
                calibration_accepts: 0,
                calibration_false_accepts: 0,
                calibration_max_false_margin_micro: None,
                calibration_min_safe_margin_micro: None,
                calibration_threshold_source: "floor_no_calibration_events",
            },
        );
    }

    for event_index in split.calibration_indices.iter().copied() {
        let event = &parsed_events[event_index];
        let matching_guards = selected_guards
            .iter()
            .filter(|guard| separator_guard_matches_event(guard, event))
            .collect::<Vec<_>>();
        for guard in matching_guards {
            let bucket_key = separator_guard_bucket_key(guard);
            let Some(state) = guard_states.get_mut(&bucket_key) else {
                continue;
            };
            let Some(compiled) = state.state.compiled.clone() else {
                continue;
            };
            let (margin_micro, parity_mismatch) =
                generic_event_margin_micro(&compiled, event, cells)?;
            state
                .calibration_margins
                .push((margin_micro, event.verified_safe_accept));
            if parity_mismatch {
                state.state.runtime_margin_parity_mismatches += 1;
            }
            state.calibration_events += 1;
            if event.verified_safe_accept {
                state.calibration_safe_events += 1;
                state.calibration_min_safe_margin_micro = Some(
                    state
                        .calibration_min_safe_margin_micro
                        .map_or(margin_micro, |current| current.min(margin_micro)),
                );
            } else {
                state.calibration_false_events += 1;
                state.calibration_max_false_margin_micro = Some(
                    state
                        .calibration_max_false_margin_micro
                        .map_or(margin_micro, |current| current.max(margin_micro)),
                );
            }
        }
    }

    for state in guard_states.values_mut() {
        let false_floor = state
            .calibration_max_false_margin_micro
            .map(|margin| margin.saturating_add(calibration_margin_guard_micro));
        state.calibrated_margin_threshold_micro = false_floor
            .map_or(calibration_margin_floor_micro, |value| {
                value.max(calibration_margin_floor_micro)
            });
        state.calibration_threshold_source = if state.calibration_false_events > 0 {
            "max_false_margin_plus_guard"
        } else if state.calibration_events > 0 {
            "floor_no_calibration_false_events"
        } else {
            "floor_no_calibration_events"
        };
        for &(margin_micro, verified_safe_accept) in &state.calibration_margins {
            if margin_micro >= state.calibrated_margin_threshold_micro {
                if verified_safe_accept {
                    state.calibration_accepts += 1;
                } else {
                    state.calibration_false_accepts += 1;
                }
            }
        }
    }

    for event_index in split.shadow_indices.iter().copied() {
        let event = &parsed_events[event_index];
        let matching_guards = selected_guards
            .iter()
            .filter(|guard| separator_guard_matches_event(guard, event))
            .collect::<Vec<_>>();
        for guard in matching_guards {
            let bucket_key = separator_guard_bucket_key(guard);
            let Some(state) = guard_states.get_mut(&bucket_key) else {
                continue;
            };
            if event.verified_safe_accept {
                state.state.verifier_true_events += 1;
            } else {
                state.state.verifier_false_events += 1;
            }
            let Some(compiled) = state.state.compiled.clone() else {
                continue;
            };
            score_generic_shadow_event(
                &mut state.state,
                &compiled,
                event,
                exact_cache_flags[event_index],
                cells,
                state.calibrated_margin_threshold_micro,
            )?;
        }
    }

    let mut bucket_reports = guard_states
        .values()
        .map(generic_calibrated_bucket_report)
        .collect::<Vec<_>>();
    bucket_reports.sort_by(|left, right| left.bucket.bucket_key.cmp(&right.bucket.bucket_key));
    let compiled_bucket_count = bucket_reports
        .iter()
        .filter(|bucket| !bucket.bucket.package_path.is_empty())
        .count();
    let calibrated_bucket_count = bucket_reports
        .iter()
        .filter(|bucket| bucket.calibration_events > 0)
        .count();
    let accepted_bucket_count = bucket_reports
        .iter()
        .filter(|bucket| bucket.bucket.accepted_for_online_shadow_review)
        .count();
    let stream_shadow_events = bucket_reports
        .iter()
        .map(|bucket| bucket.bucket.shadow_events)
        .sum();
    let stream_shadow_safe_events = bucket_reports
        .iter()
        .map(|bucket| bucket.bucket.shadow_safe_events)
        .sum();
    let stream_shadow_accepts = bucket_reports
        .iter()
        .map(|bucket| bucket.bucket.shadow_accepts)
        .sum();
    let stream_false_accepts = bucket_reports
        .iter()
        .map(|bucket| bucket.bucket.false_accepts)
        .sum();
    let token_cost_evidence_missing_events = bucket_reports
        .iter()
        .map(|bucket| bucket.bucket.token_cost_evidence_missing_events)
        .sum();
    let token_evidence_missing_events = bucket_reports
        .iter()
        .map(|bucket| bucket.bucket.token_evidence_missing_events)
        .sum();
    let cost_evidence_missing_events = bucket_reports
        .iter()
        .map(|bucket| bucket.bucket.cost_evidence_missing_events)
        .sum();
    let mut unique_accepts = BTreeMap::<String, GenericAcceptedEventReport>::new();
    for bucket in bucket_reports.iter().filter(|bucket| {
        bucket.bucket.accepted_for_online_shadow_review && bucket.bucket.false_accepts == 0
    }) {
        for accepted in &bucket.bucket.unique_accepts {
            unique_accepts
                .entry(accepted.request_fingerprint.clone())
                .or_insert_with(|| accepted.clone());
        }
    }
    let total_unique_cpu_accepts_over_exact_cache = unique_accepts.len();
    let total_nando_cpu_tokens_saved = unique_accepts
        .values()
        .map(|accepted| accepted.total_tokens)
        .sum::<usize>();
    let total_nando_cpu_cost_saved_microusd = unique_accepts
        .values()
        .map(|accepted| accepted.total_cost_microusd)
        .sum::<u64>();

    let report = GenericGuardedSeparatorCalibratedSplitShadowReport {
        report_kind: "generic_real_traffic_phase_center_guarded_separator_calibrated_split_shadow_v1",
        mode: "route_local_calibrated_split_shadow_review",
        bucket_mode: "separator_guard_calibrated_split_v1",
        split_granularity: "route_local_event_order",
        selector_permille,
        compile_permille,
        calibration_permille,
        selector_events: split.selector_indices.len(),
        compile_events: split.compile_indices.len(),
        calibration_events: split.calibration_indices.len(),
        shadow_events_window: split.shadow_indices.len(),
        selector_compile_calibration_shadow_disjoint: split.disjoint,
        shadow_window_independent: split.disjoint,
        max_guards,
        selected_guard_count: selected_guards.len(),
        selected_guards: selected_guards
            .iter()
            .map(GenericSeparatorGuardSpec::to_report)
            .collect(),
        cells,
        min_bucket_events,
        calibration_margin_floor_micro,
        calibration_margin_guard_micro,
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        candidate_package_dir: package_dir.display().to_string(),
        total_rows,
        parsed_candidate_events: parsed_events.len(),
        skipped_no_shadow_request,
        skipped_no_verifier_label,
        skipped_legacy_profile_events,
        route_compile_bucket_count,
        route_compiled_bucket_count,
        bucket_count: bucket_reports.len(),
        compiled_bucket_count,
        calibrated_bucket_count,
        accepted_bucket_count,
        stream_shadow_events,
        stream_shadow_safe_events,
        stream_shadow_accepts,
        stream_false_accepts,
        total_unique_cpu_accepts_over_exact_cache,
        total_nando_cpu_tokens_saved,
        total_nando_cpu_cost_saved_microusd,
        token_cost_evidence_missing_events,
        token_evidence_missing_events,
        cost_evidence_missing_events,
        buckets: bucket_reports,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "calibrated split-window shadow review only: selector, compile, calibration, and shadow windows are disjoint; thresholds are selected before shadow; no serving, local accept, promotion, lookup, target/proof authority, or legacy backend",
    };
    write_json_file(&report_path, &report)?;
    println!("generic_real_traffic_phase_center_guarded_separator_calibrated_split_shadow_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  split_disjoint: {}",
        report.selector_compile_calibration_shadow_disjoint
    );
    println!("  selected_guard_count: {}", report.selected_guard_count);
    println!(
        "  route_compiled_bucket_count: {}",
        report.route_compiled_bucket_count
    );
    println!(
        "  calibrated_bucket_count: {}",
        report.calibrated_bucket_count
    );
    println!("  accepted_bucket_count: {}", report.accepted_bucket_count);
    println!("  stream_false_accepts: {}", report.stream_false_accepts);
    println!(
        "  total_unique_cpu_accepts_over_exact_cache: {}",
        report.total_unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  total_nando_cpu_tokens_saved: {}",
        report.total_nando_cpu_tokens_saved
    );
    println!(
        "  total_nando_cpu_cost_saved_microusd: {}",
        report.total_nando_cpu_cost_saved_microusd
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

fn run_phase_stream_real_traffic_online_discovery_impl<I>(
    mut args: I,
    bucket_mode: GenericBucketMode,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_REAL_TRAFFIC_REPORT));
    let package_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_REAL_TRAFFIC_PACKAGE_DIR));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_CELLS);
    if cells == 0 {
        return Err("cells must be > 0".to_owned());
    }
    let min_bucket_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid min bucket events '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(16);
    if min_bucket_events < 2 {
        return Err("min bucket events must be >= 2".to_owned());
    }
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin threshold '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
    if margin_threshold_micro <= 0 {
        return Err("margin threshold must be > 0".to_owned());
    }
    let price_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PRICE_CONFIG));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_generic_real_traffic_trace_paths()
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no real-traffic trace paths provided".to_owned());
    }
    let _price_config = read_json_file::<ModelPriceConfig>(&price_config_path)?;

    let mut total_rows = 0usize;
    let mut parsed_events = Vec::new();
    let mut skipped_no_shadow_request = 0usize;
    let mut skipped_no_verifier_label = 0usize;
    let mut skipped_legacy_profile_events = 0usize;
    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read real-traffic trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows += 1;
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse real-traffic trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            match parse_generic_real_traffic_event(&row, parsed_events.len()) {
                GenericParseResult::Event(event) => parsed_events.push(*event),
                GenericParseResult::NoShadowRequest => skipped_no_shadow_request += 1,
                GenericParseResult::NoVerifierLabel => skipped_no_verifier_label += 1,
                GenericParseResult::LegacyProfile => skipped_legacy_profile_events += 1,
            }
        }
    }
    if parsed_events.is_empty() {
        return Err("no non-legacy verifier-bound real-traffic events found".to_owned());
    }

    let exact_cache_flags = exact_cache_hit_flags_generic(&parsed_events);
    let mut buckets: BTreeMap<String, GenericOnlineBucketState> = BTreeMap::new();
    for (event_index, event) in parsed_events.iter().enumerate() {
        let bucket_key = generic_bucket_key(event, bucket_mode);
        if !buckets.contains_key(&bucket_key) {
            buckets.insert(
                bucket_key.clone(),
                GenericOnlineBucketState {
                    bucket_key: bucket_key.clone(),
                    route_key: event.route_key.clone(),
                    profile_id: event.profile_id.clone(),
                    event_indices: Vec::new(),
                    verifier_true_events: 0,
                    verifier_false_events: 0,
                    compiled: None,
                    shadow_events: 0,
                    shadow_safe_events: 0,
                    shadow_accepts: 0,
                    false_accepts: 0,
                    missed_safe_accepts: 0,
                    runtime_margin_parity_mismatches: 0,
                    margins: Vec::new(),
                    exact_cache_hits_in_shadow: 0,
                    unique_cpu_accepts_over_exact_cache: 0,
                    nando_cpu_tokens_saved: 0,
                    nando_cpu_cost_saved_microusd: 0,
                    unique_accepts: BTreeMap::new(),
                    token_cost_evidence_missing_events: 0,
                    token_evidence_missing_events: 0,
                    cost_evidence_missing_events: 0,
                },
            );
        }

        let compile_request = {
            let state = buckets
                .get_mut(&bucket_key)
                .expect("generic bucket inserted before use");
            if event.verified_safe_accept {
                state.verifier_true_events += 1;
            } else {
                state.verifier_false_events += 1;
            }
            if let Some(compiled) = state.compiled.clone() {
                score_generic_shadow_event(
                    state,
                    &compiled,
                    event,
                    exact_cache_flags[event_index],
                    cells,
                    margin_threshold_micro,
                )?;
                None
            } else {
                state.event_indices.push(event_index);
                let (true_count, false_count) =
                    generic_label_counts_for_indices(&parsed_events, &state.event_indices);
                if state.event_indices.len() >= min_bucket_events
                    && true_count > 0
                    && false_count > 0
                {
                    Some((state.bucket_key.clone(), state.event_indices.clone()))
                } else {
                    None
                }
            }
        };

        if let Some((compile_bucket_key, event_indices)) = compile_request {
            let package_path = package_dir.join(format!(
                "{}.stream.nwpc",
                sanitize_file_stem(&compile_bucket_key)
            ));
            let compiled = compile_generic_bucket(
                &compile_bucket_key,
                &parsed_events,
                &event_indices,
                cells,
                margin_threshold_micro,
                &package_path,
                event_index,
            )?;
            buckets
                .get_mut(&bucket_key)
                .expect("generic bucket exists for compiled state")
                .compiled = Some(compiled);
        }
    }

    let mut reports = buckets
        .values()
        .map(generic_bucket_report)
        .collect::<Vec<_>>();
    reports.sort_by(|left, right| left.bucket_key.cmp(&right.bucket_key));
    let compiled_bucket_count = reports
        .iter()
        .filter(|bucket| bucket.compiled_after_global_event_index.is_some())
        .count();
    let accepted_bucket_count = reports
        .iter()
        .filter(|bucket| bucket.accepted_for_online_shadow_review)
        .count();
    let stream_shadow_events = reports.iter().map(|bucket| bucket.shadow_events).sum();
    let stream_shadow_safe_events = reports.iter().map(|bucket| bucket.shadow_safe_events).sum();
    let stream_shadow_accepts = reports.iter().map(|bucket| bucket.shadow_accepts).sum();
    let stream_false_accepts = reports.iter().map(|bucket| bucket.false_accepts).sum();
    let total_unique_cpu_accepts_over_exact_cache = reports
        .iter()
        .filter(|bucket| bucket.accepted_for_online_shadow_review)
        .map(|bucket| bucket.unique_cpu_accepts_over_exact_cache)
        .sum();
    let total_nando_cpu_tokens_saved = reports
        .iter()
        .filter(|bucket| bucket.accepted_for_online_shadow_review)
        .map(|bucket| bucket.nando_cpu_tokens_saved)
        .sum();
    let total_nando_cpu_cost_saved_microusd = reports
        .iter()
        .filter(|bucket| bucket.accepted_for_online_shadow_review)
        .map(|bucket| bucket.nando_cpu_cost_saved_microusd)
        .sum();
    let token_cost_evidence_missing_events = reports
        .iter()
        .map(|bucket| bucket.token_cost_evidence_missing_events)
        .sum();
    let token_evidence_missing_events = reports
        .iter()
        .map(|bucket| bucket.token_evidence_missing_events)
        .sum();
    let cost_evidence_missing_events = reports
        .iter()
        .map(|bucket| bucket.cost_evidence_missing_events)
        .sum();

    let report = GenericRealTrafficOnlineDiscoveryReport {
        report_kind: "generic_real_traffic_phase_center_online_discovery_v1",
        mode: "online_shadow_discovery_only",
        bucket_mode: bucket_mode.as_str(),
        cells,
        min_bucket_events,
        margin_threshold_micro,
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        candidate_package_dir: package_dir.display().to_string(),
        total_rows,
        parsed_candidate_events: parsed_events.len(),
        skipped_no_shadow_request,
        skipped_no_verifier_label,
        skipped_legacy_profile_events,
        bucket_count: reports.len(),
        compiled_bucket_count,
        accepted_bucket_count,
        stream_shadow_events,
        stream_shadow_safe_events,
        stream_shadow_accepts,
        stream_false_accepts,
        total_unique_cpu_accepts_over_exact_cache,
        total_nando_cpu_tokens_saved,
        total_nando_cpu_cost_saved_microusd,
        token_cost_evidence_missing_events,
        token_evidence_missing_events,
        cost_evidence_missing_events,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        buckets: reports,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "generic real-traffic online-order shadow discovery over non-legacy verifier-bound nando_shadow_request atoms; compiles quarantine .nwpc candidates and scores only future events; no product local_accept, serving promotion, legacy backend, or market money claim",
    };
    write_json_file(&report_path, &report)?;
    println!("generic_real_traffic_phase_center_online_discovery_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  parsed_candidate_events: {}",
        report.parsed_candidate_events
    );
    println!(
        "  skipped_legacy_profile_events: {}",
        report.skipped_legacy_profile_events
    );
    println!("  bucket_count: {}", report.bucket_count);
    println!("  compiled_bucket_count: {}", report.compiled_bucket_count);
    println!("  accepted_bucket_count: {}", report.accepted_bucket_count);
    println!("  stream_shadow_events: {}", report.stream_shadow_events);
    println!("  stream_shadow_accepts: {}", report.stream_shadow_accepts);
    println!("  stream_false_accepts: {}", report.stream_false_accepts);
    println!(
        "  total_unique_cpu_accepts_over_exact_cache: {}",
        report.total_unique_cpu_accepts_over_exact_cache
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    Ok(())
}

pub(crate) fn run_phase_stream_real_traffic_cost_evidence_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_COST_EVIDENCE_AUDIT_REPORT));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_cost_evidence_audit_trace_paths()
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no real-traffic trace paths provided".to_owned());
    }

    let mut total_rows = 0usize;
    let mut shadow_request_rows = 0usize;
    let mut skipped_legacy_profile_events = 0usize;
    let mut nonlegacy_candidate_rows = 0usize;
    let mut no_verifier_label_rows = 0usize;
    let mut verifier_true_events = 0usize;
    let mut verifier_false_events = 0usize;
    let mut provider_cost_events = 0usize;
    let mut estimated_cost_events = 0usize;
    let mut token_events = 0usize;
    let mut token_or_cost_events = 0usize;
    let mut verifier_bound_token_or_cost_events = 0usize;
    let mut buckets = BTreeMap::<String, GenericCostEvidenceBucketState>::new();

    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read real-traffic trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows += 1;
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse real-traffic trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            let Some(request) = row
                .get("nando_shadow_request")
                .and_then(serde_json::Value::as_object)
            else {
                continue;
            };
            shadow_request_rows += 1;

            let route_key =
                json_field_string(request.get("route_key")).unwrap_or_else(|| "unknown".into());
            let profile_id =
                json_field_string(request.get("profile_id")).unwrap_or_else(|| route_key.clone());
            if is_legacy_profile_name(&route_key) || is_legacy_profile_name(&profile_id) {
                skipped_legacy_profile_events += 1;
                continue;
            }

            nonlegacy_candidate_rows += 1;
            let verified_safe_accept = row
                .get("verified_safe_accept")
                .and_then(serde_json::Value::as_bool);
            if verified_safe_accept == Some(true) {
                verifier_true_events += 1;
            } else if verified_safe_accept == Some(false) {
                verifier_false_events += 1;
            } else {
                no_verifier_label_rows += 1;
            }

            let token_cost = phase_atom_binary_token_cost(&row);
            let has_provider_cost = row
                .get("provider_cost_microusd")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                > 0;
            let has_estimated_cost = token_cost.total_cost_microusd > 0 && !has_provider_cost;
            let has_tokens = token_cost.total_tokens > 0;
            let has_token_or_cost = has_tokens || token_cost.total_cost_microusd > 0;
            provider_cost_events += usize::from(has_provider_cost);
            estimated_cost_events += usize::from(has_estimated_cost);
            token_events += usize::from(has_tokens);
            token_or_cost_events += usize::from(has_token_or_cost);
            if verified_safe_accept.is_some() && has_token_or_cost {
                verifier_bound_token_or_cost_events += 1;
            }

            let bucket_key = format!("{profile_id}::{route_key}");
            let bucket = buckets.entry(bucket_key.clone()).or_insert_with(|| {
                GenericCostEvidenceBucketState {
                    bucket_key,
                    route_key: route_key.clone(),
                    profile_id: profile_id.clone(),
                    ..Default::default()
                }
            });
            bucket.candidate_rows += 1;
            if verified_safe_accept == Some(true) {
                bucket.verifier_true_events += 1;
            } else if verified_safe_accept == Some(false) {
                bucket.verifier_false_events += 1;
            } else {
                bucket.no_verifier_label_events += 1;
            }
            bucket.provider_cost_events += usize::from(has_provider_cost);
            bucket.estimated_cost_events += usize::from(has_estimated_cost);
            bucket.token_events += usize::from(has_tokens);
            bucket.token_or_cost_events += usize::from(has_token_or_cost);
            if verified_safe_accept == Some(true) && has_token_or_cost {
                bucket.verifier_true_token_or_cost_events += 1;
                bucket.verifier_true_cost_events += usize::from(token_cost.total_cost_microusd > 0);
            } else if verified_safe_accept == Some(false) && has_token_or_cost {
                bucket.verifier_false_token_or_cost_events += 1;
                bucket.verifier_false_cost_events +=
                    usize::from(token_cost.total_cost_microusd > 0);
            }
        }
    }

    let mut reports = buckets
        .into_values()
        .map(generic_cost_evidence_bucket_report)
        .collect::<Vec<_>>();
    reports.sort_by(|left, right| {
        right
            .verifier_true_token_or_cost_events
            .cmp(&left.verifier_true_token_or_cost_events)
            .then_with(|| right.token_or_cost_events.cmp(&left.token_or_cost_events))
            .then_with(|| right.candidate_rows.cmp(&left.candidate_rows))
            .then_with(|| left.bucket_key.cmp(&right.bucket_key))
    });
    let compile_ready_bucket_count = reports
        .iter()
        .filter(|bucket| bucket.can_compile_phase_center)
        .count();
    let money_proof_candidate_bucket_count = reports
        .iter()
        .filter(|bucket| bucket.can_measure_money)
        .count();
    let report = GenericCostEvidenceAuditReport {
        report_kind: "generic_real_traffic_cost_evidence_audit_v1",
        mode: "shadow_trace_cost_evidence_audit_only",
        trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        total_rows,
        shadow_request_rows,
        skipped_legacy_profile_events,
        nonlegacy_candidate_rows,
        no_verifier_label_rows,
        verifier_true_events,
        verifier_false_events,
        provider_cost_events,
        estimated_cost_events,
        token_events,
        token_or_cost_events,
        verifier_bound_token_or_cost_events,
        compile_ready_bucket_count,
        money_proof_candidate_bucket_count,
        buckets: reports,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "audit only: ranks non-legacy real-traffic nando_shadow_request buckets by verifier and token/cost evidence; does not compile, promote, serve, local-accept, or use legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("generic_real_traffic_cost_evidence_audit_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  shadow_request_rows: {}", report.shadow_request_rows);
    println!(
        "  nonlegacy_candidate_rows: {}",
        report.nonlegacy_candidate_rows
    );
    println!(
        "  skipped_legacy_profile_events: {}",
        report.skipped_legacy_profile_events
    );
    println!(
        "  verifier_bound_token_or_cost_events: {}",
        report.verifier_bound_token_or_cost_events
    );
    println!(
        "  compile_ready_bucket_count: {}",
        report.compile_ready_bucket_count
    );
    println!(
        "  money_proof_candidate_bucket_count: {}",
        report.money_proof_candidate_bucket_count
    );
    Ok(())
}

pub(crate) fn run_phase_stream_real_traffic_token_cost_enrich_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_TRACE_ENRICHMENT_REPORT));
    let readiness_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                "target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-v1-current5k.report.json",
            )
        });
    let output_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GENERIC_TRACE_ENRICHMENT_DIR));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_generic_real_traffic_trace_paths()
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no real-traffic trace paths provided".to_owned());
    }
    std::fs::create_dir_all(&output_dir).map_err(|error| {
        format!(
            "failed to create token-cost enrichment output dir '{}': {error}",
            output_dir.display()
        )
    })?;

    let readiness_report = read_json_value(&readiness_report_path)?;
    let (readiness_map, readiness_summary) = readiness_token_cost_map(&readiness_report)?;
    let price_config = read_json_file::<ModelPriceConfig>(Path::new(DEFAULT_PRICE_CONFIG))?;
    if readiness_map.is_empty() {
        return Err(format!(
            "readiness report '{}' has no fingerprint token/cost evidence rows",
            readiness_report_path.display()
        ));
    }

    let mut output_files = Vec::new();
    for trace_path in &trace_paths {
        let file_report =
            enrich_trace_token_cost_file(trace_path, &output_dir, &readiness_map, &price_config)?;
        output_files.push(file_report);
    }
    let input_rows = output_files.iter().map(|file| file.input_rows).sum();
    let output_rows = output_files.iter().map(|file| file.output_rows).sum();
    let rows_with_shadow_request = output_files
        .iter()
        .map(|file| file.rows_with_shadow_request)
        .sum();
    let matched_rows = output_files.iter().map(|file| file.matched_rows).sum();
    let rows_enriched_tokens = output_files
        .iter()
        .map(|file| file.rows_enriched_tokens)
        .sum();
    let rows_enriched_cost = output_files
        .iter()
        .map(|file| file.rows_enriched_cost)
        .sum();
    let report = GenericTraceTokenCostEnrichmentReport {
        report_kind: "generic_real_traffic_token_cost_enrichment_v1",
        mode: "trace_enrichment_only",
        readiness_report_path: readiness_report_path.display().to_string(),
        output_dir: output_dir.display().to_string(),
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        readiness_rows: readiness_summary.readiness_rows,
        readiness_rows_with_fingerprint: readiness_summary.readiness_rows_with_fingerprint,
        readiness_rows_with_tokens: readiness_summary.readiness_rows_with_tokens,
        readiness_rows_with_cost: readiness_summary.readiness_rows_with_cost,
        input_rows,
        output_rows,
        rows_with_shadow_request,
        matched_rows,
        rows_enriched_tokens,
        rows_enriched_cost,
        output_files,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "trace enrichment only: copies readiness-report estimated_total_tokens and estimated_total_cost_microusd by request_fingerprint into real-traffic trace rows; no compile, promote, serve, local-accept, lookup, target authority, or legacy backend",
    };
    write_json_file(&report_path, &report)?;
    println!("generic_real_traffic_token_cost_enrichment_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  output_dir: {}", report.output_dir);
    println!("  input_rows: {}", report.input_rows);
    println!(
        "  rows_with_shadow_request: {}",
        report.rows_with_shadow_request
    );
    println!("  matched_rows: {}", report.matched_rows);
    println!("  rows_enriched_tokens: {}", report.rows_enriched_tokens);
    println!("  rows_enriched_cost: {}", report.rows_enriched_cost);
    Ok(())
}

pub(crate) fn run_phase_stream_provider_billing_evidence_join_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BILLING_EVIDENCE_JOIN_REPORT));
    let provider_billing_evidence_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "provider billing evidence JSONL path is required".to_owned())?;
    let output_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BILLING_EVIDENCE_JOIN_DIR));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            default_generic_real_traffic_trace_paths()
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no real-traffic trace paths provided".to_owned());
    }
    std::fs::create_dir_all(&output_dir).map_err(|error| {
        format!(
            "failed to create provider billing join output dir '{}': {error}",
            output_dir.display()
        )
    })?;

    let evidence_map = provider_billing_evidence_map(&provider_billing_evidence_path)?;
    let mut output_files = Vec::new();
    for trace_path in &trace_paths {
        output_files.push(join_provider_billing_evidence_file(
            trace_path,
            &output_dir,
            &evidence_map,
        )?);
    }
    let input_rows = output_files.iter().map(|file| file.input_rows).sum();
    let output_rows = output_files.iter().map(|file| file.output_rows).sum();
    let rows_with_shadow_request = output_files
        .iter()
        .map(|file| file.rows_with_shadow_request)
        .sum();
    let matched_rows = output_files.iter().map(|file| file.matched_rows).sum();
    let rows_enriched_provider_cost = output_files
        .iter()
        .map(|file| file.rows_enriched_provider_cost)
        .sum();
    let rows_enriched_tokens = output_files
        .iter()
        .map(|file| file.rows_enriched_tokens)
        .sum();

    let report = ProviderBillingEvidenceJoinReport {
        report_kind: "provider_billing_evidence_join_v1",
        mode: "trace_enrichment_only",
        provider_billing_evidence_path: provider_billing_evidence_path.display().to_string(),
        output_dir: output_dir.display().to_string(),
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        billing_rows: evidence_map.billing_rows,
        billing_rows_with_match_key: evidence_map.billing_rows_with_match_key,
        billing_rows_with_provider_cost: evidence_map.billing_rows_with_provider_cost,
        duplicate_billing_keys: evidence_map.duplicate_billing_keys,
        input_rows,
        output_rows,
        rows_with_shadow_request,
        matched_rows,
        rows_enriched_provider_cost,
        rows_enriched_tokens,
        output_files,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "provider billing evidence join only: copies externally supplied provider cost/token counters into matching trace rows; does not estimate missing billing, compile, promote, serve, local-accept, claim market money, or use legacy role-binding/nwrb backend",
    };
    write_json_file(&report_path, &report)?;
    println!("provider_billing_evidence_join_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  billing_rows: {}", report.billing_rows);
    println!(
        "  billing_rows_with_provider_cost: {}",
        report.billing_rows_with_provider_cost
    );
    println!(
        "  rows_with_shadow_request: {}",
        report.rows_with_shadow_request
    );
    println!("  matched_rows: {}", report.matched_rows);
    println!(
        "  rows_enriched_provider_cost: {}",
        report.rows_enriched_provider_cost
    );
    println!("  market_money_claim_allowed: false");
    Ok(())
}

pub(crate) fn run_phase_stream_test_output_parse_promotion_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                "target/nando-wave/real-traffic-shadow/test-output-parse-tool-output-state-v1.trace.jsonl",
            )
        });
    let shadow_report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(
            "target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json",
        )
    });
    let candidate_package_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(
            "target/nando-wave/streaming/test-output-parse-tool-output-state-v1.candidate.nwpc",
        )
    });
    let audit_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROMOTION_AUDIT_REPORT));
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin threshold '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO);
    if margin_threshold_micro <= 0 {
        return Err("margin threshold must be > 0".to_owned());
    }
    let price_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PRICE_CONFIG));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let shadow = read_json_value(&shadow_report_path)?;
    let price_config = read_json_file::<ModelPriceConfig>(&price_config_path)?;
    let package_bytes = std::fs::read(&candidate_package_path).map_err(|error| {
        format!(
            "failed to read candidate package '{}': {error}",
            candidate_package_path.display()
        )
    })?;
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes)
        .map_err(|error| format!("phase-center package inspect error: {error:?}"))?;
    let offload_runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &package_bytes,
        PhaseCenterOffloadPolicy::new(margin_threshold_micro)
            .map_err(|error| format!("invalid phase-center policy: {error:?}"))?,
    )
    .map_err(|error| format!("phase-center package load error: {error:?}"))?;

    let rows = read_trace_rows(&trace_path)?;
    let mut parsed_events = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        if let Some(event) = parse_trace_row(index, row) {
            parsed_events.push(event);
        }
    }
    let (train_indices, heldout_indices) = stratified_train_heldout_indices(&parsed_events);
    let label_to_index = label_to_program_index(&parsed_events, &train_indices);
    let exact_cache_hits = exact_cache_hit_flags(&parsed_events);

    let cells = package_info.cells;
    let mut accepted_margins = Vec::new();
    let mut heldout_uncovered_events = 0usize;
    let mut verified_shadow_accepts = 0usize;
    let mut unique_cpu_accepts_over_exact_cache = 0usize;
    let mut exact_cache_hits_in_heldout = 0usize;
    let mut audit_false_accepts = 0usize;
    let mut baseline_input_tokens = 0usize;
    let mut baseline_output_tokens = 0usize;
    let mut baseline_cached_input_tokens = 0usize;
    let mut baseline_cost = 0u64;
    let mut exact_cache_tokens_saved = 0usize;
    let mut exact_cache_cost_saved = 0u64;
    let mut nando_tokens_saved = 0usize;
    let mut nando_cost_saved = 0u64;
    let mut real_token_rows = 0usize;
    let mut estimated_token_rows = 0usize;
    let mut provider_cost_rows = 0usize;
    let mut estimated_cost_rows = 0usize;

    for &event_index in &heldout_indices {
        let event = &parsed_events[event_index];
        let token_cost = event_token_cost(event, &price_config);
        let event_cost = token_cost.total_cost_microusd;
        baseline_input_tokens += token_cost.input_tokens;
        baseline_output_tokens += token_cost.output_tokens;
        baseline_cached_input_tokens += token_cost.cached_input_tokens;
        if token_cost.token_estimate_used {
            estimated_token_rows += 1;
        } else {
            real_token_rows += 1;
        }
        if token_cost.cost_estimate_used {
            estimated_cost_rows += 1;
        } else {
            provider_cost_rows += 1;
        }
        baseline_cost = baseline_cost.saturating_add(event_cost);

        let exact_hit = exact_cache_hits[event_index];
        if exact_hit {
            exact_cache_hits_in_heldout += 1;
            exact_cache_tokens_saved += token_cost.total_tokens;
            exact_cache_cost_saved = exact_cache_cost_saved.saturating_add(event_cost);
        }

        let Some(program_index) = label_to_index.get(&event.label).copied() else {
            heldout_uncovered_events += 1;
            continue;
        };
        let mut event_margins = Vec::new();
        for wrong_label in TestOutputLabel::ALL {
            if wrong_label == event.label {
                continue;
            }
            let correct_vec = event_vector(event, event.label, cells);
            let wrong_vec = event_vector(event, wrong_label, cells);
            let margin = offload_runtime
                .runtime()
                .margin(&PhaseCenterEvalTask {
                    center_index: program_index,
                    correct_vec: correct_vec.into_boxed_slice(),
                    wrong_vec: wrong_vec.into_boxed_slice(),
                })
                .map_err(|error| format!("promotion audit margin error: {error:?}"))?;
            event_margins.push(margin_to_micro(margin)?);
        }
        let min_margin = event_margins.into_iter().min().unwrap_or(0);
        if min_margin >= margin_threshold_micro {
            verified_shadow_accepts += 1;
            accepted_margins.push(min_margin);
            if min_margin <= 0 {
                audit_false_accepts += 1;
            }
            if !exact_hit {
                unique_cpu_accepts_over_exact_cache += 1;
                nando_tokens_saved += token_cost.total_tokens;
                nando_cost_saved = nando_cost_saved.saturating_add(event_cost);
            }
        }
    }
    accepted_margins.sort_unstable();

    let shadow_package_fingerprint =
        json_u64(&shadow, &["candidate_package", "package_fingerprint64"]).unwrap_or_default();
    let shadow_package_bytes =
        json_u64(&shadow, &["candidate_package", "package_bytes"]).unwrap_or_default() as usize;
    let shadow_package_kind =
        json_string(&shadow, &["candidate_package", "package_kind"]).unwrap_or_default();
    let shadow_quarantine_only =
        json_bool(&shadow, &["candidate_package", "quarantine_only"]).unwrap_or(false);
    let shadow_serving_profile_artifact =
        json_bool(&shadow, &["candidate_package", "serving_profile_artifact"]).unwrap_or(true);
    let shadow_promoted = json_bool(&shadow, &["candidate_package", "promoted"]).unwrap_or(true);
    let shadow_local_accept = json_bool(&shadow, &["local_accept_enabled"]).unwrap_or(true);
    let shadow_false_accepts =
        json_u64(&shadow, &["shadow", "false_accepts"]).unwrap_or(usize::MAX as u64) as usize;
    let shadow_wrong_wins =
        json_u64(&shadow, &["shadow", "wrong_wins"]).unwrap_or(usize::MAX as u64) as usize;
    let shadow_heldout_uncovered = json_u64(&shadow, &["trace", "heldout_uncovered_events"])
        .unwrap_or(usize::MAX as u64) as usize;
    let shadow_runtime_margin_parity = json_u64(
        &shadow,
        &["candidate_package", "runtime_margin_parity_mismatches"],
    )
    .unwrap_or(usize::MAX as u64) as usize;
    let shadow_synthetic_events =
        json_u64(&shadow, &["trace", "synthetic_events"]).unwrap_or(usize::MAX as u64) as usize;
    let shadow_generated_default =
        json_bool(&shadow, &["trace", "generated_default_trace_used"]).unwrap_or(true);
    let shadow_verifier_bound =
        json_bool(&shadow, &["candidate", "verifier_bound"]).unwrap_or(false);
    let shadow_verdict = json_string(&shadow, &["verdict"]).unwrap_or_default();
    let shadow_proof_scope = json_string(&shadow, &["proof_scope"]).unwrap_or_default();
    let shadow_metadata_status_pass =
        json_bool(&shadow, &["shadow", "metadata_status_shadow_pass"]).unwrap_or(false);
    let shadow_raw_output_pass =
        json_bool(&shadow, &["shadow", "raw_output_shadow_pass"]).unwrap_or(false);
    let shadow_metadata_status_verified_accepts =
        json_u64(&shadow, &["shadow", "metadata_status_verified_accepts"]).unwrap_or_default()
            as usize;
    let shadow_raw_output_verified_accepts =
        json_u64(&shadow, &["shadow", "raw_output_verified_accepts"]).unwrap_or_default() as usize;

    let baseline_tokens = baseline_input_tokens + baseline_output_tokens;
    let combined_tokens_saved = exact_cache_tokens_saved + nando_tokens_saved;
    let combined_cost_saved = exact_cache_cost_saved.saturating_add(nando_cost_saved);
    let metadata_scope_eligible = shadow_proof_scope == "tool_output_state_metadata_parse"
        && shadow_metadata_status_pass
        && !shadow_raw_output_pass
        && shadow_metadata_status_verified_accepts > 0;
    let raw_output_scope_eligible = shadow_proof_scope == "raw_output_parse"
        && shadow_raw_output_pass
        && shadow_raw_output_verified_accepts > 0;
    let promotion_eligible = shadow_verdict == "ONLINE_PHASE_CENTER_TEST_OUTPUT_PARSE_SHADOW_PASS"
        && (metadata_scope_eligible || raw_output_scope_eligible)
        && !shadow_generated_default
        && shadow_synthetic_events == 0
        && shadow_verifier_bound
        && shadow_false_accepts == 0
        && shadow_wrong_wins == 0
        && shadow_heldout_uncovered == 0
        && shadow_runtime_margin_parity == 0
        && shadow_package_fingerprint == package_info.fingerprint64
        && shadow_package_bytes == package_bytes.len()
        && shadow_package_kind == "quarantine_candidate_package"
        && shadow_quarantine_only
        && !shadow_serving_profile_artifact
        && !shadow_promoted
        && !shadow_local_accept
        && heldout_uncovered_events == 0
        && audit_false_accepts == 0
        && unique_cpu_accepts_over_exact_cache > 0;

    let token_cost_estimate_used = estimated_token_rows > 0 || estimated_cost_rows > 0;
    let billing_evidence_real = real_token_rows == heldout_indices.len()
        && provider_cost_rows == heldout_indices.len()
        && estimated_token_rows == 0
        && estimated_cost_rows == 0;
    let money_estimate_available = baseline_cost > 0;
    let metadata_status_claim_allowed = promotion_eligible
        && metadata_scope_eligible
        && shadow_metadata_status_verified_accepts == verified_shadow_accepts
        && verified_shadow_accepts > 0;
    let raw_output_claim_allowed = promotion_eligible
        && raw_output_scope_eligible
        && shadow_raw_output_verified_accepts == verified_shadow_accepts
        && verified_shadow_accepts > 0;
    let market_money_claim_allowed = false;
    let provider = unique_event_value_or_default(
        &parsed_events,
        &heldout_indices,
        |event| event.provider.as_deref(),
        &price_config.default_provider,
    );
    let model_id = unique_event_value_or_default(
        &parsed_events,
        &heldout_indices,
        |event| event.model_id.as_deref(),
        &price_config.default_model_id,
    );
    let mut report = PhaseStreamPromotionAuditReport {
        report_kind: "online_phase_center_test_output_parse_promotion_audit_v1",
        profile: PROFILE,
        verdict: "ONLINE_PHASE_CENTER_TEST_OUTPUT_PARSE_PROMOTION_REVIEW",
        mode: "offline_promotion_audit_only",
        trace_path: trace_path.display().to_string(),
        shadow_report_path: shadow_report_path.display().to_string(),
        candidate_package_path: candidate_package_path.display().to_string(),
        model_price_config_path: price_config_path.display().to_string(),
        proof_scope: shadow_proof_scope.clone(),
        metadata_status_claim_allowed,
        raw_output_claim_allowed,
        margin_threshold_micro,
        package: PromotionPackageAudit {
            package_kind: shadow_package_kind,
            package_fingerprint64: package_info.fingerprint64,
            package_bytes: package_bytes.len(),
            inspected_cells: package_info.cells,
            inspected_record_count: package_info.record_count,
            inspect_matches_shadow_report: shadow_package_fingerprint == package_info.fingerprint64
                && shadow_package_bytes == package_bytes.len(),
            quarantine_only: shadow_quarantine_only,
            serving_profile_artifact: shadow_serving_profile_artifact,
            promoted: shadow_promoted,
        },
        shadow_gate: PromotionShadowGateAudit {
            shadow_verdict,
            proof_scope: shadow_proof_scope.clone(),
            metadata_status_shadow_pass: shadow_metadata_status_pass,
            raw_output_shadow_pass: shadow_raw_output_pass,
            metadata_status_verified_accepts: shadow_metadata_status_verified_accepts,
            raw_output_verified_accepts: shadow_raw_output_verified_accepts,
            generated_default_trace_used: shadow_generated_default,
            synthetic_events: shadow_synthetic_events,
            verifier_bound: shadow_verifier_bound,
            false_accepts: shadow_false_accepts,
            wrong_wins: shadow_wrong_wins,
            heldout_uncovered_events: shadow_heldout_uncovered,
            runtime_margin_parity_mismatches: shadow_runtime_margin_parity,
            local_accept_enabled: shadow_local_accept,
        },
        evaluation: PromotionEvaluationAudit {
            evaluation_scope: "heldout_only_after_train_window",
            parsed_events: parsed_events.len(),
            train_events: train_indices.len(),
            heldout_events: heldout_indices.len(),
            heldout_uncovered_events,
            exact_cache_hits_in_heldout,
            verified_shadow_accepts,
            unique_cpu_accepts_over_exact_cache,
            audit_false_accepts,
            median_margin_micro: percentile_i64(&accepted_margins, 50),
            p10_margin_micro: percentile_i64(&accepted_margins, 10),
            min_margin_micro: accepted_margins.first().copied().unwrap_or(0),
            projected_nando_calls_saved_milli: per_thousand(
                unique_cpu_accepts_over_exact_cache,
                heldout_indices.len(),
            ),
            projected_combined_calls_saved_milli: per_thousand(
                exact_cache_hits_in_heldout + unique_cpu_accepts_over_exact_cache,
                heldout_indices.len(),
            ),
        },
        token_cost_meter: PromotionTokenCostMeter {
            token_cost_estimate_used,
            token_source: if token_cost_estimate_used {
                "mixed_or_estimated_trace_rows".to_owned()
            } else {
                "real_trace_token_and_provider_cost_rows".to_owned()
            },
            price_config_schema_version: price_config.schema_version,
            provider,
            model_id,
            price_source: price_config.price_source,
            input_cost_microusd_per_1k_tokens: price_config.input_cost_microusd_per_1k_tokens,
            output_cost_microusd_per_1k_tokens: price_config.output_cost_microusd_per_1k_tokens,
            real_token_rows,
            estimated_token_rows,
            provider_cost_rows,
            estimated_cost_rows,
            total_baseline_input_tokens: baseline_input_tokens,
            total_baseline_output_tokens: baseline_output_tokens,
            total_cached_input_tokens: baseline_cached_input_tokens,
            total_baseline_tokens: baseline_tokens,
            total_baseline_cost_microusd: baseline_cost,
            exact_cache_tokens_saved,
            exact_cache_cost_saved_microusd: exact_cache_cost_saved,
            nando_cpu_tokens_saved: nando_tokens_saved,
            nando_cpu_cost_saved_microusd: nando_cost_saved,
            combined_tokens_saved,
            combined_cost_saved_microusd: combined_cost_saved,
            nando_tokens_saved_milli: per_thousand(nando_tokens_saved, baseline_tokens),
            nando_cost_saved_milli: per_thousand_u64(nando_cost_saved, baseline_cost),
            combined_tokens_saved_milli: per_thousand(combined_tokens_saved, baseline_tokens),
            combined_cost_saved_milli: per_thousand_u64(combined_cost_saved, baseline_cost),
        },
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        product_runtime_changed: false,
        serving_runtime_changed: false,
        local_accept_enabled: false,
        promoted: false,
        promotion_eligible,
        billing_evidence_real,
        money_estimate_available,
        market_money_claim_allowed,
        boundary: "offline promotion/economics audit only; quarantine .nwpc remains non-serving and unpromoted; billing evidence may be real or estimated but never authorizes a market money claim in this mode",
    };
    if promotion_eligible {
        report.verdict = "ONLINE_PHASE_CENTER_TEST_OUTPUT_PARSE_PROMOTION_ELIGIBLE_REVIEW";
    }

    write_json_file(&audit_report_path, &report)?;
    println!("online_phase_center_test_output_parse_promotion_audit_v1:");
    println!("  report_path: {}", audit_report_path.display());
    println!("  verdict: {}", report.verdict);
    println!("  promotion_eligible: {}", report.promotion_eligible);
    println!(
        "  unique_cpu_accepts_over_exact_cache: {}",
        report.evaluation.unique_cpu_accepts_over_exact_cache
    );
    println!(
        "  projected_nando_calls_saved_milli: {}",
        report.evaluation.projected_nando_calls_saved_milli
    );
    println!(
        "  nando_cpu_tokens_saved: {}",
        report.token_cost_meter.nando_cpu_tokens_saved
    );
    println!(
        "  nando_cpu_cost_saved_microusd: {}",
        report.token_cost_meter.nando_cpu_cost_saved_microusd
    );
    println!(
        "  market_money_claim_allowed: {}",
        report.market_money_claim_allowed
    );
    println!("  billing_evidence_real: {}", report.billing_evidence_real);
    println!("  proof_scope: {}", report.proof_scope);
    println!(
        "  metadata_status_claim_allowed: {}",
        report.metadata_status_claim_allowed
    );
    println!(
        "  raw_output_claim_allowed: {}",
        report.raw_output_claim_allowed
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    Ok(())
}

fn read_trace_rows(path: &Path) -> Result<Vec<TestOutputTraceRow>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let mut rows = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<TestOutputTraceRow>(trimmed).map_err(|error| {
            format!(
                "failed to parse JSONL row {} in '{}': {error}",
                line_index + 1,
                path.display()
            )
        })?;
        rows.push(row);
    }
    Ok(rows)
}

fn build_discovery_candidate(
    bucket_key: &str,
    all_events: &[ParsedTestOutputEvent],
    indices: &[usize],
    cells: usize,
    price_config: &ModelPriceConfig,
    package_path: &Path,
) -> Result<DiscoveryCandidateReport, String> {
    let events = indices
        .iter()
        .map(|&index| all_events[index].clone())
        .collect::<Vec<_>>();
    let raw_output_events = events
        .iter()
        .filter(|event| event.raw_output_available && !event.metadata_verifier_used)
        .count();
    let metadata_status_events = events
        .iter()
        .filter(|event| event.metadata_verifier_used)
        .count();
    let proof_scope = if raw_output_events > 0 && metadata_status_events == 0 {
        "raw_output_parse"
    } else if metadata_status_events > 0 && raw_output_events == 0 {
        "tool_output_state_metadata_parse"
    } else {
        "mixed_or_unproven_scope"
    };
    let mut label_set = BTreeSet::new();
    for event in &events {
        label_set.insert(event.label);
    }
    let mut candidate_labels = label_set
        .iter()
        .map(|label| label.as_str().to_owned())
        .collect::<Vec<_>>();
    candidate_labels.sort();
    if events.len() < 4 || label_set.len() < 2 || proof_scope == "mixed_or_unproven_scope" {
        return Ok(DiscoveryCandidateReport {
            bucket_key: bucket_key.to_owned(),
            proof_scope: proof_scope.to_owned(),
            package_path: package_path.display().to_string(),
            package_fingerprint64: 0,
            package_bytes: 0,
            events: events.len(),
            train_events: 0,
            heldout_events: 0,
            candidate_labels,
            raw_output_events,
            metadata_status_events,
            false_accepts: 0,
            wrong_wins: 0,
            heldout_uncovered_events: 0,
            runtime_margin_parity_mismatches: 0,
            min_margin_micro: 0,
            median_margin_micro: 0,
            p10_margin_micro: 0,
            exact_cache_hits_in_heldout: 0,
            unique_cpu_accepts_over_exact_cache: 0,
            nando_cpu_tokens_saved: 0,
            nando_cpu_cost_saved_microusd: 0,
            combined_cost_saved_microusd: 0,
            verifier_bound: true,
            quarantine_only: true,
            promoted: false,
            accepted_for_offline_review: false,
            rejection_reason: if events.len() < 4 {
                "bucket_has_fewer_than_4_events".to_owned()
            } else if label_set.len() < 2 {
                "bucket_has_fewer_than_2_labels".to_owned()
            } else {
                "bucket_scope_mixed_or_unproven".to_owned()
            },
        });
    }

    let (train_indices, heldout_indices) = stratified_train_heldout_indices(&events);
    let label_to_index = label_to_program_index(&events, &train_indices);
    let mut compiler = PhaseCenterCompiler::new(cells, label_to_index.len())
        .map_err(|error| format!("phase-center discovery compiler error: {error:?}"))?;
    for &event_index in &train_indices {
        let event = &events[event_index];
        let program_index = label_to_index[&event.label];
        let positive_vec = event_vector(event, event.label, cells);
        compiler
            .add_positive_vector(program_index, &positive_vec)
            .map_err(|error| format!("phase-center discovery positive update error: {error:?}"))?;
        for wrong_label in TestOutputLabel::ALL {
            if wrong_label == event.label {
                continue;
            }
            let wrong_vec = event_vector(event, wrong_label, cells);
            compiler
                .add_negative_vector(program_index, &wrong_vec)
                .map_err(|error| {
                    format!("phase-center discovery negative update error: {error:?}")
                })?;
        }
    }
    let runtime = compiler
        .compile()
        .map_err(|error| format!("phase-center discovery compile error: {error:?}"))?;
    let package_bytes_vec = runtime.to_bytes().map_err(|error| {
        format!("phase-center discovery package serialization error: {error:?}")
    })?;
    write_binary_file(package_path, &package_bytes_vec)?;
    let read_package = std::fs::read(package_path).map_err(|error| {
        format!(
            "failed to read discovery candidate package '{}': {error}",
            package_path.display()
        )
    })?;
    if read_package != package_bytes_vec {
        return Err(format!(
            "discovery candidate package '{}' readback mismatch",
            package_path.display()
        ));
    }
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&read_package)
        .map_err(|error| format!("phase-center discovery package inspect error: {error:?}"))?;
    let offload_runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &read_package,
        PhaseCenterOffloadPolicy::default_conservative(),
    )
    .map_err(|error| format!("phase-center discovery package load error: {error:?}"))?;
    let exact_cache_flags = exact_cache_hit_flags(&events);
    let mut margins = Vec::new();
    let mut wrong_wins = 0usize;
    let mut runtime_margin_parity_mismatches = 0usize;
    let mut heldout_uncovered_events = 0usize;
    let mut exact_cache_hits_in_heldout = 0usize;
    let mut unique_cpu_accepts_over_exact_cache = 0usize;
    let mut nando_cpu_tokens_saved = 0usize;
    let mut nando_cpu_cost_saved_microusd = 0u64;
    let mut combined_cost_saved_microusd = 0u64;
    for &event_index in &heldout_indices {
        let event = &events[event_index];
        let exact_hit = exact_cache_flags[event_index];
        if exact_hit {
            exact_cache_hits_in_heldout += 1;
        }
        let Some(program_index) = label_to_index.get(&event.label).copied() else {
            heldout_uncovered_events += 1;
            continue;
        };
        let correct_vec = event_vector(event, event.label, cells);
        let mut event_wrong_wins = 0usize;
        let mut event_margins = Vec::new();
        for wrong_label in TestOutputLabel::ALL {
            if wrong_label == event.label {
                continue;
            }
            let wrong_vec = event_vector(event, wrong_label, cells);
            let task = PhaseCenterEvalTask {
                center_index: program_index,
                correct_vec: correct_vec.clone().into_boxed_slice(),
                wrong_vec: wrong_vec.into_boxed_slice(),
            };
            let margin = runtime
                .margin(&task)
                .map_err(|error| format!("phase-center discovery margin error: {error:?}"))?;
            let margin_micro = margin_to_micro(margin)?;
            let package_margin = offload_runtime
                .runtime()
                .margin(&task)
                .map_err(|error| format!("phase-center discovery parity error: {error:?}"))?;
            if margin_to_micro(package_margin)? != margin_micro {
                runtime_margin_parity_mismatches += 1;
            }
            if margin_micro <= 0 {
                wrong_wins += 1;
                event_wrong_wins += 1;
            }
            margins.push(margin_micro);
            event_margins.push(margin_micro);
        }
        if event_wrong_wins == 0 && !exact_hit {
            let token_cost = event_token_cost(event, price_config);
            unique_cpu_accepts_over_exact_cache += 1;
            nando_cpu_tokens_saved += token_cost.total_tokens;
            nando_cpu_cost_saved_microusd =
                nando_cpu_cost_saved_microusd.saturating_add(token_cost.total_cost_microusd);
            combined_cost_saved_microusd =
                combined_cost_saved_microusd.saturating_add(token_cost.total_cost_microusd);
        }
    }
    margins.sort_unstable();
    let accepted_for_offline_review = !heldout_indices.is_empty()
        && heldout_uncovered_events == 0
        && wrong_wins == 0
        && runtime_margin_parity_mismatches == 0
        && unique_cpu_accepts_over_exact_cache > 0
        && package_info.record_count == runtime.record_count()
        && package_info.serialized_len == read_package.len();
    let rejection_reason = if accepted_for_offline_review {
        "accepted_for_offline_shadow_review".to_owned()
    } else if heldout_indices.is_empty() {
        "empty_heldout".to_owned()
    } else if heldout_uncovered_events > 0 {
        "heldout_uncovered_events".to_owned()
    } else if wrong_wins > 0 {
        "wrong_wins_detected".to_owned()
    } else if runtime_margin_parity_mismatches > 0 {
        "runtime_margin_parity_mismatches".to_owned()
    } else if unique_cpu_accepts_over_exact_cache == 0 {
        "no_unique_accepts_over_exact_cache".to_owned()
    } else {
        "package_shape_mismatch".to_owned()
    };

    Ok(DiscoveryCandidateReport {
        bucket_key: bucket_key.to_owned(),
        proof_scope: proof_scope.to_owned(),
        package_path: package_path.display().to_string(),
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: read_package.len(),
        events: events.len(),
        train_events: train_indices.len(),
        heldout_events: heldout_indices.len(),
        candidate_labels,
        raw_output_events,
        metadata_status_events,
        false_accepts: wrong_wins,
        wrong_wins,
        heldout_uncovered_events,
        runtime_margin_parity_mismatches,
        min_margin_micro: margins.first().copied().unwrap_or(0),
        median_margin_micro: percentile_i64(&margins, 50),
        p10_margin_micro: percentile_i64(&margins, 10),
        exact_cache_hits_in_heldout,
        unique_cpu_accepts_over_exact_cache,
        nando_cpu_tokens_saved,
        nando_cpu_cost_saved_microusd,
        combined_cost_saved_microusd,
        verifier_bound: true,
        quarantine_only: true,
        promoted: false,
        accepted_for_offline_review,
        rejection_reason,
    })
}

fn compile_online_bucket(
    bucket_key: &str,
    all_events: &[ParsedTestOutputEvent],
    event_indices: &[usize],
    cells: usize,
    margin_threshold_micro: i64,
    package_path: &Path,
    compiled_after_global_event_index: usize,
) -> Result<OnlineCompiledBucket, String> {
    let mut label_to_index = BTreeMap::new();
    for &event_index in event_indices {
        let event = &all_events[event_index];
        let next_index = label_to_index.len();
        label_to_index.entry(event.label).or_insert(next_index);
    }
    if label_to_index.len() < 2 {
        return Err(format!(
            "online bucket '{bucket_key}' cannot compile with fewer than 2 labels"
        ));
    }
    let mut compiler = PhaseCenterCompiler::new(cells, label_to_index.len())
        .map_err(|error| format!("online phase-center compiler error: {error:?}"))?;
    for &event_index in event_indices {
        let event = &all_events[event_index];
        let program_index = label_to_index[&event.label];
        let positive_vec = event_vector(event, event.label, cells);
        compiler
            .add_positive_vector(program_index, &positive_vec)
            .map_err(|error| format!("online positive update error: {error:?}"))?;
        for wrong_label in TestOutputLabel::ALL {
            if wrong_label == event.label {
                continue;
            }
            let wrong_vec = event_vector(event, wrong_label, cells);
            compiler
                .add_negative_vector(program_index, &wrong_vec)
                .map_err(|error| format!("online negative update error: {error:?}"))?;
        }
    }
    let reference_runtime = compiler
        .compile()
        .map_err(|error| format!("online phase-center compile error: {error:?}"))?;
    let package_bytes = reference_runtime
        .to_bytes()
        .map_err(|error| format!("online package serialization error: {error:?}"))?;
    write_binary_file(package_path, &package_bytes)?;
    let read_package = std::fs::read(package_path).map_err(|error| {
        format!(
            "failed to read online discovery package '{}': {error}",
            package_path.display()
        )
    })?;
    if read_package != package_bytes {
        return Err(format!(
            "online discovery package '{}' readback mismatch",
            package_path.display()
        ));
    }
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&read_package)
        .map_err(|error| format!("online package inspect error: {error:?}"))?;
    let runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &read_package,
        PhaseCenterOffloadPolicy::new(margin_threshold_micro)
            .map_err(|error| format!("invalid online margin policy: {error:?}"))?,
    )
    .map_err(|error| format!("online package load error: {error:?}"))?;
    if runtime.runtime().record_count() != reference_runtime.record_count() {
        return Err("online package record count mismatch".to_owned());
    }

    let mut candidate_labels = label_to_index
        .keys()
        .map(|label| label.as_str().to_owned())
        .collect::<Vec<_>>();
    candidate_labels.sort();
    Ok(OnlineCompiledBucket {
        package_path: package_path.display().to_string(),
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: read_package.len(),
        compiled_after_global_event_index,
        precompile_events: event_indices.len(),
        candidate_labels,
        label_to_index,
        reference_runtime,
        runtime,
    })
}

fn score_online_shadow_event(
    state: &mut OnlineDiscoveryBucketState,
    compiled: &OnlineCompiledBucket,
    event: &ParsedTestOutputEvent,
    exact_hit: bool,
    price_config: &ModelPriceConfig,
    cells: usize,
    margin_threshold_micro: i64,
) -> Result<(), String> {
    state.shadow_events += 1;
    if exact_hit {
        state.exact_cache_hits_in_shadow += 1;
    }
    let Some(program_index) = compiled.label_to_index.get(&event.label).copied() else {
        state.shadow_uncovered_events += 1;
        return Ok(());
    };
    let correct_vec = event_vector(event, event.label, cells);
    let mut event_wrong_wins = 0usize;
    let mut event_min_margin = i64::MAX;
    for wrong_label in TestOutputLabel::ALL {
        if wrong_label == event.label {
            continue;
        }
        let wrong_vec = event_vector(event, wrong_label, cells);
        let task = PhaseCenterEvalTask {
            center_index: program_index,
            correct_vec: correct_vec.clone().into_boxed_slice(),
            wrong_vec: wrong_vec.into_boxed_slice(),
        };
        let reference_margin = compiled
            .reference_runtime
            .margin(&task)
            .map_err(|error| format!("online reference margin error: {error:?}"))?;
        let package_margin = compiled
            .runtime
            .runtime()
            .margin(&task)
            .map_err(|error| format!("online package margin error: {error:?}"))?;
        let reference_margin_micro = margin_to_micro(reference_margin)?;
        if margin_to_micro(package_margin)? != reference_margin_micro {
            state.runtime_margin_parity_mismatches += 1;
        }
        if reference_margin_micro <= 0 {
            state.wrong_wins += 1;
            event_wrong_wins += 1;
        }
        event_min_margin = event_min_margin.min(reference_margin_micro);
        state.margins.push(reference_margin_micro);
    }
    if event_wrong_wins > 0 {
        state.false_accepts += 1;
    }
    if event_wrong_wins == 0 && event_min_margin >= margin_threshold_micro {
        state.shadow_accepts += 1;
        if !exact_hit {
            let token_cost = event_token_cost(event, price_config);
            state.unique_cpu_accepts_over_exact_cache += 1;
            state.nando_cpu_tokens_saved += token_cost.total_tokens;
            state.nando_cpu_cost_saved_microusd = state
                .nando_cpu_cost_saved_microusd
                .saturating_add(token_cost.total_cost_microusd);
        }
    }
    Ok(())
}

fn online_bucket_report(state: &OnlineDiscoveryBucketState) -> OnlineDiscoveryBucketReport {
    let mut margins = state.margins.clone();
    margins.sort_unstable();
    let accepted_for_online_shadow_review = state.compiled.is_some()
        && state.shadow_events > 0
        && state.shadow_uncovered_events == 0
        && state.false_accepts == 0
        && state.wrong_wins == 0
        && state.runtime_margin_parity_mismatches == 0
        && state.unique_cpu_accepts_over_exact_cache > 0;
    let rejection_reason = if accepted_for_online_shadow_review {
        "accepted_for_online_shadow_review".to_owned()
    } else if state.compiled.is_none() {
        "bucket_never_compiled".to_owned()
    } else if state.shadow_events == 0 {
        "no_future_shadow_events_after_compile".to_owned()
    } else if state.shadow_uncovered_events > 0 {
        "shadow_uncovered_events".to_owned()
    } else if state.false_accepts > 0 || state.wrong_wins > 0 {
        "wrong_wins_detected".to_owned()
    } else if state.runtime_margin_parity_mismatches > 0 {
        "runtime_margin_parity_mismatches".to_owned()
    } else if state.unique_cpu_accepts_over_exact_cache == 0 {
        "no_unique_accepts_over_exact_cache".to_owned()
    } else {
        "unknown_rejection".to_owned()
    };
    let (
        package_path,
        package_fingerprint64,
        package_bytes,
        precompile_events,
        compiled_after_global_event_index,
        candidate_labels,
    ) = state.compiled.as_ref().map_or_else(
        || {
            (
                String::new(),
                0,
                0,
                state.event_indices.len(),
                None,
                Vec::new(),
            )
        },
        |compiled| {
            (
                compiled.package_path.clone(),
                compiled.package_fingerprint64,
                compiled.package_bytes,
                compiled.precompile_events,
                Some(compiled.compiled_after_global_event_index),
                compiled.candidate_labels.clone(),
            )
        },
    );
    OnlineDiscoveryBucketReport {
        bucket_key: state.bucket_key.clone(),
        proof_scope: state.proof_scope.clone(),
        package_path,
        package_fingerprint64,
        package_bytes,
        events_seen: state.event_indices.len() + state.shadow_events,
        precompile_events,
        compiled_after_global_event_index,
        candidate_labels,
        raw_output_events: state.raw_output_events,
        metadata_status_events: state.metadata_status_events,
        shadow_events: state.shadow_events,
        shadow_accepts: state.shadow_accepts,
        false_accepts: state.false_accepts,
        wrong_wins: state.wrong_wins,
        shadow_uncovered_events: state.shadow_uncovered_events,
        runtime_margin_parity_mismatches: state.runtime_margin_parity_mismatches,
        min_margin_micro: margins.first().copied().unwrap_or(0),
        median_margin_micro: percentile_i64(&margins, 50),
        p10_margin_micro: percentile_i64(&margins, 10),
        exact_cache_hits_in_shadow: state.exact_cache_hits_in_shadow,
        unique_cpu_accepts_over_exact_cache: state.unique_cpu_accepts_over_exact_cache,
        nando_cpu_tokens_saved: state.nando_cpu_tokens_saved,
        nando_cpu_cost_saved_microusd: state.nando_cpu_cost_saved_microusd,
        verifier_bound: true,
        quarantine_only: true,
        promoted: false,
        accepted_for_online_shadow_review,
        rejection_reason,
    }
}

fn parse_generic_real_traffic_event(
    row: &serde_json::Value,
    fallback_index: usize,
) -> GenericParseResult {
    let Some(request) = row
        .get("nando_shadow_request")
        .and_then(serde_json::Value::as_object)
    else {
        return GenericParseResult::NoShadowRequest;
    };
    let Some(verified_safe_accept) = row
        .get("verified_safe_accept")
        .and_then(serde_json::Value::as_bool)
    else {
        return GenericParseResult::NoVerifierLabel;
    };

    let route_key = json_field_string(request.get("route_key")).unwrap_or_else(|| "unknown".into());
    let profile_id =
        json_field_string(request.get("profile_id")).unwrap_or_else(|| route_key.clone());
    if is_legacy_profile_name(&route_key) || is_legacy_profile_name(&profile_id) {
        return GenericParseResult::LegacyProfile;
    }

    let request_fingerprint = row
        .get("request_fingerprint")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| json_field_string(request.get("exact_cache_key")))
        .unwrap_or_else(|| format!("generic-event-{fallback_index:08}"));
    let exact_cache_key = row
        .get("exact_cache_key")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| json_field_string(request.get("exact_cache_key")))
        .unwrap_or_else(|| request_fingerprint.clone());
    let traffic_source = row
        .get("traffic_source")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown_traffic_source")
        .to_owned();
    let verification_source = row
        .get("verification_source")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown_verification_source")
        .to_owned();
    let active_fringe = request
        .get("active_fringe")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let center_id = item.get("center_id")?.as_u64()?;
                    let strength = item
                        .get("strength")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(1);
                    Some((center_id, strength))
                })
                .take(96)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let slot_summary = request
        .get("slots")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(16)
                .map(generic_slot_atom)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let tool_call_fingerprint_count = row
        .get("tool_call_fingerprints")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    let estimated_total_cost_microusd = row
        .get("estimated_total_cost_microusd")
        .and_then(serde_json::Value::as_u64)
        .or_else(|| {
            let estimated_input = row
                .get("estimated_input_cost_microusd")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let estimated_output = row
                .get("estimated_output_cost_microusd")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let estimated_total = estimated_input.saturating_add(estimated_output);
            (estimated_total > 0).then_some(estimated_total)
        });
    GenericParseResult::Event(Box::new(GenericRealTrafficEvent {
        route_key,
        profile_id,
        traffic_source,
        verification_source,
        request_fingerprint,
        exact_cache_key,
        explicit_provider_cache_hit: row
            .get("provider_cache_hit")
            .and_then(serde_json::Value::as_bool),
        verified_safe_accept,
        expect_local_operator: request
            .get("expect_local_operator")
            .and_then(serde_json::Value::as_bool),
        active_fringe,
        slot_summary,
        tool_call_fingerprint_count,
        input_tokens: json_usize(row.get("input_tokens")),
        output_tokens: json_usize(row.get("output_tokens")),
        cached_input_tokens: json_usize(row.get("cached_input_tokens")),
        estimated_total_tokens: json_usize(row.get("estimated_total_tokens")),
        provider_cost_microusd: row
            .get("provider_cost_microusd")
            .and_then(serde_json::Value::as_u64),
        estimated_total_cost_microusd,
    }))
}

fn compile_generic_bucket(
    bucket_key: &str,
    all_events: &[GenericRealTrafficEvent],
    event_indices: &[usize],
    cells: usize,
    margin_threshold_micro: i64,
    package_path: &Path,
    compiled_after_global_event_index: usize,
) -> Result<GenericCompiledBucket, String> {
    let (true_count, false_count) = generic_label_counts_for_indices(all_events, event_indices);
    if true_count == 0 || false_count == 0 {
        return Err(format!(
            "generic bucket '{bucket_key}' cannot compile without true and false verifier labels"
        ));
    }
    let mut compiler = PhaseCenterCompiler::new(cells, 1)
        .map_err(|error| format!("generic online compiler error: {error:?}"))?;
    for &event_index in event_indices {
        let event = &all_events[event_index];
        let vector = generic_event_vector(event, cells);
        if event.verified_safe_accept {
            compiler
                .add_positive_vector(0, &vector)
                .map_err(|error| format!("generic positive update error: {error:?}"))?;
        } else {
            compiler
                .add_negative_vector(0, &vector)
                .map_err(|error| format!("generic negative update error: {error:?}"))?;
        }
    }
    let reference_runtime = compiler
        .compile()
        .map_err(|error| format!("generic online compile error: {error:?}"))?;
    let package_bytes = reference_runtime
        .to_bytes()
        .map_err(|error| format!("generic package serialization error: {error:?}"))?;
    write_binary_file(package_path, &package_bytes)?;
    let read_package = std::fs::read(package_path).map_err(|error| {
        format!(
            "failed to read generic online package '{}': {error}",
            package_path.display()
        )
    })?;
    if read_package != package_bytes {
        return Err(format!(
            "generic online package '{}' readback mismatch",
            package_path.display()
        ));
    }
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&read_package)
        .map_err(|error| format!("generic package inspect error: {error:?}"))?;
    let runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &read_package,
        PhaseCenterOffloadPolicy::new(margin_threshold_micro)
            .map_err(|error| format!("invalid generic margin policy: {error:?}"))?,
    )
    .map_err(|error| format!("generic package load error: {error:?}"))?;

    Ok(GenericCompiledBucket {
        package_path: package_path.display().to_string(),
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: read_package.len(),
        compiled_after_global_event_index,
        precompile_events: event_indices.len(),
        reference_runtime,
        runtime,
    })
}

fn score_generic_shadow_event(
    state: &mut GenericOnlineBucketState,
    compiled: &GenericCompiledBucket,
    event: &GenericRealTrafficEvent,
    exact_hit: bool,
    cells: usize,
    margin_threshold_micro: i64,
) -> Result<(), String> {
    state.shadow_events += 1;
    if event.verified_safe_accept {
        state.shadow_safe_events += 1;
    }
    if exact_hit {
        state.exact_cache_hits_in_shadow += 1;
    }
    let (margin_micro, parity_mismatch) = generic_event_margin_micro(compiled, event, cells)?;
    if parity_mismatch {
        state.runtime_margin_parity_mismatches += 1;
    }
    state.margins.push(margin_micro);
    let would_accept = margin_micro >= margin_threshold_micro;
    if would_accept && event.verified_safe_accept {
        state.shadow_accepts += 1;
        if !exact_hit {
            let token_cost = generic_event_token_cost(event);
            if token_cost.evidence_missing {
                state.token_cost_evidence_missing_events += 1;
            }
            if token_cost.token_evidence_missing {
                state.token_evidence_missing_events += 1;
            }
            if token_cost.cost_evidence_missing {
                state.cost_evidence_missing_events += 1;
            }
            state.unique_cpu_accepts_over_exact_cache += 1;
            state.nando_cpu_tokens_saved += token_cost.total_tokens;
            state.nando_cpu_cost_saved_microusd = state
                .nando_cpu_cost_saved_microusd
                .saturating_add(token_cost.total_cost_microusd);
            state.unique_accepts.insert(
                event.request_fingerprint.clone(),
                GenericAcceptedEventReport {
                    request_fingerprint: event.request_fingerprint.clone(),
                    total_tokens: token_cost.total_tokens,
                    total_cost_microusd: token_cost.total_cost_microusd,
                    token_evidence_missing: token_cost.token_evidence_missing,
                    cost_evidence_missing: token_cost.cost_evidence_missing,
                },
            );
        }
    } else if would_accept && !event.verified_safe_accept {
        state.false_accepts += 1;
    } else if !would_accept && event.verified_safe_accept {
        state.missed_safe_accepts += 1;
    }
    Ok(())
}

fn generic_event_margin_micro(
    compiled: &GenericCompiledBucket,
    event: &GenericRealTrafficEvent,
    cells: usize,
) -> Result<(i64, bool), String> {
    let vector = generic_event_vector(event, cells);
    let zero = vec![nando_core::PhaseCenterCell::default(); cells];
    let task = PhaseCenterEvalTask {
        center_index: 0,
        correct_vec: vector.into_boxed_slice(),
        wrong_vec: zero.into_boxed_slice(),
    };
    let reference_margin = compiled
        .reference_runtime
        .margin(&task)
        .map_err(|error| format!("generic reference margin error: {error:?}"))?;
    let package_margin = compiled
        .runtime
        .runtime()
        .margin(&task)
        .map_err(|error| format!("generic package margin error: {error:?}"))?;
    let margin_micro = margin_to_micro(reference_margin)?;
    let package_margin_micro = margin_to_micro(package_margin)?;
    Ok((margin_micro, package_margin_micro != margin_micro))
}

fn generic_bucket_report(state: &GenericOnlineBucketState) -> GenericOnlineBucketReport {
    let mut margins = state.margins.clone();
    margins.sort_unstable();
    let accepted_for_online_shadow_review = state.compiled.is_some()
        && state.shadow_events > 0
        && state.shadow_safe_events > 0
        && state.false_accepts == 0
        && state.runtime_margin_parity_mismatches == 0
        && state.unique_cpu_accepts_over_exact_cache > 0;
    let rejection_reason = if accepted_for_online_shadow_review {
        "accepted_for_online_shadow_review".to_owned()
    } else if state.compiled.is_none() {
        "bucket_never_compiled".to_owned()
    } else if state.shadow_events == 0 {
        "no_future_shadow_events_after_compile".to_owned()
    } else if state.shadow_safe_events == 0 {
        "no_future_verified_safe_events".to_owned()
    } else if state.false_accepts > 0 {
        "false_accepts_detected".to_owned()
    } else if state.runtime_margin_parity_mismatches > 0 {
        "runtime_margin_parity_mismatches".to_owned()
    } else if state.unique_cpu_accepts_over_exact_cache == 0 {
        "no_unique_accepts_over_exact_cache".to_owned()
    } else {
        "unknown_rejection".to_owned()
    };
    let (
        package_path,
        package_fingerprint64,
        package_bytes,
        precompile_events,
        compiled_after_global_event_index,
    ) = state.compiled.as_ref().map_or_else(
        || (String::new(), 0, 0, state.event_indices.len(), None),
        |compiled| {
            (
                compiled.package_path.clone(),
                compiled.package_fingerprint64,
                compiled.package_bytes,
                compiled.precompile_events,
                Some(compiled.compiled_after_global_event_index),
            )
        },
    );
    GenericOnlineBucketReport {
        bucket_key: state.bucket_key.clone(),
        route_key: state.route_key.clone(),
        profile_id: state.profile_id.clone(),
        package_path,
        package_fingerprint64,
        package_bytes,
        events_seen: state.event_indices.len() + state.shadow_events,
        precompile_events,
        compiled_after_global_event_index,
        verifier_true_events: state.verifier_true_events,
        verifier_false_events: state.verifier_false_events,
        shadow_events: state.shadow_events,
        shadow_safe_events: state.shadow_safe_events,
        shadow_accepts: state.shadow_accepts,
        false_accepts: state.false_accepts,
        missed_safe_accepts: state.missed_safe_accepts,
        runtime_margin_parity_mismatches: state.runtime_margin_parity_mismatches,
        min_margin_micro: margins.first().copied().unwrap_or(0),
        median_margin_micro: percentile_i64(&margins, 50),
        p10_margin_micro: percentile_i64(&margins, 10),
        exact_cache_hits_in_shadow: state.exact_cache_hits_in_shadow,
        unique_cpu_accepts_over_exact_cache: state.unique_cpu_accepts_over_exact_cache,
        nando_cpu_tokens_saved: state.nando_cpu_tokens_saved,
        nando_cpu_cost_saved_microusd: state.nando_cpu_cost_saved_microusd,
        unique_accepts: state.unique_accepts.values().cloned().collect(),
        token_cost_evidence_missing_events: state.token_cost_evidence_missing_events,
        token_evidence_missing_events: state.token_evidence_missing_events,
        cost_evidence_missing_events: state.cost_evidence_missing_events,
        verifier_bound: true,
        quarantine_only: true,
        promoted: false,
        accepted_for_online_shadow_review,
        rejection_reason,
    }
}

fn generic_calibrated_bucket_report(
    state: &GenericCalibratedBucketState,
) -> GenericCalibratedSplitBucketReport {
    let mut bucket = generic_bucket_report(&state.state);
    if state.calibration_events == 0 {
        bucket.accepted_for_online_shadow_review = false;
        bucket.rejection_reason = "no_calibration_events".to_owned();
    } else if state.calibration_accepts == 0 {
        bucket.accepted_for_online_shadow_review = false;
        bucket.rejection_reason = "no_calibration_accepts".to_owned();
    } else if state.calibration_false_accepts > 0 {
        bucket.accepted_for_online_shadow_review = false;
        bucket.rejection_reason = "calibration_false_accepts_detected".to_owned();
    }
    GenericCalibratedSplitBucketReport {
        bucket,
        calibrated_margin_threshold_micro: state.calibrated_margin_threshold_micro,
        calibration_events: state.calibration_events,
        calibration_safe_events: state.calibration_safe_events,
        calibration_false_events: state.calibration_false_events,
        calibration_accepts: state.calibration_accepts,
        calibration_false_accepts: state.calibration_false_accepts,
        calibration_max_false_margin_micro: state.calibration_max_false_margin_micro,
        calibration_min_safe_margin_micro: state.calibration_min_safe_margin_micro,
        calibration_threshold_source: state.calibration_threshold_source,
    }
}

fn generic_cost_evidence_bucket_report(
    state: GenericCostEvidenceBucketState,
) -> GenericCostEvidenceBucketReport {
    let can_compile_phase_center =
        state.verifier_true_events > 0 && state.verifier_false_events > 0;
    let can_measure_money = can_compile_phase_center && state.verifier_true_cost_events > 0;
    let recommended_next_action = if can_measure_money {
        "run_phase_stream_real_traffic_online_discovery_v1"
    } else if !can_compile_phase_center && state.verifier_true_events == 0 {
        "add_verified_safe_positive_evidence"
    } else if !can_compile_phase_center && state.verifier_false_events == 0 {
        "add_verified_safe_negative_evidence"
    } else if state.token_events == 0 {
        "enrich_trace_with_token_cost_evidence"
    } else if state.verifier_true_cost_events == 0 {
        "attach_provider_or_estimated_cost_evidence_to_verified_safe_rows"
    } else {
        "watch_or_collect_more_trace"
    };
    GenericCostEvidenceBucketReport {
        bucket_key: state.bucket_key,
        route_key: state.route_key,
        profile_id: state.profile_id,
        candidate_rows: state.candidate_rows,
        verifier_true_events: state.verifier_true_events,
        verifier_false_events: state.verifier_false_events,
        no_verifier_label_events: state.no_verifier_label_events,
        provider_cost_events: state.provider_cost_events,
        estimated_cost_events: state.estimated_cost_events,
        token_events: state.token_events,
        token_or_cost_events: state.token_or_cost_events,
        verifier_true_token_or_cost_events: state.verifier_true_token_or_cost_events,
        verifier_false_token_or_cost_events: state.verifier_false_token_or_cost_events,
        verifier_true_cost_events: state.verifier_true_cost_events,
        verifier_false_cost_events: state.verifier_false_cost_events,
        can_compile_phase_center,
        can_measure_money,
        recommended_next_action,
    }
}

fn readiness_token_cost_map(
    report: &serde_json::Value,
) -> Result<
    (
        BTreeMap<String, ReadinessTokenCostEvidence>,
        ReadinessTokenCostSummary,
    ),
    String,
> {
    let Some(rows) = report.get("rows").and_then(serde_json::Value::as_array) else {
        return Err("readiness report missing rows array".to_owned());
    };
    let mut summary = ReadinessTokenCostSummary {
        readiness_rows: rows.len(),
        ..Default::default()
    };
    let mut map = BTreeMap::new();
    for row in rows {
        let Some(fingerprint) = row
            .get("request_fingerprint")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        summary.readiness_rows_with_fingerprint += 1;
        let estimated_total_tokens = json_usize(row.get("estimated_total_tokens"));
        let estimated_total_cost_microusd = row
            .get("estimated_total_cost_microusd")
            .and_then(serde_json::Value::as_u64);
        summary.readiness_rows_with_tokens += usize::from(estimated_total_tokens.unwrap_or(0) > 0);
        summary.readiness_rows_with_cost +=
            usize::from(estimated_total_cost_microusd.unwrap_or(0) > 0);
        let token_cost_estimate_used = row
            .get("token_cost_estimate_used")
            .and_then(serde_json::Value::as_bool);
        map.insert(
            fingerprint.to_owned(),
            ReadinessTokenCostEvidence {
                estimated_total_tokens,
                estimated_total_cost_microusd,
                token_cost_estimate_used,
            },
        );
    }
    Ok((map, summary))
}

fn enrich_trace_token_cost_file(
    trace_path: &Path,
    output_dir: &Path,
    readiness_map: &BTreeMap<String, ReadinessTokenCostEvidence>,
    price_config: &ModelPriceConfig,
) -> Result<GenericTraceTokenCostEnrichmentFileReport, String> {
    let text = std::fs::read_to_string(trace_path).map_err(|error| {
        format!(
            "failed to read real-traffic trace '{}': {error}",
            trace_path.display()
        )
    })?;
    let file_name = trace_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("real-traffic.trace.jsonl");
    let output_path = output_dir.join(format!("{file_name}.token-cost-enriched.jsonl"));
    let mut output = String::new();
    let mut input_rows = 0usize;
    let mut output_rows = 0usize;
    let mut rows_with_shadow_request = 0usize;
    let mut matched_rows = 0usize;
    let mut rows_enriched_tokens = 0usize;
    let mut rows_enriched_cost = 0usize;

    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        input_rows += 1;
        let mut row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse real-traffic trace '{}' line {}: {error}",
                trace_path.display(),
                line_index + 1
            )
        })?;
        if row
            .get("nando_shadow_request")
            .and_then(serde_json::Value::as_object)
            .is_some()
        {
            rows_with_shadow_request += 1;
        }
        let fingerprint = row
            .get("request_fingerprint")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        if let Some(evidence) = fingerprint
            .as_deref()
            .and_then(|fingerprint| readiness_map.get(fingerprint))
        {
            matched_rows += 1;
            if let Some(tokens) = evidence.estimated_total_tokens {
                let missing_input = json_usize(row.get("input_tokens")).unwrap_or(0) == 0;
                let missing_output = json_usize(row.get("output_tokens")).unwrap_or(0) == 0;
                let missing_total = json_usize(row.get("estimated_total_tokens")).unwrap_or(0) == 0;
                if tokens > 0 && missing_input && missing_output && missing_total {
                    set_json_usize(&mut row, "estimated_total_tokens", tokens)?;
                    set_nested_token_cost_usize(&mut row, "total_tokens", tokens)?;
                    set_nested_token_cost_bool(&mut row, "token_evidence_missing", false)?;
                    rows_enriched_tokens += 1;
                }
            }
            if let Some(cost) = evidence.estimated_total_cost_microusd {
                let missing_provider = row
                    .get("provider_cost_microusd")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
                    == 0;
                let missing_estimated = row
                    .get("estimated_total_cost_microusd")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
                    == 0;
                if cost > 0 && missing_provider && missing_estimated {
                    set_json_u64(&mut row, "estimated_total_cost_microusd", cost)?;
                    set_nested_token_cost_u64(&mut row, "total_cost_microusd", cost)?;
                    set_nested_token_cost_bool(&mut row, "cost_evidence_missing", false)?;
                    rows_enriched_cost += 1;
                }
            }
            if let Some(estimate_used) = evidence.token_cost_estimate_used
                && (row.get("token_cost_estimate_used").is_none()
                    || row.get("token_cost_estimate_used") == Some(&serde_json::Value::Null))
            {
                set_json_bool(&mut row, "token_cost_estimate_used", estimate_used)?;
            }
            if row.get("token_cost_evidence_source").is_none() {
                set_json_string_field(
                    &mut row,
                    "token_cost_evidence_source",
                    "route_gap_payload_readiness_report",
                )?;
            }
        }
        let token_cost = phase_atom_binary_token_cost(&row);
        if token_cost.total_tokens > 0 && token_cost.total_cost_microusd == 0 {
            let estimated_cost = token_floor_cost_microusd(token_cost.total_tokens, price_config);
            if estimated_cost > 0 {
                if row
                    .get("estimated_total_tokens")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
                    == 0
                {
                    set_json_usize(&mut row, "estimated_total_tokens", token_cost.total_tokens)?;
                }
                if row
                    .get("estimated_total_cost_microusd")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
                    == 0
                {
                    set_json_u64(&mut row, "estimated_total_cost_microusd", estimated_cost)?;
                    rows_enriched_cost += 1;
                }
                set_nested_token_cost_u64(&mut row, "total_cost_microusd", estimated_cost)?;
                set_nested_token_cost_bool(&mut row, "cost_evidence_missing", false)?;
                set_json_bool(&mut row, "token_cost_estimate_used", true)?;
                set_nested_token_cost_bool(&mut row, "token_cost_estimate_used", true)?;
                if row.get("token_cost_evidence_source").is_none() {
                    set_json_string_field(
                        &mut row,
                        "token_cost_evidence_source",
                        "token_cost_total_tokens_model_price_config",
                    )?;
                }
            }
        }
        output.push_str(
            &serde_json::to_string(&row)
                .map_err(|error| format!("failed to serialize enriched row: {error}"))?,
        );
        output.push('\n');
        output_rows += 1;
    }
    std::fs::write(&output_path, output).map_err(|error| {
        format!(
            "failed to write enriched trace '{}': {error}",
            output_path.display()
        )
    })?;
    Ok(GenericTraceTokenCostEnrichmentFileReport {
        input_path: trace_path.display().to_string(),
        output_path: output_path.display().to_string(),
        input_rows,
        output_rows,
        rows_with_shadow_request,
        matched_rows,
        rows_enriched_tokens,
        rows_enriched_cost,
    })
}

fn provider_billing_evidence_map(
    provider_billing_evidence_path: &Path,
) -> Result<ProviderBillingEvidenceMap, String> {
    let text = std::fs::read_to_string(provider_billing_evidence_path).map_err(|error| {
        format!(
            "failed to read provider billing evidence '{}': {error}",
            provider_billing_evidence_path.display()
        )
    })?;
    let mut map = ProviderBillingEvidenceMap::default();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        map.billing_rows += 1;
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse provider billing evidence '{}' line {}: {error}",
                provider_billing_evidence_path.display(),
                line_index + 1
            )
        })?;
        let billing_evidence_id = json_string(&row, &["billing_evidence_id"])
            .or_else(|| json_string(&row, &["event_id"]))
            .or_else(|| json_string(&row, &["id"]))
            .unwrap_or_else(|| format!("billing-row-{:08}", line_index + 1));
        let billing_source = json_string(&row, &["billing_source"])
            .or_else(|| json_string(&row, &["source"]))
            .unwrap_or_else(|| "provider_billing_jsonl".to_owned());
        let input_tokens = json_usize(row.get("input_tokens"));
        let output_tokens = json_usize(row.get("output_tokens"));
        let cached_input_tokens = json_usize(row.get("cached_input_tokens"));
        let total_tokens = json_usize(row.get("total_tokens")).or_else(|| {
            let total = input_tokens.unwrap_or(0)
                + output_tokens.unwrap_or(0)
                + cached_input_tokens.unwrap_or(0);
            (total > 0).then_some(total)
        });
        let provider_cost_microusd = row
            .get("provider_cost_microusd")
            .or_else(|| row.get("total_cost_microusd"))
            .or_else(|| row.get("cost_microusd"))
            .and_then(serde_json::Value::as_u64)
            .filter(|cost| *cost > 0);
        let evidence = ProviderBillingEvidence {
            billing_evidence_id,
            billing_source,
            provider: json_string(&row, &["provider"]),
            model_id: json_string(&row, &["model_id"]),
            input_tokens,
            output_tokens,
            cached_input_tokens,
            total_tokens,
            provider_cost_microusd,
        };
        if provider_cost_microusd.is_some() {
            map.billing_rows_with_provider_cost += 1;
        }
        let keys = provider_billing_match_keys(&row);
        if !keys.is_empty() {
            map.billing_rows_with_match_key += 1;
        }
        for key in keys {
            if map.by_key.insert(key, evidence.clone()).is_some() {
                map.duplicate_billing_keys += 1;
            }
        }
    }
    Ok(map)
}

fn provider_billing_match_keys(row: &serde_json::Value) -> Vec<String> {
    let mut keys = BTreeSet::new();
    for field in ["request_fingerprint", "exact_cache_key", "trace_id"] {
        if let Some(value) = json_string(row, &[field]).filter(|value| !value.is_empty()) {
            keys.insert(format!("{field}:{value}"));
        }
    }
    keys.into_iter().collect()
}

fn join_provider_billing_evidence_file(
    trace_path: &Path,
    output_dir: &Path,
    evidence_map: &ProviderBillingEvidenceMap,
) -> Result<ProviderBillingEvidenceJoinFileReport, String> {
    let text = std::fs::read_to_string(trace_path).map_err(|error| {
        format!(
            "failed to read provider billing join trace '{}': {error}",
            trace_path.display()
        )
    })?;
    let file_name = trace_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("real-traffic.trace.jsonl");
    let output_path = output_dir.join(format!("{file_name}.provider-billing-enriched.jsonl"));
    let mut output = String::new();
    let mut input_rows = 0usize;
    let mut output_rows = 0usize;
    let mut rows_with_shadow_request = 0usize;
    let mut matched_rows = 0usize;
    let mut rows_enriched_provider_cost = 0usize;
    let mut rows_enriched_tokens = 0usize;

    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        input_rows += 1;
        let mut row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse provider billing join trace '{}' line {}: {error}",
                trace_path.display(),
                line_index + 1
            )
        })?;
        if row
            .get("nando_shadow_request")
            .and_then(serde_json::Value::as_object)
            .is_some()
        {
            rows_with_shadow_request += 1;
        }
        let evidence = trace_provider_billing_match_keys(&row)
            .into_iter()
            .find_map(|key| evidence_map.by_key.get(&key));
        if let Some(evidence) = evidence {
            matched_rows += 1;
            if apply_provider_billing_evidence(&mut row, evidence)? {
                rows_enriched_provider_cost += 1;
            }
            if evidence.total_tokens.unwrap_or(0) > 0
                || evidence.input_tokens.unwrap_or(0) > 0
                || evidence.output_tokens.unwrap_or(0) > 0
            {
                rows_enriched_tokens += 1;
            }
        }
        output.push_str(
            &serde_json::to_string(&row)
                .map_err(|error| format!("failed to serialize billing-enriched row: {error}"))?,
        );
        output.push('\n');
        output_rows += 1;
    }
    std::fs::write(&output_path, output).map_err(|error| {
        format!(
            "failed to write provider billing enriched trace '{}': {error}",
            output_path.display()
        )
    })?;
    Ok(ProviderBillingEvidenceJoinFileReport {
        input_path: trace_path.display().to_string(),
        output_path: output_path.display().to_string(),
        input_rows,
        output_rows,
        rows_with_shadow_request,
        matched_rows,
        rows_enriched_provider_cost,
        rows_enriched_tokens,
    })
}

fn trace_provider_billing_match_keys(row: &serde_json::Value) -> Vec<String> {
    let mut keys = BTreeSet::new();
    for field in ["request_fingerprint", "exact_cache_key", "trace_id"] {
        if let Some(value) = json_string(row, &[field]).filter(|value| !value.is_empty()) {
            keys.insert(format!("{field}:{value}"));
        }
    }
    keys.into_iter().collect()
}

fn apply_provider_billing_evidence(
    row: &mut serde_json::Value,
    evidence: &ProviderBillingEvidence,
) -> Result<bool, String> {
    set_json_string(
        row,
        "provider_billing_evidence_id",
        &evidence.billing_evidence_id,
    )?;
    set_json_string(
        row,
        "provider_billing_evidence_source",
        &evidence.billing_source,
    )?;
    if let Some(provider) = &evidence.provider {
        set_json_string(row, "provider", provider)?;
    }
    if let Some(model_id) = &evidence.model_id {
        set_json_string(row, "model_id", model_id)?;
    }
    if let Some(input_tokens) = evidence.input_tokens {
        set_json_usize(row, "input_tokens", input_tokens)?;
    }
    if let Some(output_tokens) = evidence.output_tokens {
        set_json_usize(row, "output_tokens", output_tokens)?;
    }
    if let Some(cached_input_tokens) = evidence.cached_input_tokens {
        set_json_usize(row, "cached_input_tokens", cached_input_tokens)?;
    }
    if let Some(total_tokens) = evidence.total_tokens {
        set_json_usize(row, "estimated_total_tokens", total_tokens)?;
        set_nested_token_cost_usize(row, "total_tokens", total_tokens)?;
        set_nested_token_cost_bool(row, "token_evidence_missing", false)?;
        set_json_bool(row, "token_cost_estimate_used", false)?;
        set_nested_token_cost_bool(row, "token_cost_estimate_used", false)?;
    }
    if let Some(provider_cost) = evidence.provider_cost_microusd {
        set_json_u64(row, "provider_cost_microusd", provider_cost)?;
        set_nested_token_cost_u64(row, "total_cost_microusd", provider_cost)?;
        set_nested_token_cost_bool(row, "cost_evidence_missing", false)?;
        return Ok(true);
    }
    Ok(false)
}

fn generic_event_vector(
    event: &GenericRealTrafficEvent,
    cells: usize,
) -> Vec<nando_core::PhaseCenterCell> {
    let atoms = generic_event_atoms(event);
    phase_vector_from_atoms(atoms.iter().map(String::as_str), cells)
}

fn generic_event_atoms(event: &GenericRealTrafficEvent) -> Vec<String> {
    let mut atoms = vec![
        "profile:generic_real_traffic".to_owned(),
        format!("route_key:{}", event.route_key),
        format!("profile_id:{}", event.profile_id),
        format!(
            "traffic_source:{}",
            traffic_source_kind(&event.traffic_source)
        ),
        format!(
            "verification_source:{}",
            verification_source_kind(&event.verification_source)
        ),
        format!(
            "expect_local_operator:{}",
            event.expect_local_operator.unwrap_or(false)
        ),
        format!(
            "tool_call_fingerprint_count:{}",
            event.tool_call_fingerprint_count
        ),
    ];
    for (center_id, strength) in &event.active_fringe {
        atoms.push(format!("active_center:{center_id}:strength:{strength}"));
    }
    for slot in &event.slot_summary {
        atoms.push(format!("slot_summary:{slot}"));
    }
    atoms
}

fn generic_slot_atom(slot: &serde_json::Value) -> String {
    let output_slot = slot
        .get("binding_output_slot")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let positive_count = slot
        .get("positive_impulses")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    let negative_count = slot
        .get("negative_impulses")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    format!("out:{output_slot}:pos_count:{positive_count}:neg_count:{negative_count}")
}

fn generic_event_token_cost(event: &GenericRealTrafficEvent) -> GenericTokenCost {
    let input = event.input_tokens.unwrap_or(0);
    let output = event.output_tokens.unwrap_or(0);
    let cached = event.cached_input_tokens.unwrap_or(0);
    let total_tokens = event
        .estimated_total_tokens
        .unwrap_or_else(|| input.saturating_sub(cached).saturating_add(output));
    let total_cost_microusd = event
        .provider_cost_microusd
        .or(event.estimated_total_cost_microusd)
        .unwrap_or(0);
    let token_evidence_missing = total_tokens == 0;
    let cost_evidence_missing = total_cost_microusd == 0;
    GenericTokenCost {
        total_tokens,
        total_cost_microusd,
        evidence_missing: token_evidence_missing && cost_evidence_missing,
        token_evidence_missing,
        cost_evidence_missing,
    }
}

fn exact_cache_hit_flags_generic(events: &[GenericRealTrafficEvent]) -> Vec<bool> {
    let mut seen_request = BTreeSet::new();
    let mut seen_exact_cache = BTreeSet::new();
    let mut flags = Vec::with_capacity(events.len());
    for event in events {
        let fingerprint_hit = !seen_request.insert(event.request_fingerprint.as_str());
        let exact_key_hit = !seen_exact_cache.insert(event.exact_cache_key.as_str());
        let explicit = event.explicit_provider_cache_hit == Some(true);
        flags.push(explicit || fingerprint_hit || exact_key_hit);
    }
    flags
}

fn generic_bucket_key(event: &GenericRealTrafficEvent, mode: GenericBucketMode) -> String {
    match mode {
        GenericBucketMode::Route => format!("{}::{}", event.profile_id, event.route_key),
        GenericBucketMode::RequestShape => format!(
            "{}::{}::{}",
            event.profile_id,
            event.route_key,
            generic_request_shape_subkey(event)
        ),
        GenericBucketMode::ActionFamily => format!(
            "{}::{}::{}",
            event.profile_id,
            event.route_key,
            generic_action_family_subkey(event)
        ),
        GenericBucketMode::StateAction => format!(
            "{}::{}::{}",
            event.profile_id,
            event.route_key,
            generic_state_action_subkey(event)
        ),
    }
}

fn generic_request_shape_subkey(event: &GenericRealTrafficEvent) -> String {
    let slot_shape = if event.slot_summary.is_empty() {
        "slots:none".to_owned()
    } else {
        event.slot_summary.join("|")
    };
    format!("shape_v1:slot:{slot_shape}")
}

fn generic_action_family_subkey(event: &GenericRealTrafficEvent) -> String {
    format!(
        "action_family_v1:tools:{}:active:{}:slot_count:{}",
        generic_count_band(event.tool_call_fingerprint_count),
        generic_count_band(event.active_fringe.len()),
        event.slot_summary.len()
    )
}

fn generic_state_action_subkey(event: &GenericRealTrafficEvent) -> String {
    let slot_bands = if event.slot_summary.is_empty() {
        "slots:none".to_owned()
    } else {
        event
            .slot_summary
            .iter()
            .map(|slot| generic_slot_band_atom(slot))
            .collect::<Vec<_>>()
            .join("|")
    };
    format!(
        "state_action_v1:tools:{}:active:{}:slot_count:{}:{}",
        generic_count_band(event.tool_call_fingerprint_count),
        generic_count_band(event.active_fringe.len()),
        event.slot_summary.len(),
        slot_bands
    )
}

fn generic_slot_band_atom(slot_summary: &str) -> String {
    let parts = slot_summary.split(':').collect::<Vec<_>>();
    if parts.len() == 6 && parts[0] == "out" && parts[2] == "pos_count" && parts[4] == "neg_count" {
        let output_slot = parts[1];
        let positive_count = parts[3].parse::<usize>().unwrap_or(0);
        let negative_count = parts[5].parse::<usize>().unwrap_or(0);
        format!(
            "o{output_slot}p{}n{}",
            generic_count_band(positive_count),
            generic_count_band(negative_count)
        )
    } else {
        format!("raw:{slot_summary}")
    }
}

const fn generic_count_band(count: usize) -> &'static str {
    match count {
        0 => "0",
        1..=3 => "1_3",
        4..=7 => "4_7",
        8..=15 => "8_15",
        16..=19 => "16_19",
        20..=23 => "20_23",
        24 => "24",
        25..=63 => "25_63",
        64..=95 => "64_95",
        96..=127 => "96_127",
        _ => "128p",
    }
}

fn generic_label_counts_for_indices(
    events: &[GenericRealTrafficEvent],
    indices: &[usize],
) -> (usize, usize) {
    let mut true_count = 0usize;
    let mut false_count = 0usize;
    for &index in indices {
        if events[index].verified_safe_accept {
            true_count += 1;
        } else {
            false_count += 1;
        }
    }
    (true_count, false_count)
}

fn is_legacy_profile_name(value: &str) -> bool {
    value.contains("role_binding") || value.contains("nwrb")
}

fn new_generic_online_bucket_state(
    bucket_key: String,
    route_key: String,
    profile_id: String,
) -> GenericOnlineBucketState {
    GenericOnlineBucketState {
        bucket_key,
        route_key,
        profile_id,
        event_indices: Vec::new(),
        verifier_true_events: 0,
        verifier_false_events: 0,
        compiled: None,
        shadow_events: 0,
        shadow_safe_events: 0,
        shadow_accepts: 0,
        false_accepts: 0,
        missed_safe_accepts: 0,
        runtime_margin_parity_mismatches: 0,
        margins: Vec::new(),
        exact_cache_hits_in_shadow: 0,
        unique_cpu_accepts_over_exact_cache: 0,
        nando_cpu_tokens_saved: 0,
        nando_cpu_cost_saved_microusd: 0,
        unique_accepts: BTreeMap::new(),
        token_cost_evidence_missing_events: 0,
        token_evidence_missing_events: 0,
        cost_evidence_missing_events: 0,
    }
}

fn route_local_four_way_split(
    events: &[GenericRealTrafficEvent],
    selector_permille: usize,
    compile_permille: usize,
    calibration_permille: usize,
) -> Result<GenericRouteLocalFourWaySplit, String> {
    let mut route_event_indices = BTreeMap::<String, Vec<usize>>::new();
    for (event_index, event) in events.iter().enumerate() {
        route_event_indices
            .entry(format!("{}::{}", event.profile_id, event.route_key))
            .or_default()
            .push(event_index);
    }
    let mut split = GenericRouteLocalFourWaySplit::default();
    for indices in route_event_indices.values() {
        if indices.len() < 4 {
            continue;
        }
        let selector_count = (indices.len() * selector_permille / 1000)
            .max(1)
            .min(indices.len().saturating_sub(3));
        let compile_end_count = (indices.len() * (selector_permille + compile_permille) / 1000)
            .max(selector_count + 1)
            .min(indices.len().saturating_sub(2));
        let calibration_end_count =
            (indices.len() * (selector_permille + compile_permille + calibration_permille) / 1000)
                .max(compile_end_count + 1)
                .min(indices.len().saturating_sub(1));
        for &event_index in &indices[..selector_count] {
            split.selector_indices.insert(event_index);
        }
        for &event_index in &indices[selector_count..compile_end_count] {
            split.compile_indices.insert(event_index);
        }
        for &event_index in &indices[compile_end_count..calibration_end_count] {
            split.calibration_indices.insert(event_index);
        }
        for &event_index in &indices[calibration_end_count..] {
            split.shadow_indices.insert(event_index);
        }
    }
    split.disjoint = split.selector_indices.is_disjoint(&split.compile_indices)
        && split
            .selector_indices
            .is_disjoint(&split.calibration_indices)
        && split.selector_indices.is_disjoint(&split.shadow_indices)
        && split
            .compile_indices
            .is_disjoint(&split.calibration_indices)
        && split.compile_indices.is_disjoint(&split.shadow_indices)
        && split.calibration_indices.is_disjoint(&split.shadow_indices)
        && !split.selector_indices.is_empty()
        && !split.compile_indices.is_empty()
        && !split.calibration_indices.is_empty()
        && !split.shadow_indices.is_empty();
    if !split.disjoint {
        return Err(
            "route-local selector/compile/calibration/shadow windows are not strictly disjoint"
                .to_owned(),
        );
    }
    Ok(split)
}

fn default_frontier_union_report_paths() -> Vec<PathBuf> {
    vec![
        PathBuf::from(
            "target/nando-wave/streaming/real-traffic-phase-center-refined-online-discovery-v1.report.json",
        ),
        PathBuf::from(
            "target/nando-wave/streaming/real-traffic-phase-center-state-action-online-discovery-v1.report.json",
        ),
        PathBuf::from(
            "target/nando-wave/streaming/real-traffic-phase-center-guarded-separator-split-shadow-v1.report.json",
        ),
        PathBuf::from(
            "target/nando-wave/streaming/real-traffic-phase-center-guarded-separator-calibrated-split-shadow-v1.report.json",
        ),
        PathBuf::from(DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_PROMOTION_AUDIT_REPORT),
        PathBuf::from(
            "target/nando-wave/streaming/phase-atom-metrics-report-time-split-promotion-audit-v1.report.json",
        ),
        PathBuf::from(
            "target/nando-wave/streaming/phase-atom-planning-time-split-promotion-audit-v1.report.json",
        ),
        PathBuf::from(
            "target/nando-wave/streaming/phase-atom-tool-status-time-split-promotion-audit-v1.report.json",
        ),
    ]
}

fn default_enriched_trace_paths() -> Vec<PathBuf> {
    vec![
        PathBuf::from(
            "target/nando-wave/streaming/token-cost-enriched-traces/agent-continue-execute-artifact-progress-v1-current5k.trace.jsonl.token-cost-enriched.jsonl",
        ),
        PathBuf::from(
            "target/nando-wave/streaming/token-cost-enriched-traces/metrics-report-output-evidence-v1-5k.trace.jsonl.token-cost-enriched.jsonl",
        ),
        PathBuf::from(
            "target/nando-wave/streaming/token-cost-enriched-traces/serving-ops-output-evidence-v1-current5k.trace.jsonl.token-cost-enriched.jsonl",
        ),
        PathBuf::from(
            "target/nando-wave/streaming/token-cost-enriched-traces/answer-evidence-output-evidence-v1.trace.jsonl.token-cost-enriched.jsonl",
        ),
        PathBuf::from(
            "target/nando-wave/streaming/token-cost-enriched-traces/read-inspect-output-evidence-v1.trace.jsonl.token-cost-enriched.jsonl",
        ),
    ]
}

fn forbidden_flags_value_all_false(value: &serde_json::Value) -> bool {
    [
        "target_id_used",
        "proof_rule_id_authority_used",
        "concrete_x_lookup_used",
        "manual_local_out_t_used",
        "hidden_frame_id_or_bind_x_used",
        "legacy_backend_used",
    ]
    .into_iter()
    .all(|key| value.get(key).and_then(serde_json::Value::as_bool) == Some(false))
}

fn read_cpu10_trace_events(
    path: &Path,
) -> Result<(usize, usize, Vec<GenericCpu10TraceEvent>), String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read trace '{}': {error}", path.display()))?;
    let mut total_rows = 0usize;
    let mut rows_without_shadow_request = 0usize;
    let mut events = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse trace '{}' line {}: {error}",
                path.display(),
                line_index + 1
            )
        })?;
        if !row.is_object() {
            continue;
        }
        total_rows += 1;
        if let Some(event) = parse_cpu10_trace_event(&row, events.len()) {
            events.push(event);
        } else {
            rows_without_shadow_request += 1;
        }
    }
    Ok((total_rows, rows_without_shadow_request, events))
}

fn parse_cpu10_trace_event(
    row: &serde_json::Value,
    fallback_index: usize,
) -> Option<GenericCpu10TraceEvent> {
    let request = row.get("nando_shadow_request")?.as_object()?;
    let route_key = json_field_string(request.get("route_key")).unwrap_or_else(|| "unknown".into());
    let profile_id =
        json_field_string(request.get("profile_id")).unwrap_or_else(|| route_key.clone());
    let legacy = is_legacy_profile_name(&route_key) || is_legacy_profile_name(&profile_id);
    let request_fingerprint = row
        .get("request_fingerprint")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| json_field_string(request.get("exact_cache_key")))
        .unwrap_or_else(|| format!("cpu10-event-{fallback_index:08}"));
    let exact_cache_key = row
        .get("exact_cache_key")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| json_field_string(request.get("exact_cache_key")))
        .unwrap_or_else(|| request_fingerprint.clone());
    let token_cost = GenericTokenCost {
        total_tokens: json_usize(row.get("estimated_total_tokens")).unwrap_or_else(|| {
            let input = json_usize(row.get("input_tokens")).unwrap_or(0);
            let cached = json_usize(row.get("cached_input_tokens")).unwrap_or(0);
            let output = json_usize(row.get("output_tokens")).unwrap_or(0);
            input.saturating_sub(cached).saturating_add(output)
        }),
        total_cost_microusd: row
            .get("provider_cost_microusd")
            .and_then(serde_json::Value::as_u64)
            .or_else(|| {
                row.get("estimated_total_cost_microusd")
                    .and_then(serde_json::Value::as_u64)
            })
            .unwrap_or(0),
        evidence_missing: false,
        token_evidence_missing: false,
        cost_evidence_missing: false,
    };
    Some(GenericCpu10TraceEvent {
        bucket_key: format!("{profile_id}::{route_key}"),
        route_key,
        profile_id,
        request_fingerprint,
        exact_cache_key,
        explicit_provider_cache_hit: row
            .get("provider_cache_hit")
            .or_else(|| row.get("exact_cache_hit"))
            .and_then(serde_json::Value::as_bool),
        verified_safe_accept: row
            .get("verified_safe_accept")
            .and_then(serde_json::Value::as_bool),
        token_cost,
        legacy,
    })
}

fn exact_cache_hit_flags_cpu10(events: &[GenericCpu10TraceEvent]) -> Vec<bool> {
    let mut seen_request = BTreeSet::new();
    let mut seen_exact_cache = BTreeSet::new();
    let mut flags = Vec::with_capacity(events.len());
    for event in events {
        if event.legacy {
            flags.push(false);
            continue;
        }
        let fingerprint_hit = !seen_request.insert(event.request_fingerprint.as_str());
        let exact_key_hit = !seen_exact_cache.insert(event.exact_cache_key.as_str());
        let explicit = event.explicit_provider_cache_hit == Some(true);
        flags.push(explicit || fingerprint_hit || exact_key_hit);
    }
    flags
}

fn cpu10_route_recommended_next_action(
    state: &GenericCpu10RouteGapState,
    total_true_ceiling: usize,
) -> &'static str {
    if state.verifier_true_rows == 0 {
        "collect_verified_positive_events"
    } else if state.verifier_false_rows == 0 {
        "collect_verified_negative_events"
    } else if state.verifier_true_over_exact_cache_ceiling == 0 {
        "watch_exact_cache_or_collect_new_unique_events"
    } else if state.verifier_missing_rows > state.verifier_true_rows {
        "add_missing_verifier_labels_before_scoring"
    } else if per_thousand(
        state.verifier_true_over_exact_cache_ceiling,
        total_true_ceiling,
    ) >= 100
    {
        "mine_separating_phase_buckets_for_this_route"
    } else {
        "collect_more_trace_before_optimizing"
    }
}

fn infer_shadow_gap_route_from_path(path: &Path) -> (String, String) {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if name.contains("agent-continue") {
        (
            "agent_continue_execute".to_owned(),
            "route_gap_agent_continue_execute_profile_v1".to_owned(),
        )
    } else if name.contains("metrics-report") {
        (
            "metrics_report_readout".to_owned(),
            "route_gap_metrics_report_profile_v1".to_owned(),
        )
    } else if name.contains("serving-ops") {
        (
            "serving_ops".to_owned(),
            "route_gap_serving_ops_profile_v1".to_owned(),
        )
    } else if name.contains("answer-evidence") {
        (
            "answer_or_explain".to_owned(),
            "route_gap_answer_evidence_profile_v1".to_owned(),
        )
    } else if name.contains("read-inspect") {
        (
            "read_inspect".to_owned(),
            "route_gap_read_inspect_profile_v1".to_owned(),
        )
    } else {
        (
            "unknown_route".to_owned(),
            "unknown_phase_center_profile".to_owned(),
        )
    }
}

fn shadow_gap_route_state<'a>(
    states: &'a mut BTreeMap<String, GenericShadowRequestGapState>,
    route_key: &str,
    profile_id: &str,
) -> &'a mut GenericShadowRequestGapState {
    let bucket_key = format!("{profile_id}::{route_key}");
    states
        .entry(bucket_key.clone())
        .or_insert_with(|| GenericShadowRequestGapState {
            bucket_key,
            route_key: route_key.to_owned(),
            profile_id: profile_id.to_owned(),
            ..Default::default()
        })
}

fn generic_token_cost_from_row(row: &serde_json::Value) -> GenericTokenCost {
    let input_tokens = json_usize(row.get("input_tokens"))
        .or_else(|| json_usize(json_at(row, &["token_cost", "input_tokens"])))
        .unwrap_or(0);
    let cached_input_tokens = json_usize(row.get("cached_input_tokens"))
        .or_else(|| json_usize(json_at(row, &["token_cost", "cached_input_tokens"])))
        .unwrap_or(0);
    let output_tokens = json_usize(row.get("output_tokens"))
        .or_else(|| json_usize(json_at(row, &["token_cost", "output_tokens"])))
        .unwrap_or(0);
    let total_tokens = json_usize(row.get("estimated_total_tokens"))
        .or_else(|| json_usize(json_at(row, &["token_cost", "total_tokens"])))
        .unwrap_or_else(|| input_tokens.saturating_sub(cached_input_tokens) + output_tokens);
    let provider_cost = row
        .get("provider_cost_microusd")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let estimated_total_cost = row
        .get("estimated_total_cost_microusd")
        .and_then(serde_json::Value::as_u64)
        .or_else(|| {
            row.get("token_cost")
                .and_then(|token_cost| token_cost.get("total_cost_microusd"))
                .and_then(serde_json::Value::as_u64)
        })
        .unwrap_or_else(|| {
            row.get("estimated_input_cost_microusd")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                .saturating_add(
                    row.get("estimated_output_cost_microusd")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0),
                )
        });
    let total_cost_microusd = if provider_cost > 0 {
        provider_cost
    } else {
        estimated_total_cost
    };
    GenericTokenCost {
        total_tokens,
        total_cost_microusd,
        evidence_missing: total_tokens == 0 && total_cost_microusd == 0,
        token_evidence_missing: total_tokens == 0,
        cost_evidence_missing: total_cost_microusd == 0,
    }
}

fn shadow_gap_recommended_next_action(state: &GenericShadowRequestGapState) -> &'static str {
    if state.scoreable_verifier_true_rows > 0 && state.scoreable_verifier_false_rows > 0 {
        "mine_separating_phase_buckets_for_existing_shadow_requests"
    } else if state.missing_shadow_rejected_candidate_rows == 0
        && state.missing_shadow_not_route_candidate_rows > 0
    {
        "mine_new_route_family_from_real_trace_before_payload_adapter"
    } else if state.missing_shadow_missing_verifier_signal_rows > 0 {
        "add_request_time_verifier_signal_or_external_verifier_evidence"
    } else if state.missing_shadow_missing_evidence_signal_rows > 0 {
        "build_request_side_evidence_signal_adapter"
    } else if state.missing_shadow_missing_context_signal_rows > 0 {
        "build_request_side_context_signal_adapter"
    } else if state.missing_shadow_missing_request_signal_rows > 0 {
        "build_request_side_route_signal_adapter"
    } else if state.missing_shadow_builder_rejected_request_side_features_rows > 0 {
        "inspect_builder_feature_rejection_and_split_profile"
    } else {
        "collect_more_trace_or_split_profile"
    }
}

fn mining_input_readiness_observe_row(
    state: &mut GenericMiningInputReadinessState,
    row: &serde_json::Value,
) {
    state.total_rows += 1;
    let has_shadow_request = row
        .get("nando_shadow_request")
        .and_then(serde_json::Value::as_object)
        .is_some();
    if has_shadow_request {
        state.shadow_request_rows += 1;
    } else {
        state.missing_shadow_request_rows += 1;
    }

    let llm_call = row.get("llm_call").unwrap_or(&serde_json::Value::Null);
    match llm_call {
        serde_json::Value::Object(_) => state.llm_call_object_rows += 1,
        serde_json::Value::String(_) => state.llm_call_string_rows += 1,
        serde_json::Value::Bool(_) => state.llm_call_boolean_rows += 1,
        serde_json::Value::Null => state.llm_call_null_rows += 1,
        _ => {}
    }
    let tool_count = row
        .get("tool_call_fingerprints")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    if tool_count > 0 {
        state.tool_fingerprint_rows += 1;
    }
    let has_request_side_atoms = llm_call.is_object()
        || row.get("request_atoms").is_some()
        || row.get("state_atoms").is_some()
        || row.get("action_atoms").is_some()
        || tool_count > 0;
    if !has_shadow_request && has_request_side_atoms {
        state.missing_shadow_rows_with_request_side_atoms += 1;
    }
    if !has_shadow_request && llm_call.is_boolean() && !has_request_side_atoms {
        state.missing_shadow_rows_with_only_boolean_llm_call += 1;
    }
}

fn merge_mining_input_readiness(
    total: &mut GenericMiningInputReadinessState,
    file: &GenericMiningInputReadinessState,
) {
    total.total_rows += file.total_rows;
    total.shadow_request_rows += file.shadow_request_rows;
    total.missing_shadow_request_rows += file.missing_shadow_request_rows;
    total.llm_call_object_rows += file.llm_call_object_rows;
    total.llm_call_string_rows += file.llm_call_string_rows;
    total.llm_call_boolean_rows += file.llm_call_boolean_rows;
    total.llm_call_null_rows += file.llm_call_null_rows;
    total.tool_fingerprint_rows += file.tool_fingerprint_rows;
    total.missing_shadow_rows_with_request_side_atoms +=
        file.missing_shadow_rows_with_request_side_atoms;
    total.missing_shadow_rows_with_only_boolean_llm_call +=
        file.missing_shadow_rows_with_only_boolean_llm_call;
}

fn build_phase_atom_trace_row(
    input_path: &Path,
    row: &serde_json::Value,
    state: &mut GenericPhaseAtomTraceBuildState,
) -> serde_json::Value {
    let request = row
        .get("nando_shadow_request")
        .and_then(serde_json::Value::as_object);
    let has_shadow_request = request.is_some();
    state.rows_with_shadow_request += usize::from(has_shadow_request);

    let verified_safe_accept = row
        .get("verified_safe_accept")
        .and_then(serde_json::Value::as_bool);
    let has_verifier_label = verified_safe_accept.is_some();
    state.rows_with_verifier_label += usize::from(has_verifier_label);

    let token_cost = generic_token_cost_from_row(row);
    state.rows_with_token_or_cost +=
        usize::from(token_cost.total_tokens > 0 || token_cost.total_cost_microusd > 0);
    let external_provider_correlation_keys = phase_atom_external_provider_correlation_keys(row);
    let provider_correlation_ready = !external_provider_correlation_keys.is_empty();
    state.rows_with_provider_correlation_keys += usize::from(provider_correlation_ready);

    let explicit_request_atoms = json_string_vec(row.get("request_atoms"));
    let explicit_state_atoms = json_string_vec(row.get("state_atoms"));
    let explicit_action_atoms = json_string_vec(row.get("action_atoms"));
    let explicit_tool_atoms = json_string_vec(row.get("tool_atoms"));
    state.rows_with_explicit_request_atoms += usize::from(!explicit_request_atoms.is_empty());
    state.rows_with_explicit_state_atoms += usize::from(!explicit_state_atoms.is_empty());
    state.rows_with_explicit_action_atoms += usize::from(!explicit_action_atoms.is_empty());
    state.rows_with_explicit_tool_atoms += usize::from(!explicit_tool_atoms.is_empty());

    let mut request_atoms = explicit_request_atoms;
    let mut state_atoms = explicit_state_atoms;
    let mut action_atoms = explicit_action_atoms;
    let mut tool_atoms = explicit_tool_atoms;
    let observable_llm_text = observable_llm_request_text(row);
    if let Some(prompt) = observable_llm_text.as_deref() {
        if request_atoms.is_empty() {
            request_atoms.extend(codex_history_request_atoms(prompt));
        }
        if state_atoms.is_empty() {
            let state_session = row
                .get("session_id")
                .or_else(|| row.get("trace_id"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("generic_phase_trace");
            state_atoms.extend(codex_history_state_atoms(state_session, 0, prompt));
        }
        if action_atoms.is_empty() {
            action_atoms.extend(codex_history_action_atoms(prompt));
        }
        if tool_atoms.is_empty() {
            tool_atoms.extend(codex_history_tool_atoms(prompt));
        }
    }

    let mut metadata_atoms = Vec::new();
    let traffic_source = row
        .get("traffic_source")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown_traffic_source");
    let verification_source = row
        .get("verification_source")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown_verification_source");
    metadata_atoms.push(format!(
        "traffic_source_kind:{}",
        traffic_source_kind(traffic_source)
    ));
    metadata_atoms.push(format!(
        "verification_source_kind:{}",
        verification_source_kind(verification_source)
    ));
    metadata_atoms.push(format!(
        "llm_call_kind:{}",
        llm_call_kind(row.get("llm_call"))
    ));
    metadata_atoms.push(format!("has_shadow_request:{has_shadow_request}"));
    metadata_atoms.push(format!("has_verifier_label:{has_verifier_label}"));
    metadata_atoms.push(format!(
        "synthetic_source:{}",
        row.get("synthetic_source")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    ));
    metadata_atoms.push(format!(
        "token_band:{}",
        generic_count_band(token_cost.total_tokens)
    ));
    metadata_atoms.push(format!(
        "cost_band:{}",
        generic_count_band(token_cost.total_cost_microusd as usize)
    ));

    let mut route_hint_atoms = Vec::new();
    let shadow_route_key = request.and_then(|request| json_field_string(request.get("route_key")));
    let shadow_profile_id =
        request.and_then(|request| json_field_string(request.get("profile_id")));
    let traffic_source_route_hint = route_hint_from_traffic_source(traffic_source);
    let route_key_for_atoms = shadow_route_key
        .as_deref()
        .or(traffic_source_route_hint)
        .unwrap_or("unknown_route");
    if request.is_some() {
        if let Some(route_key) = shadow_route_key.as_deref() {
            route_hint_atoms.push(format!("route_key:{route_key}"));
        }
        if let Some(profile_id) = shadow_profile_id.as_deref() {
            route_hint_atoms.push(format!("profile_id:{profile_id}"));
        }
    } else if let Some(route_hint) = traffic_source_route_hint {
        route_hint_atoms.push(format!("route_hint_from_traffic_source:{route_hint}"));
    }

    let mut derived_tool_atoms = Vec::new();
    let tool_call_fingerprint_count = row
        .get("tool_call_fingerprints")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    derived_tool_atoms.push(format!(
        "tool_call_fingerprint_count_band:{}",
        generic_count_band(tool_call_fingerprint_count)
    ));
    if tool_call_fingerprint_count > 0 {
        derived_tool_atoms.push("tool_call_fingerprint_present:true".to_owned());
    }

    let mut shadow_payload_atoms = Vec::new();
    if let Some(request) = request {
        if let Some(active_fringe) = request
            .get("active_fringe")
            .and_then(serde_json::Value::as_array)
        {
            shadow_payload_atoms.push(format!(
                "shadow_active_fringe_len_band:{}",
                generic_count_band(active_fringe.len())
            ));
            for center_id in active_fringe
                .iter()
                .filter_map(|item| item.get("center_id").and_then(serde_json::Value::as_u64))
                .take(8)
            {
                shadow_payload_atoms
                    .push(format!("shadow_active_center_page:{}", center_id / 4096));
            }
        }
        if let Some(slots) = request.get("slots").and_then(serde_json::Value::as_array) {
            shadow_payload_atoms.push(format!(
                "shadow_slot_count_band:{}",
                generic_count_band(slots.len())
            ));
            for slot in slots.iter().take(8) {
                shadow_payload_atoms.push(format!("shadow_slot_shape:{}", generic_slot_atom(slot)));
            }
        }
    }
    state.rows_with_shadow_payload_atoms += usize::from(!shadow_payload_atoms.is_empty());

    if request_atoms.is_empty() {
        request_atoms.push(format!(
            "request_traffic_source_kind:{}",
            traffic_source_kind(traffic_source)
        ));
        request_atoms.push(format!("request_route_family:{route_key_for_atoms}"));
        request_atoms.push(format!("request_has_shadow_request:{has_shadow_request}"));
        request_atoms.push(format!(
            "request_token_band:{}",
            generic_count_band(token_cost.total_tokens)
        ));
    }
    if state_atoms.is_empty() {
        state_atoms.push(format!(
            "state_verification_source_kind:{}",
            verification_source_kind(verification_source)
        ));
        state_atoms.push(format!("state_has_verifier_label:{has_verifier_label}"));
        state_atoms.extend(shadow_payload_atoms.iter().cloned());
    }
    if action_atoms.is_empty() {
        if let Some(action_family) = action_family_from_route_key(route_key_for_atoms) {
            action_atoms.push(format!("action_family:{action_family}"));
        }
        if route_key_for_atoms != "unknown_route" {
            action_atoms.push(format!("route_operator:{route_key_for_atoms}"));
        }
    }
    if tool_atoms.is_empty() {
        tool_atoms = derived_tool_atoms.clone();
    }

    let has_state_or_request_atoms = !state_atoms.is_empty() || !request_atoms.is_empty();
    let has_action_atoms = !action_atoms.is_empty();
    let ready_for_route_family_mining =
        has_state_or_request_atoms && has_action_atoms && has_verifier_label;
    let ready_for_existing_shadow_scoring = has_shadow_request && has_verifier_label;
    state.rows_ready_for_route_family_mining += usize::from(ready_for_route_family_mining);
    state.rows_ready_for_existing_shadow_scoring += usize::from(ready_for_existing_shadow_scoring);
    state.rows_missing_state_or_request_atoms += usize::from(!has_state_or_request_atoms);
    state.rows_missing_action_atoms += usize::from(!has_action_atoms);
    state.rows_missing_verifier_label += usize::from(!has_verifier_label);

    let metadata_only = request_atoms.is_empty()
        && state_atoms.is_empty()
        && action_atoms.is_empty()
        && tool_atoms.is_empty()
        && shadow_payload_atoms.is_empty();
    state.metadata_only_rows += usize::from(metadata_only);

    let output_atoms_written = metadata_atoms.len()
        + route_hint_atoms.len()
        + request_atoms.len()
        + state_atoms.len()
        + action_atoms.len()
        + tool_atoms.len()
        + derived_tool_atoms.len()
        + shadow_payload_atoms.len();
    state.output_atoms_written += output_atoms_written;

    serde_json::json!({
        "schema_version": "real_traffic_phase_atom_trace_v1",
        "input_trace_path": input_path.display().to_string(),
        "trace_id": row.get("trace_id").and_then(serde_json::Value::as_str),
        "time_ms": row.get("time_ms").and_then(serde_json::Value::as_i64),
        "request_fingerprint": row.get("request_fingerprint").and_then(serde_json::Value::as_str),
        "exact_cache_key": row.get("exact_cache_key").and_then(serde_json::Value::as_str),
        "external_provider_correlation_keys": external_provider_correlation_keys,
        "provider_correlation_ready": provider_correlation_ready,
        "traffic_source": traffic_source,
        "verification_source_kind": verification_source_kind(verification_source),
        "verified_safe_accept": verified_safe_accept,
        "has_shadow_request": has_shadow_request,
        "ready_for_route_family_mining": ready_for_route_family_mining,
        "ready_for_existing_shadow_scoring": ready_for_existing_shadow_scoring,
        "metadata_only": metadata_only,
        "missing_state_or_request_atoms": !has_state_or_request_atoms,
        "missing_action_atoms": !has_action_atoms,
        "missing_verifier_label": !has_verifier_label,
        "token_cost": {
            "total_tokens": token_cost.total_tokens,
            "total_cost_microusd": token_cost.total_cost_microusd,
            "token_evidence_missing": token_cost.token_evidence_missing,
            "cost_evidence_missing": token_cost.cost_evidence_missing
        },
        "request_atoms": request_atoms.clone(),
        "state_atoms": state_atoms.clone(),
        "action_atoms": action_atoms.clone(),
        "tool_atoms": tool_atoms.clone(),
        "route_hint_atoms": route_hint_atoms.clone(),
        "atom_groups": {
            "metadata_atoms": metadata_atoms,
            "route_hint_atoms": route_hint_atoms,
            "request_atoms": request_atoms,
            "state_atoms": state_atoms,
            "action_atoms": action_atoms,
            "tool_atoms": tool_atoms,
            "derived_tool_atoms": derived_tool_atoms,
            "shadow_payload_atoms": shadow_payload_atoms
        },
        "forbidden_fields_absent": {
            "raw_response_text": true,
            "target_id": true,
            "proof_rule_id": true,
            "concrete_x_lookup": true,
            "manual_local_out_t": true
        }
    })
}

fn json_string_vec(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn llm_call_kind(value: Option<&serde_json::Value>) -> &'static str {
    match value {
        Some(serde_json::Value::Object(_)) => "object",
        Some(serde_json::Value::String(_)) => "string",
        Some(serde_json::Value::Bool(_)) => "boolean",
        Some(serde_json::Value::Null) | None => "null",
        Some(serde_json::Value::Array(_)) => "array",
        Some(serde_json::Value::Number(_)) => "number",
    }
}

fn observable_llm_request_text(row: &serde_json::Value) -> Option<String> {
    for path in [
        &["prompt"][..],
        &["request_text"][..],
        &["user_prompt"][..],
        &["input"][..],
        &["message"][..],
        &["content"][..],
        &["llm_call"][..],
        &["llm_call", "prompt"][..],
        &["llm_call", "input"][..],
        &["llm_call", "content"][..],
        &["llm_call", "request"][..],
        &["request", "prompt"][..],
        &["request", "input"][..],
        &["request", "content"][..],
    ] {
        if let Some(text) = json_at(row, path)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            return Some(text.chars().take(4096).collect());
        }
    }
    let llm_call = row.get("llm_call")?;
    let mut parts = Vec::new();
    collect_observable_text_fragments(llm_call, &mut parts, 12);
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n").chars().take(4096).collect())
    }
}

fn collect_observable_text_fragments(
    value: &serde_json::Value,
    parts: &mut Vec<String>,
    limit: usize,
) {
    if parts.len() >= limit {
        return;
    }
    match value {
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                parts.push(trimmed.chars().take(1024).collect());
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_observable_text_fragments(item, parts, limit);
                if parts.len() >= limit {
                    break;
                }
            }
        }
        serde_json::Value::Object(map) => {
            for key in [
                "prompt",
                "input",
                "content",
                "text",
                "message",
                "messages",
                "instructions",
                "arguments",
            ] {
                if let Some(item) = map.get(key) {
                    collect_observable_text_fragments(item, parts, limit);
                    if parts.len() >= limit {
                        break;
                    }
                }
            }
        }
        _ => {}
    }
}

fn route_hint_from_traffic_source(traffic_source: &str) -> Option<&'static str> {
    if traffic_source.contains("agent_continue_execute") {
        Some("agent_continue_execute")
    } else if traffic_source.contains("metrics_report") {
        Some("metrics_report_readout")
    } else if traffic_source.contains("serving_ops") {
        Some("serving_ops")
    } else if traffic_source.contains("answer_evidence") {
        Some("answer_or_explain")
    } else if traffic_source.contains("read_inspect") {
        Some("read_inspect")
    } else if traffic_source.contains("git_control") {
        Some("git_control")
    } else if traffic_source.contains("test_output_parse") {
        Some("test_output_parse")
    } else {
        None
    }
}

fn action_family_from_route_key(route_key: &str) -> Option<&'static str> {
    match route_key {
        "agent_continue_execute" | "planning_next_step" | "artifact_progress" => Some("planning"),
        "metrics_report_readout" | "metrics_report" | "report_sync" => Some("metrics_report"),
        "serving_ops" => Some("serving_ops"),
        "answer_or_explain" | "answer_evidence" => Some("answer_or_explain"),
        "read_inspect" => Some("read_inspect"),
        "git_control" => Some("git_control"),
        "test_output_parse" | "run_check" => Some("run_check"),
        "edit_patch_small" | "edit_or_build" => Some("edit_or_build"),
        "agent_control" | "control_stop" => Some("control_stop"),
        _ => None,
    }
}

fn codex_history_request_atoms(prompt: &str) -> Vec<String> {
    let lower = prompt.to_lowercase();
    let mut atoms = BTreeSet::new();
    atoms.insert(format!(
        "request_char_band:{}",
        generic_count_band(prompt.chars().count())
    ));
    atoms.insert(format!(
        "request_line_count_band:{}",
        generic_count_band(prompt.lines().count())
    ));
    atoms.insert(format!(
        "request_word_count_band:{}",
        generic_count_band(prompt.split_whitespace().count())
    ));
    atoms.insert(format!("request_has_question:{}", prompt.contains('?')));
    atoms.insert(format!("request_has_code_fence:{}", prompt.contains("```")));
    atoms.insert(format!(
        "request_has_path:{}",
        lower.contains("/home/")
            || lower.contains("crates/")
            || lower.contains("docs/")
            || lower.contains("target/")
    ));
    atoms.insert(format!(
        "request_has_json_shape:{}",
        lower.contains(".json") || lower.contains("jsonl") || lower.contains('{')
    ));
    atoms.insert(format!(
        "request_has_cyrillic:{}",
        prompt
            .chars()
            .any(|ch| ('\u{0400}'..='\u{04ff}').contains(&ch))
    ));
    atoms.insert(format!(
        "request_has_latin:{}",
        prompt.chars().any(|ch| ch.is_ascii_alphabetic())
    ));
    for (needle, atom) in [
        ("nando", "topic:nando"),
        ("wave", "topic:wave"),
        ("llmwave", "topic:llmwave"),
        ("phase", "topic:phase_center"),
        (".nwpc", "topic:nwpc"),
        ("фурье", "topic:fourier"),
        ("goal", "topic:goal"),
        ("цель", "topic:goal"),
        ("метрик", "topic:metrics"),
        ("latency", "topic:latency"),
        ("p99", "topic:p99"),
    ] {
        if lower.contains(needle) {
            atoms.insert(atom.to_owned());
        }
    }
    for ext in [".rs", ".md", ".json", ".jsonl", ".toml", ".py"] {
        if lower.contains(ext) {
            atoms.insert(format!("request_mentions_ext:{ext}"));
        }
    }
    atoms.into_iter().collect()
}

fn codex_history_state_atoms(session_id: &str, session_turn: usize, prompt: &str) -> Vec<String> {
    let lower = prompt.to_lowercase();
    let mut atoms = BTreeSet::new();
    let session_bucket = stable_fingerprint([session_id]) % 64;
    atoms.insert("state_source:codex_history".to_owned());
    atoms.insert(format!("state_session_bucket:{session_bucket}"));
    atoms.insert(format!(
        "state_session_turn_band:{}",
        generic_count_band(session_turn)
    ));
    atoms.insert(format!(
        "state_followup_marker:{}",
        contains_any(
            &lower,
            &["дальше", "продолж", "continue", "ещё", "еще", "сейчас"]
        )
    ));
    atoms.insert(format!(
        "state_stop_marker:{}",
        contains_any(&lower, &["стоп", "стой", "останов", "блок"])
    ));
    atoms.into_iter().collect()
}

fn codex_history_action_atoms(prompt: &str) -> Vec<String> {
    let lower = prompt.to_lowercase();
    let mut atoms = BTreeSet::new();
    if contains_any(&lower, &["стоп", "стой", "останов", "блок"]) {
        atoms.insert("action_family:control_stop".to_owned());
    }
    if contains_any(
        &lower,
        &[
            "делай",
            "сделай",
            "реализ",
            "добав",
            "исправ",
            "чини",
            "пиши",
            "код",
            "implement",
            "fix",
        ],
    ) {
        atoms.insert("action_family:edit_or_build".to_owned());
    }
    if contains_any(
        &lower,
        &[
            "проверь",
            "провер",
            "тест",
            "cargo check",
            "cargo clippy",
            "cargo test",
            "fmt",
            "test",
        ],
    ) {
        atoms.insert("action_family:run_check".to_owned());
    }
    if contains_any(
        &lower,
        &["читай", "прочитай", "посмотри", "открой", "inspect", "read"],
    ) {
        atoms.insert("action_family:read_inspect".to_owned());
    }
    if contains_any(&lower, &["git", "коммит", "commit", "пуш", "push"]) {
        atoms.insert("action_family:git_control".to_owned());
    }
    if contains_any(
        &lower,
        &[
            "отчет",
            "отчёт",
            "report",
            "метрик",
            "latency",
            "p99",
            "эконом",
        ],
    ) {
        atoms.insert("action_family:metrics_report".to_owned());
    }
    if contains_any(
        &lower,
        &[
            "сервер",
            "демон",
            "daemon",
            "http",
            "vps",
            "systemd",
            "worker",
            "runtime",
        ],
    ) {
        atoms.insert("action_family:serving_ops".to_owned());
    }
    if contains_any(&lower, &["план", "goal", "цель", "roadmap"]) {
        atoms.insert("action_family:planning".to_owned());
    }
    if contains_any(
        &lower,
        &[
            "почему",
            "объясни",
            "что такое",
            "расскажи",
            "why",
            "explain",
        ],
    ) {
        atoms.insert("action_family:answer_or_explain".to_owned());
    }
    if contains_any(
        &lower,
        &["nando", "wave", "llmwave", "фурье", "phase", ".nwpc"],
    ) {
        atoms.insert("domain_family:nando_wave".to_owned());
    }
    if atoms.is_empty() {
        atoms.insert("action_family:dialogue_or_unknown".to_owned());
    }
    atoms.into_iter().collect()
}

fn codex_history_tool_atoms(prompt: &str) -> Vec<String> {
    let lower = prompt.to_lowercase();
    let mut atoms = BTreeSet::new();
    for (needle, atom) in [
        ("cargo", "tool_mention:cargo"),
        ("git", "tool_mention:git"),
        ("rg ", "tool_mention:rg"),
        ("jq", "tool_mention:jq"),
        ("python", "tool_mention:python"),
        ("ssh", "tool_mention:ssh"),
        ("curl", "tool_mention:curl"),
        ("systemd", "tool_mention:systemd"),
        ("http", "tool_mention:http"),
        ("nginx", "tool_mention:nginx"),
        ("codex", "tool_mention:codex"),
        ("nanda", "tool_mention:nanda"),
    ] {
        if lower.contains(needle) {
            atoms.insert(atom.to_owned());
        }
    }
    atoms.into_iter().collect()
}

fn codex_history_route_hint_atoms(action_atoms: &[String]) -> Vec<String> {
    let mut atoms = BTreeSet::new();
    for action in action_atoms {
        match action.as_str() {
            "action_family:control_stop" => {
                atoms.insert("route_hint:agent_control".to_owned());
            }
            "action_family:edit_or_build" => {
                atoms.insert("route_hint:edit_patch_small".to_owned());
            }
            "action_family:run_check" => {
                atoms.insert("route_hint:test_output_parse".to_owned());
            }
            "action_family:read_inspect" => {
                atoms.insert("route_hint:read_inspect".to_owned());
            }
            "action_family:git_control" => {
                atoms.insert("route_hint:git_control".to_owned());
            }
            "action_family:metrics_report" => {
                atoms.insert("route_hint:metrics_report".to_owned());
            }
            "action_family:serving_ops" => {
                atoms.insert("route_hint:serving_ops".to_owned());
            }
            "action_family:planning" => {
                atoms.insert("route_hint:planning_next_step".to_owned());
            }
            "action_family:answer_or_explain" => {
                atoms.insert("route_hint:answer_evidence".to_owned());
            }
            _ => {}
        }
    }
    atoms.into_iter().collect()
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn atom_count_reports(counts: BTreeMap<String, usize>, limit: usize) -> Vec<AtomCountReport> {
    let mut rows = counts
        .into_iter()
        .map(|(atom, count)| AtomCountReport { atom, count })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.atom.cmp(&right.atom))
    });
    rows.truncate(limit);
    rows
}

fn phase_atom_string_vec(row: &serde_json::Value, key: &str) -> Vec<String> {
    let direct = json_string_vec(row.get(key));
    if !direct.is_empty() {
        return direct;
    }
    row.get("atom_groups")
        .and_then(serde_json::Value::as_object)
        .map_or_else(Vec::new, |groups| json_string_vec(groups.get(key)))
}

fn phase_atom_action_families(action_atoms: &[String]) -> Vec<String> {
    action_atoms
        .iter()
        .filter(|atom| atom.starts_with("action_family:"))
        .cloned()
        .collect()
}

fn phase_atom_task_name_from_action_family(action_family: &str) -> String {
    action_family
        .strip_prefix("action_family:")
        .unwrap_or(action_family)
        .replace(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_', "_")
}

fn phase_atom_base_action_family(action_family_or_bucket: &str) -> &str {
    action_family_or_bucket
        .split_once("::")
        .map(|(base, _)| base)
        .unwrap_or(action_family_or_bucket)
}

fn phase_atom_live_self_mining_task_name(bucket_key: &str) -> String {
    let mut prefix = sanitize_file_stem(
        &phase_atom_task_name_from_action_family(bucket_key)
            .chars()
            .take(48)
            .collect::<String>(),
    );
    while prefix.ends_with('-') || prefix.ends_with('_') {
        prefix.pop();
    }
    if prefix.is_empty() {
        prefix.push_str("phase_bucket");
    }
    let fingerprint = stable_fingerprint([bucket_key]);
    format!("{prefix}-{fingerprint:016x}")
}

fn phase_atom_live_self_mining_value_score(
    events: &[PhaseAtomBinaryEvent],
    min_class_events: usize,
) -> u128 {
    let positive_events = events
        .iter()
        .filter(|event| event.verified_safe_accept)
        .count();
    if events.len() < min_class_events || positive_events == 0 {
        return 0;
    }
    let exact_cache_flags = exact_cache_hit_flags_phase_atom_binary(events);
    let mut non_exact_events = 0usize;
    let mut non_exact_token_ceiling = 0usize;
    for (event, exact_hit) in events.iter().zip(exact_cache_flags) {
        if !exact_hit {
            non_exact_events += 1;
            non_exact_token_ceiling =
                non_exact_token_ceiling.saturating_add(event.token_cost.total_tokens);
        }
    }
    if non_exact_events == 0 {
        return 0;
    }
    (non_exact_token_ceiling as u128)
        .saturating_mul(events.len() as u128)
        .saturating_add(non_exact_events as u128)
}

fn phase_atom_safe_accept_margin_micro_for_task(
    reference_runtime: &PhaseCenterFlatRuntime,
    offload_runtime: &PhaseCenterOffloadRuntime,
    event: &PhaseAtomBinaryEvent,
    cells: usize,
    task_name: &str,
) -> Result<(i64, bool), String> {
    let safe_accept_vec = phase_atom_binary_event_vector_for_task(event, true, cells, task_name);
    let zero = vec![nando_core::PhaseCenterCell::default(); cells];
    let safe_accept_task = PhaseCenterEvalTask {
        center_index: 0,
        correct_vec: safe_accept_vec.into_boxed_slice(),
        wrong_vec: zero.into_boxed_slice(),
    };
    let reference_safe_micro = margin_to_micro(
        reference_runtime
            .margin(&safe_accept_task)
            .map_err(|error| {
                format!("live self-mining reference safe-accept margin error: {error:?}")
            })?,
    )?;
    let runtime_safe_micro = margin_to_micro(
        offload_runtime
            .runtime()
            .margin(&safe_accept_task)
            .map_err(|error| {
                format!("live self-mining runtime safe-accept margin error: {error:?}")
            })?,
    )?;
    Ok((
        runtime_safe_micro,
        reference_safe_micro != runtime_safe_micro,
    ))
}

fn phase_atom_live_self_mining_class_report(
    action_family: &str,
    events: &[PhaseAtomBinaryEvent],
    compile_candidate: bool,
    config: PhaseAtomLiveSelfMiningClassConfig<'_>,
) -> Result<PhaseAtomLiveSelfMiningClassReport, String> {
    let task_name = phase_atom_live_self_mining_task_name(action_family);
    let exact_cache_flags = exact_cache_hit_flags_phase_atom_binary(events);
    let positive_events = events
        .iter()
        .filter(|event| event.verified_safe_accept)
        .count();
    let negative_events = events.len().saturating_sub(positive_events);
    let exact_cache_hits = exact_cache_flags.iter().filter(|flag| **flag).count();
    let mut total_tokens = 0usize;
    let mut non_exact_token_ceiling = 0usize;
    let mut total_cost_microusd = 0u64;
    let mut token_evidence_missing_events = 0usize;
    let mut cost_evidence_missing_events = 0usize;
    for (event, exact_hit) in events.iter().zip(exact_cache_flags.iter().copied()) {
        total_tokens = total_tokens.saturating_add(event.token_cost.total_tokens);
        total_cost_microusd =
            total_cost_microusd.saturating_add(event.token_cost.total_cost_microusd);
        token_evidence_missing_events += usize::from(event.token_cost.token_evidence_missing);
        cost_evidence_missing_events += usize::from(event.token_cost.cost_evidence_missing);
        if !exact_hit {
            non_exact_token_ceiling =
                non_exact_token_ceiling.saturating_add(event.token_cost.total_tokens);
        }
    }
    let non_exact_events = events.len().saturating_sub(exact_cache_hits);
    let base_background_negative_events = config
        .base_background_events
        .iter()
        .filter(|event| !event.verified_safe_accept)
        .count();
    let verifier_bound =
        positive_events > 0 && (negative_events > 0 || base_background_negative_events > 0);
    let value_score = phase_atom_live_self_mining_value_score(events, config.min_class_events);
    let high_value_candidate = value_score > 0;
    let package_path = config
        .candidate_dir
        .join(format!("{task_name}-live-self-mining.candidate.nwpc"));
    let recommended_verifier = recommended_verifier_capture_for_action_family(action_family);
    let mut report = PhaseAtomLiveSelfMiningClassReport {
        action_family: action_family.to_owned(),
        task_name: task_name.clone(),
        events_seen: events.len(),
        positive_events,
        negative_events,
        exact_cache_hits,
        exact_cache_overlap_milli: per_thousand(exact_cache_hits, events.len()),
        non_exact_events,
        total_tokens,
        non_exact_token_ceiling,
        total_cost_microusd,
        verifier_bound,
        high_value_candidate,
        value_score,
        candidate_package_path: package_path.display().to_string(),
        compiled_quarantine_candidate: false,
        package_fingerprint64: 0,
        package_bytes: 0,
        package_records: 0,
        train_events: 0,
        train_positive_events: 0,
        train_negative_events: 0,
        background_negative_train_events_used: 0,
        background_negative_heldout_events_used: 0,
        heldout_events: 0,
        heldout_positive_events: 0,
        heldout_negative_events: 0,
        heldout_non_exact_positive_events: 0,
        heldout_non_exact_negative_events: 0,
        train_heldout_time_order_ok: false,
        heldout_accuracy_milli: 0,
        heldout_local_operator_calls: 0,
        heldout_fallback_calls: 0,
        false_accepts: 0,
        wrong_wins: 0,
        runtime_margin_parity_mismatches: 0,
        safe_accept_margin_threshold_micro: 0,
        train_safe_accept_max_false_margin_micro: None,
        train_safe_accept_min_true_margin_micro: None,
        train_safe_accept_margin_separation_micro: None,
        min_margin_micro: 0,
        p10_margin_micro: 0,
        median_margin_micro: 0,
        heldout_missed_safe_accepts: 0,
        exact_cache_hits_in_heldout: 0,
        unique_cpu_accepts_over_exact_cache: 0,
        accepted_heldout_decisions: Vec::new(),
        nando_cpu_tokens_saved: 0,
        nando_cpu_cost_saved_microusd: 0,
        token_evidence_missing_events,
        cost_evidence_missing_events,
        accepted_for_shadow_review: false,
        recommended_verifier,
        recommended_next_action: "keep_observing_or_rank_below_top_n".to_owned(),
        rejection_reason: "not_selected_for_compile".to_owned(),
    };
    if !verifier_bound {
        report.recommended_next_action =
            "capture_positive_and_negative_verifier_labels_or_base_family_negative_background"
                .to_owned();
        report.rejection_reason = "missing_positive_or_negative_verifier_label".to_owned();
        return Ok(report);
    }
    if !high_value_candidate {
        report.recommended_next_action =
            "keep_observing_until_frequency_or_non_exact_value_rises".to_owned();
        report.rejection_reason =
            "below_minimum_frequency_or_exact_cache_already_covers".to_owned();
        return Ok(report);
    }
    if !compile_candidate {
        return Ok(report);
    }

    let (train_indices, heldout_indices) =
        phase_atom_binary_time_split_indices(events, config.train_permille);
    report.train_events = train_indices.len();
    report.heldout_events = heldout_indices.len();
    report.train_positive_events = train_indices
        .iter()
        .filter(|index| events[**index].verified_safe_accept)
        .count();
    report.train_negative_events = train_indices
        .len()
        .saturating_sub(report.train_positive_events);
    report.heldout_positive_events = heldout_indices
        .iter()
        .filter(|index| events[**index].verified_safe_accept)
        .count();
    report.heldout_negative_events = heldout_indices
        .len()
        .saturating_sub(report.heldout_positive_events);
    for &event_index in &heldout_indices {
        if !exact_cache_flags[event_index] {
            if events[event_index].verified_safe_accept {
                report.heldout_non_exact_positive_events += 1;
            } else {
                report.heldout_non_exact_negative_events += 1;
            }
        }
    }
    report.train_heldout_time_order_ok =
        phase_atom_binary_time_order_ok(events, &train_indices, &heldout_indices);
    if train_indices.is_empty() || heldout_indices.is_empty() {
        report.recommended_next_action = "collect_more_time_ordered_events".to_owned();
        report.rejection_reason = "empty_train_or_heldout_split".to_owned();
        return Ok(report);
    }
    let (_, train_time_max) = phase_atom_binary_time_range(events, &train_indices);
    let mut all_background_negative_events = Vec::new();
    let mut seen_background_negatives = BTreeSet::new();
    for event in config.base_background_events {
        if event.verified_safe_accept {
            continue;
        }
        let background_key = format!(
            "{}\n{}\n{}",
            event.event_timestamp, event.request_fingerprint, event.exact_cache_key
        );
        if seen_background_negatives.insert(background_key) {
            all_background_negative_events.push(event);
        }
    }
    let mut background_negative_train_events = Vec::new();
    let mut background_negative_heldout_events = Vec::new();
    let train_split_has_both_labels =
        phase_atom_binary_split_has_both_labels(events, &train_indices);
    if !train_split_has_both_labels
        && report.train_positive_events > 0
        && report.train_negative_events == 0
    {
        let timestamp_train_events = all_background_negative_events
            .iter()
            .copied()
            .filter(|event| event.event_timestamp <= train_time_max)
            .collect::<Vec<_>>();
        let timestamp_heldout_events = all_background_negative_events
            .iter()
            .copied()
            .filter(|event| event.event_timestamp > train_time_max)
            .collect::<Vec<_>>();
        if !timestamp_train_events.is_empty() && !timestamp_heldout_events.is_empty() {
            background_negative_train_events = timestamp_train_events;
            background_negative_heldout_events = timestamp_heldout_events;
        } else if all_background_negative_events.len() >= 2 {
            let split_at = ((all_background_negative_events.len() * config.train_permille) / 1000)
                .clamp(1, all_background_negative_events.len() - 1);
            background_negative_train_events
                .extend_from_slice(&all_background_negative_events[..split_at]);
            background_negative_heldout_events
                .extend_from_slice(&all_background_negative_events[split_at..]);
        } else {
            background_negative_train_events.extend_from_slice(&all_background_negative_events);
        }
        report.background_negative_train_events_used = background_negative_train_events.len();
        report.background_negative_heldout_events_used = background_negative_heldout_events.len();
    }
    if !train_split_has_both_labels && background_negative_train_events.is_empty() {
        report.recommended_next_action =
            "collect_more_train_window_negative_and_positive_labels_or_base_family_negatives"
                .to_owned();
        report.rejection_reason = "train_window_missing_label_diversity".to_owned();
        return Ok(report);
    }
    let heldout_split_has_both_labels =
        phase_atom_binary_split_has_both_labels(events, &heldout_indices);
    if !heldout_split_has_both_labels && background_negative_heldout_events.is_empty() {
        report.recommended_next_action =
            "collect_more_heldout_window_negative_and_positive_labels_or_base_family_negative_background"
                .to_owned();
        report.rejection_reason = "heldout_window_missing_label_diversity".to_owned();
        return Ok(report);
    }

    let mut compiler = PhaseCenterCompiler::new(config.cells, 1).map_err(|error| {
        format!("live self-mining compiler error for {action_family}: {error:?}")
    })?;
    for &event_index in &train_indices {
        let event = &events[event_index];
        let safe_accept_vec =
            phase_atom_binary_event_vector_for_task(event, true, config.cells, &task_name);
        if event.verified_safe_accept {
            compiler
                .add_positive_vector(0, &safe_accept_vec)
                .map_err(|error| format!("live self-mining positive update error: {error:?}"))?;
        } else {
            compiler
                .add_negative_vector(0, &safe_accept_vec)
                .map_err(|error| format!("live self-mining negative update error: {error:?}"))?;
        }
    }
    for event in &background_negative_train_events {
        let safe_accept_vec =
            phase_atom_binary_event_vector_for_task(event, true, config.cells, &task_name);
        compiler
            .add_negative_vector(0, &safe_accept_vec)
            .map_err(|error| {
                format!("live self-mining base-family negative update error: {error:?}")
            })?;
    }
    let reference_runtime = compiler
        .compile()
        .map_err(|error| format!("live self-mining compile error: {error:?}"))?;
    let package_bytes = reference_runtime
        .to_bytes()
        .map_err(|error| format!("live self-mining package serialization error: {error:?}"))?;
    write_binary_file(&package_path, &package_bytes)?;
    let read_package = std::fs::read(&package_path).map_err(|error| {
        format!(
            "failed to read live self-mining package '{}': {error}",
            package_path.display()
        )
    })?;
    if read_package != package_bytes {
        return Err(format!(
            "live self-mining package '{}' readback mismatch",
            package_path.display()
        ));
    }
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&read_package)
        .map_err(|error| format!("live self-mining package inspect error: {error:?}"))?;
    let offload_runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &read_package,
        PhaseCenterOffloadPolicy::new(config.margin_threshold_micro)
            .map_err(|error| format!("invalid live self-mining policy: {error:?}"))?,
    )
    .map_err(|error| format!("live self-mining package load error: {error:?}"))?;
    report.compiled_quarantine_candidate = true;
    report.package_fingerprint64 = package_info.fingerprint64;
    report.package_bytes = read_package.len();
    report.package_records = package_info.record_count;

    let mut train_safe_accept_true_margins = Vec::new();
    let mut train_safe_accept_false_margins = Vec::new();
    for &event_index in &train_indices {
        let event = &events[event_index];
        let (safe_accept_margin_micro, parity_mismatch) =
            phase_atom_safe_accept_margin_micro_for_task(
                &reference_runtime,
                &offload_runtime,
                event,
                config.cells,
                &task_name,
            )?;
        report.runtime_margin_parity_mismatches += usize::from(parity_mismatch);
        if event.verified_safe_accept {
            train_safe_accept_true_margins.push(safe_accept_margin_micro);
        } else {
            train_safe_accept_false_margins.push(safe_accept_margin_micro);
        }
    }
    for event in &background_negative_train_events {
        let (safe_accept_margin_micro, parity_mismatch) =
            phase_atom_safe_accept_margin_micro_for_task(
                &reference_runtime,
                &offload_runtime,
                event,
                config.cells,
                &task_name,
            )?;
        report.runtime_margin_parity_mismatches += usize::from(parity_mismatch);
        train_safe_accept_false_margins.push(safe_accept_margin_micro);
    }
    report.train_safe_accept_max_false_margin_micro =
        train_safe_accept_false_margins.iter().max().copied();
    report.train_safe_accept_min_true_margin_micro =
        train_safe_accept_true_margins.iter().min().copied();
    report.train_safe_accept_margin_separation_micro = report
        .train_safe_accept_min_true_margin_micro
        .zip(report.train_safe_accept_max_false_margin_micro)
        .map(|(min_true, max_false)| min_true.saturating_sub(max_false));
    report.safe_accept_margin_threshold_micro = report
        .train_safe_accept_max_false_margin_micro
        .map_or(config.margin_threshold_micro, |max_false| {
            config
                .margin_threshold_micro
                .max(max_false.saturating_add(1))
        });

    let mut margins = Vec::new();
    let mut correct_rows = 0usize;
    for (heldout_position, &event_index) in heldout_indices.iter().enumerate() {
        if heldout_position > 0 && heldout_position % 1000 == 0 {
            println!(
                "  live_self_mining_shadow_events_scored: action_family={} {}/{}",
                action_family,
                heldout_position,
                heldout_indices.len()
            );
        }
        let event = &events[event_index];
        let (runtime_safe_accept_micro, parity_mismatch) =
            phase_atom_safe_accept_margin_micro_for_task(
                &reference_runtime,
                &offload_runtime,
                event,
                config.cells,
                &task_name,
            )?;
        let signed_classification_margin = if event.verified_safe_accept {
            runtime_safe_accept_micro
        } else {
            runtime_safe_accept_micro.saturating_neg()
        };
        report.runtime_margin_parity_mismatches += usize::from(parity_mismatch);
        margins.push(signed_classification_margin);
        correct_rows += usize::from(signed_classification_margin > 0);
        report.wrong_wins += usize::from(signed_classification_margin <= 0);
        let safe_accept_would_local =
            runtime_safe_accept_micro >= report.safe_accept_margin_threshold_micro;
        if event.verified_safe_accept && safe_accept_would_local {
            report.heldout_local_operator_calls += 1;
            let exact_cache_hit = exact_cache_flags[event_index];
            report
                .accepted_heldout_decisions
                .push(PhaseAtomAcceptedDecisionRow {
                    heldout_position,
                    event_timestamp: event.event_timestamp.clone(),
                    request_fingerprint: event.request_fingerprint.clone(),
                    exact_cache_key: event.exact_cache_key.clone(),
                    exact_cache_hit,
                    margin_micro: runtime_safe_accept_micro,
                    token_cost: event.token_cost,
                });
            if !exact_cache_hit {
                report.unique_cpu_accepts_over_exact_cache += 1;
                report.nando_cpu_tokens_saved = report
                    .nando_cpu_tokens_saved
                    .saturating_add(event.token_cost.total_tokens);
                report.nando_cpu_cost_saved_microusd = report
                    .nando_cpu_cost_saved_microusd
                    .saturating_add(event.token_cost.total_cost_microusd);
            }
        } else if !event.verified_safe_accept {
            report.false_accepts += usize::from(safe_accept_would_local);
            report.heldout_fallback_calls += 1;
        } else {
            report.heldout_missed_safe_accepts += 1;
            report.heldout_fallback_calls += 1;
        }
        report.exact_cache_hits_in_heldout += usize::from(exact_cache_flags[event_index]);
    }
    for (background_position, event) in background_negative_heldout_events.iter().enumerate() {
        if background_position > 0 && background_position % 1000 == 0 {
            println!(
                "  live_self_mining_background_negative_shadow_events_scored: action_family={} {}/{}",
                action_family,
                background_position,
                background_negative_heldout_events.len()
            );
        }
        let (runtime_safe_accept_micro, parity_mismatch) =
            phase_atom_safe_accept_margin_micro_for_task(
                &reference_runtime,
                &offload_runtime,
                event,
                config.cells,
                &task_name,
            )?;
        let signed_classification_margin = runtime_safe_accept_micro.saturating_neg();
        report.runtime_margin_parity_mismatches += usize::from(parity_mismatch);
        margins.push(signed_classification_margin);
        correct_rows += usize::from(signed_classification_margin > 0);
        report.wrong_wins += usize::from(signed_classification_margin <= 0);
        let safe_accept_would_local =
            runtime_safe_accept_micro >= report.safe_accept_margin_threshold_micro;
        report.false_accepts += usize::from(safe_accept_would_local);
        report.heldout_fallback_calls += 1;
    }
    margins.sort_unstable();
    let heldout_eval_rows = heldout_indices
        .len()
        .saturating_add(background_negative_heldout_events.len());
    report.heldout_accuracy_milli = per_thousand(correct_rows, heldout_eval_rows);
    report.min_margin_micro = margins.first().copied().unwrap_or(0);
    report.p10_margin_micro = percentile_i64(&margins, 10);
    report.median_margin_micro = percentile_i64(&margins, 50);
    report.accepted_for_shadow_review = report.train_heldout_time_order_ok
        && report.heldout_local_operator_calls > 0
        && report.heldout_fallback_calls > 0
        && report.unique_cpu_accepts_over_exact_cache > 0
        && report.false_accepts == 0
        && report.runtime_margin_parity_mismatches == 0;
    report.rejection_reason = if report.accepted_for_shadow_review {
        "accepted_for_self_mined_shadow_review_partial_recall_product_accept_disabled".to_owned()
    } else if !report.train_heldout_time_order_ok {
        "train_heldout_time_order_failed".to_owned()
    } else if report.false_accepts > 0 {
        "false_accepts_detected".to_owned()
    } else if report.runtime_margin_parity_mismatches > 0 {
        "runtime_margin_parity_mismatches".to_owned()
    } else if report.heldout_local_operator_calls == 0 {
        "no_heldout_local_operator_calls".to_owned()
    } else if report.unique_cpu_accepts_over_exact_cache == 0 {
        "no_unique_cpu_accepts_over_exact_cache".to_owned()
    } else if report.heldout_fallback_calls == 0 {
        "no_natural_fallback_rows_in_heldout".to_owned()
    } else {
        "self_mining_shadow_gate_failed".to_owned()
    };
    report.recommended_next_action = if report.accepted_for_shadow_review {
        "run_promotion_audit_then_live_shadow_manifest_local_accept_still_disabled".to_owned()
    } else {
        "keep_quarantine_and_collect_more_stream_events".to_owned()
    };
    Ok(report)
}

fn phase_atom_state_action_bucket_key(
    action_family: &str,
    request_atoms: &[String],
    state_atoms: &[String],
    tool_atoms: &[String],
    route_hint_atoms: &[String],
) -> String {
    if let Some(candidate_atom) = request_atoms
        .iter()
        .chain(route_hint_atoms)
        .find(|atom| atom.starts_with("request_subcenter_candidate:"))
    {
        return format!("{action_family}::{candidate_atom}");
    }

    let mut selected = BTreeSet::new();
    for atom in request_atoms
        .iter()
        .chain(state_atoms)
        .chain(tool_atoms)
        .chain(route_hint_atoms)
    {
        if phase_atom_bucket_selector(atom) {
            selected.insert(atom.clone());
        }
    }
    let signature = selected.into_iter().take(14).collect::<Vec<_>>().join("|");
    if signature.is_empty() {
        action_family.to_owned()
    } else {
        format!("{action_family}::{signature}")
    }
}

fn phase_atom_bucket_selector(atom: &str) -> bool {
    [
        "request_command_kind:",
        "request_command_arg_band:",
        "request_cwd_kind:",
        "request_route_family:",
        "request_has_shadow_request:",
        "request_char_band:",
        "request_line_count_band:",
        "request_word_count_band:",
        "request_has_code_fence:",
        "request_has_json_shape:",
        "request_has_path:",
        "request_has_question:",
        "topic:",
        "domain_family:",
        "request_mentions_ext:",
        "state_session_turn_band:",
        "state_followup_marker:",
        "state_stop_marker:",
        "state_cwd_kind:",
        "state_exit_code_band:",
        "state_output_char_band:",
        "state_output_line_band:",
        "state_output_has_error_marker:",
        "state_output_has_warning_marker:",
        "state_output_marker:",
        "shadow_active_fringe_len_band:",
        "shadow_slot_count_band:",
        "tool_mention:",
        "tool_command_kind:",
        "tool_command_shell_family:",
        "tool_check_kind:",
        "route_hint:",
        "route_key:",
    ]
    .iter()
    .any(|prefix| atom.starts_with(prefix))
}

fn parse_phase_atom_binary_event_for_action(
    row: &serde_json::Value,
    index: usize,
    action_family_atom: &str,
    task_name: &str,
) -> Option<PhaseAtomBinaryEvent> {
    let verified_safe_accept = row.get("verified_safe_accept")?.as_bool()?;
    let action_atoms = phase_atom_string_vec(row, "action_atoms");
    if !action_atoms.iter().any(|atom| atom == action_family_atom) {
        return None;
    }

    let mut base_atoms = BTreeSet::new();
    for atom in phase_atom_string_vec(row, "request_atoms")
        .into_iter()
        .chain(phase_atom_string_vec(row, "state_atoms"))
        .chain(action_atoms)
        .chain(phase_atom_string_vec(row, "tool_atoms"))
        .chain(phase_atom_string_vec(row, "route_hint_atoms"))
    {
        if !atom.starts_with("output_hash64:")
            && !atom.starts_with("verifier_label:")
            && !atom.starts_with("verified_safe_accept:")
        {
            base_atoms.insert(atom);
        }
    }
    if base_atoms.is_empty() {
        return None;
    }
    base_atoms.insert(format!("phase_atom_binary_task:{task_name}_verifier_bound"));

    let request_fingerprint = json_string(row, &["request_fingerprint"])
        .unwrap_or_else(|| format!("phase_atom_binary_request_index:{index}"));
    let external_provider_correlation_keys = phase_atom_external_provider_correlation_keys(row);
    let exact_cache_key =
        json_string(row, &["exact_cache_key"]).unwrap_or_else(|| request_fingerprint.clone());
    let token_cost = phase_atom_binary_token_cost(row);
    let event_timestamp = json_string(row, &["event_timestamp"])
        .or_else(|| json_string(row, &["trace_id"]))
        .unwrap_or_else(|| format!("unknown-time-{index:08}"));
    Some(PhaseAtomBinaryEvent {
        event_timestamp,
        request_fingerprint,
        external_provider_correlation_keys,
        verified_safe_accept,
        base_atoms: base_atoms.into_iter().collect(),
        exact_cache_key,
        token_cost,
    })
}

fn phase_atom_external_provider_correlation_keys(row: &serde_json::Value) -> Vec<String> {
    let mut keys = Vec::new();
    let array_paths: &[&[&str]] = &[
        &["external_provider_correlation_keys"],
        &["provider_correlation_keys"],
        &["metadata", "external_provider_correlation_keys"],
        &["metadata", "provider_correlation_keys"],
        &["provider", "correlation_keys"],
        &["llm_call", "external_provider_correlation_keys"],
        &["llm_call", "provider_correlation_keys"],
        &["response", "external_provider_correlation_keys"],
        &["response", "provider_correlation_keys"],
    ];
    for path in array_paths {
        keys.extend(
            json_string_vec(json_at(row, path))
                .into_iter()
                .filter(|key| !key.is_empty()),
        );
    }

    let paths: &[(&str, &[&str])] = &[
        ("provider_request_id", &["provider_request_id"]),
        ("provider_request_id", &["provider", "request_id"]),
        ("provider_request_id", &["provider", "provider_request_id"]),
        ("provider_request_id", &["metadata", "provider_request_id"]),
        ("provider_request_id", &["request", "provider_request_id"]),
        ("provider_request_id", &["llm_call", "provider_request_id"]),
        ("provider_request_id", &["llm_call", "request_id"]),
        ("provider_response_id", &["provider_response_id"]),
        ("provider_response_id", &["provider", "response_id"]),
        (
            "provider_response_id",
            &["provider", "provider_response_id"],
        ),
        (
            "provider_response_id",
            &["metadata", "provider_response_id"],
        ),
        (
            "provider_response_id",
            &["response", "provider_response_id"],
        ),
        ("provider_response_id", &["response", "response_id"]),
        ("provider_response_id", &["response", "id"]),
        (
            "provider_response_id",
            &["llm_call", "provider_response_id"],
        ),
        ("provider_response_id", &["llm_call", "response_id"]),
        ("provider_trace_id", &["provider_trace_id"]),
        ("provider_trace_id", &["provider", "trace_id"]),
        ("provider_trace_id", &["provider", "provider_trace_id"]),
        ("provider_trace_id", &["metadata", "provider_trace_id"]),
        (
            "external_provider_request_id",
            &["external_provider_request_id"],
        ),
        (
            "external_provider_request_id",
            &["metadata", "external_provider_request_id"],
        ),
        (
            "external_provider_request_id",
            &["provider", "external_provider_request_id"],
        ),
        ("openai_request_id", &["openai_request_id"]),
        ("openai_request_id", &["metadata", "openai_request_id"]),
        ("openai_request_id", &["provider", "openai_request_id"]),
        ("openai_request_id", &["llm_call", "openai_request_id"]),
        ("anthropic_request_id", &["anthropic_request_id"]),
        (
            "anthropic_request_id",
            &["metadata", "anthropic_request_id"],
        ),
        (
            "anthropic_request_id",
            &["provider", "anthropic_request_id"],
        ),
        (
            "anthropic_request_id",
            &["llm_call", "anthropic_request_id"],
        ),
        ("custom_id", &["custom_id"]),
        ("custom_id", &["metadata", "custom_id"]),
        ("custom_id", &["provider", "custom_id"]),
        ("custom_id", &["llm_call", "custom_id"]),
    ];
    for (label, path) in paths {
        if let Some(value) = json_string(row, path).filter(|value| !value.is_empty()) {
            keys.push(format!("{label}:{value}"));
        }
    }
    keys.sort();
    keys.dedup();
    keys
}

fn parse_phase_atom_binary_event(
    row: &serde_json::Value,
    index: usize,
) -> Option<PhaseAtomBinaryEvent> {
    parse_phase_atom_binary_event_for_action(row, index, "action_family:run_check", "run_check")
}

fn phase_atom_binary_token_cost(row: &serde_json::Value) -> GenericTokenCost {
    let direct = generic_token_cost_from_row(row);
    let total_tokens = json_at(row, &["token_cost", "total_tokens"])
        .and_then(serde_json::Value::as_u64)
        .map_or(direct.total_tokens, |value| value as usize);
    let total_cost_microusd = json_at(row, &["token_cost", "total_cost_microusd"])
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(direct.total_cost_microusd);
    let token_evidence_missing = json_at(row, &["token_cost", "token_evidence_missing"])
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(total_tokens == 0);
    let cost_evidence_missing = json_at(row, &["token_cost", "cost_evidence_missing"])
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(total_cost_microusd == 0);
    GenericTokenCost {
        total_tokens,
        total_cost_microusd,
        evidence_missing: token_evidence_missing && cost_evidence_missing,
        token_evidence_missing,
        cost_evidence_missing,
    }
}

fn phase_atom_binary_train_heldout_indices(
    events: &[PhaseAtomBinaryEvent],
) -> (Vec<usize>, Vec<usize>) {
    let mut by_label: BTreeMap<bool, Vec<usize>> = BTreeMap::new();
    for (index, event) in events.iter().enumerate() {
        by_label
            .entry(event.verified_safe_accept)
            .or_default()
            .push(index);
    }

    let mut train = Vec::new();
    let mut heldout = Vec::new();
    for indices in by_label.values() {
        if indices.len() == 1 {
            train.push(indices[0]);
            continue;
        }
        let mut label_train = Vec::new();
        let mut label_heldout = Vec::new();
        for (position, index) in indices.iter().copied().enumerate() {
            if position % 5 == 4 {
                label_heldout.push(index);
            } else {
                label_train.push(index);
            }
        }
        if label_heldout.is_empty()
            && let Some(index) = label_train.pop()
        {
            label_heldout.push(index);
        }
        if label_train.is_empty()
            && let Some(index) = label_heldout.pop()
        {
            label_train.push(index);
        }
        train.extend(label_train);
        heldout.extend(label_heldout);
    }
    train.sort_unstable();
    heldout.sort_unstable();
    (train, heldout)
}

fn phase_atom_binary_time_split_indices(
    events: &[PhaseAtomBinaryEvent],
    train_permille: usize,
) -> (Vec<usize>, Vec<usize>) {
    let mut ordered = events.iter().enumerate().collect::<Vec<_>>();
    ordered.sort_by(|(left_index, left), (right_index, right)| {
        left.event_timestamp
            .cmp(&right.event_timestamp)
            .then_with(|| left_index.cmp(right_index))
    });
    let train_count =
        (ordered.len() * train_permille / 1000).clamp(1, ordered.len().saturating_sub(1));
    let mut train = ordered[..train_count]
        .iter()
        .map(|(index, _)| *index)
        .collect::<Vec<_>>();
    let mut heldout = ordered[train_count..]
        .iter()
        .map(|(index, _)| *index)
        .collect::<Vec<_>>();
    train.sort_unstable();
    heldout.sort_unstable();
    (train, heldout)
}

fn phase_atom_binary_split_has_both_labels(
    events: &[PhaseAtomBinaryEvent],
    indices: &[usize],
) -> bool {
    let mut has_positive = false;
    let mut has_negative = false;
    for &index in indices {
        if events[index].verified_safe_accept {
            has_positive = true;
        } else {
            has_negative = true;
        }
    }
    has_positive && has_negative
}

fn phase_atom_binary_time_range(
    events: &[PhaseAtomBinaryEvent],
    indices: &[usize],
) -> (String, String) {
    let mut times = indices
        .iter()
        .map(|index| events[*index].event_timestamp.as_str())
        .collect::<Vec<_>>();
    times.sort_unstable();
    (
        times.first().copied().unwrap_or_default().to_owned(),
        times.last().copied().unwrap_or_default().to_owned(),
    )
}

fn phase_atom_binary_time_order_ok(
    events: &[PhaseAtomBinaryEvent],
    train_indices: &[usize],
    heldout_indices: &[usize],
) -> bool {
    let (_, train_max) = phase_atom_binary_time_range(events, train_indices);
    let (heldout_min, _) = phase_atom_binary_time_range(events, heldout_indices);
    !train_max.is_empty() && !heldout_min.is_empty() && train_max <= heldout_min
}

fn phase_atom_binary_event_vector_for_task(
    event: &PhaseAtomBinaryEvent,
    candidate_safe_accept: bool,
    cells: usize,
    task_name: &str,
) -> Vec<nando_core::PhaseCenterCell> {
    let candidate_label = if candidate_safe_accept {
        "pass"
    } else {
        "not_pass"
    };
    let mut atoms = event
        .base_atoms
        .iter()
        .filter(|atom| !atom.starts_with("phase_atom_binary_task:"))
        .cloned()
        .collect::<Vec<_>>();
    atoms.push(format!("phase_atom_binary_task:{task_name}_verifier_bound"));
    atoms.push(format!("candidate_result_label:{candidate_label}"));
    atoms.push(format!(
        "candidate_verified_safe_accept:{candidate_safe_accept}"
    ));
    atoms.push(format!(
        "candidate_action_result_pair:{task_name}:{candidate_label}"
    ));
    atoms.push(format!(
        "candidate_label_index:{}",
        usize::from(!candidate_safe_accept)
    ));
    phase_vector_from_atoms(atoms.iter().map(String::as_str), cells)
}

fn phase_atom_binary_event_vector(
    event: &PhaseAtomBinaryEvent,
    candidate_safe_accept: bool,
    cells: usize,
) -> Vec<nando_core::PhaseCenterCell> {
    phase_atom_binary_event_vector_for_task(event, candidate_safe_accept, cells, "run_check")
}

fn phase_delta_vector(
    correct_vec: &[PhaseCenterCell],
    wrong_vec: &[PhaseCenterCell],
) -> Vec<PhaseCenterCell> {
    correct_vec
        .iter()
        .zip(wrong_vec.iter())
        .map(|(correct, wrong)| PhaseCenterCell {
            re: correct.re - wrong.re,
            im: correct.im - wrong.im,
        })
        .collect()
}

fn exact_cache_hit_flags_phase_atom_binary(events: &[PhaseAtomBinaryEvent]) -> Vec<bool> {
    let mut seen = BTreeSet::new();
    let mut flags = Vec::with_capacity(events.len());
    for event in events {
        flags.push(!seen.insert(event.exact_cache_key.as_str()));
    }
    flags
}

#[derive(Clone, Copy)]
struct PhaseAtomDaemonActionInput<'a> {
    action_family: &'a str,
    rows_with_verifier_label: usize,
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    verifier_true_over_exact_cache_ceiling: usize,
    expected_tokens_saved_over_exact_cache: usize,
    expected_cost_saved_microusd_over_exact_cache: u64,
    provider_cost_events: usize,
    estimated_cost_events: usize,
    rows_with_shadow_request: usize,
    rows_with_result_atoms: usize,
    rows_ready_for_route_family_mining: usize,
}

fn phase_atom_false_accept_risk(
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    rows_missing_verifier_label: usize,
    route_mining_ready: bool,
) -> &'static str {
    if verifier_false_rows == 0 {
        "high_no_negative_verifier_evidence"
    } else if verifier_true_rows == 0 {
        "high_no_positive_verifier_evidence"
    } else if rows_missing_verifier_label > verifier_true_rows.saturating_add(verifier_false_rows) {
        "medium_many_unverified_rows"
    } else if !route_mining_ready {
        "medium_not_route_mining_ready"
    } else {
        "bounded_by_positive_negative_verifier_evidence"
    }
}

fn phase_atom_daemon_next_action(input: PhaseAtomDaemonActionInput<'_>) -> &'static str {
    if input.action_family == "action_family:dialogue_or_unknown" {
        "split_unknown_action_family_before_mining"
    } else if input.rows_with_verifier_label == 0 {
        "capture_deterministic_verifier_before_mining"
    } else if input.verifier_false_rows == 0 {
        "collect_negative_verifier_rows_before_shadow"
    } else if input.verifier_true_rows == 0 {
        "collect_positive_verifier_rows_before_shadow"
    } else if input.verifier_true_over_exact_cache_ceiling == 0 {
        "deprioritize_exact_cache_overlap_until_unique_accepts_exist"
    } else if input.expected_tokens_saved_over_exact_cache == 0 {
        "attach_token_meter_before_value_ranking"
    } else if input.expected_cost_saved_microusd_over_exact_cache == 0 {
        "attach_cost_meter_before_money_ranking"
    } else if input.provider_cost_events == 0 && input.estimated_cost_events > 0 {
        "run_shadow_with_internal_estimate_and_request_provider_billing"
    } else if input.rows_with_result_atoms == 0 {
        "capture_result_atoms_before_phase_center_compile"
    } else if input.rows_with_shadow_request == 0 {
        "attach_shadow_request_payload_before_existing_score_path"
    } else if input.rows_ready_for_route_family_mining == 0 {
        "add_state_action_result_atoms_before_route_family_mining"
    } else {
        "run_verifier_bound_phase_center_shadow_mining"
    }
}

fn recommended_verifier_capture_for_action_family(action_family: &str) -> &'static str {
    match action_family {
        "action_family:edit_or_build" => "capture_git_diff_or_file_change_verifier",
        "action_family:tool_status" => "capture_exec_command_status_and_output_shape_verifier",
        "action_family:run_check" => "capture_tool_output_status_verifier",
        "action_family:planning" => "capture_goal_state_transition_verifier",
        "action_family:control_stop" => "capture_agent_control_state_verifier",
        "action_family:read_inspect" => "capture_file_presence_or_excerpt_hash_verifier",
        "action_family:serving_ops" => "capture_http_health_metrics_or_systemd_status_verifier",
        "action_family:git_control" => "capture_git_status_or_git_command_result_verifier",
        "action_family:metrics_report" => "capture_report_file_metric_assertion_verifier",
        "action_family:answer_or_explain" => "capture_cited_artifact_evidence_verifier",
        "action_family:dialogue_or_unknown" => "split_action_family_before_verifier",
        _ => "define_external_verifier_before_compile",
    }
}

fn verifier_needed_recommended_next_action(state: &PhaseAtomActionFamilyState) -> &'static str {
    if state.action_family == "action_family:dialogue_or_unknown" {
        "split_unknown_bucket_before_verifier_capture"
    } else if state.rows_with_verifier_label == 0 {
        "attach_result_verifier_capture_before_phase_center_compile"
    } else if state.verifier_true_over_exact_cache_ceiling == 0 {
        "collect_verified_safe_accepts_over_exact_cache_before_scaling"
    } else if state.rows_with_result_atoms == 0 {
        "capture_result_atoms_before_phase_center_scaling"
    } else if state.rows_with_shadow_request == 0 {
        "attach_shadow_request_payload_for_existing_score_path"
    } else if state.rows_ready_for_route_family_mining == 0 {
        "add_state_action_result_labels_before_route_family_mining"
    } else {
        "eligible_for_shadow_phase_center_review"
    }
}

fn collect_session_jsonl_files(root: &Path, out: &mut Vec<SessionFileEntry>) -> Result<(), String> {
    let entries = std::fs::read_dir(root)
        .map_err(|error| format!("failed to read sessions dir '{}': {error}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to read sessions dir entry under '{}': {error}",
                root.display()
            )
        })?;
        let path = entry.path();
        let metadata = entry.metadata().map_err(|error| {
            format!(
                "failed to read sessions metadata '{}': {error}",
                path.display()
            )
        })?;
        if metadata.is_dir() {
            collect_session_jsonl_files(&path, out)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
            out.push(SessionFileEntry {
                path,
                modified_ms: metadata
                    .modified()
                    .ok()
                    .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |duration| duration.as_millis()),
            });
        }
    }
    Ok(())
}

fn latest_codex_session_file(root: &Path) -> Result<PathBuf, String> {
    let mut session_files = Vec::new();
    collect_session_jsonl_files(root, &mut session_files)?;
    session_files
        .into_iter()
        .max_by(|left, right| {
            left.modified_ms
                .cmp(&right.modified_ms)
                .then_with(|| left.path.cmp(&right.path))
        })
        .map(|entry| entry.path)
        .ok_or_else(|| format!("no Codex session jsonl files under '{}'", root.display()))
}

fn parse_session_run_check_event(
    session_id: &str,
    timestamp: Option<&str>,
    path: &Path,
    payload: &serde_json::Map<String, serde_json::Value>,
) -> Option<SessionRunCheckEvent> {
    let command = payload
        .get("command")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .or_else(|| json_field_string(payload.get("cmd")))?;
    if !is_run_check_command(&command) {
        return None;
    }
    let output = json_field_string(payload.get("aggregated_output"))
        .or_else(|| json_field_string(payload.get("stdout")))
        .unwrap_or_default();
    let exit_code = payload
        .get("exit_code")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1);
    let (label, evidence, unknown_failure) =
        classify_run_check_output(&command, &output, exit_code);
    Some(SessionRunCheckEvent {
        session_id: session_id.to_owned(),
        turn_id: json_field_string(payload.get("turn_id")).unwrap_or_else(|| "unknown_turn".into()),
        timestamp: timestamp.unwrap_or_default().to_owned(),
        path: path.to_path_buf(),
        command,
        cwd: json_field_string(payload.get("cwd")).unwrap_or_else(|| "unknown_cwd".into()),
        output,
        exit_code,
        label,
        evidence,
        unknown_failure,
    })
}

fn parse_session_tool_status_event(
    session_id: &str,
    timestamp: Option<&str>,
    path: &Path,
    payload: &serde_json::Map<String, serde_json::Value>,
) -> Option<SessionRunCheckEvent> {
    let command = payload
        .get("command")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .or_else(|| json_field_string(payload.get("cmd")))?;
    if command.trim().is_empty() {
        return None;
    }
    let output = json_field_string(payload.get("aggregated_output"))
        .or_else(|| json_field_string(payload.get("stdout")))
        .unwrap_or_default();
    let exit_code = payload
        .get("exit_code")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1);
    let (label, evidence, unknown_failure) =
        classify_tool_status_output(&command, &output, exit_code);
    Some(SessionRunCheckEvent {
        session_id: session_id.to_owned(),
        turn_id: json_field_string(payload.get("turn_id")).unwrap_or_else(|| "unknown_turn".into()),
        timestamp: timestamp.unwrap_or_default().to_owned(),
        path: path.to_path_buf(),
        command,
        cwd: json_field_string(payload.get("cwd")).unwrap_or_else(|| "unknown_cwd".into()),
        output,
        exit_code,
        label,
        evidence,
        unknown_failure,
    })
}

fn session_tool_turn_id(payload: &serde_json::Map<String, serde_json::Value>) -> String {
    json_string(
        &serde_json::Value::Object(payload.clone()),
        &["internal_chat_message_metadata_passthrough", "turn_id"],
    )
    .or_else(|| json_field_string(payload.get("turn_id")))
    .unwrap_or_else(|| "unknown_turn".to_owned())
}

fn parse_session_tool_call_meta(
    payload: &serde_json::Map<String, serde_json::Value>,
) -> Option<(String, SessionToolCallMeta)> {
    let payload_type = payload.get("type").and_then(serde_json::Value::as_str)?;
    let call_id = json_field_string(payload.get("call_id"))?;
    let tool_name = json_field_string(payload.get("name")).unwrap_or_else(|| "unknown_tool".into());
    let turn_id = session_tool_turn_id(payload);
    if payload_type == "function_call" {
        let args = json_field_string(payload.get("arguments"))
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .unwrap_or(serde_json::Value::Null);
        let command = if tool_name == "exec_command" {
            json_string(&args, &["cmd"]).unwrap_or_else(|| "exec_command".to_owned())
        } else if tool_name == "write_stdin" {
            let session_id = json_at(&args, &["session_id"])
                .and_then(serde_json::Value::as_i64)
                .map_or_else(|| "unknown".to_owned(), |value| value.to_string());
            format!("write_stdin session:{session_id}")
        } else {
            tool_name.clone()
        };
        let cwd = json_string(&args, &["workdir"]).unwrap_or_else(|| "unknown_cwd".to_owned());
        return Some((
            call_id,
            SessionToolCallMeta {
                turn_id,
                command,
                cwd,
            },
        ));
    }
    if payload_type == "custom_tool_call" {
        return Some((
            call_id,
            SessionToolCallMeta {
                turn_id,
                command: tool_name,
                cwd: "unknown_cwd".to_owned(),
            },
        ));
    }
    None
}

fn parse_session_planning_call_meta(
    payload: &serde_json::Map<String, serde_json::Value>,
) -> Option<(String, SessionPlanningCallMeta)> {
    if payload.get("type").and_then(serde_json::Value::as_str) != Some("function_call") {
        return None;
    }
    if json_field_string(payload.get("name")).as_deref() != Some("update_plan") {
        return None;
    }
    let call_id = json_field_string(payload.get("call_id"))?;
    let arguments = json_field_string(payload.get("arguments")).unwrap_or_default();
    let plan_shape = planning_plan_shape_from_arguments(&arguments);
    Some((
        call_id,
        SessionPlanningCallMeta {
            turn_id: session_tool_turn_id(payload),
            arguments,
            plan_shape,
        },
    ))
}

fn planning_plan_shape_from_arguments(arguments: &str) -> PlanningPlanShape {
    let Ok(args) = serde_json::from_str::<serde_json::Value>(arguments) else {
        return PlanningPlanShape::default();
    };
    let has_explanation = args
        .get("explanation")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|text| !text.trim().is_empty());
    let Some(plan) = args.get("plan").and_then(serde_json::Value::as_array) else {
        return PlanningPlanShape {
            has_explanation,
            ..Default::default()
        };
    };
    let mut shape = PlanningPlanShape {
        step_count: plan.len(),
        has_explanation,
        valid_schema: !plan.is_empty(),
        ..Default::default()
    };
    for item in plan {
        match item.get("status").and_then(serde_json::Value::as_str) {
            Some("pending") => shape.pending_count += 1,
            Some("in_progress") => shape.in_progress_count += 1,
            Some("completed") => shape.completed_count += 1,
            _ => shape.other_status_count += 1,
        }
        if item
            .get("step")
            .and_then(serde_json::Value::as_str)
            .is_none()
        {
            shape.valid_schema = false;
        }
    }
    shape
}

fn parse_session_planning_event_from_tool_output(
    session_id: &str,
    timestamp: Option<&str>,
    path: &Path,
    payload: &serde_json::Map<String, serde_json::Value>,
    call_meta: &SessionPlanningCallMeta,
) -> SessionPlanningEvent {
    let output = json_field_string(payload.get("output")).unwrap_or_default();
    let output_success = output.trim() == "Plan updated";
    let verified_safe_accept = output_success && call_meta.plan_shape.valid_schema;
    let mut evidence = Vec::new();
    evidence.push(
        if output_success {
            "plan_updated_output"
        } else {
            "plan_update_output_not_confirmed"
        }
        .to_owned(),
    );
    evidence.push(
        if call_meta.plan_shape.valid_schema {
            "valid_plan_schema"
        } else {
            "invalid_plan_schema"
        }
        .to_owned(),
    );
    evidence.push(format!(
        "plan_step_count_band:{}",
        generic_count_band(call_meta.plan_shape.step_count)
    ));
    evidence.push(format!(
        "plan_completed_count_band:{}",
        generic_count_band(call_meta.plan_shape.completed_count)
    ));
    evidence.push(format!(
        "plan_in_progress_count_band:{}",
        generic_count_band(call_meta.plan_shape.in_progress_count)
    ));
    evidence.push(format!(
        "plan_pending_count_band:{}",
        generic_count_band(call_meta.plan_shape.pending_count)
    ));
    if call_meta.plan_shape.has_explanation {
        evidence.push("plan_has_explanation".to_owned());
    }
    SessionPlanningEvent {
        session_id: session_id.to_owned(),
        turn_id: call_meta.turn_id.clone(),
        timestamp: timestamp.unwrap_or_default().to_owned(),
        path: path.to_path_buf(),
        arguments: call_meta.arguments.clone(),
        output,
        plan_shape: call_meta.plan_shape.clone(),
        verified_safe_accept,
        evidence,
    }
}

fn parse_session_tool_status_event_from_tool_output(
    session_id: &str,
    timestamp: Option<&str>,
    path: &Path,
    payload: &serde_json::Map<String, serde_json::Value>,
    call_meta: Option<&SessionToolCallMeta>,
) -> Option<SessionRunCheckEvent> {
    let payload_type = payload.get("type").and_then(serde_json::Value::as_str)?;
    if payload_type != "function_call_output" && payload_type != "custom_tool_call_output" {
        return None;
    }
    let output = json_field_string(payload.get("output")).unwrap_or_default();
    let fallback_command = payload_type
        .strip_suffix("_output")
        .unwrap_or(payload_type)
        .to_owned();
    let command = call_meta
        .map(|meta| meta.command.clone())
        .unwrap_or(fallback_command);
    let cwd = call_meta
        .map(|meta| meta.cwd.clone())
        .unwrap_or_else(|| "unknown_cwd".to_owned());
    let turn_id = call_meta
        .map(|meta| meta.turn_id.clone())
        .unwrap_or_else(|| session_tool_turn_id(payload));
    let exit_code = tool_output_exit_code(&output).unwrap_or(1);
    let (label, evidence, unknown_failure) =
        classify_tool_status_output(&command, &output, exit_code);
    Some(SessionRunCheckEvent {
        session_id: session_id.to_owned(),
        turn_id,
        timestamp: timestamp.unwrap_or_default().to_owned(),
        path: path.to_path_buf(),
        command,
        cwd,
        output,
        exit_code,
        label,
        evidence,
        unknown_failure,
    })
}

fn tool_output_exit_code(output: &str) -> Option<i64> {
    for marker in ["Process exited with code ", "Exit code: "] {
        if let Some(after) = output.split(marker).nth(1) {
            let digits = after
                .chars()
                .take_while(|ch| ch.is_ascii_digit() || *ch == '-')
                .collect::<String>();
            if let Ok(code) = digits.parse::<i64>() {
                return Some(code);
            }
        }
    }
    if output.contains("Process running with session ID") {
        return None;
    }
    None
}

fn is_run_check_command(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower.contains("cargo test")
        || lower.contains("cargo check")
        || lower.contains("cargo clippy")
        || lower.contains("cargo fmt")
        || lower.contains("cargo run")
        || lower.contains("npm test")
        || lower.contains("pytest")
        || lower.contains("test result:")
}

fn classify_tool_status_output(
    command: &str,
    output: &str,
    exit_code: i64,
) -> (TestOutputLabel, Vec<String>, bool) {
    let lower_output = output.to_ascii_lowercase();
    let lower_command = command.to_ascii_lowercase();
    if exit_code == 0 {
        return (
            TestOutputLabel::Pass,
            vec![
                "exit_code_zero".to_owned(),
                tool_status_command_kind(&lower_command).to_owned(),
            ],
            false,
        );
    }
    if lower_output.contains("panic")
        || lower_output.contains("thread '")
        || lower_output.contains("segmentation fault")
    {
        return (
            TestOutputLabel::RuntimePanic,
            vec![
                "nonzero_exit_runtime_panic_marker".to_owned(),
                tool_status_command_kind(&lower_command).to_owned(),
            ],
            false,
        );
    }
    if lower_output.contains("could not compile")
        || lower_output.contains("error:")
        || lower_output.contains("error[")
    {
        return (
            TestOutputLabel::CompileError,
            vec![
                "nonzero_exit_error_marker".to_owned(),
                tool_status_command_kind(&lower_command).to_owned(),
            ],
            false,
        );
    }
    (
        TestOutputLabel::Fail,
        vec![
            "nonzero_exit_status".to_owned(),
            tool_status_command_kind(&lower_command).to_owned(),
        ],
        true,
    )
}

fn classify_run_check_output(
    command: &str,
    output: &str,
    exit_code: i64,
) -> (TestOutputLabel, Vec<String>, bool) {
    let lower_output = output.to_ascii_lowercase();
    if let Some((label, evidence)) = verify_test_output(&lower_output) {
        return (label, evidence, false);
    }
    let lower_command = command.to_ascii_lowercase();
    if exit_code == 0 {
        return (
            TestOutputLabel::Pass,
            vec![
                "exit_code_zero".to_owned(),
                run_check_command_kind(&lower_command).to_owned(),
            ],
            false,
        );
    }
    if lower_output.contains("could not compile")
        || lower_output.contains("error:")
        || lower_output.contains("error[")
    {
        return (
            TestOutputLabel::CompileError,
            vec![
                "nonzero_exit_compile_marker".to_owned(),
                run_check_command_kind(&lower_command).to_owned(),
            ],
            false,
        );
    }
    (
        TestOutputLabel::Fail,
        vec![
            "nonzero_exit_unknown_failure".to_owned(),
            run_check_command_kind(&lower_command).to_owned(),
        ],
        true,
    )
}

fn tool_status_command_kind(command_lower: &str) -> &'static str {
    if command_lower.contains("cargo test") {
        "cargo_test"
    } else if command_lower.contains("cargo check") {
        "cargo_check"
    } else if command_lower.contains("cargo clippy") {
        "cargo_clippy"
    } else if command_lower.contains("cargo fmt") {
        "cargo_fmt"
    } else if command_lower.contains("cargo run") {
        "cargo_run"
    } else if command_lower.contains("git status") {
        "git_status"
    } else if command_lower.contains("git diff") {
        "git_diff"
    } else if command_lower.contains("git ") {
        "git_other"
    } else if command_lower.contains("rg ") || command_lower.starts_with("rg") {
        "ripgrep"
    } else if command_lower.contains("python") {
        "python"
    } else if command_lower.contains("sed ") || command_lower.starts_with("sed") {
        "sed"
    } else if command_lower.contains("find ") || command_lower.starts_with("find") {
        "find"
    } else if command_lower.contains("ls ") || command_lower == "ls" {
        "ls"
    } else if command_lower.contains("wc ") || command_lower.starts_with("wc") {
        "wc"
    } else if command_lower.contains("npm ") {
        "npm"
    } else if command_lower.contains("pytest") {
        "pytest"
    } else {
        "other"
    }
}

fn tool_status_shell_family(command_lower: &str) -> &'static str {
    if command_lower.contains("cargo ") {
        "rust"
    } else if command_lower.contains("git ") {
        "git"
    } else if command_lower.contains("python") || command_lower.contains("pytest") {
        "python"
    } else if command_lower.contains("npm ") || command_lower.contains("node ") {
        "node"
    } else if command_lower.contains("rg ")
        || command_lower.starts_with("rg")
        || command_lower.contains("sed ")
        || command_lower.starts_with("sed")
        || command_lower.contains("find ")
        || command_lower.starts_with("find")
        || command_lower.contains("ls ")
        || command_lower == "ls"
        || command_lower.contains("wc ")
        || command_lower.starts_with("wc")
    {
        "shell_inspect"
    } else {
        "other"
    }
}

fn output_has_any(output_lower: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| output_lower.contains(needle))
}

fn tool_output_shape_atoms(output_lower: &str) -> Vec<String> {
    let mut atoms = Vec::new();
    for (needle, atom) in [
        ("finished", "state_output_marker:finished"),
        ("warning", "state_output_marker:warning"),
        ("error", "state_output_marker:error"),
        ("failed", "state_output_marker:failed"),
        ("panic", "state_output_marker:panic"),
        ("not found", "state_output_marker:not_found"),
        ("no such file", "state_output_marker:no_such_file"),
        ("passed", "state_output_marker:passed"),
        ("0 failed", "state_output_marker:zero_failed"),
        ("clean", "state_output_marker:clean"),
    ] {
        if output_lower.contains(needle) {
            atoms.push(atom.to_owned());
        }
    }
    if atoms.is_empty() {
        atoms.push("state_output_marker:none_selected".to_owned());
    }
    atoms
}

fn session_planning_event_to_phase_atom_row(
    event: &SessionPlanningEvent,
    row_index: usize,
) -> serde_json::Value {
    let argument_hash = stable_fingerprint([event.arguments.as_str()]);
    let output_hash = stable_fingerprint([event.output.as_str()]);
    let event_hash = stable_fingerprint([
        event.session_id.as_str(),
        event.turn_id.as_str(),
        &format!("{argument_hash:016x}"),
        &format!("{output_hash:016x}"),
    ]);
    let status_shape_atom = format!(
        "request_plan_status_shape:c{}:i{}:p{}:o{}",
        generic_count_band(event.plan_shape.completed_count),
        generic_count_band(event.plan_shape.in_progress_count),
        generic_count_band(event.plan_shape.pending_count),
        generic_count_band(event.plan_shape.other_status_count)
    );
    let request_atoms = vec![
        "request_route_family:agent_continue_execute".to_owned(),
        format!(
            "request_plan_step_count_band:{}",
            generic_count_band(event.plan_shape.step_count)
        ),
        status_shape_atom,
        format!(
            "request_plan_has_explanation:{}",
            event.plan_shape.has_explanation
        ),
    ];
    let state_atoms = vec![
        "state_source:codex_session_update_plan".to_owned(),
        format!(
            "state_session_bucket:{}",
            stable_fingerprint([event.session_id.as_str()]) % 64
        ),
        format!("state_plan_valid_schema:{}", event.plan_shape.valid_schema),
        format!(
            "state_plan_completed_count_band:{}",
            generic_count_band(event.plan_shape.completed_count)
        ),
        format!(
            "state_plan_in_progress_count_band:{}",
            generic_count_band(event.plan_shape.in_progress_count)
        ),
        format!(
            "state_plan_pending_count_band:{}",
            generic_count_band(event.plan_shape.pending_count)
        ),
    ];
    let action_atoms = vec![
        "action_family:planning".to_owned(),
        "action:update_plan_state".to_owned(),
        "route_operator:agent_continue_execute".to_owned(),
        "domain_family:agent_loop".to_owned(),
    ];
    let tool_atoms = vec!["tool_name:update_plan".to_owned()];
    let estimated_input_tokens = event.arguments.chars().count().div_ceil(4).max(1);
    let estimated_output_tokens = event.output.chars().count().div_ceil(4).max(1);
    let estimated_total_tokens = estimated_input_tokens + estimated_output_tokens;
    let mut result_atoms = vec![
        format!(
            "verifier_label:{}",
            if event.verified_safe_accept {
                "plan_updated"
            } else {
                "plan_update_failed"
            }
        ),
        format!("verified_safe_accept:{}", event.verified_safe_accept),
        format!("output_hash64:{output_hash:016x}"),
    ];
    result_atoms.extend(event.evidence.iter().map(|item| format!("evidence:{item}")));
    let metadata_atoms = vec![
        "traffic_source_kind:codex_session".to_owned(),
        "verification_source_kind:update_plan_tool_output".to_owned(),
        "llm_call_kind:null".to_owned(),
        "has_shadow_request:true".to_owned(),
        "has_verifier_label:true".to_owned(),
        "synthetic_source:false".to_owned(),
        format!("token_band:{}", generic_count_band(estimated_total_tokens)),
        "cost_band:0".to_owned(),
    ];
    let route_hint_atoms = vec!["route_hint:planning_update".to_owned()];
    let shadow_route_key = "planning_update";
    let shadow_profile_id = "phase_center_planning_update_v1";
    let shadow_source_atoms = request_atoms
        .iter()
        .chain(state_atoms.iter())
        .chain(action_atoms.iter())
        .chain(tool_atoms.iter())
        .chain(route_hint_atoms.iter())
        .collect::<Vec<_>>();
    let mut seen_shadow_centers = BTreeSet::new();
    let active_fringe = shadow_source_atoms
        .iter()
        .filter_map(|atom| {
            let center_id = stable_fingerprint([atom.as_str()]) % 131_072;
            if seen_shadow_centers.insert(center_id) {
                Some(serde_json::json!({
                    "center_id": center_id,
                    "strength": 1
                }))
            } else {
                None
            }
        })
        .take(32)
        .collect::<Vec<_>>();
    let slot_specs = [
        (
            0u64,
            "plan_step_count",
            generic_count_band(event.plan_shape.step_count),
        ),
        (
            1u64,
            "plan_completed_count",
            generic_count_band(event.plan_shape.completed_count),
        ),
        (
            2u64,
            "plan_in_progress_count",
            generic_count_band(event.plan_shape.in_progress_count),
        ),
        (
            3u64,
            "plan_pending_count",
            generic_count_band(event.plan_shape.pending_count),
        ),
        (
            4u64,
            "plan_has_explanation",
            if event.plan_shape.has_explanation {
                "true"
            } else {
                "false"
            },
        ),
    ];
    let slots = slot_specs
        .iter()
        .map(|(slot_id, slot_kind, value_band)| {
            let lane_id = stable_fingerprint([*slot_kind, *value_band]) % 4096;
            serde_json::json!({
                "binding_output_slot": slot_id,
                "slot_kind": slot_kind,
                "value_band": value_band,
                "positive_impulses": [
                    {
                        "lane_id": lane_id,
                        "strength": 1
                    }
                ],
                "negative_impulses": []
            })
        })
        .collect::<Vec<_>>();
    let mut shadow_payload_atoms = vec![
        format!("shadow_route_key:{shadow_route_key}"),
        format!("shadow_profile_id:{shadow_profile_id}"),
        format!(
            "shadow_active_fringe_len_band:{}",
            generic_count_band(active_fringe.len())
        ),
        format!("shadow_slot_count_band:{}", generic_count_band(slots.len())),
    ];
    for center_id in active_fringe
        .iter()
        .filter_map(|item| item.get("center_id").and_then(serde_json::Value::as_u64))
        .take(8)
    {
        shadow_payload_atoms.push(format!("shadow_active_center_page:{}", center_id / 4096));
    }
    for slot in slots.iter().take(8) {
        shadow_payload_atoms.push(format!("shadow_slot_shape:{}", generic_slot_atom(slot)));
    }
    let nando_shadow_request = serde_json::json!({
        "route_key": shadow_route_key,
        "profile_id": shadow_profile_id,
        "exact_cache_key": format!("codex_session_update_plan_args:{argument_hash:016x}"),
        "active_fringe": active_fringe,
        "slots": slots,
        "source": "codex_session_update_plan_request_side_atoms_v1",
        "forbidden_fields_absent": {
            "raw_plan_text": true,
            "raw_request_text": true,
            "raw_response_text": true,
            "target_id": true,
            "proof_rule_id": true,
            "concrete_x_lookup": true,
            "manual_local_out_t": true
        }
    });
    let output_atoms_written = metadata_atoms.len()
        + route_hint_atoms.len()
        + request_atoms.len()
        + state_atoms.len()
        + action_atoms.len()
        + tool_atoms.len()
        + result_atoms.len()
        + shadow_payload_atoms.len();
    serde_json::json!({
        "schema_version": "real_traffic_phase_atom_trace_v1",
        "source_schema_version": "codex_session_planning_verifier_trace_v1",
        "input_trace_path": event.path.display().to_string(),
        "trace_id": format!("codex-session-planning-{row_index:06}-{event_hash:016x}"),
        "event_timestamp": event.timestamp,
        "time_ms": serde_json::Value::Null,
        "request_fingerprint": format!("codex_session_planning:{event_hash:016x}"),
        "exact_cache_key": format!("codex_session_update_plan_args:{argument_hash:016x}"),
        "traffic_source": "codex_session_planning_verifier_trace_v1",
        "verification_source_kind": "update_plan_tool_output",
        "verified_safe_accept": event.verified_safe_accept,
        "has_shadow_request": true,
        "ready_for_route_family_mining": true,
        "ready_for_existing_shadow_scoring": true,
        "ready_for_action_family_clustering": true,
        "nando_shadow_request": nando_shadow_request,
        "metadata_only": false,
        "missing_state_or_request_atoms": false,
        "missing_action_atoms": false,
        "missing_verifier_label": false,
        "token_cost": {
            "total_tokens": estimated_total_tokens,
            "total_cost_microusd": 0,
            "token_evidence_missing": false,
            "cost_evidence_missing": true,
            "token_cost_estimate_used": true
        },
        "request_atoms": request_atoms.clone(),
        "state_atoms": state_atoms.clone(),
        "action_atoms": action_atoms.clone(),
        "tool_atoms": tool_atoms.clone(),
        "result_atoms": result_atoms.clone(),
        "route_hint_atoms": route_hint_atoms.clone(),
        "atom_groups": {
            "metadata_atoms": metadata_atoms,
            "route_hint_atoms": route_hint_atoms,
            "request_atoms": request_atoms,
            "state_atoms": state_atoms,
            "action_atoms": action_atoms,
            "tool_atoms": tool_atoms,
            "result_atoms": result_atoms,
            "derived_tool_atoms": [],
            "shadow_payload_atoms": shadow_payload_atoms
        },
        "output_atoms_written": output_atoms_written,
        "forbidden_fields_absent": {
            "raw_plan_text": true,
            "raw_request_text": true,
            "raw_response_text": true,
            "target_id": true,
            "proof_rule_id": true,
            "concrete_x_lookup": true,
            "manual_local_out_t": true
        }
    })
}

fn session_tool_status_event_to_phase_atom_row(
    event: &SessionRunCheckEvent,
    row_index: usize,
) -> serde_json::Value {
    let command_lower = event.command.to_ascii_lowercase();
    let output_lower = event.output.to_ascii_lowercase();
    let command_kind = tool_status_command_kind(&command_lower);
    let shell_family = tool_status_shell_family(&command_lower);
    let exit_band = exit_code_band(event.exit_code);
    let output_char_band = generic_count_band(event.output.chars().count());
    let output_line_band = generic_count_band(event.output.lines().count());
    let output_has_error =
        output_has_any(&output_lower, &["error", "failed", "panic", "not found"]);
    let output_has_warning = output_has_any(&output_lower, &["warning", "warn"]);
    let output_hash = stable_fingerprint([event.output.as_str()]);
    let command_hash = stable_fingerprint([event.command.as_str()]);
    let event_hash = stable_fingerprint([
        event.session_id.as_str(),
        event.turn_id.as_str(),
        event.command.as_str(),
        &event.exit_code.to_string(),
        &format!("{output_hash:016x}"),
    ]);
    let request_atoms = vec![
        format!("request_command_kind:{command_kind}"),
        format!(
            "request_command_arg_band:{}",
            shell_word_band(&event.command)
        ),
        format!("request_cwd_kind:{}", cwd_kind(&event.cwd)),
    ];
    let mut state_atoms = vec![
        "state_source:codex_session_exec_command_end".to_owned(),
        format!(
            "state_session_bucket:{}",
            stable_fingerprint([event.session_id.as_str()]) % 64
        ),
        format!("state_cwd_kind:{}", cwd_kind(&event.cwd)),
        format!("state_exit_code_band:{exit_band}"),
        format!("state_output_char_band:{output_char_band}"),
        format!("state_output_line_band:{output_line_band}"),
        format!("state_output_has_error_marker:{output_has_error}"),
        format!("state_output_has_warning_marker:{output_has_warning}"),
    ];
    state_atoms.extend(tool_output_shape_atoms(&output_lower));
    let action_atoms = vec![
        "action:continue_after_tool_result".to_owned(),
        "action:parse_tool_status".to_owned(),
        "action_family:planning".to_owned(),
        "domain_family:agent_loop".to_owned(),
        "route_operator:agent_continue_execute".to_owned(),
        "subroute_operator:command_result_followup".to_owned(),
    ];
    let tool_atoms = vec![
        format!("tool_command_kind:{command_kind}"),
        format!("tool_command_shell_family:{shell_family}"),
    ];
    let estimated_input_tokens =
        (event.command.chars().count() + event.cwd.chars().count() + event.output.chars().count())
            .div_ceil(4)
            .max(1);
    let estimated_output_tokens = event.label.as_str().chars().count().div_ceil(4).max(1);
    let estimated_total_tokens = estimated_input_tokens + estimated_output_tokens;
    let mut result_atoms = vec![
        format!("verifier_label:{}", event.label.as_str()),
        format!(
            "verified_safe_accept:{}",
            event.label == TestOutputLabel::Pass
        ),
        format!("exit_code_band:{exit_band}"),
        format!("output_hash64:{output_hash:016x}"),
    ];
    result_atoms.extend(event.evidence.iter().map(|item| format!("evidence:{item}")));
    let metadata_atoms = vec![
        "traffic_source_kind:codex_session".to_owned(),
        "verification_source_kind:tool_output_status".to_owned(),
        "llm_call_kind:null".to_owned(),
        "has_shadow_request:true".to_owned(),
        "has_verifier_label:true".to_owned(),
        "synthetic_source:false".to_owned(),
        format!("token_band:{}", generic_count_band(estimated_total_tokens)),
        "cost_band:0".to_owned(),
    ];
    let route_hint_atoms = vec![
        "route_hint:agent_continue_execute".to_owned(),
        "route_hint:tool_status_parse".to_owned(),
        "subroute_hint:command_result_followup".to_owned(),
    ];
    let shadow_route_key = "agent_continue_command_result_followup";
    let shadow_profile_id = "phase_center_agent_continue_command_result_followup_v1";
    let shadow_source_atoms = request_atoms
        .iter()
        .chain(state_atoms.iter())
        .chain(action_atoms.iter())
        .chain(tool_atoms.iter())
        .chain(route_hint_atoms.iter())
        .collect::<Vec<_>>();
    let mut seen_shadow_centers = BTreeSet::new();
    let active_fringe = shadow_source_atoms
        .iter()
        .filter_map(|atom| {
            let center_id = stable_fingerprint([atom.as_str()]) % 131_072;
            if seen_shadow_centers.insert(center_id) {
                Some(serde_json::json!({
                    "center_id": center_id,
                    "strength": 1
                }))
            } else {
                None
            }
        })
        .take(40)
        .collect::<Vec<_>>();
    let slot_specs = [
        (0u64, "command_kind", command_kind.to_owned()),
        (1u64, "shell_family", shell_family.to_owned()),
        (2u64, "exit_code_band", exit_band.to_owned()),
        (3u64, "output_char_band", output_char_band.to_owned()),
        (4u64, "output_line_band", output_line_band.to_owned()),
        (
            5u64,
            "output_has_error_marker",
            output_has_error.to_string(),
        ),
        (
            6u64,
            "output_has_warning_marker",
            output_has_warning.to_string(),
        ),
    ];
    let slots = slot_specs
        .iter()
        .map(|(slot_id, slot_kind, value_band)| {
            let lane_id = stable_fingerprint([*slot_kind, value_band.as_str()]) % 4096;
            serde_json::json!({
                "binding_output_slot": slot_id,
                "slot_kind": slot_kind,
                "value_band": value_band,
                "positive_impulses": [
                    {
                        "lane_id": lane_id,
                        "strength": 1
                    }
                ],
                "negative_impulses": []
            })
        })
        .collect::<Vec<_>>();
    let mut shadow_payload_atoms = vec![
        format!("shadow_route_key:{shadow_route_key}"),
        format!("shadow_profile_id:{shadow_profile_id}"),
        format!(
            "shadow_active_fringe_len_band:{}",
            generic_count_band(active_fringe.len())
        ),
        format!("shadow_slot_count_band:{}", generic_count_band(slots.len())),
    ];
    for center_id in active_fringe
        .iter()
        .filter_map(|item| item.get("center_id").and_then(serde_json::Value::as_u64))
        .take(8)
    {
        shadow_payload_atoms.push(format!("shadow_active_center_page:{}", center_id / 4096));
    }
    for slot in slots.iter().take(8) {
        shadow_payload_atoms.push(format!("shadow_slot_shape:{}", generic_slot_atom(slot)));
    }
    let nando_shadow_request = serde_json::json!({
        "route_key": shadow_route_key,
        "profile_id": shadow_profile_id,
        "exact_cache_key": format!("codex_session_tool_status_command:{command_hash:016x}"),
        "active_fringe": active_fringe,
        "slots": slots,
        "source": "codex_session_tool_status_request_side_atoms_v1",
        "forbidden_fields_absent": {
            "raw_tool_output": true,
            "raw_request_text": true,
            "raw_response_text": true,
            "target_id": true,
            "proof_rule_id": true,
            "concrete_x_lookup": true,
            "manual_local_out_t": true
        }
    });
    let output_atoms_written = metadata_atoms.len()
        + route_hint_atoms.len()
        + request_atoms.len()
        + state_atoms.len()
        + action_atoms.len()
        + tool_atoms.len()
        + result_atoms.len()
        + shadow_payload_atoms.len();
    serde_json::json!({
        "schema_version": "real_traffic_phase_atom_trace_v1",
        "source_schema_version": "codex_session_tool_status_verifier_trace_v1",
        "input_trace_path": event.path.display().to_string(),
        "trace_id": format!("codex-session-tool-status-{row_index:06}-{event_hash:016x}"),
        "event_timestamp": event.timestamp,
        "time_ms": serde_json::Value::Null,
        "request_fingerprint": format!("codex_session_tool_status:{event_hash:016x}"),
        "exact_cache_key": format!("codex_session_tool_status_command:{command_hash:016x}"),
        "traffic_source": "codex_session_tool_status_verifier_trace_v1",
        "verification_source_kind": "tool_output_status",
        "verified_safe_accept": event.label == TestOutputLabel::Pass,
        "has_shadow_request": true,
        "ready_for_route_family_mining": true,
        "ready_for_existing_shadow_scoring": true,
        "ready_for_action_family_clustering": true,
        "nando_shadow_request": nando_shadow_request,
        "metadata_only": false,
        "missing_state_or_request_atoms": false,
        "missing_action_atoms": false,
        "missing_verifier_label": false,
        "token_cost": {
            "total_tokens": estimated_total_tokens,
            "total_cost_microusd": 0,
            "token_evidence_missing": false,
            "cost_evidence_missing": true,
            "token_cost_estimate_used": true
        },
        "request_atoms": request_atoms.clone(),
        "state_atoms": state_atoms.clone(),
        "action_atoms": action_atoms.clone(),
        "tool_atoms": tool_atoms.clone(),
        "result_atoms": result_atoms.clone(),
        "route_hint_atoms": route_hint_atoms.clone(),
        "atom_groups": {
            "metadata_atoms": metadata_atoms,
            "route_hint_atoms": route_hint_atoms,
            "request_atoms": request_atoms,
            "state_atoms": state_atoms,
            "action_atoms": action_atoms,
            "tool_atoms": tool_atoms,
            "result_atoms": result_atoms,
            "derived_tool_atoms": [],
            "shadow_payload_atoms": shadow_payload_atoms
        },
        "output_atoms_written": output_atoms_written,
        "forbidden_fields_absent": {
            "raw_tool_output": true,
            "raw_request_text": true,
            "raw_response_text": true,
            "target_id": true,
            "proof_rule_id": true,
            "concrete_x_lookup": true,
            "manual_local_out_t": true
        }
    })
}

fn session_run_check_event_to_phase_atom_row(
    event: &SessionRunCheckEvent,
    row_index: usize,
) -> serde_json::Value {
    let command_lower = event.command.to_ascii_lowercase();
    let output_hash = stable_fingerprint([event.output.as_str()]);
    let command_hash = stable_fingerprint([event.command.as_str()]);
    let event_hash = stable_fingerprint([
        event.session_id.as_str(),
        event.turn_id.as_str(),
        event.command.as_str(),
        &event.exit_code.to_string(),
        &format!("{output_hash:016x}"),
    ]);
    let request_atoms = vec![
        format!(
            "request_command_kind:{}",
            run_check_command_kind(&command_lower)
        ),
        format!(
            "request_command_arg_band:{}",
            shell_word_band(&event.command)
        ),
        format!("request_cwd_kind:{}", cwd_kind(&event.cwd)),
    ];
    let output_lower = event.output.to_ascii_lowercase();
    let parse_detail_atoms = run_check_parse_detail_atoms(&output_lower, event.unknown_failure);
    let mut state_atoms = vec![
        "state_source:codex_session_exec_command_end".to_owned(),
        format!(
            "state_session_bucket:{}",
            stable_fingerprint([event.session_id.as_str()]) % 64
        ),
        format!("state_cwd_kind:{}", cwd_kind(&event.cwd)),
        format!("state_exit_code_band:{}", exit_code_band(event.exit_code)),
    ];
    state_atoms.extend(
        parse_detail_atoms
            .iter()
            .map(|atom| format!("state_{atom}")),
    );
    let action_atoms = vec![
        "action_family:run_check".to_owned(),
        "action:parse_test_output".to_owned(),
        "domain_family:nando_wave".to_owned(),
    ];
    let tool_check_kind = if command_lower.contains("clippy") {
        "lint"
    } else if command_lower.contains("fmt") {
        "format"
    } else if command_lower.contains("test") {
        "test"
    } else {
        "build"
    };
    let mut tool_atoms = vec![
        "tool_mention:cargo".to_owned(),
        format!(
            "tool_command_kind:{}",
            run_check_command_kind(&command_lower)
        ),
        format!("tool_check_kind:{tool_check_kind}"),
    ];
    tool_atoms.extend(parse_detail_atoms.iter().map(|atom| format!("tool_{atom}")));
    let estimated_input_tokens =
        (event.command.chars().count() + event.cwd.chars().count() + event.output.chars().count())
            .div_ceil(4)
            .max(1);
    let estimated_output_tokens = event.label.as_str().chars().count().div_ceil(4).max(1);
    let estimated_total_tokens = estimated_input_tokens + estimated_output_tokens;
    let mut result_atoms = vec![
        format!("verifier_label:{}", event.label.as_str()),
        format!(
            "verified_safe_accept:{}",
            event.label == TestOutputLabel::Pass
        ),
        format!("exit_code_band:{}", exit_code_band(event.exit_code)),
        format!("output_hash64:{output_hash:016x}"),
    ];
    result_atoms.extend(event.evidence.iter().map(|item| format!("evidence:{item}")));
    let metadata_atoms = vec![
        "traffic_source_kind:codex_session".to_owned(),
        "verification_source_kind:tool_output_status".to_owned(),
        "llm_call_kind:null".to_owned(),
        "has_shadow_request:true".to_owned(),
        "has_verifier_label:true".to_owned(),
        "synthetic_source:false".to_owned(),
        format!("token_band:{}", generic_count_band(estimated_total_tokens)),
        "cost_band:0".to_owned(),
    ];
    let route_hint_atoms = vec!["route_hint:test_output_parse".to_owned()];
    let shadow_route_key = "test_output_parse";
    let shadow_profile_id = "phase_center_run_check_v1";
    let shadow_source_atoms = request_atoms
        .iter()
        .chain(state_atoms.iter())
        .chain(action_atoms.iter())
        .chain(tool_atoms.iter())
        .chain(route_hint_atoms.iter())
        .collect::<Vec<_>>();
    let mut seen_shadow_centers = BTreeSet::new();
    let active_fringe = shadow_source_atoms
        .iter()
        .filter_map(|atom| {
            let center_id = stable_fingerprint([atom.as_str()]) % 131_072;
            if seen_shadow_centers.insert(center_id) {
                Some(serde_json::json!({
                    "center_id": center_id,
                    "strength": 1
                }))
            } else {
                None
            }
        })
        .take(32)
        .collect::<Vec<_>>();
    let slot_specs = [
        (
            0u64,
            "run_check_command_kind",
            run_check_command_kind(&command_lower).to_owned(),
        ),
        (1u64, "tool_check_kind", tool_check_kind.to_owned()),
        (2u64, "cwd_kind", cwd_kind(&event.cwd).to_owned()),
        (
            3u64,
            "command_arg_band",
            shell_word_band(&event.command).to_owned(),
        ),
        (
            4u64,
            "exit_code_band",
            exit_code_band(event.exit_code).to_owned(),
        ),
    ];
    let slots = slot_specs
        .iter()
        .map(|(slot_id, slot_kind, value_band)| {
            let lane_id = stable_fingerprint([*slot_kind, value_band.as_str()]) % 4096;
            serde_json::json!({
                "binding_output_slot": slot_id,
                "slot_kind": slot_kind,
                "value_band": value_band,
                "positive_impulses": [
                    {
                        "lane_id": lane_id,
                        "strength": 1
                    }
                ],
                "negative_impulses": []
            })
        })
        .collect::<Vec<_>>();
    let mut shadow_payload_atoms = vec![
        format!("shadow_route_key:{shadow_route_key}"),
        format!("shadow_profile_id:{shadow_profile_id}"),
        format!(
            "shadow_active_fringe_len_band:{}",
            generic_count_band(active_fringe.len())
        ),
        format!("shadow_slot_count_band:{}", generic_count_band(slots.len())),
    ];
    for center_id in active_fringe
        .iter()
        .filter_map(|item| item.get("center_id").and_then(serde_json::Value::as_u64))
        .take(8)
    {
        shadow_payload_atoms.push(format!("shadow_active_center_page:{}", center_id / 4096));
    }
    for slot in slots.iter().take(8) {
        shadow_payload_atoms.push(format!("shadow_slot_shape:{}", generic_slot_atom(slot)));
    }
    let nando_shadow_request = serde_json::json!({
        "route_key": shadow_route_key,
        "profile_id": shadow_profile_id,
        "exact_cache_key": format!("codex_session_run_check_command:{command_hash:016x}"),
        "active_fringe": active_fringe,
        "slots": slots,
        "source": "codex_session_run_check_request_side_atoms_v1",
        "forbidden_fields_absent": {
            "raw_tool_output": true,
            "raw_request_text": true,
            "raw_response_text": true,
            "target_id": true,
            "proof_rule_id": true,
            "concrete_x_lookup": true,
            "manual_local_out_t": true
        }
    });
    let output_atoms_written = metadata_atoms.len()
        + route_hint_atoms.len()
        + request_atoms.len()
        + state_atoms.len()
        + action_atoms.len()
        + tool_atoms.len()
        + result_atoms.len()
        + shadow_payload_atoms.len();
    serde_json::json!({
        "schema_version": "real_traffic_phase_atom_trace_v1",
        "source_schema_version": "codex_session_run_check_verifier_trace_v1",
        "input_trace_path": event.path.display().to_string(),
        "trace_id": format!("codex-session-run-check-{row_index:06}-{event_hash:016x}"),
        "event_timestamp": event.timestamp,
        "time_ms": serde_json::Value::Null,
        "request_fingerprint": format!("codex_session_run_check:{event_hash:016x}"),
        "exact_cache_key": format!("codex_session_run_check_command:{command_hash:016x}"),
        "traffic_source": "codex_session_run_check_verifier_trace_v1",
        "verification_source_kind": "tool_output_status",
        "verified_safe_accept": event.label == TestOutputLabel::Pass,
        "has_shadow_request": true,
        "ready_for_route_family_mining": true,
        "ready_for_existing_shadow_scoring": true,
        "ready_for_action_family_clustering": true,
        "nando_shadow_request": nando_shadow_request,
        "metadata_only": false,
        "missing_state_or_request_atoms": false,
        "missing_action_atoms": false,
        "missing_verifier_label": false,
        "token_cost": {
            "total_tokens": estimated_total_tokens,
            "total_cost_microusd": 0,
            "token_evidence_missing": false,
            "cost_evidence_missing": true,
            "token_cost_estimate_used": true
        },
        "request_atoms": request_atoms,
        "state_atoms": state_atoms,
        "action_atoms": action_atoms,
        "tool_atoms": tool_atoms,
        "result_atoms": result_atoms,
        "route_hint_atoms": route_hint_atoms.clone(),
        "atom_groups": {
            "metadata_atoms": metadata_atoms,
            "route_hint_atoms": route_hint_atoms,
            "request_atoms": request_atoms,
            "state_atoms": state_atoms,
            "action_atoms": action_atoms,
            "tool_atoms": tool_atoms,
            "result_atoms": result_atoms,
            "derived_tool_atoms": [],
            "shadow_payload_atoms": shadow_payload_atoms
        },
        "output_atoms_written": output_atoms_written,
        "raw_tool_output_written": false,
        "forbidden_fields_absent": {
            "raw_request_text": true,
            "raw_response_text": true,
            "raw_tool_output_text": true,
            "target_id": true,
            "proof_rule_id": true,
            "concrete_x_lookup": true,
            "manual_local_out_t": true
        }
    })
}

fn run_check_command_kind(lower_command: &str) -> &'static str {
    if lower_command.contains("cargo clippy") {
        "cargo_clippy"
    } else if lower_command.contains("cargo fmt") {
        "cargo_fmt"
    } else if lower_command.contains("cargo check") {
        "cargo_check"
    } else if lower_command.contains("cargo test") {
        "cargo_test"
    } else if lower_command.contains("npm test") {
        "npm_test"
    } else if lower_command.contains("pytest") {
        "pytest"
    } else {
        "run_check_other"
    }
}

fn run_check_parse_detail_atoms(lower_output: &str, unknown_failure: bool) -> Vec<String> {
    let mut atoms = output_markers(lower_output)
        .into_iter()
        .map(|marker| format!("parse_marker:{marker}"))
        .collect::<BTreeSet<_>>();
    if lower_output.contains("finished ") {
        atoms.insert("parse_signal:cargo_finished_line".to_owned());
    }
    if lower_output.contains("running ") {
        atoms.insert("parse_signal:cargo_running_line".to_owned());
    }
    if lower_output.contains("warning:") {
        atoms.insert("parse_signal:warning_marker".to_owned());
    }
    if lower_output.contains("error:") || lower_output.contains("error[") {
        atoms.insert("parse_signal:error_marker".to_owned());
    }
    if lower_output.contains("0 failed") {
        atoms.insert("parse_signal:zero_failed_marker".to_owned());
    }
    if lower_output.contains("failed") {
        atoms.insert("parse_signal:failed_word_marker".to_owned());
    }
    if unknown_failure {
        atoms.insert("parse_signal:unknown_failure_shape".to_owned());
    }
    atoms.into_iter().collect()
}

fn shell_word_band(command: &str) -> &'static str {
    generic_count_band(command.split_whitespace().count())
}

fn cwd_kind(cwd: &str) -> &'static str {
    if cwd.contains("/projects/nando-wave") {
        "nando_wave"
    } else if cwd.contains("/projects/lay") {
        "lay"
    } else if cwd.contains("/projects/") {
        "other_project"
    } else if cwd == "/home/ubu" {
        "home"
    } else {
        "other"
    }
}

fn exit_code_band(exit_code: i64) -> &'static str {
    if exit_code == 0 {
        "zero"
    } else if exit_code == 101 {
        "cargo_101"
    } else if exit_code > 0 {
        "positive_nonzero"
    } else {
        "negative"
    }
}

fn generic_separator_atoms(event: &GenericRealTrafficEvent) -> Vec<(String, String)> {
    let mut atoms = BTreeSet::<(String, String)>::new();
    atoms.insert(("route_key".to_owned(), event.route_key.clone()));
    atoms.insert(("profile_id".to_owned(), event.profile_id.clone()));
    atoms.insert((
        "tool_count_exact".to_owned(),
        event.tool_call_fingerprint_count.to_string(),
    ));
    atoms.insert((
        "tool_count_band".to_owned(),
        generic_count_band(event.tool_call_fingerprint_count).to_owned(),
    ));
    atoms.insert((
        "active_len_exact".to_owned(),
        event.active_fringe.len().to_string(),
    ));
    atoms.insert((
        "active_len_band".to_owned(),
        generic_count_band(event.active_fringe.len()).to_owned(),
    ));
    atoms.insert((
        "slot_count".to_owned(),
        event.slot_summary.len().to_string(),
    ));
    for slot in &event.slot_summary {
        atoms.insert(("slot_shape_exact".to_owned(), slot.clone()));
        atoms.insert(("slot_shape_band".to_owned(), generic_slot_band_atom(slot)));
    }
    for (center_id, strength) in &event.active_fringe {
        atoms.insert((
            "active_center_exact".to_owned(),
            format!("{center_id}:strength:{strength}"),
        ));
        atoms.insert((
            "active_center_page256".to_owned(),
            format!("page:{}:strength:{strength}", center_id / 256),
        ));
        atoms.insert((
            "active_center_page1024".to_owned(),
            format!("page:{}:strength:{strength}", center_id / 1024),
        ));
    }
    atoms.into_iter().collect()
}

impl GenericSeparatorGuardSpec {
    fn to_report(&self) -> GenericSeparatorGuardSpecReport {
        GenericSeparatorGuardSpecReport {
            route_key: self.route_key.clone(),
            profile_id: self.profile_id.clone(),
            atom_family: self.atom_family.clone(),
            atom: self.atom.clone(),
            source_true_over_exact_cache_events: self.source_true_over_exact_cache_events,
            source_verifier_false_events: self.source_verifier_false_events,
            source_shortcut_risk: self.source_shortcut_risk.clone(),
            source_recommended_next_action: self.source_recommended_next_action.clone(),
        }
    }
}

fn selected_separator_guards_from_report(
    report_path: &Path,
    max_guards: usize,
) -> Result<Vec<GenericSeparatorGuardSpec>, String> {
    let report = read_json_file::<serde_json::Value>(report_path)?;
    if report
        .get("report_kind")
        .and_then(serde_json::Value::as_str)
        != Some("generic_real_traffic_phase_center_separator_audit_v1")
    {
        return Err(format!(
            "selector report '{}' is not a separator audit report",
            report_path.display()
        ));
    }
    let local_accept_enabled = report
        .get("local_accept_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let market_money_claim_allowed = report
        .get("market_money_claim_allowed")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let forbidden_flags_ok = report
        .get("forbidden_flags")
        .is_some_and(forbidden_flags_value_all_false);
    if local_accept_enabled || market_money_claim_allowed || !forbidden_flags_ok {
        return Err(format!(
            "selector report '{}' failed boundary checks",
            report_path.display()
        ));
    }
    let Some(candidates) = report
        .get("top_candidates")
        .and_then(serde_json::Value::as_array)
    else {
        return Err(format!(
            "selector report '{}' missing top_candidates",
            report_path.display()
        ));
    };
    let mut guards = Vec::new();
    for candidate in candidates {
        if candidate
            .get("static_clean_on_current_labeled_set")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            continue;
        }
        let Some(route_key) = candidate
            .get("route_key")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let Some(profile_id) = candidate
            .get("profile_id")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let Some(atom_family) = candidate
            .get("atom_family")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let Some(atom) = candidate.get("atom").and_then(serde_json::Value::as_str) else {
            continue;
        };
        guards.push(GenericSeparatorGuardSpec {
            route_key: route_key.to_owned(),
            profile_id: profile_id.to_owned(),
            atom_family: atom_family.to_owned(),
            atom: atom.to_owned(),
            source_true_over_exact_cache_events: json_usize(
                candidate.get("true_over_exact_cache_events"),
            )
            .unwrap_or(0),
            source_verifier_false_events: json_usize(candidate.get("verifier_false_events"))
                .unwrap_or(0),
            source_shortcut_risk: candidate
                .get("shortcut_risk")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_owned(),
            source_recommended_next_action: candidate
                .get("recommended_next_action")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_owned(),
        });
        if guards.len() >= max_guards {
            break;
        }
    }
    Ok(guards)
}

fn selected_separator_guards_from_events(
    events: &[GenericRealTrafficEvent],
    exact_cache_flags: &[bool],
    event_indices: &[usize],
    max_guards: usize,
) -> Vec<GenericSeparatorGuardSpec> {
    let mut states =
        BTreeMap::<(String, String, String, String), GenericSeparatorCandidateState>::new();
    for &event_index in event_indices {
        if event_index >= events.len() {
            continue;
        }
        let event = &events[event_index];
        for (atom_family, atom) in generic_separator_atoms(event) {
            let key = (
                event.route_key.clone(),
                event.profile_id.clone(),
                atom_family.clone(),
                atom.clone(),
            );
            let state = states
                .entry(key)
                .or_insert_with(|| GenericSeparatorCandidateState {
                    route_key: event.route_key.clone(),
                    profile_id: event.profile_id.clone(),
                    atom_family: atom_family.clone(),
                    atom: atom.clone(),
                    ..Default::default()
                });
            state.events += 1;
            if event.verified_safe_accept {
                state.verifier_true_events += 1;
                if !exact_cache_flags.get(event_index).copied().unwrap_or(false) {
                    state.true_over_exact_cache_events += 1;
                    let token_cost = generic_event_token_cost(event);
                    state.token_ceiling_over_exact_cache += token_cost.total_tokens;
                    state.cost_ceiling_microusd_over_exact_cache = state
                        .cost_ceiling_microusd_over_exact_cache
                        .saturating_add(token_cost.total_cost_microusd);
                }
            } else {
                state.verifier_false_events += 1;
            }
            if exact_cache_flags.get(event_index).copied().unwrap_or(false) {
                state.exact_cache_hits += 1;
            }
        }
    }
    let mut candidates = states
        .into_values()
        .filter(|state| state.verifier_false_events == 0)
        .filter(|state| state.true_over_exact_cache_events > 0)
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .true_over_exact_cache_events
            .cmp(&left.true_over_exact_cache_events)
            .then_with(|| {
                right
                    .cost_ceiling_microusd_over_exact_cache
                    .cmp(&left.cost_ceiling_microusd_over_exact_cache)
            })
            .then_with(|| {
                right
                    .token_ceiling_over_exact_cache
                    .cmp(&left.token_ceiling_over_exact_cache)
            })
            .then_with(|| left.route_key.cmp(&right.route_key))
            .then_with(|| left.atom_family.cmp(&right.atom_family))
            .then_with(|| left.atom.cmp(&right.atom))
    });
    candidates
        .into_iter()
        .take(max_guards)
        .map(|candidate| GenericSeparatorGuardSpec {
            route_key: candidate.route_key,
            profile_id: candidate.profile_id,
            atom_family: candidate.atom_family.clone(),
            atom: candidate.atom,
            source_true_over_exact_cache_events: candidate.true_over_exact_cache_events,
            source_verifier_false_events: candidate.verifier_false_events,
            source_shortcut_risk: separator_atom_shortcut_risk(&candidate.atom_family).to_owned(),
            source_recommended_next_action: "split_window_guard_selected_from_selector_only"
                .to_owned(),
        })
        .collect()
}

fn separator_guard_matches_event(
    guard: &GenericSeparatorGuardSpec,
    event: &GenericRealTrafficEvent,
) -> bool {
    if guard.route_key != event.route_key || guard.profile_id != event.profile_id {
        return false;
    }
    generic_separator_atoms(event)
        .into_iter()
        .any(|(family, atom)| family == guard.atom_family && atom == guard.atom)
}

fn separator_guard_bucket_key(guard: &GenericSeparatorGuardSpec) -> String {
    format!(
        "{}::{}::separator_guard_v1:{}:{}",
        guard.profile_id, guard.route_key, guard.atom_family, guard.atom
    )
}

fn separator_atom_shortcut_risk(atom_family: &str) -> &'static str {
    match atom_family {
        "active_center_exact" => "review_exact_active_center_may_be_surface_specific",
        "active_center_page256" => "medium_folded_active_center_page",
        "active_center_page1024" => "lower_coarse_active_center_page",
        "slot_shape_exact" => "medium_slot_shape_specific",
        "active_len_exact" | "tool_count_exact" => "medium_count_specific",
        _ => "low_request_side_family",
    }
}

fn json_field_string(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
}

fn json_usize(value: Option<&serde_json::Value>) -> Option<usize> {
    value
        .and_then(serde_json::Value::as_u64)
        .and_then(|number| usize::try_from(number).ok())
}

fn set_json_usize(
    row: &mut serde_json::Value,
    key: &'static str,
    value: usize,
) -> Result<(), String> {
    let value = u64::try_from(value).map_err(|error| format!("usize conversion error: {error}"))?;
    set_json_u64(row, key, value)
}

fn set_json_u64(row: &mut serde_json::Value, key: &'static str, value: u64) -> Result<(), String> {
    let Some(object) = row.as_object_mut() else {
        return Err(format!("cannot set '{key}' on non-object JSON row"));
    };
    object.insert(
        key.to_owned(),
        serde_json::Value::Number(serde_json::Number::from(value)),
    );
    Ok(())
}

fn set_json_bool(
    row: &mut serde_json::Value,
    key: &'static str,
    value: bool,
) -> Result<(), String> {
    let Some(object) = row.as_object_mut() else {
        return Err(format!("cannot set '{key}' on non-object JSON row"));
    };
    object.insert(key.to_owned(), serde_json::Value::Bool(value));
    Ok(())
}

fn set_json_string_field(
    row: &mut serde_json::Value,
    key: &'static str,
    value: &'static str,
) -> Result<(), String> {
    let Some(object) = row.as_object_mut() else {
        return Err(format!("cannot set '{key}' on non-object JSON row"));
    };
    object.insert(key.to_owned(), serde_json::Value::String(value.to_owned()));
    Ok(())
}

fn set_json_string(
    row: &mut serde_json::Value,
    key: &'static str,
    value: &str,
) -> Result<(), String> {
    let Some(object) = row.as_object_mut() else {
        return Err(format!("cannot set '{key}' on non-object JSON row"));
    };
    object.insert(key.to_owned(), serde_json::Value::String(value.to_owned()));
    Ok(())
}

fn set_nested_token_cost_u64(
    row: &mut serde_json::Value,
    key: &'static str,
    value: u64,
) -> Result<(), String> {
    let Some(object) = row.as_object_mut() else {
        return Err(format!(
            "cannot set token_cost.{key} on non-object JSON row"
        ));
    };
    let token_cost = object
        .entry("token_cost")
        .or_insert_with(|| serde_json::json!({}));
    let Some(token_cost_object) = token_cost.as_object_mut() else {
        return Err(format!(
            "cannot set token_cost.{key} on non-object token_cost"
        ));
    };
    token_cost_object.insert(
        key.to_owned(),
        serde_json::Value::Number(serde_json::Number::from(value)),
    );
    Ok(())
}

fn set_nested_token_cost_usize(
    row: &mut serde_json::Value,
    key: &'static str,
    value: usize,
) -> Result<(), String> {
    let value = u64::try_from(value).map_err(|error| format!("usize conversion error: {error}"))?;
    set_nested_token_cost_u64(row, key, value)
}

fn set_nested_token_cost_bool(
    row: &mut serde_json::Value,
    key: &'static str,
    value: bool,
) -> Result<(), String> {
    let Some(object) = row.as_object_mut() else {
        return Err(format!(
            "cannot set token_cost.{key} on non-object JSON row"
        ));
    };
    let token_cost = object
        .entry("token_cost")
        .or_insert_with(|| serde_json::json!({}));
    let Some(token_cost_object) = token_cost.as_object_mut() else {
        return Err(format!(
            "cannot set token_cost.{key} on non-object token_cost"
        ));
    };
    token_cost_object.insert(key.to_owned(), serde_json::Value::Bool(value));
    Ok(())
}

fn token_floor_cost_microusd(total_tokens: usize, price_config: &ModelPriceConfig) -> u64 {
    (total_tokens as u64)
        .saturating_mul(price_config.input_cost_microusd_per_1k_tokens)
        .div_ceil(1000)
}

fn traffic_source_kind(traffic_source: &str) -> &'static str {
    let lower = traffic_source.to_ascii_lowercase();
    if lower.contains("codex_history") {
        "codex_history"
    } else if lower.contains("real") {
        "real"
    } else {
        "unknown"
    }
}

fn write_trace_jsonl(path: &Path, rows: &[TestOutputTraceRow]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    let mut text = String::new();
    for row in rows {
        let line = serde_json::to_string(row)
            .map_err(|error| format!("failed to serialize trace row: {error}"))?;
        text.push_str(&line);
        text.push('\n');
    }
    std::fs::write(path, text)
        .map_err(|error| format!("failed to write trace jsonl '{}': {error}", path.display()))
}

fn parse_trace_row(index: usize, row: &TestOutputTraceRow) -> Option<ParsedTestOutputEvent> {
    let stdout = row.stdout.clone().unwrap_or_default();
    let stderr = row.stderr.clone().unwrap_or_default();
    let combined = format!("{stdout}\n{stderr}");
    let lower = combined.to_ascii_lowercase();
    let raw_output_available = !stdout.trim().is_empty() || !stderr.trim().is_empty();
    let raw_verification = verify_test_output(&lower);
    let metadata_verification = row.notes.as_deref().and_then(verify_metadata_status);
    let (label, verifier_evidence, command_signal, metadata_verifier_used) =
        if let Some((label, evidence)) = raw_verification {
            (label, evidence, None, false)
        } else if let Some((label, evidence, signal)) = metadata_verification {
            (label, evidence, signal, true)
        } else {
            return None;
        };
    let _event_id = row
        .event_id
        .as_deref()
        .or(row.trace_id.as_deref())
        .map_or_else(|| format!("event_{index:04}"), ToOwned::to_owned);
    let _source = row.source.as_deref().unwrap_or("trace_jsonl");
    let traffic_source = row
        .traffic_source
        .clone()
        .unwrap_or_else(|| "unknown_traffic_source".to_owned());
    let verification_source = row
        .verification_source
        .clone()
        .unwrap_or_else(|| "stdout_stderr_exit_code".to_owned());
    let command = row.command.clone().unwrap_or_else(|| "unknown".to_owned());
    let request_fingerprint = row
        .request_fingerprint
        .clone()
        .unwrap_or_else(|| format!("{:016x}", stable_fingerprint([combined.as_str()])));
    let notes = row.notes.clone().unwrap_or_default();
    Some(ParsedTestOutputEvent {
        command,
        traffic_source,
        verification_source,
        stdout,
        stderr,
        exit_code: row.exit_code,
        tool_call_fingerprint_count: row
            .tool_call_fingerprints
            .as_ref()
            .map_or(0, std::vec::Vec::len),
        request_fingerprint,
        notes,
        provider: row.provider.clone(),
        model_id: row.model_id.clone(),
        input_tokens: row.input_tokens,
        output_tokens: row.output_tokens,
        cached_input_tokens: row.cached_input_tokens,
        provider_cost_microusd: row.provider_cost_microusd,
        explicit_exact_cache_hit: row.exact_cache_hit,
        synthetic_source: row.synthetic_source.unwrap_or(false),
        label,
        verifier_evidence,
        command_signal,
        raw_output_available,
        metadata_verifier_used,
    })
}

fn verify_test_output(lower_output: &str) -> Option<(TestOutputLabel, Vec<String>)> {
    if lower_output.contains("error[e") || lower_output.contains("could not compile") {
        return Some((
            TestOutputLabel::CompileError,
            vec!["compile_error_marker".to_owned()],
        ));
    }
    if lower_output.contains("panicked at") || lower_output.contains("thread 'main' panicked") {
        return Some((
            TestOutputLabel::RuntimePanic,
            vec!["panic_marker".to_owned()],
        ));
    }
    if lower_output.contains("test result: failed")
        || lower_output.contains("failures:")
        || lower_output.contains(" ... failed")
    {
        return Some((TestOutputLabel::Fail, vec!["failed_marker".to_owned()]));
    }
    if lower_output.contains("test result: ok")
        || (lower_output.contains("0 failed") && lower_output.contains("passed"))
    {
        return Some((TestOutputLabel::Pass, vec!["ok_marker".to_owned()]));
    }
    None
}

fn verify_metadata_status(notes: &str) -> Option<(TestOutputLabel, Vec<String>, Option<String>)> {
    let status = metadata_value(notes, "status")?;
    let signal = metadata_value(notes, "signal");
    let verifier_status = metadata_value(notes, "verifier_status");
    let label = match status.as_str() {
        "pass" => TestOutputLabel::Pass,
        "fail" => TestOutputLabel::Fail,
        "compile_error" => TestOutputLabel::CompileError,
        "runtime_panic" | "panic" => TestOutputLabel::RuntimePanic,
        _ => return None,
    };
    let mut evidence = vec![format!("metadata_status_{status}")];
    if let Some(signal) = signal.as_deref() {
        evidence.push(format!("metadata_signal_{signal}"));
    }
    if let Some(verifier_status) = verifier_status.as_deref() {
        evidence.push(format!("metadata_verifier_status_{verifier_status}"));
    }
    Some((label, evidence, signal))
}

fn metadata_value(notes: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    notes.split(';').find_map(|part| {
        part.trim()
            .strip_prefix(&prefix)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    })
}

fn event_vector(
    event: &ParsedTestOutputEvent,
    candidate_label: TestOutputLabel,
    cells: usize,
) -> Vec<nando_core::PhaseCenterCell> {
    let atoms = event_atoms(event, candidate_label);
    phase_vector_from_atoms(atoms.iter().map(String::as_str), cells)
}

fn event_atoms(event: &ParsedTestOutputEvent, candidate_label: TestOutputLabel) -> Vec<String> {
    let mut atoms = vec![
        format!("profile:{PROFILE}"),
        format!("action:{ACTION}"),
        format!("candidate_label:{}", candidate_label.as_str()),
        format!("exit_class:{}", exit_class(event.exit_code)),
        format!("command_kind:{}", command_kind(&event.command)),
        format!("traffic_source:{}", event.traffic_source),
        format!(
            "verification_source:{}",
            verification_source_kind(&event.verification_source)
        ),
        format!(
            "tool_call_fingerprint_count:{}",
            event.tool_call_fingerprint_count
        ),
        format!("raw_output_available:{}", event.raw_output_available),
        format!("metadata_verifier_used:{}", event.metadata_verifier_used),
    ];
    if let Some(signal) = event.command_signal.as_deref() {
        atoms.push(format!("metadata_signal:{signal}"));
    }
    let combined = format!("{}\n{}", event.stdout, event.stderr);
    let lower = combined.to_ascii_lowercase();
    for marker in output_markers(&lower) {
        atoms.push(format!("stdout_marker:{marker}"));
    }
    for evidence in &event.verifier_evidence {
        atoms.push(format!("verifier_evidence:{evidence}"));
    }
    atoms
}

fn output_markers(lower_output: &str) -> Vec<&'static str> {
    let mut markers = Vec::new();
    if lower_output.contains("test result: ok") {
        markers.push("test_result_ok");
    }
    if lower_output.contains("test result: failed") || lower_output.contains("failures:") {
        markers.push("test_result_failed");
    }
    if lower_output.contains("error[e") || lower_output.contains("could not compile") {
        markers.push("compile_error");
    }
    if lower_output.contains("panicked at") || lower_output.contains("thread 'main' panicked") {
        markers.push("panic");
    }
    if lower_output.contains("passed") {
        markers.push("passed_count");
    }
    if lower_output.contains("failed") {
        markers.push("failed_count");
    }
    markers
}

fn command_kind(command: &str) -> &'static str {
    let lower = command.to_ascii_lowercase();
    if lower.contains("cargo test") {
        "cargo_test"
    } else if lower.contains("cargo check") {
        "cargo_check"
    } else {
        "unknown"
    }
}

fn verification_source_kind(verification_source: &str) -> &'static str {
    let lower = verification_source.to_ascii_lowercase();
    if lower.contains("tool-output state") || lower.contains("tool_output") {
        "tool_output_state_metadata"
    } else if lower.contains("stdout") || lower.contains("stderr") {
        "stdout_stderr"
    } else {
        "unknown"
    }
}

fn exit_class(exit_code: Option<i32>) -> &'static str {
    match exit_code {
        Some(0) => "zero",
        Some(_) => "nonzero",
        None => "unknown",
    }
}

fn stratified_train_heldout_indices(events: &[ParsedTestOutputEvent]) -> (Vec<usize>, Vec<usize>) {
    let mut by_label: BTreeMap<TestOutputLabel, Vec<usize>> = BTreeMap::new();
    for (index, event) in events.iter().enumerate() {
        by_label.entry(event.label).or_default().push(index);
    }
    let mut train = Vec::new();
    let mut heldout = Vec::new();
    for indices in by_label.values() {
        if indices.len() == 1 {
            train.push(indices[0]);
            continue;
        }
        let heldout_count = (indices.len() / 4).max(1);
        let train_count = indices.len() - heldout_count;
        train.extend_from_slice(&indices[..train_count]);
        heldout.extend_from_slice(&indices[train_count..]);
    }
    train.sort_unstable();
    heldout.sort_unstable();
    (train, heldout)
}

fn label_to_program_index(
    events: &[ParsedTestOutputEvent],
    train_indices: &[usize],
) -> BTreeMap<TestOutputLabel, usize> {
    let mut label_to_index = BTreeMap::new();
    for &event_index in train_indices {
        let event = &events[event_index];
        let next_index = label_to_index.len();
        label_to_index.entry(event.label).or_insert(next_index);
    }
    label_to_index
}

fn exact_cache_hit_flags(events: &[ParsedTestOutputEvent]) -> Vec<bool> {
    let mut seen = BTreeSet::new();
    let mut flags = Vec::with_capacity(events.len());
    for event in events {
        let fingerprint_hit = !seen.insert(event.request_fingerprint.as_str());
        flags.push(event.explicit_exact_cache_hit == Some(true) || fingerprint_hit);
    }
    flags
}

fn event_proof_scope(event: &ParsedTestOutputEvent) -> &'static str {
    if event.raw_output_available && !event.metadata_verifier_used {
        "raw_output_parse"
    } else if event.metadata_verifier_used {
        "tool_output_state_metadata_parse"
    } else {
        "unproven_scope"
    }
}

fn labels_for_indices(
    events: &[ParsedTestOutputEvent],
    indices: &[usize],
) -> BTreeSet<TestOutputLabel> {
    let mut labels = BTreeSet::new();
    for &index in indices {
        labels.insert(events[index].label);
    }
    labels
}

fn discovery_bucket_key(event: &ParsedTestOutputEvent) -> String {
    format!("{PROFILE}::{}::{}", event_proof_scope(event), ACTION)
}

fn sanitize_file_stem(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn count_labels(events: &[ParsedTestOutputEvent]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for event in events {
        *counts.entry(event.label.as_str().to_owned()).or_insert(0) += 1;
    }
    counts
}

fn count_evidence(events: &[ParsedTestOutputEvent]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for event in events {
        for evidence in &event.verifier_evidence {
            *counts.entry(evidence.clone()).or_insert(0) += 1;
        }
    }
    counts
}

fn fingerprint_exact_cache_hits(events: &[ParsedTestOutputEvent]) -> usize {
    let mut seen = BTreeSet::new();
    let mut hits = 0usize;
    for event in events {
        if !seen.insert(event.request_fingerprint.as_str()) {
            hits += 1;
        }
    }
    hits
}

fn percentile_i64(sorted: &[i64], percentile: usize) -> i64 {
    if sorted.is_empty() {
        return 0;
    }
    let index = (sorted.len() - 1) * percentile / 100;
    sorted[index]
}

fn percentile_u128(sorted: &[u128], percentile: usize) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let index = (sorted.len() - 1) * percentile / 100;
    sorted[index]
}

fn margin_to_micro(margin: f64) -> Result<i64, String> {
    if !margin.is_finite() {
        return Err("non-finite phase margin".to_owned());
    }
    Ok((margin * 1_000_000.0).round() as i64)
}

fn estimate_input_tokens(event: &ParsedTestOutputEvent) -> usize {
    let chars = event.command.chars().count()
        + event.traffic_source.chars().count()
        + event.verification_source.chars().count()
        + event.stdout.chars().count()
        + event.stderr.chars().count()
        + event.notes.chars().count();
    chars.div_ceil(4).max(1)
}

fn estimate_output_tokens(event: &ParsedTestOutputEvent) -> usize {
    event.label.as_str().chars().count().div_ceil(4).max(1)
}

fn event_token_cost(
    event: &ParsedTestOutputEvent,
    price_config: &ModelPriceConfig,
) -> EventTokenCost {
    let input_tokens = event
        .input_tokens
        .unwrap_or_else(|| estimate_input_tokens(event));
    let output_tokens = event
        .output_tokens
        .unwrap_or_else(|| estimate_output_tokens(event));
    let cached_input_tokens = event.cached_input_tokens.unwrap_or(0);
    let estimated_cost = estimated_event_cost_microusd(input_tokens, output_tokens, price_config);
    let total_cost_microusd = event.provider_cost_microusd.unwrap_or(estimated_cost);
    EventTokenCost {
        input_tokens,
        output_tokens,
        cached_input_tokens,
        total_tokens: input_tokens + output_tokens,
        total_cost_microusd,
        token_estimate_used: event.input_tokens.is_none() || event.output_tokens.is_none(),
        cost_estimate_used: event.provider_cost_microusd.is_none(),
    }
}

fn estimated_event_cost_microusd(
    input_tokens: usize,
    output_tokens: usize,
    price_config: &ModelPriceConfig,
) -> u64 {
    let input_cost = (input_tokens as u64)
        .saturating_mul(price_config.input_cost_microusd_per_1k_tokens)
        .div_ceil(1000);
    let output_cost = (output_tokens as u64)
        .saturating_mul(price_config.output_cost_microusd_per_1k_tokens)
        .div_ceil(1000);
    input_cost.saturating_add(output_cost)
}

fn per_thousand(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1000) / denominator
}

fn per_thousand_u64(numerator: u64, denominator: u64) -> usize {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1000).saturating_div(denominator) as usize
}

fn percent_u64(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    (numerator as f64) * 100.0 / (denominator as f64)
}

fn ceil_div_u64(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        return 0;
    }
    numerator.div_ceil(denominator)
}

fn profile_name_from_action_family(action_family: &str) -> &str {
    action_family
        .strip_prefix("action_family:")
        .unwrap_or(action_family)
}

fn diversity_backlog_priority_rank(row: &serde_json::Value) -> u8 {
    match row["priority_class"].as_str().unwrap_or("") {
        "scale_existing_non_top_acceptor" => 0,
        "attach_ranking_trace_for_existing_non_top_acceptor" => 1,
        "repair_zero_global_accept_profile" => 2,
        "capture_verifier_for_high_traffic_family" => 3,
        "trace_needed" => 4,
        "exclude_top_profile_from_diversity_push" => 5,
        _ => 5,
    }
}

fn unique_event_value_or_default<F>(
    events: &[ParsedTestOutputEvent],
    indices: &[usize],
    value: F,
    default: &str,
) -> String
where
    F: Fn(&ParsedTestOutputEvent) -> Option<&str>,
{
    let mut values = BTreeSet::new();
    for &index in indices {
        if let Some(value) = value(&events[index]).filter(|value| !value.is_empty()) {
            values.insert(value.to_owned());
        }
    }
    if values.len() == 1 {
        values
            .into_iter()
            .next()
            .unwrap_or_else(|| default.to_owned())
    } else if values.is_empty() {
        default.to_owned()
    } else {
        "mixed".to_owned()
    }
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse JSON '{}': {error}", path.display()))
}

fn read_json_value(path: &Path) -> Result<serde_json::Value, String> {
    read_json_file(path)
}

fn json_at<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn json_string(value: &serde_json::Value, path: &[&str]) -> Option<String> {
    json_at(value, path)?.as_str().map(ToOwned::to_owned)
}

fn json_bool(value: &serde_json::Value, path: &[&str]) -> Option<bool> {
    json_at(value, path)?.as_bool()
}

fn json_u64(value: &serde_json::Value, path: &[&str]) -> Option<u64> {
    json_at(value, path)?.as_u64()
}

fn json_i64(value: &serde_json::Value, path: &[&str]) -> Option<i64> {
    let value = json_at(value, path)?;
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
}

fn stable_fingerprint<'a, I>(parts: I) -> u64
where
    I: IntoIterator<Item = &'a str>,
{
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize json: {error}"))?;
    std::fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write '{}': {error}", path.display()))
}

fn current_process_rss_kib() -> Option<usize> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix("VmRSS:")
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|value| value.parse::<usize>().ok())
    })
}

fn write_binary_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    std::fs::write(path, bytes)
        .map_err(|error| format!("failed to write '{}': {error}", path.display()))
}

fn default_candidate_package_path(report_path: &Path) -> PathBuf {
    let file_name = report_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("{name}.candidate.nwpc"))
        .unwrap_or_else(|| {
            "online-phase-center-test-output-parse-shadow-v1.candidate.nwpc".to_owned()
        });
    report_path.with_file_name(file_name)
}

fn default_trace_rows() -> Vec<TestOutputTraceRow> {
    let cases = [
        (
            "pass",
            0,
            "running 1 test\ntest smoke::ok_case ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored\n",
        ),
        (
            "fail",
            101,
            "running 1 test\ntest smoke::assert_case ... FAILED\n\nfailures:\n---- smoke::assert_case stdout ----\nassertion failed\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored\n",
        ),
        (
            "compile_error",
            101,
            "error[E0425]: cannot find value `missing_symbol` in this scope\nerror: could not compile `nando-core` due to 1 previous error\n",
        ),
        (
            "runtime_panic",
            101,
            "thread 'main' panicked at crates/nando-core/tests/smoke.rs:10:5:\ncalled `Option::unwrap()` on a `None` value\n",
        ),
    ];
    let mut rows = Vec::new();
    for round in 0..4 {
        for (label, exit_code, stdout) in cases {
            rows.push(TestOutputTraceRow {
                event_id: Some(format!("generated_{round}_{label}")),
                trace_id: None,
                traffic_source: Some("generated_shadow_smoke".to_owned()),
                command: Some("cargo test -p nando-core generated_smoke".to_owned()),
                stdout: Some(stdout.replace("smoke", &format!("smoke_{round}"))),
                stderr: Some(String::new()),
                exit_code: Some(exit_code),
                source: Some("generated_shadow_smoke".to_owned()),
                verification_source: Some("generated stdout/stderr smoke".to_owned()),
                tool_call_fingerprints: None,
                request_fingerprint: None,
                provider: None,
                model_id: None,
                input_tokens: None,
                output_tokens: None,
                cached_input_tokens: None,
                provider_cost_microusd: None,
                exact_cache_hit: Some(false),
                synthetic_source: Some(true),
                notes: None,
            });
        }
    }
    rows
}

fn default_raw_log_paths() -> Vec<PathBuf> {
    [
        "target/check-min.log",
        "target/check-quick.log",
        "target/check-full.log",
        "data/rule_logic_operator_battery_v4/edit/edit_runtime_boundary_gate.log",
        "data/rule_logic_operator_battery_v4/conditional/conditional_runtime_boundary_gate.log",
        "data/rule_logic_operator_battery_v4/conditional/conditional_condition_action_runtime_gate_release.log",
        "data/rule_logic_operator_battery_v4/order/order_runtime_gate_release_l1_short_token_identity.log",
        "data/rule_logic_operator_battery_v4/order/order_runtime_gate_release.log",
        "data/rule_logic_operator_battery_v4/order/order_runtime_gate_release_diagnostic.log",
        "data/rule_logic_operator_battery_v4/order/order_runtime_gate_release_candidate_cleanup.log",
        "data/rule_logic_operator_battery_v4/conditional/conditional_state_channel_runtime_gate_release.log",
        "data/rule_logic_position_sequence_v3/diagnostics/action64_role32/rust_gate.log",
        "data/rule_logic_position_sequence_v3/diagnostics/train_per_cell_4/rust_gate.log",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

fn default_discovery_trace_paths() -> Vec<PathBuf> {
    [
        "target/nando-wave/real-traffic-shadow/test-output-parse-tool-output-state-v1.trace.jsonl",
        "target/nando-wave/real-traffic-shadow/test-output-parse-raw-log-v1.trace.jsonl",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

fn default_generic_real_traffic_trace_paths() -> Vec<PathBuf> {
    [
        "target/nando-wave/real-traffic-shadow/agent-continue-execute-artifact-progress-v1-current5k.trace.jsonl",
        "target/nando-wave/real-traffic-shadow/serving-ops-output-evidence-v1-current5k.trace.jsonl",
        "target/nando-wave/real-traffic-shadow/answer-evidence-output-evidence-v1.trace.jsonl",
        "target/nando-wave/real-traffic-shadow/read-inspect-output-evidence-v1.trace.jsonl",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

fn default_cost_evidence_audit_trace_paths() -> Vec<PathBuf> {
    [
        "target/nando-wave/real-traffic-shadow/git-control-safe-policy-v3-current5k.trace.jsonl",
        "target/nando-wave/real-traffic-shadow/test-output-parse-safe-policy-window-v1-5k.trace.jsonl",
        "target/nando-wave/real-traffic-shadow/agent-continue-execute-artifact-progress-v1-current5k.trace.jsonl",
        "target/nando-wave/real-traffic-shadow/serving-ops-output-evidence-v1-current5k.trace.jsonl",
        "target/nando-wave/real-traffic-shadow/metrics-report-output-evidence-v1-5k.trace.jsonl",
        "target/nando-wave/real-traffic-shadow/read-inspect-output-evidence-v1.trace.jsonl",
        "target/nando-wave/real-traffic-shadow/answer-evidence-output-evidence-v1.trace.jsonl",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}
