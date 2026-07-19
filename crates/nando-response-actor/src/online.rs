use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nando_core::wave::{
    PhaseCenterAtomEncoder, PhaseCenterOnlineMiner, PhaseCenterOnlineMinerConfig,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::teacher_join::action_schema_enriched_frame;
use crate::{
    ECONOMICS_RECEIPT_SCHEMA_V1, EconomicsReceipt, RelationAtom, RelationFrame, ResponseProgram,
    SelfTrainingStateReport, StreamingSelfTrainingState, VerifierProgram, ground_roles,
    is_source_neutral_relation_frame, relation_frame_online_routing_atom_ids,
    synthesis::grounded_program_family_id, synthesize_response_operator, teacher_program_signature,
    teacher_transition_from_completed,
};

use crate::online_subcenter::OnlineSubcenterDiscovery;

const ONLINE_CHECKPOINT_MAGIC_V3: &[u8; 4] = b"NRO3";
const ONLINE_BUCKET_STRATEGY_VERSION: u8 = 66;
const RESTORED_CORE_MIN_BUCKET_EVENTS: usize = 20;
const MAX_PINNED_FUTURE_PARITY_CASES: usize = 4_096;
// Admission needs 32 independent future rows; larger full-frame reservoirs only
// duplicate cold evidence without increasing execution authority.
const MAX_FROZEN_FUTURE_ROWS_PER_BUCKET: usize = 32;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineResponseMinerConfig {
    pub cells: usize,
    pub min_bucket_events: usize,
    pub calibration_events: usize,
    pub max_buckets: usize,
    pub reservoir_rows: usize,
    pub threshold_floor_micro: i64,
}

impl Default for OnlineResponseMinerConfig {
    fn default() -> Self {
        Self {
            cells: 32,
            min_bucket_events: RESTORED_CORE_MIN_BUCKET_EVENTS,
            calibration_events: 16,
            max_buckets: 1_024,
            reservoir_rows: 32,
            threshold_floor_micro: 50_000,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineResponseCandidate {
    pub bucket_id: u32,
    pub structural_family_id: u64,
    pub teacher_signature_sha256: String,
    pub positive_rows: usize,
    pub negative_rows: usize,
    pub positive_tokens: u64,
    pub negative_tokens: u64,
    pub distinct_sessions: usize,
    pub wave_threshold_micro: i64,
    pub wave_runtime_bytes: usize,
    pub wave_runtime_fingerprint64: u64,
    pub program: ResponseProgram,
    pub verifier: VerifierProgram,
    pub phase_rank: u32,
    pub exact_checks: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OnlineResponseAdmissionCandidate {
    pub candidate: OnlineResponseCandidate,
    pub wave_runtime_package: Vec<u8>,
    pub support: Vec<RelationFrame>,
    pub future: Vec<RelationFrame>,
    pub negatives: Vec<RelationFrame>,
    pub required_routing_atom_ids: Vec<u64>,
    #[serde(default)]
    pub runtime_parity_cases: Vec<crate::RuntimeParityCase>,
    #[serde(default)]
    pub semantic_alias_edges: Vec<crate::SemanticAliasEdge>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineResponseAdmissionBlockerReport {
    pub cohort_id_sha256: String,
    pub blocker: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineResponseBucketReport {
    pub bucket_id: u32,
    pub structural_family_id: u64,
    pub teacher_signature_sha256: String,
    pub teacher_action_symbol: String,
    pub positive_rows: usize,
    pub negative_rows: usize,
    pub positive_tokens: u64,
    pub negative_tokens: u64,
    pub false_accepts: usize,
    pub rejected: bool,
    pub learned_threshold_micro: i64,
    pub scored_events: usize,
    pub calibration_events_seen: usize,
    pub max_calibration_false_margin_micro: Option<i64>,
    pub unique_cpu_accepts_over_exact_cache: usize,
    pub candidate_blocker: Option<String>,
    pub synthesized_operation: Option<String>,
    pub synthesis_error: Option<String>,
    pub example_positive_frame_id: Option<String>,
    pub first_false_accept_frame_id: Option<String>,
    pub cardinality_guard_rejects: usize,
    pub positive_cardinality_bounds: BTreeMap<String, (u32, u32)>,
    pub positive_cardinality_signature_count: usize,
    pub frozen_future_rows: usize,
    pub frozen_future_sessions: usize,
    pub distinct_surfaces: usize,
    pub frozen_future_program_mismatches: usize,
    pub support_watermark_event_time_unix_nanos: u64,
    pub late_or_missing_time_rows: usize,
    pub exact_guard_atom_count: usize,
    pub support_rows_with_client_capability: usize,
    pub future_rows_with_client_capability: usize,
    pub support_rows_with_reconstructed_capability: usize,
    pub future_rows_with_reconstructed_capability: usize,
    pub client_capability_profile_count: usize,
    pub admission_precheck: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineResponseActionFamilyReport {
    pub teacher_action_symbol: String,
    pub bucket_count: usize,
    pub synthesizable_bucket_count: usize,
    pub candidate_bucket_count: usize,
    pub positive_rows: usize,
    pub positive_tokens: u64,
    pub frozen_future_rows: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OnlineResponseMinerReport {
    pub schema: String,
    pub rows_seen: usize,
    pub rows_learned: usize,
    pub rows_ambiguous: usize,
    pub rows_without_teacher_action: usize,
    pub competing_negative_updates: usize,
    pub bucket_count: usize,
    pub active_bucket_count: usize,
    pub candidate_bucket_count: usize,
    pub false_accepts: usize,
    pub discovery_false_accepts: usize,
    pub warm_bytes_estimate: usize,
    pub subcenter_rows_seen: u64,
    pub subcenter_pair_count: usize,
    pub subcenter_candidate_count: usize,
    pub subcenter_bytes_estimate: usize,
    pub action_families: Vec<OnlineResponseActionFamilyReport>,
    pub buckets: Vec<OnlineResponseBucketReport>,
    pub candidates: Vec<OnlineResponseCandidate>,
    pub self_training_v2: SelfTrainingStateReport,
    #[serde(default)]
    pub live_scalar_shadow: crate::LiveScalarShadowReport,
    #[serde(default)]
    pub admission_ready_cohorts: usize,
    #[serde(default)]
    pub emitted_candidate_cohorts: usize,
    #[serde(default)]
    pub explicitly_blocked_cohorts: usize,
    #[serde(default)]
    pub admission_candidate_blockers: Vec<OnlineResponseAdmissionBlockerReport>,
    #[serde(default)]
    pub admission_accounting_complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OnlineResponseIngestResult {
    pub rows_seen: usize,
    pub rows_learned: usize,
    pub bucket_count: usize,
    pub candidate_bucket_count: usize,
    pub false_accepts: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OnlineResponseStreamStatus {
    pub checkpoint_restored: bool,
    pub rows_seen: usize,
    pub rows_learned: usize,
    pub bucket_count: usize,
    pub candidate_bucket_count: usize,
    pub false_accepts: usize,
    pub warm_bytes_estimate: usize,
    pub source_lines: u64,
    pub source_offset: u64,
    pub cegis_cohorts: usize,
    pub cegis_winners: usize,
    pub max_frozen_future_rows: usize,
    pub signal_score_out_of_10: u8,
    pub opportunity_ordinary_intents: u64,
    pub opportunity_ordinary_tokens: u64,
    pub opportunity_verified_tokens: u64,
    pub opportunity_verified_share_milli: u64,
    pub opportunity_executable_candidate_tokens: u64,
    pub opportunity_missing_dsl_tokens: u64,
    pub opportunity_missing_verifier_tokens: u64,
    pub opportunity_insufficient_repetition_tokens: u64,
    pub opportunity_unexplored_multi_source_tokens: u64,
    pub opportunity_ambiguous_tokens: u64,
    pub opportunity_non_deterministic_tokens: u64,
    pub opportunity_unresolved_tokens: u64,
    pub opportunity_upper_bound_share_milli: u64,
    pub opportunity_accounting_identity_holds: bool,
    pub opportunity_m3_reachable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ResponseBucket {
    structural_family_id: u64,
    teacher_signature_sha256: String,
    teacher_action_symbol: String,
    positives: VecDeque<RelationFrame>,
    negatives: VecDeque<RelationFrame>,
    #[serde(default)]
    future_positives: VecDeque<RelationFrame>,
    #[serde(default)]
    future_negatives: VecDeque<RelationFrame>,
    positive_rows: usize,
    negative_rows: usize,
    positive_tokens: u64,
    negative_tokens: u64,
    first_false_accept_frame_id: Option<String>,
    cardinality_guard_rejects: usize,
    positive_cardinality_bounds: BTreeMap<String, (u32, u32)>,
    positive_cardinality_signatures: BTreeSet<String>,
    #[serde(default)]
    support_watermark_event_time_unix_nanos: u64,
    #[serde(default)]
    late_or_missing_time_rows: usize,
    #[serde(default)]
    exact_guard_atom_ids: Vec<u64>,
}

#[derive(Clone, Debug)]
pub struct OnlineResponseMiner {
    config: OnlineResponseMinerConfig,
    wave: PhaseCenterOnlineMiner,
    encoder: PhaseCenterAtomEncoder,
    buckets: BTreeMap<u32, ResponseBucket>,
    keys: BTreeMap<(u64, String), u32>,
    rows_seen: usize,
    rows_learned: usize,
    rows_ambiguous: usize,
    rows_without_teacher_action: usize,
    competing_negative_updates: usize,
    seen_frame_sha256: BTreeMap<String, String>,
    future_runtime_parity_cases: BTreeMap<String, crate::RuntimeParityCase>,
    subcenters: OnlineSubcenterDiscovery,
    self_training_v2: StreamingSelfTrainingState,
    live_scalar_shadow: crate::LiveScalarShadowState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrameDisposition {
    New,
    Duplicate,
}

#[derive(Clone, Debug)]
pub struct OnlineResponseTailConfig {
    pub input_path: PathBuf,
    pub report_path: PathBuf,
    pub checkpoint_path: PathBuf,
    pub idle_sleep: Duration,
}

pub struct OnlineResponseStream {
    config: OnlineResponseTailConfig,
    miner: OnlineResponseMiner,
    source_device: u64,
    source_inode: u64,
    source_offset: u64,
    source_lines: u64,
    parse_errors: u64,
    source_prefix_hasher: Sha256,
    checkpoint_restored: bool,
    events_since_checkpoint: u32,
    last_checkpoint: Instant,
}

#[derive(Debug, Deserialize, Serialize)]
struct OnlineResponseCheckpoint {
    schema: String,
    source_device: u64,
    source_inode: u64,
    source_offset: u64,
    #[serde(default)]
    source_prefix_sha256: String,
    source_lines: u64,
    parse_errors: u64,
    config: OnlineResponseMinerConfig,
    #[serde(default)]
    bucket_strategy_version: u8,
    wave_checkpoint: Vec<u8>,
    buckets: BTreeMap<u32, ResponseBucket>,
    rows_seen: usize,
    rows_learned: usize,
    rows_ambiguous: usize,
    rows_without_teacher_action: usize,
    competing_negative_updates: usize,
    #[serde(default)]
    seen_frame_sha256: BTreeMap<String, String>,
    #[serde(default)]
    future_runtime_parity_cases: BTreeMap<String, crate::RuntimeParityCase>,
    #[serde(default)]
    subcenters: OnlineSubcenterDiscovery,
    #[serde(default)]
    self_training_v2: StreamingSelfTrainingState,
    #[serde(default)]
    live_scalar_shadow: crate::LiveScalarShadowState,
}

#[derive(Serialize)]
struct OnlineResponseCheckpointRef<'a> {
    schema: &'static str,
    source_device: u64,
    source_inode: u64,
    source_offset: u64,
    source_prefix_sha256: &'a str,
    source_lines: u64,
    parse_errors: u64,
    config: OnlineResponseMinerConfig,
    bucket_strategy_version: u8,
    wave_checkpoint: &'a [u8],
    buckets: &'a BTreeMap<u32, ResponseBucket>,
    rows_seen: usize,
    rows_learned: usize,
    rows_ambiguous: usize,
    rows_without_teacher_action: usize,
    competing_negative_updates: usize,
    seen_frame_sha256: &'a BTreeMap<String, String>,
    future_runtime_parity_cases: &'a BTreeMap<String, crate::RuntimeParityCase>,
    subcenters: &'a OnlineSubcenterDiscovery,
    self_training_v2: &'a StreamingSelfTrainingState,
    live_scalar_shadow: &'a crate::LiveScalarShadowState,
}

impl OnlineResponseStream {
    #[must_use]
    pub const fn checkpoint_restored(&self) -> bool {
        self.checkpoint_restored
    }

    #[must_use]
    pub const fn source_offset(&self) -> u64 {
        self.source_offset
    }

    #[must_use]
    pub const fn source_lines(&self) -> u64 {
        self.source_lines
    }

    #[must_use]
    pub fn replay_support_parity_cases_total(&self) -> usize {
        self.miner.replay_support_parity_cases_total()
    }

    /// Restores only the bounded miner checkpoint. Live V2 evidence arrives
    /// through framed worker segments, so production never scans the legacy
    /// relation JSON ledger during startup.
    pub fn open_streaming(config: OnlineResponseTailConfig) -> Result<Self, String> {
        if let Some(parent) = config.report_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("online_report_dir:{}:{error}", parent.display()))?;
        }
        if let Some(parent) = config.checkpoint_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("online_checkpoint_dir:{}:{error}", parent.display()))?;
        }
        let restored = decode_online_checkpoint(&config.checkpoint_path)?;
        let checkpoint_restored = restored.is_some();
        let checkpoint_needs_rewrite = restored.as_ref().is_some_and(|checkpoint| {
            checkpoint.bucket_strategy_version < ONLINE_BUCKET_STRATEGY_VERSION
        });
        let miner = match restored {
            Some(checkpoint) => OnlineResponseMiner::from_checkpoint(checkpoint)?,
            None => OnlineResponseMiner::new(OnlineResponseMinerConfig::default())?,
        };
        let mut stream = Self {
            config,
            miner,
            source_device: 0,
            source_inode: 0,
            source_offset: 0,
            source_lines: 0,
            parse_errors: 0,
            source_prefix_hasher: Sha256::new(),
            checkpoint_restored,
            events_since_checkpoint: 0,
            last_checkpoint: Instant::now(),
        };
        if checkpoint_needs_rewrite || !checkpoint_restored {
            stream.persist()?;
        } else {
            write_online_report(&stream.config.report_path, 0, 0, 0, true, &stream.miner)?;
        }
        Ok(stream)
    }

    pub fn open(config: OnlineResponseTailConfig) -> Result<Self, String> {
        if let Some(parent) = config.input_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("online_source_dir:{}:{error}", parent.display()))?;
        }
        if let Some(parent) = config.report_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("online_report_dir:{}:{error}", parent.display()))?;
        }
        let source = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&config.input_path)
            .map_err(|error| {
                format!("online_source_open:{}:{error}", config.input_path.display())
            })?;
        let metadata = source
            .metadata()
            .map_err(|error| format!("online_source_metadata:{error}"))?;
        let (source_device, source_inode) = source_identity(&metadata);
        let restored = load_online_checkpoint(
            &config.checkpoint_path,
            &config.input_path,
            source_device,
            source_inode,
        )?;
        let (
            mut miner,
            source_offset,
            mut source_lines,
            mut parse_errors,
            checkpoint_restored,
            checkpoint_needs_rewrite,
        ) = if let Some(checkpoint) =
            restored.filter(|checkpoint| checkpoint.source_offset <= metadata.len())
        {
            let offset = checkpoint.source_offset;
            let lines = checkpoint.source_lines;
            let errors = checkpoint.parse_errors;
            let needs_rewrite = checkpoint.bucket_strategy_version < ONLINE_BUCKET_STRATEGY_VERSION;
            (
                OnlineResponseMiner::from_checkpoint(checkpoint)?,
                offset,
                lines,
                errors,
                true,
                needs_rewrite,
            )
        } else {
            (
                OnlineResponseMiner::new(OnlineResponseMinerConfig::default())?,
                0,
                0,
                0,
                false,
                false,
            )
        };
        let mut reader = BufReader::new(source);
        reader
            .seek(SeekFrom::Start(source_offset))
            .map_err(|error| format!("online_source_seek:{error}"))?;
        let mut line = String::new();
        loop {
            line.clear();
            let position = reader
                .stream_position()
                .map_err(|error| format!("online_source_position:{error}"))?;
            let bytes = reader
                .read_line(&mut line)
                .map_err(|error| format!("online_source_read:{error}"))?;
            if bytes == 0 {
                let source_prefix_hasher = hash_source_prefix(&config.input_path, position)?;
                let mut stream = Self {
                    config,
                    miner,
                    source_device,
                    source_inode,
                    source_offset: position,
                    source_lines,
                    parse_errors,
                    source_prefix_hasher,
                    checkpoint_restored,
                    events_since_checkpoint: 0,
                    last_checkpoint: Instant::now(),
                };
                if !checkpoint_restored || checkpoint_needs_rewrite || position != source_offset {
                    stream.persist()?;
                } else {
                    write_online_report(
                        &stream.config.report_path,
                        stream.source_lines,
                        stream.parse_errors,
                        stream.source_offset,
                        true,
                        &stream.miner,
                    )?;
                }
                return Ok(stream);
            }
            if !line.ends_with('\n') {
                break;
            }
            source_lines = source_lines.saturating_add(1);
            match serde_json::from_str::<RelationFrame>(line.trim_end()) {
                Ok(frame) if checkpoint_restored => miner.observe_frame(frame)?,
                Ok(frame) => match miner.replay_chronological_frame(frame) {
                    Ok(()) => {}
                    Err(error) if error == "online_frame_id_content_conflict" => {
                        parse_errors = parse_errors.saturating_add(1);
                    }
                    Err(error) => return Err(error),
                },
                Err(_) => parse_errors = parse_errors.saturating_add(1),
            }
        }
        Err("online_source_partial_line_at_startup".to_owned())
    }

    /// Durably appends one canonical frame before applying it to mutable miner state.
    /// A failure after the append is recovered by replaying from the last checkpoint.
    pub fn ingest(
        &mut self,
        mut frame: RelationFrame,
    ) -> Result<OnlineResponseIngestResult, String> {
        canonicalize_online_frame(&mut frame);
        if self.miner.frame_disposition(&frame)? == FrameDisposition::Duplicate {
            return Ok(self.miner.ingest_result());
        }
        let mut bytes = crate::canonical_json_bytes(&frame)
            .map_err(|error| format!("online_audit_encode:{error}"))?;
        bytes.push(b'\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.input_path)
            .map_err(|error| format!("online_audit_open:{error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("online_audit_write:{error}"))?;
        file.sync_data()
            .map_err(|error| format!("online_audit_sync:{error}"))?;
        self.source_prefix_hasher.update(&bytes);
        self.source_offset = file
            .metadata()
            .map_err(|error| format!("online_audit_metadata:{error}"))?
            .len();
        self.source_lines = self.source_lines.saturating_add(1);
        self.miner.observe_frame(frame)?;
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
        if self.events_since_checkpoint >= 64
            || self.last_checkpoint.elapsed() >= Duration::from_secs(5)
        {
            self.persist()?;
        }
        Ok(self.miner.ingest_result())
    }

    /// Applies a transition already made durable by the framed V2 worker.
    /// This path never appends to the legacy JSON relation ledger.
    pub fn apply_teacher_transition(
        &mut self,
        transition: crate::TeacherTransition,
    ) -> Result<OnlineResponseIngestResult, String> {
        self.miner.observe_teacher_transition(transition)?;
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
        Ok(self.miner.ingest_result())
    }

    pub fn run_self_training_work_slice(&mut self) -> usize {
        self.miner.self_training_v2.run_work_slice()
    }

    pub fn run_self_training_work_slice_for_signatures(
        &mut self,
        signatures: &BTreeSet<String>,
    ) -> usize {
        self.miner
            .self_training_v2
            .run_work_slice_for_signatures(signatures)
    }

    #[must_use]
    pub fn has_self_training_work(&self) -> bool {
        self.miner.self_training_v2.has_pending_work()
    }

    #[must_use]
    pub fn has_self_training_work_for_signatures(&self, signatures: &BTreeSet<String>) -> bool {
        self.miner
            .self_training_v2
            .has_pending_work_for_signatures(signatures)
    }

    pub fn persist_now(&mut self) -> Result<(), String> {
        self.persist()
    }

    pub fn observe_ordinary_request(
        &mut self,
        intent_sha256: &str,
        input_tokens: u64,
        now_unix: u64,
    ) {
        self.miner
            .self_training_v2
            .observe_ordinary_request(intent_sha256, input_tokens, now_unix);
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
    }

    pub fn classify_ordinary_intent(
        &mut self,
        intent_sha256: &str,
        class: crate::ReducibilityClass,
        blocker: Option<&str>,
    ) {
        self.miner
            .self_training_v2
            .classify_intent(intent_sha256, class, blocker);
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
    }

    pub fn mark_verified_ordinary_intent(&mut self, intent_sha256: &str) {
        self.miner
            .self_training_v2
            .mark_verified_intent(intent_sha256);
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
    }

    pub fn mark_self_training_false_accept(&mut self, intent_sha256: &str) {
        self.miner.self_training_v2.mark_false_accept(intent_sha256);
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
    }

    pub fn mark_self_training_parity_failure(&mut self, intent_sha256: &str) {
        self.miner
            .self_training_v2
            .mark_parity_failure(intent_sha256);
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
    }

    /// Appends a replay batch with one durability barrier, then observes rows
    /// in source order. If the process stops after the append, normal checkpoint
    /// recovery observes the committed tail with the same semantics.
    pub fn ingest_batch<I>(&mut self, frames: I) -> Result<OnlineResponseIngestResult, String>
    where
        I: IntoIterator<Item = RelationFrame>,
    {
        let mut batch_ids = BTreeMap::<String, String>::new();
        let mut accepted = Vec::new();
        let mut bytes = Vec::new();
        for frame in frames {
            let digest = crate::relation_frame_learning_digest(&frame)
                .map_err(|error| format!("online_frame_digest:{error}"))?;
            match self.miner.frame_disposition(&frame)? {
                FrameDisposition::Duplicate => continue,
                FrameDisposition::New => {}
            }
            match batch_ids.get(&frame.frame_id_sha256) {
                Some(existing) if existing == &digest => continue,
                Some(_) => return Err("online_frame_id_content_conflict".to_owned()),
                None => {
                    batch_ids.insert(frame.frame_id_sha256.clone(), digest);
                }
            }
            let mut encoded = crate::canonical_json_bytes(&frame)
                .map_err(|error| format!("online_audit_encode:{error}"))?;
            encoded.push(b'\n');
            bytes.extend_from_slice(&encoded);
            accepted.push(frame);
        }
        if accepted.is_empty() {
            return Ok(self.miner.ingest_result());
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.input_path)
            .map_err(|error| format!("online_audit_open:{error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("online_audit_write:{error}"))?;
        file.sync_data()
            .map_err(|error| format!("online_audit_sync:{error}"))?;
        self.source_prefix_hasher.update(&bytes);
        self.source_offset = file
            .metadata()
            .map_err(|error| format!("online_audit_metadata:{error}"))?
            .len();
        self.source_lines = self
            .source_lines
            .saturating_add(u64::try_from(accepted.len()).unwrap_or(u64::MAX));
        for frame in accepted {
            self.miner.observe_frame(frame)?;
        }
        self.persist()?;
        Ok(self.miner.ingest_result())
    }

    /// Trains from replayable source history without appending it to the live
    /// audit or claiming frozen-future evidence.
    pub fn train_replay_batch<I>(&mut self, frames: I) -> Result<OnlineResponseIngestResult, String>
    where
        I: IntoIterator<Item = RelationFrame>,
    {
        self.train_replay_cases_batch(frames.into_iter().map(|frame| (frame, None)))
    }

    /// Imports immutable history as support evidence while retaining an
    /// independently reconstructed runtime parity case when one is available.
    pub fn train_replay_cases_batch<I>(
        &mut self,
        cases: I,
    ) -> Result<OnlineResponseIngestResult, String>
    where
        I: IntoIterator<Item = (RelationFrame, Option<crate::RuntimeParityCase>)>,
    {
        let result = self.train_replay_cases_batch_buffered(cases)?;
        self.persist()?;
        Ok(result)
    }

    /// Imports support-only replay cases without an intermediate checkpoint.
    /// Callers must persist after their bounded synthesis work completes.
    pub fn train_replay_cases_batch_buffered<I>(
        &mut self,
        cases: I,
    ) -> Result<OnlineResponseIngestResult, String>
    where
        I: IntoIterator<Item = (RelationFrame, Option<crate::RuntimeParityCase>)>,
    {
        let mut imported_signatures = BTreeSet::new();
        for (frame, runtime_parity_case) in cases {
            let economics = (frame.estimated_input_tokens > 0).then(|| EconomicsReceipt {
                schema: ECONOMICS_RECEIPT_SCHEMA_V1.to_owned(),
                exact_input_tokens: frame.estimated_input_tokens,
                ordinary: false,
                controlled: false,
                replay: true,
                dedupe_eligible: true,
                provider_evidence_ref_sha256: frame.evidence_ref_sha256.clone(),
            });
            let mut transition = teacher_transition_from_completed(&frame, economics)
                .map_err(|error| format!("online_replay_teacher_transition:{error:?}"))?;
            transition.runtime_parity_case = runtime_parity_case;
            match self.miner.import_teacher_transition(transition) {
                Ok(Some(signature)) => {
                    imported_signatures.insert(signature);
                }
                Ok(None) => {}
                Err(error) if error == "online_frame_id_content_conflict" => {}
                Err(error) => return Err(error),
            }
        }
        self.miner
            .self_training_v2
            .prepare_incremental_replay_seed(imported_signatures);
        Ok(self.miner.ingest_result())
    }

    #[must_use]
    pub fn report(&self) -> OnlineResponseMinerReport {
        self.miner.report()
    }

    #[must_use]
    pub fn admission_candidates(&self) -> Vec<OnlineResponseAdmissionCandidate> {
        self.miner.admission_candidates()
    }

    #[must_use]
    pub fn crystallized_admission_candidates(&self) -> Vec<crate::LiveScalarAdmissionCandidate> {
        self.miner.crystallized_admission_candidates()
    }

    #[must_use]
    pub fn has_admission_candidates(&self) -> bool {
        !self.miner.admission_candidates().is_empty()
    }

    #[must_use]
    pub fn status(&self) -> OnlineResponseStreamStatus {
        let ingest = self.miner.ingest_result();
        let v2 = self.miner.self_training_v2.report(unix_now_seconds());
        let max_frozen_future_rows = v2
            .generations
            .iter()
            .map(|generation| generation.future_rows)
            .max()
            .unwrap_or(0);
        let v2_warm_bytes = v2.discovery.warm_bytes_estimate.saturating_add(
            v2.cegis
                .pools
                .iter()
                .map(|pool| pool.ast_nodes.saturating_mul(256))
                .sum::<usize>(),
        );
        let class_tokens = |class: &str| {
            v2.opportunity
                .classes
                .get(class)
                .map_or(0, |report| report.input_tokens)
        };
        OnlineResponseStreamStatus {
            checkpoint_restored: self.checkpoint_restored,
            rows_seen: ingest.rows_seen,
            rows_learned: ingest.rows_learned,
            bucket_count: ingest.bucket_count,
            candidate_bucket_count: ingest.candidate_bucket_count,
            false_accepts: ingest.false_accepts,
            warm_bytes_estimate: v2_warm_bytes,
            source_lines: self.source_lines,
            source_offset: self.source_offset,
            cegis_cohorts: v2.cegis.cohorts,
            cegis_winners: v2.cegis.winners,
            max_frozen_future_rows,
            signal_score_out_of_10: v2.signal_tree.overall_score_out_of_10,
            opportunity_ordinary_intents: v2.opportunity.ordinary_intents,
            opportunity_ordinary_tokens: v2.opportunity.ordinary_tokens,
            opportunity_verified_tokens: v2.opportunity.verified_tokens,
            opportunity_verified_share_milli: v2.opportunity.verified_token_share_milli,
            opportunity_executable_candidate_tokens: class_tokens("EXECUTABLE_CANDIDATE"),
            opportunity_missing_dsl_tokens: class_tokens("MISSING_DSL_PRIMITIVE"),
            opportunity_missing_verifier_tokens: class_tokens("MISSING_EXTERNAL_VERIFIER"),
            opportunity_insufficient_repetition_tokens: class_tokens("INSUFFICIENT_REPETITION"),
            opportunity_unexplored_multi_source_tokens: class_tokens("UNEXPLORED_MULTI_SOURCE"),
            opportunity_ambiguous_tokens: class_tokens("AMBIGUOUS_PRE_ACTION_STATE"),
            opportunity_non_deterministic_tokens: class_tokens("NON_DETERMINISTIC_OR_CREATIVE"),
            opportunity_unresolved_tokens: v2.opportunity.unresolved_tokens,
            opportunity_upper_bound_share_milli: v2
                .opportunity
                .optimistic_executable_upper_bound_share_milli,
            opportunity_accounting_identity_holds: v2.opportunity.classification_identity_holds
                && v2.opportunity.upper_bound_identity_holds,
            opportunity_m3_reachable: v2.opportunity.m3_reachable_under_upper_bound,
        }
    }

    pub fn persist(&mut self) -> Result<(), String> {
        let source_prefix_sha256 = format!("{:x}", self.source_prefix_hasher.clone().finalize());
        write_online_checkpoint(
            &self.config.checkpoint_path,
            self.source_device,
            self.source_inode,
            self.source_offset,
            &source_prefix_sha256,
            self.source_lines,
            self.parse_errors,
            &self.miner,
        )?;
        write_online_report(
            &self.config.report_path,
            self.source_lines,
            self.parse_errors,
            self.source_offset,
            self.checkpoint_restored,
            &self.miner,
        )?;
        self.events_since_checkpoint = 0;
        self.last_checkpoint = Instant::now();
        Ok(())
    }
}

pub fn run_online_response_tail(config: OnlineResponseTailConfig) -> Result<(), String> {
    if let Some(parent) = config.report_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("online_report_dir:{}:{error}", parent.display()))?;
    }
    let source = OpenOptions::new()
        .read(true)
        .open(&config.input_path)
        .map_err(|error| format!("online_source_open:{}:{error}", config.input_path.display()))?;
    let source_metadata = source
        .metadata()
        .map_err(|error| format!("online_source_metadata:{error}"))?;
    let (mut source_device, mut source_inode) = source_identity(&source_metadata);
    let restored = load_online_checkpoint(
        &config.checkpoint_path,
        &config.input_path,
        source_device,
        source_inode,
    )?;
    let (mut miner, source_offset, mut source_lines, mut parse_errors, checkpoint_restored) =
        if let Some(checkpoint) =
            restored.filter(|checkpoint| checkpoint.source_offset <= source_metadata.len())
        {
            let checkpoint_offset = checkpoint.source_offset;
            let checkpoint_lines = checkpoint.source_lines;
            let checkpoint_parse_errors = checkpoint.parse_errors;
            (
                OnlineResponseMiner::from_checkpoint(checkpoint)?,
                checkpoint_offset,
                checkpoint_lines,
                checkpoint_parse_errors,
                true,
            )
        } else {
            (
                OnlineResponseMiner::new(OnlineResponseMinerConfig::default())?,
                0,
                0,
                0,
                false,
            )
        };
    let mut reader = BufReader::new(source);
    reader
        .seek(SeekFrom::Start(source_offset))
        .map_err(|error| format!("online_source_seek:{error}"))?;
    let mut source_prefix_hasher = hash_source_prefix(&config.input_path, source_offset)?;
    let mut line = String::new();
    let mut following = checkpoint_restored;
    let mut last_report = Instant::now();
    loop {
        line.clear();
        let position = reader
            .stream_position()
            .map_err(|error| format!("online_source_position:{error}"))?;
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("online_source_read:{error}"))?;
        if bytes == 0 {
            if !following || last_report.elapsed() >= Duration::from_secs(5) {
                following = true;
                write_online_checkpoint(
                    &config.checkpoint_path,
                    source_device,
                    source_inode,
                    position,
                    &format!("{:x}", source_prefix_hasher.clone().finalize()),
                    source_lines,
                    parse_errors,
                    &miner,
                )?;
                write_online_report(
                    &config.report_path,
                    source_lines,
                    parse_errors,
                    position,
                    checkpoint_restored,
                    &miner,
                )?;
                last_report = Instant::now();
            }
            let length = fs::metadata(&config.input_path)
                .map(|metadata| metadata.len())
                .unwrap_or(position);
            if length < position {
                let source = OpenOptions::new()
                    .read(true)
                    .open(&config.input_path)
                    .map_err(|error| format!("online_source_reopen:{error}"))?;
                let metadata = source
                    .metadata()
                    .map_err(|error| format!("online_source_reopen_metadata:{error}"))?;
                (source_device, source_inode) = source_identity(&metadata);
                reader = BufReader::new(source);
                source_prefix_hasher = Sha256::new();
                following = true;
            } else {
                thread::sleep(config.idle_sleep);
            }
            continue;
        }
        if !line.ends_with('\n') {
            reader
                .seek(SeekFrom::Start(position))
                .map_err(|error| format!("online_source_partial_rewind:{error}"))?;
            thread::sleep(config.idle_sleep);
            continue;
        }
        source_prefix_hasher.update(line.as_bytes());
        source_lines = source_lines.saturating_add(1);
        match serde_json::from_str::<RelationFrame>(line.trim_end()) {
            Ok(frame) if following => miner.observe_frame(frame)?,
            Ok(frame) => miner.replay_chronological_frame(frame)?,
            Err(_) => parse_errors = parse_errors.saturating_add(1),
        }
    }
}

fn write_online_report(
    path: &Path,
    source_lines: u64,
    parse_errors: u64,
    source_offset: u64,
    checkpoint_restored: bool,
    miner: &OnlineResponseMiner,
) -> Result<(), String> {
    let value = serde_json::json!({
        "schema": "nando.embedded-response-online-miner.v1",
        "generated_at_unix_ms": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        "source_lines": source_lines,
        "parse_errors": parse_errors,
        "source_offset": source_offset,
        "checkpoint_restored": checkpoint_restored,
        "tail_follow_active": true,
        "execution_authority": false,
        "miner": miner.report(),
    });
    let mut bytes = serde_json::to_vec_pretty(&value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut file = File::create(&temporary)
        .map_err(|error| format!("online_report_create:{}:{error}", temporary.display()))?;
    file.write_all(&bytes)
        .map_err(|error| format!("online_report_write:{error}"))?;
    file.sync_all()
        .map_err(|error| format!("online_report_sync:{error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("online_report_rename:{error}"))?;
    sync_parent_directory(path, "online_report_dir_sync")
}

fn load_online_checkpoint(
    path: &Path,
    source_path: &Path,
    source_device: u64,
    source_inode: u64,
) -> Result<Option<OnlineResponseCheckpoint>, String> {
    let Some(checkpoint) = decode_online_checkpoint(path)? else {
        return Ok(None);
    };
    if checkpoint.source_device != source_device || checkpoint.source_inode != source_inode {
        return Ok(None);
    }
    let actual = format!(
        "{:x}",
        hash_source_prefix(source_path, checkpoint.source_offset)?.finalize()
    );
    if checkpoint.source_prefix_sha256.len() != 64 || checkpoint.source_prefix_sha256 != actual {
        return Err("online_checkpoint_source_prefix_mismatch".to_owned());
    }
    Ok(Some(checkpoint))
}

fn decode_online_checkpoint(path: &Path) -> Result<Option<OnlineResponseCheckpoint>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("online_checkpoint_read:{}:{error}", path.display())),
    };
    let checkpoint: OnlineResponseCheckpoint =
        if let Some(payload) = bytes.strip_prefix(ONLINE_CHECKPOINT_MAGIC_V3) {
            serde_cbor::from_slice(payload)
                .map_err(|error| format!("online_checkpoint_decode:{error}"))?
        } else {
            serde_json::from_slice(&bytes)
                .map_err(|error| format!("online_checkpoint_legacy_decode:{error}"))?
        };
    if !matches!(
        checkpoint.schema.as_str(),
        "nando.online-response-checkpoint.v1"
            | "nando.online-response-checkpoint.v2"
            | "nando.online-response-checkpoint.v3"
    ) {
        return Ok(None);
    }
    Ok(Some(checkpoint))
}

#[allow(clippy::too_many_arguments)]
fn write_online_checkpoint(
    path: &Path,
    source_device: u64,
    source_inode: u64,
    source_offset: u64,
    source_prefix_sha256: &str,
    source_lines: u64,
    parse_errors: u64,
    miner: &OnlineResponseMiner,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("online_checkpoint_dir:{}:{error}", parent.display()))?;
    }
    let bytes = miner.checkpoint_bytes(
        source_device,
        source_inode,
        source_offset,
        source_prefix_sha256,
        source_lines,
        parse_errors,
    )?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|error| format!("online_checkpoint_create:{}:{error}", temporary.display()))?;
    file.write_all(&bytes)
        .map_err(|error| format!("online_checkpoint_write:{error}"))?;
    file.sync_all()
        .map_err(|error| format!("online_checkpoint_sync:{error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("online_checkpoint_rename:{error}"))?;
    sync_parent_directory(path, "online_checkpoint_dir_sync")
}

fn hash_source_prefix(path: &Path, prefix_len: u64) -> Result<Sha256, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("online_source_hash_open:{}:{error}", path.display()))?;
    if file
        .metadata()
        .map_err(|error| format!("online_source_hash_metadata:{error}"))?
        .len()
        < prefix_len
    {
        return Err("online_source_hash_prefix_beyond_end".to_owned());
    }
    let mut hasher = Sha256::new();
    let mut remaining = prefix_len;
    let mut buffer = [0_u8; 64 * 1024];
    while remaining > 0 {
        let limit = usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = file
            .read(&mut buffer[..limit])
            .map_err(|error| format!("online_source_hash_read:{error}"))?;
        if read == 0 {
            return Err("online_source_hash_unexpected_eof".to_owned());
        }
        hasher.update(&buffer[..read]);
        remaining = remaining.saturating_sub(read as u64);
    }
    Ok(hasher)
}

fn sync_parent_directory(path: &Path, error_prefix: &str) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("{error_prefix}:{}:{error}", parent.display()))
}

