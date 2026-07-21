use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::Serialize;

pub const ADMISSION_CANDIDATE_RECONSTRUCTION_SCHEMA_V1: &str =
    "nando.admission-candidate-reconstruction.v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissionCandidateCommitments {
    pub package_sha256: String,
    pub support_root_sha256: String,
    pub future_evidence_root_sha256: String,
    pub future_lineage_root_sha256: String,
    pub winner_seal_sha256: String,
    pub executable_parity_seal_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedAdmissionCandidateReconstruction {
    receipt_sha256: String,
}

impl VerifiedAdmissionCandidateReconstruction {
    #[must_use]
    pub fn receipt_sha256(&self) -> &str {
        &self.receipt_sha256
    }
}

#[derive(Serialize)]
struct ReconstructionReceiptMaterial<'a> {
    schema: &'static str,
    package_sha256: &'a str,
    support_root_sha256: &'a str,
    future_evidence_root_sha256: &'a str,
    future_lineage_root_sha256: &'a str,
    winner_seal_sha256: &'a str,
    executable_parity_seal_sha256: &'a str,
}

pub fn verify_admission_candidate_reconstruction(
    submitted: &AdmissionCandidateCommitments,
    reconstructed: &AdmissionCandidateCommitments,
) -> Result<VerifiedAdmissionCandidateReconstruction, &'static str> {
    let submitted_digests = [
        submitted.package_sha256.as_str(),
        submitted.support_root_sha256.as_str(),
        submitted.future_evidence_root_sha256.as_str(),
        submitted.future_lineage_root_sha256.as_str(),
        submitted.winner_seal_sha256.as_str(),
        submitted.executable_parity_seal_sha256.as_str(),
    ];
    if submitted_digests
        .into_iter()
        .any(|digest| !valid_nonzero_sha256(digest))
    {
        return Err("admission_candidate_reconstruction_digest_invalid");
    }
    if submitted != reconstructed {
        return Err("admission_candidate_reconstruction_mismatch");
    }
    let receipt_sha256 = canonical_json_sha256(&ReconstructionReceiptMaterial {
        schema: ADMISSION_CANDIDATE_RECONSTRUCTION_SCHEMA_V1,
        package_sha256: &submitted.package_sha256,
        support_root_sha256: &submitted.support_root_sha256,
        future_evidence_root_sha256: &submitted.future_evidence_root_sha256,
        future_lineage_root_sha256: &submitted.future_lineage_root_sha256,
        winner_seal_sha256: &submitted.winner_seal_sha256,
        executable_parity_seal_sha256: &submitted.executable_parity_seal_sha256,
    })?;
    Ok(VerifiedAdmissionCandidateReconstruction { receipt_sha256 })
}

#[cfg(test)]
mod tests {
    use nando_operator_kernel::sha256_bytes;

    use super::*;

    fn commitments() -> AdmissionCandidateCommitments {
        AdmissionCandidateCommitments {
            package_sha256: sha256_bytes(b"package"),
            support_root_sha256: sha256_bytes(b"support"),
            future_evidence_root_sha256: sha256_bytes(b"future"),
            future_lineage_root_sha256: sha256_bytes(b"lineage"),
            winner_seal_sha256: sha256_bytes(b"winner"),
            executable_parity_seal_sha256: sha256_bytes(b"parity"),
        }
    }

    #[test]
    fn reconstruction_rejects_any_commitment_drift() {
        let submitted = commitments();
        let mut reconstructed = submitted.clone();
        reconstructed.future_lineage_root_sha256 = sha256_bytes(b"other");
        assert_eq!(
            verify_admission_candidate_reconstruction(&submitted, &reconstructed),
            Err("admission_candidate_reconstruction_mismatch")
        );
        assert!(verify_admission_candidate_reconstruction(&submitted, &submitted).is_ok());
    }
}
