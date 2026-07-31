use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nando_client_evidence::{
    DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES, NandoRouteReceiptIndex, NandoRouteReceiptV1,
};
use nando_operator_kernel::{RelationFrame, sha256_bytes};
use nando_operator_learning::{FramedCborLedger, RuntimeParityCase, read_framed_cbor};
use nando_transition_serving::remote_evidence_spool::{
    REMOTE_EVIDENCE_ENDPOINT_V1, REMOTE_EVIDENCE_MAX_FRAMES_V1, RemoteEvidenceAckV1,
    RemoteEvidenceBatchV1, RemoteEvidenceFrameV1, RouteBoundEvidenceFrameV1,
    parse_remote_evidence_key_v1, remote_evidence_genesis_root, sign_remote_evidence_request_v1,
};
use nando_transition_serving::{
    SessionStreamMetrics, VerifiedRelationFrameSink, spawn_verified_relation_frame_stream,
};
use serde::{Deserialize, Serialize};

const AGENT_STATE_SCHEMA_V1: &str = "nando.local-evidence-agent-state.v1";
const AGENT_PENDING_SCHEMA_V2: &str = "nando.local-evidence-agent-pending.v2";
const ROUTE_BOUND_OUTBOX_SCHEMA_V1: &str = "nando.route-bound-evidence-outbox-frame.v1";
const OUTBOX_PREFIX: &str = "route-bound-relation-frame";
const STATE_FILE: &str = "state-v1.cbor";
const PENDING_FILE: &str = "pending-v2.cbor";
const MAX_OUTBOX_FRAMES: usize = 65_536;
const MAX_OUTBOX_BYTES: u64 = 512 * 1024 * 1024;
const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;
const MAX_STATE_BYTES: usize = 8 * 1024 * 1024;
const MAX_HTTP_RESPONSE_BYTES: u64 = 64 * 1024;
const DEFAULT_POLL_MILLIS: u64 = 1_000;
const DEFAULT_REMOTE_ORIGIN: &str = "http://192.168.3.94:8787";

