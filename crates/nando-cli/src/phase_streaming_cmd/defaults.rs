pub(super) const DEFAULT_CELLS: usize = 32;
pub(super) const DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_PROFILES_PER_WORKER: usize = 4;
pub(super) const DEFAULT_PHASE_CENTER_SHADOW_MAX_HOT_BYTES_PER_WORKER: usize = 3 * 1024 * 1024;
pub(super) const DEFAULT_PHASE_CENTER_SHADOW_MAX_WARM_PROFILES_PER_PROCESS: usize = 256;
pub(super) const DEFAULT_PHASE_CENTER_SHADOW_MAX_PROFILES_PER_ROUTE: usize = 4;
pub(super) const DEFAULT_PHASE_CENTER_SHADOW_MAX_ROUTE_TOP_K: usize = 4;
pub(super) const DEFAULT_REPORT: &str =
    "target/nando-wave/streaming/online-phase-center-test-output-parse-shadow-v1.report.json";
pub(super) const DEFAULT_PROMOTION_AUDIT_REPORT: &str = "target/nando-wave/streaming/online-phase-center-test-output-parse-promotion-audit-v1.report.json";
pub(super) const DEFAULT_DISCOVERY_REPORT: &str =
    "target/nando-wave/streaming/online-phase-center-discovery-v1.report.json";
pub(super) const DEFAULT_DISCOVERY_PACKAGE_DIR: &str = "target/nando-wave/streaming/discovery";
pub(super) const DEFAULT_ONLINE_DISCOVERY_REPORT: &str =
    "target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json";
pub(super) const DEFAULT_ONLINE_DISCOVERY_PACKAGE_DIR: &str =
    "target/nando-wave/streaming/online-discovery";
pub(super) const DEFAULT_GENERIC_REAL_TRAFFIC_REPORT: &str =
    "target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json";
pub(super) const DEFAULT_GENERIC_REAL_TRAFFIC_PACKAGE_DIR: &str =
    "target/nando-wave/streaming/real-traffic-online-discovery";
pub(super) const DEFAULT_GENERIC_COST_EVIDENCE_AUDIT_REPORT: &str =
    "target/nando-wave/streaming/real-traffic-cost-evidence-audit-v1.report.json";
pub(super) const DEFAULT_GENERIC_TRACE_ENRICHMENT_REPORT: &str =
    "target/nando-wave/streaming/real-traffic-token-cost-enrichment-v1.report.json";
pub(super) const DEFAULT_GENERIC_TRACE_ENRICHMENT_DIR: &str =
    "target/nando-wave/streaming/token-cost-enriched-traces";
pub(super) const DEFAULT_PROVIDER_BILLING_EVIDENCE_JOIN_REPORT: &str =
    "target/nando-wave/streaming/provider-billing-evidence-join-v1.report.json";
pub(super) const DEFAULT_PROVIDER_BILLING_EVIDENCE_JOIN_DIR: &str =
    "target/nando-wave/streaming/provider-billing-enriched-traces";
pub(super) const DEFAULT_GENERIC_FRONTIER_UNION_REPORT: &str =
    "target/nando-wave/streaming/real-traffic-phase-center-frontier-union-v1.report.json";
pub(super) const DEFAULT_GENERIC_CPU10_GAP_AUDIT_REPORT: &str =
    "target/nando-wave/streaming/real-traffic-phase-center-cpu10-gap-audit-v1.report.json";
pub(super) const DEFAULT_GENERIC_SHADOW_REQUEST_GAP_AUDIT_REPORT: &str =
    "target/nando-wave/streaming/real-traffic-phase-center-shadow-request-gap-audit-v1.report.json";
pub(super) const DEFAULT_GENERIC_MINING_INPUT_READINESS_REPORT: &str =
    "target/nando-wave/streaming/real-traffic-phase-center-mining-input-readiness-v1.report.json";
pub(super) const DEFAULT_GENERIC_PHASE_ATOM_TRACE_REPORT: &str =
    "target/nando-wave/streaming/real-traffic-phase-atom-trace-v1.report.json";
