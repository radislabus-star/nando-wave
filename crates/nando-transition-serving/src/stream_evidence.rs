use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Instant;

use nando_operator_kernel::canonical_json_sha256;
use nando_operator_learning::{
    CaptureCommitmentArchive, CaptureCommitmentIndex, CaptureEvidenceReceipt,
    CaptureRecordCommitment, CaptureTransitionBinding, EVIDENCE_LEDGER_SCHEMA_V1,
    EVIDENCE_POLICY_VERSION, EvidenceAccounting, EvidenceIngestOutcome, EvidenceKey,
    EvidenceLedgerRecord, EvidencePolicyV1, FramedCborLedger, MAX_CAPTURE_COMMITMENT_INDEX_RECORDS,
    RawEvidenceEnvelope, canonicalize_evidence_envelope, evidence_payload_sha256, read_framed_cbor,
    write_atomic_cbor,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::capture_transition_binding_archive::CaptureTransitionBindingArchive;

const CHECKPOINT_SCHEMA: &str = "nando.streaming-evidence-checkpoint.v2";
const CHECKPOINT_EVENTS: u64 = 64;
const RECENT_DEDUPE_KEYS: usize = 16_384;

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct StreamEvidenceKey {
    source_stream_sha256: String,
    source_offset: u64,
    event_id_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StreamingEvidenceCheckpoint {
    schema: String,
    policy: EvidencePolicyV1,
    next_sequence: u64,
    previous_record_sha256: String,
    accounting: EvidenceAccounting,
    source_offsets: BTreeMap<String, u64>,
    recent: Vec<(StreamEvidenceKey, String)>,
    checkpoint_sha256: String,
}

#[derive(Serialize)]
struct EvidenceRecordDigest<'a> {
    schema: &'a str,
    sequence: u64,
    previous_record_sha256: &'a str,
    outcome: &'a EvidenceIngestOutcome,
}

#[derive(Serialize)]
struct EvidenceCheckpointDigest<'a> {
    schema: &'a str,
    policy: EvidencePolicyV1,
    next_sequence: u64,
    previous_record_sha256: &'a str,
    accounting: EvidenceAccounting,
    source_offsets: &'a BTreeMap<String, u64>,
    recent: &'a [(StreamEvidenceKey, String)],
}

pub trait SessionEvidenceLedger {
    fn ingest_session_event(
        &mut self,
        envelope: RawEvidenceEnvelope,
    ) -> Result<EvidenceLedgerRecord, String>;
    fn resume_offset(&self, source_stream_id: &str) -> Option<u64>;
    fn accounting(&self) -> EvidenceAccounting;
    fn recovered_partial_tail_bytes(&self) -> u64;
    fn bind_transition(
        &mut self,
        frame_id_sha256: &str,
        receipt: &CaptureEvidenceReceipt,
    ) -> Result<CaptureTransitionBinding, String>;
}

pub struct StreamingEvidenceLedger {
    policy: EvidencePolicyV1,
    journal: FramedCborLedger,
    checkpoint_path: PathBuf,
    capture_index_path: PathBuf,
    next_sequence: u64,
    previous_record_sha256: String,
    accounting: EvidenceAccounting,
    source_offsets: BTreeMap<String, u64>,
    recent: BTreeMap<StreamEvidenceKey, String>,
    recent_order: VecDeque<StreamEvidenceKey>,
    recent_record_commitments: VecDeque<CaptureRecordCommitment>,
    commitment_archive: CaptureCommitmentArchive,
    transition_binding_archive: CaptureTransitionBindingArchive,
    events_since_checkpoint: u64,
    last_checkpoint: Instant,
    recovered_partial_tail_bytes: u64,
}

