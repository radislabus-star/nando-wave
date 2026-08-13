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
use nando_operator_kernel::{RelationFrame, canonical_json_sha256, sha256_bytes};
use nando_operator_learning::{FramedCborLedger, RuntimeParityCase, read_framed_cbor};
use nando_transition_serving::remote_evidence_spool::{
    REMOTE_EVIDENCE_ENDPOINT_V1, REMOTE_EVIDENCE_MAX_BATCH_BYTES_V1, REMOTE_EVIDENCE_MAX_FRAMES_V1,
    RemoteEvidenceAckV1, RemoteEvidenceBatchV1, RemoteEvidenceFrameSealErrorV1,
    RemoteEvidenceFrameV1, RemoteEvidenceFrameValidationBlockerV1, RouteBoundEvidenceFrameV1,
    parse_remote_evidence_key_v1, remote_evidence_genesis_root, sign_remote_evidence_request_v1,
};
use nando_transition_serving::{
    SessionStreamMetrics, VerifiedRelationFrameSink, spawn_verified_relation_frame_stream,
};
use serde::{Deserialize, Serialize};

include!("nando_evidence_agent/outbox.rs");
include!("nando_evidence_agent/transport.rs");
include!("nando_evidence_agent/transport_censor.rs");

const AGENT_STATE_SCHEMA_V1: &str = "nando.local-evidence-agent-state.v1";
const AGENT_PENDING_SCHEMA_V2: &str = "nando.local-evidence-agent-pending.v2";
const ROUTE_BOUND_OUTBOX_SCHEMA_V1: &str = "nando.route-bound-evidence-outbox-frame.v1";
const OUTBOX_PREFIX: &str = "route-bound-relation-frame";
const TRANSPORT_CENSOR_SCHEMA_V1: &str = "nando.evidence-agent-transport-censor.v1";
const TRANSPORT_CENSOR_PREFIX: &str = "transport-censor-receipt";
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

struct EvidenceAgent {
    config: AgentConfig,
    endpoint: HttpEndpoint,
    key: Vec<u8>,
    state: EvidenceAgentStateV1,
    outbox: Arc<Mutex<LocalEvidenceOutbox>>,
    transport_censors: Arc<Mutex<TransportCensorLedger>>,
    stream_metrics: Arc<SessionStreamMetrics>,
    route_receipts: Arc<Mutex<NandoRouteReceiptIndex>>,
    route_metrics: Arc<RouteBindingMetrics>,
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
        let transport_censors = Arc::new(Mutex::new(TransportCensorLedger::open(
            &config.state_directory.join("transport-censors-v1"),
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
                transport_censors: Arc::clone(&transport_censors),
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
            transport_censors,
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
        let selected = outbox.selected_unseen(
            &self.state.seen_frame_roots,
            REMOTE_EVIDENCE_MAX_FRAMES_V1,
            REMOTE_EVIDENCE_MAX_BATCH_BYTES_V1.saturating_sub(64 * 1024),
        )?;
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
        let durable_transport_censors = self
            .transport_censors
            .lock()
            .map_err(|_| "evidence_agent_transport_censor_lock_poisoned".to_owned())?
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
            "transport_censored_frames": self.route_metrics.transport_censored_frames.load(Ordering::Relaxed),
            "durable_transport_censors": durable_transport_censors,
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
include!("nando_evidence_agent/tests.rs");
