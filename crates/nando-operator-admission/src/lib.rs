mod authority;
mod external_generation_v3;
mod operator_certification;
mod package_policy;
mod parity;
mod runtime_revocation;

pub use authority::*;
pub use external_generation_v3::*;
pub use operator_certification::*;
pub use package_policy::*;
pub use parity::*;
pub use runtime_revocation::*;

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