pub(super) const DEFAULT_GENERIC_PHASE_ATOM_TRACE_JSONL: &str =
    "target/nando-wave/streaming/real-traffic-phase-atom-trace-v1.jsonl";
pub(super) const DEFAULT_CODEX_HISTORY_PHASE_ATOM_TRACE_REPORT: &str =
    "target/nando-wave/streaming/codex-history-phase-atom-trace-v1.report.json";
pub(super) const DEFAULT_CODEX_HISTORY_PHASE_ATOM_TRACE_JSONL: &str =
    "target/nando-wave/streaming/codex-history-phase-atom-trace-v1.jsonl";
pub(super) const DEFAULT_CODEX_HISTORY_PATH: &str = "/home/ubu/.codex/history.jsonl";
pub(super) const DEFAULT_CODEX_HISTORY_PHASE_ATOM_MAX_ROWS: usize = 5000;
pub(super) const DEFAULT_PHASE_ATOM_VERIFIER_NEEDED_RANKING_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-verifier-needed-ranking-v1.report.json";
pub(super) const DEFAULT_CODEX_SESSION_RUN_CHECK_VERIFIER_REPORT: &str =
    "target/nando-wave/streaming/codex-session-run-check-verifier-trace-v1.report.json";
pub(super) const DEFAULT_CODEX_SESSION_RUN_CHECK_VERIFIER_JSONL: &str =
    "target/nando-wave/streaming/codex-session-run-check-verifier-trace-v1.jsonl";
pub(super) const DEFAULT_CODEX_SESSION_TOOL_STATUS_VERIFIER_REPORT: &str =
    "target/nando-wave/streaming/codex-session-tool-status-verifier-trace-v1.report.json";
pub(super) const DEFAULT_CODEX_SESSION_TOOL_STATUS_VERIFIER_JSONL: &str =
    "target/nando-wave/streaming/codex-session-tool-status-verifier-trace-v1.jsonl";
pub(super) const DEFAULT_CODEX_SESSION_PLANNING_VERIFIER_REPORT: &str =
    "target/nando-wave/streaming/codex-session-planning-verifier-trace-v1.report.json";
pub(super) const DEFAULT_CODEX_SESSION_PLANNING_VERIFIER_JSONL: &str =
    "target/nando-wave/streaming/codex-session-planning-verifier-trace-v1.jsonl";
pub(super) const DEFAULT_CODEX_SESSION_LIVE_APPEND_REPORT: &str =
    "target/nando-wave/streaming/codex-session-live-append-v1.report.json";
pub(super) const DEFAULT_CODEX_SESSION_LIVE_APPEND_JSONL: &str =
    "target/nando-wave/streaming/live-agent-phase-atom-append-v1.jsonl";
pub(super) const DEFAULT_CODEX_SESSION_LIVE_APPEND_HEARTBEAT_SECS: u64 = 5;
pub(super) const DEFAULT_AGENT_CONTINUE_ACTIVE_TURN_STATE_REPORT: &str =
    "target/nando-wave/streaming/agent-continue-active-turn-state-v24.report.json";
pub(super) const DEFAULT_AGENT_CONTINUE_ACTIVE_TURN_STATE_JSONL: &str =
    "target/nando-wave/streaming/agent-continue-active-turn-state-v24.jsonl";
pub(super) const DEFAULT_AGENT_CONTINUE_SUBROUTE_SCOREBOARD_REPORT: &str =
    "target/nando-wave/streaming/agent-continue-subroute-scoreboard-v24.report.json";
pub(super) const DEFAULT_AGENT_CONTINUE_COMMAND_RESULT_FOLLOWUP_PACK_REPORT: &str =
    "target/nando-wave/streaming/agent-continue-command-result-followup-pack-v25.report.json";
pub(super) const DEFAULT_AGENT_CONTINUE_COMMAND_RESULT_FOLLOWUP_PACK_JSONL: &str =
    "target/nando-wave/streaming/agent-continue-command-result-followup-pack-v25.jsonl";
pub(super) const DEFAULT_AUTO_SUBCENTER_DISCOVERY_REPORT: &str =
    "target/nando-wave/streaming/auto-subcenter-discovery-v26.report.json";
