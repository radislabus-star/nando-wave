use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256};

pub const EVIDENCE_LEDGER_SCHEMA_V1: &str = "nando.deterministic-evidence-ledger.v1";
pub const CANONICAL_EVENT_GRAPH_SCHEMA_V1: &str = "nando.canonical-event-graph.v1";
pub const EVIDENCE_POLICY_VERSION: u32 = 3;
const EVIDENCE_LEDGER_CHECKPOINT_SCHEMA_V1: &str = "nando.evidence-ledger-checkpoint.v1";
const EVIDENCE_LEDGER_CHECKPOINT_SCHEMA_V2: &str = "nando.evidence-ledger-checkpoint.v2";
const PACKED_EVIDENCE_INDEX_ENTRY_BYTES: usize = 108;
const CHECKPOINT_EVENT_INTERVAL: u64 = 8_192;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidencePolicyV1 {
    pub max_event_bytes: usize,
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_string_bytes: usize,
}

impl Default for EvidencePolicyV1 {
    fn default() -> Self {
        Self {
            max_event_bytes: 1_048_576,
            max_depth: 64,
            max_nodes: 65_536,
            max_string_bytes: 262_144,
        }
    }
}

impl EvidencePolicyV1 {
    #[must_use]
    pub const fn streaming_bounded() -> Self {
        Self {
            max_event_bytes: 1_048_576,
            max_depth: 32,
            max_nodes: 4_096,
            max_string_bytes: 65_536,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawEvidenceEnvelope {
    pub source_stream_id: String,
    pub source_offset: u64,
    pub event_id: String,
    pub session_id: String,
    pub client_intent_id: Option<String>,
    pub call_id: Option<String>,
    pub output_ordinal: Option<u32>,
    pub event_time_unix_nanos: Option<u64>,
    pub schema_version: u32,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvidenceEventTime {
    Known { unix_nanos: u64 },
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanonicalEventGraph {
    pub schema: String,
    pub policy_version: u32,
    pub schema_version: u32,
    pub source_stream_sha256: String,
    pub source_offset: u64,
    pub event_id_sha256: String,
    pub session_id_sha256: String,
    pub client_intent_id_sha256: Option<String>,
    pub call_id_sha256: Option<String>,
    pub output_ordinal: Option<u32>,
    pub event_time: EvidenceEventTime,
    pub payload_sha256: String,
    pub nodes: Vec<CanonicalEventNode>,
    pub graph_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CanonicalEventNode {
    Null {
        path: String,
    },
    Boolean {
        path: String,
        value: bool,
    },
    Number {
        path: String,
        number_class: CanonicalNumberClass,
        value_sha256: String,
    },
    String {
        path: String,
        byte_len: usize,
        value_sha256: String,
    },
    ParsedJson {
        path: String,
        source_path: String,
    },
    Array {
        path: String,
        len: usize,
    },
    Object {
        path: String,
        len: usize,
    },
    ObjectField {
        path: String,
        ordinal: usize,
        name_sha256: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalNumberClass {
    SignedInteger,
    UnsignedInteger,
    FiniteFloat,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRejection {
    EmptyIdentity,
    PayloadTooLarge,
    InvalidJson,
    DuplicateObjectKey,
    FloatingPointUnsupported,
    GraphDepthExceeded,
    GraphNodeBudgetExceeded,
    StringBudgetExceeded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvidenceIngestOutcome {
    Normalized {
        graph: CanonicalEventGraph,
    },
    Rejected {
        key: EvidenceKey,
        payload_sha256: String,
        reason: EvidenceRejection,
    },
    DuplicateIdempotent {
        key: EvidenceKey,
        payload_sha256: String,
    },
    DuplicateConflict {
        key: EvidenceKey,
        original_payload_sha256: String,
        conflicting_payload_sha256: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EvidenceKey {
    #[serde(
        default = "legacy_evidence_policy_version",
        skip_serializing_if = "is_legacy_evidence_policy_version"
    )]
    pub policy_version: u32,
    pub source_stream_sha256: String,
    pub source_offset: u64,
    pub event_id_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceLedgerRecord {
    pub schema: String,
    pub sequence: u64,
    pub previous_record_sha256: String,
    pub outcome: EvidenceIngestOutcome,
    pub record_sha256: String,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceAccounting {
    pub ingress_total: u64,
    pub normalized_total: u64,
    pub rejected_total: u64,
    pub duplicate_idempotent_total: u64,
    pub duplicate_conflict_total: u64,
}

impl EvidenceAccounting {
    #[must_use]
    pub fn identity_holds(self) -> bool {
        self.ingress_total
            == self
                .normalized_total
                .saturating_add(self.rejected_total)
                .saturating_add(self.duplicate_idempotent_total)
                .saturating_add(self.duplicate_conflict_total)
    }

    fn observe(&mut self, outcome: &EvidenceIngestOutcome) {
        self.ingress_total = self.ingress_total.saturating_add(1);
        match outcome {
            EvidenceIngestOutcome::Normalized { .. } => {
                self.normalized_total = self.normalized_total.saturating_add(1);
            }
            EvidenceIngestOutcome::Rejected { .. } => {
                self.rejected_total = self.rejected_total.saturating_add(1);
            }
            EvidenceIngestOutcome::DuplicateIdempotent { .. } => {
                self.duplicate_idempotent_total = self.duplicate_idempotent_total.saturating_add(1);
            }
            EvidenceIngestOutcome::DuplicateConflict { .. } => {
                self.duplicate_conflict_total = self.duplicate_conflict_total.saturating_add(1);
            }
        }
    }
}

pub struct DeterministicEvidenceLedger {
    path: PathBuf,
    checkpoint_path: PathBuf,
    checkpoint_index_path: PathBuf,
    policy: EvidencePolicyV1,
    next_sequence: u64,
    previous_record_sha256: String,
    seen: EvidenceIndex,
    accounting: EvidenceAccounting,
    recovered_partial_tail_bytes: u64,
    ledger_bytes: u64,
    ledger_prefix_hasher: Sha256,
    events_since_checkpoint: u64,
}

const MAX_ACTIVE_EVIDENCE_LEDGER_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PackedEvidenceKey {
    policy_version: u32,
    source_stream_sha256: [u8; 32],
    source_offset: u64,
    event_id_sha256: [u8; 32],
}

struct EvidenceIndex {
    base_path: PathBuf,
    base_count: u64,
    base_sha256: String,
    base_file: Option<File>,
    source_max_offsets: BTreeMap<[u8; 32], u64>,
    delta: BTreeMap<PackedEvidenceKey, [u8; 32]>,
}

impl EvidenceIndex {
    fn empty(base_path: PathBuf) -> Self {
        Self {
            base_path,
            base_count: 0,
            base_sha256: format!("{:x}", Sha256::digest([])),
            base_file: None,
            source_max_offsets: BTreeMap::new(),
            delta: BTreeMap::new(),
        }
    }

    fn get(&mut self, key: &PackedEvidenceKey) -> Result<Option<[u8; 32]>, String> {
        if let Some(payload) = self.delta.get(key) {
            return Ok(Some(*payload));
        }
        let mut lower = 0_u64;
        let mut upper = self.base_count;
        if self.base_file.is_none() {
            self.base_file = Some(
                File::open(&self.base_path)
                    .map_err(|error| format!("evidence_index_lookup_open:{error}"))?,
            );
        }
        let file = self
            .base_file
            .as_mut()
            .ok_or_else(|| "evidence_index_lookup_file_missing".to_owned())?;
        while lower < upper {
            let middle = lower + (upper - lower) / 2;
            let (candidate, payload) = read_packed_evidence_entry(file, middle)?;
            match candidate.cmp(key) {
                std::cmp::Ordering::Less => lower = middle.saturating_add(1),
                std::cmp::Ordering::Greater => upper = middle,
                std::cmp::Ordering::Equal => return Ok(Some(payload)),
            }
        }
        Ok(None)
    }

    fn contains_key(&mut self, key: &PackedEvidenceKey) -> Result<bool, String> {
        self.get(key).map(|value| value.is_some())
    }

    fn insert(
        &mut self,
        key: PackedEvidenceKey,
        payload: [u8; 32],
    ) -> Result<Option<[u8; 32]>, String> {
        if let Some(previous) = self.get(&key)? {
            return Ok(Some(previous));
        }
        self.source_max_offsets
            .entry(key.source_stream_sha256)
            .and_modify(|offset| *offset = (*offset).max(key.source_offset))
            .or_insert(key.source_offset);
        self.delta.insert(key, payload);
        Ok(None)
    }

    fn len(&self) -> usize {
        usize::try_from(self.base_count)
            .unwrap_or(usize::MAX)
            .saturating_add(self.delta.len())
    }

    fn max_offset_for_source(&self, source_stream_sha256: &[u8; 32]) -> Option<u64> {
        self.source_max_offsets.get(source_stream_sha256).copied()
    }

    fn replace_base(&mut self, count: u64, sha256: String) {
        self.base_count = count;
        self.base_sha256 = sha256;
        self.base_file = File::open(&self.base_path).ok();
        self.delta.clear();
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EvidenceLedgerCheckpointV1 {
    schema: String,
    ledger_bytes: u64,
    ledger_prefix_sha256: String,
    policy: EvidencePolicyV1,
    next_sequence: u64,
    previous_record_sha256: String,
    seen: Vec<(EvidenceKey, String)>,
    accounting: EvidenceAccounting,
    checkpoint_sha256: String,
}

#[derive(Serialize)]
struct EvidenceLedgerCheckpointDigestMaterialV1<'a> {
    schema: &'static str,
    ledger_bytes: u64,
    ledger_prefix_sha256: &'a str,
    policy: EvidencePolicyV1,
    next_sequence: u64,
    previous_record_sha256: &'a str,
    seen: &'a [(EvidenceKey, String)],
    accounting: EvidenceAccounting,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EvidenceLedgerCheckpointV2 {
    schema: String,
    ledger_bytes: u64,
    ledger_prefix_sha256: String,
    policy: EvidencePolicyV1,
    next_sequence: u64,
    previous_record_sha256: String,
    seen_count: u64,
    seen_index_sha256: String,
    accounting: EvidenceAccounting,
    checkpoint_sha256: String,
}

#[derive(Serialize)]
struct EvidenceLedgerCheckpointDigestMaterialV2<'a> {
    schema: &'static str,
    ledger_bytes: u64,
    ledger_prefix_sha256: &'a str,
    policy: EvidencePolicyV1,
    next_sequence: u64,
    previous_record_sha256: &'a str,
    seen_count: u64,
    seen_index_sha256: &'a str,
    accounting: EvidenceAccounting,
}

struct LoadedEvidenceLedgerCheckpoint {
    ledger_bytes: u64,
    ledger_prefix_sha256: String,
    next_sequence: u64,
    previous_record_sha256: String,
    seen: EvidenceIndex,
    accounting: EvidenceAccounting,
    ledger_prefix_hasher: Sha256,
}

pub fn canonicalize_evidence_envelope(
    envelope: &RawEvidenceEnvelope,
    policy: EvidencePolicyV1,
) -> Result<CanonicalEventGraph, EvidenceRejection> {
    let payload_sha256 = canonical_payload_digest(&envelope.payload);
    canonicalize_event(envelope, policy, &payload_sha256)
}

#[must_use]
pub fn evidence_payload_sha256(payload: &[u8]) -> String {
    canonical_payload_digest(payload)
}

#[must_use]
pub fn evidence_session_id_sha256(session_id: &str) -> String {
    nando_client_evidence::evidence_session_id_sha256(session_id)
}

#[must_use]
pub fn evidence_client_intent_id_sha256(client_intent_id: &str) -> String {
    nando_client_evidence::evidence_client_intent_id_sha256(client_intent_id)
}

#[derive(Serialize)]
struct LedgerDigestMaterial<'a> {
    schema: &'static str,
    sequence: u64,
    previous_record_sha256: &'a str,
    outcome: &'a EvidenceIngestOutcome,
}

#[derive(Serialize)]
struct GraphDigestMaterial<'a> {
    schema: &'static str,
    policy_version: u32,
    schema_version: u32,
    source_stream_sha256: &'a str,
    source_offset: u64,
    event_id_sha256: &'a str,
    session_id_sha256: &'a str,
    client_intent_id_sha256: &'a Option<String>,
    call_id_sha256: &'a Option<String>,
    output_ordinal: Option<u32>,
    event_time: EvidenceEventTime,
    payload_sha256: &'a str,
    nodes: &'a [CanonicalEventNode],
}

impl DeterministicEvidenceLedger {
    pub fn open(path: impl Into<PathBuf>, policy: EvidencePolicyV1) -> Result<Self, String> {
        validate_policy(policy)?;
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("evidence_ledger_dir:{}:{error}", parent.display()))?;
        }
        let checkpoint_path = path.with_extension("checkpoint.json");
        recover_ledger_compaction(&path, &checkpoint_path)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| format!("evidence_ledger_create:{}:{error}", path.display()))?;
        let recovered_partial_tail_bytes = recover_partial_tail(&path)?;
        let checkpoint_index_path = path.with_extension("checkpoint.index");
        let mut ledger = Self {
            path,
            checkpoint_path,
            checkpoint_index_path: checkpoint_index_path.clone(),
            policy,
            next_sequence: 0,
            previous_record_sha256: "0".repeat(64),
            seen: EvidenceIndex::empty(checkpoint_index_path.clone()),
            accounting: EvidenceAccounting::default(),
            recovered_partial_tail_bytes,
            ledger_bytes: 0,
            ledger_prefix_hasher: Sha256::new(),
            events_since_checkpoint: 0,
        };
        if let Some(checkpoint) = load_ledger_checkpoint(
            &ledger.checkpoint_path,
            &ledger.checkpoint_index_path,
            &ledger.path,
            policy,
        )? {
            ledger.ledger_bytes = checkpoint.ledger_bytes;
            ledger.next_sequence = checkpoint.next_sequence;
            ledger.previous_record_sha256 = checkpoint.previous_record_sha256;
            ledger.seen = checkpoint.seen;
            ledger.accounting = checkpoint.accounting;
            ledger.ledger_prefix_hasher = checkpoint.ledger_prefix_hasher;
        }
        ledger.replay_from(ledger.ledger_bytes)?;
        ledger.persist_checkpoint()?;
        Ok(ledger)
    }

    pub fn ingest(
        &mut self,
        envelope: RawEvidenceEnvelope,
    ) -> Result<EvidenceLedgerRecord, String> {
        self.ingest_batch(vec![envelope])?
            .pop()
            .ok_or_else(|| "evidence_ledger_empty_batch_result".to_owned())
    }

    pub fn ingest_batch(
        &mut self,
        envelopes: Vec<RawEvidenceEnvelope>,
    ) -> Result<Vec<EvidenceLedgerRecord>, String> {
        if envelopes.is_empty() {
            return Ok(Vec::new());
        }
        if envelopes.len() > 1_024 {
            return Err("evidence_ledger_batch_budget_exceeded".to_owned());
        }
        let mut pending_seen = BTreeMap::<PackedEvidenceKey, [u8; 32]>::new();
        let mut next_sequence = self.next_sequence;
        let mut previous_record_sha256 = self.previous_record_sha256.clone();
        let mut records = Vec::with_capacity(envelopes.len());
        let mut bytes = Vec::new();
        for envelope in envelopes {
            let key = evidence_key(&envelope);
            let payload_sha256 = canonical_payload_digest(&envelope.payload);
            let packed_key = pack_evidence_key(&key)?;
            let packed_payload = decode_sha256(&payload_sha256)?;
            let original = pending_seen
                .get(&packed_key)
                .copied()
                .map(Some)
                .unwrap_or(self.seen.get(&packed_key)?);
            let outcome = if let Some(original) = original {
                if original == packed_payload {
                    EvidenceIngestOutcome::DuplicateIdempotent {
                        key,
                        payload_sha256,
                    }
                } else {
                    EvidenceIngestOutcome::DuplicateConflict {
                        key,
                        original_payload_sha256: encode_sha256(&original),
                        conflicting_payload_sha256: payload_sha256,
                    }
                }
            } else {
                let outcome = match canonicalize_event(&envelope, self.policy, &payload_sha256) {
                    Ok(graph) => EvidenceIngestOutcome::Normalized { graph },
                    Err(reason) => EvidenceIngestOutcome::Rejected {
                        key: key.clone(),
                        payload_sha256: payload_sha256.clone(),
                        reason,
                    },
                };
                pending_seen.insert(packed_key, packed_payload);
                outcome
            };
            let record_sha256 = canonical_json_sha256(&LedgerDigestMaterial {
                schema: EVIDENCE_LEDGER_SCHEMA_V1,
                sequence: next_sequence,
                previous_record_sha256: &previous_record_sha256,
                outcome: &outcome,
            })
            .map_err(|error| format!("evidence_record_digest:{error}"))?;
            let record = EvidenceLedgerRecord {
                schema: EVIDENCE_LEDGER_SCHEMA_V1.to_owned(),
                sequence: next_sequence,
                previous_record_sha256: previous_record_sha256.clone(),
                outcome,
                record_sha256: record_sha256.clone(),
            };
            bytes.extend_from_slice(
                &canonical_json_bytes(&record)
                    .map_err(|error| format!("evidence_record_encode:{error}"))?,
            );
            bytes.push(b'\n');
            next_sequence = next_sequence.saturating_add(1);
            previous_record_sha256 = record_sha256;
            records.push(record);
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| format!("evidence_ledger_open:{}:{error}", self.path.display()))?;
        file.write_all(&bytes)
            .map_err(|error| format!("evidence_ledger_write:{error}"))?;
        file.sync_data()
            .map_err(|error| format!("evidence_ledger_sync:{error}"))?;
        self.ledger_prefix_hasher.update(&bytes);
        for record in &records {
            apply_record_to_index(&mut self.seen, &record.outcome)?;
            self.accounting.observe(&record.outcome);
        }
        self.next_sequence = next_sequence;
        self.previous_record_sha256 = previous_record_sha256;
        self.ledger_bytes = self.ledger_bytes.saturating_add(bytes.len() as u64);
        self.events_since_checkpoint = self
            .events_since_checkpoint
            .saturating_add(records.len() as u64);
        if self.events_since_checkpoint >= CHECKPOINT_EVENT_INTERVAL
            || self.ledger_bytes >= MAX_ACTIVE_EVIDENCE_LEDGER_BYTES
        {
            self.persist_checkpoint()?;
        }
        Ok(records)
    }

    pub fn ingest_unseen_batch(
        &mut self,
        envelopes: Vec<RawEvidenceEnvelope>,
    ) -> Result<Vec<EvidenceLedgerRecord>, String> {
        let mut unseen = Vec::with_capacity(envelopes.len());
        for envelope in envelopes {
            let key = pack_evidence_key(&evidence_key(&envelope))?;
            if !self.seen.contains_key(&key)? {
                unseen.push(envelope);
            }
        }
        if unseen.is_empty() {
            Ok(Vec::new())
        } else {
            self.ingest_batch(unseen)
        }
    }

    #[must_use]
    pub fn accounting(&self) -> EvidenceAccounting {
        self.accounting
    }

    #[must_use]
    pub fn resume_offset(&self, source_stream_id: &str) -> Option<u64> {
        let source_stream_sha256 =
            domain_digest("nando.source-stream.v1", source_stream_id.as_bytes());
        let source_stream_sha256 = decode_sha256(&source_stream_sha256).ok()?;
        self.seen.max_offset_for_source(&source_stream_sha256)
    }

    #[must_use]
    pub fn recovered_partial_tail_bytes(&self) -> u64 {
        self.recovered_partial_tail_bytes
    }

    fn replay_from(&mut self, offset: u64) -> Result<(), String> {
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(format!(
                    "evidence_ledger_read:{}:{error}",
                    self.path.display()
                ));
            }
        };
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| format!("evidence_ledger_seek:{error}"))?;
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|error| format!("evidence_ledger_line:{error}"))?;
            if line.is_empty() {
                return Err("evidence_ledger_empty_record".to_owned());
            }
            let record: EvidenceLedgerRecord = serde_json::from_str(&line)
                .map_err(|error| format!("evidence_ledger_decode:{error}"))?;
            if record.schema != EVIDENCE_LEDGER_SCHEMA_V1
                || record.sequence != self.next_sequence
                || record.previous_record_sha256 != self.previous_record_sha256
            {
                return Err("evidence_ledger_chain_mismatch".to_owned());
            }
            let expected = canonical_json_sha256(&LedgerDigestMaterial {
                schema: EVIDENCE_LEDGER_SCHEMA_V1,
                sequence: record.sequence,
                previous_record_sha256: &record.previous_record_sha256,
                outcome: &record.outcome,
            })
            .map_err(|error| format!("evidence_record_digest:{error}"))?;
            if record.record_sha256 != expected {
                return Err("evidence_ledger_record_digest_mismatch".to_owned());
            }
            apply_record_to_index(&mut self.seen, &record.outcome)?;
            self.accounting.observe(&record.outcome);
            self.next_sequence = self.next_sequence.saturating_add(1);
            self.previous_record_sha256 = record.record_sha256;
            self.ledger_prefix_hasher.update(line.as_bytes());
            self.ledger_prefix_hasher.update(b"\n");
        }
        self.ledger_bytes = fs::metadata(&self.path)
            .map_err(|error| format!("evidence_ledger_metadata:{error}"))?
            .len();
        Ok(())
    }

    fn persist_checkpoint(&mut self) -> Result<(), String> {
        self.write_current_checkpoint()?;
        self.events_since_checkpoint = 0;
        if self.ledger_bytes >= MAX_ACTIVE_EVIDENCE_LEDGER_BYTES {
            self.compact_after_checkpoint()?;
        }
        Ok(())
    }

    fn write_current_checkpoint(&mut self) -> Result<(), String> {
        let ledger_prefix_sha256 = format!("{:x}", self.ledger_prefix_hasher.clone().finalize());
        let seen_index_sha256 = write_packed_evidence_index_atomic(&mut self.seen)?;
        let seen_count = self.seen.len() as u64;
        let checkpoint_sha256 = canonical_json_sha256(&EvidenceLedgerCheckpointDigestMaterialV2 {
            schema: EVIDENCE_LEDGER_CHECKPOINT_SCHEMA_V2,
            ledger_bytes: self.ledger_bytes,
            ledger_prefix_sha256: &ledger_prefix_sha256,
            policy: self.policy,
            next_sequence: self.next_sequence,
            previous_record_sha256: &self.previous_record_sha256,
            seen_count,
            seen_index_sha256: &seen_index_sha256,
            accounting: self.accounting,
        })
        .map_err(|error| format!("evidence_ledger_checkpoint_digest:{error}"))?;
        let checkpoint = EvidenceLedgerCheckpointV2 {
            schema: EVIDENCE_LEDGER_CHECKPOINT_SCHEMA_V2.to_owned(),
            ledger_bytes: self.ledger_bytes,
            ledger_prefix_sha256,
            policy: self.policy,
            next_sequence: self.next_sequence,
            previous_record_sha256: self.previous_record_sha256.clone(),
            seen_count,
            seen_index_sha256,
            accounting: self.accounting,
            checkpoint_sha256,
        };
        write_checkpoint_atomic(&self.checkpoint_path, &checkpoint)?;
        Ok(())
    }

    fn compact_after_checkpoint(&mut self) -> Result<(), String> {
        let sealed = self.path.with_extension("compacting");
        if sealed.exists() {
            return Err("evidence_ledger_compaction_marker_exists".to_owned());
        }
        fs::rename(&self.path, &sealed)
            .map_err(|error| format!("evidence_ledger_compaction_seal:{error}"))?;
        sync_checkpoint_parent(&self.path, "evidence_ledger_compaction_parent_sync")?;
        let file = File::create(&self.path)
            .map_err(|error| format!("evidence_ledger_compaction_create:{error}"))?;
        file.sync_all()
            .map_err(|error| format!("evidence_ledger_compaction_sync:{error}"))?;
        self.ledger_bytes = 0;
        self.ledger_prefix_hasher = Sha256::new();
        self.write_current_checkpoint()?;
        fs::remove_file(&sealed)
            .map_err(|error| format!("evidence_ledger_compaction_remove:{error}"))?;
        sync_checkpoint_parent(&self.path, "evidence_ledger_compaction_parent_sync")
    }
}

fn recover_ledger_compaction(path: &Path, checkpoint_path: &Path) -> Result<(), String> {
    let sealed = path.with_extension("compacting");
    if !sealed.exists() {
        return Ok(());
    }
    if !path.exists() {
        fs::rename(&sealed, path)
            .map_err(|error| format!("evidence_ledger_compaction_recover:{error}"))?;
        return sync_checkpoint_parent(path, "evidence_ledger_compaction_recover_parent");
    }
    let checkpoint_bytes = fs::read(checkpoint_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|value| {
            value
                .get("ledger_bytes")
                .and_then(serde_json::Value::as_u64)
        });
    if checkpoint_bytes == Some(0) {
        fs::remove_file(&sealed)
            .map_err(|error| format!("evidence_ledger_compaction_finish:{error}"))?;
    } else {
        fs::remove_file(path)
            .map_err(|error| format!("evidence_ledger_compaction_discard_tail:{error}"))?;
        fs::rename(&sealed, path)
            .map_err(|error| format!("evidence_ledger_compaction_restore:{error}"))?;
    }
    sync_checkpoint_parent(path, "evidence_ledger_compaction_recover_parent")
}

fn recover_partial_tail(path: &PathBuf) -> Result<u64, String> {
    let mut file = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(format!(
                "evidence_ledger_recovery_open:{}:{error}",
                path.display()
            ));
        }
    };
    let length = file
        .metadata()
        .map_err(|error| format!("evidence_ledger_recovery_metadata:{error}"))?
        .len();
    if length == 0 {
        return Ok(0);
    }
    file.seek(SeekFrom::End(-1))
        .map_err(|error| format!("evidence_ledger_recovery_seek:{error}"))?;
    let mut last = [0_u8; 1];
    file.read_exact(&mut last)
        .map_err(|error| format!("evidence_ledger_recovery_read:{error}"))?;
    if last[0] == b'\n' {
        return Ok(0);
    }
    let window = length.min(64 * 1024);
    file.seek(SeekFrom::Start(length - window))
        .map_err(|error| format!("evidence_ledger_recovery_seek:{error}"))?;
    let mut tail = vec![0_u8; window as usize];
    file.read_exact(&mut tail)
        .map_err(|error| format!("evidence_ledger_recovery_read:{error}"))?;
    let retained = tail
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(length - window, |index| length - window + index as u64 + 1);
    let removed = length.saturating_sub(retained);
    file.set_len(retained)
        .map_err(|error| format!("evidence_ledger_recovery_truncate:{error}"))?;
    file.sync_data()
        .map_err(|error| format!("evidence_ledger_recovery_sync:{error}"))?;
    Ok(removed)
}

fn load_ledger_checkpoint(
    checkpoint_path: &PathBuf,
    checkpoint_index_path: &PathBuf,
    ledger_path: &PathBuf,
    policy: EvidencePolicyV1,
) -> Result<Option<LoadedEvidenceLedgerCheckpoint>, String> {
    let bytes = match fs::read(checkpoint_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "evidence_ledger_checkpoint_read:{}:{error}",
                checkpoint_path.display()
            ));
        }
    };
    let mut loaded = if bytes
        .windows(EVIDENCE_LEDGER_CHECKPOINT_SCHEMA_V2.len())
        .any(|window| window == EVIDENCE_LEDGER_CHECKPOINT_SCHEMA_V2.as_bytes())
    {
        let checkpoint: EvidenceLedgerCheckpointV2 = serde_json::from_slice(&bytes)
            .map_err(|error| format!("evidence_ledger_checkpoint_decode:{error}"))?;
        if checkpoint.schema != EVIDENCE_LEDGER_CHECKPOINT_SCHEMA_V2 || checkpoint.policy != policy
        {
            return Err("evidence_ledger_checkpoint_contract_mismatch".to_owned());
        }
        let expected_checkpoint =
            canonical_json_sha256(&EvidenceLedgerCheckpointDigestMaterialV2 {
                schema: EVIDENCE_LEDGER_CHECKPOINT_SCHEMA_V2,
                ledger_bytes: checkpoint.ledger_bytes,
                ledger_prefix_sha256: &checkpoint.ledger_prefix_sha256,
                policy: checkpoint.policy,
                next_sequence: checkpoint.next_sequence,
                previous_record_sha256: &checkpoint.previous_record_sha256,
                seen_count: checkpoint.seen_count,
                seen_index_sha256: &checkpoint.seen_index_sha256,
                accounting: checkpoint.accounting,
            })
            .map_err(|error| format!("evidence_ledger_checkpoint_digest:{error}"))?;
        if checkpoint.checkpoint_sha256 != expected_checkpoint {
            return Err("evidence_ledger_checkpoint_digest_mismatch".to_owned());
        }
        let seen = load_packed_evidence_index(
            checkpoint_index_path,
            checkpoint.seen_count,
            &checkpoint.seen_index_sha256,
        )?;
        LoadedEvidenceLedgerCheckpoint {
            ledger_bytes: checkpoint.ledger_bytes,
            ledger_prefix_sha256: checkpoint.ledger_prefix_sha256,
            next_sequence: checkpoint.next_sequence,
            previous_record_sha256: checkpoint.previous_record_sha256,
            seen,
            accounting: checkpoint.accounting,
            ledger_prefix_hasher: Sha256::new(),
        }
    } else {
        let checkpoint: EvidenceLedgerCheckpointV1 = serde_json::from_slice(&bytes)
            .map_err(|error| format!("evidence_ledger_checkpoint_decode:{error}"))?;
        if checkpoint.schema != EVIDENCE_LEDGER_CHECKPOINT_SCHEMA_V1 || checkpoint.policy != policy
        {
            return Err("evidence_ledger_checkpoint_contract_mismatch".to_owned());
        }
        let expected_checkpoint =
            canonical_json_sha256(&EvidenceLedgerCheckpointDigestMaterialV1 {
                schema: EVIDENCE_LEDGER_CHECKPOINT_SCHEMA_V1,
                ledger_bytes: checkpoint.ledger_bytes,
                ledger_prefix_sha256: &checkpoint.ledger_prefix_sha256,
                policy: checkpoint.policy,
                next_sequence: checkpoint.next_sequence,
                previous_record_sha256: &checkpoint.previous_record_sha256,
                seen: &checkpoint.seen,
                accounting: checkpoint.accounting,
            })
            .map_err(|error| format!("evidence_ledger_checkpoint_digest:{error}"))?;
        if checkpoint.checkpoint_sha256 != expected_checkpoint
            || checkpoint
                .seen
                .windows(2)
                .any(|pair| pair[0].0 >= pair[1].0)
        {
            return Err("evidence_ledger_checkpoint_digest_mismatch".to_owned());
        }
        let mut seen = EvidenceIndex::empty(checkpoint_index_path.clone());
        for (key, payload) in checkpoint.seen {
            if seen
                .insert(pack_evidence_key(&key)?, decode_sha256(&payload)?)?
                .is_some()
            {
                return Err("evidence_index_duplicate_key".to_owned());
            }
        }
        write_packed_evidence_index_atomic(&mut seen)?;
        LoadedEvidenceLedgerCheckpoint {
            ledger_bytes: checkpoint.ledger_bytes,
            ledger_prefix_sha256: checkpoint.ledger_prefix_sha256,
            next_sequence: checkpoint.next_sequence,
            previous_record_sha256: checkpoint.previous_record_sha256,
            seen,
            accounting: checkpoint.accounting,
            ledger_prefix_hasher: Sha256::new(),
        }
    };
    let ledger_length = fs::metadata(ledger_path)
        .map_err(|error| format!("evidence_ledger_checkpoint_ledger_metadata:{error}"))?
        .len();
    if loaded.ledger_bytes > ledger_length {
        return Err("evidence_ledger_checkpoint_beyond_ledger".to_owned());
    }
    let ledger_prefix_hasher = hash_file_prefix_hasher(ledger_path, loaded.ledger_bytes)?;
    if format!("{:x}", ledger_prefix_hasher.clone().finalize()) != loaded.ledger_prefix_sha256 {
        return Err("evidence_ledger_checkpoint_prefix_mismatch".to_owned());
    }
    loaded.ledger_prefix_hasher = ledger_prefix_hasher;
    if !loaded.accounting.identity_holds()
        || loaded.accounting.ingress_total != loaded.next_sequence
    {
        return Err("evidence_ledger_checkpoint_accounting_mismatch".to_owned());
    }
    Ok(Some(loaded))
}

