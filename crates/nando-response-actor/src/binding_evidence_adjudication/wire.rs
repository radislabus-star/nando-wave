use serde::{Deserialize, Serialize};

use crate::binding_evidence::{BindingBaselineOutcomeV1, BindingEvaluationLabelV1};
use crate::binding_evidence_preregistration::BindingEvidencePartitionV1;

use super::canonical::{
    external_trust_receipt_digest, pretty_json_bytes, validate_physical_receipt_set,
};

pub const BINDING_PHYSICAL_LABEL_RECEIPT_SCHEMA_V1: &str =
    "nando.binding-physical-label-receipt.v1";
pub const BINDING_PHYSICAL_LABEL_SET_SCHEMA_V1: &str =
    "nando.binding-physical-label-receipt-set.v1";
pub const BINDING_EXTERNAL_LABEL_TRUST_SCHEMA_V1: &str = "nando.binding-external-label-trust.v1";
pub const BINDING_ADJUDICATION_REPORT_SCHEMA_V1: &str = "nando.binding-causal-adjudication.v1";
pub(super) const BINDING_OBSERVED_RELATION_SCHEMA_V1: &str = "nando.binding-observed-relation.v1";
pub(super) const BINDING_TRIAL_PARITY_DOMAIN_V1: &str = "nando.binding-trial-parity.v1";
pub(super) const BINDING_TRIAL_VERIFIER_DOMAIN_V1: &str = "nando.binding-trial-verifier.v1";
pub(super) const BINDING_RELATION_LAW_V1: &str = "parent_action_to_capability_instance";
pub(super) const REQUEST_CONTRACT_V1: &str = "continue active execution";
pub(super) const CONTROLLED_ROWS_PER_PARTITION_V1: usize = 12;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingPhysicalRelationStateV1 {
    Unique,
    Ambiguous,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingPhysicalActorOutcomeV1 {
    Applied,
    Abstained,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingObservedParentV1 {
    pub parent_ordinal: usize,
    pub parent_instance_sha256: String,
    pub capability_action_sha256: String,
    pub active: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingObservedCandidateV1 {
    pub candidate_ordinal: usize,
    pub action_equivalence_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingObservedRelationV1 {
    pub schema: String,
    pub relation_root_sha256: String,
    pub parents: Vec<BindingObservedParentV1>,
    pub requested_parent_instance_sha256: Vec<String>,
    pub requested_capability_action_sha256: Option<String>,
    pub candidates: Vec<BindingObservedCandidateV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingPhysicalCandidateTrialV1 {
    pub candidate_ordinal: usize,
    pub action_equivalence_sha256: String,
    pub actor_outcome: BindingPhysicalActorOutcomeV1,
    pub applied_parent_ordinal: Option<usize>,
    pub verifier_agrees: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingPhysicalLabelReceiptV1 {
    pub schema: String,
    pub receipt_sha256: String,
    pub row_id_sha256: String,
    pub evidence_ref_sha256: String,
    pub frozen_graph_root_sha256: String,
    pub capture_receipt_root_sha256: String,
    pub capture_sequence: u64,
    pub capture_record_sha256: String,
    pub pre_action_wire_root_sha256: String,
    pub session_lineage_sha256: String,
    pub partition: BindingEvidencePartitionV1,
    pub intervention_id: String,
    pub observed_relation: BindingObservedRelationV1,
    pub trials: Vec<BindingPhysicalCandidateTrialV1>,
    pub parity_receipt_root_sha256: String,
    pub verifier_root_sha256: String,
    pub label: BindingEvaluationLabelV1,
    pub expected_action_equivalence_sha256: Option<String>,
    pub baseline_outcome: BindingBaselineOutcomeV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingPhysicalLabelReceiptSetV1 {
    pub schema: String,
    pub receipt_sha256: String,
    pub support_freeze_file_sha256: String,
    pub future_freeze_file_sha256: String,
    pub future_external_receipt_file_sha256: String,
    pub capture_index_sha256: String,
    pub receipts: Vec<BindingPhysicalLabelReceiptV1>,
    pub execution_authority: bool,
}

impl BindingPhysicalLabelReceiptSetV1 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BindingAdjudicationErrorV1> {
        serde_json::to_vec(self).map_err(|_| BindingAdjudicationErrorV1::Serialization)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, BindingAdjudicationErrorV1> {
        let set: Self = serde_json::from_slice(bytes)
            .map_err(|_| BindingAdjudicationErrorV1::InvalidPhysicalReceipt)?;
        if set.canonical_bytes()? != bytes {
            return Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt);
        }
        validate_physical_receipt_set(&set)?;
        Ok(set)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingExternalLabelTrustReceiptV1 {
    pub schema: String,
    pub receipt_sha256: String,
    pub stop_id: String,
    pub owner_challenge_root_sha256: String,
    pub preregistration_file_sha256: String,
    pub b1a_report_file_sha256: String,
    pub support_freeze_file_sha256: String,
    pub support_watermark_file_sha256: String,
    pub future_freeze_file_sha256: String,
    pub future_external_receipt_file_sha256: String,
    pub physical_receipts_file_sha256: String,
    pub physical_receipts_root_sha256: String,
    pub label_manifest_file_sha256: String,
    pub external_manifest_root_sha256: String,
    pub expected_labels_joined: bool,
    pub protocol_mode_compiled: bool,
    pub execution_authority: bool,
}

impl BindingExternalLabelTrustReceiptV1 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BindingAdjudicationErrorV1> {
        pretty_json_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, BindingAdjudicationErrorV1> {
        let receipt: Self = serde_json::from_slice(bytes)
            .map_err(|_| BindingAdjudicationErrorV1::InvalidTrustReceipt)?;
        if receipt.canonical_bytes()? != bytes
            || receipt.receipt_sha256 != external_trust_receipt_digest(&receipt)?
        {
            return Err(BindingAdjudicationErrorV1::InvalidTrustReceipt);
        }
        Ok(receipt)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingHypothesisAdjudicationStatusV1 {
    Supported,
    Rejected,
    InsufficientEvidence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingInterventionAdjudicationV1 {
    pub intervention_id: String,
    pub support_rows: usize,
    pub future_rows: usize,
    pub positive_rows: usize,
    pub applicability_negative_rows: usize,
    pub observed_relation_states: Vec<BindingPhysicalRelationStateV1>,
    pub selected_parent_ordinals: Vec<usize>,
    pub selected_candidate_ordinals: Vec<usize>,
    pub prediction_matched: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingCausalAdjudicationReportV1 {
    pub schema: String,
    pub report_sha256: String,
    pub stop_id: String,
    pub trusted_label_manifest_sha256: String,
    pub trusted_label_root_sha256: String,
    pub physical_receipts_root_sha256: String,
    pub support_rows: usize,
    pub future_rows: usize,
    pub support_positive_rows: usize,
    pub support_applicability_negative_rows: usize,
    pub future_positive_rows: usize,
    pub future_applicability_negative_rows: usize,
    pub b1a_ties_total: usize,
    pub b1a_ties_evaluated_against_relation: usize,
    pub causal_relation: String,
    pub causal_relation_id_sha256: String,
    pub h0_status: BindingHypothesisAdjudicationStatusV1,
    pub h1_status: BindingHypothesisAdjudicationStatusV1,
    pub wrong_bindings: usize,
    pub applicability_negative_accepts: usize,
    pub parity_failures: usize,
    pub interventions: Vec<BindingInterventionAdjudicationV1>,
    pub selector_compiled: bool,
    pub protocol_mode_compiled: bool,
    pub f4_status: String,
    pub execution_authority: bool,
}

impl BindingCausalAdjudicationReportV1 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BindingAdjudicationErrorV1> {
        pretty_json_bytes(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingAdjudicationErrorV1 {
    InvalidFrozenSupport,
    InvalidFrozenFuture,
    InvalidExternalFutureReceipt,
    FrozenReplayMismatch,
    InvalidPhysicalReceipt,
    InvalidLabelManifest,
    InvalidTrustReceipt,
    InvalidPreregistration,
    InvalidB1aReport,
    InvalidDigest,
    InvalidDenominator,
    InvalidIntervention,
    InvalidRelation,
    ParityMismatch,
    Serialization,
}
