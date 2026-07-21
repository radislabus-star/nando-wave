//! Frozen B1B scientific contract and trusted label provenance.
//!
//! This module preregisters hypotheses and acquisition controls. It neither
//! captures evidence nor compiles a selector. Candidate envelopes carry only
//! integrity checksums; trust exists only after an external owner pins exact
//! canonical manifest bytes.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::binding_evidence::{BindingBaselineOutcomeV1, BindingEvaluationLabelV1};

pub const BINDING_LABEL_ENVELOPE_SCHEMA_V1: &str = "nando.binding-label-envelope.v1";
pub const BINDING_LABEL_MANIFEST_SCHEMA_V1: &str = "nando.binding-label-manifest.v1";
pub const BINDING_EVIDENCE_PREREGISTRATION_SCHEMA_V1: &str =
    "nando.binding-evidence-preregistration.v1";
pub const MAX_BINDING_LABEL_ENVELOPES_V1: usize = 16_384;
pub const MIN_BINDING_POSITIVE_ROWS_PER_PARTITION_V1: usize = 6;
pub const MIN_BINDING_APPLICABILITY_NEGATIVE_ROWS_PER_PARTITION_V1: usize = 6;
pub const MIN_BINDING_ROWS_PER_INTERVENTION_PER_PARTITION_V1: usize = 1;