#[derive(Clone, Debug)]
struct AgentConfig {
    sessions_directory: PathBuf,
    remote_origin: String,
    key_file: PathBuf,
    state_directory: PathBuf,
    route_receipts_path: PathBuf,
    poll_interval: Duration,
    once: bool,
    check_only: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct EvidenceAgentStateV1 {
    schema: String,
    client_id_sha256: String,
    opened_at_unix_nanos: u64,
    next_sequence: u64,
    previous_batch_root_sha256: String,
    seen_frame_roots: BTreeSet<String>,
    accepted_batches: u64,
    accepted_frames: u64,
    last_accepted_at_unix: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct EvidenceAgentPendingV1 {
    schema: String,
    client_id_sha256: String,
    batch: RemoteEvidenceBatchV1,
    frame_roots: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RouteBoundOutboxFrameV1 {
    schema: String,
    route_receipt_root_sha256: String,
    route_receipt: NandoRouteReceiptV1,
    frame: RelationFrame,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime_parity_case: Option<RuntimeParityCase>,
}

#[derive(Clone, Debug)]
struct HttpEndpoint {
    socket_host: String,
    port: u16,
    host_header: String,
    path: String,
}

struct LocalEvidenceOutbox {
    ledger: FramedCborLedger,
    frames: BTreeMap<String, RouteBoundOutboxFrameV1>,
    payload_bytes: u64,
}

struct OutboxSink {
    outbox: Arc<Mutex<LocalEvidenceOutbox>>,
    route_receipts: Arc<Mutex<NandoRouteReceiptIndex>>,
    route_metrics: Arc<RouteBindingMetrics>,
}

struct EvidenceAgent {
    config: AgentConfig,
    endpoint: HttpEndpoint,
    key: Vec<u8>,
    state: EvidenceAgentStateV1,
    outbox: Arc<Mutex<LocalEvidenceOutbox>>,
    stream_metrics: Arc<SessionStreamMetrics>,
    route_receipts: Arc<Mutex<NandoRouteReceiptIndex>>,
    route_metrics: Arc<RouteBindingMetrics>,
}

#[derive(Default)]
struct RouteBindingMetrics {
    route_bound_frames: AtomicU64,
    route_unbound_frames: AtomicU64,
    route_receipt_refresh_failures: AtomicU64,
}

#[derive(Clone, Copy, Debug, Default)]
struct AgentPollReport {
    accepted_frames: u64,
}

fn main() -> Result<(), String> {
    let config = AgentConfig::from_arguments()?;
    let mut agent = EvidenceAgent::open(config)?;
    if agent.config.check_only {
        println!("{}", agent.status_json(0)?);
        return Ok(());
    }
    if agent.config.once {
        let report = agent.run_once()?;
        println!("{}", agent.status_json(report.accepted_frames)?);
        return Ok(());
    }
    println!("{}", agent.status_json(0)?);
    let mut consecutive_failures = 0_u32;
    loop {
        let sleep_for = match agent.run_once() {
            Ok(report) if report.accepted_frames > 0 => {
                consecutive_failures = 0;
                println!("{}", agent.status_json(report.accepted_frames)?);
                agent.config.poll_interval
            }
            Ok(_) => {
                consecutive_failures = 0;
                agent.config.poll_interval
            }
            Err(error) => {
                if error == "evidence_agent_session_watcher_stopped" {
                    return Err(error);
                }
                consecutive_failures = consecutive_failures.saturating_add(1);
                if consecutive_failures == 1 || consecutive_failures.is_power_of_two() {
                    eprintln!("nando-evidence-agent: {error}; retry={consecutive_failures}");
                }
                retry_backoff(agent.config.poll_interval, consecutive_failures)
            }
        };
        thread::sleep(sleep_for);
    }
}

impl AgentConfig {
    fn from_arguments() -> Result<Self, String> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is not set".to_owned())?;
        let mut config = Self {
            sessions_directory: std::env::var_os("NANDO_EVIDENCE_SESSIONS_DIR")
                .map_or_else(|| home.join(".codex/sessions"), PathBuf::from),
            remote_origin: std::env::var("NANDO_EVIDENCE_REMOTE_ORIGIN")
                .unwrap_or_else(|_| DEFAULT_REMOTE_ORIGIN.to_owned()),
            key_file: std::env::var_os("NANDO_EVIDENCE_KEY_FILE").map_or_else(
                || home.join(".config/nando/evidence-agent.key"),
                PathBuf::from,
            ),
            state_directory: std::env::var_os("NANDO_EVIDENCE_STATE_DIR").map_or_else(
                || home.join(".local/state/nando-evidence-agent"),
                PathBuf::from,
            ),
            route_receipts_path: std::env::var_os("NANDO_EVIDENCE_ROUTE_RECEIPTS")
                .map_or_else(default_route_receipts_path, PathBuf::from),
            poll_interval: Duration::from_millis(DEFAULT_POLL_MILLIS),
            once: false,
            check_only: false,
        };
        let mut arguments = std::env::args_os().skip(1);
        while let Some(argument) = arguments.next() {
            match argument.to_str() {
                Some("--sessions-dir") => {
                    config.sessions_directory =
                        arguments.next().map(PathBuf::from).ok_or_else(usage)?;
                }
                Some("--server") => {
                    config.remote_origin = arguments
                        .next()
                        .and_then(|value| value.into_string().ok())
                        .ok_or_else(usage)?;
                }
                Some("--key-file") => {
                    config.key_file = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
                }
                Some("--state-dir") => {
                    config.state_directory =
                        arguments.next().map(PathBuf::from).ok_or_else(usage)?;
                }
                Some("--route-receipts") => {
                    config.route_receipts_path =
                        arguments.next().map(PathBuf::from).ok_or_else(usage)?;
                }
                Some("--poll-ms") => {
                    let millis = arguments
                        .next()
                        .and_then(|value| value.to_string_lossy().parse::<u64>().ok())
                        .filter(|value| (100..=60_000).contains(value))
                        .ok_or_else(usage)?;
                    config.poll_interval = Duration::from_millis(millis);
                }
                Some("--once") => config.once = true,
                Some("--check") => config.check_only = true,
                Some("-h" | "--help") => {
                    println!("{}", usage());
                    std::process::exit(0);
                }
                _ => return Err(usage()),
            }
        }
        Ok(config)
    }
}

impl EvidenceAgent {
    fn open(config: AgentConfig) -> Result<Self, String> {
        ensure_private_directory(&config.state_directory)?;
        if !config.sessions_directory.is_dir() {
            return Err("evidence_agent_sessions_directory_missing".to_owned());
        }
        let endpoint = HttpEndpoint::parse(&config.remote_origin)?;
        let key = read_private_key(&config.key_file)?;
        let client_id_sha256 = sha256_bytes(&key);
        let state_path = config.state_directory.join(STATE_FILE);
        let state = match read_optional_bounded(&state_path, MAX_STATE_BYTES)? {
            Some(bytes) => {
                let state: EvidenceAgentStateV1 = serde_cbor::from_slice(&bytes)
                    .map_err(|error| format!("evidence_agent_state_decode:{error}"))?;
                if !state.validate(&client_id_sha256)
                    || serde_cbor::to_vec(&state)
                        .map_err(|error| format!("evidence_agent_state_encode:{error}"))?
                        != bytes
                {
                    return Err("evidence_agent_state_invalid".to_owned());
                }
                state
            }
            None => {
                let state = EvidenceAgentStateV1 {
                    schema: AGENT_STATE_SCHEMA_V1.to_owned(),
                    client_id_sha256: client_id_sha256.clone(),
                    opened_at_unix_nanos: unix_now_nanos()?,
                    next_sequence: 1,
                    previous_batch_root_sha256: remote_evidence_genesis_root(&client_id_sha256),
                    seen_frame_roots: BTreeSet::new(),
                    accepted_batches: 0,
                    accepted_frames: 0,
                    last_accepted_at_unix: 0,
                };
                write_canonical_state(&state_path, &state)?;
                state
            }
        };
        let outbox = Arc::new(Mutex::new(LocalEvidenceOutbox::open(
            &config.state_directory.join("outbox-v2"),
        )?));
        let route_receipts = Arc::new(Mutex::new(NandoRouteReceiptIndex::open(
            &config.route_receipts_path,
            DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES,
        )?));
        let route_metrics = Arc::new(RouteBindingMetrics::default());
        let stream_metrics = Arc::new(SessionStreamMetrics::default());
        spawn_verified_relation_frame_stream(
            config.sessions_directory.clone(),
            config.state_directory.join("session-capture-evidence-v1"),
            Arc::new(OutboxSink {
                outbox: Arc::clone(&outbox),
                route_receipts: Arc::clone(&route_receipts),
                route_metrics: Arc::clone(&route_metrics),
            }),
            Arc::clone(&stream_metrics),
        )?;
        wait_for_session_watcher(&stream_metrics)?;
        Ok(Self {
            config,
            endpoint,
            key,
            state,
            outbox,
            stream_metrics,
            route_receipts,
            route_metrics,
        })
    }

    fn run_once(&mut self) -> Result<AgentPollReport, String> {
        if !self.stream_metrics.snapshot().5 {
            return Err("evidence_agent_session_watcher_stopped".to_owned());
        }
        self.compact_acknowledged_outbox()?;
        let pending_path = self.config.state_directory.join(PENDING_FILE);
        let pending = match read_optional_bounded(&pending_path, MAX_STATE_BYTES)? {
            Some(bytes) => {
                let pending: EvidenceAgentPendingV1 = serde_cbor::from_slice(&bytes)
                    .map_err(|error| format!("evidence_agent_pending_decode:{error}"))?;
                if !pending.validate(&self.state)
                    || serde_cbor::to_vec(&pending)
                        .map_err(|error| format!("evidence_agent_pending_encode:{error}"))?
                        != bytes
                {
                    return Err("evidence_agent_pending_invalid".to_owned());
                }
                Some(pending)
            }
            None => self.build_pending()?,
        };
        let Some(pending) = pending else {
            return Ok(AgentPollReport::default());
        };
        let body = pending.batch.canonical_bytes()?;
        let timestamp_unix = unix_now_seconds()?;
        let signature =
            sign_remote_evidence_request_v1(&self.key, timestamp_unix, &pending.batch, &body)?;
        let ack = self.endpoint.post_batch(
            &self.state.client_id_sha256,
            timestamp_unix,
            &signature,
            &body,
        )?;
        if !ack.verify(&self.key)
            || !ack.ok
            || ack.client_id_sha256 != self.state.client_id_sha256
            || ack.sequence != pending.batch.sequence
            || ack.batch_root_sha256 != pending.batch.batch_root_sha256
            || ack.accepted_frames != u64::try_from(pending.batch.frames.len()).unwrap_or(u64::MAX)
        {
            return Err("evidence_agent_ack_binding_invalid".to_owned());
        }
        self.state.next_sequence = self.state.next_sequence.saturating_add(1);
        self.state.previous_batch_root_sha256 = pending.batch.batch_root_sha256;
        self.state
            .seen_frame_roots
            .extend(pending.frame_roots.iter().cloned());
        if self.state.seen_frame_roots.len() > MAX_OUTBOX_FRAMES {
            return Err("evidence_agent_seen_frame_budget".to_owned());
        }
        self.state.accepted_batches = self.state.accepted_batches.saturating_add(1);
        self.state.accepted_frames = self
            .state
            .accepted_frames
            .saturating_add(ack.accepted_frames);
        self.state.last_accepted_at_unix = timestamp_unix;
        write_canonical_state(&self.config.state_directory.join(STATE_FILE), &self.state)?;
        fs::remove_file(&pending_path)
            .map_err(|error| format!("evidence_agent_pending_remove:{error}"))?;
        sync_directory(&self.config.state_directory)?;
        self.compact_acknowledged_outbox()?;
        Ok(AgentPollReport {
            accepted_frames: ack.accepted_frames,
        })
    }

    fn compact_acknowledged_outbox(&mut self) -> Result<(), String> {
        let mut outbox = self
            .outbox
            .lock()
            .map_err(|_| "evidence_agent_outbox_lock_poisoned".to_owned())?;
        let state_changed = if outbox.frames.is_empty() {
            !self.state.seen_frame_roots.is_empty()
        } else if outbox
            .frames
            .keys()
            .all(|root| self.state.seen_frame_roots.contains(root))
        {
            outbox.compact_all()?;
            true
        } else {
            false
        };
        drop(outbox);
        if state_changed {
            self.state.seen_frame_roots.clear();
            write_canonical_state(&self.config.state_directory.join(STATE_FILE), &self.state)?;
        }
        Ok(())
    }

    fn build_pending(&self) -> Result<Option<EvidenceAgentPendingV1>, String> {
        let outbox = self
            .outbox
            .lock()
            .map_err(|_| "evidence_agent_outbox_lock_poisoned".to_owned())?;
        let selected = outbox
            .frames
            .iter()
            .filter(|(root, _)| !self.state.seen_frame_roots.contains(*root))
            .take(REMOTE_EVIDENCE_MAX_FRAMES_V1)
            .map(|(root, frame)| (root.clone(), frame.clone()))
            .collect::<Vec<_>>();
        drop(outbox);
        if selected.is_empty() {
            return Ok(None);
        }
        let mut frame_roots = selected
            .iter()
            .map(|(root, _)| root.clone())
            .collect::<Vec<_>>();
        let mut frames = selected
            .into_iter()
            .map(|(_, bound)| RouteBoundEvidenceFrameV1 {
                frame: bound.frame,
                route_receipt: bound.route_receipt,
                runtime_parity_case: bound.runtime_parity_case,
            })
            .collect::<Vec<_>>();
        let generated_at_unix = unix_now_seconds()?;
        let batch = loop {
            match RemoteEvidenceBatchV1::seal_route_bound(
                self.state.client_id_sha256.clone(),
                self.state.next_sequence,
                self.state.previous_batch_root_sha256.clone(),
                generated_at_unix,
                frames.clone(),
            ) {
                Ok(batch) => break batch,
                Err(error) if error == "remote_evidence_batch_budget" && frames.len() > 1 => {
                    frames.pop();
                    frame_roots.pop();
                }
                Err(error) => return Err(error),
            }
        };
        let pending = EvidenceAgentPendingV1 {
            schema: AGENT_PENDING_SCHEMA_V2.to_owned(),
            client_id_sha256: self.state.client_id_sha256.clone(),
            batch,
            frame_roots,
        };
        write_canonical_state(&self.config.state_directory.join(PENDING_FILE), &pending)?;
        Ok(Some(pending))
    }

    fn status_json(&self, accepted_this_poll: u64) -> Result<String, String> {
        let (source_files, graphs, _, censored_identities, censored_utf8, watcher, events, last) =
            self.stream_metrics.snapshot();
        let outbox_frames = self
            .outbox
            .lock()
            .map_err(|_| "evidence_agent_outbox_lock_poisoned".to_owned())?
            .frames
            .len();
        let route_receipts = self
            .route_receipts
            .lock()
            .map_err(|_| "evidence_agent_route_receipt_lock_poisoned".to_owned())?
            .len();
        serde_json::to_string(&serde_json::json!({
            "ok": true,
            "client_id_sha256": self.state.client_id_sha256,
            "session_source_files": source_files,
            "session_watcher_alive": watcher,
            "session_watcher_events": events,
            "session_watcher_last_event_unix": last,
            "verified_graphs": graphs,
            "censored_invalid_session_identities": censored_identities,
            "censored_invalid_utf8_rows": censored_utf8,
            "outbox_frames": outbox_frames,
            "route_receipts": route_receipts,
            "route_bound_frames": self.route_metrics.route_bound_frames.load(Ordering::Relaxed),
            "route_unbound_frames": self.route_metrics.route_unbound_frames.load(Ordering::Relaxed),
            "route_receipt_refresh_failures": self.route_metrics.route_receipt_refresh_failures.load(Ordering::Relaxed),
            "route_provenance_required": true,
            "accepted_batches": self.state.accepted_batches,
            "accepted_frames": self.state.accepted_frames,
            "accepted_this_poll": accepted_this_poll,
            "raw_session_payload_persisted": false,
        }))
        .map_err(|error| format!("evidence_agent_status_encode:{error}"))
    }
}

impl EvidenceAgentStateV1 {
    fn validate(&self, client_id_sha256: &str) -> bool {
        self.schema == AGENT_STATE_SCHEMA_V1
            && self.client_id_sha256 == client_id_sha256
            && self.opened_at_unix_nanos > 0
            && self.next_sequence > 0
            && self.accepted_batches.saturating_add(1) == self.next_sequence
            && self.seen_frame_roots.len() <= MAX_OUTBOX_FRAMES
            && self.seen_frame_roots.iter().all(|root| valid_root(root))
            && valid_root(&self.previous_batch_root_sha256)
            && (self.accepted_batches == 0
                && self.accepted_frames == 0
                && self.last_accepted_at_unix == 0
                && self.previous_batch_root_sha256
                    == remote_evidence_genesis_root(client_id_sha256)
                || self.accepted_batches > 0
                    && self.accepted_frames > 0
                    && self.last_accepted_at_unix > 0)
    }
}

impl EvidenceAgentPendingV1 {
    fn validate(&self, state: &EvidenceAgentStateV1) -> bool {
        self.schema == AGENT_PENDING_SCHEMA_V2
            && self.client_id_sha256 == state.client_id_sha256
            && self.batch.validate()
            && self
                .batch
                .frames
                .iter()
                .all(RemoteEvidenceFrameV1::is_route_bound)
            && self.batch.client_id_sha256 == state.client_id_sha256
            && self.batch.sequence == state.next_sequence
            && self.batch.previous_batch_root_sha256 == state.previous_batch_root_sha256
            && self.frame_roots.len() == self.batch.frames.len()
            && self
                .frame_roots
                .iter()
                .zip(&self.batch.frames)
                .all(|(root, frame)| root == &frame.frame_root_sha256)
    }
}

impl LocalEvidenceOutbox {
    fn open(directory: &Path) -> Result<Self, String> {
        let ledger = FramedCborLedger::open(directory, OUTBOX_PREFIX)?;
        let persisted = read_framed_cbor::<RouteBoundOutboxFrameV1>(directory, OUTBOX_PREFIX)?;
        let mut frames = BTreeMap::<String, RouteBoundOutboxFrameV1>::new();
        let mut payload_bytes = 0_u64;
        for bound in persisted {
            let sealed = bound.seal()?;
            let bytes = frame_bytes(&bound)?;
            let new_root = !frames.contains_key(&sealed.frame_root_sha256);
            if let Some(existing) = frames.get(&sealed.frame_root_sha256) {
                if existing == &bound {
                    continue;
                }
                if !existing.same_transport_binding(&bound)
                    || existing.runtime_parity_case.is_some()
                    || bound.runtime_parity_case.is_none()
                {
                    return Err("evidence_agent_outbox_rebound".to_owned());
                }
            }
            if new_root && frames.len() >= MAX_OUTBOX_FRAMES
                || payload_bytes.saturating_add(bytes) > MAX_OUTBOX_BYTES
            {
                return Err("evidence_agent_outbox_budget".to_owned());
            }
            payload_bytes = payload_bytes.saturating_add(bytes);
            frames.insert(sealed.frame_root_sha256, bound);
        }
        Ok(Self {
            ledger,
            frames,
            payload_bytes,
        })
    }

    fn append(
        &mut self,
        frame: RelationFrame,
        route_receipt: NandoRouteReceiptV1,
        runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Result<(), String> {
        let bound = RouteBoundOutboxFrameV1 {
            schema: ROUTE_BOUND_OUTBOX_SCHEMA_V1.to_owned(),
            route_receipt_root_sha256: route_receipt.receipt_root_sha256.clone(),
            route_receipt,
            frame,
            runtime_parity_case,
        };
        let sealed = bound.seal()?;
        let new_root = !self.frames.contains_key(&sealed.frame_root_sha256);
        if let Some(existing) = self.frames.get(&sealed.frame_root_sha256) {
            if existing == &bound
                || existing.same_transport_binding(&bound)
                    && existing.runtime_parity_case.is_some()
                    && bound.runtime_parity_case.is_none()
            {
                return Ok(());
            }
            if !existing.same_transport_binding(&bound)
                || existing.runtime_parity_case.is_some()
                || bound.runtime_parity_case.is_none()
            {
                return Err("evidence_agent_outbox_rebound".to_owned());
            }
        }
        let bytes = frame_bytes(&bound)?;
        if new_root && self.frames.len() >= MAX_OUTBOX_FRAMES
            || self.payload_bytes.saturating_add(bytes) > MAX_OUTBOX_BYTES
        {
            return Err("evidence_agent_outbox_budget".to_owned());
        }
        self.ledger.append(&bound)?;
        self.ledger.sync()?;
        self.payload_bytes = self.payload_bytes.saturating_add(bytes);
        self.frames.insert(sealed.frame_root_sha256, bound);
        Ok(())
    }

    fn compact_all(&mut self) -> Result<(), String> {
        self.ledger.compact_after_checkpoint()?;
        self.frames.clear();
        self.payload_bytes = 0;
        Ok(())
    }
}

impl VerifiedRelationFrameSink for OutboxSink {
    fn append_verified_frame_with_parity(
        &self,
        frame: RelationFrame,
        runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Result<(), String> {
        let route_receipt = {
            let mut receipts = self
                .route_receipts
                .lock()
                .map_err(|_| "evidence_agent_route_receipt_lock_poisoned".to_owned())?;
            if let Err(error) = receipts.refresh() {
                self.route_metrics
                    .route_receipt_refresh_failures
                    .fetch_add(1, Ordering::Relaxed);
                return Err(error);
            }
            receipts
                .receipt_for_frame(
                    &frame.client_intent_id_sha256,
                    &frame.session_id_sha256,
                    frame.observed_at_unix_nanos,
                )
                .cloned()
        };
        let Some(route_receipt) = route_receipt else {
            self.route_metrics
                .route_unbound_frames
                .fetch_add(1, Ordering::Relaxed);
            return Ok(());
        };
        self.outbox
            .lock()
            .map_err(|_| "evidence_agent_outbox_lock_poisoned".to_owned())?
            .append(frame, route_receipt, runtime_parity_case)?;
        self.route_metrics
            .route_bound_frames
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

impl HttpEndpoint {
    fn parse(origin: &str) -> Result<Self, String> {
        let authority = origin
            .strip_prefix("http://")
            .ok_or_else(|| "evidence_agent_server_requires_http_lan_origin".to_owned())?
            .trim_end_matches('/');
        if authority.is_empty() || authority.contains('/') || authority.contains('@') {
            return Err("evidence_agent_server_invalid".to_owned());
        }
        let (socket_host, port) = authority.rsplit_once(':').map_or_else(
            || Ok::<_, String>((authority.to_owned(), 80)),
            |(host, port)| {
                let port = port
                    .parse::<u16>()
                    .map_err(|_| "evidence_agent_server_port_invalid".to_owned())?;
                Ok((host.to_owned(), port))
            },
        )?;
        if socket_host.is_empty() {
            return Err("evidence_agent_server_invalid".to_owned());
        }
        Ok(Self {
            socket_host,
            port,
            host_header: authority.to_owned(),
            path: REMOTE_EVIDENCE_ENDPOINT_V1.to_owned(),
        })
    }

    fn post_batch(
        &self,
        client_id_sha256: &str,
        timestamp_unix: u64,
        signature: &str,
        body: &[u8],
    ) -> Result<RemoteEvidenceAckV1, String> {
        let addresses = (self.socket_host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|error| format!("evidence_agent_resolve:{error}"))?
            .collect::<Vec<_>>();
        let mut stream = addresses
            .iter()
            .find_map(|address| TcpStream::connect_timeout(address, Duration::from_secs(5)).ok())
            .ok_or_else(|| "evidence_agent_connect_failed".to_owned())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .and_then(|()| stream.set_write_timeout(Some(Duration::from_secs(20))))
            .map_err(|error| format!("evidence_agent_socket_timeout:{error}"))?;
        let headers = format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/cbor\r\nContent-Length: {}\r\nX-Nando-Evidence-Client: {}\r\nX-Nando-Evidence-Timestamp: {}\r\nX-Nando-Evidence-Signature: {}\r\nConnection: close\r\n\r\n",
            self.path,
            self.host_header,
            body.len(),
            client_id_sha256,
            timestamp_unix,
            signature
        );
        stream
            .write_all(headers.as_bytes())
            .and_then(|()| stream.write_all(body))
            .and_then(|()| stream.flush())
            .map_err(|error| format!("evidence_agent_send:{error}"))?;
        let mut response = Vec::new();
        Read::by_ref(&mut stream)
            .take(MAX_HTTP_RESPONSE_BYTES.saturating_add(1))
            .read_to_end(&mut response)
            .map_err(|error| format!("evidence_agent_receive:{error}"))?;
        if response.len() as u64 > MAX_HTTP_RESPONSE_BYTES {
            return Err("evidence_agent_response_budget".to_owned());
        }
        let (status, response_body) = parse_http_response(&response)?;
        if status != 200 {
            let error = serde_json::from_slice::<serde_json::Value>(&response_body)
                .ok()
                .and_then(|value| {
                    value
                        .get("error")
                        .and_then(|error| error.as_str())
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| "remote_rejected".to_owned());
            return Err(format!("evidence_agent_http_{status}:{error}"));
        }
        serde_json::from_slice(&response_body)
            .map_err(|error| format!("evidence_agent_ack_decode:{error}"))
    }
}

fn parse_http_response(response: &[u8]) -> Result<(u16, Vec<u8>), String> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "evidence_agent_http_header_incomplete".to_owned())?;
    let headers = std::str::from_utf8(&response[..header_end])
        .map_err(|_| "evidence_agent_http_header_invalid".to_owned())?;
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_ascii_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| "evidence_agent_http_status_invalid".to_owned())?;
    let body = response[header_end.saturating_add(4)..].to_vec();
    if headers.lines().any(|line| {
        line.to_ascii_lowercase()
            .starts_with("transfer-encoding: chunked")
    }) {
        return decode_chunked_body(&body).map(|body| (status, body));
    }
    if let Some(length) = headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse::<usize>().ok())
            .flatten()
    }) && body.len() != length
    {
        return Err("evidence_agent_http_body_incomplete".to_owned());
    }
    Ok((status, body))
}