impl StreamingEvidenceLedger {
    pub fn open(directory: impl AsRef<Path>, policy: EvidencePolicyV1) -> Result<Self, String> {
        let directory = directory.as_ref();
        let checkpoint_path = directory.join("checkpoint.cbor");
        let capture_index_path = directory.join("capture-commitment-index.cbor");
        let recent_record_commitments = match std::fs::read(&capture_index_path) {
            Ok(bytes) => {
                let index = serde_cbor::from_slice::<CaptureCommitmentIndex>(&bytes)
                    .map_err(|error| format!("capture_commitment_index_decode:{error}"))?;
                index.validate().map_err(str::to_owned)?;
                index.records.into()
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => VecDeque::new(),
            Err(error) => return Err(format!("capture_commitment_index_read:{error}")),
        };
        let checkpoint = match std::fs::read(&checkpoint_path) {
            Ok(bytes) => {
                let checkpoint = serde_cbor::from_slice::<StreamingEvidenceCheckpoint>(&bytes)
                    .map_err(|error| format!("streaming_evidence_checkpoint_decode:{error}"))?;
                if checkpoint.schema != CHECKPOINT_SCHEMA || checkpoint.policy != policy {
                    return Err("streaming_evidence_checkpoint_contract_mismatch".to_owned());
                }
                let expected = checkpoint_digest(&checkpoint)?;
                if checkpoint.checkpoint_sha256 != expected {
                    return Err("streaming_evidence_checkpoint_digest_mismatch".to_owned());
                }
                Some(checkpoint)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(format!("streaming_evidence_checkpoint_read:{error}"));
            }
        };
        let archive_base_sequence = checkpoint
            .as_ref()
            .map_or(0, |checkpoint| checkpoint.next_sequence);
        let commitment_archive = CaptureCommitmentArchive::open(directory, archive_base_sequence)?;
        let transition_binding_archive = CaptureTransitionBindingArchive::open(directory)?;
        let mut state = if let Some(checkpoint) = checkpoint {
            let recent = checkpoint
                .recent
                .iter()
                .cloned()
                .collect::<BTreeMap<_, _>>();
            let recent_order = checkpoint.recent.into_iter().map(|(key, _)| key).collect();
            Self {
                policy,
                journal: FramedCborLedger::open(directory, "canonical-event")?,
                checkpoint_path,
                capture_index_path,
                next_sequence: checkpoint.next_sequence,
                previous_record_sha256: checkpoint.previous_record_sha256,
                accounting: checkpoint.accounting,
                source_offsets: checkpoint.source_offsets,
                recent,
                recent_order,
                recent_record_commitments,
                commitment_archive,
                transition_binding_archive,
                events_since_checkpoint: 0,
                last_checkpoint: Instant::now(),
                recovered_partial_tail_bytes: 0,
            }
        } else {
            Self {
                policy,
                journal: FramedCborLedger::open(directory, "canonical-event")?,
                checkpoint_path,
                capture_index_path,
                next_sequence: 0,
                previous_record_sha256: "0".repeat(64),
                accounting: EvidenceAccounting::default(),
                source_offsets: BTreeMap::new(),
                recent: BTreeMap::new(),
                recent_order: VecDeque::new(),
                recent_record_commitments,
                commitment_archive,
                transition_binding_archive,
                events_since_checkpoint: 0,
                last_checkpoint: Instant::now(),
                recovered_partial_tail_bytes: 0,
            }
        };
        state.recovered_partial_tail_bytes = state.journal.status().recovered_tail_bytes;
        for record in read_framed_cbor::<EvidenceLedgerRecord>(directory, "canonical-event")? {
            if record.sequence < state.next_sequence {
                continue;
            }
            state.apply_replayed(record)?;
        }
        state.trim_recent();
        state.persist_checkpoint()?;
        Ok(state)
    }

    fn ingest(&mut self, envelope: RawEvidenceEnvelope) -> Result<EvidenceLedgerRecord, String> {
        let source_stream_sha256 = domain_digest(
            "nando.source-stream.v1",
            envelope.source_stream_id.as_bytes(),
        );
        let event_id_sha256 = domain_digest("nando.event-id.v1", envelope.event_id.as_bytes());
        let payload_sha256 = evidence_payload_sha256(&envelope.payload);
        let stream_key = StreamEvidenceKey {
            source_stream_sha256: source_stream_sha256.clone(),
            source_offset: envelope.source_offset,
            event_id_sha256: event_id_sha256.clone(),
        };
        let key = EvidenceKey {
            policy_version: EVIDENCE_POLICY_VERSION,
            source_stream_sha256,
            source_offset: envelope.source_offset,
            event_id_sha256,
        };
        let outcome = if let Some(original) = self.recent.get(&stream_key) {
            if original == &payload_sha256 {
                EvidenceIngestOutcome::DuplicateIdempotent {
                    key,
                    payload_sha256: payload_sha256.clone(),
                }
            } else {
                EvidenceIngestOutcome::DuplicateConflict {
                    key,
                    original_payload_sha256: original.clone(),
                    conflicting_payload_sha256: payload_sha256.clone(),
                }
            }
        } else {
            match canonicalize_evidence_envelope(&envelope, self.policy) {
                Ok(graph) => EvidenceIngestOutcome::Normalized { graph },
                Err(reason) => EvidenceIngestOutcome::Rejected {
                    key,
                    payload_sha256: payload_sha256.clone(),
                    reason,
                },
            }
        };
        let record_sha256 = canonical_json_sha256(&EvidenceRecordDigest {
            schema: EVIDENCE_LEDGER_SCHEMA_V1,
            sequence: self.next_sequence,
            previous_record_sha256: &self.previous_record_sha256,
            outcome: &outcome,
        })
        .map_err(|error| format!("streaming_evidence_record_digest:{error}"))?;
        let record = EvidenceLedgerRecord {
            schema: EVIDENCE_LEDGER_SCHEMA_V1.to_owned(),
            sequence: self.next_sequence,
            previous_record_sha256: self.previous_record_sha256.clone(),
            outcome,
            record_sha256,
        };
        self.journal.append(&record)?;
        self.apply_new(record.clone(), stream_key, payload_sha256)?;
        self.events_since_checkpoint = self.events_since_checkpoint.saturating_add(1);
        if self.events_since_checkpoint >= CHECKPOINT_EVENTS
            || self.last_checkpoint.elapsed().as_secs() >= 5
        {
            self.persist_checkpoint()?;
        }
        Ok(record)
    }

    fn apply_new(
        &mut self,
        record: EvidenceLedgerRecord,
        key: StreamEvidenceKey,
        payload_sha256: String,
    ) -> Result<(), String> {
        self.remember_record_commitment(&record)?;
        self.apply_accounting_and_offset(&record);
        if !matches!(
            record.outcome,
            EvidenceIngestOutcome::DuplicateIdempotent { .. }
                | EvidenceIngestOutcome::DuplicateConflict { .. }
        ) {
            self.recent.insert(key.clone(), payload_sha256);
            self.recent_order.push_back(key);
            self.trim_recent();
        }
        Ok(())
    }

    fn apply_replayed(&mut self, record: EvidenceLedgerRecord) -> Result<(), String> {
        if record.schema != EVIDENCE_LEDGER_SCHEMA_V1
            || record.sequence != self.next_sequence
            || record.previous_record_sha256 != self.previous_record_sha256
        {
            return Err("streaming_evidence_chain_mismatch".to_owned());
        }
        let expected = canonical_json_sha256(&EvidenceRecordDigest {
            schema: &record.schema,
            sequence: record.sequence,
            previous_record_sha256: &record.previous_record_sha256,
            outcome: &record.outcome,
        })
        .map_err(|error| format!("streaming_evidence_record_digest:{error}"))?;
        if record.record_sha256 != expected {
            return Err("streaming_evidence_record_digest_mismatch".to_owned());
        }
        self.remember_record_commitment(&record)?;
        let key = outcome_key(&record.outcome);
        let payload_sha256 = outcome_payload_sha256(&record.outcome);
        self.apply_accounting_and_offset(&record);
        if let (Some(key), Some(payload_sha256)) = (key, payload_sha256)
            && !matches!(
                record.outcome,
                EvidenceIngestOutcome::DuplicateIdempotent { .. }
                    | EvidenceIngestOutcome::DuplicateConflict { .. }
            )
        {
            self.recent.insert(key.clone(), payload_sha256);
            self.recent_order.push_back(key);
        }
        Ok(())
    }

    fn trim_recent(&mut self) {
        while self.recent_order.len() > RECENT_DEDUPE_KEYS {
            if let Some(oldest) = self.recent_order.pop_front() {
                self.recent.remove(&oldest);
            }
        }
        while self.recent_record_commitments.len() > MAX_CAPTURE_COMMITMENT_INDEX_RECORDS {
            self.recent_record_commitments.pop_front();
        }
    }

    fn remember_record_commitment(&mut self, record: &EvidenceLedgerRecord) -> Result<(), String> {
        if self
            .recent_record_commitments
            .back()
            .is_some_and(|current| current.sequence >= record.sequence)
        {
            return Ok(());
        }
        let commitment = CaptureRecordCommitment {
            sequence: record.sequence,
            record_sha256: record.record_sha256.clone(),
        };
        self.commitment_archive.append(&commitment)?;
        self.recent_record_commitments.push_back(commitment);
        self.trim_recent();
        Ok(())
    }

    fn apply_accounting_and_offset(&mut self, record: &EvidenceLedgerRecord) {
        self.accounting = account_outcome(self.accounting, &record.outcome);
        if let Some(key) = outcome_key(&record.outcome) {
            self.source_offsets
                .entry(key.source_stream_sha256)
                .and_modify(|offset| *offset = (*offset).max(key.source_offset))
                .or_insert(key.source_offset);
        }
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.previous_record_sha256 = record.record_sha256.clone();
    }

    fn persist_checkpoint(&mut self) -> Result<(), String> {
        let recent = self
            .recent_order
            .iter()
            .filter_map(|key| {
                self.recent
                    .get(key)
                    .map(|payload| (key.clone(), payload.clone()))
            })
            .collect::<Vec<_>>();
        let mut checkpoint = StreamingEvidenceCheckpoint {
            schema: CHECKPOINT_SCHEMA.to_owned(),
            policy: self.policy,
            next_sequence: self.next_sequence,
            previous_record_sha256: self.previous_record_sha256.clone(),
            accounting: self.accounting,
            source_offsets: self.source_offsets.clone(),
            recent,
            checkpoint_sha256: String::new(),
        };
        checkpoint.checkpoint_sha256 = checkpoint_digest(&checkpoint)?;
        // Seal capture-owner truth first. If a later checkpoint write fails,
        // journal replay is idempotent against the already committed archive.
        self.commitment_archive.seal()?;
        self.transition_binding_archive.seal()?;
        write_atomic_cbor(&self.checkpoint_path, &checkpoint)?;
        let capture_index =
            CaptureCommitmentIndex::new(self.recent_record_commitments.iter().cloned().collect())
                .map_err(str::to_owned)?;
        write_atomic_cbor(&self.capture_index_path, &capture_index)?;
        self.journal.compact_after_checkpoint()?;
        self.events_since_checkpoint = 0;
        self.last_checkpoint = Instant::now();
        Ok(())
    }
}

fn checkpoint_digest(checkpoint: &StreamingEvidenceCheckpoint) -> Result<String, String> {
    canonical_json_sha256(&EvidenceCheckpointDigest {
        schema: &checkpoint.schema,
        policy: checkpoint.policy,
        next_sequence: checkpoint.next_sequence,
        previous_record_sha256: &checkpoint.previous_record_sha256,
        accounting: checkpoint.accounting,
        source_offsets: &checkpoint.source_offsets,
        recent: &checkpoint.recent,
    })
    .map_err(|error| format!("streaming_evidence_checkpoint_digest:{error}"))
}

impl SessionEvidenceLedger for StreamingEvidenceLedger {
    fn ingest_session_event(
        &mut self,
        envelope: RawEvidenceEnvelope,
    ) -> Result<EvidenceLedgerRecord, String> {
        self.ingest(envelope)
    }

