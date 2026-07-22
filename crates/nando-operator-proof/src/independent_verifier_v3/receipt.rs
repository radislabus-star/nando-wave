use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::{IndependentVerifierErrorV3, IndependentVerifierVerdictV3};

pub const INDEPENDENT_VERIFIER_RECEIPT_SCHEMA_V3: &str =
    "nando.operator-independent-verifier-receipt.v3.f6";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndependentVerifierReceiptV3 {
    schema: String,
    receipt_sha256: String,
    request_sha256: String,
    raw_evidence_sha256: String,
    artifact_set_sha256: String,
    actor_claim_sha256: String,
    independent_request_view_sha256: Option<String>,
    independent_candidate_set_sha256: Option<String>,
    independent_physical_action_sha256: Option<String>,
    actor_physical_action_sha256: String,
    expected_output_sha256: Option<String>,
    actor_output_sha256: String,
    effect_postcondition_sha256: Option<String>,
    preserved_frame_contract_sha256: Option<String>,
    preserved_frame_observation_sha256: Option<String>,
    role_candidates: usize,
    candidate_paths: usize,
    action_classes: usize,
    verdict: IndependentVerifierVerdictV3,
    raw_payloads_persisted: u8,
    execution_authority: bool,
}

pub(super) struct ReceiptMaterialV3 {
    pub request_sha256: String,
    pub raw_evidence_sha256: String,
    pub artifact_set_sha256: String,
    pub actor_claim_sha256: String,
    pub independent_request_view_sha256: Option<String>,
    pub independent_candidate_set_sha256: Option<String>,
    pub independent_physical_action_sha256: Option<String>,
    pub actor_physical_action_sha256: String,
    pub expected_output_sha256: Option<String>,
    pub actor_output_sha256: String,
    pub effect_postcondition_sha256: Option<String>,
    pub preserved_frame_contract_sha256: Option<String>,
    pub preserved_frame_observation_sha256: Option<String>,
    pub role_candidates: usize,
    pub candidate_paths: usize,
    pub action_classes: usize,
    pub verdict: IndependentVerifierVerdictV3,
}

#[derive(Serialize)]
struct ReceiptDigestV3<'a> {
    schema: &'a str,
    request_sha256: &'a str,
    raw_evidence_sha256: &'a str,
    artifact_set_sha256: &'a str,
    actor_claim_sha256: &'a str,
    independent_request_view_sha256: &'a Option<String>,
    independent_candidate_set_sha256: &'a Option<String>,
    independent_physical_action_sha256: &'a Option<String>,
    actor_physical_action_sha256: &'a str,
    expected_output_sha256: &'a Option<String>,
    actor_output_sha256: &'a str,
    effect_postcondition_sha256: &'a Option<String>,
    preserved_frame_contract_sha256: &'a Option<String>,
    preserved_frame_observation_sha256: &'a Option<String>,
    role_candidates: usize,
    candidate_paths: usize,
    action_classes: usize,
    verdict: IndependentVerifierVerdictV3,
    raw_payloads_persisted: u8,
    execution_authority: bool,
}

impl IndependentVerifierReceiptV3 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, IndependentVerifierErrorV3> {
        canonical_json_bytes(self).map_err(|_| IndependentVerifierErrorV3::Serialization)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, IndependentVerifierErrorV3> {
        let receipt: Self = serde_json::from_slice(bytes)
            .map_err(|_| IndependentVerifierErrorV3::InvalidReceipt)?;
        if receipt.canonical_bytes()? != bytes {
            return Err(IndependentVerifierErrorV3::InvalidReceipt);
        }
        validate_receipt_v3(&receipt)?;
        Ok(receipt)
    }

    #[must_use]
    pub fn receipt_sha256(&self) -> &str {
        &self.receipt_sha256
    }

    #[must_use]
    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    #[must_use]
    pub fn raw_evidence_sha256(&self) -> &str {
        &self.raw_evidence_sha256
    }

    #[must_use]
    pub fn artifact_set_sha256(&self) -> &str {
        &self.artifact_set_sha256
    }