fn decode_chunked_body(encoded: &[u8]) -> Result<Vec<u8>, String> {
    let mut cursor = 0_usize;
    let mut output = Vec::new();
    loop {
        let line_end = encoded[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .map(|offset| cursor.saturating_add(offset))
            .ok_or_else(|| "evidence_agent_http_chunk_invalid".to_owned())?;
        let length_text = std::str::from_utf8(&encoded[cursor..line_end])
            .map_err(|_| "evidence_agent_http_chunk_invalid".to_owned())?
            .split(';')
            .next()
            .ok_or_else(|| "evidence_agent_http_chunk_invalid".to_owned())?;
        let length = usize::from_str_radix(length_text, 16)
            .map_err(|_| "evidence_agent_http_chunk_invalid".to_owned())?;
        cursor = line_end.saturating_add(2);
        if length == 0 {
            return Ok(output);
        }
        let end = cursor.saturating_add(length);
        if encoded.get(end..end.saturating_add(2)) != Some(b"\r\n") {
            return Err("evidence_agent_http_chunk_invalid".to_owned());
        }
        output.extend_from_slice(
            encoded
                .get(cursor..end)
                .ok_or_else(|| "evidence_agent_http_chunk_invalid".to_owned())?,
        );
        if output.len() as u64 > MAX_HTTP_RESPONSE_BYTES {
            return Err("evidence_agent_response_budget".to_owned());
        }
        cursor = end.saturating_add(2);
    }
}

impl RouteBoundOutboxFrameV1 {
    fn seal(&self) -> Result<RemoteEvidenceFrameV1, String> {
        if self.schema != ROUTE_BOUND_OUTBOX_SCHEMA_V1 {
            return Err("evidence_agent_outbox_schema_invalid".to_owned());
        }
        RemoteEvidenceFrameV1::seal_route_bound_with_parity(
            self.frame.clone(),
            self.route_receipt.clone(),
            self.runtime_parity_case.clone(),
        )
    }

    fn same_transport_binding(&self, other: &Self) -> bool {
        self.schema == other.schema
            && self.route_receipt_root_sha256 == other.route_receipt_root_sha256
            && self.route_receipt == other.route_receipt
            && self.frame == other.frame
    }
}

fn frame_bytes<T: Serialize>(frame: &T) -> Result<u64, String> {
    let bytes = serde_cbor::to_vec(frame)
        .map_err(|error| format!("evidence_agent_outbox_encode:{error}"))?;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err("evidence_agent_outbox_frame_budget".to_owned());
    }
    Ok(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
}

fn valid_root(root: &str) -> bool {
    root.len() == 64 && root.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn retry_backoff(base: Duration, consecutive_failures: u32) -> Duration {
    let shift = consecutive_failures.saturating_sub(1).min(7);
    let multiplier = 1_u32 << shift;
    base.saturating_mul(multiplier).min(Duration::from_secs(30))
}

fn wait_for_session_watcher(metrics: &SessionStreamMetrics) -> Result<(), String> {
    for _attempt in 0..40 {
        if metrics.snapshot().5 {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err("evidence_agent_session_watcher_start_timeout".to_owned())
}

fn read_private_key(path: &Path) -> Result<Vec<u8>, String> {
    let metadata =
        fs::metadata(path).map_err(|error| format!("evidence_agent_key_metadata:{error}"))?;
    #[cfg(unix)]
    if metadata.mode() & 0o077 != 0 {
        return Err("evidence_agent_key_permissions".to_owned());
    }
    let bytes = fs::read(path).map_err(|error| format!("evidence_agent_key_read:{error}"))?;
    parse_remote_evidence_key_v1(&bytes)
}

fn ensure_private_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| format!("evidence_agent_state_directory:{error}"))?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("evidence_agent_state_permissions:{error}"))?;
    Ok(())
}

fn read_optional_bounded(path: &Path, max_bytes: usize) -> Result<Option<Vec<u8>>, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("evidence_agent_state_read:{error}")),
    };
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(u64::try_from(max_bytes.saturating_add(1)).unwrap_or(u64::MAX))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("evidence_agent_state_read:{error}"))?;
    if bytes.is_empty() || bytes.len() > max_bytes {
        return Err("evidence_agent_state_budget".to_owned());
    }
    Ok(Some(bytes))
}