pub(super) const DEFAULT_AUTO_SUBCENTER_DISCOVERY_CANDIDATES_JSONL: &str =
    "target/nando-wave/streaming/auto-subcenter-discovery-v26.candidates.jsonl";
pub(super) const DEFAULT_AUTO_SUBCENTER_DISCOVERY_REJECTIONS_JSONL: &str =
    "target/nando-wave/streaming/auto-subcenter-discovery-v26.rejections.jsonl";
pub(super) const DEFAULT_PHASE_ATOM_RUN_CHECK_DISCOVERY_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-run-check-discovery-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_RUN_CHECK_DISCOVERY_PACKAGE: &str =
    "target/nando-wave/streaming/phase-atom-run-check-discovery-v1.candidate.nwpc";
pub(super) const DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_DISCOVERY_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-run-check-time-split-discovery-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_DISCOVERY_PACKAGE: &str =
    "target/nando-wave/streaming/phase-atom-run-check-time-split-discovery-v1.candidate.nwpc";
pub(super) const DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_PROMOTION_AUDIT_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-run-check-time-split-promotion-audit-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_ACTION_FAMILY_TIME_SPLIT_DISCOVERY_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-action-family-time-split-discovery-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_ACTION_FAMILY_TIME_SPLIT_DISCOVERY_PACKAGE: &str =
    "target/nando-wave/streaming/phase-atom-action-family-time-split-discovery-v1.candidate.nwpc";
pub(super) const DEFAULT_PHASE_ATOM_ACTION_FAMILY_SERVING_ADMISSION_AUDIT_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-action-family-serving-admission-audit-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_TOOL_STATUS_SERVING_ADMISSION_AUDIT_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-tool-status-serving-admission-audit-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_SERVING_SHADOW_REPLAY_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-serving-shadow-replay-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_SERVING_FUTURE_SHADOW_REPLAY_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-serving-future-shadow-replay-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_SERVING_APPEND_SHADOW_REPLAY_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-serving-append-shadow-replay-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_LIVE_ADMISSION_MANIFEST_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_LIVE_ADMISSION_POLICY_SMOKE_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_LIVE_DAEMON_SHADOW_GATE_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_LIVE_DAEMON_SHADOW_DECISION_LOG: &str =
    "target/nando-wave/streaming/phase-atom-live-daemon-shadow-decisions-v1.jsonl";
pub(super) const DEFAULT_PHASE_ATOM_LIVE_SELF_MINING_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_LIVE_SELF_MINING_CANDIDATE_DIR: &str =
    "target/nando-wave/streaming/live-self-mining";
pub(super) const DEFAULT_GLOBAL_DENOMINATOR_COMPRESSIBILITY_AUDIT_REPORT: &str =
    "target/nando-wave/streaming/global-denominator-compressibility-audit-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_COMPATIBLE_DENOMINATOR_SHADOW_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_COMPATIBLE_DENOMINATOR_SHADOW_DECISION_LOG: &str =
    "target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-decisions-v1.jsonl";
pub(super) const DEFAULT_PHASE_ATOM_MARKET_MONEY_CLAIM_GATE_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-market-money-claim-gate-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_FRONTIER_SHADOW_REPLAY_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-frontier-shadow-replay-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_FRONTIER_SHADOW_REPLAY_DECISION_LOG: &str =
    "target/nando-wave/streaming/phase-atom-frontier-shadow-replay-decisions-v1.jsonl";
pub(super) const DEFAULT_PHASE_ATOM_FRONTIER_CLAIM_AUDIT_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-frontier-claim-audit-v1.report.json";
pub(super) const DEFAULT_PHASE_ATOM_DIVERSITY_BACKLOG_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-diversity-backlog-v1.report.json";
pub(super) const DEFAULT_CURRENT5K_FEEDBACK_REPORT: &str = "target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1-current5k.combined.report.json";
pub(super) const DEFAULT_PHASE_ATOM_LIVE_SELF_MINING_MULTIFAMILY_V7_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v7.report.json";
pub(super) const DEFAULT_CODEX_SESSION_TOOL_STATUS_APPEND_LATEST_JSONL: &str =
    "target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.jsonl";
