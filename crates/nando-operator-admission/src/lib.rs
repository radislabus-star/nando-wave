mod authority;
mod package_policy;
mod parity;

pub use authority::*;
pub use package_policy::*;
pub use parity::*;

pub use nando_operator_proof::{
    ADMISSION_CANDIDATE_RECONSTRUCTION_SCHEMA_V1, AdmissionCandidateCommitments,
    VerifiedAdmissionCandidateReconstruction,
};

pub fn verify_reconstructed_admission_candidate(
    submitted: &AdmissionCandidateCommitments,
    reconstructed: &AdmissionCandidateCommitments,
) -> Result<VerifiedAdmissionCandidateReconstruction, &'static str> {
    nando_operator_proof::verify_admission_candidate_reconstruction(submitted, reconstructed)
}
