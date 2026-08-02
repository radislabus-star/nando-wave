use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock, mpsc};
use std::thread;

use memchr::memchr_iter;
use nando_operator_kernel::{
    AtomSource, AtomValueType, CollectionOutputRenderer, RELATION_FRAME_SCHEMA, RelationAtom,
    RelationFrame, ResponseProgram, ResponseRenderSegment, ResponseValueSelector,
    ValueProjectionFormat, VerifierProgram, canonical_json_sha256, sha256_bytes,
};
#[cfg(test)]
use nando_operator_learning::DeterministicEvidenceLedger;
use nando_operator_learning::{
    CanonicalEventGraph, CaptureEvidenceReceipt, CaptureRecordCommitment,
    DeterministicEvidenceGraphStore, EvidenceGraphBuilder, EvidenceGraphPolicy,
    EvidenceIngestOutcome, EvidencePolicyV1, OnlineCollectionObservation, RawEvidenceEnvelope,
    RuntimeParityCase, SOURCE_NEUTRAL_EXTRACTOR_VERSION, evidence_session_id_sha256,
    teacher_action_symbol, teacher_program_signature, teacher_program_signature_from_action_atoms,
};
use nando_operator_proof::verify_response_independently;
use nando_operator_runtime::ResponseExecutionStatus;
use nando_response_actor::{
    CompletedTurnExample, OnlineCollectionMiner, TurnCompletionReason, execute_response,
};
use notify::event::ModifyKind;
use notify::{EventKind, RecursiveMode, Watcher};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

mod request_text;
use request_text::bounded_runtime_request_text;

use crate::miner_worker::MinerWorkerHandle;
#[cfg(test)]
use crate::request_learning::RequestLearningAtoms;
use crate::request_learning::RequestLearningIndex;
use crate::stream_evidence::{SessionEvidenceLedger, StreamingEvidenceLedger};
#[cfg(test)]
use nando_operator_learning::LearningRequestStructureV1;

const MAX_OBSERVATIONS: usize = 32;
const MAX_TURN_EVIDENCE_EVENTS: usize = 64;
const MAX_TURN_EVIDENCE_NODES: usize = 8_192;
const MAX_SESSION_META_BYTES: u64 = 1024 * 1024;
const MAX_PENDING_RUNTIME_PARITY_CASES: usize = 1_024;
const MAX_PENDING_MINER_INPUTS: usize = 4_096;

#[cfg(test)]
impl SessionEvidenceLedger for DeterministicEvidenceLedger {
    fn ingest_session_event(
        &mut self,
        envelope: RawEvidenceEnvelope,
    ) -> Result<nando_response_actor::EvidenceLedgerRecord, String> {
        DeterministicEvidenceLedger::ingest(self, envelope)
    }

    fn resume_offset(&self, source_stream_id: &str) -> Option<u64> {
        DeterministicEvidenceLedger::resume_offset(self, source_stream_id)
    }

    fn accounting(&self) -> nando_response_actor::EvidenceAccounting {
        DeterministicEvidenceLedger::accounting(self)
    }

    fn recovered_partial_tail_bytes(&self) -> u64 {
        DeterministicEvidenceLedger::recovered_partial_tail_bytes(self)
    }

    fn bind_transition(
        &mut self,
        frame_id_sha256: &str,
        receipt: &CaptureEvidenceReceipt,
    ) -> Result<nando_response_actor::CaptureTransitionBinding, String> {
        nando_response_actor::CaptureTransitionBinding::new(
            receipt.records.last().map_or(0, |record| record.sequence),
            frame_id_sha256,
            receipt,
        )
        .map_err(str::to_owned)
    }
}

#[allow(
    clippy::large_enum_variant,
    reason = "the bounded bridge keeps frames inline to avoid a heap allocation per live event"
)]
enum PendingMinerInput {
    Frame(RelationFrame, Option<RuntimeParityCase>),
    Collection(OnlineCollectionObservation),
}

trait SessionMinerSink: Send + Sync {
    fn submit_frame_with_parity(
        &self,
        frame: RelationFrame,
        runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Result<(), String>;
    fn submit_collection(&self, observation: OnlineCollectionObservation) -> Result<(), String>;

    fn binds_collection_capture(&self) -> bool {
        false
    }

    fn submit_failure_is_fatal(&self) -> bool {
        false
    }
}

#[derive(Default)]
struct CollectionObservationCollector {
    observations: Mutex<Vec<OnlineCollectionObservation>>,
}

impl SessionMinerSink for CollectionObservationCollector {
    fn submit_frame_with_parity(
        &self,
        _frame: RelationFrame,
        _runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Result<(), String> {
        Ok(())
    }

    fn submit_collection(&self, observation: OnlineCollectionObservation) -> Result<(), String> {
        self.observations
            .lock()
            .map_err(|_| "collection_observation_collector_poisoned".to_owned())?
            .push(observation);
        Ok(())
    }
}

impl SessionMinerSink for MinerWorkerHandle {
    fn submit_frame_with_parity(
        &self,
        frame: RelationFrame,
        runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Result<(), String> {
        MinerWorkerHandle::submit_frame_with_parity(self, frame, runtime_parity_case)
    }

    fn submit_collection(&self, observation: OnlineCollectionObservation) -> Result<(), String> {
        MinerWorkerHandle::submit_collection(self, observation)
    }

    fn binds_collection_capture(&self) -> bool {
        true
    }
}

pub trait VerifiedRelationFrameSink: Send + Sync {
    fn append_verified_frame_with_parity(
        &self,
        frame: RelationFrame,
        runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Result<(), String>;

    fn append_verified_frame(&self, frame: RelationFrame) -> Result<(), String> {
        self.append_verified_frame_with_parity(frame, None)
    }
}

struct VerifiedRelationFrameSinkAdapter {
    sink: Arc<dyn VerifiedRelationFrameSink>,
}

impl SessionMinerSink for VerifiedRelationFrameSinkAdapter {
    fn submit_frame_with_parity(
        &self,
        frame: RelationFrame,
        runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Result<(), String> {
        self.sink
            .append_verified_frame_with_parity(frame, runtime_parity_case)
    }

    fn submit_collection(&self, _observation: OnlineCollectionObservation) -> Result<(), String> {
        Ok(())
    }

    fn submit_failure_is_fatal(&self) -> bool {
        true
    }
}

pub struct SessionMinerBridge {
    worker: RwLock<Option<Arc<dyn SessionMinerSink>>>,
    pending: Mutex<VecDeque<PendingMinerInput>>,
    dropped: AtomicU64,
}

impl SessionMinerBridge {
    #[must_use]
    pub fn new() -> Self {
        Self {
            worker: RwLock::new(None),
            pending: Mutex::new(VecDeque::new()),
            dropped: AtomicU64::new(0),
        }
    }

    pub fn install(&self, worker: MinerWorkerHandle) -> Result<(), String> {
        self.install_sink(Arc::new(worker))
    }

    fn install_sink(&self, worker: Arc<dyn SessionMinerSink>) -> Result<(), String> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| "session_miner_bridge_pending_poisoned".to_owned())?;
        *self
            .worker
            .write()
            .map_err(|_| "session_miner_bridge_worker_poisoned".to_owned())? =
            Some(Arc::clone(&worker));
        while let Some(input) = pending.pop_front() {
            match input {
                PendingMinerInput::Frame(frame, runtime_parity_case) => {
                    worker.submit_frame_with_parity(frame, runtime_parity_case)?
                }
                PendingMinerInput::Collection(observation) => {
                    worker.submit_collection(observation)?
                }
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn status(&self) -> (usize, u64, bool) {
        (
            self.pending.lock().map_or(0, |pending| pending.len()),
            self.dropped.load(Ordering::Relaxed),
            self.worker.read().is_ok_and(|worker| worker.is_some()),
        )
    }

    fn submit_or_buffer(&self, input: PendingMinerInput) -> Result<(), String> {
        if let Some(worker) = self.worker.read().ok().and_then(|worker| worker.clone()) {
            return submit_miner_input(worker.as_ref(), input);
        }
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| "session_miner_bridge_pending_poisoned".to_owned())?;
        if let Some(worker) = self.worker.read().ok().and_then(|worker| worker.clone()) {
            drop(pending);
            return submit_miner_input(worker.as_ref(), input);
        }
        if pending.len() >= MAX_PENDING_MINER_INPUTS {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return Err("session_miner_bridge_capacity_exceeded".to_owned());
        }
        pending.push_back(input);
        Ok(())
    }
}

impl Default for SessionMinerBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionMinerSink for SessionMinerBridge {
    fn submit_frame_with_parity(
        &self,
        frame: RelationFrame,
        runtime_parity_case: Option<RuntimeParityCase>,
    ) -> Result<(), String> {
        self.submit_or_buffer(PendingMinerInput::Frame(frame, runtime_parity_case))
    }

    fn submit_collection(&self, observation: OnlineCollectionObservation) -> Result<(), String> {
        self.submit_or_buffer(PendingMinerInput::Collection(observation))
    }

    fn binds_collection_capture(&self) -> bool {
        true
    }

    fn submit_failure_is_fatal(&self) -> bool {
        self.worker
            .read()
            .ok()
            .and_then(|worker| worker.clone())
            .is_some_and(|worker| worker.submit_failure_is_fatal())
    }
}

fn submit_miner_input(
    worker: &dyn SessionMinerSink,
    input: PendingMinerInput,
) -> Result<(), String> {
    match input {
        PendingMinerInput::Frame(frame, runtime_parity_case) => {
            worker.submit_frame_with_parity(frame, runtime_parity_case)
        }
        PendingMinerInput::Collection(observation) => worker.submit_collection(observation),
    }
}

#[derive(Default)]
pub struct SessionStreamMetrics {
    source_files: AtomicU64,
    finalized_graphs: AtomicU64,
    rejected_overflow_graphs: AtomicU64,
    censored_invalid_session_identities: AtomicU64,
    censored_invalid_utf8_rows: AtomicU64,
    watcher_alive: std::sync::atomic::AtomicBool,
    watcher_events: AtomicU64,
    watcher_last_event_unix: AtomicU64,
}

impl SessionStreamMetrics {
    #[must_use]
    pub fn snapshot(&self) -> (u64, u64, u64, u64, u64, bool, u64, u64) {
        (
            self.source_files.load(Ordering::Relaxed),
            self.finalized_graphs.load(Ordering::Relaxed),
            self.rejected_overflow_graphs.load(Ordering::Relaxed),
            self.censored_invalid_session_identities
                .load(Ordering::Relaxed),
            self.censored_invalid_utf8_rows.load(Ordering::Relaxed),
            self.watcher_alive.load(Ordering::Acquire),
            self.watcher_events.load(Ordering::Relaxed),
            self.watcher_last_event_unix.load(Ordering::Relaxed),
        )
    }
}

pub fn spawn_verified_relation_frame_stream(
    root: PathBuf,
    evidence_root: PathBuf,
    sink: Arc<dyn VerifiedRelationFrameSink>,
    metrics: Arc<SessionStreamMetrics>,
) -> Result<(), String> {
    let evidence = Arc::new(Mutex::new(StreamingEvidenceLedger::open(
        evidence_root,
        EvidencePolicyV1::streaming_bounded(),
    )?));
    let bridge = Arc::new(SessionMinerBridge::new());
    bridge.install_sink(Arc::new(VerifiedRelationFrameSinkAdapter { sink }))?;
    spawn_session_stream(
        root,
        evidence,
        bridge,
        metrics,
        Arc::new(RequestLearningIndex::default()),
    )
}

#[derive(Clone)]
struct CallShape {
    name: String,
    shape: String,
}

#[derive(Clone)]
struct Observation {
    value_sha256: String,
    value_type: AtomValueType,
    render_value: Option<String>,
    selector: ResponseValueSelector,
    tool_kind: String,
    call_shape: String,
    output_sha256: String,
    completion_state: &'static str,
}

#[derive(Default)]
struct SessionState {
    offset: u64,
    // Restart aligns on the last committed row but never ingests it again.
    replay_source_offset: Option<u64>,
    // A partial turn cannot become proof evidence after process restart.
    censor_until_turn_boundary: bool,
    session_identity_pinned: bool,
    session_id_sha256: String,
    calls: BTreeMap<String, CallShape>,
    observations: Vec<Observation>,
    pending_frames: Vec<RelationFrame>,
    pending_action_call_id: Option<String>,
    call_count: u32,
    output_count: u32,
    message_count: u32,
    turn_index: u64,
    session_id: String,
    turn_intent_id: String,
    turn_event_graphs: Vec<CanonicalEventGraph>,
    turn_event_nodes: usize,
    turn_graph_overflow: bool,
    collection_request_item: Option<Value>,
    collection_provider_payload: Option<Value>,
    collection_expected_response: Option<String>,
    collection_completion_reason: Option<TurnCompletionReason>,
    runtime_provider_payload: Option<Value>,
    runtime_provider_payload_overflow: bool,
    latest_plan_call_item: Option<Value>,
    request_phase_atom_ids: Vec<u64>,
    capability_atom_ids: Vec<u64>,
    turn_client_intent_id_sha256: String,
    turn_session_id_sha256: String,
    turn_event_time_unix_nanos: Option<u64>,
    runtime_request_text: String,
    runtime_parity_cases: BTreeMap<String, RuntimeParityCase>,
    current_capture_record: Option<CaptureRecordCommitment>,
    turn_capture_records: Vec<CaptureRecordCommitment>,
}

pub fn spawn_session_stream<L>(
    root: PathBuf,
    evidence: Arc<Mutex<L>>,
    miner: Arc<SessionMinerBridge>,
    metrics: Arc<SessionStreamMetrics>,
    request_learning: Arc<RequestLearningIndex>,
) -> Result<(), String>
where
    L: SessionEvidenceLedger + Send + 'static,
{
    let (sender, receiver) = mpsc::sync_channel(1_024);
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = sender.send(event);
    })
    .map_err(|error| format!("session_watcher_create:{error}"))?;
    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|error| format!("session_watcher_start:{error}"))?;
    let mut states = BTreeMap::new();
    for path in session_files(&root) {
        let source_stream_id = path.to_string_lossy().into_owned();
        let resume_offset = evidence
            .lock()
            .map_err(|_| "evidence_ledger_lock_poisoned".to_owned())?
            .resume_offset(&source_stream_id);
        let offset = resume_offset.unwrap_or_else(|| {
            fs::metadata(&path)
                .map(|metadata| metadata.len())
                .unwrap_or(0)
        });
        match canonical_session_state(&path, offset) {
            Ok(mut state) => {
                // Existing sessions resume without replaying an authority-bearing
                // tail. At most the unfinished turn is sacrificed.
                state.replay_source_offset = resume_offset;
                state.censor_until_turn_boundary = true;
                states.insert(path, state);
            }
            Err(error) => {
                metrics
                    .censored_invalid_session_identities
                    .fetch_add(1, Ordering::Relaxed);
                eprintln!("nando-session-stream identity error: {error}");
            }
        }
    }
    metrics.source_files.store(
        u64::try_from(states.len()).unwrap_or(u64::MAX),
        Ordering::Release,
    );
    thread::Builder::new()
        .name("nando-session-stream".to_owned())
        .spawn(move || {
            let _watcher = watcher;
            metrics.watcher_alive.store(true, Ordering::Release);
            while let Ok(event) = receiver.recv() {
                let Ok(event) = event else { continue };
                metrics.watcher_events.fetch_add(1, Ordering::Relaxed);
                metrics
                    .watcher_last_event_unix
                    .store(unix_now_seconds(), Ordering::Relaxed);
                if !matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Modify(ModifyKind::Data(_))
                ) {
                    continue;
                }
                for path in event.paths {
                    if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
                        continue;
                    }
                    if !states.contains_key(&path) {
                        let source_stream_id = path.to_string_lossy().into_owned();
                        let resume_offset = evidence
                            .lock()
                            .ok()
                            .and_then(|ledger| ledger.resume_offset(&source_stream_id));
                        let offset = resume_offset.unwrap_or(0);
                        let mut state = match canonical_session_state(&path, offset) {
                            Ok(state) => state,
                            Err(_) => {
                                // A create event can arrive before session_meta is fully written.
                                // Censor it and leave the path untracked so a later data event retries.
                                metrics
                                    .censored_invalid_session_identities
                                    .fetch_add(1, Ordering::Relaxed);
                                continue;
                            }
                        };
                        state.replay_source_offset = resume_offset;
                        state.censor_until_turn_boundary = resume_offset.is_some();
                        states.insert(path.clone(), state);
                        metrics.source_files.store(
                            u64::try_from(states.len()).unwrap_or(u64::MAX),
                            Ordering::Release,
                        );
                    }
                    let Some(state) = states.get_mut(&path) else {
                        continue;
                    };
                    let frames = match read_appended_frames(
                        &path,
                        state,
                        SessionReadContext {
                            evidence: &evidence,
                            evidence_graphs: None,
                            miner: Some(miner.as_ref()),
                            direct_collection_miner: None,
                            metrics: &metrics,
                            request_learning: &request_learning,
                        },
                    ) {
                        Ok(frames) => frames,
                        Err(error) => {
                            eprintln!(
                                "nando-session-stream capture error path={}: {error}",
                                path.display()
                            );
                            continue;
                        }
                    };
                    retain_relevant_runtime_parity_cases(state, &frames);
                    if frames.is_empty() {
                        continue;
                    }
                    for frame in frames {
                        let runtime_parity_case =
                            state.runtime_parity_cases.remove(&frame.frame_id_sha256);
                        if let Err(error) =
                            miner.submit_frame_with_parity(frame, runtime_parity_case)
                        {
                            eprintln!("nando-session-stream miner error: {error}");
                            if miner.submit_failure_is_fatal() {
                                metrics.watcher_alive.store(false, Ordering::Release);
                                return;
                            }
                            continue;
                        }
                    }
                }
            }
            metrics.watcher_alive.store(false, Ordering::Release);
        })
        .map_err(|error| format!("session_watcher_thread:{error}"))?;
    Ok(())
}

fn relation_frames_from_session(path: &Path) -> Result<Vec<RelationFrame>, String> {
    training_cases_from_session_at(path, 0).map(|cases| {
        cases
            .into_iter()
            .map(|(frame, _)| frame)
            .collect::<Vec<_>>()
    })
}

fn relation_frames_from_session_at(
    path: &Path,
    start_offset: u64,
) -> Result<Vec<RelationFrame>, String> {
    training_cases_from_session_at(path, start_offset).map(|cases| {
        cases
            .into_iter()
            .map(|(frame, _)| frame)
            .collect::<Vec<_>>()
    })
}

fn training_cases_from_session_at(
    path: &Path,
    start_offset: u64,
) -> Result<Vec<(RelationFrame, Option<RuntimeParityCase>)>, String> {
    training_cases_from_session_range(path, start_offset, None)
}

fn training_cases_from_session_range(
    path: &Path,
    start_offset: u64,
    max_bytes: Option<u64>,
) -> Result<Vec<(RelationFrame, Option<RuntimeParityCase>)>, String> {
    let file = File::open(path).map_err(|error| format!("session_backfill_open:{error}"))?;
    let mut reader = BufReader::new(file);
    if start_offset > 0 {
        reader
            .seek(SeekFrom::Start(start_offset))
            .map_err(|error| format!("session_backfill_seek:{error}"))?;
        let mut partial = Vec::new();
        reader
            .read_until(b'\n', &mut partial)
            .map_err(|error| format!("session_backfill_partial:{error}"))?;
    }
    let mut state = canonical_session_state(path, start_offset)?;
    state.censor_until_turn_boundary = start_offset > 0;
    let mut emitted = Vec::new();
    let mut line = Vec::new();
    let end_offset = max_bytes.map(|bytes| start_offset.saturating_add(bytes));
    loop {
        line.clear();
        let position = reader
            .stream_position()
            .map_err(|error| format!("session_backfill_position:{error}"))?;
        if end_offset.is_some_and(|end| position >= end) {
            break;
        }
        let bytes = reader
            .read_until(b'\n', &mut line)
            .map_err(|error| format!("session_backfill_read:{error}"))?;
        if bytes == 0 {
            break;
        }
        if line.last() != Some(&b'\n') {
            break;
        }
        state.offset = position;
        let Ok(line) = std::str::from_utf8(&line) else {
            state.censor_until_turn_boundary = true;
            continue;
        };
        let Ok(row) = serde_json::from_str::<Value>(line.trim_end()) else {
            continue;
        };
        if state.censor_until_turn_boundary {
            if is_authoritative_turn_boundary(&row) {
                state.censor_until_turn_boundary = false;
                discard_partial_turn(&mut state);
            } else {
                continue;
            }
        }
        begin_turn_identity(&row, &mut state);
        observe_row(&row, &mut state, &mut emitted);
    }
    flush_pending(&mut state, 0, &mut emitted);
    let mut parity_cases = state.runtime_parity_cases;
    Ok(emitted
        .into_iter()
        .map(|frame| {
            let parity = parity_cases.remove(&frame.frame_id_sha256);
            (frame, parity)
        })
        .collect())
}

pub fn verified_relation_frames_from_session(path: &Path) -> Result<Vec<RelationFrame>, String> {
    relation_frames_from_session(path)
}

pub fn verified_training_cases_from_session(
    path: &Path,
) -> Result<Vec<(RelationFrame, Option<RuntimeParityCase>)>, String> {
    training_cases_from_session_at(path, 0)
}

