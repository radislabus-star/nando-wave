//! Verifier-bound local transition serving with explicit upstream abstention.
//!
//! The worker never proxies provider traffic. It either returns a verified
//! local response or HTTP 418, which the surrounding Nginx instance maps to
//! the original upstream request.

mod bridge_health;
mod capture_transition_binding_archive;
mod custom_tool_projection;
mod economics_worker;
pub mod generation_shadow;
mod learning_evidence_bridge;
mod learning_structure_bridge;
mod live_economics;
mod miner_worker;
pub mod multi_source_audit;
mod multi_source_capture;
mod multi_source_frame_archive;
mod multi_source_live;
mod nginx_terminal;
mod opportunity_bridge;
mod provider_capture;
mod request_identity;
mod request_learning;
mod runtime_policy;
pub mod session_backfill;
mod session_stream;
mod stream_evidence;
mod terminal_receipt_archive;
pub use session_stream::{
    verified_capture_bound_training_cases_from_sessions,
    verified_collection_observations_from_session, verified_relation_frames_from_session,
    verified_relation_frames_from_session_tail, verified_session_identity_sha256_candidates,
    verified_training_cases_from_session, verified_training_cases_from_session_head,
    verified_training_cases_from_session_tail, verified_write_stdin_training_cases_from_session,
    verified_write_stdin_training_cases_from_session_for_signatures,
};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use nando_expression_runtime::ExpressionRuntime;
use nando_operator_admission::{
    RuntimePackageRevocationLedgerV1, RuntimePackageRevocationV1, finalize_post_verifier_receipt,
};
use nando_operator_kernel::{RelationFrame, RuntimeProjectionV3, Sha256CommitmentV3};
use nando_operator_learning::{
    EvidencePolicyV1, LearningRequestStructureInputV1, LearningRequestStructureV1,
    OnlineCollectionConfig, OnlineCollectionStatus, OpportunityBridgeEventV1, ReducibilityClass,
    is_source_neutral_relation_frame,
};
use nando_operator_proof::{CpuDecidability, CpuDecidabilityClass, classify_cpu_decidability};
use nando_operator_runtime::{
    ResponseExecutionStatus, provider_tool_capability_atom_ids, request_phase_atom_ids,
    response_pre_action_context_atom_ids,
};
use nando_response_actor::{
    CrystallizedCollectionAdmissionCandidateV1, ONLINE_ADMISSION_CANDIDATE_BUNDLE_SCHEMA_V1,
    OnlineAdmissionCandidateBundle, OnlineCollectionMiner, OnlineResponseMinerReport,
    OnlineResponseStream, OnlineResponseTailConfig, ResponseExecutor, ResponsePackageState,
    ResponseRegistry, response_execution_payload_digest, response_runtime_contract_sha256,
};
use nando_transition_inducer::{
    LIVE_GROUNDED_TRACE_SCHEMA, LIVE_TRANSITION_REQUEST_SCHEMA, LiveTransitionExecutor,
    LiveTransitionRequest, LiveTransitionResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const RESPONSE_PACKAGE_CPU_COUNTER_CAPACITY: usize = 2_048;
const OBSERVATION_DEDUPE_CAPACITY: usize = 65_536;
const OBSERVATION_TRACE_SEGMENT_BYTES: u64 = 64 * 1024 * 1024;
const OBSERVATION_TRACE_SEGMENTS: usize = 4;

use custom_tool_projection::{
    parse_actor_custom_tool_call, responses_projection as custom_tool_responses_projection,
};
use economics_worker::{EconomicsWorkerHandle, spawn_economics_worker};
use generation_shadow::{
    GenerationShadowConfigV3, GenerationShadowIngressV3, GenerationShadowRuntimeV3,
};
use learning_evidence_bridge::LearningEvidenceBridgeRuntimeV1;
use learning_structure_bridge::LearningStructureBridgeRuntimeV2;
use miner_worker::{CollectionMinerPublishedSnapshot, MinerWorkerHandle, spawn_miner_worker};
use opportunity_bridge::OpportunityBridgeRuntime;
use provider_capture::{
    ProviderCaptureConfigV3, ProviderCaptureIngressV3, ProviderCaptureRuntimeV3,
};
use request_identity::ProviderRequestIdentityV1;
use request_learning::RequestLearningIndex;
use runtime_policy::{RuntimePolicyCache, spawn_runtime_policy_watch};
use session_stream::{SessionMinerBridge, SessionStreamMetrics, spawn_session_stream};
use stream_evidence::{SessionEvidenceLedger, StreamingEvidenceLedger};

const OBSERVATION_REQUEST_SCHEMA: &str = "nando.transition-observation.v1";
const EXECUTE_REQUEST_SCHEMA: &str = "nando.transition-execute.v1";
const MAX_REASON_BYTES: usize = 120;
const COLLECTION_SYNTHESIS_GENERATION: u32 = 37;
const OBSERVATION_RESTART_TAIL_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct ServingConfig {
    pub bind: String,
    pub registry_path: PathBuf,
    pub response_registry_path: PathBuf,
    pub runtime_package_revocations_path: PathBuf,
    pub admission_path: PathBuf,
    pub gate_build_path: PathBuf,
    pub runtime_build_path: PathBuf,
    pub project_id: String,
    pub mode_path: PathBuf,
    pub metrics_path: PathBuf,
    pub trace_path: PathBuf,
    pub event_path: PathBuf,
    pub nginx_terminal_path: Option<PathBuf>,
    pub economics_path: PathBuf,
    pub legacy_json_audit_enabled: bool,
    pub kill_switch_path: PathBuf,
    pub max_body_bytes: usize,
    pub admission_max_age_seconds: u64,
    pub refresh_interval_ms: u64,
    pub local_accept_enabled: bool,
    pub client_allow_local_accept: bool,
    pub route_ready: bool,
    pub fallback_managed_by_nginx: bool,
    pub expression_candidate_path: PathBuf,
    pub embedded_response_miner_enabled: bool,
    pub generic_response_miner_enabled: bool,
    pub response_relation_frames_path: PathBuf,
    pub response_online_report_path: PathBuf,
    pub response_online_checkpoint_path: PathBuf,
    pub codex_sessions_path: PathBuf,
    pub streaming_evidence_path: PathBuf,
    pub online_collection_checkpoint_path: PathBuf,
    pub response_admission_candidate_path: PathBuf,
    pub operator_generation_shadow_enabled: bool,
    pub operator_generation_store_path: PathBuf,
    pub operator_generation_capture_index_path: PathBuf,
    pub operator_generation_shadow_queue_capacity: usize,
    pub operator_generation_shadow_poll_ms: u64,
    pub learning_evidence_bridge_socket_path: PathBuf,
    pub learning_evidence_bridge_producer_enabled: bool,
    pub learning_evidence_bridge_consumer_enabled: bool,
    pub learning_evidence_bridge_queue_capacity: usize,
    pub learning_structure_bridge_root_path: PathBuf,
    pub learning_structure_bridge_producer_enabled: bool,
    pub learning_structure_bridge_consumer_enabled: bool,
    pub learning_structure_bridge_poll_ms: u64,
    pub provider_capture_enabled: bool,
    pub provider_capture_store_path: PathBuf,
    pub provider_capture_queue_capacity: usize,
    pub operator_generation_shadow_receipt_store_path: PathBuf,
    pub opportunity_bridge_root_path: PathBuf,
    pub opportunity_bridge_producer_enabled: bool,
    pub opportunity_bridge_consumer_enabled: bool,
    pub opportunity_bridge_poll_ms: u64,
    pub multi_source_snapshot_path: PathBuf,
    pub multi_source_snapshot_poll_ms: u64,
    pub terminal_receipt_archive_path: PathBuf,
    pub multi_source_frame_archive_path: PathBuf,
}

impl ServingConfig {
    pub fn from_env() -> Result<Self, String> {
        let state_dir = env_path(
            "NANDO_TRANSITION_STATE_DIR",
            "/var/lib/nando-wave/transition",
        );
        let collection_checkpoint = format!(
            "online-collection-program-pools-v{COLLECTION_SYNTHESIS_GENERATION}.checkpoint"
        );
        Ok(Self {
            bind: env::var("NANDO_TRANSITION_SERVING_BIND")
                .unwrap_or_else(|_| "127.0.0.1:18789".into()),
            registry_path: env_path_join("NANDO_TRANSITION_REGISTRY", &state_dir, "registry.json"),
            response_registry_path: env_path_join(
                "NANDO_RESPONSE_REGISTRY",
                &state_dir,
                "response-registry.json",
            ),
            runtime_package_revocations_path: env_path_join(
                "NANDO_RUNTIME_PACKAGE_REVOCATIONS",
                &state_dir,
                "runtime-package-revocations.json",
            ),
            admission_path: env_path_join(
                "NANDO_TRANSITION_ADMISSION_JSON",
                &state_dir,
                "admission.json",
            ),
            gate_build_path: env_path(
                "NANDO_LIVE_TRANSITION_GATE_BUILD",
                "/opt/nando-wave/bin/nando-live-transition-gate",
            ),
            runtime_build_path: env_path(
                "NANDO_TRANSITION_SERVING_BUILD",
                "/opt/nando-wave/bin/nando-transition-serving",
            ),
            project_id: env::var("NANDO_PROJECT_ID").unwrap_or_else(|_| "nando-wave".into()),
            mode_path: env_path(
                "NANDO_GATEWAY_PUBLIC_MODE_JSON",
                "/run/nando-gateway-control/mode.json",
            ),
            metrics_path: env_path_join("NANDO_TRANSITION_METRICS", &state_dir, "metrics.json"),
            trace_path: env_path_join(
                "NANDO_TRANSITION_TRACE_JSONL",
                &state_dir,
                "live-transitions.jsonl",
            ),
            event_path: env_path_join(
                "NANDO_TRANSITION_EXECUTION_EVENTS_JSONL",
                &state_dir,
                "execution-events.jsonl",
            ),
            nginx_terminal_path: env::var_os("NANDO_NGINX_TERMINAL_JSONL").map(PathBuf::from),
            economics_path: env_path_join(
                "NANDO_ECONOMICS_LEDGER_JSONL",
                &state_dir,
                "economics-terminal.jsonl",
            ),
            legacy_json_audit_enabled: env_flag("NANDO_LEGACY_JSON_AUDIT_ENABLED"),
            kill_switch_path: env_path(
                "NANDO_TRANSITION_KILL_SWITCH_FILE",
                "/etc/nando-wave/TRANSITION_KILL_SWITCH",
            ),
            max_body_bytes: env_usize("NANDO_TRANSITION_SERVING_MAX_BODY_BYTES", 67_108_864),
            admission_max_age_seconds: env_u64("NANDO_TRANSITION_ADMISSION_MAX_AGE_SECONDS", 30),
            refresh_interval_ms: env_u64("NANDO_TRANSITION_SERVING_REFRESH_MS", 500),
            local_accept_enabled: env_flag("NANDO_LOCAL_ACCEPT_ENABLED"),
            client_allow_local_accept: env_flag("NANDO_CLIENT_ALLOW_LOCAL_ACCEPT"),
            route_ready: env_flag("NANDO_GATEWAY_CPU_ROUTE_READY"),
            fallback_managed_by_nginx: env_flag("NANDO_FALLBACK_MANAGED_BY_NGINX"),
            expression_candidate_path: env_path(
                "NANDO_EXPRESSION_CANDIDATE",
                "/var/lib/nando-wave/expression-shadow/candidate.json",
            ),
            embedded_response_miner_enabled: env::var("NANDO_EMBEDDED_RESPONSE_MINER_ENABLED")
                .map_or(true, |value| {
                    !matches!(value.as_str(), "0" | "false" | "no")
                }),
            generic_response_miner_enabled: env::var("NANDO_GENERIC_RESPONSE_MINER_ENABLED")
                .map_or(true, |value| {
                    !matches!(value.as_str(), "0" | "false" | "no")
                }),
            response_relation_frames_path: env_path_join(
                "NANDO_RESPONSE_RELATION_FRAMES",
                &state_dir,
                "response-relation-frames-v4-verified.jsonl",
            ),
            response_online_report_path: env_path_join(
                "NANDO_RESPONSE_ONLINE_REPORT",
                &state_dir,
                "response-online-miner-report.json",
            ),
            response_online_checkpoint_path: env_path_join(
                "NANDO_RESPONSE_ONLINE_CHECKPOINT",
                &state_dir,
                "response-online-miner.checkpoint",
            ),
            codex_sessions_path: env_path("NANDO_CODEX_SESSIONS_DIR", "/home/ubu/.codex/sessions"),
            streaming_evidence_path: env_path_join(
                "NANDO_STREAMING_EVIDENCE_DIR",
                &state_dir,
                "streaming-evidence-v2",
            ),
            online_collection_checkpoint_path: env_path_join(
                "NANDO_ONLINE_COLLECTION_CHECKPOINT",
                &state_dir,
                &collection_checkpoint,
            ),
            response_admission_candidate_path: env_path_join(
                "NANDO_RESPONSE_ADMISSION_CANDIDATES",
                &state_dir,
                "response-admission-candidates.cbor",
            ),
            operator_generation_shadow_enabled: env_flag(
                "NANDO_OPERATOR_GENERATION_SHADOW_ENABLED",
            ),
            operator_generation_store_path: env_path_join(
                "NANDO_OPERATOR_GENERATION_STORE",
                &state_dir,
                "operator-generation-v3",
            ),
            operator_generation_capture_index_path: env_path_join(
                "NANDO_OPERATOR_GENERATION_CAPTURE_INDEX",
                &state_dir,
                "operator-generation-capture-v3.cbor",
            ),
            operator_generation_shadow_queue_capacity: env_usize(
                "NANDO_OPERATOR_GENERATION_SHADOW_QUEUE",
                32,
            ),
            operator_generation_shadow_poll_ms: env_u64(
                "NANDO_OPERATOR_GENERATION_SHADOW_POLL_MS",
                1_000,
            ),
            learning_evidence_bridge_socket_path: env_path_join(
                "NANDO_LEARNING_EVIDENCE_BRIDGE_SOCKET",
                &state_dir,
                "learning-evidence-bridge-v1/bridge.sock",
            ),
            learning_evidence_bridge_producer_enabled: env_flag(
                "NANDO_LEARNING_EVIDENCE_BRIDGE_PRODUCER_ENABLED",
            ),
            learning_evidence_bridge_consumer_enabled: env_flag(
                "NANDO_LEARNING_EVIDENCE_BRIDGE_CONSUMER_ENABLED",
            ),
            learning_evidence_bridge_queue_capacity: env_usize(
                "NANDO_LEARNING_EVIDENCE_BRIDGE_QUEUE",
                32,
            ),
            learning_structure_bridge_root_path: env_path_join(
                "NANDO_LEARNING_STRUCTURE_BRIDGE_ROOT",
                &state_dir,
                "learning-structure-bridge-v2",
            ),
            learning_structure_bridge_producer_enabled: env_flag(
                "NANDO_LEARNING_STRUCTURE_BRIDGE_PRODUCER_ENABLED",
            ),
            learning_structure_bridge_consumer_enabled: env_flag(
                "NANDO_LEARNING_STRUCTURE_BRIDGE_CONSUMER_ENABLED",
            ),
            learning_structure_bridge_poll_ms: env_u64(
                "NANDO_LEARNING_STRUCTURE_BRIDGE_POLL_MS",
                100,
            ),
            provider_capture_enabled: env_flag("NANDO_PROVIDER_CAPTURE_ENABLED"),
            provider_capture_store_path: env_path_join(
                "NANDO_PROVIDER_CAPTURE_STORE",
                &state_dir,
                "provider-capture-v3-f8a",
            ),
            provider_capture_queue_capacity: env_usize("NANDO_PROVIDER_CAPTURE_QUEUE", 32),
            operator_generation_shadow_receipt_store_path: env_path_join(
                "NANDO_OPERATOR_GENERATION_SHADOW_RECEIPT_STORE",
                &state_dir,
                "operator-generation-shadow-v3-f8b",
            ),
            opportunity_bridge_root_path: env_path_join(
                "NANDO_OPPORTUNITY_BRIDGE_ROOT",
                &state_dir,
                "opportunity-bridge-v1",
            ),
            opportunity_bridge_producer_enabled: env_flag(
                "NANDO_OPPORTUNITY_BRIDGE_PRODUCER_ENABLED",
            ),
            opportunity_bridge_consumer_enabled: env_flag(
                "NANDO_OPPORTUNITY_BRIDGE_CONSUMER_ENABLED",
            ),
            opportunity_bridge_poll_ms: env_u64("NANDO_OPPORTUNITY_BRIDGE_POLL_MS", 100),
            multi_source_snapshot_path: env_path_join(
                "NANDO_MULTI_SOURCE_SNAPSHOT_PATH",
                &state_dir,
                "multi-source-live-v2/snapshot.cbor",
            ),
            multi_source_snapshot_poll_ms: env_u64("NANDO_MULTI_SOURCE_SNAPSHOT_POLL_MS", 15_000)
                .clamp(250, 60_000),
            terminal_receipt_archive_path: env_path_join(
                "NANDO_TERMINAL_RECEIPT_ARCHIVE",
                &state_dir,
                "multi-source-live-v2/terminal-receipt-archive-v1",
            ),
            multi_source_frame_archive_path: env_path_join(
                "NANDO_MULTI_SOURCE_FRAME_ARCHIVE",
                &state_dir,
                "multi-source-live-v2/relation-frame-archive-v1",
            ),
        })
    }
}

struct ExecutorCache {
    executor: Option<Arc<LiveTransitionExecutor>>,
    ready: bool,
    last_error: String,
}

struct ResponseExecutorCache {
    executor: Option<Arc<ResponseExecutor>>,
    ready: bool,
    gate_build_sha256: String,
    runtime_build_sha256: String,
    input_fingerprint: Option<(u64, u128, u64, u128)>,
    embedded_candidate_revision: u64,
    admission_expires_at_unix: u64,
    last_error: String,
}

struct ExpressionShadowCache {
    runtime: Option<Arc<ExpressionRuntime>>,
    ready: bool,
    package_sha256: String,
    last_error: String,
}

impl Default for ExpressionShadowCache {
    fn default() -> Self {
        Self {
            runtime: None,
            ready: false,
            package_sha256: String::new(),
            last_error: "not_loaded".into(),
        }
    }
}

impl Default for ResponseExecutorCache {
    fn default() -> Self {
        Self {
            executor: None,
            ready: false,
            gate_build_sha256: String::new(),
            runtime_build_sha256: String::new(),
            input_fingerprint: None,
            embedded_candidate_revision: 0,
            admission_expires_at_unix: 0,
            last_error: "not_loaded".into(),
        }
    }
}

impl Default for ExecutorCache {
    fn default() -> Self {
        Self {
            executor: None,
            ready: false,
            last_error: "not_loaded".into(),
        }
    }
}

#[derive(Default)]
struct ServingCounters {
    requests: AtomicU64,
    fallbacks: AtomicU64,
    transition_requests: AtomicU64,
    local_accepts: AtomicU64,
    ordinary_response_local_accepts: AtomicU64,
    ordinary_response_local_accept_input_tokens: AtomicU64,
    response_cpu_by_package: Mutex<BTreeMap<String, ResponsePackageCpuCounters>>,
    response_cpu_by_package_overflow: AtomicU64,
    observations: AtomicU64,
    errors: AtomicU64,
    expression_shadow_requests: AtomicU64,
    expression_shadow_would_execute: AtomicU64,
    expression_shadow_observations: AtomicU64,
    expression_shadow_verified_matches: AtomicU64,
    expression_shadow_wrong: AtomicU64,
    expression_shadow_abstains: AtomicU64,
    expression_shadow_cache_unavailable: AtomicU64,
    expression_shadow_potential_input_tokens: AtomicU64,
}

#[derive(Clone, Debug, Default, Serialize)]
struct ResponsePackageCpuCounters {
    accepts: u64,
    ordinary_accepts: u64,
    ordinary_input_tokens: u64,
}

struct ObservationStore {
    trace_path: PathBuf,
    ids: Mutex<ObservationIds>,
    writer: Mutex<()>,
}

#[derive(Default)]
struct ObservationIds {
    digests: BTreeSet<[u8; 32]>,
    insertion_order: VecDeque<[u8; 32]>,
}

impl ObservationIds {
    fn insert(&mut self, digest: [u8; 32]) -> bool {
        if !self.digests.insert(digest) {
            return false;
        }
        self.insertion_order.push_back(digest);
        while self.insertion_order.len() > OBSERVATION_DEDUPE_CAPACITY {
            if let Some(expired) = self.insertion_order.pop_front() {
                self.digests.remove(&expired);
            }
        }
        true
    }

    fn contains(&self, digest: &[u8; 32]) -> bool {
        self.digests.contains(digest)
    }
}

#[derive(Deserialize)]
struct TraceIdOnly<'a> {
    #[serde(borrow)]
    trace_id: Option<&'a str>,
}

impl ObservationStore {
    fn load(trace_path: PathBuf) -> Result<Self, String> {
        let mut ids = ObservationIds::default();
        if trace_path.exists() {
            let mut file = fs::File::open(&trace_path)
                .map_err(|error| format!("trace_open:{}:{error}", trace_path.display()))?;
            let length = file
                .metadata()
                .map_err(|error| format!("trace_metadata:{}:{error}", trace_path.display()))?
                .len();
            let start = length.saturating_sub(OBSERVATION_RESTART_TAIL_BYTES);
            file.seek(SeekFrom::Start(start))
                .map_err(|error| format!("trace_seek:{}:{error}", trace_path.display()))?;
            let mut reader = BufReader::new(file);
            let mut line = String::new();
            if start > 0 {
                reader
                    .read_line(&mut line)
                    .map_err(|error| format!("trace_partial_line:{error}"))?;
                line.clear();
            }
            while reader.read_line(&mut line).unwrap_or(0) != 0 {
                if let Ok(row) = serde_json::from_str::<TraceIdOnly<'_>>(&line)
                    && let Some(trace_id) = row.trace_id
                {
                    ids.insert(trace_id_digest(trace_id));
                }
                line.clear();
            }
        }
        Ok(Self {
            trace_path,
            ids: Mutex::new(ids),
            writer: Mutex::new(()),
        })
    }

    fn append(&self, trace_id: &str, row: &Value) -> Result<bool, String> {
        let mut bytes =
            serde_json::to_vec(row).map_err(|error| format!("trace_json_encode:{error}"))?;
        bytes.push(b'\n');
        let _writer = self
            .writer
            .lock()
            .map_err(|_| "trace_writer_lock_poisoned".to_owned())?;
        let trace_id_digest = trace_id_digest(trace_id);
        {
            let mut ids = self
                .ids
                .lock()
                .map_err(|_| "trace_id_lock_poisoned".to_owned())?;
            if ids.contains(&trace_id_digest) {
                return Ok(false);
            }
            ids.insert(trace_id_digest);
        }
        if let Err(error) = append_rotating_trace(&self.trace_path, &bytes) {
            if let Ok(mut ids) = self.ids.lock() {
                ids.digests.remove(&trace_id_digest);
                ids.insertion_order
                    .retain(|digest| digest != &trace_id_digest);
            }
            return Err(error);
        }
        Ok(true)
    }
}