#[cfg(unix)]
fn source_identity(metadata: &fs::Metadata) -> (u64, u64) {
    (metadata.dev(), metadata.ino())
}

#[cfg(not(unix))]
fn source_identity(_metadata: &fs::Metadata) -> (u64, u64) {
    (0, 0)
}

impl OnlineResponseMiner {
    #[must_use]
    pub fn replay_support_parity_cases_total(&self) -> usize {
        self.self_training_v2.replay_support_parity_cases_total()
    }

    pub fn new(config: OnlineResponseMinerConfig) -> Result<Self, String> {
        if config.reservoir_rows == 0 {
            return Err("online_response_reservoir_rows_zero".to_owned());
        }
        let wave_config = PhaseCenterOnlineMinerConfig {
            cells: config.cells,
            min_bucket_events: config.min_bucket_events,
            threshold_floor_micro: config.threshold_floor_micro,
            calibration_events: config.calibration_events,
            max_buckets: config.max_buckets,
        };
        Ok(Self {
            config,
            wave: PhaseCenterOnlineMiner::new(wave_config)
                .map_err(|error| format!("online_response_wave_init:{error:?}"))?,
            encoder: PhaseCenterAtomEncoder::new(config.cells)
                .map_err(|error| format!("online_response_encoder_init:{error:?}"))?,
            buckets: BTreeMap::new(),
            keys: BTreeMap::new(),
            rows_seen: 0,
            rows_learned: 0,
            rows_ambiguous: 0,
            rows_without_teacher_action: 0,
            competing_negative_updates: 0,
            seen_frame_sha256: BTreeMap::new(),
            future_runtime_parity_cases: BTreeMap::new(),
            subcenters: OnlineSubcenterDiscovery::default(),
            self_training_v2: StreamingSelfTrainingState::new(unix_now_seconds()),
            live_scalar_shadow: crate::LiveScalarShadowState::default(),
        })
    }

    #[cfg(test)]
    fn checkpoint(
        &self,
        source_device: u64,
        source_inode: u64,
        source_offset: u64,
        source_lines: u64,
        parse_errors: u64,
    ) -> Result<OnlineResponseCheckpoint, String> {
        Ok(OnlineResponseCheckpoint {
            schema: "nando.online-response-checkpoint.v1".to_owned(),
            source_device,
            source_inode,
            source_offset,
            source_prefix_sha256: String::new(),
            source_lines,
            parse_errors,
            config: self.config,
            bucket_strategy_version: ONLINE_BUCKET_STRATEGY_VERSION,
            wave_checkpoint: self
                .wave
                .to_checkpoint_bytes()
                .map_err(|error| format!("online_wave_checkpoint:{error:?}"))?,
            buckets: self.buckets.clone(),
            rows_seen: self.rows_seen,
            rows_learned: self.rows_learned,
            rows_ambiguous: self.rows_ambiguous,
            rows_without_teacher_action: self.rows_without_teacher_action,
            competing_negative_updates: self.competing_negative_updates,
            seen_frame_sha256: self.seen_frame_sha256.clone(),
            future_runtime_parity_cases: self.future_runtime_parity_cases.clone(),
            subcenters: self.subcenters.clone(),
            self_training_v2: self.self_training_v2.clone(),
            live_scalar_shadow: self.live_scalar_shadow.clone(),
        })
    }