fn hash_file_prefix_hasher(path: &PathBuf, length: u64) -> Result<Sha256, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("evidence_ledger_prefix_open:{}:{error}", path.display()))?;
    let mut remaining = length;
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut hasher = Sha256::new();
    while remaining > 0 {
        let requested = usize::try_from(remaining.min(buffer.len() as u64))
            .map_err(|_| "evidence_ledger_prefix_length".to_owned())?;
        let count = file
            .read(&mut buffer[..requested])
            .map_err(|error| format!("evidence_ledger_prefix_read:{error}"))?;
        if count == 0 {
            return Err("evidence_ledger_prefix_short_read".to_owned());
        }
        hasher.update(&buffer[..count]);
        remaining = remaining.saturating_sub(count as u64);
    }
    Ok(hasher)
}

fn write_packed_evidence_index_atomic(seen: &mut EvidenceIndex) -> Result<String, String> {
    if seen.delta.is_empty() && seen.base_path.exists() {
        return Ok(seen.base_sha256.clone());
    }
    let temporary = seen.base_path.with_extension("tmp");
    let mut output =
        File::create(&temporary).map_err(|error| format!("evidence_index_create:{error}"))?;
    let mut hasher = Sha256::new();
    let mut base_reader = if seen.base_count == 0 {
        None
    } else {
        Some(BufReader::new(File::open(&seen.base_path).map_err(
            |error| format!("evidence_index_merge_open:{error}"),
        )?))
    };
    let mut base_remaining = seen.base_count;
    let mut base_next = read_next_packed_evidence_entry(&mut base_reader, &mut base_remaining)?;
    let mut delta = seen.delta.iter().peekable();
    let mut count = 0_u64;
    loop {
        match (base_next.as_ref(), delta.peek()) {
            (Some((base_key, _)), Some((delta_key, _))) if base_key == *delta_key => {
                return Err("evidence_index_duplicate_key".to_owned());
            }
            (Some((base_key, base_payload)), Some((delta_key, _))) if base_key < *delta_key => {
                write_packed_evidence_entry(&mut output, &mut hasher, base_key, base_payload)?;
                count = count.saturating_add(1);
                base_next = read_next_packed_evidence_entry(&mut base_reader, &mut base_remaining)?;
            }
            (_, Some((delta_key, delta_payload))) => {
                write_packed_evidence_entry(&mut output, &mut hasher, delta_key, delta_payload)?;
                count = count.saturating_add(1);
                delta.next();
            }
            (Some((base_key, base_payload)), None) => {
                write_packed_evidence_entry(&mut output, &mut hasher, base_key, base_payload)?;
                count = count.saturating_add(1);
                base_next = read_next_packed_evidence_entry(&mut base_reader, &mut base_remaining)?;
            }
            (None, None) => break,
        }
    }
    output
        .sync_data()
        .map_err(|error| format!("evidence_index_sync:{error}"))?;
    fs::rename(&temporary, &seen.base_path)
        .map_err(|error| format!("evidence_index_rename:{error}"))?;
    sync_checkpoint_parent(&seen.base_path, "evidence_index_parent_sync")?;
    let sha256 = format!("{:x}", hasher.finalize());
    seen.replace_base(count, sha256.clone());
    Ok(sha256)
}