fn append_rotating_trace(path: &Path, bytes: &[u8]) -> Result<(), String> {
    ensure_parent(path)?;
    let current = path.metadata().map_or(0, |metadata| metadata.len());
    if current > 0
        && current.saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
            > OBSERVATION_TRACE_SEGMENT_BYTES
    {
        for index in (1..OBSERVATION_TRACE_SEGMENTS).rev() {
            let source = trace_segment_path(path, index);
            let destination = trace_segment_path(path, index.saturating_add(1));
            if source.exists() {
                fs::rename(&source, &destination)
                    .map_err(|error| format!("trace_rotate:{}:{error}", source.display()))?;
            }
        }
        let first = trace_segment_path(path, 1);
        fs::rename(path, &first)
            .map_err(|error| format!("trace_rotate:{}:{error}", path.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("trace_append_open:{}:{error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("trace_append_write:{}:{error}", path.display()))?;
    file.flush()
        .map_err(|error| format!("trace_append_flush:{}:{error}", path.display()))
}

fn trace_segment_path(path: &Path, index: usize) -> PathBuf {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    path.with_extension(format!("{extension}.{index}"))
}

fn trace_id_digest(trace_id: &str) -> [u8; 32] {
    Sha256::digest(trace_id.as_bytes()).into()
}

#[derive(Clone)]
struct AppState {
    config: Arc<ServingConfig>,
    cache: Arc<RwLock<ExecutorCache>>,
    response_cache: Arc<RwLock<ResponseExecutorCache>>,
    expression_shadow_cache: Arc<RwLock<ExpressionShadowCache>>,
    observations: Arc<ObservationStore>,
    miners: Arc<RwLock<MinerSlots>>,
    deterministic_evidence: Arc<RwLock<Option<Arc<Mutex<StreamingEvidenceLedger>>>>>,
    miner_warmup: Arc<RwLock<MinerWarmupStatus>>,
    session_stream_metrics: Arc<SessionStreamMetrics>,
    session_miner_bridge: Arc<SessionMinerBridge>,
    request_learning: Arc<RequestLearningIndex>,
    runtime_policy: Arc<RuntimePolicyCache>,
    live_economics: EconomicsWorkerHandle,
    authority_trigger: Arc<Mutex<Option<SyncSender<()>>>>,
    event_lock: Arc<Mutex<()>>,
    counters: Arc<ServingCounters>,
    provider_capture: Arc<ProviderCaptureRuntimeV3>,
    operator_generation_shadow: Arc<GenerationShadowRuntimeV3>,
    learning_evidence_bridge: LearningEvidenceBridgeRuntimeV1,
    learning_structure_bridge: LearningStructureBridgeRuntimeV2,
    opportunity_bridge: OpportunityBridgeRuntime,
    multi_source_snapshot: Arc<
        RwLock<Option<nando_operator_learning::multi_source::LiveMultiSourceDiscoverySnapshotV3>>,
    >,
    terminal_receipt_archive: Option<Arc<Mutex<terminal_receipt_archive::TerminalReceiptArchive>>>,
    multi_source_frame_archive:
        Option<Arc<Mutex<multi_source_frame_archive::MultiSourceFrameArchive>>>,
}

#[derive(Default)]
struct MinerSlots {
    response: Option<Arc<Mutex<OnlineResponseStream>>>,
    collection: Option<Arc<Mutex<OnlineCollectionMiner>>>,
    worker: Option<MinerWorkerHandle>,
}

#[derive(Clone, Debug, Default, Serialize)]
struct MinerWarmupStatus {
    phase: String,
    error: String,
    started_at_unix: u64,
    completed_at_unix: u64,
    checkpoint_restored: bool,
    source_offset: u64,
    source_lines: u64,
    replay_support_after_open: usize,
    checkpoint_path: String,
    checkpoint_sha256_before_open: String,
    checkpoint_sha256_after_open: String,
}

#[derive(Clone, Debug, Serialize)]
struct PolicyStatus {
    mode: String,
    admission_verdict: String,
    admission_eligible: bool,
    admission_fresh: bool,
    local_accept_enabled: bool,
    client_allow_local_accept: bool,
    route_ready: bool,
    kill_switch: bool,
    effective_local_accept: bool,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct ExecuteRequest {
    schema: String,
    before: Value,
    action: Value,
}

#[derive(Debug, Deserialize)]
struct ObservationRequest {
    schema: String,
    #[serde(default)]
    trace_id: Option<String>,
    #[serde(default)]
    observed_at: Option<String>,
    before: Value,
    action: Value,
    after: Value,
    evidence: ObservationEvidence,
    #[serde(default)]
    provenance: Value,
    #[serde(default)]
    usage: Value,
}

#[derive(Debug, Deserialize)]
struct ObservationEvidence {
    source: String,
    verifier: String,
    #[serde(default = "default_receipt_schema")]
    receipt_schema: String,
    receipt_sha256: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeFalseAcceptReport {
    schema: String,
    request_sha256: String,
    package_id: String,
    #[serde(default)]
    reason: String,
}

pub async fn serve(config: ServingConfig) -> Result<(), String> {
    if config.opportunity_bridge_producer_enabled && config.embedded_response_miner_enabled {
        return Err("opportunity_bridge_producer_requires_external_miner".to_owned());
    }
    if config.opportunity_bridge_consumer_enabled && !config.embedded_response_miner_enabled {
        return Err("opportunity_bridge_consumer_requires_embedded_miner".to_owned());
    }
    if config.learning_evidence_bridge_producer_enabled && !config.provider_capture_enabled {
        return Err("learning_evidence_bridge_producer_requires_capture".to_owned());
    }
    if config.learning_structure_bridge_producer_enabled && !config.provider_capture_enabled {
        return Err("learning_structure_bridge_producer_requires_capture".to_owned());
    }
    ensure_parent(&config.trace_path)?;
    ensure_parent(&config.runtime_package_revocations_path)?;
    if config.legacy_json_audit_enabled {
        ensure_parent(&config.event_path)?;
        ensure_parent(&config.economics_path)?;
    }
    if config.embedded_response_miner_enabled {
        ensure_parent(&config.streaming_evidence_path)?;
        ensure_parent(&config.online_collection_checkpoint_path)?;
    }
    let observations = ObservationStore::load(config.trace_path.clone())?;
    let economics_state_dir = config
        .economics_path
        .parent()
        .ok_or_else(|| "live_economics_parent_missing".to_owned())?;
    let live_economics = spawn_economics_worker(economics_state_dir.to_path_buf())?;
    let miner_phase = if config.embedded_response_miner_enabled {
        "pending"
    } else {
        "disabled"
    };
    let runtime_policy = Arc::new(RuntimePolicyCache::load(
        config.mode_path.clone(),
        config.kill_switch_path.clone(),
    ));
    let operator_generation_shadow = Arc::new(GenerationShadowRuntimeV3::new(
        generation_shadow_config(&config),
    )?);
    let provider_capture = Arc::new(ProviderCaptureRuntimeV3::new(provider_capture_config(
        &config,
    ))?);
    let opportunity_bridge = OpportunityBridgeRuntime::new(
        config.opportunity_bridge_root_path.clone(),
        config.opportunity_bridge_producer_enabled,
        config.opportunity_bridge_consumer_enabled,
        Duration::from_millis(config.opportunity_bridge_poll_ms),
    )?;
    let learning_evidence_bridge = LearningEvidenceBridgeRuntimeV1::new(
        config.learning_evidence_bridge_socket_path.clone(),
        config.learning_evidence_bridge_producer_enabled,
        config.learning_evidence_bridge_consumer_enabled,
        config.learning_evidence_bridge_queue_capacity,
    )?;
    let (learning_structure_bridge, request_learning) = LearningStructureBridgeRuntimeV2::open(
        config.learning_structure_bridge_root_path.clone(),
        config.learning_structure_bridge_producer_enabled,
        config.learning_structure_bridge_consumer_enabled,
        Duration::from_millis(config.learning_structure_bridge_poll_ms),
    )?;
    let terminal_receipt_archive = if config.learning_structure_bridge_consumer_enabled {
        match config.nginx_terminal_path.as_deref() {
            Some(source_path) => {
                let mut archive = terminal_receipt_archive::TerminalReceiptArchive::open(
                    &config.terminal_receipt_archive_path,
                )?;
                archive.sync_source(source_path)?;
                Some(Arc::new(Mutex::new(archive)))
            }
            None => None,
        }
    } else {
        None
    };
    let multi_source_frame_archive = if config.learning_structure_bridge_consumer_enabled {
        Some(Arc::new(Mutex::new(
            multi_source_frame_archive::MultiSourceFrameArchive::open(
                &config.multi_source_frame_archive_path,
            )?,
        )))
    } else {
        None
    };
    let state = AppState {
        config: Arc::new(config),
        cache: Arc::new(RwLock::new(ExecutorCache::default())),
        response_cache: Arc::new(RwLock::new(ResponseExecutorCache::default())),
        expression_shadow_cache: Arc::new(RwLock::new(ExpressionShadowCache::default())),
        observations: Arc::new(observations),
        miners: Arc::new(RwLock::new(MinerSlots::default())),
        deterministic_evidence: Arc::new(RwLock::new(None)),
        miner_warmup: Arc::new(RwLock::new(MinerWarmupStatus {
            phase: miner_phase.to_owned(),
            ..MinerWarmupStatus::default()
        })),
        session_stream_metrics: Arc::new(SessionStreamMetrics::default()),
        session_miner_bridge: Arc::new(SessionMinerBridge::new()),
        request_learning,
        runtime_policy,
        live_economics,
        authority_trigger: Arc::new(Mutex::new(None)),
        event_lock: Arc::new(Mutex::new(())),
        counters: Arc::new(ServingCounters::default()),
        provider_capture,
        operator_generation_shadow,
        learning_evidence_bridge,
        learning_structure_bridge,
        opportunity_bridge,
        multi_source_snapshot: Arc::new(RwLock::new(None)),
        terminal_receipt_archive,
        multi_source_frame_archive,
    };
    spawn_runtime_policy_watch(state.runtime_policy.clone())?;
    if state.config.embedded_response_miner_enabled {
        spawn_evidence_runtime(state.clone())?;
    }
    refresh_executor(&state);
    refresh_response_authority(&state);
    spawn_response_authority_runtime(state.clone())?;
    refresh_expression_shadow(&state);
    let max_body_bytes = state.config.max_body_bytes;
    let app = Router::new()
        .route("/health", get(health))
        .route("/health/bridge", get(bridge_health))
        .route("/v2/miner/report", get(miner_report))
        .route("/v2/multi-source/report", get(multi_source_report))
        .route(
            "/v2/multi-source/ms3-failure-corpus",
            get(ms3_failure_corpus_report),
        )
        .route(
            "/v2/multi-source/ms3-representation-gaps",
            get(ms3_representation_gap_report),
        )
        .route("/v1/transitions/execute", post(execute_transition))
        .route("/v2/transitions/execute", post(execute_transition))
        .route("/v1/transitions/observe", post(observe_transition))
        .route("/v2/transitions/observe", post(observe_transition))
        .route(
            "/v2/response-relations/observe",
            post(observe_response_relation),
        )
        .route("/v2/runtime/refresh", post(refresh_runtime))
        .route("/v2/runtime/false-accept", post(report_false_accept))
        .route("/v1/responses", post(openai_responses))
        .route("/v2/responses", post(openai_responses))
        .route("/v1/chat/completions", post(openai_chat))
        .route("/v2/chat/completions", post(openai_chat))
        .fallback(any(fallback_unknown))
        .layer(DefaultBodyLimit::max(max_body_bytes))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(&state.config.bind)
        .await
        .map_err(|error| format!("bind:{}:{error}", state.config.bind))?;
    state.provider_capture.start_after_http_bind();
    state.operator_generation_shadow.start_after_http_bind()?;
    state
        .learning_structure_bridge
        .start_consumer(Arc::clone(&state.request_learning))?;
    state.learning_evidence_bridge.start(
        Arc::clone(&state.operator_generation_shadow),
        Arc::clone(&state.request_learning),
        state.config.learning_structure_bridge_consumer_enabled,
    )?;
    spawn_miner_warmup(state.clone())?;
    spawn_multi_source_snapshot_runtime(state.clone())?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|error| format!("serve:{error}"))
}

fn spawn_miner_warmup(state: AppState) -> Result<(), String> {
    if !state.config.embedded_response_miner_enabled {
        return Ok(());
    }
    std::thread::Builder::new()
        .name("nando-miner-warmup".to_owned())
        .spawn(move || {
            set_miner_warmup(&state, "loading", "", unix_now(), 0);
            if let Ok(mut warmup) = state.miner_warmup.write() {
                warmup.checkpoint_path = state
                    .config
                    .response_online_checkpoint_path
                    .display()
                    .to_string();
                warmup.checkpoint_sha256_before_open =
                    sha256_file_streaming(&state.config.response_online_checkpoint_path)
                        .unwrap_or_default();
            }
            let response = if state.config.generic_response_miner_enabled {
                OnlineResponseStream::open_streaming(OnlineResponseTailConfig {
                    input_path: state.config.response_relation_frames_path.clone(),
                    report_path: state.config.response_online_report_path.clone(),
                    checkpoint_path: state.config.response_online_checkpoint_path.clone(),
                    idle_sleep: Duration::from_millis(200),
                })
                .map(|miner| Some(Arc::new(Mutex::new(miner))))
            } else {
                Ok(None)
            };
            let response = match response {
                Ok(miner) => miner,
                Err(error) => {
                    set_miner_warmup(&state, "failed", &error, 0, unix_now());
                    eprintln!("nando-miner-warmup response: {error}");
                    return;
                }
            };
            if let Some(miner) = response.as_ref()
                && let Ok(miner) = miner.lock()
                && let Ok(mut warmup) = state.miner_warmup.write()
            {
                warmup.checkpoint_restored = miner.checkpoint_restored();
                warmup.source_offset = miner.source_offset();
                warmup.source_lines = miner.source_lines();
                warmup.replay_support_after_open = miner.replay_support_parity_cases_total();
                warmup.checkpoint_sha256_after_open =
                    sha256_file_streaming(&state.config.response_online_checkpoint_path)
                        .unwrap_or_default();
            }
            let collection = match OnlineCollectionMiner::open(
                state.config.online_collection_checkpoint_path.clone(),
                OnlineCollectionConfig::default(),
            ) {
                Ok(miner) => Arc::new(Mutex::new(miner)),
                Err(error) => {
                    set_miner_warmup(&state, "failed", &error, 0, unix_now());
                    eprintln!("nando-miner-warmup collection: {error}");
                    return;
                }
            };
            let worker = if let Some(miner) = response.as_ref() {
                let Some(state_dir) = state
                    .config
                    .response_online_checkpoint_path
                    .parent()
                    .map(Path::to_path_buf)
                else {
                    let error = "response_online_checkpoint_parent_missing";
                    set_miner_warmup(&state, "failed", error, 0, unix_now());
                    eprintln!("nando-miner-warmup worker: {error}");
                    return;
                };
                let authority_trigger = state
                    .authority_trigger
                    .lock()
                    .ok()
                    .and_then(|trigger| trigger.clone());
                match spawn_miner_worker(
                    miner.clone(),
                    collection.clone(),
                    state_dir,
                    authority_trigger,
                    state.multi_source_frame_archive.clone(),
                ) {
                    Ok(worker) => Some(worker),
                    Err(error) => {
                        set_miner_warmup(&state, "failed", &error, 0, unix_now());
                        eprintln!("nando-miner-warmup worker: {error}");
                        return;
                    }
                }
            } else {
                None
            };
            if let Some(worker) = worker.as_ref()
                && let Err(error) = state.session_miner_bridge.install(worker.clone())
            {
                set_miner_warmup(&state, "failed", &error, 0, unix_now());
                eprintln!("nando-miner-warmup bridge: {error}");
                return;
            }
            if let Some(worker) = worker.as_ref()
                && let Err(error) = state.opportunity_bridge.start_consumer(worker.clone())
            {
                set_miner_warmup(&state, "failed", &error, 0, unix_now());
                eprintln!("nando-miner-warmup opportunity bridge: {error}");
                return;
            }
            if let Ok(mut slots) = state.miners.write() {
                *slots = MinerSlots {
                    response,
                    collection: Some(collection),
                    worker,
                };
            }
            set_miner_warmup(&state, "ready", "", 0, unix_now());
        })
        .map(|_| ())
        .map_err(|error| format!("miner_warmup_thread:{error}"))
}

fn sha256_file_streaming(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("checkpoint_hash_open:{}:{error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("checkpoint_hash_read:{}:{error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn set_miner_warmup(
    state: &AppState,
    phase: &str,
    error: &str,
    started_at_unix: u64,
    completed_at_unix: u64,
) {
    if let Ok(mut status) = state.miner_warmup.write() {
        status.phase = phase.to_owned();
        status.error = error.to_owned();
        if started_at_unix > 0 {
            status.started_at_unix = started_at_unix;
        }
        if completed_at_unix > 0 {
            status.completed_at_unix = completed_at_unix;
        }
    }
}

fn spawn_evidence_runtime(state: AppState) -> Result<(), String> {
    std::thread::Builder::new()
        .name("nando-evidence-cold-start".to_owned())
        .spawn(move || {
            let evidence = match StreamingEvidenceLedger::open(
                state.config.streaming_evidence_path.clone(),
                EvidencePolicyV1::streaming_bounded(),
            ) {
                Ok(ledger) => Arc::new(Mutex::new(ledger)),
                Err(error) => {
                    eprintln!("nando-evidence-cold-start ledger: {error}");
                    return;
                }
            };
            if let Ok(mut slot) = state.deterministic_evidence.write() {
                *slot = Some(evidence.clone());
            }
            if !state.config.codex_sessions_path.exists() {
                return;
            }
            if let Err(error) = spawn_session_stream(
                state.config.codex_sessions_path.clone(),
                evidence.clone(),
                state.session_miner_bridge.clone(),
                state.session_stream_metrics.clone(),
                state.request_learning.clone(),
            ) {
                eprintln!("nando-evidence-cold-start session-stream: {error}");
            }
        })
        .map_err(|error| format!("evidence_cold_start_thread:{error}"))?;
    Ok(())
}

fn spawn_response_authority_runtime(state: AppState) -> Result<(), String> {
    let interval = Duration::from_secs(
        state
            .config
            .admission_max_age_seconds
            .saturating_div(3)
            .clamp(1, 30),
    );
    let (sender, receiver) = mpsc::sync_channel(64);
    if let Ok(mut trigger) = state.authority_trigger.lock() {
        *trigger = Some(sender);
    }
    std::thread::Builder::new()
        .name("nando-response-authority".to_owned())
        .spawn(move || {
            loop {
                if matches!(
                    receiver.recv_timeout(interval),
                    Err(mpsc::RecvTimeoutError::Disconnected)
                ) {
                    break;
                }
                while receiver.try_recv().is_ok() {}
                refresh_response_authority(&state);
            }
        })
        .map(|_| ())
        .map_err(|error| format!("response_authority_thread:{error}"))
}

async fn health(State(state): State<AppState>) -> Response {
    let policy = policy_status(&state.config);
    let (cache_ready, revision, active_profiles, cache_error) = cache_status(&state);
    let (response_ready, response_revision, response_profiles, response_error) =
        response_cache_status(&state);
    let runtime_revocations = runtime_package_revocation_health(&state.config);
    let response_admission_expires_at_unix = state
        .response_cache
        .read()
        .map_or(0, |cache| cache.admission_expires_at_unix);
    let (expression_ready, expression_package_sha256, expression_error) =
        expression_shadow_cache_status(&state);
    let response_miner = current_response_miner(&state);
    let miner_worker_handle = current_miner_worker(&state);
    let online = response_miner
        .as_ref()
        .and_then(|miner| miner.try_lock().ok().map(|miner| miner.status()))
        .or_else(|| {
            miner_worker_handle
                .as_ref()
                .and_then(MinerWorkerHandle::response_status)
        });
    let miner_worker = miner_worker_handle.as_ref().map(MinerWorkerHandle::status);
    let miner_warmup = state
        .miner_warmup
        .read()
        .map(|status| status.clone())
        .unwrap_or_default();
    let online_status = json!({
        "enabled": state.config.embedded_response_miner_enabled && state.config.generic_response_miner_enabled,
        "ready": response_miner.is_some(),
        "generic_teacher_action_learning": state.config.generic_response_miner_enabled,
        "rows_seen": online.map_or(0, |status| status.rows_seen),
        "rows_learned": online.map_or(0, |status| status.rows_learned),
        "buckets": online.map_or(0, |status| status.bucket_count),
        "candidates": online.map_or(0, |status| status.candidate_bucket_count),
        "false_accepts": online.map_or(0, |status| status.false_accepts),
        "warm_bytes": online.map_or(0, |status| status.warm_bytes_estimate),
        "source_lines": online.map_or(0, |status| status.source_lines),
        "source_offset": online.map_or(0, |status| status.source_offset),
        "cegis_cohorts": online.map_or(0, |status| status.cegis_cohorts),
        "cegis_winners": online.map_or(0, |status| status.cegis_winners),
        "max_frozen_future_rows": online.map_or(0, |status| status.max_frozen_future_rows),
        "signal_score_out_of_10": online.map_or(0, |status| status.signal_score_out_of_10),
        "opportunity": {
            "ordinary_intents": online.map_or(0, |status| status.opportunity_ordinary_intents),
            "ordinary_tokens": online.map_or(0, |status| status.opportunity_ordinary_tokens),
            "verified_tokens": online.map_or(0, |status| status.opportunity_verified_tokens),
            "verified_share_milli": online.map_or(0, |status| status.opportunity_verified_share_milli),
            "executable_candidate_tokens": online.map_or(0, |status| status.opportunity_executable_candidate_tokens),
            "missing_dsl_tokens": online.map_or(0, |status| status.opportunity_missing_dsl_tokens),
            "missing_verifier_tokens": online.map_or(0, |status| status.opportunity_missing_verifier_tokens),
            "insufficient_repetition_tokens": online.map_or(0, |status| status.opportunity_insufficient_repetition_tokens),
            "unexplored_multi_source_tokens": online.map_or(0, |status| status.opportunity_unexplored_multi_source_tokens),
            "ambiguous_tokens": online.map_or(0, |status| status.opportunity_ambiguous_tokens),
            "non_deterministic_tokens": online.map_or(0, |status| status.opportunity_non_deterministic_tokens),
            "unresolved_tokens": online.map_or(0, |status| status.opportunity_unresolved_tokens),
            "optimistic_upper_bound_share_milli": online.map_or(0, |status| status.opportunity_upper_bound_share_milli),
            "accounting_identity_holds": online.is_some_and(|status| status.opportunity_accounting_identity_holds),
            "m3_reachable": online.is_some_and(|status| status.opportunity_m3_reachable),
        },
        "stream_worker": miner_worker,
    });
    let evidence_ledger = state
        .deterministic_evidence
        .read()
        .ok()
        .and_then(|slot| slot.clone());
    let evidence = evidence_ledger.as_ref().and_then(|ledger| {
        ledger
            .try_lock()
            .ok()
            .map(|ledger| (ledger.accounting(), ledger.recovered_partial_tail_bytes()))
    });
    let evidence_busy = evidence_ledger.is_some() && evidence.is_none();
    let evidence_status = json!({
        "enabled": state.config.embedded_response_miner_enabled,
        "ready": evidence_ledger.is_some(),
        "busy": evidence_busy,
        "ingress_total": evidence.map(|value| value.0.ingress_total),
        "normalized_total": evidence.map(|value| value.0.normalized_total),
        "rejected_total": evidence.map(|value| value.0.rejected_total),
        "duplicate_idempotent_total": evidence.map(|value| value.0.duplicate_idempotent_total),
        "duplicate_conflict_total": evidence.map(|value| value.0.duplicate_conflict_total),
        "accounting_identity": evidence.map(|value| value.0.identity_holds()),
        "recovered_partial_tail_bytes": evidence.map(|value| value.1),
        "raw_payload_persisted": false,
    });
    let (
        turn_graphs_finalized,
        turn_graphs_rejected_overflow,
        censored_invalid_session_identities,
        censored_invalid_utf8_rows,
        session_watcher_alive,
        session_watcher_events,
        session_watcher_last_event_unix,
    ) = state.session_stream_metrics.snapshot();
    let evidence_graph_status = json!({
        "enabled": state.config.embedded_response_miner_enabled,
        "ready": evidence_ledger.is_some(),
        "storage": "source_session_plus_compiled_receipts",
        "graph_total": turn_graphs_finalized,
        "duplicate_graph_total": 0,
        "recovered_partial_tail_bytes": 0,
        "raw_payload_persisted": false,
        "censored_invalid_session_identity": censored_invalid_session_identities,
        "censored_invalid_utf8_rows": censored_invalid_utf8_rows,
        "session_watcher_alive": session_watcher_alive,
        "session_watcher_events": session_watcher_events,
        "session_watcher_last_event_unix": session_watcher_last_event_unix,
    });
    let (bridge_pending, bridge_dropped, bridge_worker_ready) = state.session_miner_bridge.status();
    let collection_miner = current_collection_miner(&state);
    let collection_snapshot = miner_worker_handle
        .as_ref()
        .and_then(MinerWorkerHandle::collection_snapshot)
        .or_else(|| collection_snapshot_without_worker(collection_miner.as_ref()));
    let collection_busy = collection_miner.is_some() && collection_snapshot.is_none();
    let mut collection_status = collection_snapshot.as_ref().map_or_else(
        || json!({}),
        |value| serde_json::to_value(&value.status).unwrap_or_else(|_| json!({})),
    );
    if let Some(object) = collection_status.as_object_mut() {
        object.insert(
            "enabled".to_owned(),
            json!(state.config.embedded_response_miner_enabled),
        );
        object.insert("ready".to_owned(), json!(collection_miner.is_some()));
        object.insert("busy".to_owned(), json!(collection_busy));
        object.insert(
            "quarantine_packages".to_owned(),
            json!(
                collection_snapshot
                    .as_ref()
                    .map(|value| { value.quarantine_packages.as_ref().map_or(0, Vec::len) })
            ),
        );
        object.insert("raw_examples_persisted".to_owned(), json!(false));
    }
    let mut health = json!({
        "ok": true,
        "service": "nando-transition-serving",
        "transport": "rust_cpu_worker",
        "default_client_api_version": "v2",
        "local_accept_enabled": state.config.local_accept_enabled,
        "client_allow_local_accept": state.config.client_allow_local_accept,
        "effective_local_accept_enabled": policy.effective_local_accept,
        "response_effective_local_accept_enabled": response_local_accept_enabled(&state),
        "fallback_managed_by_nginx": state.config.fallback_managed_by_nginx,
        "upstream_configured": state.config.fallback_managed_by_nginx,
        "mode": policy.mode,
        "admission_verdict": policy.admission_verdict,
        "admission_fresh": policy.admission_fresh,
        "executor_cache_ready": cache_ready,
        "registry_revision": revision,
        "transition_profile_state": if cache_ready { "loaded" } else { "unavailable" },
        "transition_active_profiles": active_profiles,
        "transition_false_accepts": 0,
        "cache_error": cache_error,
        "response_executor_cache_ready": response_ready,
        "response_registry_revision": response_revision,
        "response_active_profiles": response_profiles,
        "response_cache_error": response_error,
        "response_online": online_status,
        "expression_shadow_ready": expression_ready,
        "expression_shadow_package_sha256": expression_package_sha256,
        "expression_shadow_error": expression_error,
        "expression_shadow_requests": state.counters.expression_shadow_requests.load(Ordering::Relaxed),
        "expression_shadow_would_execute": state.counters.expression_shadow_would_execute.load(Ordering::Relaxed),
        "expression_shadow_observations": state.counters.expression_shadow_observations.load(Ordering::Relaxed),
        "expression_shadow_verified_matches": state.counters.expression_shadow_verified_matches.load(Ordering::Relaxed),
        "expression_shadow_wrong": state.counters.expression_shadow_wrong.load(Ordering::Relaxed),
        "expression_shadow_abstains": state.counters.expression_shadow_abstains.load(Ordering::Relaxed),
        "expression_shadow_cache_unavailable": state.counters.expression_shadow_cache_unavailable.load(Ordering::Relaxed),
        "expression_shadow_potential_input_tokens": state.counters.expression_shadow_potential_input_tokens.load(Ordering::Relaxed),
        "requests": state.counters.requests.load(Ordering::Relaxed),
        "fallbacks": state.counters.fallbacks.load(Ordering::Relaxed),
        "transition_requests": state.counters.transition_requests.load(Ordering::Relaxed),
        "local_accepts": state.counters.local_accepts.load(Ordering::Relaxed),
        "observations": state.counters.observations.load(Ordering::Relaxed),
        "errors": state.counters.errors.load(Ordering::Relaxed),
    });
    let response_cpu_by_package_overflow = state
        .counters
        .response_cpu_by_package_overflow
        .load(Ordering::Relaxed);
    let (response_cpu_by_package_lock_valid, response_cpu_by_package) = state
        .counters
        .response_cpu_by_package
        .lock()
        .map(|counters| (true, counters.clone()))
        .unwrap_or_else(|_| (false, BTreeMap::new()));
    let response_cpu_by_package_valid =
        response_cpu_by_package_lock_valid && response_cpu_by_package_overflow == 0;
    if let Some(object) = health.as_object_mut() {
        object.insert(
            "response_cpu_by_package_valid".to_owned(),
            json!(response_cpu_by_package_valid),
        );
        object.insert(
            "response_cpu_by_package".to_owned(),
            json!(response_cpu_by_package),
        );
        object.insert(
            "response_cpu_by_package_overflow".to_owned(),
            json!(response_cpu_by_package_overflow),
        );
        object.insert(
            "response_runtime_revocation_state_valid".to_owned(),
            json!(runtime_revocations.valid),
        );
        object.insert(
            "response_runtime_revocations_total".to_owned(),
            json!(runtime_revocations.total),
        );
        object.insert(
            "response_runtime_revocations_unresolved_active".to_owned(),
            json!(runtime_revocations.unresolved_active),
        );
        object.insert(
            "response_runtime_revocation_error".to_owned(),
            json!(runtime_revocations.error),
        );
        object.insert(
            "ordinary_response_local_accepts".to_owned(),
            json!(
                state
                    .counters
                    .ordinary_response_local_accepts
                    .load(Ordering::Relaxed)
            ),
        );
        object.insert(
            "ordinary_response_local_accept_input_tokens".to_owned(),
            json!(
                state
                    .counters
                    .ordinary_response_local_accept_input_tokens
                    .load(Ordering::Relaxed)
            ),
        );
        object.insert(
            "response_admission_expires_at_unix".to_owned(),
            json!(response_admission_expires_at_unix),
        );
        object.insert(
            "response_admission_seconds_remaining".to_owned(),
            json!(response_admission_expires_at_unix.saturating_sub(unix_now())),
        );
    }
    if let Some(object) = health.as_object_mut() {
        object.insert(
            "response_runtime_contract_sha256".to_owned(),
            json!(response_runtime_contract_sha256()),
        );
        object.insert("deterministic_evidence".to_owned(), evidence_status);
        object.insert(
            "deterministic_evidence_graphs".to_owned(),
            evidence_graph_status,
        );
        object.insert("online_collection_miner".to_owned(), collection_status);
        object.insert("miner_warmup".to_owned(), json!(miner_warmup));
        object.insert(
            "turn_graphs_finalized_since_start".to_owned(),
            json!(turn_graphs_finalized),
        );
        object.insert(
            "turn_graphs_rejected_overflow_since_start".to_owned(),
            json!(turn_graphs_rejected_overflow),
        );
        object.insert(
            "session_miner_bridge".to_owned(),
            json!({
                "pending": bridge_pending,
                "dropped": bridge_dropped,
                "worker_ready": bridge_worker_ready,
            }),
        );
        object.insert(
            "economics_worker".to_owned(),
            json!(state.live_economics.status()),
        );
        object.insert(
            "provider_capture".to_owned(),
            json!(state.provider_capture.status()),
        );
        object.insert(
            "operator_generation_shadow".to_owned(),
            json!(state.operator_generation_shadow.status()),
        );
        object.insert(
            "learning_evidence_process_bridge".to_owned(),
            json!(state.learning_evidence_bridge.status()),
        );
        object.insert(
            "opportunity_process_bridge".to_owned(),
            json!(state.opportunity_bridge.status()),
        );
    }
    json_response(StatusCode::OK, health)
}

#[derive(Default)]
struct RuntimePackageRevocationHealth {
    valid: bool,
    total: u64,
    unresolved_active: u64,
    error: Option<String>,
}

fn runtime_package_revocation_health(config: &ServingConfig) -> RuntimePackageRevocationHealth {
    if !config.runtime_package_revocations_path.is_file() {
        return RuntimePackageRevocationHealth {
            valid: true,
            ..RuntimePackageRevocationHealth::default()
        };
    }
    let result = (|| {
        let ledger: RuntimePackageRevocationLedgerV1 = serde_json::from_slice(
            &fs::read(&config.runtime_package_revocations_path)
                .map_err(|error| format!("runtime_package_revocation_read:{error}"))?,
        )
        .map_err(|error| format!("runtime_package_revocation_decode:{error}"))?;
        ledger
            .validate()
            .map_err(|error| format!("runtime_package_revocation_invalid:{error}"))?;
        let registry: ResponseRegistry = serde_json::from_slice(
            &fs::read(&config.response_registry_path)
                .map_err(|error| format!("runtime_package_revocation_registry_read:{error}"))?,
        )
        .map_err(|error| format!("runtime_package_revocation_registry_decode:{error}"))?;
        registry
            .validate()
            .map_err(|error| format!("runtime_package_revocation_registry_invalid:{error}"))?;
        let mut unresolved_active = 0_u64;
        for package in registry
            .packages
            .iter()
            .filter(|package| package.state == ResponsePackageState::Active)
        {
            let payload = response_execution_payload_digest(package)
                .map_err(|error| format!("runtime_package_revocation_payload:{error}"))?;
            if ledger.revokes(&package.package_id, &payload) {
                unresolved_active = unresolved_active.saturating_add(1);
            }
        }
        Ok::<_, String>((ledger.revocations.len() as u64, unresolved_active))
    })();
    match result {
        Ok((total, unresolved_active)) => RuntimePackageRevocationHealth {
            valid: true,
            total,
            unresolved_active,
            error: None,
        },
        Err(error) => RuntimePackageRevocationHealth {
            valid: false,
            total: 0,
            unresolved_active: 0,
            error: Some(error),
        },
    }
}

async fn bridge_health(State(state): State<AppState>) -> Response {
    let snapshot = bridge_health::snapshot(
        &state.learning_evidence_bridge.status(),
        &state.opportunity_bridge.status(),
        &state.operator_generation_shadow.status(),
        state.learning_structure_bridge.status(),
        state.request_learning.status(),
    );
    json_response(
        StatusCode::OK,
        serde_json::to_value(snapshot).unwrap_or_else(|_| {
            json!({
                "schema": bridge_health::BRIDGE_HEALTH_SCHEMA_V2,
                "ok": false,
                "execution_authority": false,
            })
        }),
    )
}

async fn miner_report(State(state): State<AppState>) -> Response {
    let miner_worker = current_miner_worker(&state);
    let response_report = miner_worker
        .as_ref()
        .and_then(MinerWorkerHandle::response_report)
        .or_else(|| {
            current_response_miner(&state)
                .as_ref()
                .and_then(|miner| miner.try_lock().ok().map(|miner| miner.report()))
        });
    let collection_miner = current_collection_miner(&state);
    let collection_snapshot = miner_worker
        .as_ref()
        .and_then(MinerWorkerHandle::collection_snapshot)
        .or_else(|| collection_snapshot_without_worker(collection_miner.as_ref()));
    let collection_report = collection_snapshot.as_ref().map(|snapshot| {
            let status = &snapshot.status;
            let quarantine_packages = &snapshot.quarantine_packages;
            let admission_candidates = &snapshot.admission_candidates;
            let emitted_candidates = admission_candidates.as_ref().map_or(0, Vec::len);
            let explicitly_blocked_candidates = status
                .frozen_buckets_total
                .saturating_sub(status.pre_admission_ready_buckets_total);
            let silent_candidate_losses = status
                .pre_admission_ready_buckets_total
                .saturating_sub(emitted_candidates);
            json!({
                "status": status,
                "quarantine_packages": quarantine_packages.as_ref().map_or(0, |rows| rows.len()),
                "quarantine_error": quarantine_packages.as_ref().err(),
                "admission_ready_candidates": emitted_candidates,
                "admission_candidate_error": admission_candidates.as_ref().err(),
                "candidate_outcomes": {
                    "frozen_cohorts": status.frozen_buckets_total,
                    "emitted_candidates": emitted_candidates,
                    "explicitly_blocked_candidates": explicitly_blocked_candidates,
                    "silent_candidate_losses": silent_candidate_losses,
                    "outcome_identity_holds": status.frozen_buckets_total
                        == emitted_candidates.saturating_add(explicitly_blocked_candidates)
                            .saturating_add(silent_candidate_losses),
                },
                "coverage_contract": "program-explainable evidence is not counted as saved tokens until a verifier-authorized runtime accept",
            })
        });
    let evidence_ledger = state
        .deterministic_evidence
        .read()
        .ok()
        .and_then(|slot| slot.clone());
    let evidence = evidence_ledger.as_ref().and_then(|ledger| {
        ledger.try_lock().ok().map(|ledger| {
            let accounting = ledger.accounting();
            json!({
                "accounting": accounting,
                "accounting_identity": accounting.identity_holds(),
                "recovered_partial_tail_bytes": ledger.recovered_partial_tail_bytes(),
            })
        })
    });
    let economics = state
        .config
        .economics_path
        .parent()
        .and_then(|parent| read_json(&parent.join("economics-live.json")));
    let signal_tree = streaming_miner_signal_tree(
        &state,
        response_report.as_ref(),
        collection_snapshot.as_ref().map(|value| &value.status),
        collection_snapshot
            .as_ref()
            .and_then(|value| value.admission_candidates.as_ref().ok())
            .map_or(0, Vec::len),
        economics.as_ref(),
    );
    json_response(
        StatusCode::OK,
        json!({
            "schema": "nando.streaming-miner-report.v3",
            "generated_at_unix": unix_now(),
            "warmup": state.miner_warmup.read().map(|status| status.clone()).unwrap_or_default(),
            "worker": miner_worker.as_ref().map(MinerWorkerHandle::status),
            "session_bridge": {
                "pending": state.session_miner_bridge.status().0,
                "dropped": state.session_miner_bridge.status().1,
                "worker_ready": state.session_miner_bridge.status().2,
            },
            "response": response_report,
            "collection": collection_report,
            "evidence": evidence,
            "economics": economics,
            "economics_worker": state.live_economics.status(),
            "provider_capture": state.provider_capture.status(),
            "operator_generation_shadow": state.operator_generation_shadow.status(),
            "learning_evidence_process_bridge": state.learning_evidence_bridge.status(),
            "opportunity_process_bridge": state.opportunity_bridge.status(),
            "signal_tree": signal_tree,
            "claim_boundary": "in-memory Rust state and generated snapshots; only admission receipts grant execution authority",
        }),
    )
}

fn collection_snapshot_without_worker(
    miner: Option<&Arc<Mutex<OnlineCollectionMiner>>>,
) -> Option<CollectionMinerPublishedSnapshot> {
    miner.and_then(|miner| {
        miner
            .try_lock()
            .ok()
            .map(|miner| CollectionMinerPublishedSnapshot {
                status: miner.status(),
                quarantine_packages: miner.quarantine_packages(),
                admission_candidates: miner.admission_candidates(),
            })
    })
}

async fn multi_source_report(State(state): State<AppState>) -> Response {
    let snapshot = state
        .multi_source_snapshot
        .read()
        .ok()
        .and_then(|snapshot| snapshot.clone());
    snapshot.map_or_else(
        || {
            json_response(
                StatusCode::SERVICE_UNAVAILABLE,
                json!({
                    "schema": "nando.live-multi-source-error.v1",
                    "error": "snapshot_initializing"
                }),
            )
        },
        |snapshot| {
            json_response(
                StatusCode::OK,
                serde_json::to_value(snapshot).unwrap_or_else(|_| {
                    json!({
                        "schema": "nando.live-multi-source-error.v1",
                        "error": "snapshot_encode"
                    })
                }),
            )
        },
    )
}

async fn ms3_failure_corpus_report(State(state): State<AppState>) -> Response {
    let evidence = current_miner_worker(&state).and_then(|worker| worker.multi_source_evidence());
    let requests = state.request_learning.audit_snapshot_v1();
    let terminals = requests
        .as_ref()
        .map_err(|error| (*error).to_owned())
        .and_then(|requests| archived_terminal_receipts(&state, requests));
    let frames = requests
        .as_ref()
        .map_err(|error| (*error).to_owned())
        .and_then(|requests| archived_relation_frames(&state, requests));
    match (evidence, requests, terminals, frames) {
        (Some(_), Ok(requests), Ok(terminals), Ok(frames)) => {
            // This route exposes proof roots and typed dispositions only. It is
            // read-only and cannot promote, compile, or authorize an operator.
            let corpus = nando_operator_learning::multi_source::build_ms3_failure_corpus_v1(
                requests, frames, terminals,
            );
            if !corpus.validate() {
                return json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({
                        "schema": "nando.ms3-failure-corpus-error.v1",
                        "error": "corpus_invalid"
                    }),
                );
            }
            json_response(
                StatusCode::OK,
                serde_json::to_value(corpus).unwrap_or_else(|_| {
                    json!({
                        "schema": "nando.ms3-failure-corpus-error.v1",
                        "error": "corpus_encode"
                    })
                }),
            )
        }
        (_, _, Err(error), _) | (_, _, _, Err(error)) => json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({
                "schema": "nando.ms3-failure-corpus-error.v1",
                "error": error
            }),
        ),
        _ => json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({
                "schema": "nando.ms3-failure-corpus-error.v1",
                "error": "evidence_inputs_pending"
            }),
        ),
    }
}

async fn ms3_representation_gap_report(State(state): State<AppState>) -> Response {
    let evidence = current_miner_worker(&state).and_then(|worker| worker.multi_source_evidence());
    let requests = state.request_learning.audit_snapshot_v1();
    let terminals = requests
        .as_ref()
        .map_err(|error| (*error).to_owned())
        .and_then(|requests| archived_terminal_receipts(&state, requests));
    let frames = requests
        .as_ref()
        .map_err(|error| (*error).to_owned())
        .and_then(|requests| archived_relation_frames(&state, requests));
    match (evidence, requests, terminals, frames) {
        (Some(_), Ok(requests), Ok(terminals), Ok(frames)) => {
            let report =
                nando_operator_learning::multi_source::build_representation_gap_adjudication_report_v1(
                    requests,
                    frames,
                    terminals,
                );
            if !report.validate() {
                return json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({
                        "schema": "nando.representation-gap-adjudication-error.v1",
                        "error": "report_invalid"
                    }),
                );
            }
            json_response(
                StatusCode::OK,
                serde_json::to_value(report).unwrap_or_else(|_| {
                    json!({
                        "schema": "nando.representation-gap-adjudication-error.v1",
                        "error": "report_encode"
                    })
                }),
            )
        }
        (_, _, Err(error), _) | (_, _, _, Err(error)) => json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({
                "schema": "nando.representation-gap-adjudication-error.v1",
                "error": error
            }),
        ),
        _ => json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({
                "schema": "nando.representation-gap-adjudication-error.v1",
                "error": "evidence_inputs_pending"
            }),
        ),
    }
}

fn spawn_multi_source_snapshot_runtime(state: AppState) -> Result<(), String> {
    std::thread::Builder::new()
        .name("nando-multi-source-snapshot".to_owned())
        .spawn(move || {
            let mut published = false;
            loop {
                if let (Some(archive), Some(source_path)) = (
                    state.terminal_receipt_archive.as_ref(),
                    state.config.nginx_terminal_path.as_deref(),
                ) && let Err(error) = archive
                    .lock()
                    .map_err(|_| "terminal_archive_lock_poisoned".to_owned())
                    .and_then(|mut archive| archive.sync_source(source_path))
                {
                    eprintln!("nando-terminal-receipt-archive: {error}");
                }
                let snapshot = if state.config.embedded_response_miner_enabled {
                    let evidence = current_miner_worker(&state)
                        .and_then(|worker| worker.multi_source_evidence());
                    let requests = state.request_learning.audit_snapshot_v1();
                    let active_protocols = multi_source_live::active_protocol_mode_roots(
                        &state.config.response_registry_path,
                    );
                    match (evidence, requests, active_protocols) {
                        (Some(evidence), Ok(requests), Ok(active_protocols)) => {
                            archived_relation_frames(&state, &requests).and_then(|frames| {
                                multi_source_live::build_snapshot(
                                    evidence.opportunities,
                                    requests,
                                    frames,
                                    &active_protocols,
                                )
                                .and_then(|snapshot| {
                                    multi_source_live::write_snapshot(
                                        &state.config.multi_source_snapshot_path,
                                        &snapshot,
                                    )?;
                                    Ok(snapshot)
                                })
                            })
                        }
                        _ => Err("live_multi_source_snapshot_inputs_pending".to_owned()),
                    }
                } else {
                    multi_source_live::read_snapshot(&state.config.multi_source_snapshot_path)
                };
                if let Ok(snapshot) = snapshot {
                    if let Ok(mut target) = state.multi_source_snapshot.write() {
                        *target = Some(snapshot);
                    }
                    published = true;
                }
                let retry_ms = if published {
                    state.config.multi_source_snapshot_poll_ms
                } else {
                    state.config.multi_source_snapshot_poll_ms.min(1_000)
                };
                std::thread::sleep(Duration::from_millis(retry_ms));
            }
        })
        .map(|_| ())
        .map_err(|error| format!("multi_source_snapshot_thread:{error}"))
}

fn archived_terminal_receipts(
    state: &AppState,
    requests: &nando_operator_learning::multi_source::RequestStructureAuditSnapshotV1,
) -> Result<Vec<nando_operator_learning::multi_source::TransportTerminalReceiptV1>, String> {
    let request_ids = requests
        .topologies
        .iter()
        .map(|row| row.structure.request_event_id_sha256.clone())
        .collect::<BTreeSet<_>>();
    state
        .terminal_receipt_archive
        .as_ref()
        .ok_or_else(|| "terminal_receipt_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "terminal_archive_lock_poisoned".to_owned())
        .map(|archive| archive.receipts_for_requests(&request_ids))
}

fn archived_relation_frames(
    state: &AppState,
    requests: &nando_operator_learning::multi_source::RequestStructureAuditSnapshotV1,
) -> Result<Vec<RelationFrame>, String> {
    let intent_ids = requests
        .topologies
        .iter()
        .map(|row| row.structure.turn_intent_id_sha256.clone())
        .collect::<BTreeSet<_>>();
    state
        .multi_source_frame_archive
        .as_ref()
        .ok_or_else(|| "multi_source_frame_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "multi_source_frame_archive_lock_poisoned".to_owned())
        .map(|archive| archive.frames_for_intents(&intent_ids))
}

fn streaming_miner_signal_tree(
    state: &AppState,
    response: Option<&OnlineResponseMinerReport>,
    collection: Option<&OnlineCollectionStatus>,
    collection_admission_candidates: usize,
    economics: Option<&Value>,
) -> Value {
    let training = response.map(|report| &report.self_training_v2);
    let transitions = training
        .map_or(0, |report| report.transitions_seen)
        .max(collection.map_or(0, |report| report.observations_total));
    let teacher_pools = training
        .map_or(0, |report| report.discovery.teacher_pool_count)
        .max(collection.map_or(0, |report| report.buckets_total));
    let invariants = training
        .map_or(0, |report| {
            report.discovery.invariant_candidates.max(
                report
                    .cegis
                    .pools
                    .iter()
                    .filter(|pool| pool.winner)
                    .map(|pool| pool.invariant_count)
                    .sum(),
            )
        })
        .max(collection.map_or(0, |report| {
            report
                .buckets
                .iter()
                .map(|bucket| bucket.common_request_atoms)
                .sum()
        }));
    let winners = training
        .map_or(0, |report| report.cegis.winners)
        .max(collection.map_or(0, |report| report.frozen_buckets_total));
    let best_generation = training.and_then(|report| {
        report.generations.iter().max_by_key(|generation| {
            (
                generation.blocker.is_none(),
                generation.future_sessions,
                generation.future_rows,
                generation.support_rows,
            )
        })
    });
    let best_collection_bucket = collection.and_then(|report| {
        report.buckets.iter().max_by_key(|bucket| {
            (
                bucket.admission_blocker.is_none(),
                bucket.future_sessions,
                bucket.future_rows,
                bucket.support_rows,
            )
        })
    });
    let legacy_future = best_generation.map(|generation| {
        (
            generation.blocker.is_none(),
            generation.future_sessions,
            generation.future_rows,
            generation.support_rows,
            generation.blocker.clone(),
        )
    });
    let collection_future = best_collection_bucket.map(|bucket| {
        (
            bucket.admission_blocker.is_none(),
            bucket.future_sessions,
            bucket.future_rows,
            bucket.support_rows,
            bucket.admission_blocker.clone(),
        )
    });
    let best_future = [legacy_future, collection_future]
        .into_iter()
        .flatten()
        .max_by_key(|value| (value.0, value.1, value.2, value.3));
    let future_rows = best_future.as_ref().map_or(0, |value| value.2);
    let frozen_future_blocker =
        best_future.map_or_else(|| Some("no_frozen_generation".to_owned()), |value| value.4);
    let candidate_ready = training
        .map_or(0, |report| report.admission_ready_cohorts)
        .max(collection_admission_candidates);
    let (_, _, active_packages, _) = response_cache_status(state);
    let active_programs = response_active_program_labels(state);
    let mut discovered_programs = training
        .map(|report| {
            report
                .discovery
                .teacher_pools
                .iter()
                .map(|pool| pool.action_symbol.clone())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    if let Some(collection) = collection {
        discovered_programs.extend(
            collection
                .frozen_program_kinds
                .iter()
                .filter(|(_, count)| **count > 0)
                .map(|(kind, _)| kind.clone()),
        );
    }
    let discovered_program_count = discovered_programs.len();
    let active_program_count = active_programs.len();
    let token_share_milli = economics
        .and_then(|value| value.get("input_token_saving_share_milli"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let verified_accepts = economics
        .and_then(|value| value.get("verified_local_accepts"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let false_accepts = economics
        .and_then(|value| value.get("false_accepts"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let parity_failures = economics
        .and_then(|value| value.get("runtime_parity_mismatches"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let (bridge_pending, bridge_dropped, bridge_ready) = state.session_miner_bridge.status();
    let runtime_ready = response_local_accept_enabled(state);
    let stages = vec![
        signal_tree_stage(
            "capture",
            transitions,
            if bridge_ready && bridge_dropped == 0 && transitions > 0 {
                10
            } else if bridge_ready {
                3
            } else {
                0
            },
            (transitions == 0 || !bridge_ready || bridge_dropped != 0).then(|| {
                format!(
                    "transitions={transitions};bridge_ready={bridge_ready};dropped={bridge_dropped};pending={bridge_pending}"
                )
            }),
        ),
        signal_tree_stage(
            "teacher_grouping",
            u64::try_from(teacher_pools).unwrap_or(u64::MAX),
            u8::try_from(teacher_pools.min(5).saturating_mul(2)).unwrap_or(10),
            (teacher_pools < 5).then(|| format!("teacher_program_pools_below_5:{teacher_pools}")),
        ),
        signal_tree_stage(
            "wave_invariants",
            u64::try_from(invariants).unwrap_or(u64::MAX),
            u8::try_from(invariants.min(8).saturating_mul(10) / 8).unwrap_or(10),
            (invariants < 8).then(|| format!("wave_invariants_below_8:{invariants}")),
        ),
        signal_tree_stage(
            "typed_synthesis",
            u64::try_from(discovered_program_count).unwrap_or(u64::MAX),
            u8::try_from(discovered_program_count.min(5).saturating_mul(2)).unwrap_or(10),
            (discovered_program_count < 5).then(|| {
                format!("typed_program_families_below_5:{discovered_program_count}")
            }),
        ),
        signal_tree_stage(
            "cegis",
            u64::try_from(winners).unwrap_or(u64::MAX),
            u8::try_from(winners.min(5).saturating_mul(2)).unwrap_or(10),
            (winners < 5).then(|| format!("cegis_winners_below_5:{winners}")),
        ),
        signal_tree_stage(
            "frozen_future",
            u64::try_from(future_rows).unwrap_or(u64::MAX),
            if frozen_future_blocker.is_none() {
                10
            } else {
                u8::try_from(future_rows.min(32).saturating_mul(9) / 32).unwrap_or(9)
            },
            frozen_future_blocker,
        ),
        signal_tree_stage(
            "candidate_bundle",
            u64::try_from(candidate_ready).unwrap_or(u64::MAX),
            u8::try_from(candidate_ready.min(4).saturating_mul(10) / 4).unwrap_or(10),
            (candidate_ready < 4)
                .then(|| format!("proof_carrying_candidates_below_4:{candidate_ready}")),
        ),
        signal_tree_stage(
            "external_admission",
            u64::try_from(active_packages).unwrap_or(u64::MAX),
            u8::try_from(active_program_count.min(5).saturating_mul(2)).unwrap_or(10),
            (active_program_count < 5)
                .then(|| format!("active_program_families_below_5:{active_program_count}")),
        ),
        signal_tree_stage(
            "cpu_serving",
            verified_accepts,
            if runtime_ready && verified_accepts > 0 {
                1_u8.saturating_add(
                    u8::try_from(verified_accepts.min(100).saturating_mul(9) / 100)
                        .unwrap_or(9),
                )
            } else {
                0
            },
            if !runtime_ready {
                Some("runtime_local_accept_not_ready".to_owned())
            } else {
                (verified_accepts < 100)
                    .then(|| format!("verified_cpu_accepts_below_100:{verified_accepts}"))
            },
        ),
        signal_tree_stage(
            "verified_economics",
            verified_accepts,
            u8::try_from(token_share_milli.min(500).saturating_mul(10) / 500).unwrap_or(10),
            (token_share_milli < 500 || false_accepts != 0 || parity_failures != 0).then(|| {
                format!(
                    "token_share_milli={token_share_milli};false_accepts={false_accepts};parity_failures={parity_failures}"
                )
            }),
        ),
    ];
    let pipeline_stages = stages.len().saturating_sub(1);
    let pipeline_score = stages[..pipeline_stages]
        .iter()
        .filter_map(|stage| stage.get("score_out_of_10").and_then(Value::as_u64))
        .sum::<u64>()
        / u64::try_from(pipeline_stages).unwrap_or(1).max(1);
    let economics_score = stages[pipeline_stages]
        .get("score_out_of_10")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let critical_path_break = stages
        .iter()
        .find(|stage| {
            stage
                .get("blocker")
                .is_some_and(|blocker| !blocker.is_null())
        })
        .and_then(|stage| stage.get("stage"))
        .cloned();
    let overall_score = pipeline_score.min(economics_score);
    json!({
        "schema": "nando.streaming-miner-signal-tree.v2",
        "overall_score_out_of_10": overall_score,
        "technical_score_out_of_10": pipeline_score,
        "commercial_score_out_of_10": economics_score,
        "critical_path_break": critical_path_break,
        "discovered_programs": discovered_programs,
        "active_programs": active_programs,
        "stages": stages,
    })
}

fn signal_tree_stage(stage: &str, rows: u64, score: u8, blocker: Option<String>) -> Value {
    let blocker_ru = blocker.as_deref().map(signal_tree_blocker_ru);
    json!({
        "stage": stage,
        "stage_ru": signal_tree_stage_ru(stage),
        "verdict": if blocker.is_none() { "PASS" } else if rows > 0 { "WATCH" } else { "BLOCK" },
        "score_out_of_10": score.min(10),
        "rows": rows,
        "blocker": blocker,
        "blocker_ru": blocker_ru,
    })
}

fn signal_tree_stage_ru(stage: &str) -> &'static str {
    match stage {
        "capture" => "Сбор завершённых трасс",
        "teacher_grouping" => "Самообучающаяся группировка",
        "wave_invariants" => "Волновые инварианты",
        "typed_synthesis" => "Синтез типизированных программ",
        "cegis" => "Исправление по контрпримерам",
        "frozen_future" => "Независимое будущее окно",
        "candidate_bundle" => "Пакет с доказательствами",
        "external_admission" => "Внешний допуск",
        "cpu_serving" => "Реальные ответы на процессоре",
        "verified_economics" => "Проверенная экономия токенов",
        _ => "Неизвестный этап",
    }
}

fn signal_tree_blocker_ru(blocker: &str) -> String {
    let value = blocker.rsplit(':').next().unwrap_or("0");
    if blocker.starts_with("teacher_program_pools_below_5") {
        format!("Нужно 5 разных учебных программ, сейчас {value}")
    } else if blocker.starts_with("wave_invariants_below_8") {
        format!("Нужно 8 чистых волновых инвариантов, сейчас {value}")
    } else if blocker.starts_with("typed_program_families_below_5") {
        format!("Нужно 5 разных семейств типизированных программ, сейчас {value}")
    } else if blocker.starts_with("cegis_winners_below_5") {
        format!("Нужно 5 программ, переживших контрпримеры, сейчас {value}")
    } else if blocker.starts_with("proof_carrying_candidates_below_4") {
        format!("Нужно 4 кандидата с доказательствами, сейчас {value}")
    } else if blocker.starts_with("active_program_families_below_5") {
        format!("Нужно 5 разных допущенных семейств, сейчас {value}")
    } else if blocker.starts_with("verified_cpu_accepts_below_100") {
        format!("Нужно 100 независимо проверенных CPU-ответов, сейчас {value}")
    } else if blocker == "runtime_local_accept_not_ready" {
        "Локальное исполнение сейчас не имеет authority".to_owned()
    } else if blocker == "no_frozen_generation" {
        "Нет поколения с независимым будущим окном".to_owned()
    } else if blocker.contains("future_rows_below") {
        "Недостаточно независимых строк после заморозки support".to_owned()
    } else if blocker.starts_with("token_share_milli=") {
        "Проверенная экономия токенов ниже 50% либо нарушен safety-контракт".to_owned()
    } else if blocker.starts_with("transitions=") {
        "Поток трасс пуст, мост не готов или теряет события".to_owned()
    } else {
        format!("Техническая причина: {blocker}")
    }
}

async fn execute_transition(State(state): State<AppState>, body: Bytes) -> Response {
    state.counters.requests.fetch_add(1, Ordering::Relaxed);
    state
        .counters
        .transition_requests
        .fetch_add(1, Ordering::Relaxed);
    let request_hash = sha256_bytes(&body);
    let request = match serde_json::from_slice::<ExecuteRequest>(&body) {
        Ok(request) => request,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid_json"),
    };
    if request.schema != EXECUTE_REQUEST_SCHEMA {
        return error_response(StatusCode::BAD_REQUEST, "unsupported_schema");
    }
    record_expression_shadow_request(
        &state,
        &request.before,
        &request.action,
        u64::try_from(body.len().div_ceil(4)).unwrap_or(u64::MAX),
    );
    execute_and_project(
        &state,
        &request_hash,
        &request_hash,
        &request_hash,
        LiveTransitionRequest {
            schema: LIVE_TRANSITION_REQUEST_SCHEMA.into(),
            before: request.before,
            action: request.action,
        },
        Projection::TransitionApi,
        "",
        false,
        0,
        false,
    )
}

async fn observe_transition(State(state): State<AppState>, body: Bytes) -> Response {
    state.counters.requests.fetch_add(1, Ordering::Relaxed);
    let request_hash = sha256_bytes(&body);
    let request = match serde_json::from_slice::<ObservationRequest>(&body) {
        Ok(request) => request,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid_json"),
    };
    match validate_observation(&request, &request_hash, &state) {
        Ok((trace_id, receipt, duplicate)) => {
            record_expression_shadow_observation(
                &state,
                &request.before,
                &request.action,
                &request.after,
            );
            state.counters.observations.fetch_add(1, Ordering::Relaxed);
            json_response(
                StatusCode::ACCEPTED,
                json!({
                    "schema": "nando.transition-observation-response.v1",
                    "accepted": true,
                    "grounded": true,
                    "duplicate": duplicate,
                    "trace_id": trace_id,
                    "receipt_sha256": receipt,
                }),
            )
        }
        Err((status, reason)) => {
            state.counters.errors.fetch_add(1, Ordering::Relaxed);
            error_response(status, reason)
        }
    }
}

async fn observe_response_relation(State(state): State<AppState>, body: Bytes) -> Response {
    state.counters.requests.fetch_add(1, Ordering::Relaxed);
    let frame = match serde_json::from_slice::<RelationFrame>(&body) {
        Ok(frame) => frame,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid_relation_frame_json"),
    };
    if !is_source_neutral_relation_frame(&frame)
        || !valid_sha256(&frame.frame_id_sha256)
        || !valid_sha256(&frame.event_id_sha256)
        || !valid_sha256(&frame.client_intent_id_sha256)
        || !valid_sha256(&frame.session_id_sha256)
        || !valid_sha256(&frame.evidence_ref_sha256)
    {
        return error_response(StatusCode::BAD_REQUEST, "invalid_relation_frame_contract");
    }
    let Some(worker) = current_miner_worker(&state) else {
        let reason = if state.config.embedded_response_miner_enabled {
            "response_miner_warming"
        } else {
            "response_miner_disabled"
        };
        return error_response(StatusCode::SERVICE_UNAVAILABLE, reason);
    };
    if worker.submit_frame(frame).is_err() {
        state.counters.errors.fetch_add(1, Ordering::Relaxed);
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "response_miner_enqueue_failed",
        );
    }
    let report = worker.status();
    json_response(
        StatusCode::ACCEPTED,
        json!({
            "schema": "nando.response-relation-observation.v1",
            "accepted": true,
            "queued": true,
            "enqueued": report.enqueued,
            "processed": report.processed,
            "queue_backlog_estimate": report.queue_backlog_estimate,
            "failed": report.failed,
        }),
    )
}

async fn refresh_runtime(State(state): State<AppState>) -> Response {
    refresh_executor(&state);
    refresh_response_authority(&state);
    refresh_expression_shadow(&state);
    let (ready, revision, profiles, error) = response_cache_status(&state);
    json_response(
        StatusCode::OK,
        json!({
            "schema": "nando.runtime-refresh.v1",
            "response_executor_ready": ready,
            "response_registry_revision": revision,
            "response_active_packages": profiles,
            "response_error": error,
        }),
    )
}

async fn report_false_accept(State(state): State<AppState>, body: Bytes) -> Response {
    let report = match serde_json::from_slice::<RuntimeFalseAcceptReport>(&body) {
        Ok(report)
            if report.schema == "nando.runtime-false-accept.v1"
                && valid_sha256(&report.request_sha256)
                && !report.package_id.is_empty()
                && report.package_id.len() <= 256 =>
        {
            report
        }
        _ => {
            return json_response(
                StatusCode::BAD_REQUEST,
                json!({"error":"invalid_false_accept_report"}),
            );
        }
    };
    if let Ok(mut cache) = state.response_cache.write() {
        cache.executor = None;
        cache.ready = false;
        cache.admission_expires_at_unix = 0;
        cache.last_error = "runtime_false_accept_reported".to_owned();
    }
    let _ = state
        .live_economics
        .observe_false_accept(report.request_sha256.clone());
    submit_opportunity_event(
        &state,
        OpportunityBridgeEventV1::false_accept(report.request_sha256.clone()),
        "false_accept",
    );
    let bounded_report_reason = bounded_reason(&report.reason);
    let durable_revocation =
        persist_runtime_package_revocation(&state, &report.package_id, &report.request_sha256);
    if let Ok(trigger) = state.authority_trigger.lock()
        && let Some(trigger) = trigger.as_ref()
    {
        let _ = trigger.try_send(());
    }
    write_event(
        &state,
        json!({
            "schema": "nando.runtime-false-accept.v1",
            "timestamp_unix": unix_now(),
            "request_sha256": report.request_sha256,
            "package_id": report.package_id,
            "reason": bounded_report_reason,
            "authority_revoked": true,
            "durable_revocation": durable_revocation.is_ok(),
            "durable_revocation_error": durable_revocation.as_ref().err(),
        }),
    );
    match durable_revocation {
        Ok(recorded) => json_response(
            StatusCode::ACCEPTED,
            json!({
                "accepted": true,
                "authority_revoked": true,
                "durable_revocation": true,
                "new_execution_identity_revocation": recorded,
            }),
        ),
        Err(error) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "accepted": false,
                "authority_revoked": true,
                "durable_revocation": false,
                "error": error,
            }),
        ),
    }
}

fn persist_runtime_package_revocation(
    state: &AppState,
    package_id: &str,
    request_sha256: &str,
) -> Result<bool, String> {
    let _guard = state
        .event_lock
        .lock()
        .map_err(|_| "runtime_package_revocation_lock_poisoned".to_owned())?;
    let registry_bytes = fs::read(&state.config.response_registry_path).map_err(|error| {
        format!(
            "runtime_package_revocation_registry_read:{}:{error}",
            state.config.response_registry_path.display()
        )
    })?;
    let registry: ResponseRegistry = serde_json::from_slice(&registry_bytes)
        .map_err(|error| format!("runtime_package_revocation_registry_decode:{error}"))?;
    registry
        .validate()
        .map_err(|error| format!("runtime_package_revocation_registry_invalid:{error}"))?;
    let package = registry
        .packages
        .iter()
        .find(|package| package.package_id == package_id)
        .ok_or_else(|| "runtime_package_revocation_package_missing".to_owned())?;
    let execution_payload_sha256 = response_execution_payload_digest(package)
        .map_err(|error| format!("runtime_package_revocation_payload_digest:{error}"))?;
    let path = state.config.runtime_package_revocations_path.clone();
    let mut ledger = if path.is_file() {
        let value: RuntimePackageRevocationLedgerV1 = serde_json::from_slice(
            &fs::read(&path).map_err(|error| format!("runtime_package_revocation_read:{error}"))?,
        )
        .map_err(|error| format!("runtime_package_revocation_decode:{error}"))?;
        value
            .validate()
            .map_err(|error| format!("runtime_package_revocation_invalid:{error}"))?;
        value
    } else {
        RuntimePackageRevocationLedgerV1::default()
    };
    let recorded = ledger
        .record(RuntimePackageRevocationV1 {
            package_id: package_id.to_owned(),
            execution_payload_sha256,
            request_sha256: request_sha256.to_owned(),
            observed_at_unix: unix_now(),
            reason: "runtime_false_accept".to_owned(),
        })
        .map_err(str::to_owned)?;
    write_bytes_atomic(
        &path,
        &serde_json::to_vec_pretty(&ledger)
            .map_err(|error| format!("runtime_package_revocation_encode:{error}"))?,
        "runtime-package-revocations",
    )?;
    Ok(recorded)
}

async fn openai_responses(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_openai(state, headers, body, Projection::Responses)
}

async fn openai_chat(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    handle_openai(state, headers, body, Projection::ChatCompletions)
}

async fn fallback_unknown(State(state): State<AppState>) -> Response {
    fallback(&state, "unsupported_worker_route")
}

fn handle_openai(
    state: AppState,
    headers: HeaderMap,
    body: Bytes,
    projection: Projection,
) -> Response {
    let request_ordinal = state
        .counters
        .requests
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);
    let request_hash = sha256_bytes(&body);
    let transport_request_id = headers
        .get("x-nando-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("anonymous:{request_hash}:{request_ordinal}"));
    let payload = match serde_json::from_slice::<Value>(&body) {
        Ok(payload) => payload,
        Err(_) => {
            state.provider_capture.observe_invalid_provenance();
            return fallback(&state, "request_json_unavailable");
        }
    };
    let traffic_source = request_traffic_source(&headers, &payload);
    let natural_evidence_eligible = traffic_source_natural_evidence_eligible(traffic_source);
    let request_identity = ProviderRequestIdentityV1::from_payload(&payload, &transport_request_id);
    let turn_intent_id = request_identity.turn_intent_id().to_owned();
    let request_event_id = request_identity.request_event_id().to_owned();
    let request_text = extract_request_text(&payload);
    let body_token_estimate = u64::try_from(body.len().div_ceil(4)).unwrap_or(u64::MAX);
    let input_tokens = token_estimate(&request_text).max(body_token_estimate);
    let capability_atom_ids = provider_tool_capability_atom_ids(&payload);
    let request_phase_atoms = request_phase_atom_ids(&request_text);
    let pre_action_context_atoms = response_pre_action_context_atom_ids(&payload);
    let multi_source_topology = natural_evidence_eligible.then(|| {
        multi_source_capture::extract_pre_action_multi_source_topology_v1(&payload, &request_text)
    });
    let request_streaming = payload
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    // Controlled probes may exercise runtime, but must never become natural evidence.
    if natural_evidence_eligible
        && let Some(capture_receipt) = capture_provider_request(
            &state,
            request_identity.session_lineage_root(),
            &request_hash,
            projection,
            request_streaming,
        )
    {
        let structure = LearningRequestStructureV1::new(LearningRequestStructureInputV1 {
            client_intent_id_sha256: request_identity.turn_intent_sha256().to_owned(),
            session_identity_sha256s: request_identity.session_identity_sha256s().to_vec(),
            request_phase_atom_ids: request_phase_atoms.clone(),
            pre_action_context_atom_ids: pre_action_context_atoms.clone(),
            capability_atom_ids: capability_atom_ids.clone(),
            provider_bound_turn_identity: request_identity.provider_bound_turn_identity(),
            estimated_input_tokens: input_tokens,
            provider_payload_bytes: u64::try_from(body.len()).unwrap_or(u64::MAX),
        });
        match structure {
            Ok(structure) => {
                let topology_shadow = if let Some(topology) = multi_source_topology.clone() {
                    let structure_v2 = nando_operator_kernel::LearningRequestStructureV2 {
                        schema: nando_operator_kernel::LEARNING_REQUEST_STRUCTURE_SCHEMA_V2
                            .to_owned(),
                        turn_intent_id_sha256: request_identity.turn_intent_sha256().to_owned(),
                        request_event_id_sha256: request_identity.request_event_sha256().to_owned(),
                        provider_bound_turn_identity: request_identity
                            .provider_bound_turn_identity(),
                        session_lineage_roots_sha256: request_identity
                            .session_identity_sha256s()
                            .to_vec(),
                        request_phase_atom_ids: request_phase_atoms.clone(),
                        pre_action_context_atom_ids: pre_action_context_atoms.clone(),
                        capability_atom_ids: capability_atom_ids.clone(),
                        estimated_input_tokens: input_tokens,
                        provider_payload_bytes: u64::try_from(body.len()).unwrap_or(u64::MAX),
                        provider_capture_request_root_sha256: request_hash.clone(),
                        decidability_reason_code: "pre_action_pending".to_owned(),
                        topology,
                    };
                    let commit = nando_operator_kernel::PreActionTopologyCommitV1::seal(
                        &structure_v2,
                        nando_operator_kernel::MultiSourceEvidenceOriginV1::FreshLive,
                        sha256_bytes(b"nando.multi-source-extractor.v1"),
                        sha256_bytes(b"nando.multi-source-extractor-config.v1"),
                        capture_receipt.capture_sequence(),
                    );
                    match commit {
                        Ok(commit) => {
                            write_event(
                                &state,
                                json!({
                                    "schema": "nando.pre-action-topology-shadow.v1",
                                    "event": "pre_action_topology_commit",
                                    "turn_intent_id_sha256": &structure_v2.turn_intent_id_sha256,
                                    "provider_capture_request_root_sha256": &structure_v2.provider_capture_request_root_sha256,
                                    "topology_root_sha256": &commit.topology_root_sha256,
                                    "commitment_root_sha256": &commit.commitment_root_sha256,
                                    "capture_sequence": commit.capture_sequence,
                                    "authority": false,
                                }),
                            );
                            Some((structure_v2, commit))
                        }
                        Err(error) => {
                            state.counters.errors.fetch_add(1, Ordering::Relaxed);
                            eprintln!("nando-pre-action-topology commit: {error}");
                            None
                        }
                    }
                } else {
                    None
                };
                if let Err(error) = state.request_learning.observe_structure(&structure) {
                    state.counters.errors.fetch_add(1, Ordering::Relaxed);
                    eprintln!("nando-request-learning index: {error}");
                }
                submit_operator_generation_shadow(
                    &state,
                    &body,
                    capture_receipt,
                    structure,
                    topology_shadow,
                    &request_text,
                );
            }
            Err(error) => {
                state.counters.errors.fetch_add(1, Ordering::Relaxed);
                eprintln!("nando-learning-evidence structure: {error:?}");
            }
        }
    }
    observe_live_economics_request(
        &state,
        &request_event_id,
        body.clone(),
        input_tokens,
        traffic_source_dedupe_eligible(traffic_source),
    );
    let request_shape =
        provider_request_shape(&payload, projection, &request_text, &capability_atom_ids);
    write_event(
        &state,
        json!({
            "schema": "nando.transition-execution-event.v1",
            "timestamp_unix": unix_now(),
            "event": "bridge_request",
            "client_intent_id": turn_intent_id.as_str(),
            "request_event_id_sha256": request_identity.request_event_sha256(),
            "request_sha256": request_hash,
            "tokens": input_tokens,
            "traffic_source": traffic_source,
            "natural_evidence_eligible": natural_evidence_eligible,
            "worker": "rust_transition_serving",
            "request_shape": request_shape,
        }),
    );
    let normalized_chat_payload = match projection {
        Projection::ChatCompletions => normalize_chat_messages_for_actor(&payload),
        Projection::TransitionApi | Projection::Responses => None,
    };
    let actor_payload = match projection {
        Projection::ChatCompletions => normalized_chat_payload.as_ref(),
        Projection::TransitionApi | Projection::Responses => Some(&payload),
    };
    if let Some(actor_payload) = actor_payload
        && let Some(response) = try_response_actor(
            &state,
            &request_event_id,
            &turn_intent_id,
            &request_hash,
            &request_text,
            actor_payload,
            projection,
            input_tokens,
            Some(traffic_source),
        )
    {
        return response;
    }
    if actor_payload.is_none() {
        record_response_actor_fallback(
            &state,
            &request_hash,
            &request_event_id,
            "adapter",
            "request_shape_unsupported",
            natural_evidence_eligible,
        );
    }
    let Some((before, action)) = transition_envelope(&payload) else {
        return fallback(&state, "no_grounded_transition_envelope");
    };
    state
        .counters
        .transition_requests
        .fetch_add(1, Ordering::Relaxed);
    write_event(
        &state,
        json!({
            "schema": "nando.transition-execution-event.v1",
            "timestamp_unix": unix_now(),
            "event": "transition_request",
            "request_sha256": request_hash,
            "tokens": input_tokens,
            "worker": "rust_transition_serving",
        }),
    );
    let stream = request_streaming;
    let model = payload
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("nando-local");
    record_expression_shadow_request(&state, &before, &action, input_tokens);
    execute_and_project(
        &state,
        &request_event_id,
        &turn_intent_id,
        &request_hash,
        LiveTransitionRequest {
            schema: LIVE_TRANSITION_REQUEST_SCHEMA.into(),
            before,
            action,
        },
        projection,
        model,
        stream,
        input_tokens,
        natural_evidence_eligible,
    )
}

fn capture_provider_request(
    state: &AppState,
    session_lineage_root: Sha256CommitmentV3,
    request_sha256: &str,
    projection: Projection,
    streaming: bool,
) -> Option<nando_operator_learning::ProviderRequestCaptureReceiptV3> {
    let projection = match projection {
        Projection::Responses => RuntimeProjectionV3::Responses,
        Projection::ChatCompletions => RuntimeProjectionV3::ChatCompletions,
        Projection::TransitionApi => RuntimeProjectionV3::TransitionApi,
    };
    let request_root_sha256 = Sha256CommitmentV3::from_hex(request_sha256).ok()?;
    state
        .provider_capture
        .try_capture(ProviderCaptureIngressV3 {
            lineage_root_sha256: session_lineage_root,
            request_root_sha256,
            projection,
            streaming,
            observed_at_unix_ms: unix_now_ms(),
        })
        .into_receipt()
}

fn submit_operator_generation_shadow(
    state: &AppState,
    body: &Bytes,
    capture_receipt: nando_operator_learning::ProviderRequestCaptureReceiptV3,
    structure: LearningRequestStructureV1,
    topology_shadow: Option<(
        nando_operator_kernel::LearningRequestStructureV2,
        nando_operator_kernel::PreActionTopologyCommitV1,
    )>,
    request_text: &str,
) {
    if state.learning_structure_bridge.producer_enabled() {
        let result = match topology_shadow {
            Some((structure_v2, topology_commit)) => state.learning_structure_bridge.submit_v3(
                capture_receipt.clone(),
                structure.clone(),
                structure_v2,
                topology_commit,
            ),
            None => state
                .learning_structure_bridge
                .submit(capture_receipt.clone(), structure.clone()),
        };
        if let Err(error) = result {
            state.counters.errors.fetch_add(1, Ordering::Relaxed);
            eprintln!("nando-learning-structure bridge: {error}");
        }
    }
    if state.learning_evidence_bridge.producer_enabled() {
        if let Err(error) =
            state
                .learning_evidence_bridge
                .submit(capture_receipt, structure, body.clone())
        {
            state.counters.errors.fetch_add(1, Ordering::Relaxed);
            eprintln!("nando-learning-evidence bridge: {error}");
        }
        return;
    }
    if !state.operator_generation_shadow.enabled() {
        return;
    }
    state
        .operator_generation_shadow
        .observe_provider_request(GenerationShadowIngressV3 {
            capture_receipt,
            request_text,
            provider_payload_bytes: body.clone(),
        });
}

#[allow(clippy::too_many_arguments)]
fn try_response_actor(
    state: &AppState,
    request_event_id: &str,
    turn_intent_id: &str,
    request_hash: &str,
    request_text: &str,
    payload: &Value,
    projection: Projection,
    input_tokens: u64,
    traffic_source_header: Option<&str>,
) -> Option<Response> {
    if !projection.avoids_upstream_llm_call() {
        return None;
    }
    let traffic_source = traffic_source_header
        .or_else(|| {
            payload
                .pointer("/metadata/nando_traffic_source")
                .and_then(Value::as_str)
        })
        .unwrap_or("ordinary");
    let natural_evidence_eligible = traffic_source_natural_evidence_eligible(traffic_source);
    // Authority renewal belongs to the background runtime. The request path
    // reads only the cache and fails closed to upstream when its lease expires.
    if !response_local_accept_enabled(state) {
        record_response_actor_fallback(
            state,
            request_hash,
            request_event_id,
            "admission",
            "response_local_accept_disabled",
            natural_evidence_eligible,
        );
        return None;
    }
    let (executor, runtime_build_sha256) = {
        let cache = match state.response_cache.read() {
            Ok(cache) => cache,
            Err(_) => {
                record_runtime_parity_failure(
                    state,
                    request_hash,
                    request_event_id,
                    "response_authority_cache_poisoned",
                    natural_evidence_eligible,
                );
                return None;
            }
        };
        if !cache.ready || unix_now() > cache.admission_expires_at_unix {
            record_response_actor_fallback(
                state,
                request_hash,
                request_event_id,
                "admission",
                "response_admission_expired",
                natural_evidence_eligible,
            );
            return None;
        }
        let Some(executor) = cache.executor.clone() else {
            record_response_actor_fallback(
                state,
                request_hash,
                request_event_id,
                "admission",
                "response_executor_unavailable",
                natural_evidence_eligible,
            );
            return None;
        };
        if cache.runtime_build_sha256.is_empty() {
            record_response_actor_fallback(
                state,
                request_hash,
                request_event_id,
                "admission",
                "runtime_build_digest_missing",
                natural_evidence_eligible,
            );
            return None;
        }
        (executor, cache.runtime_build_sha256.clone())
    };
    let execution = executor.execute(request_text, payload);
    if execution.status != ResponseExecutionStatus::Executed {
        let decidability = classify_cpu_decidability(request_text, payload);
        record_response_actor_fallback_with_decidability(
            state,
            request_hash,
            request_event_id,
            response_actor_fallback_stage(&execution.reason),
            &execution.reason,
            decidability,
            natural_evidence_eligible,
        );
        return None;
    }
    let Some(response_text) = execution.response.as_deref() else {
        record_response_actor_fallback(
            state,
            request_hash,
            request_event_id,
            "actor",
            "actor_response_missing",
            natural_evidence_eligible,
        );
        return None;
    };
    let Some(package_id) = execution.package_id.as_deref() else {
        record_response_actor_fallback(
            state,
            request_hash,
            request_event_id,
            "actor",
            "actor_package_id_missing",
            natural_evidence_eligible,
        );
        return None;
    };
    let model = payload
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("nando-local");
    let stream = payload
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let route = format!("response_actor:{package_id}");
    let intent_dedupe_eligible = traffic_source_dedupe_eligible(traffic_source);
    let function_call = serde_json::from_str::<Value>(response_text)
        .ok()
        .filter(|value| {
            value.get("name").and_then(Value::as_str).is_some()
                && value.get("arguments").and_then(Value::as_object).is_some()
        });
    let custom_tool_call = parse_actor_custom_tool_call(response_text);
    let projected = match projection {
        Projection::Responses if custom_tool_call.is_some() => custom_tool_responses_projection(
            request_hash,
            custom_tool_call.as_ref().expect("custom tool call checked"),
            model,
            &route,
            input_tokens,
            unix_now(),
        ),
        Projection::ChatCompletions if custom_tool_call.is_some() => {
            record_response_actor_fallback(
                state,
                request_hash,
                request_event_id,
                "adapter",
                "custom_tool_chat_projection_unsupported",
                natural_evidence_eligible,
            );
            return None;
        }
        Projection::Responses if function_call.is_some() => function_call_responses_projection(
            request_hash,
            function_call.as_ref().expect("function call checked"),
            model,
            &route,
            input_tokens,
        ),
        Projection::ChatCompletions if function_call.is_some() => function_call_chat_projection(
            request_hash,
            function_call.as_ref().expect("function call checked"),
            model,
            &route,
            input_tokens,
        ),
        Projection::Responses => {
            responses_projection(request_hash, response_text, model, &route, input_tokens)
        }
        Projection::ChatCompletions => {
            chat_projection(request_hash, response_text, model, &route, input_tokens)
        }
        Projection::TransitionApi => {
            record_response_actor_fallback(
                state,
                request_hash,
                request_event_id,
                "adapter",
                "response_actor_transition_projection_unsupported",
                natural_evidence_eligible,
            );
            return None;
        }
    };
    let projector_receipt_id = match sha256_json(&projected) {
        Ok(digest) => digest,
        Err(_) => {
            record_runtime_parity_failure(
                state,
                request_hash,
                request_event_id,
                "projector_digest_failed",
                natural_evidence_eligible,
            );
            return None;
        }
    };
    let Some(verifier_schema) = execution.verifier_schema.as_deref() else {
        record_runtime_parity_failure(
            state,
            request_hash,
            request_event_id,
            "verifier_schema_missing",
            natural_evidence_eligible,
        );
        return None;
    };
    let projector_schema = match projection {
        Projection::Responses => "openai.responses.projector.v1",
        Projection::ChatCompletions => "openai.chat-completions.projector.v1",
        Projection::TransitionApi => return None,
    };
    let runtime_receipt = match executor.finalize_runtime_receipt(
        &execution,
        request_hash,
        projector_schema,
        &runtime_build_sha256,
        &projected,
    ) {
        Ok(receipt) => receipt,
        Err(_) => {
            record_runtime_parity_failure(
                state,
                request_hash,
                request_event_id,
                "runtime_receipt_finalize_failed",
                natural_evidence_eligible,
            );
            return None;
        }
    };
    let post_verifier = match finalize_post_verifier_receipt(
        &runtime_receipt.receipt.actor_program_sha256,
        &runtime_receipt.receipt.independent_verifier_program_sha256,
        request_hash,
        &projector_receipt_id,
        package_id,
    ) {
        Ok(receipt) => receipt,
        Err(_) => {
            record_runtime_parity_failure(
                state,
                request_hash,
                request_event_id,
                "post_verifier_receipt_finalize_failed",
                natural_evidence_eligible,
            );
            return None;
        }
    };
    let receipt = &post_verifier.receipt_sha256;
    state.counters.local_accepts.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut counters) = state.counters.response_cpu_by_package.lock() {
        if counters.contains_key(package_id)
            || counters.len() < RESPONSE_PACKAGE_CPU_COUNTER_CAPACITY
        {
            let package = counters.entry(package_id.to_owned()).or_default();
            package.accepts = package.accepts.saturating_add(1);
            if intent_dedupe_eligible {
                package.ordinary_accepts = package.ordinary_accepts.saturating_add(1);
                package.ordinary_input_tokens =
                    package.ordinary_input_tokens.saturating_add(input_tokens);
            }
        } else {
            state
                .counters
                .response_cpu_by_package_overflow
                .fetch_add(1, Ordering::Relaxed);
        }
    }
    if intent_dedupe_eligible {
        state
            .counters
            .ordinary_response_local_accepts
            .fetch_add(1, Ordering::Relaxed);
        state
            .counters
            .ordinary_response_local_accept_input_tokens
            .fetch_add(input_tokens, Ordering::Relaxed);
    }
    write_event(
        state,
        json!({
            "schema": "nando.transition-execution-event.v1",
            "timestamp_unix": unix_now(),
            "event": "local_accept",
            "request_sha256": request_hash,
            "tokens": input_tokens,
            "route": route,
        "package_id": package_id,
        "phase_candidates": execution.phase_candidates,
        "exact_actor_checks": execution.exact_actor_checks,
        "phase_margin_micro": execution.phase_margin_micro,
            "verification_receipt_id": receipt,
            "runtime_verification_receipt_id": runtime_receipt.receipt_sha256,
            "runtime_verification_receipt": runtime_receipt.receipt,
            "post_verifier_receipt": post_verifier.receipt,
            "projector_receipt_id": projector_receipt_id,
            "verifier_schema": verifier_schema,
            "worker": "rust_transition_serving",
        }),
    );
    write_economics(
        state,
        json!({
            "schema": "nando.economics-terminal.v1",
            "timestamp_unix": unix_now(),
            "client_intent_id": turn_intent_id,
            "request_event_id": request_event_id,
            "intent_dedupe_eligible": intent_dedupe_eligible,
            "provider_attempt_id": Value::Null,
            "request_sha256": request_hash,
            "route": "local_response_actor",
            "terminal_state": "delivered",
            "delivery_worker": "rust_transition_serving",
            "traffic_source": traffic_source,
            "input_tokens": input_tokens,
            "input_token_accounting": "request_body_byte_estimate_v1",
            "upstream_socket_opened": false,
            "avoided_call": true,
            "verification_status": "verified",
            "verification_receipt_id": receipt,
            "projector_receipt_id": projector_receipt_id,
            "runtime_verification_receipt_id": runtime_receipt.receipt_sha256,
            "runtime_verification_receipt": runtime_receipt.receipt,
            "post_verifier_receipt": post_verifier.receipt,
            "local_route": route,
            "package_id": package_id,
            "verifier_schema": verifier_schema,
        }),
    );
    observe_live_economics_verified_accept(
        state,
        request_event_id,
        input_tokens,
        natural_evidence_eligible,
    );
    Some(match projection {
        Projection::Responses if stream => sse_response(responses_sse(&projected)),
        Projection::ChatCompletions if stream => sse_response(chat_sse(&projected)),
        Projection::Responses | Projection::ChatCompletions => {
            json_response(StatusCode::OK, projected)
        }
        Projection::TransitionApi => unreachable!(),
    })
}

fn response_local_accept_enabled(state: &AppState) -> bool {
    let cache_ready = state.response_cache.read().is_ok_and(|cache| {
        cache.ready && cache.executor.is_some() && unix_now() <= cache.admission_expires_at_unix
    });
    state.runtime_policy.cpu_mode()
        && cache_ready
        && state.config.local_accept_enabled
        && state.config.client_allow_local_accept
        && state.config.route_ready
        && !state.runtime_policy.kill_switch()
}

fn refresh_response_authority(state: &AppState) {
    if state.config.embedded_response_miner_enabled
        && let Err(error) = publish_embedded_response_candidates(state)
    {
        eprintln!("nando-response-candidate-publisher: {error}");
    }
    // Serving consumes authority produced by the independent Rust controller.
    refresh_response_executor(state);
}

fn publish_embedded_response_candidates(state: &AppState) -> Result<bool, String> {
    let response_miner = current_response_miner(state);
    let collection_miner = current_collection_miner(state);
    if response_miner.is_none() && collection_miner.is_none() {
        return Ok(false);
    }
    let collection_candidates = collection_miner
        .as_ref()
        .map(|miner| {
            miner
                .lock()
                .map_err(|_| "online_collection_miner_lock_poisoned".to_owned())?
                .admission_candidates()
        })
        .transpose()?
        .unwrap_or_default();
    let crystallized_collection_candidates = collection_candidates
        .iter()
        .map(CrystallizedCollectionAdmissionCandidateV1::seal)
        .collect::<Result<Vec<_>, _>>()
        .map_err(str::to_owned)?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let (relation_candidates, mut crystallized_candidates, retained_transitions) = response_miner
        .as_ref()
        .map(|miner| {
            miner
                .lock()
                .map_err(|_| "response_miner_lock_poisoned".to_owned())
                .map(|miner| {
                    (
                        miner.admission_candidates(),
                        miner.crystallized_admission_candidates(),
                        miner.retained_teacher_transitions_for_multi_source_proof_v1(),
                    )
                })
        })
        .transpose()?
        .unwrap_or_default();
    if let Some(snapshot) = state
        .multi_source_snapshot
        .read()
        .ok()
        .and_then(|snapshot| snapshot.clone())
        .filter(|snapshot| snapshot.transfer_ready)
    {
        match nando_response_actor::crystallize_multi_source_t1_candidate_v1(
            &snapshot.t1_identification,
            &retained_transitions,
        ) {
            Ok(candidate)
                if !crystallized_candidates.iter().any(|existing| {
                    existing.package.package_id == candidate.package.package_id
                }) =>
            {
                crystallized_candidates.push(candidate);
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!(
                    "nando-multi-source-crystallizer:{}:{error}",
                    snapshot.snapshot_root_sha256
                );
            }
        }
    }
    let revision_material = serde_json::json!({
        "collection": collection_candidates.iter().map(|candidate| serde_json::json!({
            "package_id": candidate.package.package_id,
            "support_manifest_sha256": candidate.support_manifest_sha256,
            "future_manifest_sha256": candidate.future_manifest_sha256,
            "runtime_parity_cases": candidate.runtime_parity_cases.len(),
        })).collect::<Vec<_>>(),
        "relation": relation_candidates.iter().map(|candidate| serde_json::json!({
            "bucket_id": candidate.candidate.bucket_id,
            "positive_rows": candidate.candidate.positive_rows,
            "support_rows": candidate.support.len(),
            "future_rows": candidate.future.len(),
            "runtime_parity_cases": candidate.runtime_parity_cases.len(),
            "wave_runtime_fingerprint64": candidate.candidate.wave_runtime_fingerprint64,
        })).collect::<Vec<_>>(),
        "crystallized": crystallized_candidates.iter().map(|candidate| serde_json::json!({
            "package_id": candidate.package.package_id,
            "support_root_sha256": candidate.support_root_sha256,
            "future_evidence_root_sha256": candidate.future_evidence_root_sha256,
            "winner_seal_sha256": candidate.winner_seal_sha256,
            "executable_parity_seal_sha256": candidate.executable_parity_seal_sha256,
        })).collect::<Vec<_>>(),
        "crystallized_collection": crystallized_collection_candidates.iter().map(|candidate| {
            serde_json::json!({
                "package_id": candidate.candidate().package.package_id,
                "seal_sha256": candidate.seal_sha256(),
            })
        }).collect::<Vec<_>>(),
    });
    let revision_digest = sha256_bytes(
        &serde_json::to_vec(&revision_material)
            .map_err(|error| format!("candidate_revision_encode:{error}"))?,
    );
    let revision = u64::from_str_radix(&revision_digest[..16], 16)
        .map_err(|error| format!("candidate_revision_parse:{error}"))?
        .max(1);
    if state
        .response_cache
        .read()
        .is_ok_and(|cache| cache.embedded_candidate_revision == revision)
        && state.config.response_admission_candidate_path.exists()
    {
        return Ok(false);
    }
    let bundle = OnlineAdmissionCandidateBundle {
        schema: ONLINE_ADMISSION_CANDIDATE_BUNDLE_SCHEMA_V1.to_owned(),
        project_id: state.config.project_id.clone(),
        revision,
        relation_candidates,
        collection_candidates,
        crystallized_candidates,
        crystallized_collection_candidates,
    };
    bundle.validate().map_err(str::to_owned)?;
    let bytes = serde_cbor::to_vec(&bundle)
        .map_err(|error| format!("response_candidate_bundle_encode:{error}"))?;
    write_bytes_atomic(
        &state.config.response_admission_candidate_path,
        &bytes,
        "response-admission-candidates",
    )?;
    if let Ok(mut cache) = state.response_cache.write() {
        cache.embedded_candidate_revision = revision;
    }
    Ok(true)
}

#[cfg(any())]
fn refresh_embedded_response_admission(state: &AppState) -> Result<bool, String> {
    let renewal_margin = state
        .config
        .admission_max_age_seconds
        .saturating_div(3)
        .max(1);
    let response_miner = current_response_miner(state);
    let collection_miner = current_collection_miner(state);
    if response_miner.is_none() && collection_miner.is_none() {
        return Ok(false);
    }
    let collection_candidates = collection_miner
        .as_ref()
        .map(|miner| {
            miner
                .lock()
                .map_err(|_| "online_collection_miner_lock_poisoned".to_owned())?
                .admission_candidates()
        })
        .transpose()?
        .unwrap_or_default();
    let candidates = response_miner
        .as_ref()
        .map(|miner| {
            let miner = miner
                .lock()
                .map_err(|_| "response_miner_lock_poisoned".to_owned())?;
            Ok::<_, String>(miner.admission_candidates())
        })
        .transpose()?
        .unwrap_or_default();
    if collection_candidates.is_empty() && candidates.is_empty() {
        if embedded_authority_owns_current_registry(state) {
            revoke_embedded_response_authority(state, "online_no_admission_candidate")?;
            return Ok(true);
        }
        discard_stale_embedded_authority_metadata(state)?;
        if let Ok(mut cache) = state.response_cache.write() {
            record_embedded_admission_unavailable(&mut cache, "online_no_admission_candidate");
        }
        return Ok(false);
    }
    let now = unix_now();
    let revision_material = serde_json::json!({
        "collection": collection_candidates.iter().map(|candidate| serde_json::json!({
            "package_id": candidate.package.package_id,
            "support_manifest_sha256": candidate.support_manifest_sha256,
            "future_manifest_sha256": candidate.future_manifest_sha256,
            "causal_report": candidate.causal_report,
        })).collect::<Vec<_>>(),
        "relation": candidates.iter().map(|candidate| serde_json::json!({
            "bucket_id": candidate.candidate.bucket_id,
            "positive_rows": candidate.candidate.positive_rows,
            "negative_rows": candidate.candidate.negative_rows,
            "support_rows": candidate.support.len(),
            "future_rows": candidate.future.len(),
            "negatives": candidate.negatives.len(),
            "wave_runtime_fingerprint64": candidate.candidate.wave_runtime_fingerprint64,
        })).collect::<Vec<_>>(),
    });
    let revision_digest = sha256_bytes(
        &serde_json::to_vec(&revision_material)
            .map_err(|error| format!("embedded_revision_encode:{error}"))?,
    );
    let revision = u64::from_str_radix(&revision_digest[..16], 16)
        .map_err(|error| format!("embedded_revision_parse:{error}"))?
        .max(1);
    if state.response_cache.read().is_ok_and(|cache| {
        cache.embedded_candidate_revision == revision
            && cache.input_fingerprint.is_none()
            && cache.ready
            && cache.executor.is_some()
            && now.saturating_add(renewal_margin) < cache.admission_expires_at_unix
    }) {
        return Ok(false);
    }
    let runtime_bytes = fs::read(&state.config.runtime_build_path)
        .map_err(|error| format!("embedded_runtime_build_read:{error}"))?;
    let runtime_sha256 = sha256_bytes(&runtime_bytes);
    let gate_sha256 =
        sha256_bytes(format!("nando.embedded-admission-controller.v1:{runtime_sha256}").as_bytes());
    let relation_snapshot = if candidates.is_empty() {
        None
    } else {
        build_online_admission_snapshot(
            &candidates,
            &state.config.project_id,
            revision,
            now,
            state.config.admission_max_age_seconds,
            &gate_sha256,
            &runtime_sha256,
        )
        .map_err(str::to_owned)?
    };
    let collection_snapshot = if collection_candidates.is_empty() {
        None
    } else {
        build_online_collection_admission_snapshot(
            &collection_candidates,
            &state.config.project_id,
            revision,
            now,
            state.config.admission_max_age_seconds,
            &gate_sha256,
            &runtime_sha256,
        )
        .map_err(str::to_owned)?
    };
    let snapshot = merge_online_admission_snapshots(
        [relation_snapshot, collection_snapshot]
            .into_iter()
            .flatten()
            .collect(),
    )
    .map_err(str::to_owned)?;
    let Some(snapshot) = snapshot else {
        if embedded_authority_owns_current_registry(state) {
            revoke_embedded_response_authority(state, "embedded_admission_not_proven")?;
            return Ok(true);
        }
        discard_stale_embedded_authority_metadata(state)?;
        if let Ok(mut cache) = state.response_cache.write() {
            cache.embedded_candidate_revision = revision;
            record_embedded_admission_unavailable(&mut cache, "embedded_admission_not_proven");
        }
        return Ok(false);
    };
    let registry_bytes = serde_json::to_vec_pretty(&snapshot.registry)
        .map_err(|error| format!("embedded_registry_encode:{error}"))?;
    let admission_bytes = serde_json::to_vec_pretty(&snapshot.admission)
        .map_err(|error| format!("embedded_admission_encode:{error}"))?;
    let authority_candidate = json!({
        "schema": "nando.response-authority-candidate.v1",
        "authority_schema": snapshot.admission.response_authority.schema,
        "registry_schema": snapshot.admission.response_authority.registry_schema,
        "registry_revision": snapshot.admission.response_authority.registry_revision,
        "registry_sha256": snapshot.admission.response_authority.registry_sha256,
        "execution_authority": false,
        "packages": snapshot.admission.response_authority.packages,
        "required_gate_fields": [
            "gate_build_sha256",
            "runtime_build_sha256",
            "generated_at_unix",
            "expires_at_unix"
        ]
    });
    let authority_candidate_bytes = serde_json::to_vec_pretty(&authority_candidate)
        .map_err(|error| format!("embedded_authority_candidate_encode:{error}"))?;
    let embedded_registry_sha256 = snapshot
        .admission
        .response_authority
        .registry_sha256
        .clone();
    let executor = ResponseExecutor::from_registry_with_admission(
        snapshot.registry,
        snapshot.admission,
        &state.config.project_id,
        &gate_sha256,
        &runtime_sha256,
        now,
        state.config.admission_max_age_seconds,
    )
    .map_err(str::to_owned)?;
    let embedded_marker = json!({
        "schema": "nando.embedded-response-authority-marker.v1",
        "revision": revision,
        "registry_sha256": embedded_registry_sha256,
        "written_at_unix": now,
        "execution_authority": false
    });
    write_bytes_atomic(
        &embedded_authority_marker_path(state),
        &serde_json::to_vec_pretty(&embedded_marker)
            .map_err(|error| format!("embedded_marker_encode:{error}"))?,
        "embedded-response-authority-marker",
    )?;
    write_bytes_atomic(
        &state.config.response_registry_path,
        &registry_bytes,
        "response-registry",
    )?;
    let authority_candidate_path = embedded_authority_candidate_path(state);
    write_bytes_atomic(
        &authority_candidate_path,
        &authority_candidate_bytes,
        "response-authority-candidate",
    )?;
    write_bytes_atomic(&state.config.admission_path, &admission_bytes, "admission")?;
    let mut cache = state
        .response_cache
        .write()
        .map_err(|_| "response_cache_lock_poisoned")?;
    cache.executor = Some(Arc::new(executor));
    cache.ready = true;
    cache.gate_build_sha256 = gate_sha256;
    cache.runtime_build_sha256 = runtime_sha256;
    cache.input_fingerprint = None;
    cache.embedded_candidate_revision = revision;
    cache.admission_expires_at_unix = now.saturating_add(state.config.admission_max_age_seconds);
    cache.last_error.clear();
    Ok(true)
}

#[cfg(any())]
fn record_embedded_admission_unavailable(cache: &mut ResponseExecutorCache, reason: &str) {
    if cache.input_fingerprint.is_some() && cache.ready && cache.executor.is_some() {
        cache.last_error.clear();
    } else {
        cache.last_error = reason.to_owned();
    }
}

#[cfg(any())]
fn embedded_authority_marker_path(state: &AppState) -> PathBuf {
    state
        .config
        .response_registry_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("response-embedded-authority.marker.json")
}

#[cfg(any())]
fn embedded_authority_candidate_path(state: &AppState) -> PathBuf {
    state
        .config
        .response_registry_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("response-authority-candidate.json")
}

#[cfg(any())]
fn embedded_authority_owns_current_registry(state: &AppState) -> bool {
    let marker_path = embedded_authority_marker_path(state);
    let Ok(marker_bytes) = fs::read(&marker_path) else {
        return false;
    };
    let Ok(marker) = serde_json::from_slice::<Value>(&marker_bytes) else {
        return true;
    };
    let Some(expected_registry_sha256) = marker.get("registry_sha256").and_then(Value::as_str)
    else {
        return true;
    };
    let Ok(registry_bytes) = fs::read(&state.config.response_registry_path) else {
        return true;
    };
    let Ok(registry) = serde_json::from_slice::<ResponseRegistry>(&registry_bytes) else {
        return true;
    };
    response_registry_digest(&registry).is_ok_and(|actual| actual == expected_registry_sha256)
}

#[cfg(any())]
fn discard_stale_embedded_authority_metadata(state: &AppState) -> Result<(), String> {
    if embedded_authority_marker_path(state).exists()
        && !embedded_authority_owns_current_registry(state)
    {
        remove_generated_authority_file(&embedded_authority_marker_path(state))?;
        remove_generated_authority_file(&embedded_authority_candidate_path(state))?;
    }
    Ok(())
}

#[cfg(any())]
fn revoke_embedded_response_authority(state: &AppState, reason: &str) -> Result<(), String> {
    if let Ok(mut cache) = state.response_cache.write() {
        cache.executor = None;
        cache.ready = false;
        cache.input_fingerprint = None;
        cache.embedded_candidate_revision = 0;
        cache.admission_expires_at_unix = 0;
        cache.last_error = reason.to_owned();
    }
    remove_generated_authority_file(&state.config.admission_path)?;
    remove_generated_authority_file(&state.config.response_registry_path)?;
    remove_generated_authority_file(&embedded_authority_candidate_path(state))?;
    remove_generated_authority_file(&embedded_authority_marker_path(state))?;
    let receipt = json!({
        "schema": "nando.embedded-response-authority-revocation.v1",
        "revoked_at_unix": unix_now(),
        "reason": reason,
        "execution_authority": false
    });
    let parent = state
        .config
        .response_registry_path
        .parent()
        .ok_or_else(|| "embedded_revocation_parent_missing".to_owned())?;
    write_bytes_atomic(
        &parent.join("response-authority-revocation.json"),
        &serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("embedded_revocation_encode:{error}"))?,
        "response-authority-revocation",
    )
}

#[cfg(any())]
fn remove_generated_authority_file(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "embedded_authority_remove:{}:{error}",
                path.display()
            ));
        }
    }
    if let Some(parent) = path.parent() {
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| {
                format!(
                    "embedded_authority_remove_sync:{}:{error}",
                    parent.display()
                )
            })?;
    }
    Ok(())
}

fn write_bytes_atomic(path: &Path, bytes: &[u8], stem: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{stem}_parent_missing"))?;
    let temporary = parent.join(format!(
        ".{stem}.{}.{}.tmp",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| format!("{stem}_temp_create:{error}"))?;
        file.write_all(bytes)
            .map_err(|error| format!("{stem}_temp_write:{error}"))?;
        file.sync_all()
            .map_err(|error| format!("{stem}_temp_sync:{error}"))?;
        fs::rename(&temporary, path).map_err(|error| format!("{stem}_rename:{error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    } else {
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("{stem}_parent_sync:{error}"))?;
    }
    result
}

fn current_response_miner(state: &AppState) -> Option<Arc<Mutex<OnlineResponseStream>>> {
    state
        .miners
        .read()
        .ok()
        .and_then(|slots| slots.response.clone())
}

fn current_miner_worker(state: &AppState) -> Option<MinerWorkerHandle> {
    state
        .miners
        .read()
        .ok()
        .and_then(|slots| slots.worker.clone())
}

fn current_collection_miner(state: &AppState) -> Option<Arc<Mutex<OnlineCollectionMiner>>> {
    state
        .miners
        .read()
        .ok()
        .and_then(|slots| slots.collection.clone())
}

fn response_actor_fallback_stage(reason: &str) -> &'static str {
    if reason.starts_with("no_phase_") || reason.starts_with("ambiguous_phase_") {
        "router"
    } else if reason.starts_with("phase_routed_actor_") || reason.starts_with("actor_") {
        "actor"
    } else if reason.starts_with("independent_verifier_") {
        "verifier"
    } else if reason.contains("authority") || reason.contains("admission") {
        "admission"
    } else {
        "runtime"
    }
}

fn record_response_actor_fallback(
    state: &AppState,
    request_hash: &str,
    request_event_id: &str,
    stage: &str,
    reason: &str,
    natural_evidence_eligible: bool,
) {
    let intent_sha256 = sha256_bytes(request_event_id.as_bytes());
    if natural_evidence_eligible {
        if let Err(error) = state.live_economics.observe_fallback(
            intent_sha256.clone(),
            stage.to_owned(),
            reason.to_owned(),
        ) {
            eprintln!("nando-live-economics fallback: {error}");
        }
        submit_opportunity_classification(
            state,
            intent_sha256,
            fallback_reducibility(stage, reason),
            reason,
        );
    }
    write_event(
        state,
        json!({
            "schema": "nando.response-actor-fallback.v1",
            "timestamp_unix": unix_now(),
            "event": "response_actor_fallback",
            "request_sha256": request_hash,
            "stage": stage,
            "reason": bounded_reason(reason),
            "worker": "rust_transition_serving",
        }),
    );
}

fn record_response_actor_fallback_with_decidability(
    state: &AppState,
    request_hash: &str,
    request_event_id: &str,
    stage: &str,
    reason: &str,
    decidability: CpuDecidability,
    natural_evidence_eligible: bool,
) {
    let intent_sha256 = sha256_bytes(request_event_id.as_bytes());
    if natural_evidence_eligible {
        if let Err(error) = state.live_economics.observe_fallback(
            intent_sha256.clone(),
            stage.to_owned(),
            reason.to_owned(),
        ) {
            eprintln!("nando-live-economics fallback: {error}");
        }
        submit_opportunity_classification(
            state,
            intent_sha256,
            decidability_reducibility(decidability),
            decidability.reason,
        );
    }
    write_event(
        state,
        json!({
            "schema": "nando.response-actor-fallback.v1",
            "timestamp_unix": unix_now(),
            "event": "response_actor_fallback",
            "request_sha256": request_hash,
            "stage": stage,
            "reason": bounded_reason(reason),
            "cpu_decidability_class": decidability.class.code(),
            "cpu_decidability_reason": decidability.reason,
            "cpu_decidability_contract": "state_before_observation_only.v1",
            "worker": "rust_transition_serving",
        }),
    );
}

fn record_runtime_parity_failure(
    state: &AppState,
    request_hash: &str,
    request_event_id: &str,
    reason: &str,
    natural_evidence_eligible: bool,
) {
    let intent_sha256 = sha256_bytes(request_event_id.as_bytes());
    if natural_evidence_eligible {
        if let Err(error) = state
            .live_economics
            .observe_parity_failure(intent_sha256.clone())
        {
            eprintln!("nando-live-economics parity: {error}");
        }
        submit_opportunity_event(
            state,
            OpportunityBridgeEventV1::parity_failure(intent_sha256),
            "parity_failure",
        );
    }
    record_response_actor_fallback(
        state,
        request_hash,
        request_event_id,
        "verifier",
        reason,
        natural_evidence_eligible,
    );
}

fn function_call_responses_projection(
    request_hash: &str,
    call: &Value,
    model: &str,
    route: &str,
    input_tokens: u64,
) -> Value {
    let suffix = request_hash.get(..16).unwrap_or(request_hash);
    let name = call.get("name").and_then(Value::as_str).unwrap_or("wait");
    let arguments = serde_json::to_string(call.get("arguments").unwrap_or(&Value::Null))
        .unwrap_or_else(|_| "{}".into());
    json!({
        "id": format!("resp_nando_{suffix}"),
        "object": "response",
        "created_at": unix_now(),
        "status": "completed",
        "model": model,
        "output": [{
            "id": format!("fc_nando_{suffix}"),
            "call_id": format!("call_nando_{suffix}"),
            "type": "function_call",
            "status": "completed",
            "name": name,
            "arguments": arguments,
        }],
        "output_text": "",
        "usage": {"input_tokens":input_tokens,"output_tokens":token_estimate(&arguments),"total_tokens":input_tokens.saturating_add(token_estimate(&arguments))},
        "nando": {"api_version":"v2","local_accept":true,"route":route,"false_accepts":0,"architecture":"wave_router_typed_actor_verifier"},
    })
}

fn function_call_chat_projection(
    request_hash: &str,
    call: &Value,
    model: &str,
    route: &str,
    input_tokens: u64,
) -> Value {
    let suffix = request_hash.get(..16).unwrap_or(request_hash);
    let name = call.get("name").and_then(Value::as_str).unwrap_or("wait");
    let arguments = serde_json::to_string(call.get("arguments").unwrap_or(&Value::Null))
        .unwrap_or_else(|_| "{}".into());
    json!({
        "id":format!("chatcmpl-nando-{suffix}"),"object":"chat.completion","created":unix_now(),"model":model,
        "choices":[{"index":0,"message":{"role":"assistant","content":Value::Null,"tool_calls":[{"id":format!("call_nando_{suffix}"),"type":"function","function":{"name":name,"arguments":arguments}}]},"finish_reason":"tool_calls"}],
        "usage":{"prompt_tokens":input_tokens,"completion_tokens":token_estimate(&arguments),"total_tokens":input_tokens.saturating_add(token_estimate(&arguments))},
        "nando":{"api_version":"v2","local_accept":true,"route":route,"false_accepts":0},
    })
}

#[derive(Clone, Copy)]
enum Projection {
    TransitionApi,
    Responses,
    ChatCompletions,
}

impl Projection {
    fn avoids_upstream_llm_call(self) -> bool {
        matches!(self, Self::Responses | Self::ChatCompletions)
    }

    const fn endpoint(self) -> &'static str {
        match self {
            Self::TransitionApi => "transition_api",
            Self::Responses => "responses",
            Self::ChatCompletions => "chat_completions",
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_and_project(
    state: &AppState,
    request_event_id: &str,
    turn_intent_id: &str,
    request_hash: &str,
    request: LiveTransitionRequest,
    projection: Projection,
    model: &str,
    stream: bool,
    input_tokens: u64,
    natural_evidence_eligible: bool,
) -> Response {
    let policy = policy_status(&state.config);
    if !policy.effective_local_accept {
        return match projection {
            Projection::TransitionApi => json_response(
                StatusCode::CONFLICT,
                json!({
                    "schema": "nando.transition-execute-response.v1",
                    "local_accept": false,
                    "fallback_required": true,
                    "reason": policy.reason,
                }),
            ),
            Projection::Responses | Projection::ChatCompletions => fallback(state, &policy.reason),
        };
    }
    let executor = {
        let Ok(cache) = state.cache.read() else {
            return fallback_or_conflict(state, projection, "executor_cache_poisoned");
        };
        if !cache.ready {
            return fallback_or_conflict(state, projection, "executor_cache_not_ready");
        }
        cache.executor.clone()
    };
    let Some(executor) = executor else {
        return fallback_or_conflict(state, projection, "executor_unavailable");
    };
    let execution = executor.execute(&request);
    if let Err(reason) = validate_execution(&execution) {
        return fallback_or_conflict(state, projection, reason);
    }
    let Some(response_text) = execution.response.as_deref() else {
        return fallback_or_conflict(state, projection, "executor_response_missing");
    };
    let route = execution.route.as_deref().unwrap_or("typed_transition");
    let projected = match projection {
        Projection::TransitionApi => json!({
            "schema": "nando.transition-execute-response.v1",
            "local_accept": true,
            "fallback_required": false,
            "verifier_ok": true,
            "route": route,
            "transition": serde_json::from_str::<Value>(response_text).unwrap_or(Value::Null),
            "elapsed_ns": execution.elapsed_ns,
            "verification_receipt_id": execution.verification_receipt_id,
            "verified_after_digest": execution.verified_after_digest,
            "verifier_schema": execution.verifier_schema,
        }),
        Projection::Responses => {
            responses_projection(request_hash, response_text, model, route, input_tokens)
        }
        Projection::ChatCompletions => {
            chat_projection(request_hash, response_text, model, route, input_tokens)
        }
    };
    let projector_receipt_id = sha256_json(&projected).unwrap_or_default();
    if projection.avoids_upstream_llm_call() {
        state.counters.local_accepts.fetch_add(1, Ordering::Relaxed);
        write_event(
            state,
            json!({
                "schema": "nando.transition-execution-event.v1",
                "timestamp_unix": unix_now(),
                "event": "local_accept",
                "request_sha256": request_hash,
                "tokens": input_tokens,
                "route": route,
                "elapsed_ns": execution.elapsed_ns,
                "verification_receipt_id": execution.verification_receipt_id,
                "projector_receipt_id": projector_receipt_id,
                "verified_after_digest": execution.verified_after_digest,
                "verifier_schema": execution.verifier_schema,
                "worker": "rust_transition_serving",
            }),
        );
        write_economics(
            state,
            json!({
                "schema": "nando.economics-terminal.v1",
                "timestamp_unix": unix_now(),
                "client_intent_id": turn_intent_id,
                "request_event_id": request_event_id,
                "intent_dedupe_eligible": natural_evidence_eligible,
                "provider_attempt_id": Value::Null,
                "request_sha256": request_hash,
                "route": "local_actor",
                "terminal_state": "delivered",
                "input_tokens": input_tokens,
                "input_token_accounting": "request_body_byte_estimate_v1",
                "upstream_socket_opened": false,
                "avoided_call": true,
                "verification_status": "verified",
                "verification_receipt_id": execution.verification_receipt_id,
                "projector_receipt_id": projector_receipt_id,
                "local_route": route,
                "verified_after_digest": execution.verified_after_digest,
                "verifier_schema": execution.verifier_schema,
            }),
        );
        observe_live_economics_verified_accept(
            state,
            request_event_id,
            input_tokens,
            natural_evidence_eligible,
        );
    } else {
        write_event(
            state,
            json!({
                "schema": "nando.transition-execution-event.v1",
                "timestamp_unix": unix_now(),
                "event": "verified_transition_execution",
                "request_sha256": request_hash,
                "tokens": 0,
                "route": route,
                "elapsed_ns": execution.elapsed_ns,
                "verification_receipt_id": execution.verification_receipt_id,
                "projector_receipt_id": projector_receipt_id,
                "verified_after_digest": execution.verified_after_digest,
                "verifier_schema": execution.verifier_schema,
                "worker": "rust_transition_serving",
                "claim_boundary": "verification execution is not an avoided LLM call",
            }),
        );
    }

    match projection {
        Projection::Responses if stream => sse_response(responses_sse(&projected)),
        Projection::ChatCompletions if stream => sse_response(chat_sse(&projected)),
        Projection::TransitionApi | Projection::Responses | Projection::ChatCompletions => {
            json_response(StatusCode::OK, projected)
        }
    }
}

fn validate_execution(execution: &LiveTransitionResponse) -> Result<(), &'static str> {
    if !execution.local_accept || !execution.verifier_ok || execution.false_accepts != 0 {
        return Err("typed_local_declined");
    }
    if execution.verifier_schema.as_deref() != Some("typed_actor_independent_verifier.v1")
        || !execution
            .verification_receipt_id
            .as_deref()
            .is_some_and(valid_sha256)
        || !execution
            .verified_after_digest
            .as_deref()
            .is_some_and(valid_sha256)
    {
        return Err("typed_verification_receipt_missing");
    }
    let response = execution
        .response
        .as_deref()
        .ok_or("typed_response_missing")?;
    let payload = serde_json::from_str::<Value>(response).map_err(|_| "typed_response_invalid")?;
    let after = payload.get("after").ok_or("typed_after_missing")?;
    let digest = sha256_json(after).map_err(|_| "typed_after_digest_failed")?;
    if execution.verified_after_digest.as_deref() != Some(digest.as_str()) {
        return Err("typed_verified_after_digest_mismatch");
    }
    Ok(())
}

fn validate_observation(
    request: &ObservationRequest,
    request_hash: &str,
    state: &AppState,
) -> Result<(String, String, bool), (StatusCode, &'static str)> {
    if request.schema != OBSERVATION_REQUEST_SCHEMA {
        return Err((StatusCode::BAD_REQUEST, "unsupported_schema"));
    }
    if !matches!(
        request.evidence.source.as_str(),
        "application_state" | "tool_result" | "environment_snapshot"
    ) || request.evidence.verifier.is_empty()
    {
        return Err((StatusCode::BAD_REQUEST, "unsupported_evidence"));
    }
    let provenance = request.provenance.as_object().cloned().unwrap_or_default();
    let observed_at = request.observed_at.as_deref().unwrap_or("");
    if request.evidence.receipt_schema == "nando.grounded-transition-receipt.v2" {
        let session = provenance
            .get("source_session_id_sha256")
            .and_then(Value::as_str)
            .unwrap_or("");
        let event = provenance
            .get("source_event_id_sha256")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !valid_sha256(session)
            || !valid_sha256(event)
            || OffsetDateTime::parse(observed_at, &Rfc3339).is_err()
        {
            return Err((StatusCode::BAD_REQUEST, "v2_provenance_required"));
        }
        for key in ["call_input_sha256", "call_output_sha256"] {
            if let Some(value) = provenance.get(key).and_then(Value::as_str)
                && !value.is_empty()
                && !valid_sha256(value)
            {
                return Err((StatusCode::BAD_REQUEST, "invalid_provenance_digest"));
            }
        }
    }
    let material = if request.evidence.receipt_schema == "nando.grounded-transition-receipt.v2" {
        json!({
            "receipt_schema": request.evidence.receipt_schema,
            "before": request.before,
            "action": request.action,
            "after": request.after,
            "evidence_source": request.evidence.source,
            "evidence_verifier": request.evidence.verifier,
            "observed_at": observed_at,
            "provenance": provenance,
        })
    } else if request.evidence.receipt_schema == "nando.grounded-transition-receipt.v1" {
        json!({
            "before": request.before,
            "action": request.action,
            "after": request.after,
            "evidence_source": request.evidence.source,
            "evidence_verifier": request.evidence.verifier,
        })
    } else {
        return Err((StatusCode::BAD_REQUEST, "unsupported_receipt_schema"));
    };
    let receipt = sha256_json(&material)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "receipt_failed"))?;
    if request.evidence.receipt_sha256.to_ascii_lowercase() != receipt {
        return Err((StatusCode::BAD_REQUEST, "evidence_receipt_mismatch"));
    }
    let trace_id = request
        .trace_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or(request_hash)
        .to_owned();
    let input_tokens = request
        .usage
        .get("input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let output_tokens = request
        .usage
        .get("output_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total_tokens = request
        .usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(input_tokens.saturating_add(output_tokens));
    let row = json!({
        "schema": LIVE_GROUNDED_TRACE_SCHEMA,
        "trace_id": trace_id,
        "timestamp": observed_at,
        "before": request.before,
        "action": request.action,
        "after": request.after,
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
        "total_tokens": total_tokens,
        "request_sha256": request_hash,
        "evidence_source": request.evidence.source,
        "evidence_verifier": request.evidence.verifier,
        "evidence_receipt_sha256": receipt,
        "source_session_id_sha256": provenance.get("source_session_id_sha256").and_then(Value::as_str).unwrap_or(""),
        "source_event_id_sha256": provenance.get("source_event_id_sha256").and_then(Value::as_str).unwrap_or(""),
    });
    let appended = state
        .observations
        .append(&trace_id, &row)
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "trace_store_unavailable"))?;
    if appended {
        write_event(
            state,
            json!({
                "schema": "nando.transition-execution-event.v1",
                "timestamp_unix": unix_now(),
                "event": "grounded_transition_observation",
                "request_sha256": request_hash,
                "tokens": total_tokens,
                "trace_id": trace_id,
                "evidence_source": request.evidence.source,
                "evidence_verifier": request.evidence.verifier,
                "worker": "rust_transition_serving",
            }),
        );
    }
    Ok((trace_id, receipt, !appended))
}

fn policy_status(config: &ServingConfig) -> PolicyStatus {
    let mode = read_json(&config.mode_path)
        .and_then(|value| value.get("mode").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_else(|| "SHADOW".into());
    let admission = read_json(&config.admission_path).unwrap_or_default();
    let verdict = admission
        .get("verdict")
        .and_then(Value::as_str)
        .unwrap_or("MISSING")
        .to_ascii_uppercase();
    let eligible = admission
        .get("eligible_for_local_accept")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let generated_at = admission
        .get("generated_at_unix")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let fresh = generated_at > 0
        && unix_now().saturating_sub(generated_at) <= config.admission_max_age_seconds;
    let kill_switch = config.kill_switch_path.exists();
    let effective = mode == "CPU"
        && verdict == "PASS"
        && eligible
        && fresh
        && config.local_accept_enabled
        && config.client_allow_local_accept
        && config.route_ready
        && !kill_switch;
    let reason = if mode != "CPU" {
        format!("mode_{}", mode.to_ascii_lowercase())
    } else if verdict != "PASS" {
        format!("admission_{}", verdict.to_ascii_lowercase())
    } else if !eligible {
        "admission_not_eligible".into()
    } else if !fresh {
        "admission_stale".into()
    } else if !config.local_accept_enabled || !config.client_allow_local_accept {
        "local_accept_policy_disabled".into()
    } else if !config.route_ready {
        "runtime_route_not_ready".into()
    } else if kill_switch {
        "kill_switch".into()
    } else {
        "verified_cpu_ready".into()
    };
    PolicyStatus {
        mode,
        admission_verdict: verdict,
        admission_eligible: eligible,
        admission_fresh: fresh,
        local_accept_enabled: config.local_accept_enabled,
        client_allow_local_accept: config.client_allow_local_accept,
        route_ready: config.route_ready,
        kill_switch,
        effective_local_accept: effective,
        reason,
    }
}

fn refresh_response_executor(state: &AppState) {
    let fingerprint = response_authority_input_fingerprint(state);
    let now = unix_now();
    let renewal_margin = state
        .config
        .admission_max_age_seconds
        .saturating_div(3)
        .max(1);
    if let Ok(current_fingerprint) = fingerprint.as_ref()
        && let Ok(cache) = state.response_cache.read()
        && cache.input_fingerprint.as_ref() == Some(current_fingerprint)
    {
        if cache.ready && now.saturating_add(renewal_margin) < cache.admission_expires_at_unix {
            return;
        }
        // A rejected immutable authority snapshot cannot become valid until
        // one of its fingerprinted inputs changes. Cache the negative result
        // instead of re-reading and re-logging it for every live event.
        if !cache.ready && !cache.last_error.is_empty() {
            return;
        }
    }
    let load = || -> Result<(ResponseExecutor, String, String, u64), String> {
        let registry = fs::read(&state.config.response_registry_path)
            .map_err(|error| format!("response_registry_read:{error}"))?;
        let admission = fs::read(&state.config.admission_path)
            .map_err(|error| format!("response_admission_read:{error}"))?;
        let admission_expires_at_unix = serde_json::from_slice::<Value>(&admission)
            .ok()
            .and_then(|value| value.get("expires_at_unix").and_then(Value::as_u64))
            .ok_or_else(|| "response_admission_expiry_missing".to_owned())?;
        let (cached_gate_sha256, cached_runtime_sha256) = state
            .response_cache
            .read()
            .map(|cache| {
                (
                    cache.gate_build_sha256.clone(),
                    cache.runtime_build_sha256.clone(),
                )
            })
            .map_err(|_| "response_cache_lock_poisoned".to_owned())?;
        let gate_build_sha256 = if cached_gate_sha256.is_empty() {
            sha256_bytes(
                &fs::read(&state.config.gate_build_path)
                    .map_err(|error| format!("response_gate_build_read:{error}"))?,
            )
        } else {
            cached_gate_sha256
        };
        let runtime_build_sha256 = if cached_runtime_sha256.is_empty() {
            response_runtime_contract_sha256()
        } else {
            cached_runtime_sha256
        };
        let now = unix_now();
        ResponseExecutor::from_authorized_json(
            &registry,
            &admission,
            &state.config.project_id,
            &gate_build_sha256,
            &runtime_build_sha256,
            now,
            state.config.admission_max_age_seconds,
        )
        .and_then(|executor| {
            if now.saturating_add(renewal_margin) < admission_expires_at_unix {
                return Ok((executor, admission_expires_at_unix));
            }
            renew_admission_timestamps(
                &state.config.admission_path,
                &admission,
                now,
                state.config.admission_max_age_seconds,
            )?;
            Ok((
                executor,
                now.saturating_add(state.config.admission_max_age_seconds),
            ))
        })
        .or_else(|error| {
            if error != "response_admission_stale" && error != "response_admission_expired" {
                return Err(error);
            }
            let executor = ResponseExecutor::from_revalidated_authorized_json(
                &registry,
                &admission,
                &state.config.project_id,
                &gate_build_sha256,
                &runtime_build_sha256,
                now,
                state.config.admission_max_age_seconds,
            )?;
            renew_admission_timestamps(
                &state.config.admission_path,
                &admission,
                now,
                state.config.admission_max_age_seconds,
            )?;
            Ok((
                executor,
                now.saturating_add(state.config.admission_max_age_seconds),
            ))
        })
        .map(|(executor, expires_at_unix)| {
            (
                executor,
                gate_build_sha256,
                runtime_build_sha256,
                expires_at_unix,
            )
        })
    };
    match load() {
        Ok((executor, gate_build_sha256, runtime_build_sha256, expires_at_unix)) => {
            if let Ok(mut cache) = state.response_cache.write() {
                cache.executor = Some(Arc::new(executor));
                cache.ready = true;
                cache.gate_build_sha256 = gate_build_sha256;
                cache.runtime_build_sha256 = runtime_build_sha256;
                cache.input_fingerprint = response_authority_input_fingerprint(state).ok();
                cache.admission_expires_at_unix = expires_at_unix;
                cache.last_error.clear();
            }
        }
        Err(error) => {
            if let Ok(mut cache) = state.response_cache.write() {
                let bounded_error = bounded_reason(&error);
                let should_log = cache.input_fingerprint != fingerprint.as_ref().ok().cloned()
                    || cache.last_error != bounded_error;
                cache.executor = None;
                cache.ready = false;
                cache.input_fingerprint = fingerprint.ok();
                cache.last_error = bounded_error;
                if should_log {
                    eprintln!("nando-response-authority refresh: {error}");
                }
            }
        }
    }
}

fn renew_admission_timestamps(
    path: &Path,
    original: &[u8],
    now_unix: u64,
    max_age_seconds: u64,
) -> Result<(), String> {
    if fs::read(path).map_err(|error| format!("response_admission_reread:{error}"))? != original {
        return Err("response_admission_changed_during_revalidation".to_owned());
    }
    let mut admission: Value = serde_json::from_slice(original)
        .map_err(|error| format!("response_admission_value_parse:{error}"))?;
    admission["generated_at_unix"] = Value::from(now_unix);
    admission["expires_at_unix"] = Value::from(now_unix.saturating_add(max_age_seconds));
    let parent = path
        .parent()
        .ok_or_else(|| "response_admission_parent_missing".to_owned())?;
    let temporary = parent.join(format!(
        ".admission.{}.{}.tmp",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| format!("response_admission_temp_create:{error}"))?;
        serde_json::to_writer_pretty(&mut file, &admission)
            .map_err(|error| format!("response_admission_temp_encode:{error}"))?;
        file.write_all(b"\n")
            .map_err(|error| format!("response_admission_temp_write:{error}"))?;
        file.sync_all()
            .map_err(|error| format!("response_admission_temp_sync:{error}"))?;
        fs::rename(&temporary, path)
            .map_err(|error| format!("response_admission_rename:{error}"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn response_authority_input_fingerprint(
    state: &AppState,
) -> Result<(u64, u128, u64, u128), String> {
    let metadata = |path: &Path| -> Result<(u64, u128), String> {
        let metadata = fs::metadata(path).map_err(|error| format!("metadata:{error}"))?;
        let modified = metadata
            .modified()
            .map_err(|error| format!("modified:{error}"))?
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("modified_before_epoch:{error}"))?
            .as_nanos();
        Ok((metadata.len(), modified))
    };
    let registry = metadata(&state.config.response_registry_path)?;
    let admission = metadata(&state.config.admission_path)?;
    Ok((registry.0, registry.1, admission.0, admission.1))
}

fn response_cache_status(state: &AppState) -> (bool, u64, usize, String) {
    let Ok(cache) = state.response_cache.read() else {
        return (false, 0, 0, "response_cache_lock_poisoned".into());
    };
    let revision = cache
        .executor
        .as_ref()
        .map_or(0, |executor| executor.revision());
    let profiles = cache
        .executor
        .as_ref()
        .map_or(0, |executor| executor.active_package_count());
    (cache.ready, revision, profiles, cache.last_error.clone())
}

fn response_active_program_labels(state: &AppState) -> BTreeMap<String, usize> {
    state
        .response_cache
        .read()
        .ok()
        .and_then(|cache| {
            cache
                .executor
                .as_ref()
                .map(|executor| executor.active_program_labels())
        })
        .unwrap_or_default()
}

fn refresh_expression_shadow(state: &AppState) {
    let load = || -> Result<(ExpressionRuntime, String), String> {
        let candidate = read_json(&state.config.expression_candidate_path)
            .ok_or_else(|| "expression_candidate_missing".to_owned())?;
        if candidate.get("schema").and_then(Value::as_str)
            != Some("nando.expression-quarantine-candidate.v1")
            || candidate.get("state").and_then(Value::as_str) != Some("quarantine")
            || candidate
                .get("execution_authority")
                .and_then(Value::as_bool)
                != Some(false)
        {
            return Err("expression_candidate_contract_invalid".into());
        }
        let admission = read_json(&state.config.admission_path)
            .ok_or_else(|| "expression_admission_missing".to_owned())?;
        let expression_section = admission.pointer("/sections/expression_shadow");
        if expression_section
            .and_then(|section| section.get("verdict"))
            .and_then(Value::as_str)
            != Some("PASS")
        {
            return Err("expression_shadow_gate_not_pass".into());
        }
        let package_path = candidate
            .pointer("/package/path")
            .and_then(Value::as_str)
            .ok_or_else(|| "expression_package_path_missing".to_owned())?;
        let expected_sha = candidate
            .pointer("/package/sha256")
            .and_then(Value::as_str)
            .ok_or_else(|| "expression_package_sha_missing".to_owned())?;
        let bytes =
            fs::read(package_path).map_err(|error| format!("expression_package_read:{error}"))?;
        let actual_sha = sha256_bytes(&bytes);
        if actual_sha != expected_sha {
            return Err("expression_package_sha_mismatch".into());
        }
        let runtime = ExpressionRuntime::load(&bytes)
            .map_err(|error| format!("expression_package_load:{error}"))?;
        Ok((runtime, actual_sha))
    };
    let Ok(mut cache) = state.expression_shadow_cache.write() else {
        return;
    };
    match load() {
        Ok((runtime, package_sha256)) => {
            cache.runtime = Some(Arc::new(runtime));
            cache.ready = true;
            cache.package_sha256 = package_sha256;
            cache.last_error.clear();
        }
        Err(error) => {
            cache.runtime = None;
            cache.ready = false;
            cache.package_sha256.clear();
            cache.last_error = bounded_reason(&error);
        }
    }
}

fn expression_shadow_cache_status(state: &AppState) -> (bool, String, String) {
    let Ok(cache) = state.expression_shadow_cache.read() else {
        return (
            false,
            String::new(),
            "expression_shadow_cache_lock_poisoned".into(),
        );
    };
    (
        cache.ready,
        cache.package_sha256.clone(),
        cache.last_error.clone(),
    )
}

fn expression_shadow_runtime(state: &AppState) -> Option<Arc<ExpressionRuntime>> {
    state.expression_shadow_cache.read().ok()?.runtime.clone()
}

fn record_expression_shadow_request(
    state: &AppState,
    before: &Value,
    action: &Value,
    input_tokens: u64,
) {
    state
        .counters
        .expression_shadow_requests
        .fetch_add(1, Ordering::Relaxed);
    let Some(runtime) = expression_shadow_runtime(state) else {
        state
            .counters
            .expression_shadow_cache_unavailable
            .fetch_add(1, Ordering::Relaxed);
        return;
    };
    if runtime.execute(before, action).after.is_some() {
        state
            .counters
            .expression_shadow_would_execute
            .fetch_add(1, Ordering::Relaxed);
        state
            .counters
            .expression_shadow_potential_input_tokens
            .fetch_add(input_tokens, Ordering::Relaxed);
    } else {
        state
            .counters
            .expression_shadow_abstains
            .fetch_add(1, Ordering::Relaxed);
    }
}

fn record_expression_shadow_observation(
    state: &AppState,
    before: &Value,
    action: &Value,
    expected_after: &Value,
) {
    state
        .counters
        .expression_shadow_observations
        .fetch_add(1, Ordering::Relaxed);
    let Some(runtime) = expression_shadow_runtime(state) else {
        state
            .counters
            .expression_shadow_cache_unavailable
            .fetch_add(1, Ordering::Relaxed);
        return;
    };
    match runtime.execute(before, action).after {
        Some(after) if after == *expected_after => {
            state
                .counters
                .expression_shadow_verified_matches
                .fetch_add(1, Ordering::Relaxed);
        }
        Some(_) => {
            state
                .counters
                .expression_shadow_wrong
                .fetch_add(1, Ordering::Relaxed);
        }
        None => {
            state
                .counters
                .expression_shadow_abstains
                .fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn refresh_executor(state: &AppState) {
    match LiveTransitionExecutor::load(&state.config.registry_path) {
        Ok(executor) => {
            if let Ok(mut cache) = state.cache.write() {
                cache.executor = Some(Arc::new(executor));
                cache.ready = true;
                cache.last_error.clear();
            }
        }
        Err(error) => {
            state.counters.errors.fetch_add(1, Ordering::Relaxed);
            if let Ok(mut cache) = state.cache.write() {
                cache.ready = false;
                cache.last_error = bounded_reason(&error);
            }
        }
    }
}

fn cache_status(state: &AppState) -> (bool, u64, usize, String) {
    let Ok(cache) = state.cache.read() else {
        return (false, 0, 0, "cache_lock_poisoned".into());
    };
    let revision = cache
        .executor
        .as_ref()
        .map_or(0, |executor| executor.registry_revision());
    let profiles = cache
        .executor
        .as_ref()
        .map_or(0, |executor| executor.active_profile_count());
    (cache.ready, revision, profiles, cache.last_error.clone())
}

fn transition_envelope(payload: &Value) -> Option<(Value, Value)> {
    let envelope = payload.get("metadata")?.get("nando_transition")?;
    Some((
        envelope.get("before")?.clone(),
        envelope.get("action")?.clone(),
    ))
}

fn normalize_chat_messages_for_actor(payload: &Value) -> Option<Value> {
    let messages = payload.get("messages")?.as_array()?;
    if messages.is_empty() {
        return None;
    }
    let mut input = Vec::new();
    let mut pending_function_calls = std::collections::BTreeSet::new();
    for message in messages {
        let role = message.get("role").and_then(Value::as_str)?;
        match role {
            "system" | "developer" | "user" => {
                if !message_content_has_supported_shape(message.get("content")?) {
                    return None;
                }
                input.push(json!({"type":"message","role":role}));
            }
            "assistant" => {
                let has_text = message
                    .get("content")
                    .is_some_and(message_content_has_nonempty_text);
                if has_text {
                    input.push(json!({"type":"message","role":"assistant"}));
                } else if !matches!(message.get("content"), None | Some(Value::Null)) {
                    return None;
                }
                if let Some(tool_calls) = message.get("tool_calls") {
                    let tool_calls = tool_calls.as_array()?;
                    if tool_calls.is_empty() {
                        return None;
                    }
                    for call in tool_calls {
                        if call.get("type").and_then(Value::as_str) != Some("function") {
                            return None;
                        }
                        let call_id = call.get("id").and_then(Value::as_str)?;
                        let function = call.get("function")?.as_object()?;
                        let name = function.get("name").and_then(Value::as_str)?;
                        let arguments = function.get("arguments").and_then(Value::as_str)?;
                        if call_id.is_empty()
                            || name.is_empty()
                            || !pending_function_calls.insert(call_id.to_owned())
                        {
                            return None;
                        }
                        input.push(json!({
                            "type":"function_call",
                            "call_id":call_id,
                            "name":name,
                            "arguments":arguments,
                        }));
                    }
                } else if !has_text {
                    return None;
                }
            }
            "tool" => {
                let call_id = message.get("tool_call_id").and_then(Value::as_str)?;
                if !pending_function_calls.remove(call_id) {
                    return None;
                }
                let output = canonical_chat_tool_output(message.get("content")?)?;
                input.push(json!({
                    "type":"function_call_output",
                    "call_id":call_id,
                    "output":output,
                }));
            }
            _ => return None,
        }
    }
    let mut normalized = json!({"input":input});
    if let Some(model) = payload.get("model").and_then(Value::as_str) {
        normalized["model"] = Value::String(model.to_owned());
    }
    if let Some(stream) = payload.get("stream").and_then(Value::as_bool) {
        normalized["stream"] = Value::Bool(stream);
    }
    Some(normalized)
}

fn message_content_has_supported_shape(content: &Value) -> bool {
    match content {
        Value::String(_) => true,
        Value::Array(parts) if !parts.is_empty() => parts.iter().all(|part| {
            part.get("type").and_then(Value::as_str) == Some("text")
                && part.get("text").and_then(Value::as_str).is_some()
        }),
        _ => false,
    }
}

fn message_content_has_nonempty_text(content: &Value) -> bool {
    match content {
        Value::String(text) => !text.is_empty(),
        Value::Array(parts) if !parts.is_empty() => parts.iter().all(|part| {
            part.get("type").and_then(Value::as_str) == Some("text")
                && part
                    .get("text")
                    .and_then(Value::as_str)
                    .is_some_and(|text| !text.is_empty())
        }),
        _ => false,
    }
}

fn canonical_chat_tool_output(content: &Value) -> Option<Value> {
    if let Some(text) = content.as_str() {
        return Some(Value::String(text.to_owned()));
    }
    let parts = content.as_array()?;
    if parts.is_empty() || parts.len() > 64 {
        return None;
    }
    parts
        .iter()
        .map(|part| {
            let part_type = part.get("type").and_then(Value::as_str)?;
            if !matches!(part_type, "text" | "input_text" | "output_text") {
                return None;
            }
            let text = part.get("text").and_then(Value::as_str)?;
            Some(json!({"type":part_type,"text":text}))
        })
        .collect::<Option<Vec<_>>>()
        .map(Value::Array)
}

fn extract_request_text(payload: &Value) -> String {
    if let Some(input) = payload.get("input") {
        if let Some(text) = input.as_str() {
            return text.to_owned();
        }
        if let Some(messages) = input.as_array()
            && let Some(text) = latest_user_text(messages)
        {
            return text;
        }
    }
    if let Some(prompt) = payload.get("prompt").and_then(Value::as_str) {
        return prompt.to_owned();
    }
    payload
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| latest_user_text(messages))
        .unwrap_or_default()
}

fn latest_user_text(messages: &[Value]) -> Option<String> {
    let mut fallback = None;
    let mut user = None;
    for message in messages {
        let Some(content) = message.get("content") else {
            continue;
        };
        let text = message_content_text(content);
        if text.is_empty() {
            continue;
        }
        fallback = Some(text.clone());
        if message.get("role").and_then(Value::as_str) == Some("user") {
            user = Some(text);
        }
    }
    user.or(fallback)
}

fn provider_request_shape(
    payload: &Value,
    projection: Projection,
    request_text: &str,
    capability_atom_ids: &[u64],
) -> Value {
    let top_level_keys = sorted_object_keys(payload);
    let metadata_keys = payload
        .get("metadata")
        .map(sorted_object_keys)
        .unwrap_or_default();
    let client_metadata = payload.get("client_metadata");
    let client_metadata_keys = client_metadata.map(sorted_object_keys).unwrap_or_default();
    let client_identity_sha256 = hashed_string_fields(
        client_metadata,
        &["session_id", "thread_id", "turn_id", "x-codex-window-id"],
    );
    let input = payload.get("input");
    let messages = payload.get("messages");
    let mut item_types = BTreeMap::<String, usize>::new();
    let mut roles = BTreeMap::<String, usize>::new();
    let mut content_part_types = BTreeMap::<String, usize>::new();
    for item in input
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .chain(messages.and_then(Value::as_array).into_iter().flatten())
    {
        increment_shape_count(
            &mut item_types,
            item.get("type")
                .and_then(Value::as_str)
                .unwrap_or("message"),
        );
        if let Some(role) = item.get("role").and_then(Value::as_str) {
            increment_shape_count(&mut roles, role);
        }
        if let Some(parts) = item.get("content").and_then(Value::as_array) {
            for part in parts {
                increment_shape_count(
                    &mut content_part_types,
                    part.get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                );
            }
        }
    }
    let instructions = payload
        .get("instructions")
        .and_then(Value::as_str)
        .unwrap_or("");
    json!({
        "schema": "nando.provider-request-shape.v1",
        "endpoint": projection.endpoint(),
        "top_level_keys": top_level_keys,
        "metadata_keys": metadata_keys,
        "client_metadata_keys": client_metadata_keys,
        "client_identity_sha256": client_identity_sha256,
        "input_kind": value_kind(input),
        "input_items": input.and_then(Value::as_array).map_or(0, Vec::len),
        "message_items": messages.and_then(Value::as_array).map_or(0, Vec::len),
        "item_types": item_types,
        "roles": roles,
        "content_part_types": content_part_types,
        "tools_count": payload.get("tools").and_then(Value::as_array).map_or(0, Vec::len),
        "capability_atom_ids": capability_atom_ids,
        "request_text_bytes": request_text.len(),
        "request_text_sha256": sha256_bytes(request_text.as_bytes()),
        "instructions_bytes": instructions.len(),
        "instructions_sha256": sha256_bytes(instructions.as_bytes()),
        "prompt_cache_key_sha256": optional_string_sha256(payload.get("prompt_cache_key")),
        "previous_response_id_sha256": optional_string_sha256(payload.get("previous_response_id")),
        "raw_text_stored": false,
    })
}

fn sorted_object_keys(value: &Value) -> Vec<String> {
    let mut keys = value
        .as_object()
        .map(|object| object.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    keys.sort();
    keys
}

fn increment_shape_count(counts: &mut BTreeMap<String, usize>, value: &str) {
    let entry = counts.entry(value.to_owned()).or_default();
    *entry = entry.saturating_add(1);
}

fn hashed_string_fields(value: Option<&Value>, keys: &[&str]) -> BTreeMap<String, String> {
    let Some(object) = value.and_then(Value::as_object) else {
        return BTreeMap::new();
    };
    keys.iter()
        .filter_map(|key| {
            object
                .get(*key)
                .and_then(Value::as_str)
                .map(|text| ((*key).to_owned(), sha256_bytes(text.as_bytes())))
        })
        .collect()
}

fn value_kind(value: Option<&Value>) -> &'static str {
    match value {
        None => "missing",
        Some(Value::Null) => "null",
        Some(Value::Bool(_)) => "bool",
        Some(Value::Number(_)) => "number",
        Some(Value::String(_)) => "string",
        Some(Value::Array(_)) => "array",
        Some(Value::Object(_)) => "object",
    }
}

fn optional_string_sha256(value: Option<&Value>) -> Value {
    value
        .and_then(Value::as_str)
        .map(|text| Value::String(sha256_bytes(text.as_bytes())))
        .unwrap_or(Value::Null)
}

fn message_content_text(content: &Value) -> String {
    if let Some(text) = content.as_str() {
        return text.to_owned();
    }
    content
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn responses_projection(
    request_hash: &str,
    text: &str,
    model: &str,
    route: &str,
    input_tokens: u64,
) -> Value {
    let suffix = request_hash.get(..16).unwrap_or(request_hash);
    let output_tokens = token_estimate(text);
    json!({
        "id": format!("resp_nando_{suffix}"),
        "object": "response",
        "created_at": unix_now(),
        "status": "completed",
        "model": model,
        "output": [{
            "id": format!("msg_nando_{suffix}"),
            "type": "message",
            "status": "completed",
            "role": "assistant",
            "content": [{
                "type": "output_text",
                "annotations": [],
                "logprobs": [],
                "text": text,
            }],
        }],
        "output_text": text,
        "usage": {
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "total_tokens": input_tokens.saturating_add(output_tokens),
        },
        "nando": {
            "api_version": "v2",
            "local_accept": true,
            "route": route,
            "false_accepts": 0,
            "architecture": "wave_router_typed_actor_verifier",
            "transition_runtime": true,
        },
    })
}

fn chat_projection(
    request_hash: &str,
    text: &str,
    model: &str,
    route: &str,
    input_tokens: u64,
) -> Value {
    let suffix = request_hash.get(..16).unwrap_or(request_hash);
    let output_tokens = token_estimate(text);
    json!({
        "id": format!("chatcmpl-nando-{suffix}"),
        "object": "chat.completion",
        "created": unix_now(),
        "model": model,
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": text},
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": input_tokens,
            "completion_tokens": output_tokens,
            "total_tokens": input_tokens.saturating_add(output_tokens),
        },
        "nando": {
            "api_version": "v2",
            "local_accept": true,
            "route": route,
            "false_accepts": 0,
            "architecture": "wave_router_typed_actor_verifier",
            "transition_runtime": true,
        },
    })
}

fn responses_sse(response: &Value) -> String {
    let item = response
        .get("output")
        .and_then(Value::as_array)
        .and_then(|output| output.first())
        .cloned()
        .unwrap_or_default();
    if item.get("type").and_then(Value::as_str) == Some("function_call") {
        return responses_function_call_sse(response, &item);
    }
    if item.get("type").and_then(Value::as_str) == Some("custom_tool_call") {
        return custom_tool_projection::responses_sse(response, &item);
    }
    let part = item
        .get("content")
        .and_then(Value::as_array)
        .and_then(|content| content.first())
        .cloned()
        .unwrap_or_default();
    let item_id = item.get("id").and_then(Value::as_str).unwrap_or("");
    let text = part.get("text").and_then(Value::as_str).unwrap_or("");
    let mut in_progress = response.clone();
    in_progress["status"] = Value::String("in_progress".into());
    in_progress["output"] = Value::Array(Vec::new());
    let events = vec![
        ("response.created", json!({"response": in_progress})),
        (
            "response.output_item.added",
            json!({"output_index":0,"item":{"id":item_id,"type":"message","status":"in_progress","role":"assistant","content":[]}}),
        ),
        (
            "response.content_part.added",
            json!({"item_id":item_id,"output_index":0,"content_index":0,"part":{"type":"output_text","annotations":[],"text":""}}),
        ),
        (
            "response.output_text.delta",
            json!({"item_id":item_id,"output_index":0,"content_index":0,"delta":text,"logprobs":[]}),
        ),
        (
            "response.output_text.done",
            json!({"item_id":item_id,"output_index":0,"content_index":0,"text":text,"logprobs":[]}),
        ),
        (
            "response.content_part.done",
            json!({"item_id":item_id,"output_index":0,"content_index":0,"part":part}),
        ),
        (
            "response.output_item.done",
            json!({"output_index":0,"item":item}),
        ),
        ("response.completed", json!({"response":response})),
    ];
    let mut body = String::new();
    for (sequence, (event, payload)) in events.into_iter().enumerate() {
        let mut payload = payload;
        payload["type"] = Value::String(event.into());
        payload["sequence_number"] = Value::from(sequence);
        body.push_str("event: ");
        body.push_str(event);
        body.push_str("\ndata: ");
        body.push_str(&serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into()));
        body.push_str("\n\n");
    }
    body.push_str("data: [DONE]\n\n");
    body
}

fn responses_function_call_sse(response: &Value, item: &Value) -> String {
    let item_id = item.get("id").and_then(Value::as_str).unwrap_or("");
    let arguments = item
        .get("arguments")
        .and_then(Value::as_str)
        .unwrap_or("{}");
    let mut in_progress = response.clone();
    in_progress["status"] = Value::String("in_progress".into());
    in_progress["output"] = Value::Array(Vec::new());
    let mut added = item.clone();
    added["arguments"] = Value::String(String::new());
    added["status"] = Value::String("in_progress".into());
    let events = vec![
        ("response.created", json!({"response":in_progress})),
        (
            "response.output_item.added",
            json!({"output_index":0,"item":added}),
        ),
        (
            "response.function_call_arguments.delta",
            json!({"item_id":item_id,"output_index":0,"delta":arguments}),
        ),
        (
            "response.function_call_arguments.done",
            json!({"item_id":item_id,"output_index":0,"arguments":arguments}),
        ),
        (
            "response.output_item.done",
            json!({"output_index":0,"item":item}),
        ),
        ("response.completed", json!({"response":response})),
    ];
    sse_events(events)
}

fn sse_events(events: Vec<(&str, Value)>) -> String {
    let mut body = String::new();
    for (sequence, (event, mut payload)) in events.into_iter().enumerate() {
        payload["type"] = Value::String(event.into());
        payload["sequence_number"] = Value::from(sequence);
        body.push_str("event: ");
        body.push_str(event);
        body.push_str("\ndata: ");
        body.push_str(&serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into()));
        body.push_str("\n\n");
    }
    body.push_str("data: [DONE]\n\n");
    body
}

fn chat_sse(response: &Value) -> String {
    let id = response.get("id").and_then(Value::as_str).unwrap_or("");
    let model = response.get("model").and_then(Value::as_str).unwrap_or("");
    let created = response.get("created").and_then(Value::as_u64).unwrap_or(0);
    if let Some(tool_call) = response
        .pointer("/choices/0/message/tool_calls/0")
        .and_then(Value::as_object)
    {
        let call_id = tool_call.get("id").and_then(Value::as_str).unwrap_or("");
        let function = tool_call.get("function").and_then(Value::as_object);
        let name = function
            .and_then(|function| function.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let arguments = function
            .and_then(|function| function.get("arguments"))
            .and_then(Value::as_str)
            .unwrap_or("");
        return chat_sse_body(&[
            json!({"id":id,"object":"chat.completion.chunk","created":created,"model":model,"choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}),
            json!({"id":id,"object":"chat.completion.chunk","created":created,"model":model,"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":call_id,"type":"function","function":{"name":name,"arguments":""}}]},"finish_reason":null}]}),
            json!({"id":id,"object":"chat.completion.chunk","created":created,"model":model,"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":arguments}}]},"finish_reason":null}]}),
            json!({"id":id,"object":"chat.completion.chunk","created":created,"model":model,"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}),
        ]);
    }
    let text = response
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .unwrap_or("");
    let chunks = [
        json!({"id":id,"object":"chat.completion.chunk","created":created,"model":model,"choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}),
        json!({"id":id,"object":"chat.completion.chunk","created":created,"model":model,"choices":[{"index":0,"delta":{"content":text},"finish_reason":null}]}),
        json!({"id":id,"object":"chat.completion.chunk","created":created,"model":model,"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}),
    ];
    chat_sse_body(&chunks)
}

fn chat_sse_body(chunks: &[Value]) -> String {
    let mut body = chunks
        .iter()
        .map(|chunk| {
            format!(
                "data: {}\n\n",
                serde_json::to_string(chunk).unwrap_or_else(|_| "{}".into())
            )
        })
        .collect::<String>();
    body.push_str("data: [DONE]\n\n");
    body
}

fn fallback_or_conflict(state: &AppState, projection: Projection, reason: &str) -> Response {
    match projection {
        Projection::TransitionApi => json_response(
            StatusCode::CONFLICT,
            json!({
                "schema": "nando.transition-execute-response.v1",
                "local_accept": false,
                "fallback_required": true,
                "reason": reason,
            }),
        ),
        Projection::Responses | Projection::ChatCompletions => fallback(state, reason),
    }
}

fn fallback(state: &AppState, reason: &str) -> Response {
    state.counters.fallbacks.fetch_add(1, Ordering::Relaxed);
    let reason = bounded_reason(reason);
    let mut response = (
        StatusCode::IM_A_TEAPOT,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&json!({
            "schema": "nando.transition-abstain.v1",
            "local_accept": false,
            "fallback_required": true,
            "reason": reason,
        }))
        .unwrap_or_else(|_| "{}".into()),
    )
        .into_response();
    response.headers_mut().insert(
        "x-nando-fallback",
        HeaderValue::from_static("upstream_required"),
    );
    response
}

fn json_response(status: StatusCode, payload: Value) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into()),
    )
        .into_response()
}