    fn checkpoint_bytes(
        &self,
        source_device: u64,
        source_inode: u64,
        source_offset: u64,
        source_prefix_sha256: &str,
        source_lines: u64,
        parse_errors: u64,
    ) -> Result<Vec<u8>, String> {
        let wave_checkpoint = self
            .wave
            .to_checkpoint_bytes()
            .map_err(|error| format!("online_wave_checkpoint:{error:?}"))?;
        let mut bytes = ONLINE_CHECKPOINT_MAGIC_V3.to_vec();
        bytes.extend_from_slice(
            &serde_cbor::to_vec(&OnlineResponseCheckpointRef {
                schema: "nando.online-response-checkpoint.v3",
                source_device,
                source_inode,
                source_offset,
                source_prefix_sha256,
                source_lines,
                parse_errors,
                config: self.config,
                bucket_strategy_version: ONLINE_BUCKET_STRATEGY_VERSION,
                wave_checkpoint: &wave_checkpoint,
                buckets: &self.buckets,
                rows_seen: self.rows_seen,
                rows_learned: self.rows_learned,
                rows_ambiguous: self.rows_ambiguous,
                rows_without_teacher_action: self.rows_without_teacher_action,
                competing_negative_updates: self.competing_negative_updates,
                seen_frame_sha256: &self.seen_frame_sha256,
                future_runtime_parity_cases: &self.future_runtime_parity_cases,
                subcenters: &self.subcenters,
                self_training_v2: &self.self_training_v2,
                live_scalar_shadow: &self.live_scalar_shadow,
            })
            .map_err(|error| format!("online_checkpoint_encode:{error}"))?,
        );
        Ok(bytes)
    }

    fn from_checkpoint(mut checkpoint: OnlineResponseCheckpoint) -> Result<Self, String> {
        checkpoint.config.reservoir_rows = checkpoint
            .config
            .reservoir_rows
            .clamp(1, OnlineResponseMinerConfig::default().reservoir_rows);
        if checkpoint.bucket_strategy_version < ONLINE_BUCKET_STRATEGY_VERSION {
            checkpoint.config.min_bucket_events = checkpoint
                .config
                .min_bucket_events
                .max(RESTORED_CORE_MIN_BUCKET_EVENTS);
            let mut migrated = Self::new(checkpoint.config)?;
            if checkpoint.self_training_v2.teacher_pool_count() > 0 {
                let mut preserved_self_training = checkpoint.self_training_v2;
                if checkpoint.bucket_strategy_version < 33 {
                    preserved_self_training.prepare_strategy_migration();
                } else if checkpoint.bucket_strategy_version < 44 {
                    preserved_self_training.prepare_teacher_signature_migration()?;
                } else if checkpoint.bucket_strategy_version < 64 {
                    preserved_self_training.prepare_effect_law_migration();
                } else if checkpoint.bucket_strategy_version < 65 {
                    preserved_self_training.prepare_phase_route_migration();
                } else if checkpoint.bucket_strategy_version < 66 {
                    preserved_self_training.prepare_teacher_signature_migration()?;
                }
                let support_frames =
                    preserved_self_training.bounded_teacher_frames_for_wave_migration();
                for frame in support_frames {
                    migrated.process_frame(frame, false, None, false)?;
                }
                // The replay above rebuilds only the restored Wave buckets.
                // Keep the already learned V2 state byte-for-byte and never
                // reinterpret historical rows as frozen future.
                migrated.self_training_v2 = preserved_self_training;
                migrated.rows_seen = checkpoint.rows_seen;
                migrated.rows_learned = checkpoint.rows_learned;
                migrated.rows_ambiguous = checkpoint.rows_ambiguous;
                migrated.rows_without_teacher_action = checkpoint.rows_without_teacher_action;
                migrated.seen_frame_sha256 = checkpoint.seen_frame_sha256;
                return Ok(migrated);
            }
            let mut replay = checkpoint
                .buckets
                .values()
                .flat_map(|bucket| {
                    bucket
                        .positives
                        .iter()
                        .chain(bucket.future_positives.iter())
                        .chain(bucket.negatives.iter())
                        .chain(bucket.future_negatives.iter())
                        .cloned()
                })
                .collect::<Vec<_>>();
            replay.sort_by(|left, right| {
                left.observed_at_unix_nanos
                    .cmp(&right.observed_at_unix_nanos)
                    .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
            });
            replay.dedup_by(|left, right| left.frame_id_sha256 == right.frame_id_sha256);
            for frame in replay {
                let economics = (frame.estimated_input_tokens > 0).then(|| EconomicsReceipt {
                    schema: ECONOMICS_RECEIPT_SCHEMA_V1.to_owned(),
                    exact_input_tokens: frame.estimated_input_tokens,
                    ordinary: false,
                    controlled: false,
                    replay: true,
                    dedupe_eligible: true,
                    provider_evidence_ref_sha256: frame.evidence_ref_sha256.clone(),
                });
                if let Ok(transition) = teacher_transition_from_completed(&frame, economics) {
                    migrated.import_teacher_transition(transition)?;
                }
            }
            if checkpoint.bucket_strategy_version < 33 {
                migrated.self_training_v2.prepare_strategy_migration();
            } else if checkpoint.bucket_strategy_version >= 44 {
                migrated.self_training_v2.prepare_replay_seed();
            } else {
                migrated
                    .self_training_v2
                    .prepare_teacher_signature_migration()?;
            }
            return Ok(migrated);
        }
        checkpoint.self_training_v2.repair_missing_synthesis_state();
        let wave = PhaseCenterOnlineMiner::from_checkpoint_bytes(&checkpoint.wave_checkpoint)
            .map_err(|error| format!("online_wave_checkpoint_restore:{error:?}"))?;
        let expected_wave_config = PhaseCenterOnlineMinerConfig {
            cells: checkpoint.config.cells,
            min_bucket_events: checkpoint.config.min_bucket_events,
            threshold_floor_micro: checkpoint.config.threshold_floor_micro,
            calibration_events: checkpoint.config.calibration_events,
            max_buckets: checkpoint.config.max_buckets,
        };
        if wave.config() != expected_wave_config {
            return Err("online_checkpoint_config_mismatch".to_owned());
        }
        for bucket in checkpoint.buckets.values_mut() {
            trim_session_diverse_future(
                &mut bucket.future_positives,
                MAX_FROZEN_FUTURE_ROWS_PER_BUCKET,
            );
            recompute_exact_guard(bucket);
        }
        let mut keys = BTreeMap::new();
        for (bucket_id, bucket) in &checkpoint.buckets {
            let key = (
                bucket.structural_family_id,
                bucket.teacher_signature_sha256.clone(),
            );
            if keys.insert(key, *bucket_id).is_some() {
                return Err("online_checkpoint_duplicate_bucket_key".to_owned());
            }
        }
        Ok(Self {
            config: checkpoint.config,
            wave,
            encoder: PhaseCenterAtomEncoder::new(checkpoint.config.cells)
                .map_err(|error| format!("online_response_encoder_restore:{error:?}"))?,
            buckets: checkpoint.buckets,
            keys,
            rows_seen: checkpoint.rows_seen,
            rows_learned: checkpoint.rows_learned,
            rows_ambiguous: checkpoint.rows_ambiguous,
            rows_without_teacher_action: checkpoint.rows_without_teacher_action,
            competing_negative_updates: checkpoint.competing_negative_updates,
            seen_frame_sha256: checkpoint.seen_frame_sha256,
            future_runtime_parity_cases: checkpoint.future_runtime_parity_cases,
            subcenters: checkpoint.subcenters,
            self_training_v2: checkpoint.self_training_v2,
            live_scalar_shadow: checkpoint.live_scalar_shadow,
        })
    }

    pub fn train_frame(&mut self, frame: RelationFrame) -> Result<(), String> {
        self.process_frame(frame, false, None, true)
    }

    pub fn observe_frame(&mut self, frame: RelationFrame) -> Result<(), String> {
        self.process_frame(frame, true, None, true)
    }

    pub fn observe_teacher_transition(
        &mut self,
        transition: crate::TeacherTransition,
    ) -> Result<(), String> {
        let mut frame = transition.as_training_relation_frame();
        let economics = transition.economics;
        let runtime_parity_case = transition.runtime_parity_case;
        frame = action_schema_enriched_frame(&frame, runtime_parity_case.as_ref());
        canonicalize_online_frame(&mut frame);
        let mut transition = teacher_transition_from_completed(&frame, economics)
            .map_err(|error| format!("online_teacher_transition:{error:?}"))?;
        transition.runtime_parity_case = runtime_parity_case;
        if self.frame_disposition(&frame)? == FrameDisposition::Duplicate {
            self.self_training_v2
                .observe_runtime_parity_case(&transition);
            return Ok(());
        }
        self.live_scalar_shadow.observe(&transition);
        self.process_frame(frame, true, Some(transition), true)
    }

    fn import_teacher_transition(
        &mut self,
        transition: crate::TeacherTransition,
    ) -> Result<Option<String>, String> {
        let mut frame = transition.as_training_relation_frame();
        let economics = transition.economics;
        let runtime_parity_case = transition.runtime_parity_case;
        frame = action_schema_enriched_frame(&frame, runtime_parity_case.as_ref());
        canonicalize_online_frame(&mut frame);
        let mut canonical_transition = teacher_transition_from_completed(&frame, economics)
            .map_err(|error| format!("online_migration_teacher_transition:{error:?}"))?;
        canonical_transition.runtime_parity_case = runtime_parity_case;
        let teacher_signature = crate::teacher_program_signature(&frame);
        if self.frame_disposition(&frame)? == FrameDisposition::Duplicate {
            self.self_training_v2
                .observe_migration_transition(&canonical_transition)?;
            return Ok(teacher_signature);
        }
        let frame_digest = crate::relation_frame_learning_digest(&frame)
            .map_err(|error| format!("online_frame_digest:{error}"))?;
        self.seen_frame_sha256
            .insert(frame.frame_id_sha256.clone(), frame_digest);
        self.rows_seen = self.rows_seen.saturating_add(1);
        if transition.outcome.verifier.accepted {
            self.rows_learned = self.rows_learned.saturating_add(1);
        }
        self.self_training_v2
            .observe_migration_transition(&canonical_transition)?;
        Ok(teacher_signature)
    }

    /// Replays immutable source history in event order. The first bounded
    /// positives of each learned family form support; only later rows are
    /// scored as frozen future and can calibrate a candidate.
    pub fn replay_chronological_frame(&mut self, frame: RelationFrame) -> Result<(), String> {
        let verified_teacher_event = frame.verifier_label == Some(true);
        self.process_frame(frame, verified_teacher_event, None, true)
    }