#[derive(Debug, Default)]
pub struct CaptureBoundTrainingCaseBatch {
    pub cases: Vec<(RelationFrame, RuntimeParityCase)>,
    pub files_scanned: usize,
    pub censored_session_identities: BTreeMap<String, String>,
}

/// Replays selected immutable session files through the durable capture owner
/// and returns only parity cases whose transition receipts were bound there.
/// Callers must serialize access to `evidence_root` with the live capture
/// process; this is an offline support-rehydration path, not a serving route.
pub fn verified_capture_bound_training_cases_from_sessions(
    paths: &[PathBuf],
    evidence_root: &Path,
) -> Result<CaptureBoundTrainingCaseBatch, String> {
    let evidence = Arc::new(Mutex::new(StreamingEvidenceLedger::open(
        evidence_root,
        EvidencePolicyV1::streaming_bounded(),
    )?));
    let metrics = Arc::new(SessionStreamMetrics::default());
    let request_learning = Arc::new(RequestLearningIndex::default());
    let mut output = BTreeMap::<String, (RelationFrame, RuntimeParityCase)>::new();
    let mut censored_session_identities = BTreeMap::new();
    let mut paths = paths.to_vec();
    paths.sort();
    paths.dedup();
    let files_scanned = paths.len();

    for path in paths {
        let mut state = match canonical_session_state(&path, 0) {
            Ok(state) => state,
            Err(error) if is_censored_session_identity_error(&error) => {
                censored_session_identities.insert(path.display().to_string(), error);
                continue;
            }
            Err(error) => return Err(format!("capture_bound_session:{}:{error}", path.display())),
        };
        let mut frames = match read_appended_frames(
            &path,
            &mut state,
            SessionReadContext {
                evidence: &evidence,
                evidence_graphs: None,
                miner: None,
                direct_collection_miner: None,
                metrics: &metrics,
                request_learning: &request_learning,
            },
        ) {
            Ok(frames) => frames,
            Err(error) if is_censored_session_identity_error(&error) => {
                censored_session_identities.insert(path.display().to_string(), error);
                continue;
            }
            Err(error) => return Err(format!("capture_bound_session:{}:{error}", path.display())),
        };
        flush_pending(&mut state, 0, &mut frames);
        bind_pending_runtime_parity_cases(&mut state, &evidence)?;
        retain_relevant_runtime_parity_cases(&mut state, &frames);
        for frame in frames {
            let Some(parity) = state.runtime_parity_cases.remove(&frame.frame_id_sha256) else {
                continue;
            };
            let Some(receipt) = parity.capture_receipt.as_ref() else {
                continue;
            };
            let Some(binding) = receipt.transition_binding.as_ref() else {
                continue;
            };
            receipt.validate().map_err(str::to_owned)?;
            if parity.evidence_ref_sha256 != frame.frame_id_sha256
                || binding.frame_id_sha256 != frame.frame_id_sha256
            {
                return Err("capture_bound_training_case_frame_mismatch".to_owned());
            }
            output
                .entry(frame.frame_id_sha256.clone())
                .or_insert((frame, parity));
        }
    }
    Ok(CaptureBoundTrainingCaseBatch {
        cases: output.into_values().collect(),
        files_scanned,
        censored_session_identities,
    })
}

fn is_censored_session_identity_error(error: &str) -> bool {
    matches!(
        error,
        "session_identity_changed"
            | "session_identity_filename_mismatch"
            | "session_identity_missing_meta"
    )
}

pub fn verified_collection_observations_from_session(
    path: &Path,
) -> Result<Vec<OnlineCollectionObservation>, String> {
    let scratch = std::env::temp_dir().join(format!(
        "nando-collection-rehydrate-{}-{}",
        std::process::id(),
        sha256_bytes(path.to_string_lossy().as_bytes())
    ));
    let _ = fs::remove_dir_all(&scratch);
    let result = (|| {
        let evidence = Arc::new(Mutex::new(StreamingEvidenceLedger::open(
            &scratch,
            EvidencePolicyV1::streaming_bounded(),
        )?));
        let collector = CollectionObservationCollector::default();
        let metrics = Arc::new(SessionStreamMetrics::default());
        let request_learning = Arc::new(RequestLearningIndex::default());
        let mut state = canonical_session_state(path, 0)?;
        read_appended_frames(
            path,
            &mut state,
            SessionReadContext {
                evidence: &evidence,
                evidence_graphs: None,
                miner: Some(&collector),
                direct_collection_miner: None,
                metrics: &metrics,
                request_learning: &request_learning,
            },
        )?;
        collector
            .observations
            .into_inner()
            .map_err(|_| "collection_observation_collector_poisoned".to_owned())
    })();
    let _ = fs::remove_dir_all(&scratch);
    result
}

pub fn verified_session_identity_sha256_candidates(
    path: &Path,
) -> Result<BTreeSet<String>, String> {
    let session_id = canonical_session_id(path)?;
    let path_identity = path.to_string_lossy();
    // Path commitments are accepted only while rehydrating legacy receipts.
    // Live capture is pinned exclusively to the verified session_meta identity.
    Ok(BTreeSet::from([
        evidence_session_id_sha256(&session_id),
        sha256_bytes(session_id.as_bytes()),
        evidence_session_id_sha256(&path_identity),
        sha256_bytes(path_identity.as_bytes()),
    ]))
}

pub fn verified_relation_frames_from_session_tail(
    path: &Path,
    max_bytes: u64,
) -> Result<Vec<RelationFrame>, String> {
    let length = fs::metadata(path)
        .map_err(|error| format!("session_backfill_metadata:{error}"))?
        .len();
    relation_frames_from_session_at(path, length.saturating_sub(max_bytes))
}

pub fn verified_training_cases_from_session_tail(
    path: &Path,
    max_bytes: u64,
) -> Result<Vec<(RelationFrame, Option<RuntimeParityCase>)>, String> {
    let length = fs::metadata(path)
        .map_err(|error| format!("session_backfill_metadata:{error}"))?
        .len();
    training_cases_from_session_at(path, length.saturating_sub(max_bytes))
}

pub fn verified_training_cases_from_session_head(
    path: &Path,
    max_bytes: u64,
) -> Result<Vec<(RelationFrame, Option<RuntimeParityCase>)>, String> {
    training_cases_from_session_range(path, 0, Some(max_bytes))
}

pub fn verified_write_stdin_training_cases_from_session(
    path: &Path,
) -> Result<Vec<(RelationFrame, Option<RuntimeParityCase>)>, String> {
    verified_write_stdin_training_cases_from_session_matching(path, None)
}

pub fn verified_write_stdin_training_cases_from_session_for_signatures(
    path: &Path,
    target_signatures: &BTreeSet<String>,
) -> Result<Vec<(RelationFrame, Option<RuntimeParityCase>)>, String> {
    verified_write_stdin_training_cases_from_session_matching(path, Some(target_signatures))
}

fn verified_write_stdin_training_cases_from_session_matching(
    path: &Path,
    target_signatures: Option<&BTreeSet<String>>,
) -> Result<Vec<(RelationFrame, Option<RuntimeParityCase>)>, String> {
    const CONTEXT_ROWS: usize = 64;
    const CONTEXT_BYTES: usize = 2 * 1024 * 1024;
    const ACTIVE_ROWS: usize = 128;
    const ACTIVE_BYTES: usize = 4 * 1024 * 1024;
    const MAX_CASES_PER_SIGNATURE: usize = 64;

    let mut recent = VecDeque::<(u64, Vec<u8>)>::new();
    let mut recent_bytes = 0_usize;
    let mut active = None::<Vec<(u64, Vec<u8>)>>;
    let mut active_bytes = 0_usize;
    let mut session_id_sha256 = sha256_bytes(path.to_string_lossy().as_bytes());
    let mut output = BTreeMap::<String, (RelationFrame, Option<RuntimeParityCase>)>::new();
    let mut selected_by_signature = BTreeMap::<String, usize>::new();
    visit_complete_lines(path, |position, line| {
        let prefix = &line[..line.len().min(2 * 1024)];
        let session_meta = contains_bytes(prefix, b"\"type\":\"session_meta\"");
        let turn_context = contains_bytes(prefix, b"\"type\":\"turn_context\"");
        let user_message = contains_bytes(prefix, b"\"type\":\"user_message\"");
        let function_call = contains_bytes(prefix, b"\"type\":\"function_call\"");
        let function_call_output = contains_bytes(prefix, b"\"type\":\"function_call_output\"");
        let custom_tool_call = contains_bytes(prefix, b"\"type\":\"custom_tool_call\"");
        let custom_tool_call_output =
            contains_bytes(prefix, b"\"type\":\"custom_tool_call_output\"");
        let token_count = contains_bytes(prefix, b"\"type\":\"token_count\"");
        let task_started = contains_bytes(prefix, b"\"type\":\"task_started\"");
        let task_complete = contains_bytes(prefix, b"\"type\":\"task_complete\"");
        if session_meta
            && let Ok(row) = serde_json::from_slice::<Value>(line)
            && let Some(id) = row
                .get("payload")
                .and_then(|payload| payload.get("id"))
                .and_then(Value::as_str)
        {
            session_id_sha256 = sha256_bytes(id.as_bytes());
        }
        let turn_start = turn_context || user_message;
        if turn_start {
            if let Some(rows) = active.take() {
                append_write_stdin_window(
                    path,
                    &session_id_sha256,
                    rows,
                    target_signatures,
                    MAX_CASES_PER_SIGNATURE,
                    &mut selected_by_signature,
                    &mut output,
                )?;
                active_bytes = 0;
            }
            recent.clear();
            recent_bytes = 0;
        }
        let relevant = session_meta
            || turn_context
            || user_message
            || function_call
            || function_call_output
            || custom_tool_call
            || custom_tool_call_output
            || token_count
            || task_started
            || task_complete;
        let is_write_stdin_call = if (function_call || custom_tool_call)
            && contains_bytes(line, b"write_stdin")
        {
            let signatures = write_stdin_call_signatures(line);
            !signatures.is_empty()
                && target_signatures
                    .is_none_or(|targets| signatures.iter().any(|value| targets.contains(value)))
        } else {
            false
        };
        if is_write_stdin_call && active.is_none() {
            active_bytes = recent_bytes;
            active = Some(recent.iter().cloned().collect());
        }
        if relevant {
            let line = line.to_vec();
            if let Some(rows) = active.as_mut() {
                active_bytes = active_bytes.saturating_add(line.len());
                rows.push((position, line.clone()));
            }
            recent_bytes = recent_bytes.saturating_add(line.len());
            recent.push_back((position, line.clone()));
            while recent.len() > CONTEXT_ROWS || recent_bytes > CONTEXT_BYTES {
                let Some(removed) = recent.pop_front() else {
                    break;
                };
                recent_bytes = recent_bytes.saturating_sub(removed.1.len());
            }
        }
        if active.as_ref().is_some_and(|rows| {
            token_count || rows.len() >= ACTIVE_ROWS || active_bytes >= ACTIVE_BYTES
        }) && let Some(rows) = active.take()
        {
            append_write_stdin_window(
                path,
                &session_id_sha256,
                rows,
                target_signatures,
                MAX_CASES_PER_SIGNATURE,
                &mut selected_by_signature,
                &mut output,
            )?;
            active_bytes = 0;
        }
        let evidence_complete = target_signatures.map_or_else(
            || output.len() >= MAX_CASES_PER_SIGNATURE,
            |targets| {
                !targets.is_empty()
                    && targets.iter().all(|signature| {
                        selected_by_signature.get(signature).copied().unwrap_or(0)
                            >= MAX_CASES_PER_SIGNATURE
                    })
            },
        );
        Ok(!evidence_complete)
    })?;
    if let Some(rows) = active {
        append_write_stdin_window(
            path,
            &session_id_sha256,
            rows,
            target_signatures,
            MAX_CASES_PER_SIGNATURE,
            &mut selected_by_signature,
            &mut output,
        )?;
    }
    Ok(output.into_values().collect())
}

fn visit_complete_lines(
    path: &Path,
    mut visit: impl FnMut(u64, &[u8]) -> Result<bool, String>,
) -> Result<(), String> {
    const BUFFER_BYTES: usize = 8 * 1024 * 1024;

    let file = File::open(path).map_err(|error| format!("session_sparse_open:{error}"))?;
    let mut reader = BufReader::with_capacity(BUFFER_BYTES, file);
    let mut absolute_offset = 0_u64;
    let mut partial = Vec::new();
    let mut partial_offset = 0_u64;
    loop {
        let consumed = {
            let buffer = reader
                .fill_buf()
                .map_err(|error| format!("session_sparse_read:{error}"))?;
            if buffer.is_empty() {
                break;
            }
            let mut line_start = 0_usize;
            for newline in memchr_iter(b'\n', buffer) {
                let line_end = newline.saturating_add(1);
                let keep_scanning = if partial.is_empty() {
                    visit(
                        absolute_offset.saturating_add(line_start as u64),
                        &buffer[line_start..line_end],
                    )?
                } else {
                    partial.extend_from_slice(&buffer[line_start..line_end]);
                    let keep_scanning = visit(partial_offset, &partial)?;
                    partial.clear();
                    keep_scanning
                };
                if !keep_scanning {
                    return Ok(());
                }
                line_start = line_end;
            }
            if line_start < buffer.len() {
                if partial.is_empty() {
                    partial_offset = absolute_offset.saturating_add(line_start as u64);
                }
                partial.extend_from_slice(&buffer[line_start..]);
            }
            buffer.len()
        };
        reader.consume(consumed);
        absolute_offset = absolute_offset.saturating_add(consumed as u64);
    }
    Ok(())
}

fn append_write_stdin_window(
    path: &Path,
    session_id_sha256: &str,
    rows: Vec<(u64, Vec<u8>)>,
    target_signatures: Option<&BTreeSet<String>>,
    cases_per_signature: usize,
    selected_by_signature: &mut BTreeMap<String, usize>,
    output: &mut BTreeMap<String, (RelationFrame, Option<RuntimeParityCase>)>,
) -> Result<(), String> {
    let mut state = SessionState {
        session_id: path.to_string_lossy().into_owned(),
        session_id_sha256: session_id_sha256.to_owned(),
        ..SessionState::default()
    };
    let mut emitted = Vec::new();
    for (offset, raw) in rows {
        let Ok(row) = serde_json::from_slice::<Value>(&raw) else {
            continue;
        };
        state.offset = offset;
        observe_row(&row, &mut state, &mut emitted);
    }
    flush_pending(&mut state, 0, &mut emitted);
    let mut parity_cases = state.runtime_parity_cases;
    for frame in emitted {
        if matches!(
            teacher_action_symbol(&frame).as_str(),
            "function:write_stdin" | "custom_tool:exec/write_stdin"
        ) {
            let Some(signature) = teacher_program_signature(&frame) else {
                continue;
            };
            if target_signatures.is_some_and(|targets| !targets.contains(&signature))
                || selected_by_signature.get(&signature).copied().unwrap_or(0)
                    >= cases_per_signature
            {
                continue;
            }
            let parity = parity_cases.remove(&frame.frame_id_sha256);
            *selected_by_signature.entry(signature).or_default() += 1;
            output.insert(frame.frame_id_sha256.clone(), (frame, parity));
        }
    }
    Ok(())
}

fn write_stdin_call_signatures(line: &[u8]) -> BTreeSet<String> {
    let mut output = BTreeSet::new();
    let Ok(row) = serde_json::from_slice::<Value>(line) else {
        return output;
    };
    if row.get("type").and_then(Value::as_str) != Some("response_item") {
        return output;
    }
    let Some(payload) = row.get("payload").and_then(Value::as_object) else {
        return output;
    };
    let Some(outer_name) = payload.get("name").and_then(Value::as_str) else {
        return output;
    };
    let (action_name, arguments, transport_atoms) =
        match payload.get("type").and_then(Value::as_str) {
            Some("function_call") if outer_name == "write_stdin" => {
                let Some(arguments) = direct_call_arguments(payload) else {
                    return output;
                };
                (
                    outer_name.to_owned(),
                    arguments,
                    vec![RelationAtom::ActionFunction {
                        value: outer_name.to_owned(),
                    }],
                )
            }
            Some("custom_tool_call") => {
                let Some(custom) = payload
                    .get("input")
                    .or_else(|| payload.get("arguments"))
                    .and_then(Value::as_str)
                    .and_then(parse_custom_tool_source)
                else {
                    return output;
                };
                if custom.inner_tool_name != "write_stdin" {
                    return output;
                }
                let atoms = vec![
                    RelationAtom::ActionCustomTool {
                        value: outer_name.to_owned(),
                    },
                    RelationAtom::ActionInnerTool {
                        value: custom.inner_tool_name.clone(),
                    },
                    custom.projection.clone(),
                ];
                (custom.inner_tool_name, custom.arguments, atoms)
            }
            _ => return output,
        };
    if action_name != "write_stdin" {
        return output;
    }
    for (role_name, role_value) in &arguments {
        let Some(role_type) = action_argument_value_type(role_value) else {
            continue;
        };
        let mut atoms = transport_atoms.clone();
        atoms.push(RelationAtom::ActionRoleArgument {
            name: role_name.clone(),
            slot_id: 0,
            value_type: Some(role_type),
        });
        for (name, value) in &arguments {
            if name == role_name {
                continue;
            }
            let atom = match value {
                Value::Number(number) => {
                    number
                        .as_u64()
                        .map(|value| RelationAtom::ActionIntegerArgument {
                            name: name.clone(),
                            value,
                        })
                }
                Value::String(value) => Some(RelationAtom::ActionStringArgument {
                    name: name.clone(),
                    value: value.clone(),
                }),
                Value::Bool(value) => Some(RelationAtom::ActionBooleanArgument {
                    name: name.clone(),
                    value: *value,
                }),
                _ => None,
            };
            if let Some(atom) = atom {
                atoms.push(atom);
            }
        }
        if let Some(signature) = teacher_program_signature_from_action_atoms(&atoms) {
            output.insert(signature);
        }
    }
    output
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    memchr::memmem::find(haystack, needle).is_some()
}

fn session_files(root: &Path) -> Vec<PathBuf> {
    let mut output = Vec::new();
    let mut pending = vec![root.to_owned()];
    while let Some(path) = pending.pop() {
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
                output.push(path);
            }
        }
    }
    output
}

fn canonical_session_state(path: &Path, offset: u64) -> Result<SessionState, String> {
    let session_id = canonical_session_id(path)?;
    Ok(SessionState {
        offset,
        session_identity_pinned: true,
        session_id_sha256: sha256_bytes(session_id.as_bytes()),
        session_id,
        ..SessionState::default()
    })
}

fn canonical_session_id(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|error| format!("session_identity_open:{error}"))?;
    let mut reader = BufReader::new(file).take(MAX_SESSION_META_BYTES);
    let mut line = Vec::new();
    loop {
        line.clear();
        let bytes = reader
            .read_until(b'\n', &mut line)
            .map_err(|error| format!("session_identity_read:{error}"))?;
        if bytes == 0 {
            break;
        }
        if contains_bytes(&line, b"\"type\":\"session_meta\"")
            && let Ok(row) = serde_json::from_slice::<Value>(&line)
            && let Some(session_id) = session_id_from_meta(&row)
        {
            if session_id.is_empty() {
                return Err("session_identity_empty".to_owned());
            }
            if let Some(filename_id) = rollout_session_id_from_path(path)
                && filename_id != session_id
            {
                return Err("session_identity_filename_mismatch".to_owned());
            }
            return Ok(session_id.to_owned());
        }
    }
    Err("session_identity_missing_meta".to_owned())
}

fn rollout_session_id_from_path(path: &Path) -> Option<&str> {
    let filename = path.file_name()?.to_str()?;
    let stem = filename.strip_prefix("rollout-")?.strip_suffix(".jsonl")?;
    let candidate = stem.get(stem.len().checked_sub(36)?..)?;
    (candidate.len() == 36
        && candidate
            .bytes()
            .enumerate()
            .all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                _ => byte.is_ascii_hexdigit(),
            }))
    .then_some(candidate)
}

struct SessionReadContext<'a, L> {
    evidence: &'a Arc<Mutex<L>>,
    evidence_graphs: Option<&'a Arc<Mutex<DeterministicEvidenceGraphStore>>>,
    miner: Option<&'a dyn SessionMinerSink>,
    direct_collection_miner: Option<&'a Arc<Mutex<OnlineCollectionMiner>>>,
    metrics: &'a Arc<SessionStreamMetrics>,
    request_learning: &'a Arc<RequestLearningIndex>,
}