fn load_packed_evidence_index(
    path: &PathBuf,
    expected_count: u64,
    expected_sha256: &str,
) -> Result<EvidenceIndex, String> {
    let expected_length = expected_count
        .checked_mul(PACKED_EVIDENCE_INDEX_ENTRY_BYTES as u64)
        .ok_or_else(|| "evidence_index_length_overflow".to_owned())?;
    let file = File::open(path).map_err(|error| format!("evidence_index_read:{error}"))?;
    if file
        .metadata()
        .map_err(|error| format!("evidence_index_metadata:{error}"))?
        .len()
        != expected_length
    {
        return Err("evidence_index_digest_mismatch".to_owned());
    }
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut previous = None;
    let mut source_max_offsets = BTreeMap::<[u8; 32], u64>::new();
    for _ in 0..expected_count {
        let mut entry = [0_u8; PACKED_EVIDENCE_INDEX_ENTRY_BYTES];
        reader
            .read_exact(&mut entry)
            .map_err(|error| format!("evidence_index_read:{error}"))?;
        hasher.update(entry);
        let (key, _) = decode_packed_evidence_entry(&entry)?;
        if previous.is_some_and(|previous| previous >= key) {
            return Err("evidence_index_duplicate_key".to_owned());
        }
        previous = Some(key);
        source_max_offsets
            .entry(key.source_stream_sha256)
            .and_modify(|offset| *offset = (*offset).max(key.source_offset))
            .or_insert(key.source_offset);
    }
    if format!("{:x}", hasher.finalize()) != expected_sha256 {
        return Err("evidence_index_digest_mismatch".to_owned());
    }
    Ok(EvidenceIndex {
        base_path: path.clone(),
        base_count: expected_count,
        base_sha256: expected_sha256.to_owned(),
        base_file: File::open(path).ok(),
        source_max_offsets,
        delta: BTreeMap::new(),
    })
}

