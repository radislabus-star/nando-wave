//! Append-only owner for source-neutral pre-action topology evidence.

use std::collections::BTreeMap;
use std::path::Path;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::{
    FramedCborLedger, multi_source::PreActionTopologyAuditRowV1, read_framed_cbor,
};

const LEDGER_PREFIX: &str = "multi-source-topology";
const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ROW_BYTES: usize = 256 * 1024;
const MAX_ARCHIVE_ROWS: usize = 262_144;

pub(super) struct MultiSourceTopologyArchive {
    ledger: FramedCborLedger,
    by_commitment: BTreeMap<String, PreActionTopologyAuditRowV1>,
    append_order: Vec<String>,
    payload_bytes: u64,
}

impl MultiSourceTopologyArchive {
    pub(super) fn open(directory: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(directory)
            .map_err(|error| format!("multi_source_topology_archive_dir:{error}"))?;
        let ledger = FramedCborLedger::open(directory, LEDGER_PREFIX)?;
        let rows = read_framed_cbor::<PreActionTopologyAuditRowV1>(directory, LEDGER_PREFIX)?;
        let mut by_commitment = BTreeMap::new();
        let mut append_order = Vec::new();
        let mut payload_bytes = 0_u64;
        for row in rows {
            validate_row(&row)?;
            let bytes = row_bytes(&row)?;
            let root = row.commit.commitment_root_sha256.clone();
            match by_commitment.get(&root) {
                Some(existing) if existing == &row => continue,
                Some(_) => return Err("multi_source_topology_archive_rebound".to_owned()),
                None => {}
            }
            payload_bytes = payload_bytes.saturating_add(bytes);
            if payload_bytes > MAX_ARCHIVE_BYTES || by_commitment.len() >= MAX_ARCHIVE_ROWS {
                return Err("multi_source_topology_archive_budget".to_owned());
            }
            append_order.push(root.clone());
            by_commitment.insert(root, row);
        }
        Ok(Self {
            ledger,
            by_commitment,
            append_order,
            payload_bytes,
        })
    }

    pub(super) fn append(&mut self, row: &PreActionTopologyAuditRowV1) -> Result<(), String> {
        self.append_unsynced(row)?;
        self.ledger.sync()
    }

    pub(super) fn append_batch(
        &mut self,
        rows: &[PreActionTopologyAuditRowV1],
    ) -> Result<(), String> {
        for row in rows {
            self.append_unsynced(row)?;
        }
        self.ledger.sync()
    }

    fn append_unsynced(&mut self, row: &PreActionTopologyAuditRowV1) -> Result<(), String> {
        validate_row(row)?;
        let root = &row.commit.commitment_root_sha256;
        if let Some(existing) = self.by_commitment.get(root) {
            return if existing == row {
                Ok(())
            } else {
                Err("multi_source_topology_archive_rebound".to_owned())
            };
        }
        let bytes = row_bytes(row)?;
        if self.by_commitment.len() >= MAX_ARCHIVE_ROWS
            || self.payload_bytes.saturating_add(bytes) > MAX_ARCHIVE_BYTES
        {
            return Err("multi_source_topology_archive_budget".to_owned());
        }
        self.ledger.append(row)?;
        self.payload_bytes = self.payload_bytes.saturating_add(bytes);
        self.append_order.push(root.clone());
        self.by_commitment.insert(root.clone(), row.clone());
        Ok(())
    }

    pub(super) fn rows(&self) -> Vec<PreActionTopologyAuditRowV1> {
        self.by_commitment.values().cloned().collect()
    }

    pub(super) fn row_by_root(
        &self,
        topology_root_sha256: &str,
    ) -> Option<PreActionTopologyAuditRowV1> {
        self.by_commitment.get(topology_root_sha256).cloned()
    }

    pub(super) fn len(&self) -> usize {
        self.by_commitment.len()
    }

    pub(super) fn max_bridge_sequence(&self) -> u64 {
        self.by_commitment
            .values()
            .filter_map(|row| row.bridge_sequence)
            .max()
            .unwrap_or(0)
    }

    pub(super) fn bridge_sequence_at_cursor(&self, rows: usize) -> Result<u64, String> {
        if rows == 0 || rows > self.append_order.len() {
            return Err("multi_source_topology_archive_cursor_out_of_range".to_owned());
        }
        self.append_order[..rows]
            .iter()
            .filter_map(|root| self.by_commitment.get(root))
            .filter_map(|row| row.bridge_sequence)
            .max()
            .filter(|sequence| *sequence > 0)
            .ok_or_else(|| "multi_source_topology_archive_sequence_missing".to_owned())
    }

