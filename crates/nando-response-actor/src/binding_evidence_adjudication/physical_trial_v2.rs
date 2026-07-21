use serde::{Deserialize, Serialize};

use super::canonical::{is_sha256, pretty_json_bytes, sha256_json};
use super::independent_trial_verifier_v2::{
    IndependentTrialVerifierOutcomeV2, IndependentTrialVerifierReceiptV2,
    validate_independent_trial_verifier_receipt_v2,
};
use super::physical_actor_observation_v2::{
    PhysicalActorObservationV2, PhysicalActorOutcomeV2, validate_physical_actor_observation_v2,
};
use super::wire::BindingAdjudicationErrorV1;

pub const PHYSICAL_TRIAL_RECEIPT_SCHEMA_V2: &str = "nando.binding-physical-trial-receipt.v2";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalTrialOutcomeV2 {
    Pass,
    Fail,
    Abstain,
    Censored,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalTrialJoinedRootsV2 {
    pub frozen_row_root_sha256: String,
    pub frozen_graph_root_sha256: String,
    pub capture_root_sha256: String,
    pub pre_state_root_sha256: String,
    pub actor_program_digest_sha256: String,
    pub verifier_program_digest_sha256: String,
    pub candidate_action_digest_sha256: String,
    pub observed_post_state_root_sha256: String,
    pub observed_delta_root_sha256: String,
    pub actor_observation_sha256: String,
    pub verifier_receipt_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalTrialReceiptV2 {
    pub schema: String,
    pub receipt_sha256: String,
    pub joined_roots: PhysicalTrialJoinedRootsV2,
    pub actor_observation: PhysicalActorObservationV2,
    pub verifier_receipt: IndependentTrialVerifierReceiptV2,
    pub outcome: PhysicalTrialOutcomeV2,
    pub execution_authority: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicalTrialV2Error {
    InvalidDigest,
    InvalidActor,
    InvalidVerifier,
    InvalidJoinedRoots,
    ProgramDigestNotIndependent,
    InvalidOutcome,
    InvalidReceipt,
    Serialization,
}

impl From<BindingAdjudicationErrorV1> for PhysicalTrialV2Error {
    fn from(value: BindingAdjudicationErrorV1) -> Self {
        match value {
            BindingAdjudicationErrorV1::Serialization => Self::Serialization,
            BindingAdjudicationErrorV1::InvalidDigest => Self::InvalidDigest,
            _ => Self::InvalidReceipt,
        }
    }
}

impl From<IndependentTrialVerifierOutcomeV2> for PhysicalTrialOutcomeV2 {
    fn from(value: IndependentTrialVerifierOutcomeV2) -> Self {
        match value {
            IndependentTrialVerifierOutcomeV2::Pass => Self::Pass,
            IndependentTrialVerifierOutcomeV2::Fail => Self::Fail,
            IndependentTrialVerifierOutcomeV2::Abstain => Self::Abstain,
            IndependentTrialVerifierOutcomeV2::Censored => Self::Censored,
        }
    }
}

impl PhysicalTrialReceiptV2 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, PhysicalTrialV2Error> {
        pretty_json_bytes(self).map_err(PhysicalTrialV2Error::from)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, PhysicalTrialV2Error> {
        let receipt: Self =
            serde_json::from_slice(bytes).map_err(|_| PhysicalTrialV2Error::InvalidReceipt)?;
        if receipt.canonical_bytes()? != bytes {
            return Err(PhysicalTrialV2Error::InvalidReceipt);
        }
        validate_physical_trial_receipt_v2(&receipt)?;
        Ok(receipt)
    }

    pub fn from_canonical_bytes_with_joined_roots(
        bytes: &[u8],
        joined_roots: &PhysicalTrialJoinedRootsV2,
    ) -> Result<Self, PhysicalTrialV2Error> {
        let receipt = Self::from_canonical_bytes(bytes)?;
        if &receipt.joined_roots != joined_roots {
            return Err(PhysicalTrialV2Error::InvalidJoinedRoots);
        }
        Ok(receipt)
    }
}

pub fn seal_physical_trial_receipt_v2(
    joined_roots: PhysicalTrialJoinedRootsV2,
    actor_observation: PhysicalActorObservationV2,
    verifier_receipt: IndependentTrialVerifierReceiptV2,
) -> Result<PhysicalTrialReceiptV2, PhysicalTrialV2Error> {
    validate_joined_roots_v2(&joined_roots)?;
    validate_physical_actor_observation_v2(&actor_observation)?;
    validate_independent_trial_verifier_receipt_v2(&verifier_receipt)?;
    validate_physical_trial_join_v2(&joined_roots, &actor_observation, &verifier_receipt)?;
    let outcome = PhysicalTrialOutcomeV2::from(verifier_receipt.outcome);
    let mut receipt = PhysicalTrialReceiptV2 {
        schema: PHYSICAL_TRIAL_RECEIPT_SCHEMA_V2.to_owned(),
        receipt_sha256: String::new(),
        joined_roots,
        actor_observation,
        verifier_receipt,
        outcome,
        execution_authority: false,
    };
    validate_trial_outcome_v2(&receipt)?;
    receipt.receipt_sha256 = physical_trial_receipt_digest_v2(&receipt)?;
    Ok(receipt)
}

pub(crate) fn validate_physical_trial_receipt_v2(
    receipt: &PhysicalTrialReceiptV2,
) -> Result<(), PhysicalTrialV2Error> {
    if receipt.schema != PHYSICAL_TRIAL_RECEIPT_SCHEMA_V2
        || receipt.execution_authority
        || receipt.receipt_sha256 != physical_trial_receipt_digest_v2(receipt)?
    {
        return Err(PhysicalTrialV2Error::InvalidReceipt);
    }
    validate_joined_roots_v2(&receipt.joined_roots)?;
    validate_physical_actor_observation_v2(&receipt.actor_observation)?;
    validate_independent_trial_verifier_receipt_v2(&receipt.verifier_receipt)?;
    validate_physical_trial_join_v2(
        &receipt.joined_roots,
        &receipt.actor_observation,
        &receipt.verifier_receipt,
    )?;
    validate_trial_outcome_v2(receipt)
}

pub(crate) fn physical_trial_receipt_digest_v2(
    receipt: &PhysicalTrialReceiptV2,
) -> Result<String, PhysicalTrialV2Error> {
    sha256_json(&(
        receipt.schema.as_str(),
        &receipt.joined_roots,
        &receipt.actor_observation,
        &receipt.verifier_receipt,
        receipt.outcome,
        receipt.execution_authority,
    ))
    .map_err(PhysicalTrialV2Error::from)
}

fn validate_joined_roots_v2(
    roots: &PhysicalTrialJoinedRootsV2,
) -> Result<(), PhysicalTrialV2Error> {
    let values = [
        roots.frozen_row_root_sha256.as_str(),
        roots.frozen_graph_root_sha256.as_str(),
        roots.capture_root_sha256.as_str(),
        roots.pre_state_root_sha256.as_str(),
        roots.actor_program_digest_sha256.as_str(),
        roots.verifier_program_digest_sha256.as_str(),
        roots.candidate_action_digest_sha256.as_str(),
        roots.observed_post_state_root_sha256.as_str(),
        roots.observed_delta_root_sha256.as_str(),
        roots.actor_observation_sha256.as_str(),
        roots.verifier_receipt_sha256.as_str(),
    ];
    if values.into_iter().all(is_sha256) {
        Ok(())
    } else {
        Err(PhysicalTrialV2Error::InvalidDigest)
    }
}

fn validate_physical_trial_join_v2(
    roots: &PhysicalTrialJoinedRootsV2,
    actor: &PhysicalActorObservationV2,
    verifier: &IndependentTrialVerifierReceiptV2,
) -> Result<(), PhysicalTrialV2Error> {
    if actor.actor_program_digest_sha256 == verifier.independent_verifier_program_digest_sha256
        || roots.actor_program_digest_sha256 == roots.verifier_program_digest_sha256
    {
        return Err(PhysicalTrialV2Error::ProgramDigestNotIndependent);
    }
    if roots.frozen_row_root_sha256 != actor.frozen_row_root_sha256
        || roots.frozen_graph_root_sha256 != actor.frozen_graph_root_sha256
        || roots.capture_root_sha256 != actor.capture_root_sha256
        || roots.pre_state_root_sha256 != actor.pre_state_root_sha256
        || roots.actor_program_digest_sha256 != actor.actor_program_digest_sha256
        || roots.candidate_action_digest_sha256 != actor.candidate_action_digest_sha256
        || roots.observed_post_state_root_sha256 != actor.observed_post_state_root_sha256
        || roots.observed_delta_root_sha256 != actor.observed_delta_root_sha256
        || roots.actor_observation_sha256 != actor.observation_sha256
        || roots.verifier_program_digest_sha256
            != verifier.independent_verifier_program_digest_sha256
        || roots.verifier_receipt_sha256 != verifier.verifier_receipt_sha256
        || verifier.actor_observation_sha256 != actor.observation_sha256
    {
        return Err(PhysicalTrialV2Error::InvalidJoinedRoots);
    }
    Ok(())
}

fn validate_trial_outcome_v2(receipt: &PhysicalTrialReceiptV2) -> Result<(), PhysicalTrialV2Error> {
    if receipt.outcome != PhysicalTrialOutcomeV2::from(receipt.verifier_receipt.outcome) {
        return Err(PhysicalTrialV2Error::InvalidOutcome);
    }
    if receipt.verifier_receipt.outcome == IndependentTrialVerifierOutcomeV2::Pass
        && (receipt.actor_observation.actor_outcome != PhysicalActorOutcomeV2::Applied
            || receipt
                .verifier_receipt
                .independently_recomputed_delta_root_sha256
                != receipt.actor_observation.observed_delta_root_sha256)
    {
        return Err(PhysicalTrialV2Error::InvalidOutcome);
    }
    Ok(())
}
