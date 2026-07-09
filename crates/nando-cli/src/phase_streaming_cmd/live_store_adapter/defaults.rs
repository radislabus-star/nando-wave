pub(super) const DEFAULT_LIVE_STORE_ADAPTER_SMOKE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-live-store-adapter-smoke-v1.report.json";
pub(super) const DEFAULT_LIVE_STORE_CLEAN_MANIFEST: &str = "target/nando-wave/streaming/phase-stream-live-store-adapter-smoke-v1-clean-promotion-manifest.json";
pub(super) const DEFAULT_LIVE_STORE_CLEAN_MANIFEST_SHADOW_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-shadow-v1.report.json";
pub(super) const DEFAULT_LIVE_STORE_PREPARED_HOT_PACK: &str =
    "target/nando-wave/streaming/phase-stream-live-store-prepared-hot-pack-v1.json";
pub(super) const DEFAULT_LIVE_STORE_PREPARED_HOT_PACK_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-live-store-prepared-hot-pack-v1.report.json";
pub(super) const DEFAULT_LIVE_STORE_MEMORY_HOT_WORKER_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-live-worker-memory-smoke-v1.report.json";
pub(super) const DEFAULT_LIVE_STORE_SOURCE_ADAPTER_WORKER_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-live-source-adapter-worker-v1.report.json";
pub(super) const DEFAULT_LIVE_STORE_WORKER_QUEUE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-live-worker-queue-smoke-v1.report.json";
pub(super) const DEFAULT_LIVE_STORE_WORKER_THREAD_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-live-worker-thread-smoke-v1.report.json";
pub(super) const DEFAULT_LIVE_STORE_WORKER_BATCH_THREAD_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-live-worker-batch-thread-smoke-v1.report.json";
pub(super) const DEFAULT_LIVE_STORE_DIRECT_WORKER_BATCH_THREAD_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-live-store-direct-batch-thread-smoke-v1.report.json";
pub(super) const DEFAULT_HOT_PATH_BENCHMARK_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-hot-path-benchmark-v1.report.json";
pub(super) const DEFAULT_HOT_PATH_DAEMON_ADMISSION_POLICY_REPORT: &str = "target/nando-wave/streaming/phase-stream-hot-path-benchmark-v1-release.report-daemon-admission-policy.json";
pub(super) const DEFAULT_HOT_PATH_DAEMON_ADMISSION_POLICY_SMOKE_REPORT: &str = "target/nando-wave/streaming/phase-stream-hot-path-daemon-admission-policy-smoke-v1.report.json";
pub(super) const DEFAULT_HOT_PATH_DAEMON_SHADOW_GATE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-hot-path-daemon-shadow-gate-v1.report.json";
pub(super) const DEFAULT_HOT_PATH_DAEMON_SHADOW_DECISION_LOG: &str =
    "target/nando-wave/streaming/phase-stream-hot-path-daemon-shadow-decisions-v1.jsonl";
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_SHADOW_GATE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-hot-path-daemon-append-shadow-gate-v1.report.json";
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_SHADOW_DECISION_LOG: &str =
    "target/nando-wave/streaming/phase-stream-hot-path-daemon-append-shadow-decisions-v1.jsonl";
pub(super) const DEFAULT_HOT_PATH_DAEMON_LIVE_LOOP_BUDGET_SMOKE_REPORT: &str = "target/nando-wave/streaming/phase-stream-hot-path-daemon-live-loop-budget-smoke-v1.report.json";
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_LOOP_SMOKE_REPORT: &str = "target/nando-wave/streaming/phase-stream-hot-path-daemon-append-live-loop-smoke-v1.report.json";
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_LOOP_DECISION_LOG: &str =
    "target/nando-wave/streaming/phase-stream-hot-path-daemon-append-live-loop-decisions-v1.jsonl";
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-hot-path-daemon-append-live-tail-v1.report.json";
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_DECISION_LOG: &str =
    "target/nando-wave/streaming/phase-stream-hot-path-daemon-append-live-tail-decisions-v1.jsonl";
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_HEARTBEAT_SECS: u64 = 5;
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_COLD_REFRESH_SECS: u64 = 60;
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_DECISION_FLUSH_ROWS: usize = 64;
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_AUTO_SUBCENTER_ATOMS: usize = 32;
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_CANDIDATE_FRONTIER_LIMIT: usize = 256;
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_HOT_PROFILES_PER_WORKER: usize =
    DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_CANDIDATE_FRONTIER_LIMIT;
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_PROFILES_PER_ROUTE: usize =
    DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_CANDIDATE_FRONTIER_LIMIT;
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_MAX_ROUTE_TOP_K: usize =
    DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_CANDIDATE_FRONTIER_LIMIT;
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_COMPRESSION_CLAIM_MIN_ROWS: usize = 100;
pub(super) const DEFAULT_HOT_PATH_DAEMON_APPEND_LIVE_TAIL_DISCOVERY_SAMPLE_PERMILLE: usize = 100;
pub(super) const DEFAULT_HOT_PATH_DAEMON_LIVE_LOOP_NUMERIC_BENCHMARK_REPORT: &str = "target/nando-wave/streaming/phase-stream-hot-path-daemon-live-loop-numeric-benchmark-v1.report.json";
pub(super) const DEFAULT_HOT_PATH_DAEMON_NUMERIC_PACKAGE_SHADOW_AUDIT_REPORT: &str = "target/nando-wave/streaming/phase-stream-hot-path-daemon-numeric-package-shadow-audit-v1.report.json";
pub(super) const DEFAULT_HOT_PATH_DAEMON_NUMERIC_FUTURE_PACKAGE_AUDIT_REPORT: &str = "target/nando-wave/streaming/phase-stream-hot-path-daemon-numeric-future-package-audit-v1.report.json";
pub(super) const DEFAULT_HOT_PATH_DAEMON_NUMERIC_FUTURE_PORTFOLIO_AUDIT_REPORT: &str = "target/nando-wave/streaming/phase-stream-hot-path-daemon-numeric-future-portfolio-audit-v1.report.json";
pub(super) const DEFAULT_HOT_PATH_DAEMON_NUMERIC_FUTURE_PORTFOLIO_MAX_CHILDREN: usize = 16;
pub(super) const DEFAULT_HOT_PATH_DAEMON_NUMERIC_FALSE_ACCEPT_SPLIT_AUDIT_REPORT: &str = "target/nando-wave/streaming/phase-stream-hot-path-daemon-numeric-false-accept-split-audit-v1.report.json";
pub(super) const DEFAULT_HOT_PATH_DAEMON_NUMERIC_ADMISSION_PORTFOLIO_GATE_REPORT: &str = "target/nando-wave/streaming/phase-stream-hot-path-daemon-numeric-admission-portfolio-gate-v1.report.json";
pub(super) const DEFAULT_HOT_PATH_DAEMON_NUMERIC_ADMISSION_PORTFOLIO_RUNTIME_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-hot-path-daemon-numeric-admission-portfolio-runtime-replay-v1.report.json";
pub(super) const DEFAULT_LIVE_STORE_WORKER_QUEUE_BATCH_CAPACITY: usize = 64;
pub(super) const DEFAULT_LIVE_STORE_DIRECT_HOT_SNAPSHOT_CAPACITY: usize = 64;
pub(super) const DEFAULT_HOT_PATH_BENCHMARK_ITERATIONS: usize = 65_536;
pub(super) const DEFAULT_ONLINE_MINER_NUMERIC_LANE_P99_BUDGET_NS: u128 = 50_000;