fn read_packed_evidence_entry(
    file: &mut File,
    index: u64,
) -> Result<(PackedEvidenceKey, [u8; 32]), String> {
    file.seek(SeekFrom::Start(
        index.saturating_mul(PACKED_EVIDENCE_INDEX_ENTRY_BYTES as u64),
    ))
    .map_err(|error| format!("evidence_index_lookup_seek:{error}"))?;
    let mut entry = [0_u8; PACKED_EVIDENCE_INDEX_ENTRY_BYTES];
    file.read_exact(&mut entry)
        .map_err(|error| format!("evidence_index_lookup_read:{error}"))?;
    decode_packed_evidence_entry(&entry)
}

fn read_next_packed_evidence_entry(
    reader: &mut Option<BufReader<File>>,
    remaining: &mut u64,
) -> Result<Option<(PackedEvidenceKey, [u8; 32])>, String> {
    if *remaining == 0 {
        return Ok(None);
    }
    let mut entry = [0_u8; PACKED_EVIDENCE_INDEX_ENTRY_BYTES];
    reader
        .as_mut()
        .ok_or_else(|| "evidence_index_merge_reader_missing".to_owned())?
        .read_exact(&mut entry)
        .map_err(|error| format!("evidence_index_merge_read:{error}"))?;
    *remaining = remaining.saturating_sub(1);
    decode_packed_evidence_entry(&entry).map(Some)
}