    fn resume_offset(&self, source_stream_id: &str) -> Option<u64> {
        self.source_offsets
            .get(&domain_digest(
                "nando.source-stream.v1",
                source_stream_id.as_bytes(),
            ))
            .copied()
    }

    fn accounting(&self) -> EvidenceAccounting {
        self.accounting
    }

    fn recovered_partial_tail_bytes(&self) -> u64 {
        self.recovered_partial_tail_bytes
    }

    fn bind_transition(
        &mut self,
        frame_id_sha256: &str,
        receipt: &CaptureEvidenceReceipt,
    ) -> Result<CaptureTransitionBinding, String> {
        let binding = self
            .transition_binding_archive
            .append(frame_id_sha256, receipt)?;
        // A parity case must never outrun its capture-owned binding on crash.
        self.transition_binding_archive.seal()?;
        Ok(binding)
    }
}

fn outcome_key(outcome: &EvidenceIngestOutcome) -> Option<StreamEvidenceKey> {
    let key = match outcome {
        EvidenceIngestOutcome::Normalized { graph } => {
            return Some(StreamEvidenceKey {
                source_stream_sha256: graph.source_stream_sha256.clone(),
                source_offset: graph.source_offset,
                event_id_sha256: graph.event_id_sha256.clone(),
            });
        }
        EvidenceIngestOutcome::Rejected { key, .. }
        | EvidenceIngestOutcome::DuplicateIdempotent { key, .. }
        | EvidenceIngestOutcome::DuplicateConflict { key, .. } => key,
    };
    Some(StreamEvidenceKey {
        source_stream_sha256: key.source_stream_sha256.clone(),
        source_offset: key.source_offset,
        event_id_sha256: key.event_id_sha256.clone(),
    })
}

fn outcome_payload_sha256(outcome: &EvidenceIngestOutcome) -> Option<String> {
    match outcome {
        EvidenceIngestOutcome::Normalized { graph } => Some(graph.payload_sha256.clone()),
        EvidenceIngestOutcome::Rejected { payload_sha256, .. }
        | EvidenceIngestOutcome::DuplicateIdempotent { payload_sha256, .. } => {
            Some(payload_sha256.clone())
        }
        EvidenceIngestOutcome::DuplicateConflict {
            original_payload_sha256,
            ..
        } => Some(original_payload_sha256.clone()),
    }
}

fn account_outcome(
    mut accounting: EvidenceAccounting,
    outcome: &EvidenceIngestOutcome,
) -> EvidenceAccounting {
    accounting.ingress_total = accounting.ingress_total.saturating_add(1);
    match outcome {
        EvidenceIngestOutcome::Normalized { .. } => {
            accounting.normalized_total = accounting.normalized_total.saturating_add(1);
        }
        EvidenceIngestOutcome::Rejected { .. } => {
            accounting.rejected_total = accounting.rejected_total.saturating_add(1);
        }
        EvidenceIngestOutcome::DuplicateIdempotent { .. } => {
            accounting.duplicate_idempotent_total =
                accounting.duplicate_idempotent_total.saturating_add(1);
        }
        EvidenceIngestOutcome::DuplicateConflict { .. } => {
            accounting.duplicate_conflict_total =
                accounting.duplicate_conflict_total.saturating_add(1);
        }
    }
    accounting
}

fn domain_digest(domain: &str, bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_directory(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nando-stream-evidence-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }

    fn envelope(offset: u64, payload: &[u8]) -> RawEvidenceEnvelope {
        RawEvidenceEnvelope {
            source_stream_id: "session-jsonl".to_owned(),
            source_offset: offset,
            event_id: format!("event-{offset}"),
            session_id: "private-session".to_owned(),
            client_intent_id: Some("private-intent".to_owned()),
            call_id: None,
            output_ordinal: None,
            event_time_unix_nanos: Some(offset),
            schema_version: 1,
            payload: payload.to_vec(),
        }
    }

    #[test]
    fn restart_replays_tail_and_preserves_duplicate_semantics() {
        let root = temp_directory("restart");
        let policy = EvidencePolicyV1::default();
        let event = envelope(7, br#"{"value":7}"#);
        let mut ledger = StreamingEvidenceLedger::open(&root, policy).expect("open");
        ledger.ingest(event.clone()).expect("ingest");
        ledger.persist_checkpoint().expect("persist capture index");
        let index: CaptureCommitmentIndex = serde_cbor::from_slice(
            &fs::read(root.join("capture-commitment-index.cbor")).expect("read capture index"),
        )
        .expect("decode capture index");
        assert_eq!(index.records.len(), 1);
        assert_eq!(index.validate(), Ok(()));
        drop(ledger);

        let mut restored = StreamingEvidenceLedger::open(&root, policy).expect("restore");
        assert_eq!(restored.accounting().ingress_total, 1);
        assert_eq!(restored.resume_offset("session-jsonl"), Some(7));
        let duplicate = restored.ingest(event).expect("duplicate");
        assert!(matches!(
            duplicate.outcome,
            EvidenceIngestOutcome::DuplicateIdempotent { .. }
        ));
        assert!(restored.accounting().identity_holds());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn partial_tail_is_truncated_before_replay() {
        let root = temp_directory("partial-tail");
        let policy = EvidencePolicyV1::default();
        let mut ledger = StreamingEvidenceLedger::open(&root, policy).expect("open");
        ledger
            .ingest(envelope(1, br#"{"value":1}"#))
            .expect("ingest");
        drop(ledger);
        let segment = fs::read_dir(&root)
            .expect("segments")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("canonical-event-"))
            })
            .expect("active segment");
        OpenOptions::new()
            .append(true)
            .open(&segment)
            .expect("append tail")
            .write_all(b"partial-frame")
            .expect("write tail");

        let restored = StreamingEvidenceLedger::open(&root, policy).expect("recover");
        assert_eq!(restored.accounting().ingress_total, 1);
        assert!(restored.recovered_partial_tail_bytes() > 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn corrupted_checkpoint_fails_closed() {
        let root = temp_directory("checkpoint-corrupt");
        let policy = EvidencePolicyV1::default();
        let mut ledger = StreamingEvidenceLedger::open(&root, policy).expect("open");
        ledger
            .ingest(envelope(1, br#"{"value":1}"#))
            .expect("ingest");
        ledger.persist_checkpoint().expect("checkpoint");
        drop(ledger);
        let checkpoint = root.join("checkpoint.cbor");
        let mut bytes = fs::read(&checkpoint).expect("read checkpoint");
        let last = bytes.last_mut().expect("checkpoint bytes");
        *last ^= 0xff;
        fs::write(&checkpoint, bytes).expect("corrupt checkpoint");
        assert!(StreamingEvidenceLedger::open(&root, policy).is_err());
        let _ = fs::remove_dir_all(root);
    }
}