fn sse_response(body: String) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/event-stream"),
            (header::CACHE_CONTROL, "no-cache"),
            (header::CONNECTION, "keep-alive"),
            (HeaderName::from_static("x-nando-local-accept"), "true"),
        ],
        body,
    )
        .into_response()
}

fn error_response(status: StatusCode, reason: &'static str) -> Response {
    json_response(
        status,
        json!({"error":{"message":reason,"type":"bad_request"}}),
    )
}

fn write_event(state: &AppState, row: Value) {
    if !state.config.legacy_json_audit_enabled {
        return;
    }
    let Ok(_guard) = state.event_lock.lock() else {
        state.counters.errors.fetch_add(1, Ordering::Relaxed);
        return;
    };
    if append_json_line(&state.config.event_path, &row).is_err() {
        state.counters.errors.fetch_add(1, Ordering::Relaxed);
    }
}

fn write_economics(state: &AppState, row: Value) {
    if !state.config.legacy_json_audit_enabled {
        return;
    }
    let Ok(_guard) = state.event_lock.lock() else {
        state.counters.errors.fetch_add(1, Ordering::Relaxed);
        return;
    };
    if append_json_line(&state.config.economics_path, &row).is_err() {
        state.counters.errors.fetch_add(1, Ordering::Relaxed);
    }
}