fn write_packed_evidence_entry(
    file: &mut File,
    hasher: &mut Sha256,
    key: &PackedEvidenceKey,
    payload: &[u8; 32],
) -> Result<(), String> {
    let entry = encode_packed_evidence_entry(key, payload);
    file.write_all(&entry)
        .map_err(|error| format!("evidence_index_write:{error}"))?;
    hasher.update(entry);
    Ok(())
}

fn encode_packed_evidence_entry(
    key: &PackedEvidenceKey,
    payload: &[u8; 32],
) -> [u8; PACKED_EVIDENCE_INDEX_ENTRY_BYTES] {
    let mut entry = [0_u8; PACKED_EVIDENCE_INDEX_ENTRY_BYTES];
    entry[0..4].copy_from_slice(&key.policy_version.to_be_bytes());
    entry[4..36].copy_from_slice(&key.source_stream_sha256);
    entry[36..44].copy_from_slice(&key.source_offset.to_be_bytes());
    entry[44..76].copy_from_slice(&key.event_id_sha256);
    entry[76..108].copy_from_slice(payload);
    entry
}

fn decode_packed_evidence_entry(
    entry: &[u8; PACKED_EVIDENCE_INDEX_ENTRY_BYTES],
) -> Result<(PackedEvidenceKey, [u8; 32]), String> {
    let key = PackedEvidenceKey {
        policy_version: u32::from_be_bytes(
            entry[0..4]
                .try_into()
                .map_err(|_| "evidence_index_policy_decode".to_owned())?,
        ),
        source_stream_sha256: entry[4..36]
            .try_into()
            .map_err(|_| "evidence_index_source_decode".to_owned())?,
        source_offset: u64::from_be_bytes(
            entry[36..44]
                .try_into()
                .map_err(|_| "evidence_index_offset_decode".to_owned())?,
        ),
        event_id_sha256: entry[44..76]
            .try_into()
            .map_err(|_| "evidence_index_event_decode".to_owned())?,
    };
    let payload = entry[76..108]
        .try_into()
        .map_err(|_| "evidence_index_payload_decode".to_owned())?;
    Ok((key, payload))
}