    #[must_use]
    pub fn actor_claim_sha256(&self) -> &str {
        &self.actor_claim_sha256
    }

    #[must_use]
    pub fn independent_request_view_sha256(&self) -> Option<&str> {
        self.independent_request_view_sha256.as_deref()
    }

    #[must_use]
    pub fn independent_candidate_set_sha256(&self) -> Option<&str> {
        self.independent_candidate_set_sha256.as_deref()
    }

    #[must_use]
    pub fn independent_physical_action_sha256(&self) -> Option<&str> {
        self.independent_physical_action_sha256.as_deref()
    }

    #[must_use]
    pub fn actor_physical_action_sha256(&self) -> &str {
        &self.actor_physical_action_sha256
    }

    #[must_use]
    pub fn expected_output_sha256(&self) -> Option<&str> {
        self.expected_output_sha256.as_deref()
    }

    #[must_use]
    pub fn actor_output_sha256(&self) -> &str {
        &self.actor_output_sha256
    }

    #[must_use]
    pub fn effect_postcondition_sha256(&self) -> Option<&str> {
        self.effect_postcondition_sha256.as_deref()
    }

    #[must_use]
    pub fn preserved_frame_contract_sha256(&self) -> Option<&str> {
        self.preserved_frame_contract_sha256.as_deref()
    }

    #[must_use]
    pub fn preserved_frame_observation_sha256(&self) -> Option<&str> {
        self.preserved_frame_observation_sha256.as_deref()
    }

    #[must_use]
    pub const fn role_candidates(&self) -> usize {
        self.role_candidates
    }

    #[must_use]
    pub const fn candidate_paths(&self) -> usize {
        self.candidate_paths
    }

    #[must_use]
    pub const fn action_classes(&self) -> usize {
        self.action_classes
    }

    #[must_use]
    pub const fn verdict(&self) -> IndependentVerifierVerdictV3 {
        self.verdict
    }

    #[must_use]
    pub const fn raw_payloads_persisted(&self) -> u8 {
        self.raw_payloads_persisted
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        self.execution_authority
    }
}

pub(super) fn seal_receipt_v3(
    material: ReceiptMaterialV3,
) -> Result<IndependentVerifierReceiptV3, IndependentVerifierErrorV3> {
    let mut receipt = IndependentVerifierReceiptV3 {
        schema: INDEPENDENT_VERIFIER_RECEIPT_SCHEMA_V3.to_owned(),
        receipt_sha256: String::new(),
        request_sha256: material.request_sha256,
        raw_evidence_sha256: material.raw_evidence_sha256,
        artifact_set_sha256: material.artifact_set_sha256,
        actor_claim_sha256: material.actor_claim_sha256,
        independent_request_view_sha256: material.independent_request_view_sha256,
        independent_candidate_set_sha256: material.independent_candidate_set_sha256,
        independent_physical_action_sha256: material.independent_physical_action_sha256,
        actor_physical_action_sha256: material.actor_physical_action_sha256,
        expected_output_sha256: material.expected_output_sha256,
        actor_output_sha256: material.actor_output_sha256,
        effect_postcondition_sha256: material.effect_postcondition_sha256,
        preserved_frame_contract_sha256: material.preserved_frame_contract_sha256,
        preserved_frame_observation_sha256: material.preserved_frame_observation_sha256,
        role_candidates: material.role_candidates,
        candidate_paths: material.candidate_paths,
        action_classes: material.action_classes,
        verdict: material.verdict,
        raw_payloads_persisted: 0,
        execution_authority: false,
    };
    receipt.receipt_sha256 = receipt_digest_v3(&receipt)?;
    validate_receipt_fields_v3(&receipt)?;
    Ok(receipt)
}

fn validate_receipt_v3(
    receipt: &IndependentVerifierReceiptV3,
) -> Result<(), IndependentVerifierErrorV3> {
    validate_receipt_fields_v3(receipt)?;
    if !valid_nonzero_sha256(&receipt.receipt_sha256)
        || receipt.receipt_sha256 != receipt_digest_v3(receipt)?
    {
        return Err(IndependentVerifierErrorV3::InvalidReceipt);
    }
    Ok(())
}