fn write_canonical_state<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_cbor::to_vec(value)
        .map_err(|error| format!("evidence_agent_state_encode:{error}"))?;
    if bytes.is_empty() || bytes.len() > MAX_STATE_BYTES {
        return Err("evidence_agent_state_budget".to_owned());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "evidence_agent_state_parent_missing".to_owned())?;
    ensure_private_directory(parent)?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("evidence_agent_state_open:{error}"))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("evidence_agent_state_write:{error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("evidence_agent_state_rename:{error}"))?;
    sync_directory(parent)
}

fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("evidence_agent_directory_sync:{error}"))
}

fn unix_now_seconds() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| "evidence_agent_clock_before_epoch".to_owned())
}

fn unix_now_nanos() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX))
        .map_err(|_| "evidence_agent_clock_before_epoch".to_owned())
}

fn default_route_receipts_path() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("nando-connector/route-receipts-v1.jsonl")
}

fn usage() -> String {
    "usage: nando-evidence-agent [--sessions-dir PATH] [--server http://HOST:PORT] [--key-file PATH] [--state-dir PATH] [--route-receipts FILE] [--poll-ms 100..60000] [--once] [--check]".to_owned()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use std::time::{SystemTime, UNIX_EPOCH};

    use nando_client_evidence::{
        ClientRouteIdentityV1, DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES, NandoRouteReceiptIndex,
        NandoRouteReceiptLedger, NandoRouteReceiptV1, evidence_client_intent_id_sha256,
        evidence_session_id_sha256, route_receipt_genesis_root,
        sha256_bytes as client_sha256_bytes,
    };
    use nando_operator_kernel::{
        AtomSource, AtomValueType, RELATION_FRAME_SCHEMA, RelationAtom, RelationFrame,
        ResponseValueSelector, sha256_bytes,
    };
    use nando_operator_learning::{RuntimeParityCase, SOURCE_NEUTRAL_EXTRACTOR_VERSION};
    use serde_json::json;

    use super::{
        HttpEndpoint, LocalEvidenceOutbox, OutboxSink, RouteBindingMetrics,
        VerifiedRelationFrameSink, decode_chunked_body, parse_http_response, retry_backoff,
        valid_root,
    };

    fn hash(value: &str) -> String {
        sha256_bytes(value.as_bytes())
    }

    fn completed_frame(label: &str) -> RelationFrame {
        let value_root = hash(&format!("value:{label}"));
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: hash(&format!("frame:{label}")),
            event_id_sha256: hash(&format!("event:{label}")),
            client_intent_id_sha256: hash(&format!("intent:{label}")),
            session_id_sha256: hash(&format!("session:{label}")),
            observed_at_unix_nanos: 1_700_000_000_000_000_000,
            estimated_input_tokens: 100,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
                RelationAtom::TypedSlot {
                    slot_id: 7,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Observation,
                    value_sha256: value_root.clone(),
                },
                RelationAtom::UniqueSlot { slot_id: 7 },
                RelationAtom::ObservationSelector {
                    slot_id: 7,
                    selector: ResponseValueSelector::JsonField {
                        field: "opaque".to_owned(),
                        value_type: AtomValueType::Integer,
                    },
                },
                RelationAtom::TypedSlot {
                    slot_id: 11,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Action,
                    value_sha256: value_root,
                },
                RelationAtom::SlotEquality {
                    left_slot: 7,
                    right_slot: 11,
                },
                RelationAtom::ActionFunction {
                    value: "transport_a".to_owned(),
                },
                RelationAtom::ActionRoleArgument {
                    name: "value".to_owned(),
                    slot_id: 11,
                    value_type: Some(AtomValueType::Integer),
                },
            ],
            evidence_ref_sha256: hash(&format!("evidence:{label}")),
        }
    }

    fn temporary_root(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "nando-evidence-agent-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn route_receipt_for_frame(frame: &RelationFrame) -> NandoRouteReceiptV1 {
        NandoRouteReceiptV1::seal(
            1,
            route_receipt_genesis_root(),
            &ClientRouteIdentityV1 {
                turn_intent_id_sha256: frame.client_intent_id_sha256.clone(),
                session_id_sha256: frame.session_id_sha256.clone(),
            },
            client_sha256_bytes(b"request"),
            418,
            frame.observed_at_unix_nanos.saturating_sub(2),
            frame.observed_at_unix_nanos.saturating_sub(1),
        )
        .expect("route receipt")
    }

    fn runtime_parity(frame: &RelationFrame, expected_response: &str) -> RuntimeParityCase {
        RuntimeParityCase {
            evidence_ref_sha256: frame.frame_id_sha256.clone(),
            capture_receipt: None,
            request_text: "Return opaque".to_owned(),
            provider_payload: json!({
                "input": [{
                    "type": "function_call_output",
                    "output": "{\"opaque\":7}"
                }]
            }),
            expected_response: expected_response.to_owned(),
        }
    }

    #[test]
    fn parses_lan_origin_without_response_api_prefix() {
        let endpoint = HttpEndpoint::parse("http://192.168.3.94:8787/").expect("endpoint");
        assert_eq!(endpoint.socket_host, "192.168.3.94");
        assert_eq!(endpoint.port, 8787);
        assert_eq!(endpoint.path, "/_nando/evidence/v1/batches");
        assert!(HttpEndpoint::parse("https://192.168.3.94:8787").is_err());
        assert!(HttpEndpoint::parse("http://192.168.3.94:8787/v1").is_err());
    }

    #[test]
    fn parses_content_length_and_chunked_json_responses() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n{\"ok\":true}";
        let (status, body) = parse_http_response(response).expect("response");
        assert_eq!(status, 200);
        assert_eq!(body, b"{\"ok\":true}");

        let encoded = b"4\r\n{\"ok\r\n7\r\n\":true}\r\n0\r\n\r\n";
        assert_eq!(
            decode_chunked_body(encoded).expect("chunked"),
            b"{\"ok\":true}"
        );
    }

    #[test]
    fn retry_backoff_is_bounded() {
        let base = Duration::from_millis(250);
        assert_eq!(retry_backoff(base, 1), base);
        assert_eq!(retry_backoff(base, 4), Duration::from_secs(2));
        assert_eq!(retry_backoff(base, 100), Duration::from_secs(30));
    }

    #[test]
    fn acknowledged_outbox_compacts_and_restarts_empty() {
        let root = temporary_root("compact");
        let frame = completed_frame("compact");
        let mut outbox = LocalEvidenceOutbox::open(&root).expect("outbox");
        outbox
            .append(frame.clone(), route_receipt_for_frame(&frame), None)
            .expect("append");
        assert_eq!(outbox.frames.len(), 1);
        outbox.compact_all().expect("compact");
        assert!(outbox.frames.is_empty());
        assert_eq!(outbox.payload_bytes, 0);
        drop(outbox);

        let restored = LocalEvidenceOutbox::open(&root).expect("restore");
        assert!(restored.frames.is_empty());
        assert_eq!(restored.payload_bytes, 0);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn outbox_allows_only_monotonic_runtime_parity_enrichment() {
        let root = temporary_root("parity-enrichment");
        let frame = completed_frame("parity-enrichment");
        let route_receipt = route_receipt_for_frame(&frame);
        let mut outbox = LocalEvidenceOutbox::open(&root).expect("outbox");
        outbox
            .append(frame.clone(), route_receipt.clone(), None)
            .expect("legacy append");
        outbox
            .append(
                frame.clone(),
                route_receipt.clone(),
                Some(runtime_parity(&frame, "7")),
            )
            .expect("parity enrichment");
        assert_eq!(outbox.frames.len(), 1);
        assert!(
            outbox
                .frames
                .values()
                .all(|bound| bound.runtime_parity_case.is_some())
        );
        drop(outbox);

        let mut restored = LocalEvidenceOutbox::open(&root).expect("restore");
        assert!(
            restored
                .frames
                .values()
                .all(|bound| bound.runtime_parity_case.is_some())
        );
        restored
            .append(frame.clone(), route_receipt.clone(), None)
            .expect("legacy replay cannot erase parity");
        assert_eq!(
            restored
                .append(
                    frame.clone(),
                    route_receipt,
                    Some(runtime_parity(&frame, "8")),
                )
                .expect_err("parity rebound must fail"),
            "evidence_agent_outbox_rebound"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn outbox_accepts_only_pre_action_nando_route_bound_frames() {
        let root = temporary_root("route-bound");
        let route_path = root.join("route-receipts-v1.jsonl");
        let outbox = Arc::new(Mutex::new(
            LocalEvidenceOutbox::open(&root.join("outbox")).expect("outbox"),
        ));
        let route_index = Arc::new(Mutex::new(
            NandoRouteReceiptIndex::open(&route_path, DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES)
                .expect("route index"),
        ));
        let metrics = Arc::new(RouteBindingMetrics::default());
        let sink = OutboxSink {
            outbox: Arc::clone(&outbox),
            route_receipts: Arc::clone(&route_index),
            route_metrics: Arc::clone(&metrics),
        };

        let mut unbound = completed_frame("unbound");
        unbound.client_intent_id_sha256 = evidence_client_intent_id_sha256("turn-unbound");
        unbound.session_id_sha256 = evidence_session_id_sha256("session-unbound");
        sink.append_verified_frame(unbound).expect("censor unbound");
        assert!(outbox.lock().expect("outbox lock").frames.is_empty());
        assert_eq!(metrics.route_unbound_frames.load(Ordering::Relaxed), 1);

        let identity = ClientRouteIdentityV1 {
            turn_intent_id_sha256: evidence_client_intent_id_sha256("turn-bound"),
            session_id_sha256: evidence_session_id_sha256("session-bound"),
        };
        let mut ledger =
            NandoRouteReceiptLedger::open(&route_path, DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES)
                .expect("route ledger");
        ledger
            .append(
                &identity,
                client_sha256_bytes(b"request"),
                418,
                1_600_000_000_000_000_000,
                1_650_000_000_000_000_000,
            )
            .expect("route receipt");
        let mut bound = completed_frame("bound");
        bound.client_intent_id_sha256 = identity.turn_intent_id_sha256;
        bound.session_id_sha256 = identity.session_id_sha256;
        sink.append_verified_frame(bound).expect("append bound");
        let outbox = outbox.lock().expect("outbox lock");
        assert_eq!(outbox.frames.len(), 1);
        assert!(outbox.frames.values().all(|bound| {
            valid_root(&bound.route_receipt_root_sha256)
                && bound.route_receipt.validate()
                && bound.route_receipt.receipt_root_sha256 == bound.route_receipt_root_sha256
        }));
        assert_eq!(metrics.route_bound_frames.load(Ordering::Relaxed), 1);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