fn write_checkpoint_atomic(
    path: &PathBuf,
    checkpoint: &EvidenceLedgerCheckpointV2,
) -> Result<(), String> {
    let temporary = path.with_extension("tmp");
    let mut file = File::create(&temporary)
        .map_err(|error| format!("evidence_ledger_checkpoint_create:{error}"))?;
    file.write_all(
        &canonical_json_bytes(checkpoint)
            .map_err(|error| format!("evidence_ledger_checkpoint_encode:{error}"))?,
    )
    .map_err(|error| format!("evidence_ledger_checkpoint_write:{error}"))?;
    file.sync_data()
        .map_err(|error| format!("evidence_ledger_checkpoint_sync:{error}"))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("evidence_ledger_checkpoint_rename:{error}"))?;
    sync_checkpoint_parent(path, "evidence_ledger_checkpoint_parent_sync")
}

fn sync_checkpoint_parent(path: &Path, prefix: &str) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("{prefix}:{error}"))
}

fn validate_policy(policy: EvidencePolicyV1) -> Result<(), String> {
    if policy.max_event_bytes == 0
        || policy.max_depth == 0
        || policy.max_nodes == 0
        || policy.max_string_bytes == 0
    {
        return Err("evidence_policy_zero_budget".to_owned());
    }
    Ok(())
}

fn canonicalize_event(
    envelope: &RawEvidenceEnvelope,
    policy: EvidencePolicyV1,
    payload_sha256: &str,
) -> Result<CanonicalEventGraph, EvidenceRejection> {
    if envelope.source_stream_id.is_empty()
        || envelope.event_id.is_empty()
        || envelope.session_id.is_empty()
    {
        return Err(EvidenceRejection::EmptyIdentity);
    }
    if envelope.payload.len() > policy.max_event_bytes {
        return Err(EvidenceRejection::PayloadTooLarge);
    }
    let value = parse_strict_json(&envelope.payload)?;
    let mut nodes = Vec::new();
    append_nodes(&value, "$", 0, policy, &mut nodes)?;
    let source_stream_sha256 = domain_digest(
        "nando.source-stream.v1",
        envelope.source_stream_id.as_bytes(),
    );
    let event_id_sha256 = domain_digest("nando.event-id.v1", envelope.event_id.as_bytes());
    let session_id_sha256 = evidence_session_id_sha256(&envelope.session_id);
    let client_intent_id_sha256 = envelope
        .client_intent_id
        .as_deref()
        .map(evidence_client_intent_id_sha256);
    let call_id_sha256 = envelope
        .call_id
        .as_deref()
        .map(|value| domain_digest("nando.call-id.v1", value.as_bytes()));
    let event_time = envelope
        .event_time_unix_nanos
        .map_or(EvidenceEventTime::Unknown, |unix_nanos| {
            EvidenceEventTime::Known { unix_nanos }
        });
    let graph_sha256 = canonical_json_sha256(&GraphDigestMaterial {
        schema: CANONICAL_EVENT_GRAPH_SCHEMA_V1,
        policy_version: EVIDENCE_POLICY_VERSION,
        schema_version: envelope.schema_version,
        source_stream_sha256: &source_stream_sha256,
        source_offset: envelope.source_offset,
        event_id_sha256: &event_id_sha256,
        session_id_sha256: &session_id_sha256,
        client_intent_id_sha256: &client_intent_id_sha256,
        call_id_sha256: &call_id_sha256,
        output_ordinal: envelope.output_ordinal,
        event_time,
        payload_sha256,
        nodes: &nodes,
    })
    .map_err(|_| EvidenceRejection::InvalidJson)?;
    Ok(CanonicalEventGraph {
        schema: CANONICAL_EVENT_GRAPH_SCHEMA_V1.to_owned(),
        policy_version: EVIDENCE_POLICY_VERSION,
        schema_version: envelope.schema_version,
        source_stream_sha256,
        source_offset: envelope.source_offset,
        event_id_sha256,
        session_id_sha256,
        client_intent_id_sha256,
        call_id_sha256,
        output_ordinal: envelope.output_ordinal,
        event_time,
        payload_sha256: payload_sha256.to_owned(),
        nodes,
        graph_sha256,
    })
}

fn append_nodes(
    value: &Value,
    path: &str,
    depth: usize,
    policy: EvidencePolicyV1,
    nodes: &mut Vec<CanonicalEventNode>,
) -> Result<(), EvidenceRejection> {
    if depth > policy.max_depth {
        return Err(EvidenceRejection::GraphDepthExceeded);
    }
    if nodes.len() >= policy.max_nodes {
        return Err(EvidenceRejection::GraphNodeBudgetExceeded);
    }
    match value {
        Value::Null => nodes.push(CanonicalEventNode::Null {
            path: path.to_owned(),
        }),
        Value::Bool(value) => nodes.push(CanonicalEventNode::Boolean {
            path: path.to_owned(),
            value: *value,
        }),
        Value::Number(value) => {
            let number_class = if value.as_i64().is_some() {
                CanonicalNumberClass::SignedInteger
            } else if value.as_u64().is_some() {
                CanonicalNumberClass::UnsignedInteger
            } else {
                CanonicalNumberClass::FiniteFloat
            };
            nodes.push(CanonicalEventNode::Number {
                path: path.to_owned(),
                number_class,
                value_sha256: domain_digest("nando.number-value.v1", value.to_string().as_bytes()),
            });
        }
        Value::String(value) => {
            if value.len() > policy.max_string_bytes {
                return Err(EvidenceRejection::StringBudgetExceeded);
            }
            nodes.push(CanonicalEventNode::String {
                path: path.to_owned(),
                byte_len: value.len(),
                value_sha256: domain_digest("nando.string-value.v1", value.as_bytes()),
            });
            if let Ok(parsed) = parse_strict_json(value.as_bytes())
                && !matches!(parsed, Value::String(_))
            {
                let derived_path = format!("{path}#parsed");
                if nodes.len() >= policy.max_nodes {
                    return Err(EvidenceRejection::GraphNodeBudgetExceeded);
                }
                nodes.push(CanonicalEventNode::ParsedJson {
                    path: derived_path.clone(),
                    source_path: path.to_owned(),
                });
                append_nodes(&parsed, &derived_path, depth + 1, policy, nodes)?;
            }
        }
        Value::Array(values) => {
            nodes.push(CanonicalEventNode::Array {
                path: path.to_owned(),
                len: values.len(),
            });
            for (index, child) in values.iter().enumerate() {
                append_nodes(child, &format!("{path}[{index}]"), depth + 1, policy, nodes)?;
            }
        }
        Value::Object(values) => {
            nodes.push(CanonicalEventNode::Object {
                path: path.to_owned(),
                len: values.len(),
            });
            let mut fields = values.iter().collect::<Vec<_>>();
            fields.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
            for (ordinal, (name, child)) in fields.into_iter().enumerate() {
                let child_path = format!("{path}.f{ordinal}");
                if nodes.len() >= policy.max_nodes {
                    return Err(EvidenceRejection::GraphNodeBudgetExceeded);
                }
                nodes.push(CanonicalEventNode::ObjectField {
                    path: format!("{child_path}#field"),
                    ordinal,
                    name_sha256: domain_digest("nando.field-name.v1", name.as_bytes()),
                });
                append_nodes(child, &child_path, depth + 1, policy, nodes)?;
            }
        }
    }
    Ok(())
}

