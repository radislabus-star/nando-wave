//! Closed B1B proof route: physical outcomes, external label trust, and H0/H1 adjudication.
//!
//! This module is proof/eval only. It can neither compile a protocol mode nor
//! grant execution authority. It preserves one causal route while splitting the
//! proof owners into physical trial, label manifest, external trust, causal
//! adjudication, report, and canonical validation boundaries.

mod binding_law_evidence_v2;
mod canonical;
mod causal_adjudicator;
mod controlled_replay;
mod external_trust;
mod independent_trial_verifier_v2;
mod label_manifest;
mod physical_actor_observation_v2;
mod physical_trial;
mod physical_trial_v2;
mod protocol_mode_compiler_v2;
mod report;
mod trusted_resolver_v2;
mod wire;

pub use binding_law_evidence_v2::{
    AcceptedBindingEvidenceScopeV2, AcceptedBindingLawEvidenceV2,
    BINDING_ADJUDICATION_REPORT_SCHEMA_V2, BindingAdjudicationOutcomeV2,
    BindingAdjudicationReportV2, BindingLawEvidenceV2Error, adjudicate_binding_law_evidence_v2,
};
pub use causal_adjudicator::adjudicate_binding_hypotheses_v1;
pub use external_trust::seal_binding_external_label_trust_v1;
pub use independent_trial_verifier_v2::{
    INDEPENDENT_TRIAL_VERIFIER_RECEIPT_SCHEMA_V2, IndependentTrialVerifierInputV2,
    IndependentTrialVerifierOutcomeV2, IndependentTrialVerifierReceiptV2,
    verify_independent_physical_trial_v2,
};
pub use label_manifest::build_binding_label_manifest_v1;
pub use physical_actor_observation_v2::{
    PHYSICAL_ACTOR_OBSERVATION_SCHEMA_V2, PhysicalActorObservationInputV2,
    PhysicalActorObservationV2, PhysicalActorOutcomeV2, observe_physical_actor_v2,
};
pub use physical_trial::observe_frozen_binding_labels_v1;
pub use physical_trial_v2::{
    PHYSICAL_TRIAL_RECEIPT_SCHEMA_V2, PhysicalTrialJoinedRootsV2, PhysicalTrialOutcomeV2,
    PhysicalTrialReceiptV2, PhysicalTrialV2Error, seal_physical_trial_receipt_v2,
};
pub use protocol_mode_compiler_v2::{
    BindingProtocolCompileVerdictV2, BindingProtocolCompilerErrorV2,
    BoundedProtocolModeCandidateV2, PROTOCOL_MODE_SET_SCHEMA_V2, ProtocolModeCompilerBudgetV2,
    ProtocolModeSetV2, ProtocolModeV2, compile_protocol_modes_v2,
};
pub use trusted_resolver_v2::{
    BindingEvidencePartitionV2, BindingTrialEvidenceLabelV2, FrozenBindingTrialRowV2,
    TRUSTED_RESOLVED_BINDING_ROWS_SCHEMA_V2, TrustedBindingResolverInputV2,
    TrustedBindingResolverReceiptSourceV2, TrustedResolvedBindingRowV2,
    TrustedResolvedBindingRowsV2, TrustedResolverV2Error, resolve_trusted_binding_rows_v2,
    trusted_binding_resolver_manifest_root_v2,
};
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