    fn process_frame(
        &mut self,
        mut frame: RelationFrame,
        score_before_update: bool,
        explicit_transition: Option<crate::TeacherTransition>,
        update_self_training: bool,
    ) -> Result<(), String> {
        canonicalize_online_frame(&mut frame);
        if self.frame_disposition(&frame)? == FrameDisposition::Duplicate {
            return Ok(());
        }
        let frame_digest = crate::relation_frame_learning_digest(&frame)
            .map_err(|error| format!("online_frame_digest:{error}"))?;
        self.seen_frame_sha256
            .insert(frame.frame_id_sha256.clone(), frame_digest);
        self.rows_seen = self.rows_seen.saturating_add(1);
        if !is_source_neutral_relation_frame(&frame) {
            self.rows_ambiguous = self.rows_ambiguous.saturating_add(1);
            return Ok(());
        }
        let streaming_v2_transition = explicit_transition.is_some();
        let future_runtime_parity_case = explicit_transition
            .as_ref()
            .and_then(|transition| transition.runtime_parity_case.clone());
        let (explicit_economics, explicit_runtime_parity_case) = explicit_transition
            .map_or((None, None), |transition| {
                (transition.economics, transition.runtime_parity_case)
            });
        let transition = if streaming_v2_transition {
            teacher_transition_from_completed(&frame, explicit_economics)
                .ok()
                .map(|mut transition| {
                    transition.runtime_parity_case = explicit_runtime_parity_case;
                    transition
                })
        } else {
            let economics = (frame.estimated_input_tokens > 0).then(|| EconomicsReceipt {
                schema: ECONOMICS_RECEIPT_SCHEMA_V1.to_owned(),
                exact_input_tokens: frame.estimated_input_tokens,
                ordinary: false,
                controlled: false,
                replay: true,
                dedupe_eligible: true,
                provider_evidence_ref_sha256: frame.evidence_ref_sha256.clone(),
            });
            teacher_transition_from_completed(&frame, economics).ok()
        };
        if update_self_training && let Some(transition) = transition {
            self.self_training_v2.observe_transition(&transition)?;
        }
        let teacher_signature = teacher_program_signature(&frame);
        let hypotheses = ground_roles(&frame);
        let plan_advance = frame
            .atoms
            .iter()
            .any(|atom| matches!(atom, RelationAtom::ActionPlanAdvance));
        let ambiguous_grounding = teacher_signature.is_some()
            && !plan_advance
            && (hypotheses.len() != 1 || hypotheses[0].competing_binding_count != 0);
        if ambiguous_grounding {
            self.rows_ambiguous = self.rows_ambiguous.saturating_add(1);
        }
        let atom_ids = relation_frame_online_routing_atom_ids(&frame);
        if atom_ids.is_empty() {
            self.rows_ambiguous = self.rows_ambiguous.saturating_add(1);
            return Ok(());
        }
        if teacher_signature.is_none() {
            self.rows_without_teacher_action = self.rows_without_teacher_action.saturating_add(1);
            if frame.verifier_label != Some(false) {
                return Ok(());
            }
        }

        let target_is_positive = frame.verifier_label == Some(true);
        if target_is_positive && teacher_signature.is_some() {
            self.subcenters.observe(
                &teacher_action_symbol(&frame),
                &atom_ids,
                frame.estimated_input_tokens,
            );
        }
        let mut target_family_ids = teacher_signature
            .as_ref()
            .map(|signature| restored_miner_family_ids(self, &frame, signature, &atom_ids))
            .unwrap_or_default();
        target_family_ids.sort_unstable();
        target_family_ids.dedup();
        let mut target_bucket_ids = BTreeSet::new();
        if let Some(signature) = teacher_signature.as_deref() {
            for family_id in target_family_ids {
                target_bucket_ids.insert(self.bucket_for(family_id, signature, &frame)?);
            }
        }
        if target_is_positive && !target_bucket_ids.is_empty() {
            self.rows_learned = self.rows_learned.saturating_add(1);
        }
        let family_bucket_ids = self.buckets.keys().copied().collect::<Vec<_>>();

        let mut retained_as_frozen_future = false;
        for bucket_id in family_bucket_ids {
            let is_target = target_bucket_ids.contains(&bucket_id);
            let same_teacher_program = teacher_signature.as_deref().is_some_and(|signature| {
                self.buckets
                    .get(&bucket_id)
                    .is_some_and(|bucket| bucket.teacher_signature_sha256 == signature)
            });
            if target_is_positive && !is_target && same_teacher_program {
                continue;
            }
            let safe_for_bucket = is_target && target_is_positive;
            let guard_matches = self
                .buckets
                .get(&bucket_id)
                .is_some_and(|bucket| cardinality_guard_matches(bucket, &frame));
            let (support_frozen, event_is_after_watermark) = self
                .buckets
                .get(&bucket_id)
                .map(|bucket| {
                    (
                        bucket.positives.len() >= self.config.reservoir_rows,
                        frame.observed_at_unix_nanos > 0
                            && frame.observed_at_unix_nanos
                                > bucket.support_watermark_event_time_unix_nanos,
                    )
                })
                .unwrap_or((false, false));
            let score_for_bucket =
                score_before_update && support_frozen && event_is_after_watermark;
            if score_before_update && support_frozen && !event_is_after_watermark {
                let bucket = self.buckets.get_mut(&bucket_id).expect("bucket exists");
                bucket.late_or_missing_time_rows =
                    bucket.late_or_missing_time_rows.saturating_add(1);
            }
            let needs_wave_calibration = self.wave.bucket(bucket_id).is_some_and(|bucket| {
                bucket.calibration_events_seen < self.config.calibration_events
                    || bucket.max_calibration_false_margin_micro.is_none()
            });
            if score_for_bucket
                && needs_wave_calibration
                && self
                    .wave
                    .bucket(bucket_id)
                    .is_some_and(|bucket| bucket.max_calibration_false_margin_micro.is_none())
            {
                let calibration_negatives = self
                    .buckets
                    .get(&bucket_id)
                    .map(|bucket| {
                        bucket
                            .negatives
                            .iter()
                            .take(self.config.calibration_events)
                            .cloned()
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                for negative in calibration_negatives {
                    self.wave
                        .observe_atom_ids(
                            &mut self.encoder,
                            bucket_id,
                            relation_frame_online_routing_atom_ids(&negative),
                            false,
                            false,
                            0,
                            0,
                        )
                        .map_err(|error| {
                            format!("online_response_support_negative_calibration:{error:?}")
                        })?;
                }
            }
            let calibrate_behind_guard =
                score_for_bucket && !guard_matches && needs_wave_calibration;
            let decision = if score_for_bucket && (guard_matches || calibrate_behind_guard) {
                Some(
                    self.wave
                        .observe_atom_ids(
                            &mut self.encoder,
                            bucket_id,
                            atom_ids.iter().copied(),
                            safe_for_bucket,
                            false,
                            frame.estimated_input_tokens,
                            0,
                        )
                        .map_err(|error| format!("online_response_observe:{error:?}"))?,
                )
            } else {
                self.wave
                    .train_atom_ids(
                        &mut self.encoder,
                        bucket_id,
                        atom_ids.iter().copied(),
                        safe_for_bucket,
                    )
                    .map_err(|error| format!("online_response_train:{error:?}"))?;
                None
            };
            if score_for_bucket && !guard_matches {
                let bucket = self.buckets.get_mut(&bucket_id).expect("bucket exists");
                bucket.cardinality_guard_rejects =
                    bucket.cardinality_guard_rejects.saturating_add(1);
            }
            if guard_matches && decision.is_some_and(|decision| decision.false_accept) {
                let bucket = self.buckets.get_mut(&bucket_id).expect("bucket exists");
                bucket
                    .first_false_accept_frame_id
                    .get_or_insert_with(|| frame.frame_id_sha256.clone());
            }
            if safe_for_bucket {
                let bucket = self.buckets.get_mut(&bucket_id).expect("bucket exists");
                bucket.positive_rows = bucket.positive_rows.saturating_add(1);
                bucket.positive_tokens = bucket
                    .positive_tokens
                    .saturating_add(frame.estimated_input_tokens);
                update_exact_guard(bucket, &frame);
                if bucket.positives.len() < self.config.reservoir_rows {
                    bucket.positives.push_back(frame.clone());
                    bucket.support_watermark_event_time_unix_nanos = bucket
                        .support_watermark_event_time_unix_nanos
                        .max(frame.observed_at_unix_nanos);
                } else if score_for_bucket {
                    push_session_diverse_future(
                        &mut bucket.future_positives,
                        frame.clone(),
                        MAX_FROZEN_FUTURE_ROWS_PER_BUCKET,
                    );
                    retained_as_frozen_future = true;
                }
                update_cardinality_bounds(bucket, &frame);
            } else {
                let mut negative = frame.clone();
                negative.verifier_label = Some(false);
                let bucket = self.buckets.get_mut(&bucket_id).expect("bucket exists");
                bucket.negative_rows = bucket.negative_rows.saturating_add(1);
                bucket.negative_tokens = bucket
                    .negative_tokens
                    .saturating_add(frame.estimated_input_tokens);
                if score_for_bucket {
                    push_bounded(&mut bucket.future_negatives, negative, 8);
                } else {
                    push_bounded(
                        &mut bucket.negatives,
                        negative,
                        self.config.reservoir_rows.min(8),
                    );
                }
                if !is_target {
                    self.competing_negative_updates =
                        self.competing_negative_updates.saturating_add(1);
                }
            }
        }
        if retained_as_frozen_future && let Some(mut parity_case) = future_runtime_parity_case {
            parity_case.evidence_ref_sha256 = frame.frame_id_sha256.clone();
            self.future_runtime_parity_cases
                .insert(canonical_runtime_parity_key(&frame), parity_case);
            if self.rows_seen % 64 == 0 {
                self.prune_future_runtime_parity_cases();
            }
        }
        Ok(())
    }

    fn prune_future_runtime_parity_cases(&mut self) {
        let retained = self
            .buckets
            .values()
            .flat_map(|bucket| bucket.future_positives.iter())
            .map(canonical_runtime_parity_key)
            .collect::<BTreeSet<_>>();
        self.future_runtime_parity_cases
            .retain(|key, _| retained.contains(key));
        while self.future_runtime_parity_cases.len() > MAX_PINNED_FUTURE_PARITY_CASES {
            let Some(oldest) = self.future_runtime_parity_cases.keys().next().cloned() else {
                break;
            };
            self.future_runtime_parity_cases.remove(&oldest);
        }
    }

    fn frame_disposition(&self, frame: &RelationFrame) -> Result<FrameDisposition, String> {
        let digest = crate::relation_frame_learning_digest(frame)
            .map_err(|error| format!("online_frame_digest:{error}"))?;
        match self.seen_frame_sha256.get(&frame.frame_id_sha256) {
            Some(existing) if existing == &digest => Ok(FrameDisposition::Duplicate),
            Some(_) => Err("online_frame_id_content_conflict".to_owned()),
            None => Ok(FrameDisposition::New),
        }
    }

    pub fn report(&self) -> OnlineResponseMinerReport {
        let summary = self.wave.summary();
        let mut candidates = Vec::new();
        let mut bucket_reports = Vec::new();
        for (bucket_id, bucket) in &self.buckets {
            // Wave/anti-center owns applicability negatives. The typed actor is
            // synthesized from teacher-positive executions and independently
            // verified; a negative with the same action shape is not evidence
            // that the actor bytecode itself is wrong.
            let support = bucket.positives.iter().cloned().collect::<Vec<_>>();
            let synthesis = synthesize_response_operator(&support);
            let wave_bucket = self.wave.bucket(*bucket_id);
            let frozen_future_sessions = bucket
                .future_positives
                .iter()
                .map(|frame| frame.session_id_sha256.as_str())
                .collect::<BTreeSet<_>>()
                .len();
            let distinct_surfaces = bucket
                .positives
                .iter()
                .chain(bucket.future_positives.iter())
                .filter_map(crate::relation_frame_structural_family_id)
                .collect::<BTreeSet<_>>()
                .len();
            let frozen_future_program_mismatches = synthesis.as_ref().map_or(0, |operator| {
                bucket
                    .future_positives
                    .iter()
                    .filter(|frame| {
                        frame.verifier_label != Some(true)
                            || !crate::synthesis::program_is_consistent(
                                &operator.candidate.program,
                                frame,
                            )
                    })
                    .count()
            });
            let learned_wave_route = self
                .wave
                .candidate_package_bytes(*bucket_id)
                .ok()
                .flatten()
                .and_then(|wave_package| {
                    crate::online_admission::learned_wave_route_from_bytes(
                        &wave_package.package_bytes,
                        wave_package.threshold_micro,
                    )
                });
            let admission_precheck = if bucket.positives.len() < 32 {
                "support_rows_below_32".to_owned()
            } else if bucket.future_positives.len() < 32 {
                "future_rows_below_32".to_owned()
            } else if frozen_future_sessions < 3 {
                "future_sessions_below_3".to_owned()
            } else if distinct_surfaces < 2 {
                "surfaces_below_2".to_owned()
            } else if frozen_future_program_mismatches > 0 {
                "future_program_mismatch".to_owned()
            } else {
                online_admission_precheck(bucket, learned_wave_route.as_ref())
            };
            let candidate_blocker = wave_bucket.and_then(|wave_bucket| {
                if wave_bucket.rejected || wave_bucket.false_accepts > 0 {
                    Some("rejected_or_false_accept".to_owned())
                } else if wave_bucket.calibration_events_seen < self.config.calibration_events {
                    Some("calibration_events_pending".to_owned())
                } else if wave_bucket.max_calibration_false_margin_micro.is_none() {
                    Some("negative_margin_missing".to_owned())
                } else if wave_bucket.unique_cpu_accepts_over_exact_cache == 0 {
                    Some("verified_shadow_accept_missing".to_owned())
                } else {
                    None
                }
            });
            let support_rows_with_client_capability = bucket
                .positives
                .iter()
                .filter(|frame| {
                    frame
                        .atoms
                        .iter()
                        .any(|atom| matches!(atom, RelationAtom::ClientCapabilityAtom { .. }))
                })
                .count();
            let future_rows_with_client_capability = bucket
                .future_positives
                .iter()
                .filter(|frame| {
                    frame
                        .atoms
                        .iter()
                        .any(|atom| matches!(atom, RelationAtom::ClientCapabilityAtom { .. }))
                })
                .count();
            let client_capability_profile_count = bucket
                .positives
                .iter()
                .chain(bucket.future_positives.iter())
                .map(|frame| {
                    frame
                        .atoms
                        .iter()
                        .filter_map(|atom| match atom {
                            RelationAtom::ClientCapabilityAtom { atom_id }
                            | RelationAtom::ReconstructedClientCapabilityAtom { atom_id } => {
                                Some(*atom_id)
                            }
                            _ => None,
                        })
                        .collect::<BTreeSet<_>>()
                })
                .filter(|profile| !profile.is_empty())
                .collect::<BTreeSet<_>>()
                .len();
            bucket_reports.push(OnlineResponseBucketReport {
                bucket_id: *bucket_id,
                structural_family_id: bucket.structural_family_id,
                teacher_signature_sha256: bucket.teacher_signature_sha256.clone(),
                teacher_action_symbol: bucket.teacher_action_symbol.clone(),
                positive_rows: bucket.positive_rows,
                negative_rows: bucket.negative_rows,
                positive_tokens: bucket.positive_tokens,
                negative_tokens: bucket.negative_tokens,
                false_accepts: wave_bucket.map_or(0, |bucket| bucket.false_accepts),
                rejected: wave_bucket.is_some_and(|bucket| bucket.rejected),
                learned_threshold_micro: wave_bucket
                    .map_or(self.config.threshold_floor_micro, |bucket| {
                        bucket.learned_threshold_micro
                    }),
                scored_events: wave_bucket.map_or(0, |bucket| bucket.scored_events),
                calibration_events_seen: wave_bucket
                    .map_or(0, |bucket| bucket.calibration_events_seen),
                max_calibration_false_margin_micro: wave_bucket
                    .and_then(|bucket| bucket.max_calibration_false_margin_micro),
                unique_cpu_accepts_over_exact_cache: wave_bucket
                    .map_or(0, |bucket| bucket.unique_cpu_accepts_over_exact_cache),
                candidate_blocker,
                synthesized_operation: synthesis.as_ref().ok().map(|operator| {
                    response_operation_name(&operator.candidate.program).to_owned()
                }),
                synthesis_error: synthesis
                    .as_ref()
                    .err()
                    .map(|error| error.code().to_owned()),
                example_positive_frame_id: bucket
                    .positives
                    .front()
                    .map(|frame| frame.frame_id_sha256.clone()),
                first_false_accept_frame_id: bucket.first_false_accept_frame_id.clone(),
                cardinality_guard_rejects: bucket.cardinality_guard_rejects,
                positive_cardinality_bounds: bucket.positive_cardinality_bounds.clone(),
                positive_cardinality_signature_count: bucket.positive_cardinality_signatures.len(),
                frozen_future_rows: bucket.future_positives.len(),
                frozen_future_sessions,
                distinct_surfaces,
                frozen_future_program_mismatches,
                support_watermark_event_time_unix_nanos: bucket
                    .support_watermark_event_time_unix_nanos,
                late_or_missing_time_rows: bucket.late_or_missing_time_rows,
                exact_guard_atom_count: bucket.exact_guard_atom_ids.len(),
                support_rows_with_client_capability,
                future_rows_with_client_capability,
                support_rows_with_reconstructed_capability: bucket
                    .positives
                    .iter()
                    .filter(|frame| {
                        frame.atoms.iter().any(|atom| {
                            matches!(atom, RelationAtom::ReconstructedClientCapabilityAtom { .. })
                        })
                    })
                    .count(),
                future_rows_with_reconstructed_capability: bucket
                    .future_positives
                    .iter()
                    .filter(|frame| {
                        frame.atoms.iter().any(|atom| {
                            matches!(atom, RelationAtom::ReconstructedClientCapabilityAtom { .. })
                        })
                    })
                    .count(),
                client_capability_profile_count,
                admission_precheck,
            });
            let (Some(wave_package), Ok(synthesized)) = (
                self.wave
                    .provisional_package_bytes(*bucket_id)
                    .ok()
                    .flatten(),
                synthesis,
            ) else {
                continue;
            };
            let distinct_sessions = bucket
                .positives
                .iter()
                .map(|frame| frame.session_id_sha256.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            candidates.push(OnlineResponseCandidate {
                bucket_id: *bucket_id,
                structural_family_id: bucket.structural_family_id,
                teacher_signature_sha256: bucket.teacher_signature_sha256.clone(),
                positive_rows: bucket.positive_rows,
                negative_rows: bucket.negative_rows,
                positive_tokens: bucket.positive_tokens,
                negative_tokens: bucket.negative_tokens,
                distinct_sessions,
                wave_threshold_micro: wave_package.threshold_micro,
                wave_runtime_bytes: wave_package.package_info.serialized_len,
                wave_runtime_fingerprint64: wave_package.package_info.fingerprint64,
                program: synthesized.candidate.program,
                verifier: synthesized.verifier,
                phase_rank: synthesized.candidate.phase_rank,
                exact_checks: synthesized.candidate.exact_checks,
            });
        }
        candidates.sort_by(|left, right| {
            right
                .positive_tokens
                .cmp(&left.positive_tokens)
                .then_with(|| left.bucket_id.cmp(&right.bucket_id))
        });
        bucket_reports.sort_by(|left, right| {
            right
                .positive_tokens
                .cmp(&left.positive_tokens)
                .then_with(|| right.positive_rows.cmp(&left.positive_rows))
                .then_with(|| left.bucket_id.cmp(&right.bucket_id))
        });
        let candidate_bucket_ids = candidates
            .iter()
            .map(|candidate| candidate.bucket_id)
            .collect::<BTreeSet<_>>();
        let mut action_families = BTreeMap::<String, OnlineResponseActionFamilyReport>::new();
        for bucket in &bucket_reports {
            let family = action_families
                .entry(bucket.teacher_action_symbol.clone())
                .or_insert_with(|| OnlineResponseActionFamilyReport {
                    teacher_action_symbol: bucket.teacher_action_symbol.clone(),
                    bucket_count: 0,
                    synthesizable_bucket_count: 0,
                    candidate_bucket_count: 0,
                    positive_rows: 0,
                    positive_tokens: 0,
                    frozen_future_rows: 0,
                });
            family.bucket_count = family.bucket_count.saturating_add(1);
            family.synthesizable_bucket_count = family
                .synthesizable_bucket_count
                .saturating_add(usize::from(bucket.synthesized_operation.is_some()));
            family.candidate_bucket_count = family.candidate_bucket_count.saturating_add(
                usize::from(candidate_bucket_ids.contains(&bucket.bucket_id)),
            );
            family.positive_rows = family.positive_rows.saturating_add(bucket.positive_rows);
            family.positive_tokens = family
                .positive_tokens
                .saturating_add(bucket.positive_tokens);
            family.frozen_future_rows = family
                .frozen_future_rows
                .saturating_add(bucket.frozen_future_rows);
        }
        let mut action_families = action_families.into_values().collect::<Vec<_>>();
        action_families.sort_by(|left, right| {
            right
                .positive_tokens
                .cmp(&left.positive_tokens)
                .then_with(|| left.teacher_action_symbol.cmp(&right.teacher_action_symbol))
        });
        let subcenter_candidate_count = self
            .buckets
            .values()
            .map(|bucket| bucket.teacher_action_symbol.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|action| self.subcenters.clean_subcenters(action, 64, 256).len())
            .sum();
        let admission_evaluation = self.combined_admission_evaluation();
        let self_training_v2 = self.self_training_v2.report(unix_now_seconds());
        let admission_ready_cohorts = admission_evaluation.ready_cohorts;
        let emitted_candidate_cohorts = admission_evaluation.candidates.len();
        let explicitly_blocked_cohorts = admission_evaluation.blockers.len();
        let v2_warm_bytes = self_training_v2
            .discovery
            .warm_bytes_estimate
            .saturating_add(
                self_training_v2
                    .cegis
                    .pools
                    .iter()
                    .map(|pool| pool.ast_nodes.saturating_mul(256))
                    .sum::<usize>(),
            );
        OnlineResponseMinerReport {
            schema: "nando.online-response-miner-report.v2".to_owned(),
            rows_seen: self.rows_seen,
            rows_learned: self.rows_learned,
            rows_ambiguous: self.rows_ambiguous,
            rows_without_teacher_action: self.rows_without_teacher_action,
            competing_negative_updates: self.competing_negative_updates,
            bucket_count: summary.bucket_count,
            active_bucket_count: summary.active_bucket_count,
            // Provisional programs remain visible in `candidates`, but this
            // counter represents only cohorts that reached admission gates.
            candidate_bucket_count: admission_ready_cohorts,
            false_accepts: usize::try_from(self_training_v2.opportunity.false_accepts)
                .unwrap_or(usize::MAX),
            discovery_false_accepts: summary.false_accepts,
            warm_bytes_estimate: self
                .wave
                .bytes_estimate()
                .saturating_add(self.subcenters.bytes_estimate())
                .saturating_add(v2_warm_bytes),
            subcenter_rows_seen: self.subcenters.rows_seen(),
            subcenter_pair_count: self.subcenters.pair_count(),
            subcenter_candidate_count,
            subcenter_bytes_estimate: self.subcenters.bytes_estimate(),
            action_families,
            buckets: bucket_reports,
            candidates,
            self_training_v2,
            live_scalar_shadow: self.live_scalar_shadow.report(),
            admission_ready_cohorts,
            emitted_candidate_cohorts,
            explicitly_blocked_cohorts,
            admission_candidate_blockers: admission_evaluation.blockers,
            admission_accounting_complete: admission_ready_cohorts
                == emitted_candidate_cohorts.saturating_add(explicitly_blocked_cohorts)
                && admission_ready_cohorts == admission_evaluation.ready_cohorts,
        }
    }

    fn ingest_result(&self) -> OnlineResponseIngestResult {
        OnlineResponseIngestResult {
            rows_seen: self.rows_seen,
            rows_learned: self.rows_learned,
            bucket_count: self.self_training_v2.teacher_pool_count(),
            candidate_bucket_count: self.self_training_v2.admission_ready_cohort_count(),
            false_accepts: 0,
        }
    }

    pub fn candidate_wave_package(&self, bucket_id: u32) -> Option<Vec<u8>> {
        self.wave
            .candidate_package_bytes(bucket_id)
            .ok()
            .flatten()
            .map(|package| package.package_bytes)
    }

    fn restored_core_admission_evaluation(&self) -> SelfTrainingAdmissionEvaluation {
        let mut ready_cohorts = 0usize;
        let mut evaluated = Vec::<(String, OnlineResponseAdmissionCandidate)>::new();
        let mut blockers = Vec::new();

        for bucket in self.buckets.values() {
            if bucket.positives.len() < 32
                || bucket.future_positives.len() < 32
                || (bucket.negatives.is_empty() && bucket.future_negatives.is_empty())
            {
                continue;
            }
            ready_cohorts = ready_cohorts.saturating_add(1);
            let cohort_id_sha256 = format!(
                "{:x}",
                Sha256::digest(
                    serde_json::to_vec(&(
                        "nando.restored-miner-cohort.v1",
                        bucket.structural_family_id,
                        &bucket.teacher_signature_sha256,
                    ))
                    .unwrap_or_default()
                )
            );
            let negatives = bucket
                .negatives
                .iter()
                .chain(bucket.future_negatives.iter())
                .cloned()
                .collect::<Vec<_>>();
            let support = bucket.positives.iter().cloned().collect::<Vec<_>>();
            let future = bucket.future_positives.iter().cloned().collect::<Vec<_>>();
            let synthesized = match synthesize_response_operator(&support) {
                Ok(synthesized) => synthesized,
                Err(error) => {
                    blockers.push(OnlineResponseAdmissionBlockerReport {
                        cohort_id_sha256,
                        blocker: format!("synthesis:{}", error.code()),
                    });
                    continue;
                }
            };
            let (required_atom_ids, support, future) = match repair_frozen_admission_guard(
                &synthesized.candidate.program,
                &bucket.exact_guard_atom_ids,
                &support,
                &future,
                &negatives,
            ) {
                Ok(repaired) => repaired,
                Err(blocker) => {
                    blockers.push(OnlineResponseAdmissionBlockerReport {
                        cohort_id_sha256,
                        blocker,
                    });
                    continue;
                }
            };
            let state_runtime_parity_cases = self
                .self_training_v2
                .runtime_parity_cases_for_frames(future.iter());
            let mut state_cases_by_ref = state_runtime_parity_cases
                .into_iter()
                .map(|case| (case.evidence_ref_sha256.clone(), case))
                .collect::<BTreeMap<_, _>>();
            let mut seen_future_events = BTreeSet::new();
            let mut receipt_backed_future = Vec::new();
            let mut runtime_parity_cases = Vec::new();
            for frame in future {
                let parity_key = canonical_runtime_parity_key(&frame);
                if !seen_future_events.insert(parity_key.clone()) {
                    continue;
                }
                let parity_case = self
                    .future_runtime_parity_cases
                    .get(&parity_key)
                    .cloned()
                    .or_else(|| state_cases_by_ref.remove(&frame.frame_id_sha256));
                if let Some(mut parity_case) = parity_case {
                    parity_case.evidence_ref_sha256 = frame.frame_id_sha256.clone();
                    receipt_backed_future.push(frame);
                    runtime_parity_cases.push(parity_case);
                }
            }
            if receipt_backed_future.len() < 32 {
                blockers.push(OnlineResponseAdmissionBlockerReport {
                    cohort_id_sha256,
                    blocker: format!(
                        "receipt_backed_future_rows_below_32:{}/32",
                        receipt_backed_future.len()
                    ),
                });
                continue;
            }
            match build_subcenter_admission_candidate(
                self.config,
                bucket,
                &required_atom_ids,
                support,
                receipt_backed_future,
                negatives,
                Some(ProvenAdmissionProgram {
                    program: &synthesized.candidate.program,
                    phase_rank: synthesized.candidate.phase_rank,
                    exact_checks: synthesized.candidate.exact_checks,
                }),
            ) {
                Ok(mut candidate) => {
                    candidate.runtime_parity_cases = runtime_parity_cases;
                    evaluated.push((cohort_id_sha256, candidate));
                }
                Err(blocker) => blockers.push(OnlineResponseAdmissionBlockerReport {
                    cohort_id_sha256,
                    blocker,
                }),
            }
        }

        evaluated.sort_by(|left, right| {
            right
                .1
                .candidate
                .positive_tokens
                .cmp(&left.1.candidate.positive_tokens)
                .then_with(|| left.1.candidate.bucket_id.cmp(&right.1.candidate.bucket_id))
        });
        let mut selected_signatures = BTreeSet::new();
        let mut candidates = Vec::new();
        for (cohort_id_sha256, candidate) in evaluated {
            if selected_signatures.insert(candidate.candidate.teacher_signature_sha256.clone()) {
                candidates.push(candidate);
            } else {
                blockers.push(OnlineResponseAdmissionBlockerReport {
                    cohort_id_sha256,
                    blocker: "dominated_by_higher_value_restored_bucket".to_owned(),
                });
            }
        }
        candidates.sort_by(|left, right| {
            right
                .candidate
                .positive_tokens
                .cmp(&left.candidate.positive_tokens)
                .then_with(|| left.candidate.bucket_id.cmp(&right.candidate.bucket_id))
        });
        blockers.sort_by(|left, right| left.cohort_id_sha256.cmp(&right.cohort_id_sha256));
        SelfTrainingAdmissionEvaluation {
            ready_cohorts,
            candidates,
            blockers,
        }
    }

    fn self_training_v2_admission_evaluation(&self) -> SelfTrainingAdmissionEvaluation {
        let mut candidates = Vec::new();
        let mut blockers = Vec::new();
        for cohort in self.self_training_v2.admission_cohorts() {
            let cohort_id_sha256 = cohort.winner.cohort_id_sha256.clone();
            let admission_negatives = cohort
                .generation
                .negatives
                .iter()
                .filter(|frame| {
                    frame.observed_at_unix_nanos > cohort.winner.repair_watermark_unix_nanos
                        && teacher_action_symbol(frame) != cohort.winner.action_symbol
                })
                .cloned()
                .collect::<Vec<_>>();
            if admission_negatives.is_empty() {
                blockers.push(OnlineResponseAdmissionBlockerReport {
                    cohort_id_sha256,
                    blocker: format!(
                        "post_repair_negative_evidence_missing:repair_watermark={}",
                        cohort.winner.repair_watermark_unix_nanos
                    ),
                });
                continue;
            }
            let (required_atom_ids, support, future) = match repair_frozen_admission_guard(
                &cohort.winner.program,
                &cohort.winner.required_atom_ids,
                &cohort.generation.support,
                &cohort.generation.future,
                &admission_negatives,
            ) {
                Ok(repaired) => repaired,
                Err(blocker) => {
                    blockers.push(OnlineResponseAdmissionBlockerReport {
                        cohort_id_sha256,
                        blocker,
                    });
                    continue;
                }
            };
            let family_digest = Sha256::digest(cohort.winner.cohort_id_sha256.as_bytes());
            let structural_family_id =
                u64::from_be_bytes(family_digest[..8].try_into().unwrap_or([0; 8]));
            let virtual_bucket = ResponseBucket {
                structural_family_id,
                teacher_signature_sha256: cohort.winner.teacher_signature_sha256.clone(),
                teacher_action_symbol: cohort.winner.action_symbol.clone(),
                positives: support.iter().cloned().collect(),
                negatives: admission_negatives.iter().cloned().collect(),
                future_positives: future.iter().cloned().collect(),
                future_negatives: VecDeque::new(),
                positive_rows: usize::try_from(cohort.pool.positive_rows).unwrap_or(usize::MAX),
                negative_rows: admission_negatives.len(),
                positive_tokens: cohort.pool.positive_tokens,
                negative_tokens: cohort.pool.negative_tokens,
                first_false_accept_frame_id: None,
                cardinality_guard_rejects: 0,
                positive_cardinality_bounds: BTreeMap::new(),
                positive_cardinality_signatures: BTreeSet::new(),
                support_watermark_event_time_unix_nanos: cohort
                    .generation
                    .support_watermark_unix_nanos,
                late_or_missing_time_rows: 0,
                exact_guard_atom_ids: required_atom_ids.clone(),
            };
            let mut candidate = match build_subcenter_admission_candidate(
                self.config,
                &virtual_bucket,
                &required_atom_ids,
                support,
                future,
                admission_negatives,
                Some(ProvenAdmissionProgram {
                    program: &cohort.winner.program,
                    phase_rank: cohort.winner.phase_rank,
                    exact_checks: u32::try_from(cohort.winner.exact_checks).unwrap_or(u32::MAX),
                }),
            ) {
                Ok(candidate) => candidate,
                Err(blocker) => {
                    blockers.push(OnlineResponseAdmissionBlockerReport {
                        cohort_id_sha256,
                        blocker,
                    });
                    continue;
                }
            };
            if candidate.candidate.program != cohort.winner.program {
                blockers.push(OnlineResponseAdmissionBlockerReport {
                    cohort_id_sha256,
                    blocker: "candidate_program_changed_after_proof".to_owned(),
                });
                continue;
            }
            let future_runtime_parity_cases = self
                .self_training_v2
                .runtime_parity_cases_for_frames(candidate.future.iter());
            if future_runtime_parity_cases.len() < 32 {
                blockers.push(OnlineResponseAdmissionBlockerReport {
                    cohort_id_sha256,
                    blocker: format!(
                        "semantic_future_runtime_parity_below_32:{}/32",
                        future_runtime_parity_cases.len()
                    ),
                });
                continue;
            }
            candidate.runtime_parity_cases = self.self_training_v2.runtime_parity_cases_for_frames(
                candidate.support.iter().chain(candidate.future.iter()),
            );
            candidate.semantic_alias_edges = cohort.semantic_alias_edges;
            candidates.push(candidate);
        }
        candidates.sort_by(|left, right| {
            right
                .candidate
                .positive_tokens
                .cmp(&left.candidate.positive_tokens)
                .then_with(|| left.candidate.bucket_id.cmp(&right.candidate.bucket_id))
        });
        blockers.sort_by(|left, right| left.cohort_id_sha256.cmp(&right.cohort_id_sha256));
        SelfTrainingAdmissionEvaluation {
            ready_cohorts: candidates.len().saturating_add(blockers.len()),
            candidates,
            blockers,
        }
    }

    fn combined_admission_evaluation(&self) -> SelfTrainingAdmissionEvaluation {
        let restored = self.restored_core_admission_evaluation();
        let semantic = self.self_training_v2_admission_evaluation();
        let ready_cohorts = restored
            .ready_cohorts
            .saturating_add(semantic.ready_cohorts);
        let mut blockers = restored
            .blockers
            .into_iter()
            .chain(semantic.blockers)
            .collect::<Vec<_>>();
        let mut candidates = Vec::new();
        let mut proof_payloads = BTreeSet::new();
        for candidate in restored.candidates.into_iter().chain(semantic.candidates) {
            let proof_payload_sha256 = format!(
                "{:x}",
                Sha256::digest(
                    serde_json::to_vec(&(
                        "nando.combined-admission-proof-payload.v1",
                        &candidate.candidate.program,
                        &candidate.candidate.verifier,
                        &candidate.required_routing_atom_ids,
                        &candidate.wave_runtime_package,
                    ))
                    .unwrap_or_default()
                )
            );
            if proof_payloads.insert(proof_payload_sha256.clone()) {
                candidates.push(candidate);
            } else {
                blockers.push(OnlineResponseAdmissionBlockerReport {
                    cohort_id_sha256: proof_payload_sha256,
                    blocker: "duplicate_admission_proof_payload".to_owned(),
                });
            }
        }
        candidates.sort_by(|left, right| {
            right
                .candidate
                .positive_tokens
                .cmp(&left.candidate.positive_tokens)
                .then_with(|| left.candidate.bucket_id.cmp(&right.candidate.bucket_id))
        });
        blockers.sort_by(|left, right| left.cohort_id_sha256.cmp(&right.cohort_id_sha256));
        SelfTrainingAdmissionEvaluation {
            ready_cohorts,
            candidates,
            blockers,
        }
    }

    pub fn admission_candidates(&self) -> Vec<OnlineResponseAdmissionCandidate> {
        self.combined_admission_evaluation().candidates
    }

    #[must_use]
    pub fn crystallized_admission_candidates(&self) -> Vec<crate::LiveScalarAdmissionCandidate> {
        self.live_scalar_shadow.admission_candidates()
    }

    fn bucket_for(
        &mut self,
        family_id: u64,
        signature: &str,
        frame: &RelationFrame,
    ) -> Result<u32, String> {
        let key = (family_id, signature.to_owned());
        if let Some(bucket_id) = self.keys.get(&key) {
            return Ok(*bucket_id);
        }
        if self.buckets.len() >= self.config.max_buckets {
            return Err("online_response_bucket_capacity_exceeded".to_owned());
        }
        let bucket_id = stable_bucket_id(family_id, signature);
        if self.buckets.contains_key(&bucket_id) {
            return Err("online_response_bucket_hash_collision".to_owned());
        }
        self.keys.insert(key, bucket_id);
        self.buckets.insert(
            bucket_id,
            ResponseBucket {
                structural_family_id: family_id,
                teacher_signature_sha256: signature.to_owned(),
                teacher_action_symbol: teacher_action_symbol(frame),
                positives: VecDeque::with_capacity(self.config.reservoir_rows),
                negatives: VecDeque::with_capacity(self.config.reservoir_rows),
                future_positives: VecDeque::with_capacity(self.config.reservoir_rows),
                future_negatives: VecDeque::with_capacity(8),
                positive_rows: 0,
                negative_rows: 0,
                positive_tokens: 0,
                negative_tokens: 0,
                first_false_accept_frame_id: None,
                cardinality_guard_rejects: 0,
                positive_cardinality_bounds: BTreeMap::new(),
                positive_cardinality_signatures: BTreeSet::new(),
                support_watermark_event_time_unix_nanos: 0,
                late_or_missing_time_rows: 0,
                exact_guard_atom_ids: Vec::new(),
            },
        );
        Ok(bucket_id)
    }
}

type RepairedAdmissionGuard = (Vec<u64>, Vec<RelationFrame>, Vec<RelationFrame>);

fn repair_frozen_admission_guard(
    program: &crate::ResponseProgram,
    required_atom_ids: &[u64],
    support: &[RelationFrame],
    future: &[RelationFrame],
    negatives: &[RelationFrame],
) -> Result<RepairedAdmissionGuard, String> {
    if support.len() < 32 || future.len() < 32 {
        return Err(format!(
            "guard_evidence_below_gate:support={}:future={}",
            support.len(),
            future.len()
        ));
    }
    let mut base_required = required_atom_ids.to_vec();
    base_required.sort_unstable();
    base_required.dedup();
    let frame_matches = |frame: &RelationFrame, required: &[u64]| {
        let observed = relation_frame_online_routing_atom_ids(frame);
        required
            .iter()
            .all(|atom| observed.binary_search(atom).is_ok())
    };
    let routed_negatives = negatives
        .iter()
        .filter(|frame| frame_matches(frame, &base_required))
        .collect::<Vec<_>>();
    if routed_negatives.is_empty() {
        return Ok((base_required, support.to_vec(), future.to_vec()));
    }
    let applicable_negatives = routed_negatives
        .iter()
        .copied()
        .filter(|frame| crate::synthesis::program_runtime_applicable(program, frame))
        .collect::<Vec<_>>();
    if applicable_negatives.is_empty() {
        return Ok((base_required, support.to_vec(), future.to_vec()));
    }

    let support_atoms = support
        .iter()
        .map(relation_frame_online_routing_atom_ids)
        .collect::<Vec<_>>();
    let future_atoms = future
        .iter()
        .map(relation_frame_online_routing_atom_ids)
        .collect::<Vec<_>>();
    let negative_atoms = applicable_negatives
        .iter()
        .map(|frame| relation_frame_online_routing_atom_ids(frame))
        .collect::<Vec<_>>();
    let mut frequency = BTreeMap::<u64, usize>::new();
    for observed in support_atoms.iter().chain(&future_atoms) {
        for atom in observed {
            if base_required.binary_search(atom).is_err() {
                *frequency.entry(*atom).or_default() += 1;
            }
        }
    }
    let mut ranked_atoms = frequency.into_iter().collect::<Vec<_>>();
    ranked_atoms.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    ranked_atoms.truncate(64);
    let ranked_atoms = ranked_atoms
        .into_iter()
        .map(|(atom, _)| atom)
        .collect::<Vec<_>>();
    let mut predicates = ranked_atoms
        .iter()
        .map(|atom| vec![*atom])
        .collect::<Vec<_>>();
    for (left_index, left) in ranked_atoms.iter().enumerate() {
        for right in ranked_atoms.iter().skip(left_index.saturating_add(1)) {
            predicates.push(vec![*left, *right]);
        }
    }
    let Some(best) = predicates
        .into_iter()
        .filter_map(|predicate| {
            let mut combined = base_required.clone();
            combined.extend(predicate);
            combined.sort_unstable();
            combined.dedup();
            let support_rows = support_atoms
                .iter()
                .filter(|observed| {
                    combined
                        .iter()
                        .all(|atom| observed.binary_search(atom).is_ok())
                })
                .count();
            let future_rows = future_atoms
                .iter()
                .filter(|observed| {
                    combined
                        .iter()
                        .all(|atom| observed.binary_search(atom).is_ok())
                })
                .count();
            if support_rows < 32
                || future_rows < 32
                || negative_atoms.iter().any(|observed| {
                    combined
                        .iter()
                        .all(|atom| observed.binary_search(atom).is_ok())
                })
            {
                return None;
            }
            Some((combined, support_rows, future_rows))
        })
        .max_by(|left, right| {
            left.1
                .saturating_add(left.2)
                .cmp(&right.1.saturating_add(right.2))
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| right.0.len().cmp(&left.0.len()))
                .then_with(|| right.0.cmp(&left.0))
        })
    else {
        // CEGIS deliberately delegates a pre-action collision that no exact
        // guard can separate to the learned anti-center. The candidate builder
        // below must still prove that Wave rejects these negatives with zero
        // false accepts before external admission sees the package.
        return Ok((base_required, support.to_vec(), future.to_vec()));
    };
    let repaired_support = support
        .iter()
        .filter(|frame| frame_matches(frame, &best.0))
        .cloned()
        .collect::<Vec<_>>();
    let repaired_future = future
        .iter()
        .filter(|frame| frame_matches(frame, &best.0))
        .cloned()
        .collect::<Vec<_>>();
    Ok((best.0, repaired_support, repaired_future))
}

fn build_subcenter_admission_candidate(
    config: OnlineResponseMinerConfig,
    bucket: &ResponseBucket,
    required_atom_ids: &[u64],
    support: Vec<RelationFrame>,
    future: Vec<RelationFrame>,
    negatives: Vec<RelationFrame>,
    proven: Option<ProvenAdmissionProgram<'_>>,
) -> Result<OnlineResponseAdmissionCandidate, String> {
    let parent_bucket_id = stable_action_signature_bucket_id(
        &bucket.teacher_action_symbol,
        &bucket.teacher_signature_sha256,
    );
    if support.len() < 32 || future.len() < 32 || negatives.is_empty() {
        trace_subcenter_build(parent_bucket_id, required_atom_ids, "evidence_below_gate");
        return Err(format!(
            "evidence_below_gate:support={}:future={}:negatives={}",
            support.len(),
            future.len(),
            negatives.len()
        ));
    }
    let calibration_events = config.calibration_events.min(negatives.len()).max(1);
    let (program, verifier, phase_rank, exact_checks) = if let Some(proven) = proven {
        if !support
            .iter()
            .all(|frame| crate::synthesis::program_is_consistent(proven.program, frame))
        {
            trace_subcenter_build(
                parent_bucket_id,
                required_atom_ids,
                "cegis_support_mismatch",
            );
            return Err("cegis_support_mismatch".to_owned());
        }
        let verifier = match crate::synthesis::compile_independent_verifier(proven.program) {
            Ok(verifier) => verifier,
            Err(error) => {
                trace_subcenter_build(parent_bucket_id, required_atom_ids, error.code());
                return Err(format!("verifier_compile:{}", error.code()));
            }
        };
        (
            proven.program.clone(),
            verifier,
            proven.phase_rank,
            proven.exact_checks,
        )
    } else {
        let synthesized = match synthesize_response_operator(&support) {
            Ok(synthesized) => synthesized,
            Err(error) => {
                trace_subcenter_build(parent_bucket_id, required_atom_ids, error.code());
                return Err(format!("synthesis:{}", error.code()));
            }
        };
        (
            synthesized.candidate.program,
            synthesized.verifier,
            synthesized.candidate.phase_rank,
            synthesized.candidate.exact_checks,
        )
    };
    let offered_future = future.len();
    let future = future
        .into_iter()
        .filter(|frame| {
            frame.verifier_label == Some(true)
                && crate::synthesis::program_is_consistent(&program, frame)
        })
        .collect::<Vec<_>>();
    if future.len() < 32 {
        trace_subcenter_build(
            parent_bucket_id,
            required_atom_ids,
            &format!(
                "consistent_future_below_gate:{}/{}",
                future.len(),
                offered_future
            ),
        );
        return Err(format!(
            "consistent_future_below_gate:{}/{}",
            future.len(),
            offered_future
        ));
    }
    let bucket_id = stable_subcenter_bucket_id(parent_bucket_id, required_atom_ids);
    let wave_config = PhaseCenterOnlineMinerConfig {
        cells: config.cells,
        min_bucket_events: config.min_bucket_events,
        threshold_floor_micro: config.threshold_floor_micro,
        calibration_events,
        max_buckets: 1,
    };
    let mut wave =
        PhaseCenterOnlineMiner::new(wave_config).map_err(|error| format!("wave_init:{error:?}"))?;
    let mut encoder = PhaseCenterAtomEncoder::new(config.cells)
        .map_err(|error| format!("wave_encoder_init:{error:?}"))?;
    for frame in &support {
        let wave_atoms = subcenter_wave_atom_ids(frame, required_atom_ids);
        wave.train_atom_ids(&mut encoder, bucket_id, wave_atoms, true)
            .map_err(|error| format!("wave_support_train:{error:?}"))?;
    }
    for frame in negatives.iter().take(calibration_events) {
        let wave_atoms = subcenter_wave_atom_ids(frame, required_atom_ids);
        wave.train_atom_ids(&mut encoder, bucket_id, wave_atoms, false)
            .map_err(|error| format!("wave_negative_train:{error:?}"))?;
    }
    for frame in negatives.iter().take(calibration_events) {
        let wave_atoms = subcenter_wave_atom_ids(frame, required_atom_ids);
        wave.observe_atom_ids(&mut encoder, bucket_id, wave_atoms, false, false, 0, 0)
            .map_err(|error| format!("wave_negative_calibration:{error:?}"))?;
    }
    for frame in &future {
        let wave_atoms = subcenter_wave_atom_ids(frame, required_atom_ids);
        let decision = wave
            .observe_atom_ids(
                &mut encoder,
                bucket_id,
                wave_atoms,
                true,
                false,
                frame.estimated_input_tokens,
                0,
            )
            .map_err(|error| format!("wave_future_observe:{error:?}"))?;
        if decision.unique_cpu_accept_over_exact_cache {
            break;
        }
    }
    let wave_bucket = wave
        .bucket(bucket_id)
        .ok_or_else(|| "wave_bucket_missing".to_owned())?;
    if wave_bucket.rejected
        || wave_bucket.false_accepts != 0
        || !wave_bucket.is_shadow_ready(config.min_bucket_events, calibration_events)
    {
        trace_subcenter_build(parent_bucket_id, required_atom_ids, "wave_not_shadow_ready");
        return Err("wave_not_shadow_ready".to_owned());
    }
    // This is only a proof candidate. The independent admission controller
    // replays frozen future, calibrates routing, runs causal ablations, and
    // checks runtime parity before granting execution authority.
    let wave_package = wave
        .shadow_ready_package_bytes(bucket_id)
        .map_err(|error| format!("wave_package:{error:?}"))?
        .ok_or_else(|| "wave_package_missing".to_owned())?;
    let distinct_sessions = support
        .iter()
        .map(|frame| frame.session_id_sha256.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let positive_tokens = support
        .iter()
        .chain(future.iter())
        .map(|frame| frame.estimated_input_tokens)
        .sum();
    Ok(OnlineResponseAdmissionCandidate {
        candidate: OnlineResponseCandidate {
            bucket_id,
            structural_family_id: stable_subcenter_family_id(
                bucket.structural_family_id,
                required_atom_ids,
            ),
            teacher_signature_sha256: bucket.teacher_signature_sha256.clone(),
            positive_rows: support.len().saturating_add(future.len()),
            negative_rows: bucket.negative_rows,
            positive_tokens,
            negative_tokens: bucket.negative_tokens,
            distinct_sessions,
            wave_threshold_micro: wave_package.threshold_micro,
            wave_runtime_bytes: wave_package.package_info.serialized_len,
            wave_runtime_fingerprint64: wave_package.package_info.fingerprint64,
            program,
            verifier,
            phase_rank,
            exact_checks,
        },
        wave_runtime_package: wave_package.package_bytes,
        support,
        future,
        negatives,
        required_routing_atom_ids: required_atom_ids.to_vec(),
        runtime_parity_cases: Vec::new(),
        semantic_alias_edges: Vec::new(),
    })
}

fn subcenter_wave_atom_ids(frame: &RelationFrame, _required_atom_ids: &[u64]) -> Vec<u64> {
    relation_frame_online_routing_atom_ids(frame)
}

#[derive(Clone, Copy)]
struct ProvenAdmissionProgram<'a> {
    program: &'a crate::ResponseProgram,
    phase_rank: u32,
    exact_checks: u32,
}

struct SelfTrainingAdmissionEvaluation {
    ready_cohorts: usize,
    candidates: Vec<OnlineResponseAdmissionCandidate>,
    blockers: Vec<OnlineResponseAdmissionBlockerReport>,
}

fn trace_subcenter_build(parent_bucket_id: u32, atom_ids: &[u64], reason: &str) {
    if std::env::var_os("NANDO_ONLINE_ADMISSION_TRACE").is_some() {
        eprintln!(
            "online_subcenter_build parent={parent_bucket_id} atoms={atom_ids:?} blocker={reason}"
        );
    }
}

fn stable_action_signature_bucket_id(action: &str, signature: &str) -> u32 {
    let digest = Sha256::digest(
        serde_json::to_vec(&("nando.online-action-signature.v1", action, signature))
            .unwrap_or_default(),
    );
    u32::from_be_bytes(digest[..4].try_into().unwrap_or([0; 4]))
}

fn stable_subcenter_bucket_id(parent_bucket_id: u32, atom_ids: &[u64]) -> u32 {
    let digest = Sha256::digest(
        serde_json::to_vec(&("nando.online-subcenter.v1", parent_bucket_id, atom_ids))
            .unwrap_or_default(),
    );
    u32::from_be_bytes(digest[..4].try_into().unwrap_or([0; 4]))
}

fn stable_subcenter_family_id(parent_family_id: u64, atom_ids: &[u64]) -> u64 {
    let digest = Sha256::digest(
        serde_json::to_vec(&(
            "nando.online-subcenter-family.v1",
            parent_family_id,
            atom_ids,
        ))
        .unwrap_or_default(),
    );
    u64::from_be_bytes(digest[..8].try_into().unwrap_or([0; 8]))
}

fn normalize_online_completion_state(frame: &mut RelationFrame) {
    let continuation_pending = frame.atoms.iter().any(|atom| {
        matches!(
            atom,
            RelationAtom::ObservationSelector {
                selector: crate::ResponseValueSelector::ContentLinePrefix { prefix, .. },
                ..
            } if prefix == "Script running with cell ID "
                || prefix == "Process running with session ID "
        )
    });
    if continuation_pending {
        for atom in &mut frame.atoms {
            if let RelationAtom::CompletionState { value } = atom {
                *value = "pending".to_owned();
            }
        }
    }
}

fn canonicalize_online_frame(frame: &mut RelationFrame) {
    normalize_online_completion_state(frame);
    reconstruct_online_client_capability(frame);
    frame.atoms.sort();
    frame.atoms.dedup();
}

fn reconstruct_online_client_capability(frame: &mut RelationFrame) {
    if frame.atoms.iter().any(|atom| {
        matches!(
            atom,
            RelationAtom::ClientCapabilityAtom { .. }
                | RelationAtom::ReconstructedClientCapabilityAtom { .. }
        )
    }) {
        return;
    }
    let capability = frame.atoms.iter().find_map(|atom| match atom {
        RelationAtom::ActionFunction { value } => Some(crate::package::stable_atom_id(&format!(
            "client_capability:function:{value}"
        ))),
        RelationAtom::ActionCustomTool { value } => Some(crate::package::stable_atom_id(&format!(
            "client_capability:custom:{value}"
        ))),
        _ => None,
    });
    if let Some(atom_id) = capability {
        frame
            .atoms
            .push(RelationAtom::ReconstructedClientCapabilityAtom { atom_id });
    }
}

fn response_operation_name(program: &ResponseProgram) -> &'static str {
    match &program.operation {
        crate::ResponseOperation::AdvancePlan { .. } => "advance_plan",
        crate::ResponseOperation::FunctionCallFromRoles { function_name, .. } => {
            if function_name == "write_stdin" {
                "write_stdin"
            } else {
                "function_call_from_roles"
            }
        }
        crate::ResponseOperation::CustomToolCallFromRoles {
            inner_tool_name, ..
        } => {
            if inner_tool_name == "write_stdin" {
                "write_stdin"
            } else {
                "custom_tool_call_from_roles"
            }
        }
        crate::ResponseOperation::ProjectSelectedValue { .. } => "project_selected_value",
        crate::ResponseOperation::ProjectStatus { .. } => "project_status",
        crate::ResponseOperation::ComposeCollection { .. } => "compose_collection",
        _ => "other",
    }
}

fn push_bounded<T>(rows: &mut VecDeque<T>, row: T, limit: usize) {
    if rows.len() == limit {
        rows.pop_front();
    }
    rows.push_back(row);
}

fn canonical_runtime_parity_key(frame: &RelationFrame) -> String {
    let digest = Sha256::digest(
        serde_json::to_vec(&(
            "nando.future-runtime-parity.v1",
            &frame.evidence_ref_sha256,
            &frame.event_id_sha256,
            &frame.session_id_sha256,
        ))
        .unwrap_or_default(),
    );
    format!("{:020}:{digest:x}", frame.observed_at_unix_nanos)
}

fn push_session_diverse_future(
    rows: &mut VecDeque<RelationFrame>,
    row: RelationFrame,
    limit: usize,
) {
    if limit == 0 {
        return;
    }
    if rows.len() < limit {
        rows.push_back(row);
        return;
    }

    let incoming_session = row.session_id_sha256.as_str();
    let replacement = rows
        .iter()
        .position(|existing| existing.session_id_sha256 == incoming_session)
        .or_else(|| {
            rows.iter().enumerate().find_map(|(index, existing)| {
                let copies = rows
                    .iter()
                    .filter(|candidate| candidate.session_id_sha256 == existing.session_id_sha256)
                    .count();
                (copies > 1).then_some(index)
            })
        })
        .unwrap_or(0);
    rows.remove(replacement);
    rows.push_back(row);
}

fn trim_session_diverse_future(rows: &mut VecDeque<RelationFrame>, limit: usize) {
    if rows.len() <= limit {
        return;
    }
    let mut bounded = VecDeque::with_capacity(limit);
    for row in std::mem::take(rows) {
        push_session_diverse_future(&mut bounded, row, limit);
    }
    *rows = bounded;
}

fn online_admission_precheck(
    bucket: &ResponseBucket,
    learned_wave_route: Option<&crate::LearnedWaveRoute>,
) -> String {
    let negatives = bucket
        .negatives
        .iter()
        .chain(bucket.future_negatives.iter())
        .cloned()
        .collect::<Vec<_>>();
    let (support, mut frozen_future, required_routing_atom_ids) =
        clean_admission_partition(bucket, &negatives);
    let training = support
        .iter()
        .chain(negatives.iter())
        .cloned()
        .collect::<Vec<_>>();
    let mut packages = crate::compile_source_neutral_quarantine_packages(&training, true);
    if packages.len() != 1 {
        return format!("package_compile_count:{}", packages.len());
    }
    let package = &mut packages[0];
    frozen_future
        .retain(|frame| crate::frame_matches_program_action_contract(&package.program, frame));
    crate::lifecycle::apply_clean_routing_refinement(package, &support, &negatives);
    let _ = required_routing_atom_ids;
    let mut refined_support = support
        .iter()
        .filter(|frame| crate::package::relation_frame_matches_package_guard(package, frame))
        .cloned()
        .collect::<Vec<_>>();
    if refined_support.len() < 32 {
        return format!("refined_support_below_32:{}", refined_support.len());
    }
    if let Some(route) = learned_wave_route {
        package.wave_margin_micro = route.threshold_micro;
        package.learned_wave_route = Some(route.clone());
        let guard_relevant_negatives = negatives
            .iter()
            .filter(|frame| crate::package::relation_frame_matches_package_guard(package, frame))
            .cloned()
            .collect::<Vec<_>>();
        if !crate::online_admission::ensure_support_separating_learned_route(
            package,
            &refined_support,
            &guard_relevant_negatives,
        ) {
            return "learned_wave_overlap:no_support_only_separating_route".to_owned();
        }
        (refined_support, frozen_future) = crate::online_admission::phase_clean_support_future(
            package,
            &refined_support,
            &frozen_future,
            &negatives,
        );
        if refined_support.len() < 32 || frozen_future.len() < 32 {
            return format!(
                "phase_clean_rows_below_32:support={}:future={}",
                refined_support.len(),
                frozen_future.len()
            );
        }
    }
    let routed_future = frozen_future
        .iter()
        .filter(|frame| crate::relation_frame_routes_to_package(package, frame))
        .cloned()
        .collect::<Vec<_>>();
    if routed_future.len() < 32 {
        return format!("routed_future_below_32:{}", routed_future.len());
    }
    let future_wrong = routed_future
        .iter()
        .filter(|frame| {
            frame.verifier_label != Some(true)
                || !crate::frame_matches_program_action_contract(&package.program, frame)
        })
        .count();
    if future_wrong != 0 {
        return format!("future_action_contract_mismatch:{future_wrong}");
    }
    let negative_accepts = bucket
        .negatives
        .iter()
        .chain(bucket.future_negatives.iter())
        .filter(|frame| crate::relation_frame_routes_to_package(package, frame))
        .count();
    if negative_accepts != 0 {
        return format!("negative_routes_to_package:{negative_accepts}");
    }
    let causal = crate::evaluate_grounded_wave_causality(
        package,
        &refined_support,
        &routed_future,
        &negatives,
    );
    if causal.verdict != "PASS" {
        return format!(
            "causal_{}:full={}/{}:negative_accepts={}",
            causal.verdict.to_ascii_lowercase(),
            causal.full_phase_correct,
            causal.future_rows,
            causal.negative_accepts
        );
    }
    "pass_to_snapshot".to_owned()
}

fn clean_admission_partition(
    bucket: &ResponseBucket,
    negatives: &[RelationFrame],
) -> (Vec<RelationFrame>, Vec<RelationFrame>, Vec<u64>) {
    let mut positives = bucket
        .positives
        .iter()
        .chain(bucket.future_positives.iter())
        .cloned()
        .collect::<Vec<_>>();
    positives.sort_by(|left, right| {
        left.observed_at_unix_nanos
            .cmp(&right.observed_at_unix_nanos)
            .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
    });
    let mut seen_events = BTreeSet::new();
    positives.retain(|frame| seen_events.insert(frame.event_id_sha256.clone()));
    let positive_atoms = positives
        .iter()
        .map(relation_frame_online_routing_atom_ids)
        .collect::<Vec<_>>();
    let collision_atoms = negatives
        .iter()
        .map(relation_frame_online_routing_atom_ids)
        .filter(|negative| positive_atoms.iter().any(|positive| positive == negative))
        .collect::<Vec<_>>();
    positives.retain(|frame| {
        let atoms = relation_frame_online_routing_atom_ids(frame);
        collision_atoms.iter().all(|collision| collision != &atoms)
    });

    let mut counts = BTreeMap::<u64, usize>::new();
    for frame in &positives {
        for atom in relation_frame_online_routing_atom_ids(frame) {
            if collision_atoms
                .iter()
                .all(|collision| collision.binary_search(&atom).is_err())
            {
                *counts.entry(atom).or_default() += 1;
            }
        }
    }
    let separator = counts
        .into_iter()
        .filter(|(_, count)| *count >= 64)
        .max_by(|(left_atom, left_count), (right_atom, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| right_atom.cmp(left_atom))
        })
        .map(|(atom, _)| atom);
    if let Some(separator) = separator {
        positives.retain(|frame| {
            relation_frame_online_routing_atom_ids(frame)
                .binary_search(&separator)
                .is_ok()
        });
    }
    let support = positives.iter().take(32).cloned().collect::<Vec<_>>();
    let watermark = support
        .iter()
        .map(|frame| frame.observed_at_unix_nanos)
        .max()
        .unwrap_or(0);
    let future = positives
        .into_iter()
        .skip(support.len())
        .filter(|frame| frame.observed_at_unix_nanos > watermark)
        .collect::<Vec<_>>();
    let mut required = bucket.exact_guard_atom_ids.clone();
    if let Some(separator) = separator {
        required.push(separator);
    }
    required.sort_unstable();
    required.dedup();
    (support, future, required)
}

