//! Compatibility facade for capture commitments owned by operator learning.

use std::collections::BTreeMap;

pub use nando_operator_learning::CaptureCommitmentArchiveReader;
pub use nando_operator_learning::capture_provenance::*;

use crate::LiveScalarAdmissionCandidate;

/// The facade extracts candidate-owned receipts; the learning owner validates
/// only immutable capture commitments and never imports admission candidates.
pub fn verify_crystallized_capture_provenance(
    candidates: &[LiveScalarAdmissionCandidate],
    index: &CaptureCommitmentIndex,
) -> Result<(), &'static str> {
    index.validate()?;
    for candidate in candidates {
        for transition in candidate.support.iter().chain(&candidate.future) {
            let receipt = transition
                .runtime_parity_case
                .as_ref()
                .and_then(|case| case.capture_receipt.as_ref())
                .ok_or("crystallized_capture_receipt_missing")?;
            index.verify_receipt(receipt)?;
        }
    }
    Ok(())
}

pub fn verify_crystallized_capture_provenance_durable(
    candidates: &[LiveScalarAdmissionCandidate],
    index: &CaptureCommitmentIndex,
    archive: &mut CaptureCommitmentArchiveReader,
) -> Result<(), String> {
    index.validate().map_err(str::to_owned)?;
    let indexed = index
        .records
        .iter()
        .map(|record| (record.sequence, record.record_sha256.as_str()))
        .collect::<BTreeMap<_, _>>();
    for candidate in candidates {
        candidate
            .verify_evidence_partition()
            .map_err(str::to_owned)?;
        let support_max_sequence = partition_sequence_bound(&candidate.support, true)?;
        let future_min_sequence = partition_sequence_bound(&candidate.future, false)?;
        if support_max_sequence >= future_min_sequence {
            return Err("crystallized_capture_partition_reordered".to_owned());
        }
        for transition in candidate.support.iter().chain(&candidate.future) {
            let receipt = transition
                .runtime_parity_case
                .as_ref()
                .and_then(|case| case.capture_receipt.as_ref())
                .ok_or_else(|| "crystallized_capture_receipt_missing".to_owned())?;
            // The archive is authority for every fresh-generation receipt.
            // The rolling index is only a cache, but any cached disagreement
            // remains a hard provenance failure.
            archive.verify_receipt(receipt)?;
            for record in &receipt.records {
                if let Some(indexed_digest) = indexed.get(&record.sequence)
                    && *indexed_digest != record.record_sha256.as_str()
                {
                    return Err("capture_receipt_index_mismatch".to_owned());
                }
            }
        }
    }
    Ok(())
}

fn partition_sequence_bound(
    rows: &[crate::TeacherTransition],
    maximum: bool,
) -> Result<u64, String> {
    let mut sequences = rows
        .iter()
        .map(|transition| {
            transition
                .runtime_parity_case
                .as_ref()
                .and_then(|case| case.capture_receipt.as_ref())
                .ok_or_else(|| "crystallized_capture_receipt_missing".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flat_map(|receipt| receipt.records.iter().map(|record| record.sequence));
    let first = sequences
        .next()
        .ok_or_else(|| "crystallized_capture_partition_empty".to_owned())?;
    Ok(sequences.fold(first, |bound, sequence| {
        if maximum {
            bound.max(sequence)
        } else {
            bound.min(sequence)
        }
    }))
}