fn observe_live_economics_request(
    state: &AppState,
    request_event_id: &str,
    request_body: Bytes,
    input_tokens: u64,
    eligible: bool,
) {
    let intent_sha256 = sha256_bytes(request_event_id.as_bytes());
    let result =
        state
            .live_economics
            .observe_request(intent_sha256.clone(), request_body, eligible);
    if let Err(error) = result {
        state.counters.errors.fetch_add(1, Ordering::Relaxed);
        eprintln!("nando-live-economics request: {error}");
    }
    if eligible {
        submit_opportunity_event(
            state,
            OpportunityBridgeEventV1::request(intent_sha256, input_tokens, unix_now()),
            "request",
        );
    }
}

fn observe_live_economics_verified_accept(
    state: &AppState,
    request_event_id: &str,
    input_tokens: u64,
    natural_evidence_eligible: bool,
) {
    if !natural_evidence_eligible {
        return;
    }
    let intent_sha256 = sha256_bytes(request_event_id.as_bytes());
    let result = state
        .live_economics
        .observe_verified(intent_sha256.clone(), input_tokens);
    if let Err(error) = result {
        state.counters.errors.fetch_add(1, Ordering::Relaxed);
        eprintln!("nando-live-economics accept: {error}");
    }
    submit_opportunity_event(
        state,
        OpportunityBridgeEventV1::verified(intent_sha256),
        "verified",
    );
}