const BINDING_CAUSAL_INTERVENTION_IDS_V1: [&str; 6] = ["I1", "I2", "I3", "I4", "I5", "I6"];

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingEvidencePartitionV1 {
    Support,
    Future,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingLabelObservationSourceV1 {
    PreActionWire,
    TeacherAction,
    PostActionState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UntrustedBindingLabelEnvelopeV1 {
    pub schema: String,
    pub envelope_sha256: String,
    pub row_id_sha256: String,
    pub evidence_ref_sha256: String,
    pub frozen_graph_root_sha256: String,
    pub capture_receipt_root_sha256: String,
    pub parity_receipt_root_sha256: String,
    pub verifier_root_sha256: String,
    pub external_manifest_root_sha256: String,
    pub pre_action_wire_root_sha256: String,
    pub observed_relation_root_sha256: Option<String>,
    pub observation_source: BindingLabelObservationSourceV1,
    pub intervention_id: String,
    pub session_lineage_sha256: String,
    pub partition: BindingEvidencePartitionV1,
    pub captured_post_freeze: bool,
    pub label: BindingEvaluationLabelV1,
    pub expected_action_equivalence_sha256: Option<String>,
    pub baseline_outcome: BindingBaselineOutcomeV1,
}

impl UntrustedBindingLabelEnvelopeV1 {
    pub fn refresh_integrity_checksum(&mut self) -> Result<(), BindingPreregistrationErrorV1> {
        self.envelope_sha256 = binding_label_envelope_digest(self)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UntrustedBindingLabelManifestV1 {
    pub schema: String,
    pub external_manifest_root_sha256: String,
    pub freeze_watermark_root_sha256: String,
    pub support_lineage_root_sha256: String,
    pub future_lineage_root_sha256: String,
    pub envelopes: Vec<UntrustedBindingLabelEnvelopeV1>,
}

impl UntrustedBindingLabelManifestV1 {
    pub fn new(
        external_manifest_root_sha256: impl Into<String>,
        freeze_watermark_root_sha256: impl Into<String>,
        mut envelopes: Vec<UntrustedBindingLabelEnvelopeV1>,
    ) -> Result<Self, BindingPreregistrationErrorV1> {
        envelopes.sort_by(|left, right| left.row_id_sha256.cmp(&right.row_id_sha256));
        let support_lineages = lineage_set(&envelopes, BindingEvidencePartitionV1::Support);
        let future_lineages = lineage_set(&envelopes, BindingEvidencePartitionV1::Future);
        Ok(Self {
            schema: BINDING_LABEL_MANIFEST_SCHEMA_V1.to_owned(),
            external_manifest_root_sha256: external_manifest_root_sha256.into(),
            freeze_watermark_root_sha256: freeze_watermark_root_sha256.into(),
            support_lineage_root_sha256: sha256_json(&support_lineages)?,
            future_lineage_root_sha256: sha256_json(&future_lineages)?,
            envelopes,
        })
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BindingPreregistrationErrorV1> {
        serde_json::to_vec(self).map_err(|_| BindingPreregistrationErrorV1::Serialization)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedBindingLabelManifestRootV1 {
    manifest_bytes_sha256: String,
    external_manifest_root_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedBindingLabelSetV1 {
    manifest_bytes_sha256: String,
    external_manifest_root_sha256: String,
    freeze_watermark_root_sha256: String,
    support_lineage_root_sha256: String,
    future_lineage_root_sha256: String,
    positive_rows: usize,
    applicability_negative_rows: usize,
}

impl TrustedBindingLabelSetV1 {
    #[must_use]
    pub fn positive_rows(&self) -> usize {
        self.positive_rows
    }

    #[must_use]
    pub fn applicability_negative_rows(&self) -> usize {
        self.applicability_negative_rows
    }

    #[must_use]
    pub fn manifest_bytes_sha256(&self) -> &str {
        &self.manifest_bytes_sha256
    }

    #[must_use]
    pub fn external_manifest_root_sha256(&self) -> &str {
        &self.external_manifest_root_sha256
    }

    #[must_use]
    pub fn freeze_watermark_root_sha256(&self) -> &str {
        &self.freeze_watermark_root_sha256
    }

    #[must_use]
    pub fn support_lineage_root_sha256(&self) -> &str {
        &self.support_lineage_root_sha256
    }

    #[must_use]
    pub fn future_lineage_root_sha256(&self) -> &str {
        &self.future_lineage_root_sha256
    }
}

pub fn resolve_trusted_binding_label_set_v1(
    manifest_bytes: &[u8],
    expected_root: &TrustedBindingLabelManifestRootV1,
) -> Result<TrustedBindingLabelSetV1, BindingPreregistrationErrorV1> {
    if !is_sha256(&expected_root.manifest_bytes_sha256)
        || !is_sha256(&expected_root.external_manifest_root_sha256)
        || sha256_bytes(manifest_bytes) != expected_root.manifest_bytes_sha256
    {
        return Err(BindingPreregistrationErrorV1::InvalidTrustRoot);
    }
    let manifest: UntrustedBindingLabelManifestV1 = serde_json::from_slice(manifest_bytes)
        .map_err(|_| BindingPreregistrationErrorV1::InvalidTrustRoot)?;
    if manifest.canonical_bytes()? != manifest_bytes
        || manifest.schema != BINDING_LABEL_MANIFEST_SCHEMA_V1
        || manifest.external_manifest_root_sha256 != expected_root.external_manifest_root_sha256
        || !is_sha256(&manifest.freeze_watermark_root_sha256)
        || manifest.envelopes.is_empty()
        || manifest.envelopes.len() > MAX_BINDING_LABEL_ENVELOPES_V1
        || manifest
            .envelopes
            .windows(2)
            .any(|pair| pair[0].row_id_sha256 >= pair[1].row_id_sha256)
    {
        return Err(BindingPreregistrationErrorV1::InvalidTrustRoot);
    }

    let mut evidence_refs = BTreeSet::new();
    for envelope in &manifest.envelopes {
        validate_binding_label_envelope(envelope, &manifest.external_manifest_root_sha256)?;
        if !evidence_refs.insert(envelope.evidence_ref_sha256.clone()) {
            return Err(BindingPreregistrationErrorV1::DuplicateRow);
        }
    }

    let support_lineages = lineage_set(&manifest.envelopes, BindingEvidencePartitionV1::Support);
    let future_lineages = lineage_set(&manifest.envelopes, BindingEvidencePartitionV1::Future);
    if !support_lineages.is_disjoint(&future_lineages) {
        return Err(BindingPreregistrationErrorV1::LineageOverlap);
    }
    if sha256_json(&support_lineages)? != manifest.support_lineage_root_sha256
        || sha256_json(&future_lineages)? != manifest.future_lineage_root_sha256
    {
        return Err(BindingPreregistrationErrorV1::InvalidManifest);
    }

    let support_positive = count_labels(
        &manifest.envelopes,
        BindingEvidencePartitionV1::Support,
        BindingEvaluationLabelV1::Positive,
    );
    let future_positive = count_labels(
        &manifest.envelopes,
        BindingEvidencePartitionV1::Future,
        BindingEvaluationLabelV1::Positive,
    );
    let support_negative = count_labels(
        &manifest.envelopes,
        BindingEvidencePartitionV1::Support,
        BindingEvaluationLabelV1::ApplicabilityNegative,
    );
    let future_negative = count_labels(
        &manifest.envelopes,
        BindingEvidencePartitionV1::Future,
        BindingEvaluationLabelV1::ApplicabilityNegative,
    );
    if support_positive < MIN_BINDING_POSITIVE_ROWS_PER_PARTITION_V1
        || future_positive < MIN_BINDING_POSITIVE_ROWS_PER_PARTITION_V1
    {
        return Err(BindingPreregistrationErrorV1::MissingPositiveDenominator);
    }
    if support_negative < MIN_BINDING_APPLICABILITY_NEGATIVE_ROWS_PER_PARTITION_V1
        || future_negative < MIN_BINDING_APPLICABILITY_NEGATIVE_ROWS_PER_PARTITION_V1
    {
        return Err(BindingPreregistrationErrorV1::MissingApplicabilityNegativeDenominator);
    }
    for partition in [
        BindingEvidencePartitionV1::Support,
        BindingEvidencePartitionV1::Future,
    ] {
        for intervention_id in BINDING_CAUSAL_INTERVENTION_IDS_V1 {
            let rows = manifest
                .envelopes
                .iter()
                .filter(|envelope| {
                    envelope.partition == partition && envelope.intervention_id == intervention_id
                })
                .count();
            if rows < MIN_BINDING_ROWS_PER_INTERVENTION_PER_PARTITION_V1 {
                return Err(BindingPreregistrationErrorV1::MissingInterventionDenominator);
            }
        }
    }

    Ok(TrustedBindingLabelSetV1 {
        manifest_bytes_sha256: expected_root.manifest_bytes_sha256.clone(),
        external_manifest_root_sha256: manifest.external_manifest_root_sha256,
        freeze_watermark_root_sha256: manifest.freeze_watermark_root_sha256,
        support_lineage_root_sha256: manifest.support_lineage_root_sha256,
        future_lineage_root_sha256: manifest.future_lineage_root_sha256,
        positive_rows: support_positive + future_positive,
        applicability_negative_rows: support_negative + future_negative,
    })
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingCausalHypothesisKindV1 {
    CandidateRelation,
    RelationNotObservable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingCausalHypothesisStatusV1 {
    Unproven,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingCausalHypothesisV1 {
    pub hypothesis_id: String,
    pub kind: BindingCausalHypothesisKindV1,
    pub candidate_relation: Option<String>,
    pub status: BindingCausalHypothesisStatusV1,
    pub observation_source: BindingLabelObservationSourceV1,
    pub teacher_action_allowed: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingInterventionPredictionV1 {
    PreservesBinding,
    ChangesBinding,
    AmbiguousBinding,
    NotApplicable,
    InsufficientEvidence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingCausalInterventionV1 {
    pub intervention_id: String,
    pub manipulated_factor: String,
    pub held_constant: Vec<String>,
    pub h1_prediction: BindingInterventionPredictionV1,
    pub null_prediction: BindingInterventionPredictionV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingTrustedLabelContractV1 {
    pub capture_receipt_root_required: bool,
    pub parity_receipt_root_required: bool,
    pub verifier_root_required: bool,
    pub external_manifest_root_required: bool,
    pub expected_digest_integrity_is_not_trust: bool,
    pub forged_recomputed_digest_rejected_by_external_root: bool,
    pub observation_source: BindingLabelObservationSourceV1,
    pub teacher_action_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingLineageSplitContractV1 {
    pub lineage_unit: String,
    pub support_future_overlap_max: usize,
    pub historical_to_future_rows: usize,
    pub future_must_be_post_freeze: bool,
    pub minimum_positive_rows_per_partition: usize,
    pub minimum_applicability_negative_rows_per_partition: usize,
    pub minimum_rows_per_intervention_per_partition: usize,
    pub censored_rows_may_be_negative: bool,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingEvidencePreregistrationV1 {
    pub schema: String,
    pub stop_id: String,
    pub missing_discriminator_exists: bool,
    pub resolving_relation_known: bool,
    pub candidate_hypotheses: Vec<BindingCausalHypothesisV1>,
    pub trusted_label_contract: BindingTrustedLabelContractV1,
    pub interventions: Vec<BindingCausalInterventionV1>,
    pub lineage_split: BindingLineageSplitContractV1,
    pub acquisition_run: bool,
    pub protocol_mode_compiled: bool,
    pub f4_started: bool,
    pub execution_authority: bool,
}

#[must_use]
pub fn binding_evidence_preregistration_v1() -> BindingEvidencePreregistrationV1 {
    let candidate_hypotheses = vec![
        BindingCausalHypothesisV1 {
            hypothesis_id: "H0".to_owned(),
            kind: BindingCausalHypothesisKindV1::RelationNotObservable,
            candidate_relation: None,
            status: BindingCausalHypothesisStatusV1::Unproven,
            observation_source: BindingLabelObservationSourceV1::PreActionWire,
            teacher_action_allowed: false,
        },
        BindingCausalHypothesisV1 {
            hypothesis_id: "H1".to_owned(),
            kind: BindingCausalHypothesisKindV1::CandidateRelation,
            candidate_relation: Some("parent_action_to_capability_instance".to_owned()),
            status: BindingCausalHypothesisStatusV1::Unproven,
            observation_source: BindingLabelObservationSourceV1::PreActionWire,
            teacher_action_allowed: false,
        },
    ];
    let interventions = vec![
        intervention(
            "I1",
            "candidate_order",
            &["parent_linkage", "candidate_values", "layout"],
            BindingInterventionPredictionV1::PreservesBinding,
        ),
        intervention(
            "I2",
            "parent_linkage",
            &["candidate_order", "candidate_values", "layout"],
            BindingInterventionPredictionV1::ChangesBinding,
        ),
        intervention(
            "I3",
            "same_type_decoy_presence",
            &["parent_linkage", "candidate_order", "layout"],
            BindingInterventionPredictionV1::PreservesBinding,
        ),
        intervention(
            "I4",
            "parent_completion_state",
            &["candidate_order", "candidate_values", "layout"],
            BindingInterventionPredictionV1::NotApplicable,
        ),
        intervention(
            "I5",
            "active_parent_cardinality",
            &["candidate_order", "candidate_values", "layout"],
            BindingInterventionPredictionV1::AmbiguousBinding,
        ),
        intervention(
            "I6",
            "matching_parent_presence",
            &["candidate_order", "candidate_values", "layout"],
            BindingInterventionPredictionV1::NotApplicable,
        ),
    ];
    BindingEvidencePreregistrationV1 {
        schema: BINDING_EVIDENCE_PREREGISTRATION_SCHEMA_V1.to_owned(),
        stop_id: "STOP-B1B0".to_owned(),
        missing_discriminator_exists: true,
        resolving_relation_known: false,
        candidate_hypotheses,
        trusted_label_contract: BindingTrustedLabelContractV1 {
            capture_receipt_root_required: true,
            parity_receipt_root_required: true,
            verifier_root_required: true,
            external_manifest_root_required: true,
            expected_digest_integrity_is_not_trust: true,
            forged_recomputed_digest_rejected_by_external_root: true,
            observation_source: BindingLabelObservationSourceV1::PreActionWire,
            teacher_action_allowed: false,
        },
        interventions,
        lineage_split: BindingLineageSplitContractV1 {
            lineage_unit: "session".to_owned(),
            support_future_overlap_max: 0,
            historical_to_future_rows: 0,
            future_must_be_post_freeze: true,
            minimum_positive_rows_per_partition: MIN_BINDING_POSITIVE_ROWS_PER_PARTITION_V1,
            minimum_applicability_negative_rows_per_partition:
                MIN_BINDING_APPLICABILITY_NEGATIVE_ROWS_PER_PARTITION_V1,
            minimum_rows_per_intervention_per_partition:
                MIN_BINDING_ROWS_PER_INTERVENTION_PER_PARTITION_V1,
            censored_rows_may_be_negative: false,
            status: "SEALED".to_owned(),
        },
        acquisition_run: false,
        protocol_mode_compiled: false,
        f4_started: false,
        execution_authority: false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingPreregistrationErrorV1 {
    InvalidDigest,
    InvalidEnvelope,
    InvalidManifest,
    InvalidTrustRoot,
    DuplicateRow,
    LineageOverlap,
    MissingPositiveDenominator,
    MissingApplicabilityNegativeDenominator,
    MissingInterventionDenominator,
    Serialization,
}

fn intervention(
    intervention_id: &str,
    manipulated_factor: &str,
    held_constant: &[&str],
    h1_prediction: BindingInterventionPredictionV1,
) -> BindingCausalInterventionV1 {
    BindingCausalInterventionV1 {
        intervention_id: intervention_id.to_owned(),
        manipulated_factor: manipulated_factor.to_owned(),
        held_constant: held_constant
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        h1_prediction,
        null_prediction: BindingInterventionPredictionV1::InsufficientEvidence,
    }
}

fn validate_binding_label_envelope(
    envelope: &UntrustedBindingLabelEnvelopeV1,
    external_manifest_root_sha256: &str,
) -> Result<(), BindingPreregistrationErrorV1> {
    let digests = [
        envelope.row_id_sha256.as_str(),
        envelope.evidence_ref_sha256.as_str(),
        envelope.frozen_graph_root_sha256.as_str(),
        envelope.capture_receipt_root_sha256.as_str(),
        envelope.parity_receipt_root_sha256.as_str(),
        envelope.verifier_root_sha256.as_str(),
        envelope.external_manifest_root_sha256.as_str(),
        envelope.pre_action_wire_root_sha256.as_str(),
        envelope.session_lineage_sha256.as_str(),
    ];
    if envelope.schema != BINDING_LABEL_ENVELOPE_SCHEMA_V1
        || envelope.external_manifest_root_sha256 != external_manifest_root_sha256
        || digests.into_iter().any(|value| !is_sha256(value))
        || envelope
            .observed_relation_root_sha256
            .as_deref()
            .is_some_and(|value| !is_sha256(value))
        || envelope.observation_source != BindingLabelObservationSourceV1::PreActionWire
        || !BINDING_CAUSAL_INTERVENTION_IDS_V1.contains(&envelope.intervention_id.as_str())
        || envelope.captured_post_freeze
            != (envelope.partition == BindingEvidencePartitionV1::Future)
        || (envelope.label == BindingEvaluationLabelV1::Positive
            && envelope
                .expected_action_equivalence_sha256
                .as_deref()
                .is_none_or(|value| !is_sha256(value)))
        || (envelope.label == BindingEvaluationLabelV1::ApplicabilityNegative
            && envelope.expected_action_equivalence_sha256.is_some())
        || binding_label_envelope_digest(envelope)? != envelope.envelope_sha256
    {
        return Err(BindingPreregistrationErrorV1::InvalidEnvelope);
    }
    Ok(())
}

fn binding_label_envelope_digest(
    envelope: &UntrustedBindingLabelEnvelopeV1,
) -> Result<String, BindingPreregistrationErrorV1> {
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        row_id_sha256: &'a str,
        evidence_ref_sha256: &'a str,
        frozen_graph_root_sha256: &'a str,
        capture_receipt_root_sha256: &'a str,
        parity_receipt_root_sha256: &'a str,
        verifier_root_sha256: &'a str,
        external_manifest_root_sha256: &'a str,
        pre_action_wire_root_sha256: &'a str,
        observed_relation_root_sha256: Option<&'a str>,
        observation_source: BindingLabelObservationSourceV1,
        intervention_id: &'a str,
        session_lineage_sha256: &'a str,
        partition: BindingEvidencePartitionV1,
        captured_post_freeze: bool,
        label: BindingEvaluationLabelV1,
        expected_action_equivalence_sha256: Option<&'a str>,
        baseline_outcome: BindingBaselineOutcomeV1,
    }

    sha256_json(&DigestFields {
        schema: &envelope.schema,
        row_id_sha256: &envelope.row_id_sha256,
        evidence_ref_sha256: &envelope.evidence_ref_sha256,
        frozen_graph_root_sha256: &envelope.frozen_graph_root_sha256,
        capture_receipt_root_sha256: &envelope.capture_receipt_root_sha256,
        parity_receipt_root_sha256: &envelope.parity_receipt_root_sha256,
        verifier_root_sha256: &envelope.verifier_root_sha256,
        external_manifest_root_sha256: &envelope.external_manifest_root_sha256,
        pre_action_wire_root_sha256: &envelope.pre_action_wire_root_sha256,
        observed_relation_root_sha256: envelope.observed_relation_root_sha256.as_deref(),
        observation_source: envelope.observation_source,
        intervention_id: &envelope.intervention_id,
        session_lineage_sha256: &envelope.session_lineage_sha256,
        partition: envelope.partition,
        captured_post_freeze: envelope.captured_post_freeze,
        label: envelope.label,
        expected_action_equivalence_sha256: envelope.expected_action_equivalence_sha256.as_deref(),
        baseline_outcome: envelope.baseline_outcome,
    })
}

fn count_labels(
    envelopes: &[UntrustedBindingLabelEnvelopeV1],
    partition: BindingEvidencePartitionV1,
    label: BindingEvaluationLabelV1,
) -> usize {
    envelopes
        .iter()
        .filter(|envelope| envelope.partition == partition && envelope.label == label)
        .count()
}

fn lineage_set(
    envelopes: &[UntrustedBindingLabelEnvelopeV1],
    partition: BindingEvidencePartitionV1,
) -> BTreeSet<String> {
    envelopes
        .iter()
        .filter(|envelope| envelope.partition == partition)
        .map(|envelope| envelope.session_lineage_sha256.clone())
        .collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256_json<T: Serialize>(value: &T) -> Result<String, BindingPreregistrationErrorV1> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_bytes(&bytes))
        .map_err(|_| BindingPreregistrationErrorV1::Serialization)
}

// The production manifest owner deliberately has no constructor in B1B0.
// Tests pin immutable bytes before exercising recomputed-label forgeries.
#[cfg(test)]
fn pin_trusted_binding_label_manifest_root(
    manifest_bytes: &[u8],
    external_manifest_root_sha256: &str,
) -> Result<TrustedBindingLabelManifestRootV1, BindingPreregistrationErrorV1> {
    if !is_sha256(external_manifest_root_sha256) {
        return Err(BindingPreregistrationErrorV1::InvalidTrustRoot);
    }
    Ok(TrustedBindingLabelManifestRootV1 {
        manifest_bytes_sha256: sha256_bytes(manifest_bytes),
        external_manifest_root_sha256: external_manifest_root_sha256.to_owned(),
    })
}

#[cfg(test)]
#[path = "binding_evidence_preregistration_tests.rs"]
mod tests;
