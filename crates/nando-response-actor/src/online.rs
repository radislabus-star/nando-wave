use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::ops::Deref;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nando_core::wave::{
    PhaseCenterAtomEncoder, PhaseCenterOnlineMiner, PhaseCenterOnlineMinerConfig,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::teacher_join::action_schema_enriched_frame;
use crate::{
    ECONOMICS_RECEIPT_SCHEMA_V1, EconomicsReceipt, RelationAtom, RelationFrame, ResponseProgram,
    SelfTrainingStateReport, StreamingSelfTrainingState, VerifierProgram, ground_roles,
    is_source_neutral_relation_frame, relation_atom_is_teacher_only,
    relation_frame_online_routing_atom_ids, synthesis::grounded_program_family_id,
    synthesize_response_operator, teacher_program_signature, teacher_transition_from_completed,
};

use crate::online_subcenter::OnlineSubcenterDiscovery;

mod admission;
mod evidence;
mod stream;

use admission::{
    ProvenAdmissionProgram, SelfTrainingAdmissionEvaluation, build_subcenter_admission_candidate,
    clean_admission_partition_for_ids, online_admission_precheck, repair_frozen_admission_guard,
};
#[cfg(test)]
use evidence::reconstruct_online_client_capability;
use evidence::{
    canonical_runtime_parity_key, canonicalize_online_frame, cardinality_guard_matches,
    intern_bucket_evidence, push_bounded, push_session_diverse_future, recompute_exact_guard,
    response_operation_name, trim_session_diverse_future, update_cardinality_bounds,
    update_exact_guard,
};
pub use stream::run_online_response_tail;

const ONLINE_CHECKPOINT_MAGIC_V3: &[u8; 4] = b"NRO3";
// Version 72 replays only bounded teacher reservoirs so typed call actors and
// request-independent rows receive the same source-neutral extraction as new
// live events.
// Historical rows remain support-only; frozen future is never reconstructed.
const ONLINE_BUCKET_STRATEGY_VERSION: u8 = 96;
const RESTORED_CORE_MIN_BUCKET_EVENTS: usize = 20;
const MAX_PINNED_FUTURE_PARITY_CASES: usize = 4_096;
// Admission needs 32 independent future rows; larger full-frame reservoirs only
// duplicate cold evidence without increasing execution authority.
const MAX_FROZEN_FUTURE_ROWS_PER_BUCKET: usize = 32;
const ONLINE_ROUTING_ATOM_CACHE_ENTRIES: usize = 512;

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
    #[serde(default)]
    pub semantic_evidence_receipts: Vec<crate::SemanticEvidenceReceipt>,
    #[serde(default)]
    pub semantic_evidence_root_sha256: String,
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
    positives: VecDeque<SharedRelationFrame>,
    negatives: VecDeque<SharedRelationFrame>,
    #[serde(default)]
    future_positives: VecDeque<SharedRelationFrame>,
    #[serde(default)]
    future_negatives: VecDeque<SharedRelationFrame>,
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

/// Shares immutable evidence between competing bucket reservoirs while
/// preserving the existing checkpoint representation of a `RelationFrame`.
#[derive(Clone, Debug)]
struct SharedRelationFrame(Arc<RelationFrame>);

impl SharedRelationFrame {
    fn new(frame: RelationFrame) -> Self {
        Self(Arc::new(frame))
    }

    fn materialize(&self) -> RelationFrame {
        self.0.as_ref().clone()
    }

    fn as_frame(&self) -> &RelationFrame {
        self.0.as_ref()
    }
}

impl Deref for SharedRelationFrame {
    type Target = RelationFrame;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl Serialize for SharedRelationFrame {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SharedRelationFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        RelationFrame::deserialize(deserializer).map(Self::new)
    }
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
    // Runtime-only cache: checkpoints retain evidence, never derived scratch.
    routing_atom_cache: BTreeMap<[u8; 32], Vec<u64>>,
    routing_atom_cache_order: VecDeque<[u8; 32]>,
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
            routing_atom_cache: BTreeMap::new(),
            routing_atom_cache_order: VecDeque::new(),
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
                let preserved_shadow_support = checkpoint
                    .live_scalar_shadow
                    .historical_support_transitions();
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
                } else if checkpoint.bucket_strategy_version < 96 {
                    // Strategies 86-96 scope layouts to the observation,
                    // normalize equality orientation, and recover typed roles
                    // from receipt-backed legacy relations. Immutable evidence
                    // remains byte-identical while derived programs rebuild.
                    preserved_self_training.prepare_derived_program_migration();
                }
                let support_frames =
                    preserved_self_training.bounded_teacher_frames_for_wave_migration();
                let parity_cases = preserved_self_training
                    .runtime_parity_cases_for_frames(support_frames.iter())
                    .into_iter()
                    .map(|case| (case.evidence_ref_sha256.clone(), case))
                    .collect::<BTreeMap<_, _>>();
                let mut shadow_support = preserved_shadow_support
                    .into_iter()
                    .map(|transition| (transition.before.frame_id_sha256.clone(), transition))
                    .collect::<BTreeMap<_, _>>();
                for frame in &support_frames {
                    if let Some(parity_case) = parity_cases.get(&frame.frame_id_sha256)
                        && let Ok(mut transition) = teacher_transition_from_completed(frame, None)
                    {
                        transition.runtime_parity_case = Some(parity_case.clone());
                        shadow_support
                            .entry(transition.before.frame_id_sha256.clone())
                            .or_insert(transition);
                    }
                }
                let mut shadow_support = shadow_support.into_values().collect::<Vec<_>>();
                shadow_support.sort_by(|left, right| {
                    left.before
                        .observed_at_unix_nanos
                        .cmp(&right.before.observed_at_unix_nanos)
                        .then_with(|| {
                            left.before
                                .frame_id_sha256
                                .cmp(&right.before.frame_id_sha256)
                        })
                });
                for transition in shadow_support {
                    migrated
                        .live_scalar_shadow
                        .observe_historical_support(&transition);
                }
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
                        .map(SharedRelationFrame::materialize)
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
        intern_bucket_evidence(&mut checkpoint.buckets)?;
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
            routing_atom_cache: BTreeMap::new(),
            routing_atom_cache_order: VecDeque::new(),
        })
    }

    fn cached_online_routing_atom_ids(
        &mut self,
        frame: &RelationFrame,
    ) -> Result<Vec<u64>, String> {
        // Hash the complete pre-action structure. The fixed-size key keeps the
        // scratch cache bounded without persisting payload-bearing atoms.
        let pre_action_atoms = frame
            .atoms
            .iter()
            .filter(|atom| !relation_atom_is_teacher_only(atom))
            .collect::<Vec<_>>();
        let key: [u8; 32] = Sha256::digest(
            serde_json::to_vec(&pre_action_atoms)
                .map_err(|error| format!("online_routing_cache_key:{error}"))?,
        )
        .into();
        if let Some(ids) = self.routing_atom_cache.get(&key) {
            return Ok(ids.clone());
        }

        let ids = relation_frame_online_routing_atom_ids(frame);
        if self.routing_atom_cache.len() == ONLINE_ROUTING_ATOM_CACHE_ENTRIES
            && let Some(oldest) = self.routing_atom_cache_order.pop_front()
        {
            self.routing_atom_cache.remove(&oldest);
        }
        self.routing_atom_cache_order.push_back(key);
        self.routing_atom_cache.insert(key, ids.clone());
        Ok(ids)
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
        let atom_ids = self.cached_online_routing_atom_ids(&frame)?;
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
        // Every competing bucket sees the same pre-action event. Encoding it
        // once avoids repeating the trigonometric phase projection per bucket.
        let event_phase_vector = self
            .encoder
            .encode_atom_ids(atom_ids.iter().copied())
            .map_err(|error| format!("online_response_encode:{error:?}"))?
            .to_vec();

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

        // One immutable event can update hundreds of competing buckets. Keep
        // at most one positive and one teacher-negative payload allocation.
        let shared_positive = SharedRelationFrame::new(frame.clone());
        let mut negative_frame = frame.clone();
        negative_frame.verifier_label = Some(false);
        let shared_negative = SharedRelationFrame::new(negative_frame);
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
                        .observe(
                            bucket_id,
                            &event_phase_vector,
                            safe_for_bucket,
                            false,
                            frame.estimated_input_tokens,
                            0,
                        )
                        .map_err(|error| format!("online_response_observe:{error:?}"))?,
                )
            } else {
                self.wave
                    .train(bucket_id, &event_phase_vector, safe_for_bucket)
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
                    bucket.positives.push_back(shared_positive.clone());
                    bucket.support_watermark_event_time_unix_nanos = bucket
                        .support_watermark_event_time_unix_nanos
                        .max(frame.observed_at_unix_nanos);
                } else if score_for_bucket {
                    push_session_diverse_future(
                        &mut bucket.future_positives,
                        shared_positive.clone(),
                        MAX_FROZEN_FUTURE_ROWS_PER_BUCKET,
                    );
                    retained_as_frozen_future = true;
                }
                update_cardinality_bounds(bucket, &frame);
            } else {
                let bucket = self.buckets.get_mut(&bucket_id).expect("bucket exists");
                bucket.negative_rows = bucket.negative_rows.saturating_add(1);
                bucket.negative_tokens = bucket
                    .negative_tokens
                    .saturating_add(frame.estimated_input_tokens);
                if score_for_bucket {
                    push_bounded(&mut bucket.future_negatives, shared_negative.clone(), 8);
                } else {
                    push_bounded(
                        &mut bucket.negatives,
                        shared_negative.clone(),
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
            if self.rows_seen.is_multiple_of(64) {
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
            .map(|frame| canonical_runtime_parity_key(frame.as_frame()))
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
            let support = bucket
                .positives
                .iter()
                .map(SharedRelationFrame::materialize)
                .collect::<Vec<_>>();
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
                .filter_map(|frame| crate::relation_frame_structural_family_id(frame.as_frame()))
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
                .map(SharedRelationFrame::materialize)
                .collect::<Vec<_>>();
            let all_frames = bucket
                .positives
                .iter()
                .chain(bucket.future_positives.iter())
                .map(SharedRelationFrame::as_frame)
                .collect::<Vec<_>>();
            let mut parity_cases_by_frame = self
                .self_training_v2
                .runtime_parity_cases_for_frames(all_frames.iter().copied())
                .into_iter()
                .map(|case| (case.evidence_ref_sha256.clone(), case))
                .collect::<BTreeMap<_, _>>();
            for frame in &all_frames {
                let parity_key = canonical_runtime_parity_key(frame);
                if let Some(mut parity_case) =
                    self.future_runtime_parity_cases.get(&parity_key).cloned()
                {
                    parity_case.evidence_ref_sha256 = frame.frame_id_sha256.clone();
                    parity_cases_by_frame
                        .entry(frame.frame_id_sha256.clone())
                        .or_insert(parity_case);
                }
            }
            let parity_eligible_ids = parity_cases_by_frame
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>();
            let (support, future, partition_required_atom_ids) =
                clean_admission_partition_for_ids(bucket, &negatives, Some(&parity_eligible_ids));
            if support.len() < 32 || future.len() < 32 {
                blockers.push(OnlineResponseAdmissionBlockerReport {
                    cohort_id_sha256,
                    blocker: format!(
                        "receipt_backed_partition_below_32:support={}:future={}",
                        support.len(),
                        future.len()
                    ),
                });
                continue;
            }
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
            let mut exact_guard_atom_ids = bucket.exact_guard_atom_ids.clone();
            exact_guard_atom_ids.extend(partition_required_atom_ids);
            exact_guard_atom_ids.sort_unstable();
            exact_guard_atom_ids.dedup();
            let (required_atom_ids, support, future) = match repair_frozen_admission_guard(
                &synthesized.candidate.program,
                &exact_guard_atom_ids,
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
            let mut receipt_backed_support = Vec::new();
            let mut runtime_parity_cases = Vec::new();
            for frame in support {
                if let Some(mut parity_case) = parity_cases_by_frame.remove(&frame.frame_id_sha256)
                {
                    parity_case.evidence_ref_sha256 = frame.frame_id_sha256.clone();
                    receipt_backed_support.push(frame);
                    runtime_parity_cases.push(parity_case);
                }
            }
            if receipt_backed_support.len() < 32 {
                blockers.push(OnlineResponseAdmissionBlockerReport {
                    cohort_id_sha256,
                    blocker: format!(
                        "receipt_backed_support_rows_below_32:{}/32",
                        receipt_backed_support.len()
                    ),
                });
                continue;
            }
            let mut seen_future_events = BTreeSet::new();
            let mut receipt_backed_future = Vec::new();
            for frame in future {
                let parity_key = canonical_runtime_parity_key(&frame);
                if !seen_future_events.insert(parity_key.clone()) {
                    continue;
                }
                let parity_case = parity_cases_by_frame.remove(&frame.frame_id_sha256);
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
                receipt_backed_support,
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
            let hard_contradiction_reasons = cohort
                .semantic_evidence_receipts
                .iter()
                .filter(|receipt| {
                    receipt.outcome == crate::SemanticEvidenceOutcome::HardContradiction
                })
                .fold(BTreeMap::<&str, usize>::new(), |mut reasons, receipt| {
                    *reasons.entry(receipt.reason.as_str()).or_default() += 1;
                    reasons
                });
            let hard_contradictions = hard_contradiction_reasons.values().sum::<usize>();
            if hard_contradictions > 0 {
                let reasons = hard_contradiction_reasons
                    .into_iter()
                    .map(|(reason, count)| format!("{reason}={count}"))
                    .collect::<Vec<_>>()
                    .join(",");
                blockers.push(OnlineResponseAdmissionBlockerReport {
                    cohort_id_sha256,
                    blocker: format!(
                        "semantic_hard_contradictions_present:{hard_contradictions}:{reasons}"
                    ),
                });
                continue;
            }
            let evidence_by_frame = cohort
                .semantic_evidence_receipts
                .iter()
                .map(|receipt| (receipt.frame_id_sha256.as_str(), receipt))
                .collect::<BTreeMap<_, _>>();
            let admission_negatives = cohort
                .pool
                .negatives
                .iter()
                .filter(|frame| {
                    frame.observed_at_unix_nanos > cohort.winner.repair_watermark_unix_nanos
                        && evidence_by_frame
                            .get(frame.frame_id_sha256.as_str())
                            .is_some_and(|receipt| {
                                receipt.outcome
                                    == crate::SemanticEvidenceOutcome::ApplicabilityNegative
                            })
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
                positives: support
                    .iter()
                    .cloned()
                    .map(SharedRelationFrame::new)
                    .collect(),
                negatives: admission_negatives
                    .iter()
                    .cloned()
                    .map(SharedRelationFrame::new)
                    .collect(),
                future_positives: future
                    .iter()
                    .cloned()
                    .map(SharedRelationFrame::new)
                    .collect(),
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
                candidate
                    .support
                    .iter()
                    .chain(&candidate.future)
                    .chain(&candidate.negatives),
            );
            candidate.semantic_alias_edges = cohort.semantic_alias_edges;
            let candidate_frame_ids = candidate
                .support
                .iter()
                .chain(&candidate.future)
                .chain(&candidate.negatives)
                .map(|frame| frame.frame_id_sha256.as_str())
                .collect::<BTreeSet<_>>();
            candidate.semantic_evidence_receipts = cohort
                .semantic_evidence_receipts
                .into_iter()
                .filter(|receipt| candidate_frame_ids.contains(receipt.frame_id_sha256.as_str()))
                .collect();
            if candidate.semantic_evidence_receipts.len() != candidate_frame_ids.len() {
                blockers.push(OnlineResponseAdmissionBlockerReport {
                    cohort_id_sha256,
                    blocker: "semantic_evidence_receipt_coverage_incomplete".to_owned(),
                });
                continue;
            }
            candidate.semantic_evidence_root_sha256 = format!(
                "{:x}",
                Sha256::digest(
                    serde_json::to_vec(&(
                        "nando.semantic-evidence-set.v1",
                        &candidate.semantic_evidence_receipts,
                    ))
                    .unwrap_or_default()
                )
            );
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
#[path = "online_tests.rs"]
mod tests;