fn submit_opportunity_classification(
    state: &AppState,
    intent_sha256: String,
    class: ReducibilityClass,
    blocker: &str,
) {
    submit_opportunity_event(
        state,
        OpportunityBridgeEventV1::classify(intent_sha256, class, bounded_reason(blocker)),
        "classification",
    );
}

fn submit_opportunity_event(state: &AppState, event: OpportunityBridgeEventV1, event_kind: &str) {
    let result = if let Some(worker) = current_miner_worker(state) {
        worker.submit_opportunity_event(event)
    } else if state.opportunity_bridge.producer_enabled() {
        state.opportunity_bridge.submit(event)
    } else {
        return;
    };
    if let Err(error) = result {
        state.counters.errors.fetch_add(1, Ordering::Relaxed);
        eprintln!("nando-response-miner opportunity {event_kind}: {error}");
    }
}

fn decidability_reducibility(decidability: CpuDecidability) -> ReducibilityClass {
    match decidability.class {
        CpuDecidabilityClass::PotentiallyCpuExecutable => ReducibilityClass::ExecutableCandidate,
        CpuDecidabilityClass::UnexploredMultiSource => ReducibilityClass::UnexploredMultiSource,
        CpuDecidabilityClass::UnsupportedByCurrentDsl => ReducibilityClass::MissingDslPrimitive,
        CpuDecidabilityClass::NotExecutableCurrentEvidence
            if decidability.reason.contains("ambiguous")
                || decidability.reason.contains("multiple_competing") =>
        {
            ReducibilityClass::AmbiguousPreActionState
        }
        CpuDecidabilityClass::NotExecutableCurrentEvidence => {
            ReducibilityClass::NonDeterministicOrCreative
        }
    }
}

