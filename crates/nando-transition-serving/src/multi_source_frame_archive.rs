//! Append-only evidence owner for source-neutral verified relation frames.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use nando_operator_kernel::{RelationFrame, canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::{
    FramedCborLedger, is_source_neutral_relation_frame, read_framed_cbor,
};

const LEDGER_PREFIX: &str = "multi-source-frame";
const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_FRAME_BYTES: usize = 256 * 1024;
const MAX_ARCHIVE_ROWS: usize = 262_144;

pub(super) struct MultiSourceFrameArchive {
    ledger: FramedCborLedger,
    by_frame: BTreeMap<String, RelationFrame>,
    payload_bytes: u64,
}

impl MultiSourceFrameArchive {
    pub(super) fn open(directory: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(directory)
            .map_err(|error| format!("multi_source_frame_archive_dir:{error}"))?;
        let ledger = FramedCborLedger::open(directory, LEDGER_PREFIX)?;
        let frames = read_framed_cbor::<RelationFrame>(directory, LEDGER_PREFIX)?;
        let mut by_frame = BTreeMap::new();
        let mut payload_bytes = 0_u64;
        for frame in frames {
            validate_frame(&frame)?;
            let bytes = frame_bytes(&frame)?;
            match by_frame.get(&frame.frame_id_sha256) {
                Some(existing) if existing == &frame => continue,
                Some(_) => return Err("multi_source_frame_archive_rebound".to_owned()),
                None => {}
            }
            payload_bytes = payload_bytes.saturating_add(bytes);
            if payload_bytes > MAX_ARCHIVE_BYTES || by_frame.len() >= MAX_ARCHIVE_ROWS {
                return Err("multi_source_frame_archive_budget".to_owned());
            }
            by_frame.insert(frame.frame_id_sha256.clone(), frame);
        }
        Ok(Self {
            ledger,
            by_frame,
            payload_bytes,
        })
    }

    pub(super) fn append(&mut self, frame: &RelationFrame) -> Result<(), String> {
        self.append_unsynced(frame)?;
        self.ledger.sync()
    }

    pub(super) fn append_batch(&mut self, frames: &[RelationFrame]) -> Result<(), String> {
        for frame in frames {
            self.append_unsynced(frame)?;
        }
        self.ledger.sync()
    }

    fn append_unsynced(&mut self, frame: &RelationFrame) -> Result<(), String> {
        validate_frame(frame)?;
        if let Some(existing) = self.by_frame.get(&frame.frame_id_sha256) {
            return if existing == frame {
                Ok(())
            } else {
                Err("multi_source_frame_archive_rebound".to_owned())
            };
        }
        let bytes = frame_bytes(frame)?;
        if self.by_frame.len() >= MAX_ARCHIVE_ROWS
            || self.payload_bytes.saturating_add(bytes) > MAX_ARCHIVE_BYTES
        {
            return Err("multi_source_frame_archive_budget".to_owned());
        }
        self.ledger.append(frame)?;
        self.payload_bytes = self.payload_bytes.saturating_add(bytes);
        self.by_frame
            .insert(frame.frame_id_sha256.clone(), frame.clone());
        Ok(())
    }

    pub(super) fn frames_for_intents(&self, intent_ids: &BTreeSet<String>) -> Vec<RelationFrame> {
        self.by_frame
            .values()
            .filter(|frame| intent_ids.contains(&frame.client_intent_id_sha256))
            .cloned()
            .collect()
    }

    pub(super) fn frames(&self) -> Vec<RelationFrame> {
        self.by_frame.values().cloned().collect()
    }

    pub(super) fn frame_by_root(&self, frame_root_sha256: &str) -> Option<RelationFrame> {
        self.by_frame
            .values()
            .find(|frame| canonical_json_sha256(*frame).is_ok_and(|root| root == frame_root_sha256))
            .cloned()
    }

    pub(super) fn len(&self) -> usize {
        self.by_frame.len()
    }
}

fn validate_frame(frame: &RelationFrame) -> Result<(), String> {
    if !is_source_neutral_relation_frame(frame)
        || frame.verifier_label != Some(true)
        || frame.observed_at_unix_nanos == 0
        || !valid_nonzero_sha256(&frame.frame_id_sha256)
        || !valid_nonzero_sha256(&frame.event_id_sha256)
        || !valid_nonzero_sha256(&frame.client_intent_id_sha256)
        || !valid_nonzero_sha256(&frame.session_id_sha256)
        || !valid_nonzero_sha256(&frame.evidence_ref_sha256)
        || frame.atoms.is_empty()
        || canonical_json_sha256(frame).is_err()
    {
        return Err("multi_source_frame_archive_frame_invalid".to_owned());
    }
    Ok(())
}

fn frame_bytes(frame: &RelationFrame) -> Result<u64, String> {
    let bytes = serde_cbor::to_vec(frame)
        .map_err(|error| format!("multi_source_frame_archive_encode:{error}"))?;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err("multi_source_frame_archive_frame_budget".to_owned());
    }
    Ok(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
}

#[cfg(test)]
#[path = "multi_source_frame_archive_tests.rs"]
mod tests;