fn evidence_key(envelope: &RawEvidenceEnvelope) -> EvidenceKey {
    EvidenceKey {
        policy_version: EVIDENCE_POLICY_VERSION,
        source_stream_sha256: domain_digest(
            "nando.source-stream.v1",
            envelope.source_stream_id.as_bytes(),
        ),
        source_offset: envelope.source_offset,
        event_id_sha256: domain_digest("nando.event-id.v1", envelope.event_id.as_bytes()),
    }
}

fn outcome_key_and_payload(outcome: &EvidenceIngestOutcome) -> Option<(&EvidenceKey, &str)> {
    match outcome {
        EvidenceIngestOutcome::Normalized { graph: _ } => None,
        EvidenceIngestOutcome::Rejected {
            key,
            payload_sha256,
            ..
        }
        | EvidenceIngestOutcome::DuplicateIdempotent {
            key,
            payload_sha256,
        } => Some((key, payload_sha256)),
        EvidenceIngestOutcome::DuplicateConflict { .. } => None,
    }
}

fn apply_record_to_index(
    seen: &mut EvidenceIndex,
    outcome: &EvidenceIngestOutcome,
) -> Result<(), String> {
    if let EvidenceIngestOutcome::Normalized { graph } = outcome {
        let key = EvidenceKey {
            policy_version: graph.policy_version,
            source_stream_sha256: graph.source_stream_sha256.clone(),
            source_offset: graph.source_offset,
            event_id_sha256: graph.event_id_sha256.clone(),
        };
        if seen
            .insert(
                pack_evidence_key(&key)?,
                decode_sha256(&graph.payload_sha256)?,
            )?
            .is_some()
        {
            return Err("evidence_ledger_duplicate_normalized_key".to_owned());
        }
        return Ok(());
    }
    if let EvidenceIngestOutcome::Rejected { .. } = outcome {
        let (key, payload) = outcome_key_and_payload(outcome)
            .ok_or_else(|| "evidence_ledger_missing_rejected_key".to_owned())?;
        if seen
            .insert(pack_evidence_key(key)?, decode_sha256(payload)?)?
            .is_some()
        {
            return Err("evidence_ledger_duplicate_rejected_key".to_owned());
        }
    }
    Ok(())
}

fn pack_evidence_key(key: &EvidenceKey) -> Result<PackedEvidenceKey, String> {
    Ok(PackedEvidenceKey {
        policy_version: key.policy_version,
        source_stream_sha256: decode_sha256(&key.source_stream_sha256)?,
        source_offset: key.source_offset,
        event_id_sha256: decode_sha256(&key.event_id_sha256)?,
    })
}

fn decode_sha256(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 {
        return Err("evidence_digest_length".to_owned());
    }
    let mut digest = [0_u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&value[offset..offset + 2], 16)
            .map_err(|_| "evidence_digest_hex".to_owned())?;
    }
    Ok(digest)
}

fn encode_sha256(value: &[u8; 32]) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(64);
    for byte in value {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

const fn legacy_evidence_policy_version() -> u32 {
    1
}

const fn is_legacy_evidence_policy_version(value: &u32) -> bool {
    *value == legacy_evidence_policy_version()
}

fn domain_digest(domain: &str, bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn canonical_payload_digest(payload: &[u8]) -> String {
    parse_strict_json(payload)
        .ok()
        .and_then(|value| evidence_canonical_json_bytes(&value).ok())
        .map(|bytes| domain_digest("nando.canonical-json-payload.v1", &bytes))
        .unwrap_or_else(|| domain_digest("nando.rejected-raw-payload.v1", payload))
}

fn evidence_canonical_json_bytes(value: &Value) -> Result<Vec<u8>, &'static str> {
    let mut output = Vec::new();
    write_evidence_canonical_json(value, &mut output)?;
    Ok(output)
}

fn write_evidence_canonical_json(value: &Value, output: &mut Vec<u8>) -> Result<(), &'static str> {
    match value {
        Value::Null => output.extend_from_slice(b"null"),
        Value::Bool(true) => output.extend_from_slice(b"true"),
        Value::Bool(false) => output.extend_from_slice(b"false"),
        Value::Number(number) => output.extend_from_slice(number.to_string().as_bytes()),
        Value::String(value) => output.extend_from_slice(
            serde_json::to_string(value)
                .map_err(|_| "evidence_canonical_string")?
                .as_bytes(),
        ),
        Value::Array(values) => {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                write_evidence_canonical_json(value, output)?;
            }
            output.push(b']');
        }
        Value::Object(values) => {
            output.push(b'{');
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_by_cached_key(|key| key.encode_utf16().collect::<Vec<_>>());
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                output.extend_from_slice(
                    serde_json::to_string(key)
                        .map_err(|_| "evidence_canonical_key")?
                        .as_bytes(),
                );
                output.push(b':');
                write_evidence_canonical_json(&values[key], output)?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}

fn parse_strict_json(payload: &[u8]) -> Result<Value, EvidenceRejection> {
    let mut deserializer = serde_json::Deserializer::from_slice(payload);
    let value = StrictValueSeed
        .deserialize(&mut deserializer)
        .map_err(|error| {
            if error.to_string().contains("duplicate object key") {
                EvidenceRejection::DuplicateObjectKey
            } else {
                EvidenceRejection::InvalidJson
            }
        })?;
    deserializer
        .end()
        .map_err(|_| EvidenceRejection::InvalidJson)?;
    Ok(value)
}

struct StrictValueSeed;

impl<'de> DeserializeSeed<'de> for StrictValueSeed {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictValueVisitor)
    }
}