fn validate_receipt_fields_v3(
    receipt: &IndependentVerifierReceiptV3,
) -> Result<(), IndependentVerifierErrorV3> {
    let required = [
        receipt.request_sha256.as_str(),
        receipt.raw_evidence_sha256.as_str(),
        receipt.artifact_set_sha256.as_str(),
        receipt.actor_claim_sha256.as_str(),
        receipt.actor_physical_action_sha256.as_str(),
        receipt.actor_output_sha256.as_str(),
    ];
    let optional = [
        receipt.independent_request_view_sha256.as_deref(),
        receipt.independent_candidate_set_sha256.as_deref(),
        receipt.independent_physical_action_sha256.as_deref(),
        receipt.expected_output_sha256.as_deref(),
        receipt.effect_postcondition_sha256.as_deref(),
        receipt.preserved_frame_contract_sha256.as_deref(),
        receipt.preserved_frame_observation_sha256.as_deref(),
    ];
    if receipt.schema != INDEPENDENT_VERIFIER_RECEIPT_SCHEMA_V3
        || required.into_iter().any(|root| !valid_nonzero_sha256(root))
        || optional
            .into_iter()
            .flatten()
            .any(|root| !valid_nonzero_sha256(root))
        || receipt.raw_payloads_persisted != 0
        || receipt.execution_authority
    {
        return Err(IndependentVerifierErrorV3::InvalidReceipt);
    }
    if receipt.verdict == IndependentVerifierVerdictV3::Verified
        && (receipt.independent_request_view_sha256.is_none()
            || receipt.independent_candidate_set_sha256.is_none()
            || receipt.independent_physical_action_sha256.as_deref()
                != Some(receipt.actor_physical_action_sha256.as_str())
            || receipt.expected_output_sha256.as_deref()
                != Some(receipt.actor_output_sha256.as_str())
            || receipt.effect_postcondition_sha256.is_none()
            || receipt.preserved_frame_contract_sha256.is_none()
            || receipt.preserved_frame_observation_sha256.is_none()
            || receipt.candidate_paths == 0
            || receipt.action_classes != 1)
    {
        return Err(IndependentVerifierErrorV3::InvalidReceipt);
    }
    Ok(())
}

fn receipt_digest_v3(
    receipt: &IndependentVerifierReceiptV3,
) -> Result<String, IndependentVerifierErrorV3> {
    canonical_json_sha256(&ReceiptDigestV3 {
        schema: receipt.schema.as_str(),
        request_sha256: receipt.request_sha256.as_str(),
        raw_evidence_sha256: receipt.raw_evidence_sha256.as_str(),
        artifact_set_sha256: receipt.artifact_set_sha256.as_str(),
        actor_claim_sha256: receipt.actor_claim_sha256.as_str(),
        independent_request_view_sha256: &receipt.independent_request_view_sha256,
        independent_candidate_set_sha256: &receipt.independent_candidate_set_sha256,
        independent_physical_action_sha256: &receipt.independent_physical_action_sha256,
        actor_physical_action_sha256: receipt.actor_physical_action_sha256.as_str(),
        expected_output_sha256: &receipt.expected_output_sha256,
        actor_output_sha256: receipt.actor_output_sha256.as_str(),
        effect_postcondition_sha256: &receipt.effect_postcondition_sha256,
        preserved_frame_contract_sha256: &receipt.preserved_frame_contract_sha256,
        preserved_frame_observation_sha256: &receipt.preserved_frame_observation_sha256,
        role_candidates: receipt.role_candidates,
        candidate_paths: receipt.candidate_paths,
        action_classes: receipt.action_classes,
        verdict: receipt.verdict,
        raw_payloads_persisted: receipt.raw_payloads_persisted,
        execution_authority: receipt.execution_authority,
    })
    .map_err(|_| IndependentVerifierErrorV3::Serialization)
}