fn update_cardinality_bounds(bucket: &mut ResponseBucket, frame: &RelationFrame) {
    for atom in &frame.atoms {
        let RelationAtom::Cardinality { role, count } = atom else {
            continue;
        };
        bucket
            .positive_cardinality_bounds
            .entry(role.clone())
            .and_modify(|(minimum, maximum)| {
                *minimum = (*minimum).min(*count);
                *maximum = (*maximum).max(*count);
            })
            .or_insert((*count, *count));
    }
    if bucket.positive_cardinality_signatures.len() < 64
        && let Some(signature) = cardinality_signature(frame)
    {
        bucket.positive_cardinality_signatures.insert(signature);
    }
}

fn cardinality_guard_matches(bucket: &ResponseBucket, frame: &RelationFrame) -> bool {
    if !bucket.exact_guard_atom_ids.is_empty() {
        let observed = exact_guard_atom_ids(frame);
        if !bucket
            .exact_guard_atom_ids
            .iter()
            .all(|required| observed.binary_search(required).is_ok())
        {
            return false;
        }
    }
    if bucket.positive_cardinality_bounds.is_empty() {
        return true;
    }
    if !bucket.positive_cardinality_signatures.is_empty()
        && cardinality_signature(frame)
            .is_none_or(|signature| !bucket.positive_cardinality_signatures.contains(&signature))
    {
        return false;
    }
    let observed = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::Cardinality { role, count } => Some((role.as_str(), *count)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    bucket
        .positive_cardinality_bounds
        .iter()
        .all(|(role, (minimum, maximum))| {
            observed
                .get(role.as_str())
                .is_some_and(|count| count >= minimum && count <= maximum)
        })
}

fn update_exact_guard(bucket: &mut ResponseBucket, frame: &RelationFrame) {
    let observed = exact_guard_atom_ids(frame);
    if bucket.positives.is_empty() {
        bucket.exact_guard_atom_ids = observed;
    } else {
        bucket
            .exact_guard_atom_ids
            .retain(|required| observed.binary_search(required).is_ok());
    }
}

fn recompute_exact_guard(bucket: &mut ResponseBucket) {
    let mut positives = bucket
        .positives
        .iter()
        .chain(bucket.future_positives.iter());
    let Some(first) = positives.next() else {
        bucket.exact_guard_atom_ids.clear();
        return;
    };
    let mut required = exact_guard_atom_ids(first);
    for frame in positives {
        let observed = exact_guard_atom_ids(frame);
        required.retain(|atom| observed.binary_search(atom).is_ok());
    }
    bucket.exact_guard_atom_ids = required;
}

fn exact_guard_atom_ids(frame: &RelationFrame) -> Vec<u64> {
    let filtered = RelationFrame {
        atoms: frame
            .atoms
            .iter()
            .filter(|atom| !matches!(atom, RelationAtom::Cardinality { .. }))
            .cloned()
            .collect(),
        ..frame.clone()
    };
    relation_frame_online_routing_atom_ids(&filtered)
}

fn cardinality_signature(frame: &RelationFrame) -> Option<String> {
    let mut parts = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::Cardinality { role, count } => Some(format!("{role}={count}")),
            _ => None,
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    parts.sort();
    Some(parts.join("|"))
}

fn online_bucket_identity(frame: &RelationFrame) -> Option<(u64, String)> {
    let teacher_signature = teacher_program_signature(frame)?;
    if frame
        .atoms
        .iter()
        .any(|atom| matches!(atom, RelationAtom::ActionPlanAdvance))
    {
        let function_name = frame.atoms.iter().find_map(|atom| match atom {
            RelationAtom::ActionFunction { value } => Some(value.as_str()),
            _ => None,
        })?;
        let digest = Sha256::digest(
            serde_json::to_vec(&("nando.plan-advance-family.v1", function_name)).ok()?,
        );
        let family_id = u64::from_be_bytes(digest[..8].try_into().unwrap_or([0; 8]));
        return Some((family_id, teacher_signature));
    }
    let hypotheses = ground_roles(frame);
    let hypothesis = hypotheses
        .first()
        .filter(|_| hypotheses.len() == 1 && hypotheses[0].competing_binding_count == 0)?;
    let family_id = grounded_program_family_id(frame, hypothesis)?;
    Some((family_id, teacher_signature))
}

fn restored_miner_family_ids(
    miner: &OnlineResponseMiner,
    frame: &RelationFrame,
    teacher_signature: &str,
    atom_ids: &[u64],
) -> Vec<u64> {
    let action = teacher_action_symbol(frame);
    let mut family_ids = vec![stable_restored_family_id(
        "broad_action",
        &action,
        teacher_signature,
        &[],
    )];
    if let Some((structural_family_id, _)) = online_bucket_identity(frame) {
        family_ids.push(structural_family_id);
    }

    let observed = atom_ids.iter().copied().collect::<BTreeSet<_>>();
    for subcenter in miner.subcenters.clean_subcenters(&action, 64, 8) {
        if subcenter
            .atom_ids
            .iter()
            .all(|atom_id| observed.contains(atom_id))
        {
            family_ids.push(stable_restored_family_id(
                "learned_subcenter",
                &action,
                teacher_signature,
                &subcenter.atom_ids,
            ));
        }
    }
    family_ids
}

fn stable_restored_family_id(
    kind: &str,
    action: &str,
    teacher_signature: &str,
    atom_ids: &[u64],
) -> u64 {
    let digest = Sha256::digest(
        serde_json::to_vec(&(
            "nando.restored-online-miner-family.v1",
            kind,
            action,
            teacher_signature,
            atom_ids,
        ))
        .unwrap_or_default(),
    );
    u64::from_be_bytes(digest[..8].try_into().unwrap_or([0; 8]))
}

fn stable_bucket_id(family_id: u64, signature: &str) -> u32 {
    let digest = Sha256::digest(format!("{family_id}:{signature}").as_bytes());
    u32::from_be_bytes(digest[..4].try_into().unwrap_or([0; 4]))
}

fn teacher_action_symbol(frame: &RelationFrame) -> String {
    let mut custom_tool = None;
    let mut inner_tool = None;
    for atom in &frame.atoms {
        match atom {
            crate::RelationAtom::ActionFunction { value } => return format!("function:{value}"),
            crate::RelationAtom::ActionCustomTool { value } => custom_tool = Some(value.as_str()),
            crate::RelationAtom::ActionInnerTool { value } => inner_tool = Some(value.as_str()),
            crate::RelationAtom::ActionValueProjection { .. } => {
                return "value_projection".to_owned();
            }
            crate::RelationAtom::ActionStatusProjection { .. } => {
                return "status_projection".to_owned();
            }
            _ => {}
        }
    }
    match (custom_tool, inner_tool) {
        (Some(outer), Some(inner)) => format!("custom_tool:{outer}/{inner}"),
        (Some(outer), None) => format!("custom_tool:{outer}"),
        _ => "unknown".to_owned(),
    }
}

fn unix_now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AtomSource, AtomValueType, RELATION_FRAME_SCHEMA, RelationAtom, ResponseValueSelector,
        SOURCE_NEUTRAL_EXTRACTOR_VERSION,
    };

    fn frame(index: usize, action: &str, accepted: bool) -> RelationFrame {
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: format!("{index:064x}"),
            event_id_sha256: format!("{:064x}", index + 1),
            client_intent_id_sha256: "c".repeat(64),
            session_id_sha256: format!("{:064x}", index % 4),
            observed_at_unix_nanos: index as u64,
            estimated_input_tokens: 100,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(accepted),
            atoms: vec![
                RelationAtom::ToolKind {
                    value: "exec".to_owned(),
                },
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
                RelationAtom::TypedSlot {
                    slot_id: 1,
                    value_type: AtomValueType::Identifier,
                    source: AtomSource::Observation,
                    value_sha256: "a".repeat(64),
                },
                RelationAtom::UniqueSlot { slot_id: 1 },
                RelationAtom::ObservationSelector {
                    slot_id: 1,
                    selector: ResponseValueSelector::UniqueScalar {
                        value_type: AtomValueType::Identifier,
                    },
                },
                RelationAtom::TypedSlot {
                    slot_id: 2,
                    value_type: AtomValueType::Identifier,
                    source: AtomSource::Action,
                    value_sha256: "a".repeat(64),
                },
                RelationAtom::SlotEquality {
                    left_slot: 1,
                    right_slot: 2,
                },
                RelationAtom::ActionFunction {
                    value: action.to_owned(),
                },
                RelationAtom::ActionRoleArgument {
                    name: "session_id".to_owned(),
                    slot_id: 2,
                    value_type: None,
                },
            ],
            evidence_ref_sha256: format!("{:064x}", index + 10_000),
        }
    }

    fn plan_frame(index: usize) -> RelationFrame {
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: format!("{:064x}", index + 50_000),
            event_id_sha256: format!("{:064x}", index + 60_000),
            client_intent_id_sha256: format!("{:064x}", index + 70_000),
            session_id_sha256: format!("{:064x}", index % 4 + 80_000),
            observed_at_unix_nanos: index as u64,
            estimated_input_tokens: 500,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::ToolKind {
                    value: "exec_command".to_owned(),
                },
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
                RelationAtom::OutputStatus {
                    value: "success".to_owned(),
                },
                RelationAtom::PlanState {
                    step_count: 3,
                    completed_count: 0,
                    active_index: 0,
                },
                RelationAtom::ActionFunction {
                    value: "update_plan".to_owned(),
                },
                RelationAtom::ActionPlanAdvance,
            ],
            evidence_ref_sha256: format!("{:064x}", index + 90_000),
        }
    }

    fn plan_parity_case(index: usize) -> crate::RuntimeParityCase {
        // Replay support is valid only when the production actor can execute
        // the captured before-state and reproduce the completed teacher call.
        let plan = serde_json::json!([
            {"step": "inspect", "status": "in_progress"},
            {"step": "repair", "status": "pending"},
            {"step": "verify", "status": "pending"}
        ]);
        let provider_payload = serde_json::json!({
            "input": [
                {
                    "type": "function_call",
                    "name": "update_plan",
                    "arguments": {"plan": plan}
                },
                {
                    "type": "function_call_output",
                    "output": {"ok": true}
                }
            ]
        });
        let expected_response = serde_json::json!({
            "name": "update_plan",
            "arguments": {
                "plan": [
                    {"step": "inspect", "status": "completed"},
                    {"step": "repair", "status": "in_progress"},
                    {"step": "verify", "status": "pending"}
                ]
            }
        })
        .to_string();
        crate::RuntimeParityCase {
            evidence_ref_sha256: format!("{:064x}", index + 90_000),
            request_text: String::new(),
            provider_payload,
            expected_response,
        }
    }

    fn write_stdin_parity_case(index: usize, prefix: &str) -> crate::RuntimeParityCase {
        // Model the real continuation surface; an empty payload would create
        // a receipt that cannot prove runtime parity.
        crate::RuntimeParityCase {
            evidence_ref_sha256: String::new(),
            request_text: String::new(),
            provider_payload: serde_json::json!({
                "input": [{
                    "type": "function_call_output",
                    "output": format!("{prefix}handle-{index}")
                }]
            }),
            expected_response: serde_json::json!({
                "name": "write_stdin",
                "arguments": {"session_id": format!("handle-{index}")}
            })
            .to_string(),
        }
    }

    #[test]
    fn plan_teacher_frame_reaches_a_synthesizable_online_bucket() {
        let mut miner =
            OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("miner");
        miner
            .observe_frame(plan_frame(1))
            .expect("plan frame ingest");
        let report = miner.report();
        assert_eq!(report.rows_ambiguous, 0);
        assert_eq!(report.bucket_count, 2);
        assert_eq!(report.candidates.len(), 0);
        assert_eq!(report.buckets.len(), 2);
        assert!(
            report
                .buckets
                .iter()
                .all(|bucket| { bucket.synthesized_operation.as_deref() == Some("advance_plan") })
        );
    }

    #[test]
    fn restored_broad_teacher_bucket_accumulates_across_structural_surfaces() {
        let mut miner =
            OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("miner");
        for index in 1..=64 {
            let mut row = frame(index, "write_stdin", true);
            row.atoms.push(RelationAtom::ClientCapabilityAtom {
                atom_id: if index % 2 == 0 { 100 } else { 200 },
            });
            miner.observe_frame(row).expect("teacher row");
        }

        let signature =
            teacher_program_signature(&frame(1, "write_stdin", true)).expect("teacher signature");
        let broad_family =
            stable_restored_family_id("broad_action", "function:write_stdin", &signature, &[]);
        let broad_id = stable_bucket_id(broad_family, &signature);
        let broad = miner.buckets.get(&broad_id).expect("restored broad bucket");
        assert_eq!(broad.positive_rows, 64);
        assert_eq!(broad.positives.len(), 32);
        assert_eq!(broad.future_positives.len(), 32);
        let structural_surfaces = [100, 200].map(|atom_id| {
            let mut row = frame(1000 + atom_id as usize, "write_stdin", true);
            row.atoms
                .push(RelationAtom::ClientCapabilityAtom { atom_id });
            online_bucket_identity(&row)
                .map(|identity| identity.0)
                .expect("structural surface")
        });
        for structural_family in structural_surfaces {
            let structural_id = stable_bucket_id(structural_family, &signature);
            let structural = miner
                .buckets
                .get(&structural_id)
                .expect("structural bucket");
            assert_eq!(structural.positives.len(), 32);
            assert_eq!(structural.future_positives.len(), 0);
        }
    }

    #[test]
    fn restored_core_compiles_provisional_candidate_before_admission_gate() {
        let mut miner =
            OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("restored miner");
        for index in 1..=16 {
            miner
                .observe_frame(frame(index, "write_stdin", true))
                .expect("positive teacher row");
        }
        for index in 100..104 {
            let mut negative = frame(index, "exec_command", true);
            negative.atoms[0] = RelationAtom::ToolKind {
                value: "exec_command".to_owned(),
            };
            miner
                .observe_frame(negative)
                .expect("competing teacher row");
        }

        let report = miner.report();
        assert!(report.active_bucket_count >= 1);
        assert!(report.candidates.iter().any(|candidate| {
            candidate.teacher_signature_sha256
                == teacher_program_signature(&frame(1, "write_stdin", true))
                    .expect("teacher signature")
        }));
        assert_eq!(report.admission_ready_cohorts, 0);
        assert_eq!(report.emitted_candidate_cohorts, 0);
        assert!(
            report
                .buckets
                .iter()
                .filter(|bucket| bucket.teacher_action_symbol == "function:write_stdin")
                .all(|bucket| bucket.frozen_future_rows == 0)
        );
    }

    #[test]
    fn restored_core_emits_a_verifier_bound_admission_candidate() {
        let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig {
            min_bucket_events: 2,
            calibration_events: 2,
            reservoir_rows: 32,
            ..OnlineResponseMinerConfig::default()
        })
        .expect("miner");
        for index in 1..=64 {
            let mut source = frame(index, "write_stdin", true);
            source
                .atoms
                .retain(|atom| !matches!(atom, RelationAtom::ObservationSelector { .. }));
            source.atoms.push(RelationAtom::ObservationSelector {
                slot_id: 1,
                selector: ResponseValueSelector::ContentLinePrefix {
                    prefix: "Process running with session ID ".to_owned(),
                    value_type: AtomValueType::Identifier,
                },
            });
            source.atoms.push(RelationAtom::ClientCapabilityAtom {
                atom_id: if index % 2 == 0 { 100 } else { 200 },
            });
            let evidence_ref_sha256 = source.evidence_ref_sha256.clone();
            let mut transition = crate::teacher_transition_from_completed(&source, None)
                .expect("teacher transition");
            let provider_payload = serde_json::json!({
                "input": [{
                    "type": "function_call_output",
                    "output": format!("Process running with session ID handle-{index}")
                }]
            });
            let training_frame = transition.as_training_relation_frame();
            let program = crate::synthesize_response_operator(&[training_frame])
                .expect("typed program")
                .candidate
                .program;
            let execution = crate::execute_response(&program, "", &provider_payload);
            assert_eq!(execution.status, crate::ResponseExecutionStatus::Executed);
            transition.runtime_parity_case = Some(crate::RuntimeParityCase {
                evidence_ref_sha256,
                request_text: String::new(),
                provider_payload,
                expected_response: execution.response.expect("exact response"),
            });
            miner
                .observe_teacher_transition(transition)
                .expect("observe teacher transition");
        }
        for index in 100..104 {
            let mut negative = frame(index, "exec_command", true);
            negative.atoms[0] = RelationAtom::ToolKind {
                value: "exec_command".to_owned(),
            };
            miner.observe_frame(negative).expect("negative teacher row");
        }

        let evaluation = miner.restored_core_admission_evaluation();
        assert!(evaluation.ready_cohorts >= 1);
        assert_eq!(
            evaluation.ready_cohorts,
            evaluation
                .candidates
                .len()
                .saturating_add(evaluation.blockers.len())
        );
        let candidate = evaluation
            .candidates
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate.program.operation,
                    crate::ResponseOperation::FunctionCallFromRoles { .. }
                )
            })
            .unwrap_or_else(|| panic!("restored candidate blockers={:?}", evaluation.blockers));
        assert!(candidate.support.len() >= 32);
        assert!(candidate.future.len() >= 32);
        assert!(candidate.runtime_parity_cases.len() >= candidate.future.len());
        assert_eq!(miner.report().false_accepts, 0);
    }

    #[test]
    fn proven_subcenter_trains_real_negative_before_calibration() {
        let guard_atom = crate::package::stable_atom_id("test:clean-subcenter");
        let support = (1..=32)
            .map(|index| {
                let mut frame = frame(index, "write_stdin", true);
                frame.atoms.push(RelationAtom::ClientCapabilityAtom {
                    atom_id: guard_atom,
                });
                frame
            })
            .collect::<Vec<_>>();
        let future = (33..=64)
            .map(|index| {
                let mut frame = frame(index, "write_stdin", true);
                frame.atoms.push(RelationAtom::ClientCapabilityAtom {
                    atom_id: guard_atom,
                });
                frame
            })
            .collect::<Vec<_>>();
        let mut negative = frame(100, "exec_command", false);
        negative.atoms[0] = RelationAtom::ToolKind {
            value: "network".to_owned(),
        };
        let synthesized = synthesize_response_operator(&support).expect("program");
        let bucket = ResponseBucket {
            structural_family_id: 1,
            teacher_signature_sha256: "d".repeat(64),
            teacher_action_symbol: "function:write_stdin".to_owned(),
            positives: support.iter().cloned().collect(),
            negatives: VecDeque::from([negative.clone()]),
            future_positives: future.iter().cloned().collect(),
            future_negatives: VecDeque::new(),
            positive_rows: 64,
            negative_rows: 1,
            positive_tokens: 6_400,
            negative_tokens: 100,
            first_false_accept_frame_id: None,
            cardinality_guard_rejects: 0,
            positive_cardinality_bounds: BTreeMap::new(),
            positive_cardinality_signatures: BTreeSet::new(),
            support_watermark_event_time_unix_nanos: 32,
            late_or_missing_time_rows: 0,
            exact_guard_atom_ids: Vec::new(),
        };
        let candidate = build_subcenter_admission_candidate(
            OnlineResponseMinerConfig::default(),
            &bucket,
            &[guard_atom],
            support.clone(),
            future.clone(),
            vec![negative.clone()],
            Some(ProvenAdmissionProgram {
                program: &synthesized.candidate.program,
                phase_rank: synthesized.candidate.phase_rank,
                exact_checks: synthesized.candidate.exact_checks,
            }),
        )
        .expect("subcenter candidate");
        assert_eq!(candidate.required_routing_atom_ids, [guard_atom]);
        assert!(!candidate.wave_runtime_package.is_empty());
    }

    #[test]
    fn frozen_admission_guard_repair_adds_clean_atom_without_repartitioning_future() {
        let base_atom = crate::package::stable_atom_id("test:base-guard");
        let clean_atom = crate::package::stable_atom_id("test:clean-guard");
        let add_atoms = |mut frame: RelationFrame| {
            frame
                .atoms
                .push(RelationAtom::ClientCapabilityAtom { atom_id: base_atom });
            frame.atoms.push(RelationAtom::ClientCapabilityAtom {
                atom_id: clean_atom,
            });
            frame
        };
        let support = (1..=32)
            .map(|index| add_atoms(frame(index, "write_stdin", true)))
            .collect::<Vec<_>>();
        let future = (33..=64)
            .map(|index| add_atoms(frame(index, "write_stdin", true)))
            .collect::<Vec<_>>();
        let mut negative = support[0].clone();
        negative.frame_id_sha256 = format!("{:064x}", 100_000);
        negative.event_id_sha256 = format!("{:064x}", 100_001);
        negative.session_id_sha256 = format!("{:064x}", 100_002);
        negative.observed_at_unix_nanos = 100_000;
        negative.verifier_label = Some(false);
        negative.atoms.retain(|atom| {
            !matches!(atom, RelationAtom::ClientCapabilityAtom { atom_id } if *atom_id == clean_atom)
        });
        let program = synthesize_response_operator(&support)
            .expect("program")
            .candidate
            .program;

        let (required, repaired_support, repaired_future) =
            repair_frozen_admission_guard(&program, &[base_atom], &support, &future, &[negative])
                .expect("clean exact guard");
        let mut expected_required = vec![base_atom, clean_atom];
        expected_required.sort_unstable();
        assert_eq!(required, expected_required);
        assert_eq!(
            repaired_support
                .iter()
                .map(|frame| frame.frame_id_sha256.as_str())
                .collect::<Vec<_>>(),
            support
                .iter()
                .map(|frame| frame.frame_id_sha256.as_str())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            repaired_future
                .iter()
                .map(|frame| frame.frame_id_sha256.as_str())
                .collect::<Vec<_>>(),
            future
                .iter()
                .map(|frame| frame.frame_id_sha256.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn frozen_admission_guard_delegates_unseparable_negative_to_wave() {
        let base_atom = crate::package::stable_atom_id("test:base-guard");
        let add_atom = |mut frame: RelationFrame| {
            frame
                .atoms
                .push(RelationAtom::ClientCapabilityAtom { atom_id: base_atom });
            frame
        };
        let support = (1..=32)
            .map(|index| add_atom(frame(index, "write_stdin", true)))
            .collect::<Vec<_>>();
        let future = (33..=64)
            .map(|index| add_atom(frame(index, "write_stdin", true)))
            .collect::<Vec<_>>();
        let negative = add_atom(frame(100, "exec_command", false));
        let program = synthesize_response_operator(&support)
            .expect("program")
            .candidate
            .program;
        let (required, repaired_support, repaired_future) =
            repair_frozen_admission_guard(&program, &[base_atom], &support, &future, &[negative])
                .expect("anti-center delegation");
        assert_eq!(required, [base_atom]);
        assert_eq!(repaired_support.len(), 32);
        assert_eq!(repaired_future.len(), 32);
    }

    #[test]
    fn frozen_admission_guard_ignores_negative_where_actor_cannot_run() {
        let base_atom = crate::package::stable_atom_id("test:base-guard");
        let add_atom = |mut frame: RelationFrame| {
            frame
                .atoms
                .push(RelationAtom::ClientCapabilityAtom { atom_id: base_atom });
            frame
        };
        let support = (1..=32)
            .map(|index| add_atom(frame(index, "write_stdin", true)))
            .collect::<Vec<_>>();
        let future = (33..=64)
            .map(|index| add_atom(frame(index, "write_stdin", true)))
            .collect::<Vec<_>>();
        let program = synthesize_response_operator(&support)
            .expect("program")
            .candidate
            .program;
        let mut negative = add_atom(frame(100, "exec_command", false));
        negative.atoms.retain(|atom| {
            !matches!(
                atom,
                RelationAtom::ObservationSelector { .. } | RelationAtom::UniqueSlot { .. }
            )
        });

        let (required, repaired_support, repaired_future) =
            repair_frozen_admission_guard(&program, &[base_atom], &support, &future, &[negative])
                .expect("actor abstain makes routed negative harmless");
        assert_eq!(required, [base_atom]);
        assert_eq!(repaired_support.len(), 32);
        assert_eq!(repaired_future.len(), 32);
    }

    #[test]
    fn reconstructed_support_capability_matches_observed_future_family() {
        let capability = crate::package::stable_atom_id("client_capability:function:wait");
        let mut historical = frame(1, "wait", true);
        reconstruct_online_client_capability(&mut historical);
        assert!(historical.atoms.iter().any(|atom| {
            matches!(
                atom,
                RelationAtom::ReconstructedClientCapabilityAtom { atom_id }
                    if *atom_id == capability
            )
        }));

        let mut future = frame(2, "wait", true);
        future.atoms.push(RelationAtom::ClientCapabilityAtom {
            atom_id: capability,
        });
        reconstruct_online_client_capability(&mut future);
        assert!(!future.atoms.iter().any(|atom| {
            matches!(atom, RelationAtom::ReconstructedClientCapabilityAtom { .. })
        }));
        assert_eq!(
            online_bucket_identity(&historical).map(|value| value.0),
            online_bucket_identity(&future).map(|value| value.0)
        );
    }

    #[test]
    fn online_response_miner_learns_before_update_and_bounds_support() {
        let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig {
            min_bucket_events: 2,
            calibration_events: 2,
            reservoir_rows: 4,
            ..OnlineResponseMinerConfig::default()
        })
        .expect("miner");
        for index in 0..12 {
            miner
                .observe_frame(frame(index, "write_stdin", true))
                .expect("observe");
        }
        for index in 12..18 {
            miner
                .observe_frame(frame(index, "other_action", true))
                .expect("competing action");
        }
        let report = miner.report();
        assert_eq!(report.rows_seen, 18);
        assert_eq!(report.rows_learned, 18);
        assert!(report.competing_negative_updates > 0);
        assert_eq!(report.false_accepts, 0);
        assert!(
            miner
                .buckets
                .values()
                .all(|bucket| { bucket.positives.len() <= 4 && bucket.negatives.len() <= 4 })
        );
    }

    #[test]
    fn exact_support_guard_rejects_a_foreign_preaction_surface() {
        let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig {
            reservoir_rows: 4,
            ..OnlineResponseMinerConfig::default()
        })
        .expect("miner");
        for index in 0..4 {
            miner
                .train_frame(frame(index, "wait", true))
                .expect("support");
        }
        let bucket = miner.buckets.values().next().expect("bucket");
        let mut foreign = frame(10, "write_stdin", true);
        foreign.atoms[0] = RelationAtom::ToolKind {
            value: "exec_command".to_owned(),
        };
        assert!(!cardinality_guard_matches(bucket, &foreign));
    }

    #[test]
    fn grounded_program_family_merges_surface_shapes_without_merging_programs() {
        let mut first = frame(100, "write_stdin", true);
        first.atoms.insert(
            0,
            RelationAtom::ObservationCallShape {
                value: "surface_a".to_owned(),
            },
        );
        first.atoms.push(RelationAtom::ActionIntegerArgument {
            name: "max_output_tokens".to_owned(),
            value: 3_000,
        });
        let mut second = frame(101, "write_stdin", true);
        second.atoms.insert(
            0,
            RelationAtom::ObservationCallShape {
                value: "surface_b".to_owned(),
            },
        );
        second.atoms.push(RelationAtom::ActionIntegerArgument {
            name: "max_output_tokens".to_owned(),
            value: 12_000,
        });
        assert_ne!(
            crate::relation_frame_structural_family_id(&first),
            crate::relation_frame_structural_family_id(&second)
        );
        assert_eq!(
            online_bucket_identity(&first),
            online_bucket_identity(&second)
        );

        let mut miner =
            OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("miner");
        miner.train_frame(first).expect("first surface");
        miner.train_frame(second).expect("second surface");
        assert_eq!(miner.report().bucket_count, 2);

        let different_program = frame(102, "exec_command", true);
        assert_ne!(
            online_bucket_identity(
                miner
                    .buckets
                    .values()
                    .next()
                    .and_then(|bucket| bucket.positives.front())
                    .expect("support frame")
            ),
            online_bucket_identity(&different_program)
        );
    }

    #[test]
    fn online_response_miner_dedupes_metadata_enrichment_and_rejects_semantic_conflicts() {
        let mut miner =
            OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("miner");
        let original = frame(1, "write_stdin", true);
        miner.observe_frame(original.clone()).expect("first");
        miner
            .observe_frame(original.clone())
            .expect("duplicate no-op");
        assert_eq!(miner.report().rows_seen, 1);

        let mut enriched = original.clone();
        enriched.estimated_input_tokens = enriched.estimated_input_tokens.saturating_add(1);
        enriched.observed_at_unix_nanos = enriched.observed_at_unix_nanos.saturating_add(1);
        enriched.evidence_ref_sha256 = "new-receipt-for-the-same-transition".to_owned();
        enriched.client_intent_id_sha256 = "enriched-client-intent".to_owned();
        enriched.session_id_sha256 = "enriched-session-lineage".to_owned();
        enriched.extractor_version = "enriched-extractor-provenance".to_owned();
        miner
            .observe_frame(enriched)
            .expect("metadata and receipt enrichment is a duplicate");
        assert_eq!(miner.report().rows_seen, 1);

        let mut family_miner =
            OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("family miner");
        family_miner
            .observe_teacher_transition(
                crate::teacher_transition_from_completed(&original, None).expect("transition"),
            )
            .expect("first family transition");
        family_miner.seen_frame_sha256.clear();
        let mut new_receipt = original.clone();
        new_receipt.evidence_ref_sha256 = "new-family-receipt".to_owned();
        new_receipt.client_intent_id_sha256 = "new-family-intent".to_owned();
        new_receipt.session_id_sha256 = "new-family-session".to_owned();
        family_miner
            .observe_teacher_transition(
                crate::teacher_transition_from_completed(&new_receipt, None)
                    .expect("enriched transition"),
            )
            .expect("family receipt enrichment is a duplicate");
        assert_eq!(
            family_miner
                .report()
                .self_training_v2
                .discovery
                .duplicate_rows,
            1
        );

        let mut conflict = original;
        conflict
            .atoms
            .push(RelationAtom::RequestPhaseAtom { atom_id: 999 });
        assert_eq!(
            miner.observe_frame(conflict),
            Err("online_frame_id_content_conflict".to_owned())
        );
        assert_eq!(miner.report().rows_seen, 1);
    }

    #[test]
    fn historical_training_does_not_claim_future_accepts() {
        let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig {
            min_bucket_events: 2,
            calibration_events: 2,
            ..OnlineResponseMinerConfig::default()
        })
        .expect("miner");
        for index in 0..20 {
            miner
                .train_frame(frame(index, "write_stdin", index % 3 != 0))
                .expect("train");
        }
        let report = miner.report();
        assert_eq!(report.false_accepts, 0);
        assert_eq!(report.candidate_bucket_count, 0);
    }

    #[test]
    fn response_checkpoint_restores_wave_and_bounded_synthesis_state() {
        let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig {
            min_bucket_events: 2,
            calibration_events: 2,
            reservoir_rows: 4,
            ..OnlineResponseMinerConfig::default()
        })
        .expect("miner");
        for index in 0..18 {
            miner
                .observe_frame(frame(index, "write_stdin", index % 5 != 0))
                .expect("observe");
        }
        let before = miner.report();
        let checkpoint = miner.checkpoint(11, 22, 33, 18, 1).expect("checkpoint");
        let encoded = serde_json::to_vec(&checkpoint).expect("checkpoint encoding");
        let decoded = serde_json::from_slice(&encoded).expect("checkpoint decoding");
        let restored = OnlineResponseMiner::from_checkpoint(decoded).expect("restore");
        assert_eq!(restored.report(), before);
        assert!(
            restored
                .buckets
                .values()
                .all(|bucket| bucket.positives.len() <= 4 && bucket.negatives.len() <= 4)
        );
    }

    #[test]
    fn online_response_miner_freezes_support_before_collecting_future() {
        let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig {
            min_bucket_events: 2,
            calibration_events: 2,
            reservoir_rows: 32,
            ..OnlineResponseMinerConfig::default()
        })
        .expect("miner");
        for index in 0..64 {
            miner
                .replay_chronological_frame(frame(index, "write_stdin", true))
                .expect("chronological replay");
        }
        let signature =
            teacher_program_signature(&frame(0, "write_stdin", true)).expect("teacher signature");
        let broad_family =
            stable_restored_family_id("broad_action", "function:write_stdin", &signature, &[]);
        let bucket = miner
            .buckets
            .get(&stable_bucket_id(broad_family, &signature))
            .expect("broad bucket");
        let support_ids = bucket
            .positives
            .iter()
            .map(|frame| frame.frame_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let future_ids = bucket
            .future_positives
            .iter()
            .map(|frame| frame.frame_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(support_ids.len(), 32);
        assert_eq!(future_ids.len(), 32);
        assert!(support_ids.is_disjoint(&future_ids));
    }

    #[test]
    fn online_response_miner_rejects_late_or_missing_event_time_from_future() {
        let mut miner = OnlineResponseMiner::new(OnlineResponseMinerConfig {
            min_bucket_events: 2,
            calibration_events: 2,
            reservoir_rows: 2,
            ..OnlineResponseMinerConfig::default()
        })
        .expect("miner");
        for index in [10, 11] {
            miner
                .replay_chronological_frame(frame(index, "write_stdin", true))
                .expect("support");
        }
        miner
            .observe_frame(frame(5, "write_stdin", true))
            .expect("late event");
        let bucket = miner.buckets.values().next().expect("bucket");
        assert!(bucket.future_positives.is_empty());
        assert_eq!(bucket.late_or_missing_time_rows, 1);

        miner
            .observe_frame(frame(12, "write_stdin", true))
            .expect("future event");
        let bucket = miner.buckets.values().next().expect("bucket");
        assert_eq!(bucket.future_positives.len(), 1);
    }

    #[test]
    fn frozen_future_reservoir_retains_new_sessions() {
        let mut rows = VecDeque::new();
        for index in 0..32 {
            let mut row = frame(index, "write_stdin", true);
            row.session_id_sha256 = "a".repeat(64);
            push_session_diverse_future(&mut rows, row, 32);
        }
        for (index, session) in ['b', 'c'].into_iter().enumerate() {
            let mut row = frame(100 + index, "write_stdin", true);
            row.session_id_sha256 = session.to_string().repeat(64);
            push_session_diverse_future(&mut rows, row, 32);
        }

        assert_eq!(rows.len(), 32);
        assert_eq!(
            rows.iter()
                .map(|frame| frame.session_id_sha256.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            3
        );
    }

    #[test]
    fn response_stream_restarts_at_checkpoint_and_ingests_each_frame_once() {
        let root = std::env::temp_dir().join(format!(
            "nando-response-stream-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp dir");
        let config = OnlineResponseTailConfig {
            input_path: root.join("frames.jsonl"),
            report_path: root.join("report.json"),
            checkpoint_path: root.join("miner.checkpoint"),
            idle_sleep: Duration::from_millis(1),
        };
        {
            let mut audit = File::create(&config.input_path).expect("audit");
            for index in 0..10 {
                serde_json::to_writer(&mut audit, &frame(index, "write_stdin", true))
                    .expect("frame encoding");
                audit.write_all(b"\n").expect("newline");
            }
        }
        let mut stream = OnlineResponseStream::open(config.clone()).expect("initial stream");
        assert_eq!(stream.report().rows_seen, 10);
        assert_eq!(
            stream
                .ingest(frame(10, "write_stdin", true))
                .expect("first streamed frame")
                .rows_seen,
            11
        );
        stream.persist().expect("persist");
        drop(stream);

        let mut restored = OnlineResponseStream::open(config.clone()).expect("restored stream");
        assert_eq!(restored.report().rows_seen, 11);
        assert_eq!(
            restored
                .ingest(frame(11, "write_stdin", true))
                .expect("second streamed frame")
                .rows_seen,
            12
        );
        let line_count = BufReader::new(File::open(&config.input_path).expect("audit read"))
            .lines()
            .count();
        assert_eq!(line_count, 12);
        fs::remove_dir_all(root).expect("temp cleanup");
    }

    #[test]
    fn teacher_transition_is_idempotent_across_checkpoint_restart() {
        let root = std::env::temp_dir().join(format!(
            "nando-teacher-restart-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp dir");
        let config = OnlineResponseTailConfig {
            input_path: root.join("frames.jsonl"),
            report_path: root.join("report.json"),
            checkpoint_path: root.join("miner.checkpoint"),
            idle_sleep: Duration::from_millis(1),
        };
        File::create(&config.input_path).expect("empty audit");
        let transition =
            crate::teacher_transition_from_completed(&frame(1, "write_stdin", true), None)
                .expect("teacher transition");
        {
            let mut stream = OnlineResponseStream::open_streaming(config.clone()).expect("stream");
            stream
                .apply_teacher_transition(transition.clone())
                .expect("first transition");
            stream.persist_now().expect("checkpoint");
            assert_eq!(stream.report().rows_seen, 1);
            assert_eq!(stream.report().bucket_count, 2);
        }

        let mut restored =
            OnlineResponseStream::open_streaming(config.clone()).expect("restored stream");
        let shadow_before_duplicate = restored.report().live_scalar_shadow;
        restored
            .apply_teacher_transition(transition)
            .expect("duplicate transition after restart");
        assert_eq!(restored.report().rows_seen, 1);
        assert_eq!(restored.report().bucket_count, 2);
        assert_eq!(
            restored.report().live_scalar_shadow,
            shadow_before_duplicate
        );
        fs::remove_dir_all(root).expect("temp cleanup");
    }

    #[test]
    fn v43_teacher_pools_seed_wave_support_without_future_claims() {
        let mut miner =
            OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("online miner");
        for index in 0..40 {
            let mut transition =
                crate::teacher_transition_from_completed(&frame(index, "write_stdin", true), None)
                    .expect("teacher transition");
            transition.runtime_parity_case = Some(write_stdin_parity_case(
                index,
                "Process running with session ID ",
            ));
            miner
                .observe_teacher_transition(transition)
                .expect("observe teacher transition");
        }
        let mut checkpoint = miner.checkpoint(0, 0, 0, 0, 0).expect("checkpoint");
        checkpoint.bucket_strategy_version = 43;

        let restored = OnlineResponseMiner::from_checkpoint(checkpoint).expect("migrated miner");
        let report = restored.report();
        assert!(report.bucket_count > 0);
        assert!(
            report
                .buckets
                .iter()
                .all(|bucket| bucket.frozen_future_rows == 0)
        );
    }

    #[test]
    fn replay_parity_batch_builds_support_without_claiming_live_future() {
        let root = std::env::temp_dir().join(format!(
            "nando-replay-parity-batch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        let config = OnlineResponseTailConfig {
            input_path: root.join("frames.jsonl"),
            report_path: root.join("report.json"),
            checkpoint_path: root.join("miner.checkpoint"),
            idle_sleep: Duration::from_millis(1),
        };
        File::create(&config.input_path).expect("empty audit");
        let mut stream = OnlineResponseStream::open_streaming(config).expect("stream");
        let target_signature = crate::teacher_program_signature(&frame(0, "write_stdin", true))
            .expect("target signature");
        let target_signatures = BTreeSet::from([target_signature]);
        let cases = (0..40).map(|index| {
            (
                frame(index, "write_stdin", true),
                Some(write_stdin_parity_case(
                    index,
                    "Script running with cell ID ",
                )),
            )
        });
        stream
            .train_replay_cases_batch(cases)
            .expect("replay support import");
        for _ in 0..512 {
            let checks = stream.run_self_training_work_slice_for_signatures(&target_signatures);
            if checks == 0 && !stream.has_self_training_work_for_signatures(&target_signatures) {
                break;
            }
        }

        let report = stream.report().self_training_v2;
        assert_eq!(report.runtime_parity_cases_total, 0);
        assert_eq!(report.replay_support_parity_cases_total, 40);
        assert!(!report.generations.is_empty());
        assert!(report.generations.iter().all(|generation| {
            generation.support_rows == 32
                && generation.future_rows == 0
                && generation.runtime_parity_rows == 0
        }));
        fs::remove_dir_all(root).expect("temp cleanup");
    }

    #[test]
    fn v48_canonical_parity_migration_preserves_teacher_pools_and_parity() {
        let mut miner =
            OnlineResponseMiner::new(OnlineResponseMinerConfig::default()).expect("online miner");
        for index in 0..40 {
            let mut source_frame = frame(index, "write_stdin", true);
            source_frame
                .atoms
                .retain(|atom| !matches!(atom, RelationAtom::ObservationSelector { .. }));
            source_frame.atoms.push(RelationAtom::ObservationSelector {
                slot_id: 1,
                selector: ResponseValueSelector::ContentLinePrefix {
                    prefix: "Process running with session ID ".to_owned(),
                    value_type: AtomValueType::Identifier,
                },
            });
            source_frame.atoms.sort();
            let mut transition = crate::teacher_transition_from_completed(&source_frame, None)
                .expect("teacher transition");
            let provider_payload = serde_json::json!({
                "input": [{
                    "type": "function_call_output",
                    "output": format!("Process running with session ID handle-{index}")
                }]
            });
            let expected_response = serde_json::json!({
                "name": "write_stdin",
                "arguments": {"session_id": format!("handle-{index}")}
            })
            .to_string();
            transition.runtime_parity_case = Some(crate::RuntimeParityCase {
                evidence_ref_sha256: String::new(),
                request_text: String::new(),
                provider_payload,
                expected_response,
            });
            let enriched = transition.as_training_relation_frame();
            let synthesized = crate::synthesize_response_operator(&[enriched])
                .expect("typed synthesis")
                .candidate
                .program;
            let parity = transition.runtime_parity_case.as_ref().expect("parity");
            let execution = crate::execute_response(
                &synthesized,
                &parity.request_text,
                &parity.provider_payload,
            );
            assert_eq!(
                execution.response.as_deref(),
                Some(parity.expected_response.as_str())
            );
            miner
                .observe_teacher_transition(transition)
                .expect("observe teacher transition");
        }
        let accepted = miner
            .report()
            .self_training_v2
            .discovery
            .accepted_transitions;
        let mut checkpoint = miner.checkpoint(0, 0, 0, 0, 0).expect("checkpoint");
        let parity_before_report = checkpoint.self_training_v2.report(0);
        let parity_before = parity_before_report
            .runtime_parity_cases_total
            .saturating_add(parity_before_report.replay_support_parity_cases_total);
        assert_eq!(parity_before, 40);
        checkpoint.bucket_strategy_version = 48;

        let mut restored =
            OnlineResponseMiner::from_checkpoint(checkpoint).expect("migrated miner");
        assert_eq!(
            restored
                .report()
                .self_training_v2
                .discovery
                .accepted_transitions,
            accepted
        );
        assert!(restored.self_training_v2.has_pending_work());
        assert!(restored.report().self_training_v2.generations.is_empty());
        // A strategy migration demotes historical live parity to support-only
        // evidence so it cannot be reinterpreted as post-freeze future.
        let migrated_parity = restored.report().self_training_v2;
        assert_eq!(migrated_parity.runtime_parity_cases_total, 0);
        assert_eq!(
            migrated_parity
                .runtime_parity_cases_total
                .saturating_add(migrated_parity.replay_support_parity_cases_total),
            parity_before
        );
        for _ in 0..1_024 {
            if !restored.self_training_v2.has_pending_work() {
                break;
            }
            let _ = restored.self_training_v2.run_work_slice();
        }
        assert!(!restored.self_training_v2.has_pending_work());
        let migrated_report = restored.report().self_training_v2;
        assert_eq!(
            migrated_report
                .generations
                .iter()
                .map(|generation| generation.support_rows)
                .max(),
            Some(32),
            "parity_overlap={} accepted={} signature_match={} cegis={:?} semantic_blockers={:?} generations={:?}",
            migrated_report.parity_discovery_key_overlap,
            migrated_report.parity_accepted_frame_rows,
            migrated_report.parity_signature_match_rows,
            migrated_report.cegis,
            migrated_report.semantic_law_blockers,
            migrated_report.generations,
        );
    }

    #[test]
    fn response_stream_rejects_checkpoint_when_committed_prefix_changes() {
        let root = std::env::temp_dir().join(format!(
            "nando-response-prefix-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp dir");
        let config = OnlineResponseTailConfig {
            input_path: root.join("frames.jsonl"),
            report_path: root.join("report.json"),
            checkpoint_path: root.join("miner.checkpoint"),
            idle_sleep: Duration::from_millis(1),
        };
        let mut stream = OnlineResponseStream::open(config.clone()).expect("empty stream");
        stream
            .ingest(frame(1, "write_stdin", true))
            .expect("ingest");
        stream.persist().expect("checkpoint");
        drop(stream);

        let mut bytes = fs::read(&config.input_path).expect("ledger");
        let changed = bytes
            .iter_mut()
            .find(|byte| **byte == b'a')
            .expect("mutable payload byte");
        *changed = b'b';
        fs::write(&config.input_path, bytes).expect("corrupt committed prefix");

        assert!(matches!(
            OnlineResponseStream::open(config),
            Err(error) if error == "online_checkpoint_source_prefix_mismatch"
        ));
        fs::remove_dir_all(root).expect("temp cleanup");
    }

    #[test]
    fn response_stream_appends_canonical_frame_bytes() {
        let root = std::env::temp_dir().join(format!(
            "nando-response-canonical-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp dir");
        let config = OnlineResponseTailConfig {
            input_path: root.join("frames.jsonl"),
            report_path: root.join("report.json"),
            checkpoint_path: root.join("miner.checkpoint"),
            idle_sleep: Duration::from_millis(1),
        };
        let mut expected_frame = frame(2, "write_stdin", true);
        canonicalize_online_frame(&mut expected_frame);
        let mut expected = crate::canonical_json_bytes(&expected_frame).expect("canonical frame");
        expected.push(b'\n');

        let mut stream = OnlineResponseStream::open(config.clone()).expect("empty stream");
        stream.ingest(expected_frame).expect("ingest");
        assert_eq!(fs::read(&config.input_path).expect("ledger"), expected);

        fs::remove_dir_all(root).expect("temp cleanup");
    }

    #[test]
    fn historical_loader_rejects_conflicting_duplicate_without_future_claims() {
        let root = std::env::temp_dir().join(format!(
            "nando-response-history-conflict-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp dir");
        let config = OnlineResponseTailConfig {
            input_path: root.join("frames.jsonl"),
            report_path: root.join("report.json"),
            checkpoint_path: root.join("miner.checkpoint"),
            idle_sleep: Duration::from_millis(1),
        };
        let first = frame(9, "write_stdin", true);
        let mut conflict = first.clone();
        conflict
            .atoms
            .push(RelationAtom::RequestPhaseAtom { atom_id: 999 });
        let mut audit = File::create(&config.input_path).expect("audit");
        for value in [first, conflict] {
            serde_json::to_writer(&mut audit, &value).expect("frame encoding");
            audit.write_all(b"\n").expect("newline");
        }
        drop(audit);
        let stream = OnlineResponseStream::open(config).expect("historical rebuild");
        assert_eq!(stream.report().rows_seen, 1);
        assert_eq!(stream.report().false_accepts, 0);
        assert_eq!(stream.parse_errors, 1);
        fs::remove_dir_all(root).expect("temp cleanup");
    }

    #[test]
    fn replay_training_does_not_append_or_claim_future() {
        let root = std::env::temp_dir().join(format!(
            "nando-response-replay-training-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp dir");
        let config = OnlineResponseTailConfig {
            input_path: root.join("frames.jsonl"),
            report_path: root.join("report.json"),
            checkpoint_path: root.join("miner.checkpoint"),
            idle_sleep: Duration::from_millis(1),
        };
        let mut stream = OnlineResponseStream::open(config.clone()).expect("empty stream");
        stream
            .train_replay_batch((20..60).map(|index| frame(index, "write_stdin", true)))
            .expect("replay train");
        let report = stream.report();
        assert_eq!(report.rows_seen, 40);
        assert_eq!(stream.report().false_accepts, 0);
        assert!(report.self_training_v2.generations.is_empty());
        assert_eq!(
            report
                .self_training_v2
                .discovery
                .teacher_pools
                .iter()
                .map(|pool| pool.positive_rows)
                .sum::<u64>(),
            40
        );
        assert_eq!(fs::metadata(&config.input_path).expect("audit").len(), 0);
        fs::remove_dir_all(root).expect("temp cleanup");
    }

    #[test]
    fn replay_parity_receipts_enable_support_but_never_future() {
        let root = std::env::temp_dir().join(format!(
            "nando-response-replay-parity-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp dir");
        let config = OnlineResponseTailConfig {
            input_path: root.join("frames.jsonl"),
            report_path: root.join("report.json"),
            checkpoint_path: root.join("miner.checkpoint"),
            idle_sleep: Duration::from_millis(1),
        };
        let mut stream = OnlineResponseStream::open_streaming(config.clone()).expect("stream");
        stream
            .train_replay_cases_batch((0..40).map(|index| {
                let frame = plan_frame(index);
                let parity = plan_parity_case(index);
                (frame, Some(parity))
            }))
            .expect("replay parity train");
        for _ in 0..2_048 {
            if !stream.has_self_training_work() {
                break;
            }
            stream.run_self_training_work_slice();
        }
        assert!(!stream.has_self_training_work());
        stream.persist_now().expect("persist replay parity");
        let report = stream.report();
        assert!(
            report
                .self_training_v2
                .generations
                .iter()
                .any(|generation| generation.support_rows == 32)
        );
        assert!(
            report
                .self_training_v2
                .generations
                .iter()
                .all(|generation| generation.future_rows == 0)
        );
        drop(stream);

        let restored = OnlineResponseStream::open_streaming(config).expect("restored stream");
        assert!(
            restored
                .report()
                .self_training_v2
                .generations
                .iter()
                .all(|generation| generation.future_rows == 0)
        );
        fs::remove_dir_all(root).expect("temp cleanup");
    }

    #[test]
    fn restored_future_reservoir_is_compacted_to_authority_bound() {
        let mut rows = (0..128)
            .map(|index| frame(index, "wait", true))
            .collect::<VecDeque<_>>();

        trim_session_diverse_future(&mut rows, MAX_FROZEN_FUTURE_ROWS_PER_BUCKET);

        assert_eq!(rows.len(), MAX_FROZEN_FUTURE_ROWS_PER_BUCKET);
        assert_eq!(
            rows.iter()
                .map(|row| row.session_id_sha256.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            4
        );
    }
}
