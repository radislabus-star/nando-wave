//! Closed B1B proof route: physical outcomes, external label trust, and H0/H1 adjudication.
//!
//! This module is proof/eval only. It can neither compile a protocol mode nor
//! grant execution authority. It preserves one causal route while splitting the
//! proof owners into physical trial, label manifest, external trust, causal
//! adjudication, report, and canonical validation boundaries.

mod canonical;
mod causal_adjudicator;
mod controlled_replay;
mod external_trust;
mod label_manifest;
mod physical_trial;
mod report;
mod wire;

pub use causal_adjudicator::adjudicate_binding_hypotheses_v1;
pub use external_trust::seal_binding_external_label_trust_v1;
pub use label_manifest::build_binding_label_manifest_v1;
pub use nando_operator_proof::binding::*;
pub use physical_trial::observe_frozen_binding_labels_v1;
pub use wire::{
    BINDING_ADJUDICATION_REPORT_SCHEMA_V1, BINDING_EXTERNAL_LABEL_TRUST_SCHEMA_V1,
    BINDING_PHYSICAL_LABEL_RECEIPT_SCHEMA_V1, BINDING_PHYSICAL_LABEL_SET_SCHEMA_V1,
    BindingAdjudicationErrorV1, BindingCausalAdjudicationReportV1,
    BindingExternalLabelTrustReceiptV1, BindingHypothesisAdjudicationStatusV1,
    BindingInterventionAdjudicationV1, BindingObservedCandidateV1, BindingObservedParentV1,
    BindingObservedRelationV1, BindingPhysicalActorOutcomeV1, BindingPhysicalCandidateTrialV1,
    BindingPhysicalLabelReceiptSetV1, BindingPhysicalLabelReceiptV1,
    BindingPhysicalRelationStateV1,
};

#[cfg(test)]
use crate::binding_evidence::{BindingBaselineOutcomeV1, BindingEvaluationLabelV1};
#[cfg(test)]
use crate::binding_evidence_preregistration::{
    BindingEvidencePartitionV1, UntrustedBindingLabelManifestV1,
};
#[cfg(test)]
use canonical::{
    observed_relation_digest, physical_label_receipt_digest, physical_receipt_set_digest,
    sha256_bytes,
};
#[cfg(test)]
use causal_adjudicator::count_partition_labels;

#[cfg(test)]
#[path = "../binding_evidence_adjudication_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../physical_trial_v2_tests.rs"]
mod physical_trial_v2_tests;
