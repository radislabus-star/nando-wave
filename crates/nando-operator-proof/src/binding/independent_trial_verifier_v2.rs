use serde::{Deserialize, Serialize};

use super::canonical::{is_sha256, pretty_json_bytes, sha256_json};
use super::physical_trial_v2::PhysicalTrialV2Error;

pub const INDEPENDENT_TRIAL_VERIFIER_RECEIPT_SCHEMA_V2: &str =
    "nando.binding-independent-trial-verifier-receipt.v2";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IndependentTrialVerifierOutcomeV2 {
    Pass,
    Fail,
    Abstain,
    Censored,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndependentTrialVerifierInputV2 {
    pub actor_observation_sha256: String,
    pub independent_verifier_program_digest_sha256: String,
    pub independently_recomputed_delta_root_sha256: String,
    pub structural_invariant_roots_sha256: Vec<String>,
    pub outcome: IndependentTrialVerifierOutcomeV2,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndependentTrialVerifierReceiptV2 {
    pub schema: String,
    pub verifier_receipt_sha256: String,
    pub actor_observation_sha256: String,
    pub independent_verifier_program_digest_sha256: String,
    pub independently_recomputed_delta_root_sha256: String,
    pub structural_invariant_roots_sha256: Vec<String>,
    pub outcome: IndependentTrialVerifierOutcomeV2,
}

impl IndependentTrialVerifierReceiptV2 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, PhysicalTrialV2Error> {
        pretty_json_bytes(self).map_err(PhysicalTrialV2Error::from)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, PhysicalTrialV2Error> {
        let receipt: Self =
            serde_json::from_slice(bytes).map_err(|_| PhysicalTrialV2Error::InvalidVerifier)?;
        if receipt.canonical_bytes()? != bytes {
            return Err(PhysicalTrialV2Error::InvalidVerifier);
        }
        validate_independent_trial_verifier_receipt_v2(&receipt)?;
        Ok(receipt)
    }
}

pub fn verify_independent_physical_trial_v2(
    mut input: IndependentTrialVerifierInputV2,
) -> Result<IndependentTrialVerifierReceiptV2, PhysicalTrialV2Error> {
    input.structural_invariant_roots_sha256.sort();
    input.structural_invariant_roots_sha256.dedup();
    let mut receipt = IndependentTrialVerifierReceiptV2 {
        schema: INDEPENDENT_TRIAL_VERIFIER_RECEIPT_SCHEMA_V2.to_owned(),
        verifier_receipt_sha256: String::new(),
        actor_observation_sha256: input.actor_observation_sha256,
        independent_verifier_program_digest_sha256: input
            .independent_verifier_program_digest_sha256,
        independently_recomputed_delta_root_sha256: input
            .independently_recomputed_delta_root_sha256,
        structural_invariant_roots_sha256: input.structural_invariant_roots_sha256,
        outcome: input.outcome,
    };
    validate_verifier_roots_v2(&receipt)?;
    receipt.verifier_receipt_sha256 = independent_trial_verifier_receipt_digest_v2(&receipt)?;
    Ok(receipt)
}

pub(crate) fn validate_independent_trial_verifier_receipt_v2(
    receipt: &IndependentTrialVerifierReceiptV2,
) -> Result<(), PhysicalTrialV2Error> {
    if receipt.schema != INDEPENDENT_TRIAL_VERIFIER_RECEIPT_SCHEMA_V2
        || receipt.verifier_receipt_sha256 != independent_trial_verifier_receipt_digest_v2(receipt)?
    {
        return Err(PhysicalTrialV2Error::InvalidVerifier);
    }
    validate_verifier_roots_v2(receipt)
}

pub(crate) fn independent_trial_verifier_receipt_digest_v2(
    receipt: &IndependentTrialVerifierReceiptV2,
) -> Result<String, PhysicalTrialV2Error> {
    sha256_json(&(
        receipt.schema.as_str(),
        receipt.actor_observation_sha256.as_str(),
        receipt.independent_verifier_program_digest_sha256.as_str(),
        receipt.independently_recomputed_delta_root_sha256.as_str(),
        &receipt.structural_invariant_roots_sha256,
        receipt.outcome,
    ))
    .map_err(PhysicalTrialV2Error::from)
}

fn validate_verifier_roots_v2(
    receipt: &IndependentTrialVerifierReceiptV2,
) -> Result<(), PhysicalTrialV2Error> {
    if !is_sha256(&receipt.actor_observation_sha256)
        || !is_sha256(&receipt.independent_verifier_program_digest_sha256)
        || !is_sha256(&receipt.independently_recomputed_delta_root_sha256)
        || receipt.structural_invariant_roots_sha256.is_empty()
        || receipt
            .structural_invariant_roots_sha256
            .iter()
            .any(|root| !is_sha256(root))
        || receipt
            .structural_invariant_roots_sha256
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(PhysicalTrialV2Error::InvalidDigest);
    }
    Ok(())
}