pub(super) const DEFAULT_CODEX_SESSIONS_DIR: &str = "/home/ubu/.codex/sessions";
pub(super) const DEFAULT_CODEX_SESSION_RUN_CHECK_MAX_EVENTS: usize = 5000;
pub(super) const DEFAULT_CODEX_SESSION_TOOL_STATUS_MAX_EVENTS: usize = 10000;
pub(super) const DEFAULT_CODEX_SESSION_PLANNING_MAX_EVENTS: usize = 10000;
pub(super) const DEFAULT_PHASE_ATOM_RUN_CHECK_TIME_SPLIT_TRAIN_PERMILLE: usize = 800;
pub(super) const DEFAULT_PHASE_ATOM_LIVE_SELF_MINING_MIN_EVENTS: usize = 20;
pub(super) const DEFAULT_PHASE_ATOM_LIVE_SELF_MINING_TOP_N: usize = 8;
pub(super) const DEFAULT_GENERIC_SEPARATOR_AUDIT_REPORT: &str =
    "target/nando-wave/streaming/real-traffic-phase-center-separator-audit-v1.report.json";
pub(super) const DEFAULT_GENERIC_GUARDED_SEPARATOR_SHADOW_REPORT: &str =
    "target/nando-wave/streaming/real-traffic-phase-center-guarded-separator-shadow-v1.report.json";
pub(super) const DEFAULT_GENERIC_GUARDED_SEPARATOR_PACKAGE_DIR: &str =
    "target/nando-wave/streaming/guarded-separator-shadow";
pub(super) const DEFAULT_GENERIC_GUARDED_SEPARATOR_SPLIT_SHADOW_REPORT: &str = "target/nando-wave/streaming/real-traffic-phase-center-guarded-separator-split-shadow-v1.report.json";
pub(super) const DEFAULT_GENERIC_GUARDED_SEPARATOR_SPLIT_PACKAGE_DIR: &str =
    "target/nando-wave/streaming/guarded-separator-split-shadow";
pub(super) const DEFAULT_GENERIC_GUARDED_SEPARATOR_CALIBRATED_SPLIT_SHADOW_REPORT: &str = "target/nando-wave/streaming/real-traffic-phase-center-guarded-separator-calibrated-split-shadow-v1.report.json";
pub(super) const DEFAULT_GENERIC_GUARDED_SEPARATOR_CALIBRATED_SPLIT_PACKAGE_DIR: &str =
    "target/nando-wave/streaming/guarded-separator-calibrated-split-shadow";
pub(super) const DEFAULT_PRICE_CONFIG: &str = "data/real_traffic/model_price_config.v1.json";
pub(super) const DEFAULT_PROMOTION_MARGIN_THRESHOLD_MICRO: i64 = 100_000;
pub(super) const DEFAULT_SPLIT_SHADOW_MARGIN_THRESHOLD_MICRO: i64 = 200_000;
pub(super) const DEFAULT_CALIBRATION_MARGIN_FLOOR_MICRO: i64 = 150_000;
pub(super) const DEFAULT_CALIBRATION_MARGIN_GUARD_MICRO: i64 = 150_000;
pub(super) const DEFAULT_ONLINE_DISCOVERY_MIN_BUCKET_EVENTS: usize = 4;
pub(super) const PROFILE: &str = "test_output_parse";
pub(super) const VERIFIER_NAME: &str = "test_output_parse_tool_output_verifier";
pub(super) const VERIFIER_VERSION: &str = "v1";
pub(super) const VERIFIER_INPUT_KIND: &str = "stdout_stderr_exit_code_or_verifier_metadata";
pub(super) const VERIFIER_EVIDENCE_SOURCE: &str =
    "stdout/stderr marker rules or request-time tool-output status metadata";
pub(super) const ACCEPT_RULE: &str =
    "shadow_only: score_margin separates verifier-derived label from verifier-false labels";
pub(super) const ACTION: &str = "parse_test_output";