struct StrictValueVisitor;

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_string(value.to_owned())
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(StrictValueSeed)? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = BTreeSet::new();
        let mut values = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom(format!("duplicate object key: {key}")));
            }
            values.insert(key, map.next_value_seed(StrictValueSeed)?);
        }
        Ok(Value::Object(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(offset: u64, payload: &[u8]) -> RawEvidenceEnvelope {
        RawEvidenceEnvelope {
            source_stream_id: "session-jsonl".to_owned(),
            source_offset: offset,
            event_id: format!("event-{offset}"),
            session_id: "private-session-id".to_owned(),
            client_intent_id: Some("private-intent-id".to_owned()),
            call_id: Some("private-call-id".to_owned()),
            output_ordinal: Some(1),
            event_time_unix_nanos: Some(42),
            schema_version: 1,
            payload: payload.to_vec(),
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nando-evidence-{name}-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }

    #[test]
    fn ledger_accounts_every_ingress_terminally_and_survives_restart() {
        let path = temp_path("accounting");
        let policy = EvidencePolicyV1::default();
        let mut ledger = DeterministicEvidenceLedger::open(&path, policy).expect("ledger");
        let first = envelope(1, br#"{"parts":[{"text":"private customer Alice"},3]}"#);
        ledger.ingest(first.clone()).expect("normalized");
        ledger.ingest(first).expect("duplicate");
        let mut conflict = envelope(1, br#"{"parts":[]}"#);
        conflict.event_id = "event-1".to_owned();
        ledger.ingest(conflict).expect("conflict");
        ledger
            .ingest(envelope(2, b"not-json"))
            .expect("rejection receipt");
        assert_eq!(
            ledger.accounting(),
            EvidenceAccounting {
                ingress_total: 4,
                normalized_total: 1,
                rejected_total: 1,
                duplicate_idempotent_total: 1,
                duplicate_conflict_total: 1,
            }
        );
        assert!(ledger.accounting().identity_holds());
        drop(ledger);
        let restored = DeterministicEvidenceLedger::open(&path, policy).expect("replay");
        assert!(restored.accounting().identity_holds());
        assert_eq!(restored.accounting().ingress_total, 4);
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn batch_append_is_one_durable_chain_with_in_batch_idempotency() {
        let path = temp_path("batch");
        let policy = EvidencePolicyV1::default();
        let mut ledger = DeterministicEvidenceLedger::open(&path, policy).expect("ledger");
        let first = envelope(1, br#"{"value":1}"#);
        let duplicate = first.clone();
        let second = envelope(2, br#"{"value":2}"#);
        let records = ledger
            .ingest_batch(vec![first, duplicate, second])
            .expect("batch");
        assert_eq!(records.len(), 3);
        assert!(matches!(
            records[1].outcome,
            EvidenceIngestOutcome::DuplicateIdempotent { .. }
        ));
        assert!(records.windows(2).all(|pair| {
            pair[1].sequence == pair[0].sequence + 1
                && pair[1].previous_record_sha256 == pair[0].record_sha256
        }));
        drop(ledger);
        let restored = DeterministicEvidenceLedger::open(&path, policy).expect("restart");
        assert_eq!(restored.accounting().ingress_total, 3);
        assert!(restored.accounting().identity_holds());
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn historical_unseen_batch_does_not_append_replay_duplicates() {
        let path = temp_path("unseen-batch");
        let policy = EvidencePolicyV1::default();
        let mut ledger = DeterministicEvidenceLedger::open(&path, policy).expect("ledger");
        let event = envelope(7, br#"{"value":7}"#);
        ledger.ingest(event.clone()).expect("first ingress");
        let bytes_before = fs::metadata(&path).expect("metadata").len();
        let accounting_before = ledger.accounting();
        let records = ledger
            .ingest_unseen_batch(vec![event])
            .expect("historical replay");
        assert!(records.is_empty());
        assert_eq!(ledger.accounting(), accounting_before);
        assert_eq!(fs::metadata(&path).expect("metadata").len(), bytes_before);
        assert_eq!(ledger.resume_offset("session-jsonl"), Some(7));
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn durable_graph_preserves_structure_without_private_raw_values() {
        let path = temp_path("privacy");
        let mut ledger =
            DeterministicEvidenceLedger::open(&path, EvidencePolicyV1::default()).expect("ledger");
        let record = ledger
            .ingest(envelope(
                7,
                br#"{"client_name":"Alice Example","phone":3715551234,"parts":["secret text",true]}"#,
            ))
            .expect("normalized");
        let EvidenceIngestOutcome::Normalized { graph } = record.outcome else {
            panic!("expected graph");
        };
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| matches!(node, CanonicalEventNode::Array { len: 2, .. }))
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| matches!(node, CanonicalEventNode::Number { .. }))
        );
        let durable = String::from_utf8(fs::read(&path).expect("ledger bytes")).expect("utf8");
        for private in [
            "Alice Example",
            "3715551234",
            "secret text",
            "private-session-id",
            "private-intent-id",
            "private-call-id",
            "client_name",
            "phone",
        ] {
            assert!(!durable.contains(private), "leaked {private}");
        }
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn embedded_json_string_becomes_provenance_preserving_derived_structure() {
        let path = temp_path("embedded-json");
        let mut ledger =
            DeterministicEvidenceLedger::open(&path, EvidencePolicyV1::default()).expect("ledger");
        let record = ledger
            .ingest(envelope(
                8,
                br#"{"output":"{\"rows\":[{\"amount\":3},{\"amount\":5}]}"}"#,
            ))
            .expect("normalized");
        let EvidenceIngestOutcome::Normalized { graph } = record.outcome else {
            panic!("expected graph");
        };
        assert!(graph.nodes.iter().any(|node| matches!(
            node,
            CanonicalEventNode::ParsedJson { source_path, .. } if source_path == "$.f0"
        )));
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| matches!(node, CanonicalEventNode::Array { len: 2, .. }))
        );
        let durable = String::from_utf8(fs::read(&path).expect("ledger bytes")).expect("utf8");
        for private in ["rows", "amount"] {
            assert!(!durable.contains(private), "leaked {private}");
        }
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn canonical_graph_is_byte_identical_for_object_key_order() {
        let path_a = temp_path("order-a");
        let path_b = temp_path("order-b");
        let policy = EvidencePolicyV1::default();
        let mut a = DeterministicEvidenceLedger::open(&path_a, policy).expect("a");
        let mut b = DeterministicEvidenceLedger::open(&path_b, policy).expect("b");
        let first = envelope(1, br#"{"z":2,"a":[true,"x"]}"#);
        let second = envelope(1, br#"{"a":[true,"x"],"z":2}"#);
        let canonical_payload = canonical_payload_digest(br#"{"a":[true,"x"],"z":2}"#);
        let left = a.ingest(first).expect("left");
        let right = b.ingest(second).expect("right");
        assert_eq!(left, right);
        assert!(matches!(
            &left.outcome,
            EvidenceIngestOutcome::Normalized { graph } if graph.payload_sha256 == canonical_payload
        ));
        fs::remove_file(path_a).expect("cleanup a");
        fs::remove_file(path_b).expect("cleanup b");
    }

    #[test]
    fn duplicate_json_key_receives_explicit_rejection() {
        let path = temp_path("duplicate-key");
        let mut ledger =
            DeterministicEvidenceLedger::open(&path, EvidencePolicyV1::default()).expect("ledger");
        let record = ledger
            .ingest(envelope(3, br#"{"value":1,"value":2}"#))
            .expect("terminal receipt");
        assert!(matches!(
            record.outcome,
            EvidenceIngestOutcome::Rejected {
                reason: EvidenceRejection::DuplicateObjectKey,
                ..
            }
        ));
        assert_eq!(ledger.accounting().rejected_total, 1);
        assert!(ledger.accounting().identity_holds());
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn finite_float_is_normalized_and_key_order_is_canonical() {
        let path_a = temp_path("float-a");
        let path_b = temp_path("float-b");
        let policy = EvidencePolicyV1::default();
        let mut a = DeterministicEvidenceLedger::open(&path_a, policy).expect("a");
        let mut b = DeterministicEvidenceLedger::open(&path_b, policy).expect("b");
        let left = a.ingest(envelope(1, br#"{"z":1.5,"a":2}"#)).expect("left");
        let right = b.ingest(envelope(1, br#"{"a":2,"z":1.5}"#)).expect("right");
        assert_eq!(left, right);
        assert!(matches!(
            left.outcome,
            EvidenceIngestOutcome::Normalized { graph }
                if graph.nodes.iter().any(|node| matches!(
                    node,
                    CanonicalEventNode::Number {
                        number_class: CanonicalNumberClass::FiniteFloat,
                        ..
                    }
                ))
        ));
        fs::remove_file(path_a).expect("cleanup a");
        fs::remove_file(path_b).expect("cleanup b");
    }

    #[test]
    fn restart_truncates_only_incomplete_tail_and_preserves_chain() {
        let path = temp_path("partial-tail");
        let policy = EvidencePolicyV1::default();
        let mut ledger = DeterministicEvidenceLedger::open(&path, policy).expect("ledger");
        ledger
            .ingest(envelope(1, br#"{"ok":true}"#))
            .expect("first record");
        drop(ledger);
        let committed_len = fs::metadata(&path).expect("metadata").len();
        OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("append")
            .write_all(br#"{"partial":"power-loss""#)
            .expect("partial write");

        let restored = DeterministicEvidenceLedger::open(&path, policy).expect("safe recovery");
        assert!(restored.recovered_partial_tail_bytes() > 0);
        assert_eq!(restored.accounting().ingress_total, 1);
        assert_eq!(fs::metadata(&path).expect("metadata").len(), committed_len);
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn restart_rejects_corruption_inside_completed_record() {
        let path = temp_path("committed-corruption");
        let policy = EvidencePolicyV1::default();
        let mut ledger = DeterministicEvidenceLedger::open(&path, policy).expect("ledger");
        ledger
            .ingest(envelope(1, br#"{"ok":true}"#))
            .expect("record");
        drop(ledger);
        let mut bytes = fs::read(&path).expect("bytes");
        let changed = bytes
            .iter_mut()
            .find(|byte| **byte == b't')
            .expect("mutable byte");
        *changed = b'f';
        fs::write(&path, bytes).expect("corrupt record");
        assert!(DeterministicEvidenceLedger::open(&path, policy).is_err());
        fs::remove_file(path).expect("cleanup");
    }
}
