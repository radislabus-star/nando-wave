mod artifact_set;
mod capability;
mod input;
mod receipt;
mod reconstruct;
mod reference;
mod request_provenance;
mod surface;

pub use input::{
    IndependentVerifierBudgetV3, IndependentVerifierInputErrorV3, IndependentVerifierInputV3,
};
pub use receipt::{INDEPENDENT_VERIFIER_RECEIPT_SCHEMA_V3, IndependentVerifierReceiptV3};

use nando_operator_kernel::{canonical_json_sha256, sha256_bytes};
use serde::{Deserialize, Serialize};

use self::receipt::{ReceiptMaterialV3, seal_receipt_v3};
use self::reconstruct::{ReconstructionOutcomeV3, reconstruct_action_v3};
use self::reference::verify_reference_effect_v3;
use self::surface::{SurfaceOutcomeV3, extract_surface_v3};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IndependentVerifierVerdictV3 {
    Verified,
    RejectInvalidEvidence,
    RejectInvalidArtifact,
    RejectActorMutation,
    RejectProtocolParity,
    RejectPreservedFrame,
    AbstainUnsupportedProjection,
    AbstainBudgetExhausted,
    AbstainMissingRole,
    AbstainMissingCapability,
    AbstainAmbiguousCandidate,
    AbstainUnsupportedEffect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndependentVerifierErrorV3 {
    InvalidReceipt,
    Serialization,
}

pub fn verify_operator_result_v3(
    input: &IndependentVerifierInputV3<'_>,
    budget: IndependentVerifierBudgetV3,
) -> Result<IndependentVerifierReceiptV3, IndependentVerifierErrorV3> {
    let raw_payload_sha256 = sha256_bytes(input.provider_payload_bytes());
    let actor_output_sha256 = sha256_bytes(input.actor_output().as_bytes());
    let raw_evidence_sha256 = canonical_json_sha256(&(
        "nando.f6.raw-bounded-evidence.v3",
        input.request_sha256(),
        input.projection(),
        raw_payload_sha256.as_str(),
    ))
    .map_err(|_| IndependentVerifierErrorV3::Serialization)?;
    let artifact_set_sha256 = input.artifact_set().artifact_set_sha256().to_owned();
    let actor_claim_sha256 = actor_claim_digest_v3(input, &actor_output_sha256)?;
    let mut material = ReceiptMaterialV3 {
        request_sha256: input.request_sha256().to_owned(),
        raw_evidence_sha256,
        artifact_set_sha256,
        actor_claim_sha256,
        independent_request_view_sha256: None,
        independent_candidate_set_sha256: None,
        independent_physical_action_sha256: None,
        actor_physical_action_sha256: input.actor_action().physical_action_sha256().to_owned(),
        expected_output_sha256: None,
        actor_output_sha256,
        effect_postcondition_sha256: None,
        preserved_frame_contract_sha256: None,
        preserved_frame_observation_sha256: None,
        role_candidates: 0,
        candidate_paths: 0,
        action_classes: 0,
        verdict: IndependentVerifierVerdictV3::RejectInvalidEvidence,
    };

    if !budget.valid() {
        material.verdict = IndependentVerifierVerdictV3::AbstainBudgetExhausted;
        return seal_receipt_v3(material);
    }
    let surface = match extract_surface_v3(input, &raw_payload_sha256, budget) {
        SurfaceOutcomeV3::Complete(surface) => *surface,
        SurfaceOutcomeV3::Blocked(verdict) => {
            material.verdict = verdict;
            return seal_receipt_v3(material);
        }
    };
    material.independent_request_view_sha256 =
        Some(surface.request_view.request_view_sha256.clone());
    material.role_candidates = surface.request_view.structural.roles.len();

    let reconstructed = match reconstruct_action_v3(input, &surface, budget)? {
        ReconstructionOutcomeV3::Complete(value) => *value,
        ReconstructionOutcomeV3::Blocked(report) => {
            material.independent_candidate_set_sha256 = report.candidate_set_sha256;
            material.candidate_paths = report.candidate_paths;
            material.action_classes = report.action_classes;
            material.verdict = report.verdict;
            return seal_receipt_v3(material);
        }
    };
    material.independent_candidate_set_sha256 = Some(reconstructed.candidate_set_sha256.clone());
    material.independent_physical_action_sha256 =
        Some(reconstructed.action.physical_action_sha256().to_owned());
    material.candidate_paths = reconstructed.candidate_paths;
    material.action_classes = 1;

    let reference = verify_reference_effect_v3(input, &surface, &reconstructed, budget)?;
    material.expected_output_sha256 = reference.expected_output_sha256;
    material.effect_postcondition_sha256 = reference.effect_postcondition_sha256;
    material.preserved_frame_contract_sha256 = reference.preserved_frame_contract_sha256;
    material.preserved_frame_observation_sha256 = reference.preserved_frame_observation_sha256;
    material.verdict = reference.verdict;
    seal_receipt_v3(material)
}

fn actor_claim_digest_v3(
    input: &IndependentVerifierInputV3<'_>,
    actor_output_sha256: &str,
) -> Result<String, IndependentVerifierErrorV3> {
    let action = input.actor_action();
    canonical_json_sha256(&(
        "nando.f6.actor-result-claim.v3",
        action.derivation_sha256(),
        action.physical_action_sha256(),
        actor_output_sha256,
    ))
    .map_err(|_| IndependentVerifierErrorV3::Serialization)
}
pub use artifact_set::{IndependentVerifierArtifactSetErrorV3, IndependentVerifierArtifactSetV3};