fn read_appended_frames<L: SessionEvidenceLedger>(
    path: &Path,
    state: &mut SessionState,
    context: SessionReadContext<'_, L>,
) -> Result<Vec<RelationFrame>, String> {
    let SessionReadContext {
        evidence,
        evidence_graphs,
        miner,
        direct_collection_miner,
        metrics,
        request_learning,
    } = context;
    let bind_collection_capture = miner.is_some_and(|miner| miner.binds_collection_capture());
    let file = File::open(path).map_err(|error| format!("session_open:{error}"))?;
    let length = file
        .metadata()
        .map_err(|error| format!("session_metadata:{error}"))?
        .len();
    if state.offset > length {
        *state = canonical_session_state(path, length)?;
        state.censor_until_turn_boundary = true;
        return Ok(Vec::new());
    }
    let (mut reader, aligned_offset) = aligned_session_reader(file, state.offset)?;
    state.offset = aligned_offset;
    let mut line = Vec::new();
    let mut emitted = Vec::new();
    loop {
        line.clear();
        let position = reader
            .stream_position()
            .map_err(|error| error.to_string())?;
        let bytes = reader
            .read_until(b'\n', &mut line)
            .map_err(|error| error.to_string())?;
        if bytes == 0 {
            state.offset = position;
            break;
        }
        if line.last() != Some(&b'\n') {
            state.offset = position;
            break;
        }
        state.offset = position.saturating_add(bytes as u64);
        let Ok(line) = std::str::from_utf8(&line) else {
            // A malformed row invalidates its partial turn, not every later
            // turn in the same append-only session.
            if state.replay_source_offset == Some(position) {
                state.replay_source_offset = None;
            }
            state.censor_until_turn_boundary = true;
            metrics
                .censored_invalid_utf8_rows
                .fetch_add(1, Ordering::Relaxed);
            continue;
        };
        let parsed = serde_json::from_str::<Value>(line.trim_end()).ok();
        if state.replay_source_offset == Some(position) {
            state.replay_source_offset = None;
            continue;
        }
        if state.censor_until_turn_boundary {
            if parsed.as_ref().is_some_and(is_authoritative_turn_boundary) {
                state.censor_until_turn_boundary = false;
                discard_partial_turn(state);
            } else {
                continue;
            }
        }
        if parsed.as_ref().is_some_and(is_turn_start)
            && let Some(observation) = take_collection_observation(
                state,
                evidence,
                evidence_graphs,
                metrics,
                0,
                bind_collection_capture,
            )?
        {
            submit_collection_observation(observation, miner, direct_collection_miner)?;
        }
        if let Some(turn_boundary) = parsed
            .as_ref()
            .filter(|row| is_authoritative_turn_boundary(row))
        {
            finalize_turn_evidence_graph(state, evidence_graphs, metrics)?;
            begin_turn_identity(turn_boundary, state);
        }
        if let Some(session_id) = parsed.as_ref().and_then(session_id_from_meta) {
            if state.session_identity_pinned && state.session_id != session_id {
                return Err("session_identity_changed".to_owned());
            }
            state.session_id = session_id.to_owned();
            state.session_id_sha256 = sha256_bytes(session_id.as_bytes());
            state.session_identity_pinned = true;
        }
        let event_time_unix_nanos = parsed.as_ref().and_then(event_time_unix_nanos);
        let event_id = parsed
            .as_ref()
            .and_then(event_id_from_row)
            .unwrap_or_else(|| sha256_bytes(line.trim_end().as_bytes()));
        let envelope = RawEvidenceEnvelope {
            source_stream_id: path.to_string_lossy().into_owned(),
            source_offset: position,
            event_id,
            session_id: state.session_id.clone(),
            client_intent_id: (!state.turn_intent_id.is_empty())
                .then(|| state.turn_intent_id.clone()),
            call_id: parsed.as_ref().and_then(call_id_from_row),
            output_ordinal: parsed
                .as_ref()
                .is_some_and(is_tool_output)
                .then(|| state.output_count.saturating_add(1)),
            event_time_unix_nanos,
            schema_version: 1,
            payload: line.trim_end().as_bytes().to_vec(),
        };
        let record = evidence
            .lock()
            .map_err(|_| "evidence_ledger_lock_poisoned".to_owned())?
            .ingest_session_event(envelope)?;
        state.current_capture_record = Some(CaptureRecordCommitment {
            sequence: record.sequence,
            record_sha256: record.record_sha256.clone(),
        });
        if !parsed.as_ref().is_some_and(is_turn_start) {
            state.turn_capture_records.push(
                state
                    .current_capture_record
                    .clone()
                    .expect("capture record"),
            );
        }
        if state.turn_index > 0
            && parsed.as_ref().is_some_and(is_evidence_graph_event)
            && let EvidenceIngestOutcome::Normalized { graph } = record.outcome
        {
            state.turn_client_intent_id_sha256 =
                graph.client_intent_id_sha256.clone().unwrap_or_default();
            state.turn_session_id_sha256 = graph.session_id_sha256.clone();
            state.turn_event_time_unix_nanos = match graph.event_time {
                nando_response_actor::EvidenceEventTime::Known { unix_nanos } => Some(unix_nanos),
                nando_response_actor::EvidenceEventTime::Unknown => None,
            };
            let next_nodes = state.turn_event_nodes.saturating_add(graph.nodes.len());
            if state.turn_event_graphs.len() >= MAX_TURN_EVIDENCE_EVENTS
                || next_nodes > MAX_TURN_EVIDENCE_NODES
            {
                state.turn_event_graphs.clear();
                state.turn_event_nodes = 0;
                state.turn_graph_overflow = true;
            } else if !state.turn_graph_overflow {
                state.turn_event_nodes = next_nodes;
                state.turn_event_graphs.push(graph);
            }
        }
        let Some(row) = parsed else { continue };
        let learning_atoms = request_learning.lookup(
            &state.session_id_sha256,
            &state.turn_client_intent_id_sha256,
        );
        if !learning_atoms.request_phase_atom_ids.is_empty() {
            state
                .request_phase_atom_ids
                .extend(learning_atoms.request_phase_atom_ids);
            state.request_phase_atom_ids.sort_unstable();
            state.request_phase_atom_ids.dedup();
        }
        if !learning_atoms.capability_atom_ids.is_empty() {
            state.capability_atom_ids = learning_atoms.capability_atom_ids;
        }
        observe_row(&row, state, &mut emitted);
        bind_pending_runtime_parity_cases(state, evidence)?;
        if is_token_count(&row)
            && let Some(observation) = take_collection_observation(
                state,
                evidence,
                evidence_graphs,
                metrics,
                token_count_from_row(&row),
                bind_collection_capture,
            )?
        {
            submit_collection_observation(observation, miner, direct_collection_miner)?;
        }
    }
    Ok(emitted)
}

fn aligned_session_reader(
    mut file: File,
    requested_offset: u64,
) -> Result<(BufReader<File>, u64), String> {
    if requested_offset == 0 {
        return Ok((BufReader::new(file), 0));
    }
    file.seek(SeekFrom::Start(requested_offset.saturating_sub(1)))
        .map_err(|error| format!("session_alignment_seek:{error}"))?;
    let mut previous = [0_u8; 1];
    file.read_exact(&mut previous)
        .map_err(|error| format!("session_alignment_read:{error}"))?;
    file.seek(SeekFrom::Start(requested_offset))
        .map_err(|error| format!("session_seek:{error}"))?;
    let mut reader = BufReader::new(file);
    if previous[0] == b'\n' {
        return Ok((reader, requested_offset));
    }
    let mut partial = Vec::new();
    reader
        .read_until(b'\n', &mut partial)
        .map_err(|error| format!("session_alignment_partial:{error}"))?;
    let aligned_offset = reader
        .stream_position()
        .map_err(|error| format!("session_alignment_position:{error}"))?;
    Ok((reader, aligned_offset))
}

fn is_token_count(row: &Value) -> bool {
    row.get("type").and_then(Value::as_str) == Some("event_msg")
        && row
            .get("payload")
            .and_then(|payload| payload.get("type"))
            .and_then(Value::as_str)
            == Some("token_count")
}