fn fallback_reducibility(stage: &str, reason: &str) -> ReducibilityClass {
    match stage {
        "admission" | "verifier" => ReducibilityClass::MissingExternalVerifier,
        "router" => ReducibilityClass::InsufficientRepetition,
        "adapter" => ReducibilityClass::MissingDslPrimitive,
        "actor" if reason.contains("ambiguous") || reason.contains("selector") => {
            ReducibilityClass::AmbiguousPreActionState
        }
        "runtime" if reason.contains("stale") || reason.contains("expired") => {
            ReducibilityClass::StaleOrInvalidEvidence
        }
        "actor" | "runtime" => ReducibilityClass::ExecutableCandidate,
        _ => ReducibilityClass::UnclassifiedBug,
    }
}

fn append_json_line(path: &Path, row: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("append_open:{}:{error}", path.display()))?;
    serde_json::to_writer(&mut file, row)
        .map_err(|error| format!("append_json:{}:{error}", path.display()))?;
    file.write_all(b"\n")
        .map_err(|error| format!("append_write:{}:{error}", path.display()))?;
    file.flush()
        .map_err(|error| format!("append_flush:{}:{error}", path.display()))
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("path_without_parent:{}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| format!("mkdir:{}:{error}", parent.display()))
}

fn sha256_json(value: &Value) -> Result<String, serde_json::Error> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn token_estimate(text: &str) -> u64 {
    u64::try_from(text.len().div_ceil(4)).unwrap_or(u64::MAX)
}

