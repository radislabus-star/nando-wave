//! Read-only STOP-MS0 adapters for live evidence stores.
//!
//! The learning crate owns accounting and shape semantics. This module only
//! decodes owner-local stores and never mutates checkpoints or authority.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

use nando_operator_kernel::{AtomSource, RelationAtom, RelationFrame};
use nando_operator_learning::multi_source::{
    MultiSourceEvidenceAuditV1, RelationEvidenceAuditV1, build_multi_source_evidence_audit_v1,
};
use nando_operator_learning::opportunity::ReducibilityClass;
use sha2::{Digest, Sha256};

use crate::request_learning::RequestLearningIndex;

pub fn run_multi_source_evidence_audit_v1(
    opportunity_checkpoint: &Path,
    request_learning_checkpoint: &Path,
    relation_frames: &Path,
) -> Result<MultiSourceEvidenceAuditV1, String> {
    let opportunity_bytes = fs::read(opportunity_checkpoint).map_err(|error| {
        format!(
            "multi_source_opportunity_checkpoint_read:{}:{error}",
            opportunity_checkpoint.display()
        )
    })?;
    let opportunities = nando_response_actor::read_opportunity_audit_rows_from_checkpoint_bytes_v1(
        &opportunity_bytes,
    )?;
    let request_bytes = fs::read(request_learning_checkpoint).map_err(|error| {
        format!(
            "multi_source_request_checkpoint_read:{}:{error}",
            request_learning_checkpoint.display()
        )
    })?;
    let (request_index, _) = RequestLearningIndex::from_checkpoint_cbor(&request_bytes)?;
    let request_snapshot = request_index.audit_snapshot_v1().map_err(str::to_owned)?;
    let relevant = opportunities
        .iter()
        .filter(|row| {
            row.authority_observed && row.class == ReducibilityClass::UnexploredMultiSource
        })
        .map(|row| row.intent_sha256.clone())
        .collect::<BTreeSet<_>>();
    let (relations, rows_scanned, parse_errors, relation_sha256) =
        read_relation_summaries(relation_frames, &relevant)?;
    Ok(build_multi_source_evidence_audit_v1(
        opportunities,
        request_snapshot,
        relations,
        sha256_bytes(&opportunity_bytes),
        sha256_bytes(&request_bytes),
        relation_sha256,
        rows_scanned,
        parse_errors,
    ))
}

fn read_relation_summaries(
    path: &Path,
    relevant: &BTreeSet<String>,
) -> Result<(BTreeMap<String, RelationEvidenceAuditV1>, u64, u64, String), String> {
    let file = File::open(path).map_err(|error| {
        format!(
            "multi_source_relation_frames_open:{}:{error}",
            path.display()
        )
    })?;
    let mut summaries = BTreeMap::<String, RelationEvidenceAuditV1>::new();
    let mut rows = 0_u64;
    let mut parse_errors = 0_u64;
    let mut hasher = Sha256::new();
    let mut reader = BufReader::new(file);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        let read = reader
            .read_until(b'\n', &mut bytes)
            .map_err(|error| format!("multi_source_relation_frames_read:{error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&bytes);
        rows = rows.saturating_add(1);
        let frame = match serde_json::from_slice::<RelationFrame>(&bytes) {
            Ok(frame) => frame,
            Err(_) => {
                parse_errors = parse_errors.saturating_add(1);
                continue;
            }
        };
        if !relevant.contains(&frame.client_intent_id_sha256) {
            continue;
        }
        let summary = summaries
            .entry(frame.client_intent_id_sha256.clone())
            .or_default();
        summary.frames = summary.frames.saturating_add(1);
        match frame.verifier_label {
            Some(true) => summary.positive = summary.positive.saturating_add(1),
            Some(false) => summary.negative = summary.negative.saturating_add(1),
            None => summary.unlabeled = summary.unlabeled.saturating_add(1),
        }
        collect_observation_roles(summary, frame.atoms);
    }
    Ok((
        summaries,
        rows,
        parse_errors,
        format!("{:x}", hasher.finalize()),
    ))
}

fn collect_observation_roles(summary: &mut RelationEvidenceAuditV1, atoms: Vec<RelationAtom>) {
    for atom in atoms {
        if let RelationAtom::TypedSlot {
            slot_id,
            value_type,
            source: AtomSource::Observation,
            ..
        } = atom
        {
            summary
                .observation_roles
                .insert((slot_id, format!("{value_type:?}")));
        }
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