fn is_turn_start(row: &Value) -> bool {
    if row.get("type").and_then(Value::as_str) == Some("turn_context") {
        return true;
    }
    let row_type = row.get("type").and_then(Value::as_str).unwrap_or("");
    let payload = row.get("payload").and_then(Value::as_object);
    let payload_type = payload
        .and_then(|payload| payload.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("");
    (row_type == "event_msg" && payload_type == "user_message")
        || (row_type == "response_item"
            && payload_type == "message"
            && payload
                .and_then(|payload| payload.get("role"))
                .and_then(Value::as_str)
                == Some("user"))
}

fn submit_collection_observation(
    observation: OnlineCollectionObservation,
    miner: Option<&dyn SessionMinerSink>,
    direct_collection_miner: Option<&Arc<Mutex<OnlineCollectionMiner>>>,
) -> Result<(), String> {
    if let Some(miner) = miner {
        miner.submit_collection(observation)
    } else if let Some(collection_miner) = direct_collection_miner {
        collection_miner
            .lock()
            .map_err(|_| "online_collection_miner_lock_poisoned".to_owned())?
            .observe(observation)
    } else {
        Ok(())
    }
}

fn token_count_from_row(row: &Value) -> u64 {
    row.get("payload")
        .and_then(|payload| payload.get("info"))
        .and_then(|info| info.get("last_token_usage"))
        .and_then(|usage| usage.get("input_tokens"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn take_collection_observation<L: SessionEvidenceLedger>(
    state: &mut SessionState,
    evidence: &Arc<Mutex<L>>,
    evidence_graphs: Option<&Arc<Mutex<DeterministicEvidenceGraphStore>>>,
    metrics: &Arc<SessionStreamMetrics>,
    estimated_input_tokens: u64,
    bind_collection_capture: bool,
) -> Result<Option<OnlineCollectionObservation>, String> {
    let Some(completion_reason) = state.collection_completion_reason.take() else {
        return Ok(None);
    };
    let Some(expected_response) = state.collection_expected_response.take() else {
        return Ok(None);
    };
    let provider_payload = state.collection_provider_payload.take();
    let Some(provider_payload) = provider_payload else {
        return Ok(None);
    };
    if state.turn_graph_overflow
        || state.turn_event_graphs.is_empty()
        || state.turn_client_intent_id_sha256.is_empty()
        || state.turn_session_id_sha256.is_empty()
    {
        return Ok(None);
    }
    let graph = EvidenceGraphBuilder::build(&state.turn_event_graphs, live_evidence_graph_policy())
        .map_err(str::to_owned)?;
    let evidence_graph_sha256 = graph.graph_sha256.clone();
    if let Some(evidence_graphs) = evidence_graphs {
        evidence_graphs
            .lock()
            .map_err(|_| "evidence_graph_store_lock_poisoned".to_owned())?
            .append(graph)?;
    }
    metrics.finalized_graphs.fetch_add(1, Ordering::Relaxed);
    let capture_binding = if bind_collection_capture {
        let receipt = current_capture_receipt(state)
            .ok_or_else(|| "collection_capture_receipt_missing".to_owned())?;
        // Only the live capture owner may mint this compact binding. Offline
        // replay emits observations without it and therefore remains shadow.
        let binding = evidence
            .lock()
            .map_err(|_| "evidence_ledger_lock_poisoned".to_owned())?
            .bind_transition(&evidence_graph_sha256, &receipt)?;
        binding.validate(&receipt).map_err(str::to_owned)?;
        Some(binding)
    } else {
        None
    };
    let example = CompletedTurnExample::final_response_with_reason(
        provider_payload,
        expected_response,
        completion_reason,
    )
    .map_err(str::to_owned)?;
    Ok(Some(OnlineCollectionObservation {
        evidence_graph_sha256,
        client_intent_id_sha256: state.turn_client_intent_id_sha256.clone(),
        session_id_sha256: state.turn_session_id_sha256.clone(),
        event_time_unix_nanos: state.turn_event_time_unix_nanos,
        estimated_input_tokens,
        capture_binding,
        example: example.into_synthesis_example(),
    }))
}

fn finalize_turn_evidence_graph(
    state: &mut SessionState,
    evidence_graphs: Option<&Arc<Mutex<DeterministicEvidenceGraphStore>>>,
    metrics: &Arc<SessionStreamMetrics>,
) -> Result<(), String> {
    if state.turn_graph_overflow {
        state.turn_event_graphs.clear();
        state.turn_event_nodes = 0;
        state.turn_graph_overflow = false;
        metrics
            .rejected_overflow_graphs
            .fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }
    if state.turn_event_graphs.is_empty() {
        return Ok(());
    }
    let graph = EvidenceGraphBuilder::build(&state.turn_event_graphs, live_evidence_graph_policy())
        .map_err(str::to_owned)?;
    if let Some(evidence_graphs) = evidence_graphs {
        evidence_graphs
            .lock()
            .map_err(|_| "evidence_graph_store_lock_poisoned".to_owned())?
            .append(graph)?;
    }
    state.turn_event_graphs.clear();
    state.turn_event_nodes = 0;
    metrics.finalized_graphs.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

fn session_id_from_meta(row: &Value) -> Option<&str> {
    (row.get("type").and_then(Value::as_str) == Some("session_meta"))
        .then(|| row.get("payload")?.get("id")?.as_str())
        .flatten()
}

fn bind_pending_runtime_parity_cases<L: SessionEvidenceLedger>(
    state: &mut SessionState,
    evidence: &Arc<Mutex<L>>,
) -> Result<(), String> {
    let pending = state
        .runtime_parity_cases
        .iter()
        .filter_map(|(frame_id, parity)| {
            let receipt = parity.capture_receipt.as_ref()?;
            receipt
                .transition_binding
                .is_none()
                .then(|| (frame_id.clone(), receipt.clone()))
        })
        .collect::<Vec<_>>();
    for (frame_id, receipt) in pending {
        // Only the capture owner may bind a runtime case to archived evidence.
        // Learning and admission can carry or verify this receipt, never mint it.
        let binding = evidence
            .lock()
            .map_err(|_| "evidence_ledger_lock_poisoned".to_owned())?
            .bind_transition(&frame_id, &receipt)?;
        state
            .runtime_parity_cases
            .get_mut(&frame_id)
            .and_then(|case| case.capture_receipt.as_mut())
            .ok_or_else(|| "runtime_parity_capture_receipt_disappeared".to_owned())?
            .bind_transition(binding)
            .map_err(str::to_owned)?;
    }
    Ok(())
}

fn turn_intent_id_from_context(row: &Value) -> Option<&str> {
    (row.get("type").and_then(Value::as_str) == Some("turn_context"))
        .then(|| row.get("payload")?.get("turn_id")?.as_str())
        .flatten()
        .filter(|value| !value.is_empty() && value.len() <= 256)
}

fn begin_turn_identity(row: &Value, state: &mut SessionState) {
    if !is_authoritative_turn_boundary(row) {
        return;
    }
    state.turn_index = state.turn_index.saturating_add(1);
    state.turn_intent_id = turn_intent_id_from_context(row)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{}:turn:{}", state.session_id, state.turn_index));
    state.turn_client_intent_id_sha256 = sha256_bytes(state.turn_intent_id.as_bytes());
    state.turn_session_id_sha256 = state.session_id_sha256.clone();
    state.turn_event_time_unix_nanos = event_time_unix_nanos(row);
}

fn event_id_from_row(row: &Value) -> Option<String> {
    row.get("payload")
        .and_then(|payload| payload.get("id").or_else(|| payload.get("call_id")))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn call_id_from_row(row: &Value) -> Option<String> {
    row.get("payload")
        .and_then(|payload| payload.get("call_id"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn event_time_unix_nanos(row: &Value) -> Option<u64> {
    let timestamp = row.get("timestamp")?.as_str()?;
    let parsed = OffsetDateTime::parse(timestamp, &Rfc3339).ok()?;
    u64::try_from(parsed.unix_timestamp_nanos()).ok()
}

fn is_authoritative_turn_boundary(row: &Value) -> bool {
    row.get("type").and_then(Value::as_str) == Some("turn_context")
}

fn is_evidence_graph_event(row: &Value) -> bool {
    let row_type = row.get("type").and_then(Value::as_str).unwrap_or("");
    let payload_type = row
        .get("payload")
        .and_then(|payload| payload.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("");
    row_type == "turn_context"
        || (row_type == "response_item"
            && matches!(
                payload_type,
                "function_call"
                    | "custom_tool_call"
                    | "function_call_output"
                    | "custom_tool_call_output"
                    | "message"
            ))
        || (row_type == "event_msg"
            && matches!(
                payload_type,
                "agent_message" | "token_count" | "user_message"
            ))
}

const fn live_evidence_graph_policy() -> EvidenceGraphPolicy {
    EvidenceGraphPolicy {
        max_events: MAX_TURN_EVIDENCE_EVENTS,
        max_atoms: 32_768,
    }
}

fn is_tool_output(row: &Value) -> bool {
    row.get("type").and_then(Value::as_str) == Some("response_item")
        && matches!(
            row.get("payload")
                .and_then(|value| value.get("type"))
                .and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        )
}

fn observe_row(row: &Value, state: &mut SessionState, emitted: &mut Vec<RelationFrame>) {
    let row_type = row.get("type").and_then(Value::as_str).unwrap_or("");
    let Some(payload) = row.get("payload").and_then(Value::as_object) else {
        return;
    };
    let payload_type = payload.get("type").and_then(Value::as_str).unwrap_or("");
    if row_type == "session_meta" {
        if let Some(id) = payload.get("id").and_then(Value::as_str) {
            state.session_id_sha256 = sha256_bytes(id.as_bytes());
        }
        return;
    }
    if row_type == "turn_context"
        || (row_type == "event_msg" && payload_type == "context_compacted")
    {
        flush_pending(state, 0, emitted);
        reset_turn(state);
        return;
    }
    if let Some(text) = bounded_runtime_request_text(row) {
        flush_pending(state, 0, emitted);
        reset_turn(state);
        state.request_phase_atom_ids = nando_response_actor::request_phase_atom_ids(&text);
        state.collection_request_item = collection_request_item(&text);
        state.collection_provider_payload = state
            .collection_request_item
            .clone()
            .map(|request| serde_json::json!({"input":[request]}));
        state.runtime_request_text = text;
        return;
    }
    if row_type == "event_msg" && payload_type == "token_count" {
        let tokens = payload
            .get("info")
            .and_then(Value::as_object)
            .and_then(|info| info.get("last_token_usage"))
            .and_then(Value::as_object)
            .and_then(|usage| usage.get("input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        flush_pending(state, tokens, emitted);
        return;
    }
    if row_type == "event_msg"
        && payload_type == "agent_message"
        && payload.get("phase").and_then(Value::as_str) == Some("final_answer")
    {
        let text = payload.get("message").and_then(Value::as_str).unwrap_or("");
        remember_collection_response(text, state);
        mark_completed_turn_if_settled(state);
        state.pending_frames = assistant_frames(row, text, state);
        state.message_count = state.message_count.saturating_add(1);
        return;
    }
    if row_type != "response_item" {
        return;
    }
    match payload_type {
        "function_call" | "custom_tool_call" => {
            flush_pending(state, 0, emitted);
            state.pending_frames = action_frames(row, payload, state);
            state.pending_action_call_id = payload
                .get("call_id")
                .and_then(Value::as_str)
                .map(str::to_owned);
            remember_call(payload, payload_type, state);
        }
        "function_call_output" | "custom_tool_call_output" => remember_output(payload, state),
        "message"
            if payload.get("role").and_then(Value::as_str) == Some("assistant")
                && payload.get("phase").and_then(Value::as_str) == Some("final_answer") =>
        {
            flush_pending(state, 0, emitted);
            let text = message_text(payload.get("content"));
            remember_collection_response(&text, state);
            mark_completed_turn_if_settled(state);
            state.pending_frames = assistant_frames(row, &text, state);
            state.message_count = state.message_count.saturating_add(1);
        }
        _ => {}
    }
}

fn reset_turn(state: &mut SessionState) {
    state.calls.clear();
    // Completed turns can contain large payload-derived allocations. Replace the
    // buffers so inactive session states do not retain their peak capacity.
    state.observations = Vec::new();
    state.pending_frames = Vec::new();
    state.pending_action_call_id = None;
    state.call_count = 0;
    state.output_count = 0;
    state.message_count = 0;
    state.collection_request_item = None;
    state.collection_provider_payload = None;
    state.collection_expected_response = None;
    state.collection_completion_reason = None;
    state.runtime_provider_payload = None;
    state.runtime_provider_payload_overflow = false;
    state.latest_plan_call_item = None;
    state.request_phase_atom_ids.clear();
    state.capability_atom_ids.clear();
    state.runtime_request_text = String::new();
    // The user-message row was already committed before observe_row called
    // reset_turn. Preserve that one record as the first commitment of the new
    // turn and discard all commitments from the completed turn.
    state.turn_capture_records = state.current_capture_record.clone().into_iter().collect();
}

fn discard_partial_turn(state: &mut SessionState) {
    reset_turn(state);
    state.turn_event_graphs.clear();
    state.turn_event_nodes = 0;
    state.turn_graph_overflow = false;
    state.runtime_parity_cases.clear();
    state.turn_capture_records.clear();
}

fn current_capture_receipt(state: &SessionState) -> Option<CaptureEvidenceReceipt> {
    CaptureEvidenceReceipt::new(state.turn_capture_records.clone()).ok()
}

fn flush_pending(state: &mut SessionState, tokens: u64, output: &mut Vec<RelationFrame>) {
    state.pending_action_call_id = None;
    for mut frame in state.pending_frames.drain(..) {
        if frame.verifier_label.is_none() {
            continue;
        }
        frame.estimated_input_tokens = tokens;
        output.push(frame);
    }
}

fn remember_call(payload: &serde_json::Map<String, Value>, shape: &str, state: &mut SessionState) {
    let Some(call_id) = payload.get("call_id").and_then(Value::as_str) else {
        return;
    };
    let Some(name) = payload.get("name").and_then(Value::as_str) else {
        return;
    };
    state.calls.insert(
        call_id.to_owned(),
        CallShape {
            name: name.to_owned(),
            shape: shape.to_owned(),
        },
    );
    if shape == "function_call"
        && let Some(arguments) = direct_call_arguments(payload)
        && is_plan_arguments(&arguments)
    {
        state.latest_plan_call_item = Some(serde_json::json!({
            "type": "function_call",
            "name": name,
            "call_id": call_id,
            "arguments": Value::Object(arguments),
        }));
    }
    state.call_count = state.call_count.saturating_add(1);
}

fn remember_output(payload: &serde_json::Map<String, Value>, state: &mut SessionState) {
    let Some(call_id) = payload.get("call_id").and_then(Value::as_str) else {
        return;
    };
    let Some(call) = state.calls.remove(call_id) else {
        return;
    };
    if state.pending_action_call_id.as_deref() == Some(call_id) {
        for frame in &mut state.pending_frames {
            frame.verifier_label = Some(true);
        }
    }
    let Some(output) = payload.get("output") else {
        return;
    };
    let accumulated = (!state.runtime_provider_payload_overflow)
        .then_some(state.runtime_provider_payload.as_ref())
        .flatten();
    let runtime_payload = append_runtime_provider_output(
        accumulated,
        state.collection_request_item.as_ref(),
        state.latest_plan_call_item.as_ref(),
        call_id,
        &call,
        output,
    )
    .or_else(|| {
        // Scalar parity owns only the observation that grounds the pending
        // action. If earlier outputs overflow its bounded window, rebuild from
        // the current request and latest output; collection learning retains
        // the complete multi-source history through its separate payload.
        append_runtime_provider_output(
            None,
            state.collection_request_item.as_ref(),
            state.latest_plan_call_item.as_ref(),
            call_id,
            &call,
            output,
        )
    });
    match runtime_payload {
        Some(payload) => {
            state.runtime_provider_payload = Some(payload);
            state.runtime_provider_payload_overflow = false;
        }
        None => {
            state.runtime_provider_payload = None;
            state.runtime_provider_payload_overflow = true;
        }
    }
    if let Some(mut collection_payload) = collection_provider_payload(output) {
        if let Some(input) = collection_payload
            .get_mut("input")
            .and_then(Value::as_array_mut)
        {
            input.insert(
                0,
                serde_json::json!({
                    "type": call.shape.clone(),
                    "name": call.name.clone(),
                    "call_id": call_id,
                }),
            );
        }
        if let Some(current) = &mut state.collection_provider_payload {
            if let (Some(target), Some(mut appended)) = (
                current.get_mut("input").and_then(Value::as_array_mut),
                collection_payload
                    .get("input")
                    .and_then(Value::as_array)
                    .cloned(),
            ) {
                target.append(&mut appended);
            }
        } else {
            if let Some(request) = state.collection_request_item.clone()
                && let Some(input) = collection_payload
                    .get_mut("input")
                    .and_then(Value::as_array_mut)
            {
                input.insert(0, request);
            }
            state.collection_provider_payload = Some(collection_payload);
        }
    }
    state.output_count = state.output_count.saturating_add(1);
    let output_sha256 = hash_value(output);
    let observations = scalar_observations(output, &call, &output_sha256);
    state.observations.extend(observations);
    if state.observations.len() > MAX_OBSERVATIONS {
        let excess = state.observations.len().saturating_sub(MAX_OBSERVATIONS);
        state.observations.drain(..excess);
    }
    mark_completed_turn_if_settled(state);
}

fn collection_provider_payload(output: &Value) -> Option<Value> {
    bounded_session_output_text(output)?;
    Some(serde_json::json!({
        "input": [{
            "type": "function_call_output",
            "output": output,
        }]
    }))
}

fn bounded_session_output_text(output: &Value) -> Option<String> {
    let parts = bounded_session_output_text_parts(output)?;
    let mut text = String::new();
    for part in parts {
        let next_len = text
            .len()
            .checked_add(part.len() + usize::from(!text.is_empty()))?;
        if next_len > 65_536 {
            return None;
        }
        if !text.is_empty() && !part.is_empty() {
            text.push('\n');
        }
        text.push_str(part);
    }
    (!text.is_empty()).then_some(text)
}

fn bounded_session_output_text_parts(output: &Value) -> Option<Vec<&str>> {
    if let Some(text) = output.as_str() {
        return (!text.is_empty() && text.len() <= 65_536).then_some(vec![text]);
    }
    let parts = output.as_array()?;
    if parts.is_empty() || parts.len() > 64 {
        return None;
    }
    let mut texts = Vec::with_capacity(parts.len());
    let mut total_bytes = 0_usize;
    for part in parts {
        if !matches!(
            part.get("type").and_then(Value::as_str),
            Some("text" | "input_text" | "output_text")
        ) {
            return None;
        }
        let text = part.get("text").and_then(Value::as_str)?;
        total_bytes = total_bytes.checked_add(text.len())?;
        texts.push(text);
    }
    (total_bytes > 0 && total_bytes <= 65_536).then_some(texts)
}

fn append_runtime_provider_output(
    current: Option<&Value>,
    request: Option<&Value>,
    latest_plan_call: Option<&Value>,
    call_id: &str,
    call: &CallShape,
    output: &Value,
) -> Option<Value> {
    serde_json::to_vec(output)
        .ok()
        .filter(|bytes| !bytes.is_empty() && bytes.len() <= 65_536)?;
    let output_type = match call.shape.as_str() {
        "function_call" => "function_call_output",
        "custom_tool_call" => "custom_tool_call_output",
        _ => return None,
    };
    let mut input = current
        .and_then(|payload| payload.get("input"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| {
            let mut input = Vec::with_capacity(4);
            if let Some(request) = request {
                input.push(request.clone());
            }
            if latest_plan_call
                .is_some_and(|plan| plan.get("call_id").and_then(Value::as_str) != Some(call_id))
                && let Some(plan) = latest_plan_call
            {
                input.push(plan.clone());
            }
            input
        });
    if input.len() > 126 {
        return None;
    }
    let call_item = latest_plan_call
        .filter(|plan| plan.get("call_id").and_then(Value::as_str) == Some(call_id))
        .cloned()
        .unwrap_or_else(|| {
            serde_json::json!({
                "type": call.shape,
                "name": call.name,
                "call_id": call_id,
            })
        });
    input.push(call_item);
    input.push(serde_json::json!({
        "type": output_type,
        "call_id": call_id,
        "output": output,
    }));
    let payload = serde_json::json!({"input": input});
    serde_json::to_vec(&payload)
        .ok()
        .filter(|bytes| bytes.len() <= 131_072)
        .map(|_| payload)
}

fn collection_request_item(message: &str) -> Option<Value> {
    (!message.is_empty() && message.len() <= 16_384).then(|| {
        serde_json::json!({
            "type": "message",
            "role": "user",
            "content": [{"type":"input_text", "text":message}],
        })
    })
}

fn remember_collection_response(text: &str, state: &mut SessionState) {
    if state.collection_provider_payload.is_some() && !text.is_empty() && text.len() <= 16_384 {
        state.collection_expected_response = Some(text.to_owned());
    }
}

fn mark_completed_turn_if_settled(state: &mut SessionState) {
    if state.collection_expected_response.is_some() && state.calls.is_empty() {
        state.collection_completion_reason = Some(TurnCompletionReason::FinalAnswerSettled);
    }
}

fn scalar_observations(output: &Value, call: &CallShape, output_sha256: &str) -> Vec<Observation> {
    let Some(parts) = bounded_session_output_text_parts(output) else {
        return Vec::new();
    };
    let text = parts.join("\n");
    let trimmed = text.trim();
    let completion_state = if text.lines().any(|line| {
        line.starts_with("Script running with cell ID ")
            || line.starts_with("Process running with session ID ")
    }) {
        "pending"
    } else {
        "completed"
    };
    let mut values = Vec::new();
    for prefix in [
        "Script running with cell ID ",
        "Process running with session ID ",
    ] {
        if let Some(cell_id) = text.lines().find_map(|line| {
            line.strip_prefix(prefix)
                .and_then(|rest| rest.split_whitespace().next())
                .filter(|value| !value.is_empty())
        }) {
            values.push((
                Value::String(cell_id.to_owned()),
                AtomValueType::Identifier,
                ResponseValueSelector::ContentLinePrefix {
                    prefix: prefix.to_owned(),
                    value_type: AtomValueType::Identifier,
                },
            ));
        }
    }
    for part in parts {
        for object in session_embedded_json_objects(part) {
            collect_session_json_scalars(&Value::Object(object), 0, &mut values);
        }
    }
    if values.is_empty() {
        let parsed = serde_json::from_str::<Value>(trimmed)
            .unwrap_or_else(|_| Value::String(trimmed.to_owned()));
        if let Some(value_type) = scalar_type(&parsed) {
            values.push((
                parsed,
                value_type,
                ResponseValueSelector::UniqueScalar { value_type },
            ));
        }
    }
    values.sort_by_cached_key(|(value, _, selector)| {
        (
            serde_json::to_string(selector).unwrap_or_default(),
            hash_value(value),
        )
    });
    values.dedup_by(|left, right| left.0 == right.0 && left.2 == right.2);
    values
        .into_iter()
        .map(|(value, value_type, selector)| Observation {
            value_sha256: hash_value(&value),
            value_type,
            render_value: scalar_render_value(&value),
            selector,
            tool_kind: call.name.clone(),
            call_shape: call.shape.clone(),
            output_sha256: output_sha256.to_owned(),
            completion_state,
        })
        .collect()
}

fn scalar_render_value(value: &Value) -> Option<String> {
    let rendered = match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        _ => return None,
    };
    (!rendered.is_empty() && rendered.len() <= 512).then_some(rendered)
}

fn session_embedded_json_objects(text: &str) -> Vec<serde_json::Map<String, Value>> {
    session_embedded_json_objects_at_depth(text, 0)
}

fn session_embedded_json_objects_at_depth(
    text: &str,
    depth: usize,
) -> Vec<serde_json::Map<String, Value>> {
    if depth > 4 {
        return Vec::new();
    }
    let mut objects = BTreeMap::<Vec<u8>, serde_json::Map<String, Value>>::new();
    let mut sources = vec![text.trim()];
    if let Some((_, output)) = text.rsplit_once("\nOutput:\n") {
        sources.push(output.trim());
    }
    for source in sources {
        if let Ok(value) = serde_json::from_str::<Value>(source) {
            collect_session_json_objects(&value, depth, &mut objects);
        }
    }
    objects.into_values().collect()
}

fn collect_session_json_objects(
    value: &Value,
    depth: usize,
    output: &mut BTreeMap<Vec<u8>, serde_json::Map<String, Value>>,
) {
    if depth > 4 || output.len() >= 64 {
        return;
    }
    match value {
        Value::Object(object) => {
            let mut encoded_children = BTreeMap::new();
            for text in object.values().filter_map(Value::as_str) {
                for child in session_embedded_json_objects_at_depth(text, depth + 1) {
                    if let Ok(key) = serde_json::to_vec(&child) {
                        encoded_children.insert(key, child);
                    }
                }
            }
            if encoded_children.len() == 1 {
                output.extend(encoded_children);
                return;
            }
            if let Ok(key) = serde_json::to_vec(value) {
                output.insert(key, object.clone());
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_session_json_objects(value, depth + 1, output);
            }
        }
        _ => {}
    }
}

fn collect_session_json_scalars(
    value: &Value,
    depth: usize,
    output: &mut Vec<(Value, AtomValueType, ResponseValueSelector)>,
) {
    if depth > 8 || output.len() >= 64 {
        return;
    }
    match value {
        Value::Object(object) => {
            for (field, value) in object {
                if let Some(value_type) = scalar_type(value) {
                    output.push((
                        value.clone(),
                        value_type,
                        ResponseValueSelector::JsonField {
                            field: field.clone(),
                            value_type,
                        },
                    ));
                }
                collect_session_json_scalars(value, depth + 1, output);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_session_json_scalars(value, depth + 1, output);
            }
        }
        _ => {}
    }
}

fn action_frames(
    row: &Value,
    payload: &serde_json::Map<String, Value>,
    state: &mut SessionState,
) -> Vec<RelationFrame> {
    let Some(outer_name) = payload.get("name").and_then(Value::as_str) else {
        return Vec::new();
    };
    let source = payload
        .get("arguments")
        .or_else(|| payload.get("input"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let custom = (payload.get("type").and_then(Value::as_str) == Some("custom_tool_call"))
        .then(|| parse_custom_tool_source(source))
        .flatten();
    let (action_name, arguments) = if let Some(custom) = &custom {
        (custom.inner_tool_name.as_str(), custom.arguments.clone())
    } else {
        let arguments = serde_json::from_str::<Value>(source)
            .ok()
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        (outer_name, arguments)
    };
    if custom.is_none()
        && payload.get("type").and_then(Value::as_str) == Some("function_call")
        && is_plan_arguments(&arguments)
    {
        return plan_advance_frames(row, state, outer_name, &arguments);
    }
    if arguments.is_empty() {
        return Vec::new();
    }
    let mut matches = Vec::new();
    for observation in &state.observations {
        for (argument, value) in &arguments {
            if argument_matches_observation(value, observation) {
                matches.push((observation, argument.as_str()));
            }
        }
    }
    let Some(bindings) = unique_structural_action_bindings(&matches) else {
        return Vec::new();
    };
    let mut observations = BTreeMap::new();
    for (observation, _) in &bindings {
        observations
            .entry((
                observation.value_sha256.clone(),
                observation.value_type,
                observation.selector.clone(),
            ))
            .or_insert(*observation);
    }
    let observation_slots = observations
        .keys()
        .cloned()
        .enumerate()
        .map(|(index, key)| (key, u16::try_from(index + 1).unwrap_or(u16::MAX)))
        .collect::<BTreeMap<_, _>>();
    let observations_with_slots = observations
        .iter()
        .filter_map(|(key, observation)| {
            observation_slots
                .get(key)
                .copied()
                .map(|slot_id| (*observation, slot_id))
        })
        .collect::<Vec<_>>();
    let mut atoms = base_atoms_for_observations(&observations_with_slots, state);
    atoms.push(RelationAtom::ResponseShape {
        value: if custom.is_some() {
            "custom_tool_call".to_owned()
        } else {
            "function_call".to_owned()
        },
    });
    if let Some(custom) = &custom {
        atoms.push(RelationAtom::ActionCustomTool {
            value: outer_name.to_owned(),
        });
        atoms.push(RelationAtom::ActionInnerTool {
            value: custom.inner_tool_name.clone(),
        });
        atoms.push(custom.projection.clone());
    } else {
        atoms.push(RelationAtom::ActionFunction {
            value: action_name.to_owned(),
        });
    }
    let first_action_slot = u16::try_from(observations.len() + 1).unwrap_or(u16::MAX);
    let mut bound_arguments = BTreeSet::new();
    for (index, (observation, argument)) in bindings.iter().enumerate() {
        let key = (
            observation.value_sha256.clone(),
            observation.value_type,
            observation.selector.clone(),
        );
        let Some(observation_slot) = observation_slots.get(&key).copied() else {
            return Vec::new();
        };
        let action_slot =
            first_action_slot.saturating_add(u16::try_from(index).unwrap_or(u16::MAX));
        atoms.push(RelationAtom::TypedSlot {
            slot_id: action_slot,
            value_type: observation.value_type,
            source: AtomSource::Action,
            value_sha256: observation.value_sha256.clone(),
        });
        atoms.push(RelationAtom::ActionRoleArgument {
            name: (*argument).to_owned(),
            slot_id: action_slot,
            value_type: arguments
                .get(*argument)
                .and_then(action_argument_value_type),
        });
        atoms.push(RelationAtom::SlotEquality {
            left_slot: observation_slot,
            right_slot: action_slot,
        });
        bound_arguments.insert(*argument);
    }
    for (name, value) in &arguments {
        if bound_arguments.contains(name.as_str()) {
            continue;
        }
        match value {
            Value::Number(number) if number.as_u64().is_some() => {
                atoms.push(RelationAtom::ActionIntegerArgument {
                    name: name.clone(),
                    value: number.as_u64().unwrap_or_default(),
                });
            }
            Value::String(value) => atoms.push(RelationAtom::ActionStringArgument {
                name: name.clone(),
                value: value.clone(),
            }),
            Value::Bool(value) => atoms.push(RelationAtom::ActionBooleanArgument {
                name: name.clone(),
                value: *value,
            }),
            _ => {}
        }
    }
    let evidence = canonical_json_sha256(&(
        "nando.multi-role-action-evidence.v1",
        observations
            .values()
            .map(|observation| observation.output_sha256.as_str())
            .collect::<Vec<_>>(),
    ))
    .unwrap_or_default();
    let mut frame = build_frame(row, state, atoms, false, &evidence);
    frame.verifier_label = None;
    if let Some(provider_payload) = bounded_runtime_provider_payload(state)
        && let Some(expected_response) =
            expected_action_response(outer_name, action_name, &arguments, custom.as_ref())
    {
        state.runtime_parity_cases.insert(
            frame.frame_id_sha256.clone(),
            RuntimeParityCase {
                evidence_ref_sha256: frame.frame_id_sha256.clone(),
                capture_receipt: current_capture_receipt(state),
                request_text: state.runtime_request_text.clone(),
                provider_payload,
                expected_response,
            },
        );
        trim_runtime_parity_outbox(state);
    }
    vec![frame]
}

fn unique_structural_action_bindings<'a>(
    matches: &[(&'a Observation, &'a str)],
) -> Option<Vec<(&'a Observation, &'a str)>> {
    let mut by_argument = BTreeMap::<&str, Vec<(&Observation, &str)>>::new();
    for (observation, argument) in matches {
        by_argument
            .entry(*argument)
            .or_default()
            .push((*observation, *argument));
    }
    let mut bindings = Vec::with_capacity(by_argument.len());
    for candidates in by_argument.into_values() {
        let identifier_candidates = candidates
            .iter()
            .copied()
            .filter(|(observation, _)| observation.value_type == AtomValueType::Identifier)
            .collect::<Vec<_>>();
        let selected = if identifier_candidates.is_empty() {
            unique_structural_action_match(&candidates)
        } else {
            unique_structural_action_match(&identifier_candidates)
        }?;
        bindings.push(selected);
    }
    (!bindings.is_empty()).then_some(bindings)
}

/// Collapses only duplicate extraction of the same structural anchor. Equal
/// values observed through different selectors remain ambiguous: choosing one
/// would hide a role swap when those values diverge on frozen future traffic.
fn unique_structural_action_match<'a>(
    matches: &[(&'a Observation, &'a str)],
) -> Option<(&'a Observation, &'a str)> {
    let mut unique = BTreeMap::new();
    for (observation, argument) in matches {
        unique
            .entry((
                (*argument).to_owned(),
                observation.value_type,
                observation.selector.clone(),
            ))
            .or_insert((*observation, *argument));
    }
    (unique.len() == 1)
        .then(|| unique.into_values().next())
        .flatten()
}

fn direct_call_arguments(
    payload: &serde_json::Map<String, Value>,
) -> Option<serde_json::Map<String, Value>> {
    match payload.get("arguments")? {
        Value::Object(arguments) => Some(arguments.clone()),
        Value::String(arguments) if arguments.len() <= 131_072 => {
            serde_json::from_str::<Value>(arguments)
                .ok()?
                .as_object()
                .cloned()
        }
        _ => None,
    }
}

fn is_plan_arguments(arguments: &serde_json::Map<String, Value>) -> bool {
    if arguments.len() != 1 {
        return false;
    }
    let Some(plan) = arguments.get("plan").and_then(Value::as_array) else {
        return false;
    };
    if plan.is_empty() || plan.len() > 32 {
        return false;
    }
    plan.iter().all(|step| {
        let Some(step) = step.as_object() else {
            return false;
        };
        if step.len() != 2 {
            return false;
        }
        let Some(text) = step.get("step").and_then(Value::as_str) else {
            return false;
        };
        let Some(status) = step.get("status").and_then(Value::as_str) else {
            return false;
        };
        !text.is_empty()
            && text.len() <= 1_024
            && !text.chars().any(char::is_control)
            && matches!(status, "pending" | "in_progress" | "completed")
    })
}

fn canonical_plan_state(arguments: &serde_json::Map<String, Value>) -> Option<(u16, u16, u16)> {
    if !is_plan_arguments(arguments) {
        return None;
    }
    let plan = arguments.get("plan")?.as_array()?;
    let mut completed_count = 0_usize;
    while plan.get(completed_count)?.get("status")?.as_str()? == "completed" {
        completed_count = completed_count.saturating_add(1);
        if completed_count == plan.len() {
            return None;
        }
    }
    if plan.get(completed_count)?.get("status")?.as_str()? != "in_progress" {
        return None;
    }
    if plan
        .iter()
        .skip(completed_count.saturating_add(1))
        .any(|step| step.get("status").and_then(Value::as_str) != Some("pending"))
    {
        return None;
    }
    Some((
        u16::try_from(plan.len()).ok()?,
        u16::try_from(completed_count).ok()?,
        u16::try_from(completed_count).ok()?,
    ))
}

fn latest_prior_plan_state(
    provider_payload: &Value,
    function_name: &str,
) -> Option<(u16, u16, u16)> {
    provider_payload
        .get("input")?
        .as_array()?
        .iter()
        .rev()
        .filter_map(Value::as_object)
        .find_map(|item| {
            (item.get("type").and_then(Value::as_str) == Some("function_call")
                && item.get("name").and_then(Value::as_str) == Some(function_name))
            .then(|| direct_call_arguments(item))
            .flatten()
            .and_then(|arguments| canonical_plan_state(&arguments))
        })
}

fn immediate_tool_shape(provider_payload: &Value) -> Option<(&str, &str)> {
    let input = provider_payload.get("input")?.as_array()?;
    let output = input.iter().rev().find_map(Value::as_object)?;
    let call_id = output.get("call_id")?.as_str()?;
    input
        .iter()
        .rev()
        .filter_map(Value::as_object)
        .find_map(|item| {
            (item.get("call_id").and_then(Value::as_str) == Some(call_id)
                && matches!(
                    item.get("type").and_then(Value::as_str),
                    Some("function_call" | "custom_tool_call")
                ))
            .then(|| Some((item.get("name")?.as_str()?, item.get("type")?.as_str()?)))
            .flatten()
        })
}

fn plan_advance_frames(
    row: &Value,
    state: &mut SessionState,
    function_name: &str,
    teacher_arguments: &serde_json::Map<String, Value>,
) -> Vec<RelationFrame> {
    let Some(provider_payload) = bounded_runtime_provider_payload(state) else {
        return Vec::new();
    };
    let Some((step_count, completed_count, active_index)) =
        latest_prior_plan_state(&provider_payload, function_name)
    else {
        return Vec::new();
    };
    let Some((tool_name, tool_shape)) = immediate_tool_shape(&provider_payload) else {
        return Vec::new();
    };
    let program = ResponseProgram::advance_plan(function_name);
    let execution = execute_response(&program, &state.runtime_request_text, &provider_payload);
    if execution.status != ResponseExecutionStatus::Executed {
        return Vec::new();
    }
    let Some(response) = execution.response else {
        return Vec::new();
    };
    let teacher_response = serde_json::json!({
        "name": function_name,
        "arguments": teacher_arguments,
    });
    if serde_json::from_str::<Value>(&response).ok().as_ref() != Some(&teacher_response) {
        return Vec::new();
    }
    let verifier = VerifierProgram::AdvancePlan {
        function_name: function_name.to_owned(),
        require_explicit_tool_success: true,
        require_canonical_plan: true,
    };
    if verify_response_independently(&verifier, &provider_payload, &response).is_err() {
        return Vec::new();
    }

    let evidence_sha256 = hash_value(&provider_payload);
    let mut atoms = vec![
        RelationAtom::ToolKind {
            value: tool_name.to_owned(),
        },
        RelationAtom::ObservationCallShape {
            value: tool_shape.to_owned(),
        },
        RelationAtom::CompletionState {
            value: "completed".to_owned(),
        },
        RelationAtom::OutputStatus {
            value: "success".to_owned(),
        },
        RelationAtom::ResponseShape {
            value: "model_action".to_owned(),
        },
        RelationAtom::PlanState {
            step_count,
            completed_count,
            active_index,
        },
        RelationAtom::Cardinality {
            role: "plan_step_count_band".to_owned(),
            count: count_band(u32::from(step_count)),
        },
        RelationAtom::Cardinality {
            role: "plan_completed_count_band".to_owned(),
            count: count_band(u32::from(completed_count)),
        },
        RelationAtom::Cardinality {
            role: "turn_call_count_band".to_owned(),
            count: count_band(state.call_count),
        },
        RelationAtom::Cardinality {
            role: "turn_output_count_band".to_owned(),
            count: count_band(state.output_count),
        },
        RelationAtom::Cardinality {
            role: "turn_message_count_band".to_owned(),
            count: count_band(state.message_count),
        },
        RelationAtom::ActionFunction {
            value: function_name.to_owned(),
        },
        RelationAtom::ActionPlanAdvance,
    ];
    atoms.extend(
        state
            .request_phase_atom_ids
            .iter()
            .copied()
            .map(|atom_id| RelationAtom::RequestPhaseAtom { atom_id }),
    );
    atoms.extend(
        state
            .capability_atom_ids
            .iter()
            .copied()
            .map(|atom_id| RelationAtom::ClientCapabilityAtom { atom_id }),
    );
    let mut frame = build_frame(row, state, atoms, false, &evidence_sha256);
    frame.verifier_label = None;
    state.runtime_parity_cases.insert(
        frame.frame_id_sha256.clone(),
        RuntimeParityCase {
            evidence_ref_sha256: frame.frame_id_sha256.clone(),
            capture_receipt: current_capture_receipt(state),
            request_text: state.runtime_request_text.clone(),
            provider_payload,
            expected_response: response,
        },
    );
    trim_runtime_parity_outbox(state);
    vec![frame]
}

fn expected_action_response(
    outer_name: &str,
    action_name: &str,
    arguments: &serde_json::Map<String, Value>,
    custom: Option<&ParsedCustomToolSource>,
) -> Option<String> {
    if let Some(custom) = custom {
        let arguments_json = serde_json::to_string(arguments).ok()?;
        let source = match &custom.projection {
            RelationAtom::ActionOutputProjection { output_field } => format!(
                "const r=await tools.{}({arguments_json});text(r.{output_field});",
                custom.inner_tool_name
            ),
            RelationAtom::ActionJsonResultProjection => format!(
                "const r=await tools.{}({arguments_json});text(JSON.stringify(r));",
                custom.inner_tool_name
            ),
            _ => return None,
        };
        serde_json::to_string(&serde_json::json!({
            "kind": "custom_tool_call",
            "name": outer_name,
            "input": source,
        }))
        .ok()
    } else {
        serde_json::to_string(&serde_json::json!({
            "name": action_name,
            "arguments": arguments,
        }))
        .ok()
    }
}

struct ParsedCustomToolSource {
    inner_tool_name: String,
    arguments: serde_json::Map<String, Value>,
    projection: RelationAtom,
}

fn parse_custom_tool_source(source: &str) -> Option<ParsedCustomToolSource> {
    if source.len() > 16 * 1024 {
        return None;
    }
    let tool_start = source.find("tools.")?.saturating_add("tools.".len());
    let name_end = source[tool_start..]
        .find(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))?
        .saturating_add(tool_start);
    let inner_tool_name = source[tool_start..name_end].to_owned();
    if inner_tool_name.is_empty() {
        return None;
    }
    let call_tail = source.get(name_end..)?;
    let call_start = call_tail.find('(')?.saturating_add(name_end);
    let object_start = source
        .get(call_start.saturating_add(1)..)?
        .find('{')?
        .saturating_add(call_start)
        .saturating_add(1);
    let object_end = matching_brace(source, object_start)?;
    let arguments = parse_flat_js_object(source.get(object_start..=object_end)?)?;
    let suffix = source
        .get(object_end.saturating_add(1)..)?
        .replace(char::is_whitespace, "");
    let projection = if suffix.contains("text(JSON.stringify(r))") {
        RelationAtom::ActionJsonResultProjection
    } else {
        if suffix.contains("if(r.") {
            return None;
        }
        let marker = "text(r.";
        let start = suffix.find(marker)?.saturating_add(marker.len());
        let end = suffix[start..]
            .find(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))?
            .saturating_add(start);
        let output_field = suffix[start..end].to_owned();
        if output_field.is_empty() {
            return None;
        }
        RelationAtom::ActionOutputProjection { output_field }
    };
    Some(ParsedCustomToolSource {
        inner_tool_name,
        arguments,
        projection,
    })
}

fn matching_brace(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(start) != Some(&b'{') {
        return None;
    }
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0_u16;
    for (offset, byte) in bytes.get(start..)?.iter().copied().enumerate() {
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == active {
                quote = None;
            }
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = Some(byte),
            b'{' => depth = depth.saturating_add(1),
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(start.saturating_add(offset));
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_flat_js_object(source: &str) -> Option<serde_json::Map<String, Value>> {
    let inner = source.strip_prefix('{')?.strip_suffix('}')?;
    let mut cursor = 0;
    let mut output = serde_json::Map::new();
    while cursor < inner.len() {
        cursor = skip_js_space_and_commas(inner, cursor);
        if cursor >= inner.len() {
            break;
        }
        let (key, next) = parse_js_key(inner, cursor)?;
        cursor = skip_js_space(inner, next);
        if inner.as_bytes().get(cursor) != Some(&b':') {
            return None;
        }
        cursor = skip_js_space(inner, cursor.saturating_add(1));
        let (value, next) = parse_js_scalar(inner, cursor)?;
        output.insert(key, value);
        cursor = skip_js_space(inner, next);
        if cursor < inner.len() && inner.as_bytes().get(cursor) != Some(&b',') {
            return None;
        }
    }
    Some(output)
}

fn skip_js_space(source: &str, mut cursor: usize) -> usize {
    while source
        .as_bytes()
        .get(cursor)
        .is_some_and(u8::is_ascii_whitespace)
    {
        cursor = cursor.saturating_add(1);
    }
    cursor
}

fn skip_js_space_and_commas(source: &str, mut cursor: usize) -> usize {
    while source
        .as_bytes()
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b',')
    {
        cursor = cursor.saturating_add(1);
    }
    cursor
}

fn parse_js_key(source: &str, cursor: usize) -> Option<(String, usize)> {
    if matches!(source.as_bytes().get(cursor), Some(b'"' | b'\'')) {
        return parse_js_string(source, cursor);
    }
    let end = source[cursor..]
        .find(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .map_or(source.len(), |offset| cursor.saturating_add(offset));
    (end > cursor).then(|| (source[cursor..end].to_owned(), end))
}

fn parse_js_scalar(source: &str, cursor: usize) -> Option<(Value, usize)> {
    if matches!(source.as_bytes().get(cursor), Some(b'"' | b'\'')) {
        let (value, end) = parse_js_string(source, cursor)?;
        return Some((Value::String(value), end));
    }
    let end = source[cursor..]
        .find(|character: char| character == ',' || character.is_ascii_whitespace())
        .map_or(source.len(), |offset| cursor.saturating_add(offset));
    let token = source.get(cursor..end)?;
    let value = match token {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "null" => Value::Null,
        _ => serde_json::from_str::<Value>(token).ok()?,
    };
    matches!(value, Value::Null | Value::Bool(_) | Value::Number(_)).then_some((value, end))
}

fn parse_js_string(source: &str, cursor: usize) -> Option<(String, usize)> {
    let quote = *source.as_bytes().get(cursor)?;
    let mut escaped = false;
    for index in cursor.saturating_add(1)..source.len() {
        let byte = source.as_bytes()[index];
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == quote {
            let raw = source.get(cursor.saturating_add(1)..index)?;
            let value = if quote == b'"' {
                serde_json::from_str::<String>(source.get(cursor..=index)?).ok()?
            } else if raw.contains('\\') {
                return None;
            } else {
                raw.to_owned()
            };
            return Some((value, index.saturating_add(1)));
        }
    }
    None
}

fn argument_matches_observation(value: &Value, observation: &Observation) -> bool {
    if hash_value(value) == observation.value_sha256 {
        return true;
    }
    if observation.value_type != AtomValueType::Identifier {
        return false;
    }
    let normalized = match value {
        Value::Number(number) => Some(number.to_string()),
        Value::String(text) => Some(text.clone()),
        _ => None,
    };
    normalized.is_some_and(|text| hash_value(&Value::String(text)) == observation.value_sha256)
}

fn action_argument_value_type(value: &Value) -> Option<AtomValueType> {
    match value {
        Value::Number(value) if value.as_u64().is_some() => Some(AtomValueType::Integer),
        Value::String(_) => Some(AtomValueType::String),
        Value::Bool(_) => Some(AtomValueType::Boolean),
        _ => None,
    }
}

fn assistant_frames(row: &Value, text: &str, state: &mut SessionState) -> Vec<RelationFrame> {
    let trimmed = text.trim();
    let parsed = serde_json::from_str::<Value>(trimmed)
        .unwrap_or_else(|_| Value::String(trimmed.to_owned()));
    let matches = state
        .observations
        .iter()
        .filter(|observation| hash_value(&parsed) == observation.value_sha256)
        .collect::<Vec<_>>();
    let frame = if matches.len() == 1 {
        let observation = matches[0];
        let mut atoms = base_atoms(observation, state);
        atoms.push(RelationAtom::ResponseShape {
            value: "assistant_message".to_owned(),
        });
        atoms.push(RelationAtom::TypedSlot {
            slot_id: 2,
            value_type: observation.value_type,
            source: AtomSource::Action,
            value_sha256: observation.value_sha256.clone(),
        });
        atoms.push(RelationAtom::SlotEquality {
            left_slot: 1,
            right_slot: 2,
        });
        atoms.push(RelationAtom::ActionValueProjection {
            format: if matches!(parsed, Value::String(_)) {
                ValueProjectionFormat::PlainText
            } else {
                ValueProjectionFormat::CanonicalJson
            },
            renderer: CollectionOutputRenderer::Direct,
        });
        build_frame(row, state, atoms, true, &observation.output_sha256)
    } else {
        let Some(frame) = multi_role_assistant_frame(row, text, state) else {
            return Vec::new();
        };
        frame
    };
    if let Some(provider_payload) = bounded_runtime_provider_payload(state) {
        state.runtime_parity_cases.insert(
            frame.frame_id_sha256.clone(),
            RuntimeParityCase {
                evidence_ref_sha256: frame.frame_id_sha256.clone(),
                capture_receipt: current_capture_receipt(state),
                request_text: state.runtime_request_text.clone(),
                provider_payload,
                expected_response: text.to_owned(),
            },
        );
        trim_runtime_parity_outbox(state);
    }
    vec![frame]
}

fn multi_role_assistant_frame(
    row: &Value,
    text: &str,
    state: &SessionState,
) -> Option<RelationFrame> {
    if text.is_empty() || text.len() > 16_384 {
        return None;
    }
    let mut by_rendered =
        BTreeMap::<String, BTreeMap<(AtomValueType, ResponseValueSelector), &Observation>>::new();
    for observation in &state.observations {
        let Some(rendered) = observation.render_value.as_ref() else {
            continue;
        };
        if rendered_occurrences(text, rendered)?.is_empty() {
            continue;
        }
        by_rendered.entry(rendered.clone()).or_default().insert(
            (observation.value_type, observation.selector.clone()),
            observation,
        );
    }
    let mut role_occurrences = Vec::new();
    for (rendered, candidates) in by_rendered {
        if candidates.len() != 1 {
            return None;
        }
        let observation = candidates.into_values().next()?;
        for (start, end) in rendered_occurrences(text, &rendered)? {
            role_occurrences.push((start, end, observation));
        }
    }
    role_occurrences.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.selector.cmp(&right.2.selector))
    });
    if role_occurrences
        .windows(2)
        .any(|pair| pair[0].1 > pair[1].0)
    {
        return None;
    }
    let mut observations = Vec::<&Observation>::new();
    for (_, _, observation) in &role_occurrences {
        if !observations.iter().any(|known| {
            known.value_sha256 == observation.value_sha256
                && known.value_type == observation.value_type
                && known.selector == observation.selector
        }) {
            observations.push(observation);
        }
    }
    if observations.len() < 2 || observations.len() > 32 {
        return None;
    }
    let observation_slots = observations
        .iter()
        .enumerate()
        .map(|(index, observation)| {
            (
                (
                    observation.value_sha256.clone(),
                    observation.value_type,
                    observation.selector.clone(),
                ),
                u16::try_from(index + 1).unwrap_or(u16::MAX),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let observations_with_slots = observations
        .iter()
        .filter_map(|observation| {
            observation_slots
                .get(&(
                    observation.value_sha256.clone(),
                    observation.value_type,
                    observation.selector.clone(),
                ))
                .copied()
                .map(|slot_id| (*observation, slot_id))
        })
        .collect::<Vec<_>>();
    let mut segments = Vec::new();
    let mut cursor = 0usize;
    for (start, end, observation) in &role_occurrences {
        if *start > cursor {
            segments.push(ResponseRenderSegment::Static {
                text: text.get(cursor..*start)?.to_owned(),
            });
        }
        let role_index = observations.iter().position(|known| {
            known.value_sha256 == observation.value_sha256
                && known.value_type == observation.value_type
                && known.selector == observation.selector
        })?;
        if role_index == 0 {
            segments.push(ResponseRenderSegment::Primary);
        } else {
            segments.push(ResponseRenderSegment::Selected {
                selector: observation.selector.clone(),
                format: ValueProjectionFormat::PlainText,
            });
        }
        cursor = *end;
    }
    if cursor < text.len() {
        segments.push(ResponseRenderSegment::Static {
            text: text.get(cursor..)?.to_owned(),
        });
    }
    let static_bytes = segments
        .iter()
        .filter_map(|segment| match segment {
            ResponseRenderSegment::Static { text } => Some(text.len()),
            _ => None,
        })
        .sum::<usize>();
    if segments.len() > 64 || static_bytes > 1_024 {
        return None;
    }
    let renderer = CollectionOutputRenderer::RenderSequence { segments };
    let mut atoms = base_atoms_for_observations(&observations_with_slots, state);
    atoms.push(RelationAtom::ResponseShape {
        value: "assistant_message".to_owned(),
    });
    let first_action_slot = u16::try_from(observations.len() + 1).ok()?;
    for (index, observation) in observations.iter().enumerate() {
        let observation_slot = u16::try_from(index + 1).ok()?;
        let action_slot = first_action_slot.checked_add(u16::try_from(index).ok()?)?;
        atoms.push(RelationAtom::TypedSlot {
            slot_id: action_slot,
            value_type: observation.value_type,
            source: AtomSource::Action,
            value_sha256: observation.value_sha256.clone(),
        });
        atoms.push(RelationAtom::SlotEquality {
            left_slot: observation_slot,
            right_slot: action_slot,
        });
    }
    atoms.push(RelationAtom::ActionValueProjection {
        format: ValueProjectionFormat::PlainText,
        renderer,
    });
    let evidence = canonical_json_sha256(&(
        "nando.multi-role-assistant-evidence.v1",
        observations
            .iter()
            .map(|observation| observation.output_sha256.as_str())
            .collect::<Vec<_>>(),
    ))
    .ok()?;
    Some(build_frame(row, state, atoms, true, &evidence))
}

fn rendered_occurrences(text: &str, rendered: &str) -> Option<Vec<(usize, usize)>> {
    if rendered.is_empty() || rendered.len() > 512 {
        return None;
    }
    let starts_word = rendered
        .chars()
        .next()
        .is_some_and(|character| character.is_alphanumeric() || character == '_');
    let ends_word = rendered
        .chars()
        .next_back()
        .is_some_and(|character| character.is_alphanumeric() || character == '_');
    let mut occurrences = Vec::new();
    for (start, _) in text.match_indices(rendered) {
        let end = start.checked_add(rendered.len())?;
        let left_ok = !starts_word
            || text
                .get(..start)
                .and_then(|prefix| prefix.chars().next_back())
                .is_none_or(|character| !(character.is_alphanumeric() || character == '_'));
        let right_ok = !ends_word
            || text
                .get(end..)
                .and_then(|suffix| suffix.chars().next())
                .is_none_or(|character| !(character.is_alphanumeric() || character == '_'));
        if left_ok && right_ok {
            occurrences.push((start, end));
            if occurrences.len() > 64 {
                return None;
            }
        }
    }
    Some(occurrences)
}

fn trim_runtime_parity_outbox(state: &mut SessionState) {
    while state.runtime_parity_cases.len() > MAX_PENDING_RUNTIME_PARITY_CASES {
        let Some(oldest) = state.runtime_parity_cases.keys().next().cloned() else {
            break;
        };
        state.runtime_parity_cases.remove(&oldest);
    }
}

fn retain_relevant_runtime_parity_cases(state: &mut SessionState, emitted: &[RelationFrame]) {
    let relevant = state
        .pending_frames
        .iter()
        .chain(emitted)
        .map(|frame| frame.frame_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    state
        .runtime_parity_cases
        .retain(|frame_id, _| relevant.contains(frame_id.as_str()));
}

fn bounded_runtime_provider_payload(state: &SessionState) -> Option<Value> {
    let payload = state.runtime_provider_payload.clone()?;
    serde_json::to_vec(&payload)
        .ok()
        .filter(|bytes| bytes.len() <= 131_072)
        .map(|_| payload)
}

fn base_atoms(observation: &Observation, state: &SessionState) -> Vec<RelationAtom> {
    base_atoms_for_observations(&[(observation, 1)], state)
}

fn base_atoms_for_observations(
    observations: &[(&Observation, u16)],
    state: &SessionState,
) -> Vec<RelationAtom> {
    let mut atoms = Vec::new();
    for (index, (observation, slot_id)) in observations.iter().enumerate() {
        atoms.push(RelationAtom::ToolKind {
            value: observation.tool_kind.clone(),
        });
        atoms.push(RelationAtom::ObservationCallShape {
            value: observation.call_shape.clone(),
        });
        atoms.push(RelationAtom::CompletionState {
            value: observation.completion_state.to_owned(),
        });
        if index == 0 {
            atoms.push(RelationAtom::ResponseShape {
                value: "model_action".to_owned(),
            });
        }
        atoms.push(RelationAtom::TypedSlot {
            slot_id: *slot_id,
            value_type: observation.value_type,
            source: AtomSource::Observation,
            value_sha256: observation.value_sha256.clone(),
        });
        atoms.push(RelationAtom::UniqueSlot { slot_id: *slot_id });
        atoms.push(RelationAtom::ObservationSelector {
            slot_id: *slot_id,
            selector: observation.selector.clone(),
        });
    }
    atoms.extend([
        RelationAtom::Cardinality {
            role: "turn_call_count_band".to_owned(),
            count: count_band(state.call_count),
        },
        RelationAtom::Cardinality {
            role: "turn_output_count_band".to_owned(),
            count: count_band(state.output_count),
        },
        RelationAtom::Cardinality {
            role: "turn_message_count_band".to_owned(),
            count: count_band(state.message_count),
        },
    ]);
    atoms.extend(
        state
            .request_phase_atom_ids
            .iter()
            .copied()
            .map(|atom_id| RelationAtom::RequestPhaseAtom { atom_id }),
    );
    atoms.extend(
        state
            .capability_atom_ids
            .iter()
            .copied()
            .map(|atom_id| RelationAtom::ClientCapabilityAtom { atom_id }),
    );
    atoms
}

fn build_frame(
    row: &Value,
    state: &SessionState,
    atoms: Vec<RelationAtom>,
    verifier_label: bool,
    evidence: &str,
) -> RelationFrame {
    let event_id = hash_value(row);
    let client_intent_id_sha256 = if state.turn_client_intent_id_sha256.is_empty() {
        event_id.clone()
    } else {
        state.turn_client_intent_id_sha256.clone()
    };
    let session_id_sha256 = if state.turn_session_id_sha256.is_empty() {
        state.session_id_sha256.clone()
    } else {
        state.turn_session_id_sha256.clone()
    };
    let frame_id = sha256_bytes(
        serde_json::to_vec(&(
            state.session_id_sha256.as_str(),
            state.offset,
            event_id.as_str(),
            &atoms,
        ))
        .unwrap_or_default()
        .as_slice(),
    );
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: frame_id,
        event_id_sha256: event_id.clone(),
        client_intent_id_sha256,
        session_id_sha256,
        observed_at_unix_nanos: event_time_unix_nanos(row)
            .or(state.turn_event_time_unix_nanos)
            .unwrap_or(0),
        estimated_input_tokens: 0,
        extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(verifier_label),
        atoms,
        evidence_ref_sha256: evidence.to_owned(),
    }
}

fn message_text(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|part| {
            matches!(
                part.get("type").and_then(Value::as_str),
                Some("output_text" | "text")
            )
            .then(|| part.get("text").and_then(Value::as_str))
            .flatten()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn scalar_type(value: &Value) -> Option<AtomValueType> {
    match value {
        Value::String(_) => Some(AtomValueType::String),
        Value::Number(number) if number.is_i64() || number.is_u64() => Some(AtomValueType::Integer),
        Value::Bool(_) => Some(AtomValueType::Boolean),
        _ => None,
    }
}

fn hash_value(value: &Value) -> String {
    sha256_bytes(serde_json::to_vec(value).unwrap_or_default().as_slice())
}

fn unix_now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn count_band(value: u32) -> u32 {
    if value == 0 {
        0
    } else {
        1 << (31 - value.leading_zeros())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nando_operator_learning::{
        EvidencePolicyV1, LearningRequestStructureInputV1, synthesize_response_operator,
    };
    use nando_response_actor::frame_matches_program_action_contract;
    use serde_json::json;
    use std::io::{Seek, Write};

    #[derive(Default)]
    struct RecordingMinerSink {
        frame_ids: Mutex<Vec<String>>,
    }

    impl SessionMinerSink for RecordingMinerSink {
        fn submit_frame_with_parity(
            &self,
            frame: RelationFrame,
            _runtime_parity_case: Option<RuntimeParityCase>,
        ) -> Result<(), String> {
            self.frame_ids
                .lock()
                .map_err(|_| "recording_sink_poisoned".to_owned())?
                .push(frame.frame_id_sha256);
            Ok(())
        }

        fn submit_collection(
            &self,
            _observation: OnlineCollectionObservation,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct CaptureOwnedCollectionSink {
        observations: Mutex<Vec<OnlineCollectionObservation>>,
    }

    impl SessionMinerSink for CaptureOwnedCollectionSink {
        fn submit_frame_with_parity(
            &self,
            _frame: RelationFrame,
            _runtime_parity_case: Option<RuntimeParityCase>,
        ) -> Result<(), String> {
            Ok(())
        }

        fn submit_collection(
            &self,
            observation: OnlineCollectionObservation,
        ) -> Result<(), String> {
            self.observations
                .lock()
                .map_err(|_| "capture_owned_collection_sink_poisoned".to_owned())?
                .push(observation);
            Ok(())
        }

        fn binds_collection_capture(&self) -> bool {
            true
        }
    }

    fn identity_test_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "nando-session-identity-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("identity test root");
        root
    }

    #[test]
    fn canonical_identity_survives_a_large_resume_offset() {
        let root = identity_test_root("large-resume");
        let session_id = "019e6470-ba33-7840-9015-79a294d89a15";
        let path = root.join(format!("rollout-2026-05-26T16-19-29-{session_id}.jsonl"));
        fs::write(
            &path,
            format!("{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{session_id}\"}}}}\n"),
        )
        .expect("session meta");

        let offset = 8 * 1024 * 1024 * 128;
        let state = canonical_session_state(&path, offset).expect("canonical state");
        assert_eq!(state.offset, offset);
        assert_eq!(state.session_id, session_id);
        assert!(state.session_identity_pinned);
        assert_ne!(
            state.session_id_sha256,
            sha256_bytes(path.to_string_lossy().as_bytes())
        );
        fs::remove_dir_all(root).expect("identity cleanup");
    }

    #[test]
    fn resumed_capture_skips_the_committed_row_and_waits_for_a_fresh_turn() {
        let root = identity_test_root("resume-censor");
        let session_path = root.join("session.jsonl");
        let evidence_root = root.join("evidence");
        let first_turn = [
            json!({"type":"session_meta","payload":{"id":"session-a"}}),
            json!({"type":"turn_context","payload":{"turn_id":"turn-1"}}),
            json!({"type":"event_msg","payload":{"type":"user_message","message":"continue"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":"exec-1","arguments":"{}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"exec-1","output":"Process running with session ID 101"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"write_stdin","call_id":"wait-1","arguments":"{\"session_id\":101,\"yield_time_ms\":1000}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"wait-1","output":"accepted"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":123}}}}),
        ];
        let mut session = File::create(&session_path).expect("session");
        for row in &first_turn {
            writeln!(session, "{row}").expect("first turn");
        }
        session.sync_all().expect("first turn sync");

        let evidence = Arc::new(Mutex::new(
            StreamingEvidenceLedger::open(&evidence_root, EvidencePolicyV1::streaming_bounded())
                .expect("streaming evidence"),
        ));
        let metrics = Arc::new(SessionStreamMetrics::default());
        let request_learning = Arc::new(RequestLearningIndex::default());
        let mut initial = canonical_session_state(&session_path, 0).expect("initial state");
        let first_frames = read_appended_frames(
            &session_path,
            &mut initial,
            SessionReadContext {
                evidence: &evidence,
                evidence_graphs: None,
                miner: None,
                direct_collection_miner: None,
                metrics: &metrics,
                request_learning: &request_learning,
            },
        )
        .expect("first capture");
        assert_eq!(first_frames.len(), 1);

        let (resume_offset, ingress_before_resume) = {
            let ledger = evidence.lock().expect("evidence lock");
            (
                ledger
                    .resume_offset(session_path.to_string_lossy().as_ref())
                    .expect("resume offset"),
                ledger.accounting().ingress_total,
            )
        };
        let mut resumed =
            canonical_session_state(&session_path, resume_offset).expect("resumed state");
        resumed.replay_source_offset = Some(resume_offset);
        resumed.censor_until_turn_boundary = true;
        assert!(
            read_appended_frames(
                &session_path,
                &mut resumed,
                SessionReadContext {
                    evidence: &evidence,
                    evidence_graphs: None,
                    miner: None,
                    direct_collection_miner: None,
                    metrics: &metrics,
                    request_learning: &request_learning,
                },
            )
            .expect("resume replay")
            .is_empty()
        );
        assert_eq!(
            evidence
                .lock()
                .expect("evidence lock")
                .accounting()
                .ingress_total,
            ingress_before_resume
        );

        writeln!(
            session,
            "{}",
            json!({"type":"response_item","payload":{"type":"message","role":"assistant","content":[]}})
        )
        .expect("partial turn");
        session.sync_all().expect("partial sync");
        assert!(
            read_appended_frames(
                &session_path,
                &mut resumed,
                SessionReadContext {
                    evidence: &evidence,
                    evidence_graphs: None,
                    miner: None,
                    direct_collection_miner: None,
                    metrics: &metrics,
                    request_learning: &request_learning,
                },
            )
            .expect("partial resume")
            .is_empty()
        );
        assert_eq!(
            evidence
                .lock()
                .expect("evidence lock")
                .accounting()
                .ingress_total,
            ingress_before_resume
        );

        let fresh_turn = [
            json!({"type":"turn_context","payload":{"turn_id":"turn-2"}}),
            json!({"type":"event_msg","payload":{"type":"user_message","message":"continue again"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":"exec-2","arguments":"{}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"exec-2","output":"Process running with session ID 202"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"write_stdin","call_id":"wait-2","arguments":"{\"session_id\":202,\"yield_time_ms\":1000}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"wait-2","output":"accepted"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":321}}}}),
        ];
        for row in &fresh_turn {
            writeln!(session, "{row}").expect("fresh turn");
        }
        session.sync_all().expect("fresh turn sync");
        let fresh_frames = read_appended_frames(
            &session_path,
            &mut resumed,
            SessionReadContext {
                evidence: &evidence,
                evidence_graphs: None,
                miner: None,
                direct_collection_miner: None,
                metrics: &metrics,
                request_learning: &request_learning,
            },
        )
        .expect("fresh capture");
        assert_eq!(fresh_frames.len(), 1);
        assert_eq!(
            evidence
                .lock()
                .expect("evidence lock")
                .accounting()
                .ingress_total,
            ingress_before_resume + fresh_turn.len() as u64
        );
        assert!(
            resumed
                .runtime_parity_cases
                .get(&fresh_frames[0].frame_id_sha256)
                .and_then(|case| case.capture_receipt.as_ref())
                .and_then(|receipt| receipt.transition_binding.as_ref())
                .is_some()
        );
        fs::remove_dir_all(root).expect("identity cleanup");
    }

    #[test]
    fn rollout_filename_mismatch_is_censored() {
        let root = identity_test_root("mismatch");
        let path =
            root.join("rollout-2026-05-26T16-19-29-019e6470-ba33-7840-9015-79a294d89a15.jsonl");
        fs::write(
            &path,
            b"{\"type\":\"session_meta\",\"payload\":{\"id\":\"019e3309-e6c6-75d0-80a9-25969b44266e\"}}\n",
        )
        .expect("mismatched session meta");

        assert_eq!(
            canonical_session_state(&path, 0)
                .err()
                .expect("mismatch must be censored"),
            "session_identity_filename_mismatch"
        );
        fs::remove_dir_all(root).expect("identity cleanup");
    }

    #[test]
    fn missing_session_meta_is_censored_without_path_fallback() {
        let root = identity_test_root("missing");
        let path = root.join("session.jsonl");
        fs::write(&path, b"{\"type\":\"turn_context\",\"payload\":{}}\n")
            .expect("session without meta");

        assert_eq!(
            canonical_session_state(&path, 0)
                .err()
                .expect("missing identity must be censored"),
            "session_identity_missing_meta"
        );
        assert!(verified_session_identity_sha256_candidates(&path).is_err());
        fs::remove_dir_all(root).expect("identity cleanup");
    }

    #[test]
    fn runtime_provider_payload_retains_all_outputs_in_the_active_turn() {
        let first_call = CallShape {
            name: "first".to_owned(),
            shape: "function_call".to_owned(),
        };
        let second_call = CallShape {
            name: "second".to_owned(),
            shape: "function_call".to_owned(),
        };
        let first = append_runtime_provider_output(
            None,
            None,
            None,
            "call-1",
            &first_call,
            &json!({"role": 41}),
        )
        .expect("first bounded output");
        let second = append_runtime_provider_output(
            Some(&first),
            None,
            None,
            "call-2",
            &second_call,
            &json!({"noise": 7}),
        )
        .expect("second bounded output");

        let input = second["input"].as_array().expect("provider input");
        assert_eq!(input.len(), 4);
        assert_eq!(input[1]["output"]["role"], 41);
        assert_eq!(input[3]["output"]["noise"], 7);
    }

    #[test]
    fn miner_bridge_drains_frames_captured_before_warmup() {
        let bridge = SessionMinerBridge::new();
        let frame = RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: "a".repeat(64),
            event_id_sha256: "b".repeat(64),
            client_intent_id_sha256: "c".repeat(64),
            session_id_sha256: "d".repeat(64),
            observed_at_unix_nanos: 1,
            estimated_input_tokens: 10,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: vec![RelationAtom::CompletionState {
                value: "completed".to_owned(),
            }],
            evidence_ref_sha256: "e".repeat(64),
        };
        bridge
            .submit_frame_with_parity(frame, None)
            .expect("buffer frame");
        assert_eq!(bridge.status(), (1, 0, false));

        let sink = Arc::new(RecordingMinerSink::default());
        bridge
            .install_sink(sink.clone())
            .expect("install recording sink");
        assert_eq!(bridge.status(), (0, 0, true));
        assert_eq!(
            sink.frame_ids.lock().expect("recorded frames").as_slice(),
            &["a".repeat(64)]
        );
    }

    #[test]
    fn compact_request_structure_joins_by_turn_and_session_without_raw_payload_storage() {
        let index = RequestLearningIndex::default();
        let structure = LearningRequestStructureV1::new(LearningRequestStructureInputV1 {
            client_intent_id_sha256: sha256_bytes(b"turn-a"),
            session_identity_sha256s: vec![sha256_bytes(b"session-a")],
            request_phase_atom_ids: vec![7, 9],
            pre_action_context_atom_ids: vec![11],
            capability_atom_ids: vec![17],
            provider_bound_turn_identity: true,
            estimated_input_tokens: 23,
            provider_payload_bytes: 29,
        })
        .expect("learning structure");
        index
            .observe_structure(&structure)
            .expect("compact learning evidence");
        let atoms = index.lookup(&sha256_bytes(b"session-a"), &sha256_bytes(b"turn-a"));
        assert_eq!(atoms.request_phase_atom_ids, vec![7, 9]);
        assert_eq!(atoms.capability_atom_ids, vec![17]);
        assert_eq!(
            index.lookup(&sha256_bytes(b"session-b"), &sha256_bytes(b"turn-b")),
            RequestLearningAtoms::default()
        );
    }

    #[test]
    fn scalar_tool_output_and_teacher_action_emit_one_grounded_frame() {
        let mut state = SessionState {
            session_id_sha256: "a".repeat(64),
            ..SessionState::default()
        };
        let mut emitted = Vec::new();
        for row in [
            json!({"type":"event_msg","payload":{"type":"user_message","message":"submit the observed count"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"lookup","call_id":"1","arguments":"{}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"1","output":"{\"count\":7}"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"submit","call_id":"2","arguments":"{\"value\":7}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"2","output":"accepted"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":123}}}}),
        ] {
            observe_row(&row, &mut state, &mut emitted);
        }
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].estimated_input_tokens, 123);
        assert!(emitted[0].atoms.iter().any(
            |atom| matches!(atom, RelationAtom::ActionFunction { value } if value == "submit")
        ));
        let expected_request_atoms =
            nando_response_actor::request_phase_atom_ids("submit the observed count");
        assert!(!expected_request_atoms.is_empty());
        assert!(expected_request_atoms.iter().all(|expected| {
            emitted[0].atoms.iter().any(
            |atom| matches!(atom, RelationAtom::RequestPhaseAtom { atom_id } if atom_id == expected)
        )
        }));
        let operator = synthesize_response_operator(&emitted).expect("generic function synthesis");
        assert!(frame_matches_program_action_contract(
            &operator.candidate.program,
            &emitted[0]
        ));
    }

    #[test]
    fn action_without_linked_tool_output_is_not_teacher_evidence() {
        let mut state = SessionState {
            session_id_sha256: "a".repeat(64),
            ..SessionState::default()
        };
        let mut emitted = Vec::new();
        for row in [
            json!({"type":"response_item","payload":{"type":"function_call","name":"lookup","call_id":"1","arguments":"{}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"1","output":"Process running with session ID abc"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"submit","call_id":"2","arguments":"{\"value\":\"abc\"}"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":123}}}}),
        ] {
            observe_row(&row, &mut state, &mut emitted);
        }
        assert!(emitted.is_empty());
    }

    #[test]
    fn completed_plan_transition_emits_typed_frame_only_after_explicit_success() {
        fn observe_plan_transition(tool_output: Value) -> (SessionState, Vec<RelationFrame>) {
            let mut state = SessionState {
                session_id_sha256: "a".repeat(64),
                ..SessionState::default()
            };
            let mut emitted = Vec::new();
            for row in [
                json!({"type":"event_msg","payload":{"type":"user_message","message":"apply the change"}}),
                json!({"type":"response_item","payload":{"type":"function_call","name":"update_plan","call_id":"plan-1","arguments":serde_json::to_string(&json!({"plan":[
                    {"step":"Inspect state","status":"in_progress"},
                    {"step":"Apply change","status":"pending"},
                    {"step":"Verify runtime","status":"pending"}
                ]})).expect("initial plan")}}),
                json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"plan-1","output":"Plan updated"}}),
                json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":"exec-1","arguments":"{\"cmd\":\"true\"}"}}),
                json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"exec-1","output":tool_output}}),
                json!({"type":"response_item","payload":{"type":"function_call","name":"update_plan","call_id":"plan-2","arguments":serde_json::to_string(&json!({"plan":[
                    {"step":"Inspect state","status":"completed"},
                    {"step":"Apply change","status":"in_progress"},
                    {"step":"Verify runtime","status":"pending"}
                ]})).expect("advanced plan")}}),
                json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"plan-2","output":"Plan updated"}}),
                json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":321}}}}),
            ] {
                observe_row(&row, &mut state, &mut emitted);
            }
            (state, emitted)
        }

        let (state, emitted) = observe_plan_transition(json!(
            "Chunk ID: plan\nWall time: 0.1 seconds\nProcess exited with code 0\nFinal output:\nverified"
        ));
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].estimated_input_tokens, 321);
        assert!(
            emitted[0]
                .atoms
                .iter()
                .any(|atom| matches!(atom, RelationAtom::ActionPlanAdvance))
        );
        assert!(emitted[0].atoms.iter().any(|atom| matches!(
            atom,
            RelationAtom::PlanState {
                step_count: 3,
                completed_count: 0,
                active_index: 0
            }
        )));
        let parity = state
            .runtime_parity_cases
            .get(&emitted[0].frame_id_sha256)
            .expect("plan runtime parity case");
        assert!(
            verify_response_independently(
                &VerifierProgram::AdvancePlan {
                    function_name: "update_plan".to_owned(),
                    require_explicit_tool_success: true,
                    require_canonical_plan: true,
                },
                &parity.provider_payload,
                &parity.expected_response,
            )
            .is_ok()
        );

        let (_, failed) = observe_plan_transition(json!(
            "Chunk ID: plan\nProcess exited with code 1\nFinal output:\nfailed"
        ));
        assert!(failed.is_empty());
    }

    #[test]
    fn custom_exec_wrapper_emits_typed_inner_tool_frame() {
        let mut state = SessionState {
            session_id_sha256: "a".repeat(64),
            ..SessionState::default()
        };
        let mut emitted = Vec::new();
        for row in [
            json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":"1","arguments":"{}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"1","output":"Process running with session ID abc"}}),
            json!({"timestamp":"2026-07-16T00:00:00Z","type":"response_item","payload":{"type":"custom_tool_call","name":"exec","call_id":"2","input":"const r = await tools.write_stdin({session_id:\"abc\",yield_time_ms:1000}); text(r.output);"}}),
            json!({"type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"2","output":"accepted"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":123}}}}),
        ] {
            observe_row(&row, &mut state, &mut emitted);
        }
        assert_eq!(emitted.len(), 1);
        assert!(emitted[0].atoms.iter().any(
            |atom| matches!(atom, RelationAtom::ActionCustomTool { value } if value == "exec")
        ));
        assert!(emitted[0].atoms.iter().any(
            |atom| matches!(atom, RelationAtom::ActionInnerTool { value } if value == "write_stdin")
        ));
        assert!(emitted[0].atoms.iter().any(
            |atom| matches!(atom, RelationAtom::ActionOutputProjection { output_field } if output_field == "output")
        ));
        let operator = synthesize_response_operator(&emitted).expect("custom tool synthesis");
        assert!(matches!(
            operator.candidate.program.operation,
            nando_response_actor::ResponseOperation::CustomToolCallFromRoles { .. }
        ));
        let parity = state
            .runtime_parity_cases
            .get(&emitted[0].frame_id_sha256)
            .expect("custom tool parity case");
        assert_eq!(
            serde_json::from_str::<Value>(&parity.expected_response).expect("parity response"),
            json!({
                "kind": "custom_tool_call",
                "name": "exec",
                "input": "const r=await tools.write_stdin({\"session_id\":\"abc\",\"yield_time_ms\":1000});text(r.output);",
            })
        );
    }

    #[test]
    fn custom_exec_array_output_grounds_embedded_json_handle() {
        let mut state = SessionState {
            session_id_sha256: "a".repeat(64),
            ..SessionState::default()
        };
        let mut emitted = Vec::new();
        for row in [
            json!({"type":"response_item","payload":{"type":"custom_tool_call","name":"exec","call_id":"1","input":"const r = await tools.exec_command({cmd:\"cargo build\"}); text(JSON.stringify(r));"}}),
            json!({"type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"1","output":[
                {"type":"input_text","text":""},
                {"type":"input_text","text":"{\"chunk_id\":\"abc\",\"session_id\":60906,\"output\":\"Compiling\"}"}
            ]}}),
            json!({"type":"response_item","payload":{"type":"custom_tool_call","name":"exec","call_id":"2","input":"const r = await tools.write_stdin({session_id:60906,chars:\"\",yield_time_ms:1000}); text(r.output);"}}),
            json!({"type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"2","output":[{"type":"input_text","text":"accepted"}]}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":123}}}}),
        ] {
            observe_row(&row, &mut state, &mut emitted);
        }
        assert_eq!(emitted.len(), 1);
        assert_eq!(
            teacher_action_symbol(&emitted[0]),
            "custom_tool:exec/write_stdin"
        );
        assert_eq!(emitted[0].verifier_label, Some(true));
        assert!(
            state
                .runtime_parity_cases
                .contains_key(&emitted[0].frame_id_sha256)
        );
    }

    fn capture_structured_custom_continuation(
        root: &Path,
        evidence: &Arc<Mutex<DeterministicEvidenceLedger>>,
        session_id: &str,
        turn_id: &str,
        session_value: u64,
    ) -> (RelationFrame, RuntimeParityCase) {
        let session_path = root.join(format!("{session_id}.jsonl"));
        let action_timestamp = format!("2026-07-23T00:00:{:02}Z", session_value % 60);
        let rows = [
            json!({"type":"session_meta","payload":{"id":session_id}}),
            json!({"type":"turn_context","payload":{"turn_id":turn_id}}),
            json!({"type":"event_msg","payload":{"type":"user_message","message":"continue the pending command"}}),
            json!({"type":"response_item","payload":{
                "type":"custom_tool_call",
                "name":"exec",
                "call_id":"start",
                "input":"const r=await tools.exec_command({\"cmd\":\"cargo check\"});text(JSON.stringify(r));"
            }}),
            json!({"type":"response_item","payload":{
                "type":"custom_tool_call_output",
                "call_id":"start",
                "output":[
                    {"type":"input_text","text":""},
                    {"type":"input_text","text":format!(
                        "{{\"chunk_id\":\"chunk-{session_value}\",\"session_id\":{session_value},\"output\":\"Compiling\"}}"
                    )}
                ]
            }}),
            json!({"timestamp":action_timestamp,"type":"response_item","payload":{
                "type":"custom_tool_call",
                "name":"exec",
                "call_id":"continue",
                "input":format!(
                    "const r=await tools.write_stdin({{\"session_id\":{session_value},\"chars\":\"\",\"yield_time_ms\":1000}});text(r.output);"
                )
            }}),
            json!({"type":"response_item","payload":{
                "type":"custom_tool_call_output",
                "call_id":"continue",
                "output":[{"type":"input_text","text":"accepted"}]
            }}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":123}}}}),
        ];
        let mut session = File::create(&session_path).expect("structured custom session");
        for row in rows {
            writeln!(session, "{row}").expect("structured custom row");
        }
        session.sync_all().expect("structured custom sync");

        let metrics = Arc::new(SessionStreamMetrics::default());
        let mut state = SessionState {
            session_id: session_path.to_string_lossy().into_owned(),
            ..SessionState::default()
        };
        let frames = read_appended_frames(
            &session_path,
            &mut state,
            SessionReadContext {
                evidence,
                evidence_graphs: None,
                miner: None,
                direct_collection_miner: None,
                metrics: &metrics,
                request_learning: &Arc::new(RequestLearningIndex::default()),
            },
        )
        .expect("capture structured custom continuation");
        assert_eq!(frames.len(), 1, "{frames:#?}");
        let frame = frames.into_iter().next().expect("captured frame");
        let parity = state
            .runtime_parity_cases
            .remove(&frame.frame_id_sha256)
            .expect("capture-bound runtime parity");
        assert!(parity.capture_receipt.as_ref().is_some_and(|receipt| {
            receipt.validate().is_ok()
                && receipt
                    .transition_binding
                    .as_ref()
                    .is_some_and(|binding| binding.frame_id_sha256 == frame.frame_id_sha256)
        }));
        (frame, parity)
    }

    fn replace_structured_session_output_with_ambiguous_roles(value: &mut Value) -> bool {
        match value {
            Value::String(text)
                if text.contains("\"chunk_id\"") && text.contains("\"session_id\"") =>
            {
                *text = "{\"chunk_id\":\"ambiguous\",\"session_id\":93139,\"other\":{\"session_id\":94140},\"output\":\"Compiling\"}".to_owned();
                true
            }
            Value::Array(values) => values
                .iter_mut()
                .any(replace_structured_session_output_with_ambiguous_roles),
            Value::Object(values) => values
                .values_mut()
                .any(replace_structured_session_output_with_ambiguous_roles),
            _ => false,
        }
    }

    #[test]
    fn offline_replay_returns_only_durable_capture_bound_cases() {
        let root = std::env::temp_dir().join(format!(
            "nando-capture-bound-replay-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("capture-bound replay root");
        let evidence = Arc::new(Mutex::new(
            DeterministicEvidenceLedger::open(
                root.join("fixture-evidence.jsonl"),
                EvidencePolicyV1::default(),
            )
            .expect("fixture evidence"),
        ));
        let session_id = "capture-bound-session";
        capture_structured_custom_continuation(
            &root,
            &evidence,
            session_id,
            "capture-bound-turn",
            60_906,
        );

        let cases = verified_capture_bound_training_cases_from_sessions(
            &[root.join(format!("{session_id}.jsonl"))],
            &root.join("durable-evidence"),
        )
        .expect("capture-bound replay");
        assert_eq!(cases.files_scanned, 1, "{cases:#?}");
        assert!(cases.censored_session_identities.is_empty(), "{cases:#?}");
        assert_eq!(cases.cases.len(), 1, "{cases:#?}");
        let (frame, parity) = &cases.cases[0];
        let receipt = parity.capture_receipt.as_ref().expect("capture receipt");
        receipt.validate().expect("valid capture receipt");
        assert_eq!(parity.evidence_ref_sha256, frame.frame_id_sha256);
        assert_eq!(
            receipt
                .transition_binding
                .as_ref()
                .expect("transition binding")
                .frame_id_sha256,
            frame.frame_id_sha256
        );

        fs::remove_dir_all(root).expect("capture-bound replay cleanup");
    }

    #[test]
    fn offline_replay_censors_mixed_session_identity_without_losing_batch() {
        let root = std::env::temp_dir().join(format!(
            "nando-mixed-session-replay-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("mixed session root");
        let mixed = root.join("mixed.jsonl");
        let mut file = File::create(&mixed).expect("mixed session file");
        writeln!(
            file,
            "{}",
            json!({"type":"session_meta","payload":{"id":"session-a"}})
        )
        .expect("first session");
        writeln!(
            file,
            "{}",
            json!({"type":"session_meta","payload":{"id":"session-b"}})
        )
        .expect("second session");
        file.sync_all().expect("mixed session sync");

        let batch = verified_capture_bound_training_cases_from_sessions(
            std::slice::from_ref(&mixed),
            &root.join("durable-evidence"),
        )
        .expect("censored batch");
        assert!(batch.cases.is_empty(), "{batch:#?}");
        assert_eq!(
            batch
                .censored_session_identities
                .get(&mixed.display().to_string()),
            Some(&"session_identity_changed".to_owned()),
            "{batch:#?}"
        );

        fs::remove_dir_all(root).expect("mixed session cleanup");
    }

    #[test]
    fn structured_custom_continuation_reaches_crystallized_cpu_operator() {
        let root = std::env::temp_dir().join(format!(
            "nando-structured-custom-operator-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("structured custom root");
        let evidence = Arc::new(Mutex::new(
            DeterministicEvidenceLedger::open(
                root.join("evidence.jsonl"),
                EvidencePolicyV1::default(),
            )
            .expect("structured custom evidence"),
        ));
        let mut shadow = nando_response_actor::LiveScalarShadowState::default();
        for (session, turn, value) in [
            ("support-session", "support-turn", 60_906),
            ("probe-session", "probe-turn", 71_017),
            ("future-session", "future-turn", 82_128),
        ] {
            let (frame, parity) =
                capture_structured_custom_continuation(&root, &evidence, session, turn, value);
            let mut transition =
                nando_response_actor::teacher_transition_from_completed(&frame, None)
                    .expect("structured custom teacher transition");
            transition.runtime_parity_case = Some(parity);
            shadow.observe(&transition);
        }

        let report = shadow.report();
        assert_eq!(report.support_rows, 1, "{report:#?}");
        assert_eq!(report.future_rows, 2, "{report:#?}");
        assert_eq!(report.admission_candidates, 1, "{report:#?}");
        let snapshot = nando_response_actor::build_crystallized_admission_snapshot(
            &shadow.admission_candidates(),
            "structured-custom-test",
            1,
            100,
            30,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("structured custom admission")
        .expect("structured custom registry");
        let executor = nando_response_actor::ResponseExecutor::from_registry(snapshot.registry)
            .expect("structured custom CPU executor");

        let (_, unseen) = capture_structured_custom_continuation(
            &root,
            &evidence,
            "unseen-session",
            "unseen-turn",
            93_139,
        );
        let execution = executor.execute_shadow(&unseen.request_text, &unseen.provider_payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Executed);
        let actual = execution
            .response
            .as_deref()
            .and_then(|response| serde_json::from_str::<Value>(response).ok())
            .expect("executed custom response");
        let expected =
            serde_json::from_str::<Value>(&unseen.expected_response).expect("teacher response");
        assert_eq!(actual.get("name"), expected.get("name"), "{execution:#?}");
        let actual_arguments = actual
            .get("input")
            .and_then(Value::as_str)
            .and_then(parse_custom_tool_source)
            .map(|source| Value::Object(source.arguments))
            .expect("executed custom arguments");
        let expected_arguments = expected
            .get("input")
            .and_then(Value::as_str)
            .and_then(parse_custom_tool_source)
            .map(|source| Value::Object(source.arguments))
            .expect("teacher custom arguments");
        assert_eq!(
            actual_arguments.get("session_id"),
            expected_arguments.get("session_id"),
            "{execution:#?}"
        );

        let mut ambiguous_payload = unseen.provider_payload;
        assert!(
            replace_structured_session_output_with_ambiguous_roles(&mut ambiguous_payload),
            "{ambiguous_payload:#?}"
        );
        let ambiguous = executor.execute_shadow(&unseen.request_text, &ambiguous_payload);
        assert_eq!(ambiguous.status, ResponseExecutionStatus::Abstain);
        fs::remove_dir_all(root).expect("structured custom cleanup");
    }

    #[test]
    fn bounded_session_head_recovers_custom_write_stdin_parity() {
        let path = std::env::temp_dir().join(format!(
            "nando-custom-head-parity-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let rows = [
            json!({"type":"session_meta","payload":{"id":"session-a"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":"1","arguments":"{}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"1","output":"Process running with session ID abc"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100}}}}),
            json!({"type":"response_item","payload":{"type":"custom_tool_call","name":"exec","call_id":"2","input":"const r = await tools.write_stdin({session_id:\"abc\",yield_time_ms:1000}); text(r.output);"}}),
            json!({"type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"2","output":"accepted"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":123}}}}),
        ];
        let mut file = File::create(&path).expect("session file");
        for row in rows {
            writeln!(file, "{row}").expect("session row");
        }
        file.flush().expect("head flush");
        let head_bytes = file.metadata().expect("head metadata").len();
        for index in 0..128 {
            writeln!(
                file,
                "{}",
                json!({"type":"event_msg","payload":{"type":"user_message","message":format!("filler-{index}")}})
            )
            .expect("filler row");
        }
        drop(file);

        let head = verified_training_cases_from_session_head(&path, head_bytes)
            .expect("head training cases");
        assert_eq!(head.len(), 1);
        assert!(head[0].1.is_some());
        assert_eq!(
            teacher_action_symbol(&head[0].0),
            "custom_tool:exec/write_stdin"
        );
        let sparse = verified_write_stdin_training_cases_from_session(&path)
            .expect("sparse custom training cases");
        assert_eq!(sparse.len(), 1);
        assert!(sparse[0].1.is_some());
        assert_eq!(sparse[0].0.frame_id_sha256, head[0].0.frame_id_sha256);
        let tail =
            verified_training_cases_from_session_tail(&path, 128).expect("tail training cases");
        assert!(tail.is_empty());
        fs::remove_file(path).ok();
    }

    #[test]
    fn invalid_utf8_censors_only_the_partial_turn() {
        let path = std::env::temp_dir().join(format!(
            "nando-invalid-utf8-session-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let mut file = File::create(&path).expect("session file");
        let first_turn = [
            json!({"type":"session_meta","payload":{"id":"invalid-utf8-session"}}),
            json!({"type":"turn_context","payload":{"turn_id":"turn-before-corruption"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":"exec-before","arguments":"{}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"exec-before","output":"Process running with session ID 11"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"write_stdin","call_id":"wait-before","arguments":"{\"session_id\":11,\"yield_time_ms\":1000}"}}),
        ];
        for row in first_turn {
            writeln!(file, "{row}").expect("first turn row");
        }
        file.write_all(b"{\"corrupt\":\"\xff\"}\n")
            .expect("corrupt row");
        let second_turn = [
            json!({"type":"turn_context","payload":{"turn_id":"turn-after-corruption"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":"exec-after","arguments":"{}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"exec-after","output":"Process running with session ID 22"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"write_stdin","call_id":"wait-after","arguments":"{\"session_id\":22,\"yield_time_ms\":1000}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"wait-after","output":"accepted"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":123}}}}),
        ];
        for row in second_turn {
            writeln!(file, "{row}").expect("second turn row");
        }
        drop(file);

        let cases =
            verified_training_cases_from_session(&path).expect("training after corrupt row");
        fs::remove_file(path).ok();

        assert_eq!(cases.len(), 1);
        assert_eq!(teacher_action_symbol(&cases[0].0), "function:write_stdin");
        assert!(cases[0].1.is_some());
    }

    #[test]
    fn tail_backfill_waits_for_an_authoritative_turn_boundary() {
        let path = std::env::temp_dir().join(format!(
            "nando-tail-turn-boundary-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let mut file = File::create(&path).expect("session file");
        for row in [
            json!({"type":"session_meta","payload":{"id":"tail-boundary-session"}}),
            json!({"type":"turn_context","payload":{"turn_id":"partial-turn"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":"partial-exec","arguments":"{}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"partial-exec","output":"Process running with session ID 11"}}),
        ] {
            writeln!(file, "{row}").expect("prefix row");
        }
        let tail_offset = file.stream_position().expect("tail offset");
        for row in [
            json!({"type":"response_item","payload":{"type":"function_call","name":"write_stdin","call_id":"partial-wait","arguments":"{\"session_id\":11,\"yield_time_ms\":1000}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"partial-wait","output":"accepted"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":111}}}}),
            json!({"type":"turn_context","payload":{"turn_id":"complete-turn"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":"complete-exec","arguments":"{}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"complete-exec","output":"Process running with session ID 22"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"write_stdin","call_id":"complete-wait","arguments":"{\"session_id\":22,\"yield_time_ms\":1000}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"complete-wait","output":"accepted"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":222}}}}),
        ] {
            writeln!(file, "{row}").expect("tail row");
        }
        drop(file);

        let cases = training_cases_from_session_at(&path, tail_offset).expect("bounded tail cases");
        fs::remove_file(path).ok();

        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].0.estimated_input_tokens, 222);
        assert!(cases[0].1.is_some());
    }

    #[test]
    fn sparse_backfill_recovers_direct_write_stdin_parity() {
        let path = std::env::temp_dir().join(format!(
            "nando-direct-write-stdin-parity-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let rows = [
            json!({"type":"session_meta","payload":{"id":"session-direct"}}),
            json!({"type":"event_msg","payload":{"type":"user_message","message":"continue the pending command"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":"exec","arguments":"{}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"exec","output":"Process running with session ID 4242"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"write_stdin","call_id":"wait","arguments":"{\"session_id\":4242,\"yield_time_ms\":1000}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"wait","output":"accepted"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":123}}}}),
        ];
        let mut file = File::create(&path).expect("session file");
        for row in rows {
            writeln!(file, "{row}").expect("session row");
        }
        drop(file);

        let cases = verified_write_stdin_training_cases_from_session(&path)
            .expect("direct write_stdin cases");
        fs::remove_file(path).ok();
        assert_eq!(cases.len(), 1);
        assert_eq!(teacher_action_symbol(&cases[0].0), "function:write_stdin");
        assert!(cases[0].1.is_some());
    }

    #[test]
    fn relation_backfill_replays_request_phases_without_raw_request_text() {
        let path = std::env::temp_dir().join(format!(
            "nando-response-relation-backfill-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let turn_id = "019f8c37-675d-7211-8034-af56fb7d5da5";
        let rows = [
            json!({"type":"session_meta","payload":{"id":"relation-backfill-session"}}),
            json!({"type":"turn_context","payload":{"turn_id":turn_id}}),
            json!({"type":"event_msg","payload":{"type":"user_message","message":"submit the observed count"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"lookup","call_id":"1","arguments":"{}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"1","output":"{\"count\":7}"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"submit","call_id":"2","arguments":"{\"value\":7}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"2","output":"accepted"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":123}}}}),
        ];
        let mut file = File::create(&path).expect("session file");
        for row in rows {
            writeln!(file, "{row}").expect("session row");
        }
        drop(file);
        let frames = relation_frames_from_session(&path).expect("backfill frames");
        fs::remove_file(&path).ok();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].estimated_input_tokens, 123);
        assert_eq!(
            frames[0].client_intent_id_sha256,
            sha256_bytes(turn_id.as_bytes())
        );
        let encoded = serde_json::to_string(&frames[0]).expect("frame json");
        assert!(!encoded.contains("submit the observed count"));
        assert!(
            frames[0]
                .atoms
                .iter()
                .any(|atom| matches!(atom, RelationAtom::RequestPhaseAtom { .. }))
        );
    }

    #[test]
    fn multi_turn_batch_retains_runtime_parity_until_frames_are_delivered() {
        let root = std::env::temp_dir().join(format!(
            "nando-session-parity-outbox-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let session_path = root.join("session.jsonl");
        let mut rows = vec![
            json!({"timestamp":"2026-07-14T00:00:00Z","type":"session_meta","payload":{"id":"session-a"}}),
        ];
        for (turn, cell_id) in [(1, "101"), (2, "202")] {
            rows.extend([
                json!({"timestamp":format!("2026-07-14T00:00:{turn:02}Z"),"type":"turn_context","payload":{}}),
                json!({"type":"event_msg","payload":{"type":"user_message","message":"continue the pending command"}}),
                json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":format!("exec-{turn}"),"arguments":"{}"}}),
                json!({"type":"response_item","payload":{"type":"function_call_output","call_id":format!("exec-{turn}"),"output":format!("Process running with session ID {cell_id}")}}),
                json!({"type":"response_item","payload":{"type":"function_call","name":"write_stdin","call_id":format!("wait-{turn}"),"arguments":format!("{{\"session_id\":{cell_id},\"yield_time_ms\":1000}}")}}),
                json!({"type":"response_item","payload":{"type":"function_call_output","call_id":format!("wait-{turn}"),"output":"accepted"}}),
                json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":123}}}}),
            ]);
        }
        let mut session = File::create(&session_path).expect("session");
        for row in &rows {
            serde_json::to_writer(&mut session, row).expect("row");
            session.write_all(b"\n").expect("newline");
        }
        session.sync_all().expect("sync");

        let evidence = Arc::new(Mutex::new(
            DeterministicEvidenceLedger::open(
                root.join("evidence.jsonl"),
                EvidencePolicyV1::default(),
            )
            .expect("ledger"),
        ));
        let metrics = Arc::new(SessionStreamMetrics::default());
        let mut state = SessionState {
            session_id: session_path.to_string_lossy().into_owned(),
            ..SessionState::default()
        };
        let frames = read_appended_frames(
            &session_path,
            &mut state,
            SessionReadContext {
                evidence: &evidence,
                evidence_graphs: None,
                miner: None,
                direct_collection_miner: None,
                metrics: &metrics,
                request_learning: &Arc::new(RequestLearningIndex::default()),
            },
        )
        .expect("capture");

        assert_eq!(frames.len(), 2);
        assert!(frames.iter().all(|frame| {
            state
                .runtime_parity_cases
                .get(&frame.frame_id_sha256)
                .is_some_and(|case| {
                    case.evidence_ref_sha256 == frame.frame_id_sha256
                        && case.capture_receipt.as_ref().is_some_and(|receipt| {
                            receipt.validate().is_ok()
                                && !receipt.records.is_empty()
                                && receipt.transition_binding.as_ref().is_some_and(|binding| {
                                    binding.frame_id_sha256 == frame.frame_id_sha256
                                })
                        })
                })
        }));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn session_tail_alignment_skips_partial_utf8_json_line() {
        let path = std::env::temp_dir().join(format!(
            "nando-session-alignment-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let first =
            json!({"message":"\u{043f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}"}).to_string();
        let second = json!({"message":"next"}).to_string();
        fs::write(&path, format!("{first}\n{second}\n")).expect("session");
        let bytes = fs::read(&path).expect("session bytes");
        let utf8_start = bytes
            .windows("\u{043f}".len())
            .position(|window| window == "\u{043f}".as_bytes())
            .expect("utf8 marker");

        let (mut reader, aligned_offset) =
            aligned_session_reader(File::open(&path).expect("open"), (utf8_start + 1) as u64)
                .expect("align");
        let mut line = String::new();
        reader.read_line(&mut line).expect("next line");

        assert_eq!(aligned_offset, (first.len() + 1) as u64);
        assert_eq!(line, format!("{second}\n"));
        fs::remove_file(path).ok();
    }

    #[test]
    fn runtime_parity_uses_latest_output_not_accumulated_collection_history() {
        let mut state = SessionState {
            session_id_sha256: "a".repeat(64),
            ..SessionState::default()
        };
        let mut emitted = Vec::new();
        observe_row(
            &json!({"type":"event_msg","payload":{"type":"user_message","message":"continue the pending command"}}),
            &mut state,
            &mut emitted,
        );
        for index in 0..3 {
            observe_row(
                &json!({"type":"response_item","payload":{"type":"function_call","name":"unrelated","call_id":format!("large-{index}"),"arguments":"{}"}}),
                &mut state,
                &mut emitted,
            );
            observe_row(
                &json!({"type":"response_item","payload":{"type":"function_call_output","call_id":format!("large-{index}"),"output":"x".repeat(50_000)}}),
                &mut state,
                &mut emitted,
            );
        }
        observe_row(
            &json!({"type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":"exec","arguments":"{}"}}),
            &mut state,
            &mut emitted,
        );
        observe_row(
            &json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"exec","output":[
                {"type":"input_text","text":"Script running with cell ID 2911\nOutput:\n"},
                {"type":"input_text","text":"{\"session_id\":2911}"}
            ]}}),
            &mut state,
            &mut emitted,
        );
        assert!(
            serde_json::to_vec(
                state
                    .collection_provider_payload
                    .as_ref()
                    .expect("collection")
            )
            .expect("collection json")
            .len()
                > 131_072
        );

        observe_row(
            &json!({"type":"response_item","payload":{"type":"function_call","name":"write_stdin","call_id":"wait","arguments":"{\"cell_id\":\"2911\",\"yield_time_ms\":1000}"}}),
            &mut state,
            &mut emitted,
        );

        let parity = state
            .runtime_parity_cases
            .values()
            .next()
            .expect("runtime parity fixture");
        let encoded = serde_json::to_vec(&parity.provider_payload).expect("parity json");
        assert!(encoded.len() < 70_000);
        assert_eq!(
            parity.provider_payload["input"]
                .as_array()
                .and_then(|input| input.last())
                .and_then(|item| item.get("output"))
                .and_then(Value::as_array)
                .and_then(|parts| parts.first())
                .and_then(|part| part.get("text"))
                .and_then(Value::as_str),
            Some("Script running with cell ID 2911\nOutput:\n")
        );
    }

    #[test]
    fn scalar_assistant_projection_emits_synthesizable_typed_binding() {
        let mut state = SessionState {
            session_id_sha256: "a".repeat(64),
            ..SessionState::default()
        };
        let mut emitted = Vec::new();
        for row in [
            json!({"type":"response_item","payload":{"type":"function_call","name":"lookup","call_id":"1","arguments":"{}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"1","output":"{\"count\":7}"}}),
            json!({"type":"event_msg","payload":{"type":"agent_message","phase":"final_answer","message":"7"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":123}}}}),
        ] {
            observe_row(&row, &mut state, &mut emitted);
        }
        assert_eq!(emitted.len(), 1);
        let operator = synthesize_response_operator(&emitted).expect("projection synthesis");
        assert!(frame_matches_program_action_contract(
            &operator.candidate.program,
            &emitted[0]
        ));
    }

    #[test]
    fn assistant_renderer_captures_two_source_roles_without_raw_output_payload() {
        let call = CallShape {
            name: "lookup".to_owned(),
            shape: "function_call".to_owned(),
        };
        let observations = scalar_observations(
            &json!("{\"city\":\"Tallinn\",\"temperature\":7}"),
            &call,
            &"e".repeat(64),
        );
        let mut state = SessionState {
            session_id_sha256: "a".repeat(64),
            observations,
            ..SessionState::default()
        };
        let row = json!({"type":"event_msg","payload":{"type":"agent_message"}});

        let frames = assistant_frames(&row, "Tallinn: 7 C", &mut state);

        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0]
                .atoms
                .iter()
                .filter(|atom| matches!(atom, RelationAtom::ObservationSelector { .. }))
                .count(),
            2
        );
        assert_eq!(
            frames[0]
                .atoms
                .iter()
                .filter(|atom| matches!(atom, RelationAtom::SlotEquality { .. }))
                .count(),
            2
        );
        let renderer = frames[0].atoms.iter().find_map(|atom| match atom {
            RelationAtom::ActionValueProjection { renderer, .. } => Some(renderer),
            _ => None,
        });
        assert!(matches!(
            renderer,
            Some(CollectionOutputRenderer::RenderSequence { segments })
                if segments.iter().filter(|segment| matches!(
                    segment,
                    ResponseRenderSegment::Primary | ResponseRenderSegment::Selected { .. }
                )).count() == 2
        ));
        let encoded = serde_json::to_string(&frames[0]).expect("frame");
        assert!(!encoded.contains("{\"city\""));
    }

    #[test]
    fn yielded_cell_is_extracted_structurally_for_wait() {
        let call = CallShape {
            name: "exec".to_owned(),
            shape: "custom_tool_call".to_owned(),
        };
        let observations = scalar_observations(
            &Value::String("Script running with cell ID 2911\n".to_owned()),
            &call,
            &"e".repeat(64),
        );
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].value_sha256, hash_value(&json!("2911")));
        assert!(matches!(
            observations[0].selector,
            ResponseValueSelector::ContentLinePrefix { .. }
        ));
        assert!(argument_matches_observation(&json!(2911), &observations[0]));
        let legacy = scalar_observations(
            &Value::String(
                "Chunk ID: abc\nProcess running with session ID 33605\nOutput:\n".to_owned(),
            ),
            &call,
            &"f".repeat(64),
        );
        assert_eq!(legacy.len(), 1);
        assert!(argument_matches_observation(&json!(33605), &legacy[0]));
        assert_eq!(observations[0].completion_state, "pending");
        assert_eq!(legacy[0].completion_state, "pending");
    }

    #[test]
    fn unique_identifier_binding_wins_over_incidental_numeric_matches() {
        let identifier = Observation {
            value_sha256: hash_value(&json!("2911")),
            value_type: AtomValueType::Identifier,
            render_value: Some("2911".to_owned()),
            selector: ResponseValueSelector::ContentLinePrefix {
                prefix: "Script running with cell ID ".to_owned(),
                value_type: AtomValueType::Identifier,
            },
            tool_kind: "exec".to_owned(),
            call_shape: "custom_tool_call".to_owned(),
            output_sha256: "e".repeat(64),
            completion_state: "pending",
        };
        let incidental = Observation {
            value_sha256: hash_value(&json!(1000)),
            value_type: AtomValueType::Integer,
            render_value: Some("1000".to_owned()),
            selector: ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::Integer,
            },
            tool_kind: "exec".to_owned(),
            call_shape: "custom_tool_call".to_owned(),
            output_sha256: "f".repeat(64),
            completion_state: "completed",
        };
        let mut state = SessionState {
            session_id_sha256: "a".repeat(64),
            observations: vec![identifier, incidental],
            ..SessionState::default()
        };
        let row = json!({"type":"response_item","payload":{}});
        let payload = json!({
            "name":"write_stdin",
            "arguments":"{\"session_id\":2911,\"yield_time_ms\":1000}"
        });
        let frames = action_frames(
            &row,
            payload.as_object().expect("action payload"),
            &mut state,
        );
        assert_eq!(frames.len(), 1);
        assert!(frames[0].atoms.iter().any(|atom| matches!(
            atom,
            RelationAtom::ActionRoleArgument { name, .. } if name == "session_id"
        )));
    }

    #[test]
    fn multi_role_action_emits_every_unambiguous_source_binding() {
        let left = Observation {
            value_sha256: hash_value(&json!(7)),
            value_type: AtomValueType::Integer,
            render_value: Some("7".to_owned()),
            selector: ResponseValueSelector::JsonField {
                field: "alpha".to_owned(),
                value_type: AtomValueType::Integer,
            },
            tool_kind: "lookup".to_owned(),
            call_shape: "function_call".to_owned(),
            output_sha256: "a".repeat(64),
            completion_state: "completed",
        };
        let right = Observation {
            value_sha256: hash_value(&json!("ok")),
            value_type: AtomValueType::String,
            render_value: Some("ok".to_owned()),
            selector: ResponseValueSelector::JsonField {
                field: "beta".to_owned(),
                value_type: AtomValueType::String,
            },
            tool_kind: "lookup".to_owned(),
            call_shape: "function_call".to_owned(),
            output_sha256: "b".repeat(64),
            completion_state: "completed",
        };
        let mut state = SessionState {
            session_id_sha256: "a".repeat(64),
            observations: vec![left, right],
            ..SessionState::default()
        };
        let row = json!({"type":"response_item","payload":{}});
        let payload = json!({
            "name":"combine",
            "arguments":"{\"left\":7,\"right\":\"ok\",\"mode\":\"exact\"}"
        });

        let frames = action_frames(
            &row,
            payload.as_object().expect("action payload"),
            &mut state,
        );

        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0]
                .atoms
                .iter()
                .filter(|atom| matches!(atom, RelationAtom::ActionRoleArgument { .. }))
                .count(),
            2
        );
        assert_eq!(
            frames[0]
                .atoms
                .iter()
                .filter(|atom| matches!(atom, RelationAtom::SlotEquality { .. }))
                .count(),
            2
        );
        assert!(frames[0].atoms.iter().any(|atom| matches!(
            atom,
            RelationAtom::ActionStringArgument { name, value }
                if name == "mode" && value == "exact"
        )));
    }

    #[test]
    fn duplicate_extraction_of_same_structural_selector_is_one_binding() {
        let first = Observation {
            value_sha256: hash_value(&json!(2911)),
            value_type: AtomValueType::Identifier,
            render_value: Some("2911".to_owned()),
            selector: ResponseValueSelector::LatestTurnOutputScalarOrdinal {
                scalar_ordinal: 0,
                value_type: AtomValueType::Identifier,
            },
            tool_kind: "exec".to_owned(),
            call_shape: "custom_tool_call".to_owned(),
            output_sha256: "a".repeat(64),
            completion_state: "pending",
        };
        let mut duplicate = first.clone();
        duplicate.output_sha256 = "b".repeat(64);

        let selected =
            unique_structural_action_match(&[(&first, "session_id"), (&duplicate, "session_id")]);

        assert!(selected.is_some());
    }

    #[test]
    fn equal_values_from_different_structural_selectors_remain_ambiguous() {
        let first = Observation {
            value_sha256: hash_value(&json!(2911)),
            value_type: AtomValueType::Identifier,
            render_value: Some("2911".to_owned()),
            selector: ResponseValueSelector::LatestTurnOutputScalarOrdinal {
                scalar_ordinal: 0,
                value_type: AtomValueType::Identifier,
            },
            tool_kind: "exec".to_owned(),
            call_shape: "custom_tool_call".to_owned(),
            output_sha256: "a".repeat(64),
            completion_state: "pending",
        };
        let mut competing = first.clone();
        competing.selector = ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal: 1,
            value_type: AtomValueType::Identifier,
        };

        let selected =
            unique_structural_action_match(&[(&first, "session_id"), (&competing, "session_id")]);

        assert!(selected.is_none());
    }

    #[test]
    fn frame_identity_distinguishes_repeated_content_at_different_offsets() {
        let row =
            json!({"type":"response_item","payload":{"type":"function_call","name":"submit"}});
        let atoms = vec![RelationAtom::CompletionState {
            value: "completed".to_owned(),
        }];
        let mut state = SessionState {
            session_id_sha256: "a".repeat(64),
            offset: 100,
            ..SessionState::default()
        };
        let first = build_frame(&row, &state, atoms.clone(), true, &"e".repeat(64));
        state.offset = 200;
        let second = build_frame(&row, &state, atoms, true, &"e".repeat(64));
        assert_ne!(first.frame_id_sha256, second.frame_id_sha256);
        assert_eq!(first.evidence_ref_sha256, second.evidence_ref_sha256);
    }

    #[test]
    fn total_capture_accounts_rows_and_retains_multiple_outputs_without_raw_text() {
        let root = std::env::temp_dir().join(format!(
            "nando-session-total-capture-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp dir");
        let session_path = root.join("session.jsonl");
        let ledger_path = root.join("evidence.jsonl");
        let graph_path = root.join("evidence-graphs.jsonl");
        let collection_path = root.join("online-collection.json");
        let rows = [
            json!({"timestamp":"2026-07-14T00:00:00Z","type":"session_meta","payload":{"id":"private-session"}}),
            json!({"timestamp":"2026-07-14T00:00:01Z","type":"turn_context","payload":{}}),
            json!({"timestamp":"2026-07-14T00:00:02Z","type":"response_item","payload":{"type":"function_call","name":"first","call_id":"call-1","arguments":"{}"}}),
            json!({"timestamp":"2026-07-14T00:00:03Z","type":"response_item","payload":{"type":"function_call_output","call_id":"call-1","output":"{\"customer\":\"Alice Private\",\"first\":7}"}}),
            json!({"timestamp":"2026-07-14T00:00:04Z","type":"response_item","payload":{"type":"function_call","name":"second","call_id":"call-2","arguments":"{}"}}),
            json!({"timestamp":"2026-07-14T00:00:05Z","type":"response_item","payload":{"type":"function_call_output","call_id":"call-2","output":"{\"second\":9}"}}),
        ];
        let mut session = File::create(&session_path).expect("session");
        for row in &rows {
            serde_json::to_writer(&mut session, row).expect("row");
            session.write_all(b"\n").expect("newline");
        }
        session.sync_all().expect("session sync");

        let evidence = Arc::new(Mutex::new(
            DeterministicEvidenceLedger::open(&ledger_path, EvidencePolicyV1::default())
                .expect("ledger"),
        ));
        let evidence_graphs = Arc::new(Mutex::new(
            DeterministicEvidenceGraphStore::open(&graph_path).expect("graph store"),
        ));
        let collection_miner = Arc::new(Mutex::new(
            OnlineCollectionMiner::open(
                &collection_path,
                nando_response_actor::OnlineCollectionConfig::default(),
            )
            .expect("collection miner"),
        ));
        let metrics = Arc::new(SessionStreamMetrics::default());
        let mut state = SessionState {
            session_id: session_path.to_string_lossy().into_owned(),
            ..SessionState::default()
        };
        let frames = read_appended_frames(
            &session_path,
            &mut state,
            SessionReadContext {
                evidence: &evidence,
                evidence_graphs: Some(&evidence_graphs),
                miner: None,
                direct_collection_miner: Some(&collection_miner),
                metrics: &metrics,
                request_learning: &Arc::new(RequestLearningIndex::default()),
            },
        )
        .expect("capture");
        assert!(frames.is_empty());
        assert_eq!(state.observations.len(), 3);
        assert_eq!(state.output_count, 2);
        let ledger = evidence.lock().expect("ledger lock");
        assert_eq!(ledger.accounting().ingress_total, rows.len() as u64);
        assert!(ledger.accounting().identity_holds());
        assert!(
            ledger
                .resume_offset(session_path.to_string_lossy().as_ref())
                .is_some()
        );
        drop(ledger);
        finalize_turn_evidence_graph(&mut state, Some(&evidence_graphs), &metrics)
            .expect("turn graph");
        assert_eq!(
            evidence_graphs
                .lock()
                .expect("graph lock")
                .status()
                .graph_total,
            1
        );
        let durable =
            String::from_utf8(fs::read(&ledger_path).expect("ledger bytes")).expect("ledger utf8");
        let durable_graph =
            String::from_utf8(fs::read(&graph_path).expect("graph bytes")).expect("graph utf8");
        for private in ["private-session", "Alice Private", "customer"] {
            assert!(!durable.contains(private), "durable raw leak: {private}");
            assert!(
                !durable_graph.contains(private),
                "durable graph raw leak: {private}"
            );
        }
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn live_turn_feeds_collection_version_space_without_persisting_raw_example() {
        let root = std::env::temp_dir().join(format!(
            "nando-session-collection-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let session_path = root.join("session.jsonl");
        let rows = [
            json!({"timestamp":"2026-07-14T00:00:00Z","type":"session_meta","payload":{"id":"session-private"}}),
            json!({"timestamp":"2026-07-14T00:00:01Z","type":"turn_context","payload":{}}),
            json!({"timestamp":"2026-07-14T00:00:02Z","type":"response_item","payload":{"type":"function_call","name":"query","call_id":"call-1","arguments":"{}"}}),
            json!({"timestamp":"2026-07-14T00:00:03Z","type":"response_item","payload":{"type":"function_call_output","call_id":"call-1","output":"{\"private_rows\":[{\"customer\":1},{\"customer\":2}]}"}}),
            json!({"timestamp":"2026-07-14T00:00:04Z","type":"event_msg","payload":{"type":"agent_message","phase":"final_answer","message":"2"}}),
            json!({"timestamp":"2026-07-14T00:00:05Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":321}}}}),
        ];
        let mut session = File::create(&session_path).expect("session");
        for row in &rows {
            serde_json::to_writer(&mut session, row).expect("row");
            session.write_all(b"\n").expect("newline");
        }
        session.sync_all().expect("sync");
        let evidence = Arc::new(Mutex::new(
            DeterministicEvidenceLedger::open(
                root.join("evidence.jsonl"),
                EvidencePolicyV1::default(),
            )
            .expect("ledger"),
        ));
        let evidence_graphs = Arc::new(Mutex::new(
            DeterministicEvidenceGraphStore::open(root.join("graphs.jsonl")).expect("graphs"),
        ));
        let collection_path = root.join("collection.json");
        let collection_miner = Arc::new(Mutex::new(
            OnlineCollectionMiner::open(
                &collection_path,
                nando_response_actor::OnlineCollectionConfig::default(),
            )
            .expect("miner"),
        ));
        let metrics = Arc::new(SessionStreamMetrics::default());
        let mut state = SessionState {
            session_id: session_path.to_string_lossy().into_owned(),
            ..SessionState::default()
        };
        read_appended_frames(
            &session_path,
            &mut state,
            SessionReadContext {
                evidence: &evidence,
                evidence_graphs: Some(&evidence_graphs),
                miner: None,
                direct_collection_miner: Some(&collection_miner),
                metrics: &metrics,
                request_learning: &Arc::new(RequestLearningIndex::default()),
            },
        )
        .expect("capture");
        let status = collection_miner.lock().expect("lock").status();
        assert_eq!(status.observations_total, 1);
        assert!(!status.buckets.is_empty());
        assert!(
            status.buckets.len()
                <= nando_response_actor::OnlineCollectionConfig::default().max_buckets
        );
        assert_eq!(status.support_receipts_unique_total, 1);
        assert_eq!(status.support_tokens_unique_total, 321);
        let durable = fs::read(&collection_path).expect("checkpoint");
        for private in ["private_rows", "customer", "session-private"] {
            assert!(
                !durable
                    .windows(private.len())
                    .any(|window| window == private.as_bytes()),
                "checkpoint leaked {private}"
            );
        }
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn live_collection_binding_is_archived_but_offline_replay_cannot_mint_it() {
        let root = std::env::temp_dir().join(format!(
            "nando-session-collection-binding-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let session_path = root.join("session.jsonl");
        let rows = [
            json!({"type":"session_meta","payload":{"id":"capture-owned-session"}}),
            json!({"type":"turn_context","payload":{"turn_id":"capture-owned-turn"}}),
            json!({"type":"response_item","payload":{"type":"function_call","name":"query","call_id":"call-1","arguments":"{}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"call-1","output":"{\"rows\":[1,2]}"}}),
            json!({"type":"event_msg","payload":{"type":"agent_message","phase":"final_answer","message":"2"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":321}}}}),
        ];
        let mut session = File::create(&session_path).expect("session");
        for row in rows {
            writeln!(session, "{row}").expect("row");
        }
        session.sync_all().expect("sync");

        let evidence_root = root.join("streaming-evidence");
        let evidence = Arc::new(Mutex::new(
            StreamingEvidenceLedger::open(&evidence_root, EvidencePolicyV1::streaming_bounded())
                .expect("streaming evidence"),
        ));
        let sink = CaptureOwnedCollectionSink::default();
        let metrics = Arc::new(SessionStreamMetrics::default());
        let mut state = canonical_session_state(&session_path, 0).expect("session state");
        read_appended_frames(
            &session_path,
            &mut state,
            SessionReadContext {
                evidence: &evidence,
                evidence_graphs: None,
                miner: Some(&sink),
                direct_collection_miner: None,
                metrics: &metrics,
                request_learning: &Arc::new(RequestLearningIndex::default()),
            },
        )
        .expect("live capture");
        let observations = sink.observations.lock().expect("observations");
        let binding = observations[0]
            .capture_binding
            .as_ref()
            .expect("capture binding");
        nando_response_actor::CaptureTransitionBindingArchiveReader::open(&evidence_root)
            .expect("binding archive")
            .verify_binding(binding)
            .expect("archived binding");
        nando_operator_learning::CaptureCommitmentArchiveReader::open(&evidence_root)
            .expect("commitment archive")
            .verify_record(&binding.source_record)
            .expect("archived source record");
        drop(observations);

        let replay =
            verified_collection_observations_from_session(&session_path).expect("offline replay");
        assert_eq!(replay.len(), 1);
        assert!(replay[0].capture_binding.is_none());
        fs::remove_dir_all(root).expect("cleanup");
    }
}