    pub(super) fn cursor_after_bridge_sequence(
        &self,
        closure_capture_sequence: u64,
    ) -> Result<usize, String> {
        if closure_capture_sequence == 0 {
            return Err("multi_source_topology_archive_sequence_missing".to_owned());
        }
        let mut cursor = 0;
        let mut closure_matches = 0_u64;
        let mut future_seen = false;
        for root in &self.append_order {
            let sequence = self
                .by_commitment
                .get(root)
                .and_then(|row| row.bridge_sequence)
                .ok_or_else(|| "multi_source_topology_archive_sequence_missing".to_owned())?;
            if sequence == closure_capture_sequence {
                closure_matches = closure_matches.saturating_add(1);
            }
            if sequence <= closure_capture_sequence {
                if future_seen {
                    return Err(
                        "multi_source_topology_archive_sequence_boundary_invalid".to_owned()
                    );
                }
                cursor += 1;
            } else {
                future_seen = true;
            }
        }
        match closure_matches {
            0 => Err("multi_source_topology_archive_sequence_missing".to_owned()),
            1 => Ok(cursor),
            _ => Err("multi_source_topology_archive_sequence_order_invalid".to_owned()),
        }
    }

    pub(super) fn prefix_root(&self, rows: usize) -> Result<String, String> {
        if rows > self.append_order.len() {
            return Err("multi_source_topology_archive_prefix_out_of_range".to_owned());
        }
        canonical_json_sha256(&(
            "nando.multi-source-topology-archive-prefix.v1",
            self.append_order[..rows]
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        ))
        .map_err(|error| format!("multi_source_topology_archive_prefix:{error}"))
    }

    pub(super) fn rows_after(
        &self,
        watermark_rows: usize,
    ) -> Result<Vec<PreActionTopologyAuditRowV1>, String> {
        if watermark_rows > self.append_order.len() {
            return Err("multi_source_topology_archive_watermark_out_of_range".to_owned());
        }
        self.append_order[watermark_rows..]
            .iter()
            .map(|root| {
                self.by_commitment
                    .get(root)
                    .cloned()
                    .ok_or_else(|| "multi_source_topology_archive_index_invalid".to_owned())
            })
            .collect()
    }

    pub(super) fn rows_between(
        &self,
        start_rows: usize,
        end_rows: usize,
    ) -> Result<Vec<PreActionTopologyAuditRowV1>, String> {
        if start_rows > end_rows || end_rows > self.append_order.len() {
            return Err("multi_source_topology_archive_cursor_out_of_range".to_owned());
        }
        self.append_order[start_rows..end_rows]
            .iter()
            .map(|root| {
                self.by_commitment
                    .get(root)
                    .cloned()
                    .ok_or_else(|| "multi_source_topology_archive_index_invalid".to_owned())
            })
            .collect()
    }
}

fn validate_row(row: &PreActionTopologyAuditRowV1) -> Result<(), String> {
    let roots_valid = [
        Some(row.bridge_epoch_sha256.as_str()),
        row.record_sha256.as_deref(),
        row.capture_epoch_sha256.as_deref(),
        row.capture_event_sha256.as_deref(),
        row.capture_receipt_sha256.as_deref(),
        row.session_lineage_sha256.as_deref(),
    ]
    .into_iter()
    .all(|root| root.is_some_and(valid_nonzero_sha256));
    if !roots_valid
        || row.bridge_sequence.is_none_or(|sequence| sequence == 0)
        || row.captured_at_unix_ms.is_none_or(|captured| captured == 0)
        || !row.physical_order_proven
        || row.structure.validate().is_err()
        || row.commit.validate().is_err()
        || row.structure.turn_intent_id_sha256 != row.commit.turn_intent_id_sha256
        || row.structure.provider_capture_request_root_sha256
            != row.commit.provider_capture_request_root_sha256
        || canonical_json_sha256(row).is_err()
    {
        return Err("multi_source_topology_archive_row_invalid".to_owned());
    }
    Ok(())
}

fn row_bytes(row: &PreActionTopologyAuditRowV1) -> Result<u64, String> {
    let bytes = serde_cbor::to_vec(row)
        .map_err(|error| format!("multi_source_topology_archive_encode:{error}"))?;
    if bytes.len() > MAX_ROW_BYTES {
        return Err("multi_source_topology_archive_row_budget".to_owned());
    }
    Ok(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
}

#[cfg(test)]
#[path = "multi_source_topology_archive_tests.rs"]
mod tests;