fn request_traffic_source<'a>(headers: &'a HeaderMap, payload: &'a Value) -> &'a str {
    headers
        .get("x-nando-traffic-source")
        .and_then(|value| value.to_str().ok())
        .or_else(|| {
            payload
                .pointer("/metadata/nando_traffic_source")
                .and_then(Value::as_str)
        })
        .unwrap_or("ordinary")
}

fn traffic_source_natural_evidence_eligible(traffic_source: &str) -> bool {
    traffic_source_dedupe_eligible(traffic_source)
}

fn traffic_source_dedupe_eligible(traffic_source: &str) -> bool {
    !traffic_source.starts_with("controlled_")
        && !traffic_source.starts_with("dogfood_")
        && !matches!(traffic_source, "smoke" | "fixture" | "audit")
}

fn bounded_reason(reason: &str) -> String {
    reason
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || "_:-. ".contains(*character))
        .take(MAX_REASON_BYTES)
        .collect()
}

fn default_receipt_schema() -> String {
    "nando.grounded-transition-receipt.v1".into()
}

fn env_flag(name: &str) -> bool {
    env::var(name)
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

fn generation_shadow_config(config: &ServingConfig) -> GenerationShadowConfigV3 {
    GenerationShadowConfigV3 {
        enabled: config.operator_generation_shadow_enabled,
        store_path: config.operator_generation_store_path.clone(),
        // This index commits raw provider requests. Session JSONL evidence is
        // a different physical stream and cannot satisfy the F6 request root.
        capture_index_path: config.operator_generation_capture_index_path.clone(),
        provider_capture_store_path: config.provider_capture_store_path.clone(),
        receipt_store_path: config.operator_generation_shadow_receipt_store_path.clone(),
        queue_capacity: config.operator_generation_shadow_queue_capacity,
        poll_interval: Duration::from_millis(config.operator_generation_shadow_poll_ms),
    }
}

fn provider_capture_config(config: &ServingConfig) -> ProviderCaptureConfigV3 {
    ProviderCaptureConfigV3 {
        enabled: config.provider_capture_enabled,
        store_path: config.provider_capture_store_path.clone(),
        queue_capacity: config.provider_capture_queue_capacity,
    }
}

fn env_u64(name: &str, fallback: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

fn env_usize(name: &str, fallback: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

fn env_path(name: &str, fallback: &str) -> PathBuf {
    env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(fallback))
}

fn env_path_join(name: &str, root: &Path, fallback: &str) -> PathBuf {
    env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(fallback))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use nando_response_actor::{
        AtomValueType, COMPOSITE_ADMISSION_SCHEMA_V2, CompositeResponseAdmissionV2,
        ProjectStatusMapping, RESPONSE_AUTHORITY_SCHEMA_V2, RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2,
        RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2, RESPONSE_REGISTRY_SCHEMA_V6,
        RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1, RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1,
        RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1, ResponseAuthorityV2, ResponsePackage,
        ResponsePackageAuthorityBindingV2, ResponsePackageOrigin, ResponsePackageProof,
        ResponsePackageState, ResponseProgram, ResponseRegistry, ResponseValueSelector,
        VerifierProgram, response_actor_program_digest, response_execution_payload_digest,
        response_independent_verifier_program_digest, response_package_digest,
        response_program_required_routing_atom_ids, response_proof_receipts_digest,
        response_registry_digest,
    };
    use std::path::Path;
    use std::sync::atomic::AtomicU64;

    use super::*;

    static PROJECT_STATUS_TEST_ID: AtomicU64 = AtomicU64::new(0);
    const STATUS_PROJECTION_EXTERNAL_VERIFIER_SCHEMA: &str =
        "status_projection_external_evidence.v1";

    #[test]
    fn observation_dedupe_memory_is_bounded() {
        let mut ids = ObservationIds::default();
        for value in 0..=OBSERVATION_DEDUPE_CAPACITY {
            let digest = Sha256::digest(value.to_le_bytes()).into();
            assert!(ids.insert(digest));
        }
        assert_eq!(ids.digests.len(), OBSERVATION_DEDUPE_CAPACITY);
        assert_eq!(ids.insertion_order.len(), OBSERVATION_DEDUPE_CAPACITY);
        let expired = Sha256::digest(0_usize.to_le_bytes()).into();
        let newest = Sha256::digest(OBSERVATION_DEDUPE_CAPACITY.to_le_bytes()).into();
        assert!(!ids.contains(&expired));
        assert!(ids.contains(&newest));
    }

    #[test]
    fn observation_trace_rotates_before_exceeding_segment_budget() {
        let root = std::env::temp_dir().join(format!(
            "nando-observation-trace-rotation-{}-{}",
            std::process::id(),
            PROJECT_STATUS_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test root");
        let path = root.join("observations.jsonl");
        fs::File::create(&path)
            .expect("trace")
            .set_len(OBSERVATION_TRACE_SEGMENT_BYTES)
            .expect("sparse trace");
        append_rotating_trace(&path, b"{\"trace_id\":\"next\"}\n").expect("rotate and append");
        assert_eq!(
            fs::metadata(&path).expect("current trace").len(),
            b"{\"trace_id\":\"next\"}\n".len() as u64
        );
        assert_eq!(
            fs::metadata(trace_segment_path(&path, 1))
                .expect("rotated trace")
                .len(),
            OBSERVATION_TRACE_SEGMENT_BYTES
        );
        fs::remove_dir_all(root).expect("remove test root");
    }

    fn project_status_registry() -> ResponseRegistry {
        let selector = ResponseValueSelector::JsonField {
            field: "exit_code".to_owned(),
            value_type: AtomValueType::Integer,
        };
        let program = ResponseProgram::project_status(
            selector.clone(),
            ProjectStatusMapping::ZeroIsSuccess,
            "completed",
        );
        let required_routing_atom_ids = response_program_required_routing_atom_ids(&program);
        let package = ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: "serving-project-status".to_owned(),
            origin: ResponsePackageOrigin::GroundedSynthesis,
            state: ResponsePackageState::Active,
            program,
            verifier: Some(VerifierProgram::ProjectStatus {
                selector,
                mapping: ProjectStatusMapping::ZeroIsSuccess,
                renderer: nando_response_actor::CollectionOutputRenderer::Direct,
                completion_state: "completed".to_owned(),
                require_unique_value: true,
            }),
            routing_predicates: Vec::new(),
            phase_centers: required_routing_atom_ids.clone(),
            required_routing_atom_ids,
            anti_centers: Vec::new(),
            wave_margin_micro: 1,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: ResponsePackageProof {
                support_rows: 32,
                future_rows: 32,
                distinct_sessions: 3,
                distinct_surfaces: 2,
                wrong_accepts: 0,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass: true,
                verifier_schema: STATUS_PROJECTION_EXTERNAL_VERIFIER_SCHEMA.to_owned(),
                adaptive_identification: None,
            },
        };
        ResponseRegistry {
            schema: RESPONSE_REGISTRY_SCHEMA_V6.to_owned(),
            revision: 7,
            packages: vec![package],
        }
    }

    fn write_json(path: &Path, value: &Value) {
        fs::write(path, serde_json::to_vec(value).expect("json bytes")).expect("write json");
    }

    fn project_status_admission(
        registry: &ResponseRegistry,
        gate_build_sha256: &str,
        runtime_build_sha256: &str,
    ) -> CompositeResponseAdmissionV2 {
        let package = &registry.packages[0];
        let verifier = package.verifier.as_ref().expect("external verifier");
        let proof_digest = "a".repeat(64);
        let mut binding = ResponsePackageAuthorityBindingV2 {
            package_id: package.package_id.clone(),
            registry_revision: registry.revision,
            package_sha256: response_package_digest(package).expect("package digest"),
            execution_payload_sha256: response_execution_payload_digest(package)
                .expect("execution payload digest"),
            actor_program_sha256: response_actor_program_digest(&package.program)
                .expect("actor digest"),
            independent_verifier_program_sha256: response_independent_verifier_program_digest(
                verifier,
            )
            .expect("verifier digest"),
            verifier_schema: package.proof.verifier_schema.clone(),
            support_manifest_schema: RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1.to_owned(),
            support_manifest_sha256: proof_digest.clone(),
            exact_causal_proof_schema: RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2.to_owned(),
            exact_causal_proof_sha256: proof_digest.clone(),
            runtime_parity_receipt_set_schema: RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1
                .to_owned(),
            runtime_parity_receipt_set_sha256: proof_digest.clone(),
            future_verifier_receipt_set_schema: RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2
                .to_owned(),
            future_verifier_receipt_set_sha256: proof_digest,
            semantic_alias_proof_schema: RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1.to_owned(),
            semantic_alias_proof_sha256: "b".repeat(64),
            proof_receipts_sha256: String::new(),
        };
        binding.proof_receipts_sha256 =
            response_proof_receipts_digest(&binding).expect("proof receipt digest");
        CompositeResponseAdmissionV2 {
            schema: COMPOSITE_ADMISSION_SCHEMA_V2.to_owned(),
            project_id: "nando-wave".to_owned(),
            generated_at_unix: unix_now(),
            expires_at_unix: unix_now().saturating_add(60),
            verdict: "PASS".to_owned(),
            eligible_for_local_accept: true,
            response_authority: ResponseAuthorityV2 {
                schema: RESPONSE_AUTHORITY_SCHEMA_V2.to_owned(),
                registry_schema: registry.schema.clone(),
                registry_revision: registry.revision,
                registry_sha256: response_registry_digest(registry).expect("registry digest"),
                gate_build_sha256: gate_build_sha256.to_owned(),
                runtime_build_sha256: runtime_build_sha256.to_owned(),
                packages: vec![binding],
            },
        }
    }

    fn project_status_test_state(root: &Path, response_registry_path: &Path) -> AppState {
        let mode_path = root.join("mode.json");
        let admission_path = root.join("admission.json");
        let trace_path = root.join("trace.jsonl");
        write_json(&mode_path, &json!({"mode":"CPU"}));
        let gate_build_path = root.join("nando-live-transition-gate");
        let runtime_build_path = root.join("nando-transition-serving");
        fs::write(&gate_build_path, b"test-gate-build").expect("gate build");
        fs::write(&runtime_build_path, b"test-runtime-build").expect("runtime build");
        let registry: ResponseRegistry =
            serde_json::from_slice(&fs::read(response_registry_path).expect("registry bytes"))
                .expect("registry");
        let admission = project_status_admission(
            &registry,
            &sha256_bytes(b"test-gate-build"),
            &response_runtime_contract_sha256(),
        );
        write_json(
            &admission_path,
            &serde_json::to_value(admission).expect("admission json"),
        );
        let config = ServingConfig {
            bind: "127.0.0.1:0".into(),
            registry_path: root.join("unused-transition-registry.json"),
            response_registry_path: response_registry_path.to_owned(),
            runtime_package_revocations_path: root.join("runtime-package-revocations.json"),
            admission_path,
            gate_build_path,
            runtime_build_path,
            project_id: "nando-wave".to_owned(),
            mode_path,
            metrics_path: root.join("metrics.json"),
            trace_path: trace_path.clone(),
            event_path: root.join("events.jsonl"),
            nginx_terminal_path: None,
            economics_path: root.join("economics.jsonl"),
            legacy_json_audit_enabled: true,
            kill_switch_path: root.join("KILL_SWITCH"),
            max_body_bytes: 1_048_576,
            admission_max_age_seconds: 60,
            refresh_interval_ms: 60_000,
            local_accept_enabled: true,
            client_allow_local_accept: true,
            route_ready: true,
            fallback_managed_by_nginx: true,
            expression_candidate_path: root.join("expression-candidate.json"),
            embedded_response_miner_enabled: false,
            generic_response_miner_enabled: false,
            response_relation_frames_path: root.join("response-relation-frames.jsonl"),
            response_online_report_path: root.join("response-online-miner-report.json"),
            response_online_checkpoint_path: root.join("response-online-miner.checkpoint"),
            codex_sessions_path: root.join("sessions"),
            streaming_evidence_path: root.join("streaming-evidence"),
            online_collection_checkpoint_path: root.join("online-collection-version-space.json"),
            response_admission_candidate_path: root.join("response-admission-candidates.cbor"),
            operator_generation_shadow_enabled: false,
            operator_generation_store_path: root.join("operator-generation-v3"),
            operator_generation_capture_index_path: root
                .join("operator-generation-capture-v3.cbor"),
            operator_generation_shadow_queue_capacity: 8,
            operator_generation_shadow_poll_ms: 1_000,
            learning_evidence_bridge_socket_path: root
                .join("learning-evidence-bridge-v1/bridge.sock"),
            learning_evidence_bridge_producer_enabled: false,
            learning_evidence_bridge_consumer_enabled: false,
            learning_evidence_bridge_queue_capacity: 8,
            learning_structure_bridge_root_path: root.join("learning-structure-bridge-v2"),
            learning_structure_bridge_producer_enabled: false,
            learning_structure_bridge_consumer_enabled: false,
            learning_structure_bridge_poll_ms: 100,
            provider_capture_enabled: false,
            provider_capture_store_path: root.join("provider-capture-v3-f8a"),
            provider_capture_queue_capacity: 8,
            operator_generation_shadow_receipt_store_path: root
                .join("operator-generation-shadow-v3-f8b"),
            opportunity_bridge_root_path: root.join("opportunity-bridge-v1"),
            opportunity_bridge_producer_enabled: false,
            opportunity_bridge_consumer_enabled: false,
            opportunity_bridge_poll_ms: 100,
            multi_source_snapshot_path: root.join("multi-source-live-v2/snapshot.cbor"),
            multi_source_snapshot_poll_ms: 1_000,
            terminal_receipt_archive_path: root
                .join("multi-source-live-v2/terminal-receipt-archive-v1"),
            multi_source_frame_archive_path: root
                .join("multi-source-live-v2/relation-frame-archive-v1"),
        };
        let provider_capture = Arc::new(
            ProviderCaptureRuntimeV3::new(provider_capture_config(&config))
                .expect("provider capture"),
        );
        let operator_generation_shadow = Arc::new(
            GenerationShadowRuntimeV3::new(generation_shadow_config(&config))
                .expect("generation shadow"),
        );
        let opportunity_bridge = OpportunityBridgeRuntime::new(
            config.opportunity_bridge_root_path.clone(),
            false,
            false,
            Duration::from_millis(100),
        )
        .expect("opportunity bridge");
        let learning_evidence_bridge = LearningEvidenceBridgeRuntimeV1::new(
            config.learning_evidence_bridge_socket_path.clone(),
            false,
            false,
            8,
        )
        .expect("learning evidence bridge");
        let (learning_structure_bridge, request_learning) = LearningStructureBridgeRuntimeV2::open(
            config.learning_structure_bridge_root_path.clone(),
            false,
            false,
            Duration::from_millis(100),
        )
        .expect("learning structure bridge");
        let state = AppState {
            config: Arc::new(config),
            cache: Arc::new(RwLock::new(ExecutorCache::default())),
            response_cache: Arc::new(RwLock::new(ResponseExecutorCache::default())),
            expression_shadow_cache: Arc::new(RwLock::new(ExpressionShadowCache::default())),
            observations: Arc::new(ObservationStore::load(trace_path).expect("trace store")),
            miners: Arc::new(RwLock::new(MinerSlots::default())),
            deterministic_evidence: Arc::new(RwLock::new(None)),
            miner_warmup: Arc::new(RwLock::new(MinerWarmupStatus {
                phase: "disabled".to_owned(),
                ..MinerWarmupStatus::default()
            })),
            session_stream_metrics: Arc::new(SessionStreamMetrics::default()),
            session_miner_bridge: Arc::new(SessionMinerBridge::new()),
            request_learning,
            runtime_policy: Arc::new(RuntimePolicyCache::load(
                root.join("mode.json"),
                root.join("kill-switch"),
            )),
            live_economics: spawn_economics_worker(root.join("economics"))
                .expect("economics worker"),
            authority_trigger: Arc::new(Mutex::new(None)),
            event_lock: Arc::new(Mutex::new(())),
            counters: Arc::new(ServingCounters::default()),
            provider_capture,
            operator_generation_shadow,
            learning_evidence_bridge,
            learning_structure_bridge,
            opportunity_bridge,
            multi_source_snapshot: Arc::new(RwLock::new(None)),
            terminal_receipt_archive: None,
            multi_source_frame_archive: None,
        };
        refresh_response_executor(&state);
        state
    }

    #[test]
    fn runtime_false_accept_persists_execution_identity_revocation() {
        let root = std::env::temp_dir().join(format!(
            "nando-serving-runtime-revocation-{}-{}",
            std::process::id(),
            PROJECT_STATUS_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test root");
        let registry_path = root.join("response-registry.json");
        write_json(
            &registry_path,
            &serde_json::to_value(project_status_registry()).expect("registry"),
        );
        let state = project_status_test_state(&root, &registry_path);
        assert_eq!(
            persist_runtime_package_revocation(&state, "serving-project-status", &"66".repeat(32),),
            Ok(true)
        );
        assert_eq!(
            persist_runtime_package_revocation(&state, "serving-project-status", &"77".repeat(32),),
            Ok(false)
        );
        let ledger: RuntimePackageRevocationLedgerV1 = serde_json::from_slice(
            &fs::read(&state.config.runtime_package_revocations_path).expect("revocation ledger"),
        )
        .expect("decode ledger");
        let package = &project_status_registry().packages[0];
        assert!(ledger.revokes(
            &package.package_id,
            &response_execution_payload_digest(package).expect("payload"),
        ));
        assert_eq!(ledger.revocations.len(), 1);
        let unresolved = runtime_package_revocation_health(&state.config);
        assert!(unresolved.valid);
        assert_eq!(unresolved.total, 1);
        assert_eq!(unresolved.unresolved_active, 1);

        let mut corrected_registry = project_status_registry();
        corrected_registry.packages.clear();
        write_json(
            &registry_path,
            &serde_json::to_value(corrected_registry).expect("corrected registry"),
        );
        let contained = runtime_package_revocation_health(&state.config);
        assert!(contained.valid);
        assert_eq!(contained.total, 1);
        assert_eq!(contained.unresolved_active, 0);
        fs::remove_dir_all(&root).expect("cleanup test root");
    }

    #[test]
    fn rejected_response_authority_is_cached_until_inputs_change() {
        let root = std::env::temp_dir().join(format!(
            "nando-serving-negative-authority-cache-{}-{}",
            std::process::id(),
            PROJECT_STATUS_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test root");
        let registry_path = root.join("response-registry.json");
        write_json(
            &registry_path,
            &serde_json::to_value(project_status_registry()).expect("registry"),
        );
        let state = project_status_test_state(&root, &registry_path);
        write_json(
            &state.config.admission_path,
            &json!({"schema": "invalid-admission-v1"}),
        );
        *state.response_cache.write().expect("response cache") = ResponseExecutorCache::default();

        refresh_response_executor(&state);
        let first_fingerprint = state
            .response_cache
            .read()
            .expect("response cache")
            .input_fingerprint
            .expect("negative fingerprint");
        refresh_response_executor(&state);
        let unchanged = state.response_cache.read().expect("response cache");
        assert_eq!(
            unchanged.input_fingerprint.as_ref(),
            Some(&first_fingerprint)
        );
        assert!(!unchanged.ready);
        assert!(!unchanged.last_error.is_empty());
        drop(unchanged);

        // Size must change as well: some filesystems coalesce rapid mtime updates.
        write_json(
            &state.config.admission_path,
            &json!({"schema": "invalid-admission-v2-expanded"}),
        );
        refresh_response_executor(&state);
        let changed = state.response_cache.read().expect("response cache");
        assert_ne!(changed.input_fingerprint.as_ref(), Some(&first_fingerprint));
        assert!(!changed.ready);
        fs::remove_dir_all(&root).expect("cleanup test root");
    }

    #[test]
    fn expired_hot_cache_guard_does_not_renew_authority() {
        let root = std::env::temp_dir().join(format!(
            "nando-serving-hot-authority-no-io-{}-{}",
            std::process::id(),
            PROJECT_STATUS_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test root");
        let registry_path = root.join("response-registry.json");
        write_json(
            &registry_path,
            &serde_json::to_value(project_status_registry()).expect("registry"),
        );
        let state = project_status_test_state(&root, &registry_path);
        let admission_before =
            fs::read(&state.config.admission_path).expect("admission before hot guard");
        {
            let mut cache = state.response_cache.write().expect("response cache");
            assert!(cache.ready);
            cache.admission_expires_at_unix = unix_now().saturating_sub(1);
        }

        assert!(!response_local_accept_enabled(&state));
        assert_eq!(
            fs::read(&state.config.admission_path).expect("admission after hot guard"),
            admission_before
        );
        let cache = state.response_cache.read().expect("response cache");
        assert!(cache.ready);
        assert!(cache.executor.is_some());
        assert!(cache.admission_expires_at_unix <= unix_now());
        assert!(cache.last_error.is_empty());
        fs::remove_dir_all(&root).expect("cleanup test root");
    }

    #[cfg(any())]
    #[test]
    fn embedded_miner_without_candidates_preserves_external_response_authority() {
        let root = std::env::temp_dir().join(format!(
            "nando-serving-external-authority-{}-{}",
            std::process::id(),
            PROJECT_STATUS_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test root");
        let registry_path = root.join("response-registry.json");
        write_json(
            &registry_path,
            &serde_json::to_value(project_status_registry()).expect("registry"),
        );
        let mut state = project_status_test_state(&root, &registry_path);
        Arc::get_mut(&mut state.config)
            .expect("unique config")
            .embedded_response_miner_enabled = true;
        write_json(
            &embedded_authority_marker_path(&state),
            &json!({
                "schema": "nando.embedded-response-authority-marker.v1",
                "revision": 7,
                "registry_sha256": "0".repeat(64),
                "execution_authority": false
            }),
        );
        write_json(
            &embedded_authority_candidate_path(&state),
            &json!({"schema": "nando.response-authority-candidate.v1"}),
        );
        *state.response_cache.write().expect("response cache") = ResponseExecutorCache::default();

        refresh_response_authority(&state);
        refresh_response_authority(&state);

        let cache = state.response_cache.read().expect("response cache");
        assert!(cache.ready);
        assert_eq!(
            cache
                .executor
                .as_ref()
                .map(|executor| executor.active_package_count()),
            Some(1)
        );
        assert!(cache.last_error.is_empty());
        drop(cache);
        assert!(registry_path.exists());
        assert!(state.config.admission_path.exists());
        assert!(!embedded_authority_marker_path(&state).exists());
        assert!(!embedded_authority_candidate_path(&state).exists());
        fs::remove_dir_all(&root).expect("cleanup test root");
    }

    #[cfg(any())]
    #[test]
    fn embedded_candidate_is_revoked_immediately_when_miner_loses_its_winner() {
        let root = std::env::temp_dir().join(format!(
            "nando-serving-embedded-revocation-{}-{}",
            std::process::id(),
            PROJECT_STATUS_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test root");
        let registry_path = root.join("response-registry.json");
        write_json(
            &registry_path,
            &serde_json::to_value(project_status_registry()).expect("registry"),
        );
        let mut state = project_status_test_state(&root, &registry_path);
        Arc::get_mut(&mut state.config)
            .expect("unique config")
            .embedded_response_miner_enabled = true;
        let collection = OnlineCollectionMiner::open(
            state.config.online_collection_checkpoint_path.clone(),
            OnlineCollectionConfig::default(),
        )
        .expect("empty collection miner");
        state.miners.write().expect("miner slots").collection =
            Some(Arc::new(Mutex::new(collection)));
        write_json(
            &embedded_authority_marker_path(&state),
            &json!({
                "schema": "nando.embedded-response-authority-marker.v1",
                "revision": 7,
                "execution_authority": false
            }),
        );
        write_json(
            &embedded_authority_candidate_path(&state),
            &json!({"schema": "nando.response-authority-candidate.v1"}),
        );
        {
            let mut cache = state.response_cache.write().expect("response cache");
            cache.embedded_candidate_revision = 7;
            cache.input_fingerprint = None;
            assert!(cache.ready);
            assert!(cache.executor.is_some());
        }

        refresh_response_authority(&state);

        let cache = state.response_cache.read().expect("response cache");
        assert!(!cache.ready);
        assert!(cache.executor.is_none());
        assert_eq!(cache.last_error, "online_no_admission_candidate");
        drop(cache);
        assert!(!registry_path.exists());
        assert!(!state.config.admission_path.exists());
        assert!(!embedded_authority_marker_path(&state).exists());
        assert!(!embedded_authority_candidate_path(&state).exists());
        assert!(root.join("response-authority-revocation.json").exists());
        fs::remove_dir_all(&root).expect("cleanup test root");
    }

    #[tokio::test]
    async fn failed_checkpoint_warmup_never_blocks_health_or_publishes_partial_miners() {
        let root = std::env::temp_dir().join(format!(
            "nando-serving-warmup-failure-{}-{}",
            std::process::id(),
            PROJECT_STATUS_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test root");
        let registry_path = root.join("response-registry.json");
        write_json(
            &registry_path,
            &serde_json::to_value(project_status_registry()).expect("registry"),
        );
        let mut state = project_status_test_state(&root, &registry_path);
        Arc::get_mut(&mut state.config)
            .expect("unique config")
            .embedded_response_miner_enabled = true;
        fs::write(
            &state.config.online_collection_checkpoint_path,
            b"invalid-checkpoint",
        )
        .expect("invalid checkpoint");
        if let Ok(mut status) = state.miner_warmup.write() {
            status.phase = "pending".to_owned();
        }

        let started = std::time::Instant::now();
        spawn_miner_warmup(state.clone()).expect("spawn warmup");
        assert!(started.elapsed() < Duration::from_millis(100));

        let response = health(State(state.clone())).await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("health body");
        let health: Value = serde_json::from_slice(&bytes).expect("health json");
        assert_eq!(health["ok"], true);
        assert_eq!(health["online_collection_miner"]["ready"], false);

        for _ in 0..100 {
            if state
                .miner_warmup
                .read()
                .is_ok_and(|status| status.phase == "failed")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let status = state.miner_warmup.read().expect("warmup status");
        assert_eq!(status.phase, "failed");
        assert!(!status.error.is_empty());
        drop(status);
        let slots = state.miners.read().expect("miner slots");
        assert!(slots.response.is_none());
        assert!(slots.collection.is_none());
        drop(slots);
        fs::remove_dir_all(&root).expect("cleanup test root");
    }

    #[tokio::test]
    async fn successful_warmup_atomically_publishes_complete_miner_slots() {
        let root = std::env::temp_dir().join(format!(
            "nando-serving-warmup-success-{}-{}",
            std::process::id(),
            PROJECT_STATUS_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test root");
        let registry_path = root.join("response-registry.json");
        write_json(
            &registry_path,
            &serde_json::to_value(project_status_registry()).expect("registry"),
        );
        let mut state = project_status_test_state(&root, &registry_path);
        let config = Arc::get_mut(&mut state.config).expect("unique config");
        config.embedded_response_miner_enabled = true;
        config.generic_response_miner_enabled = true;
        spawn_miner_warmup(state.clone()).expect("spawn warmup");
        for _ in 0..100 {
            if state
                .miner_warmup
                .read()
                .is_ok_and(|status| status.phase == "ready")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(state.miner_warmup.read().expect("status").phase, "ready");
        let slots = state.miners.read().expect("miner slots");
        assert!(slots.response.is_some());
        assert!(slots.collection.is_some());
        drop(slots);
        fs::remove_dir_all(&root).expect("cleanup test root");
    }

    #[test]
    fn stale_external_admission_is_revalidated_against_current_artifacts() {
        let root = std::env::temp_dir().join(format!(
            "nando-serving-revalidated-authority-{}-{}",
            std::process::id(),
            PROJECT_STATUS_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test root");
        let registry_path = root.join("response-registry.json");
        write_json(
            &registry_path,
            &serde_json::to_value(project_status_registry()).expect("registry"),
        );
        let state = project_status_test_state(&root, &registry_path);
        let mut admission = read_json(&state.config.admission_path).expect("admission");
        admission["generated_at_unix"] = Value::from(1_u64);
        admission["expires_at_unix"] = Value::from(2_u64);
        write_json(&state.config.admission_path, &admission);
        *state.response_cache.write().expect("response cache") = ResponseExecutorCache::default();

        refresh_response_executor(&state);

        let cache = state.response_cache.read().expect("response cache");
        assert!(cache.ready);
        assert!(cache.admission_expires_at_unix > unix_now());
        assert_eq!(
            cache
                .executor
                .as_ref()
                .map(|executor| executor.active_package_count()),
            Some(1)
        );
        drop(cache);
        let renewed = read_json(&state.config.admission_path).expect("renewed admission");
        assert!(renewed["generated_at_unix"].as_u64().unwrap_or(0) > 1);
        assert!(renewed["expires_at_unix"].as_u64().unwrap_or(0) > unix_now());
        fs::remove_dir_all(&root).expect("cleanup test root");
    }

    async fn response_text(response: Response) -> (StatusCode, HeaderMap, String) {
        let status = response.status();
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        (
            status,
            headers,
            String::from_utf8(body.to_vec()).expect("utf8 response"),
        )
    }

    async fn responses_call(state: &AppState, payload: &Value) -> (StatusCode, HeaderMap, String) {
        let response = openai_responses(
            State(state.clone()),
            HeaderMap::new(),
            Bytes::from(serde_json::to_vec(payload).expect("request bytes")),
        )
        .await;
        response_text(response).await
    }

    async fn chat_call(state: &AppState, payload: &Value) -> (StatusCode, HeaderMap, String) {
        let response = openai_chat(
            State(state.clone()),
            HeaderMap::new(),
            Bytes::from(serde_json::to_vec(payload).expect("request bytes")),
        )
        .await;
        response_text(response).await
    }

    fn sse_events(body: &str) -> Vec<(String, Value)> {
        let mut event = None;
        let mut parsed = Vec::new();
        for line in body.lines() {
            if let Some(name) = line.strip_prefix("event: ") {
                event = Some(name.to_owned());
            } else if let Some(data) = line.strip_prefix("data: ")
                && data != "[DONE]"
            {
                parsed.push((
                    event.take().expect("event before data"),
                    serde_json::from_str(data).expect("sse json"),
                ));
            }
        }
        parsed
    }

    fn chat_sse_chunks(body: &str) -> Vec<Value> {
        body.lines()
            .filter_map(|line| line.strip_prefix("data: "))
            .filter(|data| *data != "[DONE]")
            .map(|data| serde_json::from_str(data).expect("chat sse json"))
            .collect()
    }

    fn jsonl(path: &Path) -> Vec<Value> {
        fs::read_to_string(path)
            .expect("jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).expect("jsonl row"))
            .collect()
    }

    fn assert_receipt(row: &Value, schema: &str) {
        assert_eq!(row.get("verifier_schema"), Some(&json!(schema)));
        assert!(
            row.get("verification_receipt_id")
                .and_then(Value::as_str)
                .is_some_and(valid_sha256)
        );
        assert!(
            row.get("projector_receipt_id")
                .and_then(Value::as_str)
                .is_some_and(valid_sha256)
        );
    }

    #[tokio::test]
    async fn project_status_serving_integration_is_verified_and_fails_closed() {
        let root = env::temp_dir().join(format!(
            "nando-transition-serving-project-status-{}-{}-{}",
            std::process::id(),
            unix_now(),
            PROJECT_STATUS_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test root");
        let registry_path = root.join("response-registry.json");
        let registry = project_status_registry();
        assert_eq!(registry.schema, RESPONSE_REGISTRY_SCHEMA_V6);
        assert_eq!(registry.packages.len(), 1);
        assert_eq!(
            registry.packages[0].proof.verifier_schema,
            STATUS_PROJECTION_EXTERNAL_VERIFIER_SCHEMA
        );
        assert!(registry.packages[0].eligible_for_admission_candidate());
        write_json(
            &registry_path,
            &serde_json::to_value(&registry).expect("registry json"),
        );
        let loaded = ResponseExecutor::load(&registry_path).expect("active registry");
        assert_eq!(loaded.active_package_count(), 0);
        assert_eq!(loaded.diagnostic_package_count(), 1);

        let state = project_status_test_state(&root, &registry_path);
        {
            let response_cache = state.response_cache.read().expect("response cache");
            assert!(response_cache.ready, "{}", response_cache.last_error);
        }
        let success_payload = json!({
            "model":"project-status-test",
            "input":[{"type":"function_call_output","call_id":"status-0","output":"{\"exit_code\":0}"}]
        });
        let (status, _, body) = responses_call(&state, &success_payload).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let success: Value = serde_json::from_str(&body).expect("responses json");
        assert_eq!(success.get("status"), Some(&json!("completed")));
        assert_eq!(success.get("output_text"), Some(&json!("success")));
        assert_eq!(success.pointer("/output/0/role"), Some(&json!("assistant")));
        assert_eq!(
            success.pointer("/output/0/content/0/type"),
            Some(&json!("output_text"))
        );
        assert_eq!(
            success.pointer("/output/0/content/0/text"),
            Some(&json!("success"))
        );

        let failure_payload = json!({
            "model":"project-status-test",
            "input":[{"type":"function_call_output","call_id":"status-23","output":"{\"exit_code\":23}"}]
        });
        let (status, _, body) = responses_call(&state, &failure_payload).await;
        assert_eq!(status, StatusCode::OK);
        let failure: Value = serde_json::from_str(&body).expect("responses json");
        assert_eq!(failure.get("status"), Some(&json!("completed")));
        assert_eq!(failure.get("output_text"), Some(&json!("failure")));
        assert_eq!(
            failure.pointer("/output/0/content/0/text"),
            Some(&json!("failure"))
        );

        let stream_payload = json!({
            "model":"project-status-test",
            "stream":true,
            "input":[{"type":"function_call_output","call_id":"status-stream","output":"{\"exit_code\":0}"}]
        });
        let (status, headers, body) = responses_call(&state, &stream_payload).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            headers
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/event-stream")
        );
        let events = sse_events(&body);
        let names = events
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "response.created",
                "response.output_item.added",
                "response.content_part.added",
                "response.output_text.delta",
                "response.output_text.done",
                "response.content_part.done",
                "response.output_item.done",
                "response.completed",
            ]
        );
        for (sequence, (_, payload)) in events.iter().enumerate() {
            assert_eq!(payload.get("sequence_number"), Some(&json!(sequence)));
        }
        assert_eq!(events[3].1.get("delta"), Some(&json!("success")));
        assert_eq!(events[4].1.get("text"), Some(&json!("success")));
        assert_eq!(
            events[7].1.pointer("/response/output_text"),
            Some(&json!("success"))
        );
        assert!(body.ends_with("data: [DONE]\n\n"));

        let chat_payload = json!({
            "model":"project-status-chat-test",
            "messages":[
                {"role":"user","content":"Check the project status"},
                {
                    "role":"assistant",
                    "content":Value::Null,
                    "tool_calls":[{
                        "id":"call-status-0",
                        "type":"function",
                        "function":{"name":"exec","arguments":"{\"cmd\":\"status\"}"}
                    }]
                },
                {
                    "role":"tool",
                    "tool_call_id":"call-status-0",
                    "content":"{\"exit_code\":0}"
                }
            ]
        });
        let (status, _, body) = chat_call(&state, &chat_payload).await;
        assert_eq!(status, StatusCode::OK);
        let chat: Value = serde_json::from_str(&body).expect("chat json");
        assert_eq!(chat.get("object"), Some(&json!("chat.completion")));
        assert_eq!(chat.get("model"), Some(&json!("project-status-chat-test")));
        assert_eq!(
            chat.pointer("/choices/0/message/role"),
            Some(&json!("assistant"))
        );
        assert_eq!(
            chat.pointer("/choices/0/message/content"),
            Some(&json!("success"))
        );
        assert_eq!(
            chat.pointer("/choices/0/finish_reason"),
            Some(&json!("stop"))
        );

        let mut chat_stream_payload = chat_payload.clone();
        chat_stream_payload["stream"] = Value::Bool(true);
        let (status, headers, body) = chat_call(&state, &chat_stream_payload).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            headers
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/event-stream")
        );
        let chunks = chat_sse_chunks(&body);
        assert_eq!(chunks.len(), 3);
        assert_eq!(
            chunks[0].pointer("/choices/0/delta/role"),
            Some(&json!("assistant"))
        );
        assert_eq!(
            chunks[1].pointer("/choices/0/delta/content"),
            Some(&json!("success"))
        );
        assert_eq!(
            chunks[2].pointer("/choices/0/finish_reason"),
            Some(&json!("stop"))
        );
        assert!(body.ends_with("data: [DONE]\n\n"));

        let accepted_events = jsonl(&state.config.event_path)
            .into_iter()
            .filter(|row| row.get("event") == Some(&json!("local_accept")))
            .collect::<Vec<_>>();
        let accepted_economics = jsonl(&state.config.economics_path);
        assert_eq!(accepted_events.len(), 5);
        assert_eq!(accepted_economics.len(), 5);
        for row in &accepted_events {
            assert_receipt(row, STATUS_PROJECTION_EXTERNAL_VERIFIER_SCHEMA);
            assert_eq!(
                row.pointer("/post_verifier_receipt/schema"),
                Some(&json!("nando.response-post-verifier-receipt.v1"))
            );
            assert_eq!(
                row.pointer("/runtime_verification_receipt/schema"),
                Some(&json!("nando.response-runtime-verification-receipt.v2"))
            );
            assert_eq!(
                row.get("package_id"),
                Some(&json!("serving-project-status"))
            );
        }
        for row in &accepted_economics {
            assert_receipt(row, STATUS_PROJECTION_EXTERNAL_VERIFIER_SCHEMA);
            assert_eq!(row.get("upstream_socket_opened"), Some(&Value::Bool(false)));
            assert_eq!(row.get("avoided_call"), Some(&Value::Bool(true)));
        }
        let persisted_acceptance = format!(
            "{}\n{}",
            fs::read_to_string(&state.config.event_path).expect("event text"),
            fs::read_to_string(&state.config.economics_path).expect("economics text")
        );
        assert!(!persisted_acceptance.contains("Check the project status"));
        assert!(!persisted_acceptance.contains("exit_code"));

        let invalid_payloads = [
            json!({
                "input":[{"type":"function_call_output","output":[
                    {"type":"input_text","text":"{\"exit_code\":0}"},
                    {"type":"input_text","text":"{\"exit_code\":1}"}
                ]}]
            }),
            json!({"input":[{"type":"function_call_output","output":"{\"other\":0}"}]}),
            json!({"input":[
                {"type":"function_call_output","output":"{\"exit_code\":0}"},
                {"type":"message","role":"user","content":"new turn"}
            ]}),
            json!({"input":[{"type":"function_call_output","output":"{\"exit_code\":true}"}]}),
            json!({"input":[{"type":"function_call_output","output":"{\"exit_code\":1000001}"}]}),
        ];
        for payload in invalid_payloads {
            let (status, headers, body) = responses_call(&state, &payload).await;
            assert_eq!(status, StatusCode::IM_A_TEAPOT);
            assert_eq!(
                headers
                    .get("x-nando-fallback")
                    .and_then(|value| value.to_str().ok()),
                Some("upstream_required")
            );
            let fallback: Value = serde_json::from_str(&body).expect("fallback json");
            assert_eq!(fallback.get("local_accept"), Some(&Value::Bool(false)));
            assert_eq!(fallback.get("fallback_required"), Some(&Value::Bool(true)));
        }
        let invalid_chat_payloads = [
            success_payload.clone(),
            json!({
                "messages":[
                    {"role":"user","content":"Check status"},
                    {
                        "role":"assistant",
                        "content":Value::Null,
                        "tool_calls":[{
                            "id":"call-unknown-part",
                            "type":"function",
                            "function":{"name":"exec","arguments":"{}"}
                        }]
                    },
                    {
                        "role":"tool",
                        "tool_call_id":"call-unknown-part",
                        "content":[{"type":"future_text","text":"{\"exit_code\":0}"}]
                    }
                ]
            }),
            json!({
                "messages":[
                    {"role":"user","content":"Check status"},
                    {
                        "role":"tool",
                        "tool_call_id":"missing-call",
                        "content":"{\"exit_code\":0}"
                    }
                ]
            }),
        ];
        for payload in invalid_chat_payloads {
            let (status, headers, body) = chat_call(&state, &payload).await;
            assert_eq!(status, StatusCode::IM_A_TEAPOT);
            assert_eq!(
                headers
                    .get("x-nando-fallback")
                    .and_then(|value| value.to_str().ok()),
                Some("upstream_required")
            );
            let fallback: Value = serde_json::from_str(&body).expect("fallback json");
            assert_eq!(fallback.get("local_accept"), Some(&Value::Bool(false)));
            assert_eq!(fallback.get("fallback_required"), Some(&Value::Bool(true)));
        }
        assert_eq!(state.counters.local_accepts.load(Ordering::Relaxed), 5);
        assert_eq!(
            state
                .counters
                .ordinary_response_local_accepts
                .load(Ordering::Relaxed),
            5
        );
        let accepted_input_tokens = state
            .counters
            .ordinary_response_local_accept_input_tokens
            .load(Ordering::Relaxed);
        assert!(accepted_input_tokens > 0);
        {
            let package_counters = state
                .counters
                .response_cpu_by_package
                .lock()
                .expect("package cpu counters");
            let status_counters = package_counters
                .get("serving-project-status")
                .expect("status package counters");
            assert_eq!(status_counters.accepts, 5);
            assert_eq!(status_counters.ordinary_accepts, 5);
            assert_eq!(status_counters.ordinary_input_tokens, accepted_input_tokens);
        }
        assert_eq!(jsonl(&state.config.economics_path).len(), 5);

        let mut admission = read_json(&state.config.admission_path).expect("admission");
        admission["response_authority"]["registry_sha256"] = Value::String("f".repeat(64));
        write_json(&state.config.admission_path, &admission);
        refresh_response_executor(&state);
        {
            let cache = state.response_cache.read().expect("response cache");
            assert!(!cache.ready);
            assert_eq!(
                cache.last_error,
                "response_authority_registry_digest_mismatch"
            );
        }
        let (status, _, _) = responses_call(&state, &success_payload).await;
        assert_eq!(status, StatusCode::IM_A_TEAPOT);
        assert_eq!(state.counters.local_accepts.load(Ordering::Relaxed), 5);
        assert_eq!(
            state
                .counters
                .ordinary_response_local_accept_input_tokens
                .load(Ordering::Relaxed),
            accepted_input_tokens
        );

        fs::remove_dir_all(&root).expect("cleanup test root");
    }

    #[test]
    fn envelope_requires_explicit_metadata() {
        assert!(transition_envelope(&json!({"input":"hello"})).is_none());
        let envelope = transition_envelope(&json!({
            "metadata":{"nando_transition":{"before":{"count":1},"action":{"kind":"inc"}}}
        }));
        assert_eq!(envelope, Some((json!({"count":1}), json!({"kind":"inc"}))));
    }

    #[test]
    fn response_stream_has_terminal_event() {
        let response = responses_projection("abcdef", "OK", "model", "route", 1);
        let stream = responses_sse(&response);
        assert!(stream.contains("event: response.completed"));
        assert!(stream.ends_with("data: [DONE]\n\n"));
    }

    #[test]
    fn verified_scalar_projection_is_a_real_assistant_output_text() {
        let response = responses_projection("abcdef", "42", "model", "value-route", 10);
        assert_eq!(response.pointer("/output/0/type"), Some(&json!("message")));
        assert_eq!(
            response.pointer("/output/0/content/0/type"),
            Some(&json!("output_text"))
        );
        assert_eq!(
            response.pointer("/output/0/content/0/text"),
            Some(&json!("42"))
        );
        assert_eq!(response.get("output_text"), Some(&json!("42")));
        let stream = responses_sse(&response);
        assert!(stream.contains("response.output_text.delta"));
        assert!(stream.contains("response.output_text.done"));
        assert!(stream.ends_with("data: [DONE]\n\n"));
    }

    #[test]
    fn function_call_projection_is_structured_and_streamable() {
        let call = json!({
            "name":"wait",
            "arguments":{"cell_id":"859","yield_time_ms":1000,"max_tokens":5000}
        });
        let response = function_call_responses_projection("abcdef", &call, "model", "route", 10);
        assert_eq!(
            response.pointer("/output/0/type"),
            Some(&json!("function_call"))
        );
        assert_eq!(response.pointer("/output/0/name"), Some(&json!("wait")));
        let stream = responses_sse(&response);
        assert!(stream.contains("response.function_call_arguments.done"));
        assert!(stream.contains("\\\"cell_id\\\":\\\"859\\\""));
        assert!(stream.ends_with("data: [DONE]\n\n"));

        let chat = function_call_chat_projection("abcdef", &call, "model", "route", 10);
        assert_eq!(
            chat.pointer("/choices/0/message/tool_calls/0/function/name"),
            Some(&json!("wait"))
        );
        let chunks = chat_sse_chunks(&chat_sse(&chat));
        assert_eq!(chunks.len(), 4);
        assert_eq!(
            chunks[1].pointer("/choices/0/delta/tool_calls/0/function/name"),
            Some(&json!("wait"))
        );
        assert_eq!(
            chunks[2].pointer("/choices/0/delta/tool_calls/0/function/arguments"),
            Some(&json!(
                "{\"cell_id\":\"859\",\"max_tokens\":5000,\"yield_time_ms\":1000}"
            ))
        );
        assert_eq!(
            chunks[3].pointer("/choices/0/finish_reason"),
            Some(&json!("tool_calls"))
        );
    }

    #[test]
    fn verifier_receipt_validation_rejects_missing_authority() {
        let response = LiveTransitionResponse::decline("no_profile", 1);
        assert_eq!(validate_execution(&response), Err("typed_local_declined"));
    }

    #[test]
    fn diagnostic_execution_is_not_booked_as_llm_savings() {
        assert!(!Projection::TransitionApi.avoids_upstream_llm_call());
        assert!(Projection::Responses.avoids_upstream_llm_call());
        assert!(Projection::ChatCompletions.avoids_upstream_llm_call());
    }

    #[test]
    fn controlled_and_dogfood_sources_are_not_commercial_denominator() {
        assert!(!traffic_source_dedupe_eligible("controlled_probe"));
        assert!(!traffic_source_dedupe_eligible("dogfood_live_cell"));
        assert!(traffic_source_dedupe_eligible("ordinary"));
        assert!(traffic_source_dedupe_eligible("codex"));
        assert!(traffic_source_dedupe_eligible("unspecified"));
    }

    #[test]
    fn controlled_sources_are_not_natural_learning_evidence() {
        for source in [
            "controlled_probe",
            "dogfood_live_cell",
            "smoke",
            "fixture",
            "audit",
        ] {
            assert!(
                !traffic_source_natural_evidence_eligible(source),
                "{source}"
            );
        }
        for source in ["ordinary", "codex", "unspecified"] {
            assert!(traffic_source_natural_evidence_eligible(source), "{source}");
        }
    }

    #[test]
    fn traffic_source_header_precedes_payload_metadata() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-nando-traffic-source",
            "controlled_header".parse().expect("header value"),
        );
        let payload = json!({
            "metadata": {"nando_traffic_source": "ordinary"}
        });
        assert_eq!(
            request_traffic_source(&headers, &payload),
            "controlled_header"
        );
        assert_eq!(
            request_traffic_source(&HeaderMap::new(), &payload),
            "ordinary"
        );
    }

    #[tokio::test]
    async fn controlled_request_does_not_enter_provider_capture() {
        let root = std::env::temp_dir().join(format!(
            "nando-serving-controlled-evidence-{}-{}",
            std::process::id(),
            PROJECT_STATUS_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test root");
        let registry_path = root.join("response-registry.json");
        write_json(
            &registry_path,
            &serde_json::to_value(project_status_registry()).expect("registry"),
        );
        let mut state = project_status_test_state(&root, &registry_path);
        state.opportunity_bridge = OpportunityBridgeRuntime::new(
            root.join("controlled-opportunity-bridge"),
            true,
            false,
            Duration::from_millis(100),
        )
        .expect("opportunity bridge");
        let payload = json!({
            "model": "test",
            "input": [{
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "observe boundary"}]
            }]
        });
        let body = Bytes::from(serde_json::to_vec(&payload).expect("request bytes"));
        let mut controlled_headers = HeaderMap::new();
        controlled_headers.insert(
            "x-nando-traffic-source",
            "controlled_probe".parse().expect("header value"),
        );

        let _ = handle_openai(
            state.clone(),
            controlled_headers,
            body.clone(),
            Projection::Responses,
        );
        assert_eq!(state.provider_capture.status().submitted, 0);
        assert_eq!(state.opportunity_bridge.status().producer.events, 0);

        let _ = handle_openai(state.clone(), HeaderMap::new(), body, Projection::Responses);
        assert_eq!(state.provider_capture.status().submitted, 1);
        let opportunity = state.opportunity_bridge.status();
        assert_eq!(opportunity.producer.request_events, 1);
        assert!(opportunity.producer.events >= 1);
        fs::remove_dir_all(&root).expect("cleanup test root");
    }

    #[test]
    fn request_text_skips_non_message_items() {
        let payload = json!({
            "input": [
                {"type":"function_call_output","call_id":"call-1","output":"done"},
                {"type":"message","role":"user","content":[{"type":"input_text","text":"ground me"}]}
            ]
        });
        assert_eq!(extract_request_text(&payload), "ground me");
    }

    #[test]
    fn provider_shape_is_bounded_and_stores_no_raw_text() {
        let payload = json!({
            "model":"test",
            "instructions":"private instructions",
            "input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"private prompt"}]}],
            "tools":[{"type":"function","name":"shell"}],
            "metadata":{"traffic":"live"},
            "client_metadata":{
                "session_id":"private-session",
                "thread_id":"private-thread",
                "turn_id":"private-turn",
                "x-codex-window-id":"private-window",
                "x-codex-installation-id":"private-installation"
            }
        });
        let capability_atom_ids = provider_tool_capability_atom_ids(&payload);
        let shape = provider_request_shape(
            &payload,
            Projection::Responses,
            "private prompt",
            &capability_atom_ids,
        );
        assert_eq!(shape.get("raw_text_stored"), Some(&Value::Bool(false)));
        assert_eq!(shape.get("request_text_bytes"), Some(&Value::from(14)));
        assert_eq!(
            shape
                .get("request_text_sha256")
                .and_then(Value::as_str)
                .map(str::len),
            Some(64)
        );
        assert!(!shape.to_string().contains("private prompt"));
        assert!(!shape.to_string().contains("private instructions"));
        assert!(!shape.to_string().contains("private-session"));
        assert!(!shape.to_string().contains("private-installation"));
        assert_eq!(
            shape
                .get("client_identity_sha256")
                .and_then(|identity| identity.get("session_id"))
                .and_then(Value::as_str)
                .map(str::len),
            Some(64)
        );
        assert!(
            shape
                .get("client_identity_sha256")
                .and_then(|identity| identity.get("x-codex-installation-id"))
                .is_none()
        );
    }

    #[test]
    fn response_actor_fallback_stage_is_exact() {
        assert_eq!(
            response_actor_fallback_stage("no_phase_routed_profile"),
            "router"
        );
        assert_eq!(
            response_actor_fallback_stage("phase_routed_actor_abstain:selector_missing"),
            "actor"
        );
        assert_eq!(
            response_actor_fallback_stage("independent_verifier_failed:output_mismatch"),
            "verifier"
        );
        assert_eq!(
            response_actor_fallback_stage("execution_authority_missing"),
            "admission"
        );
    }

    #[test]
    fn expression_candidate_runs_only_in_shadow_counters() {
        let root = std::env::temp_dir().join(format!(
            "nando-expression-serving-shadow-{}-{}",
            std::process::id(),
            PROJECT_STATUS_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test root");
        let response_registry = root.join("response-registry.json");
        write_json(
            &response_registry,
            &serde_json::to_value(project_status_registry()).expect("registry"),
        );
        let state = project_status_test_state(&root, &response_registry);

        let package = minimal_action_role_package();
        let package_path = root.join("package.rsef");
        fs::write(&package_path, &package).expect("package");
        write_json(
            &state.config.expression_candidate_path,
            &json!({
                "schema":"nando.expression-quarantine-candidate.v1",
                "state":"quarantine",
                "execution_authority":false,
                "package":{"path":package_path,"sha256":sha256_bytes(&package)}
            }),
        );
        let mut admission = read_json(&state.config.admission_path).expect("admission");
        admission["sections"]["expression_shadow"] = json!({
            "verdict":"PASS",
            "required":false,
            "reason":"not_required_by_project_profile"
        });
        write_json(&state.config.admission_path, &admission);
        refresh_expression_shadow(&state);
        assert!(expression_shadow_cache_status(&state).0);

        let before = json!({"value":1});
        let action = json!({"next":2});
        let after = json!({"value":2});
        record_expression_shadow_request(&state, &before, &action, 7);
        record_expression_shadow_observation(&state, &before, &action, &after);
        assert_eq!(
            state
                .counters
                .expression_shadow_would_execute
                .load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            state
                .counters
                .expression_shadow_verified_matches
                .load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            state
                .counters
                .expression_shadow_wrong
                .load(Ordering::Relaxed),
            0
        );
        assert_eq!(
            state
                .counters
                .expression_shadow_potential_input_tokens
                .load(Ordering::Relaxed),
            7
        );
        assert_eq!(state.counters.local_accepts.load(Ordering::Relaxed), 0);
        fs::remove_dir_all(root).expect("remove test root");
    }

    fn minimal_action_role_package() -> Vec<u8> {
        let mut bytes = b"RSEF0001".to_vec();
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&[2, 2, 1, 0, 0]);
        bytes.push(2);
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u64.to_le_bytes());
        bytes
    }
}
