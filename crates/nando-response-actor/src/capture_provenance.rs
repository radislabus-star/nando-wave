//! Compatibility facade for capture commitments owned by operator learning.

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
