use super::*;

fn digest(seed: &str) -> String {
    sha256_bytes(seed.as_bytes())
}

fn envelope(
    seed: &str,
    intervention_id: &str,
    partition: BindingEvidencePartitionV1,
    label: BindingEvaluationLabelV1,
) -> UntrustedBindingLabelEnvelopeV1 {
    let expected = (label == BindingEvaluationLabelV1::Positive)
        .then(|| digest(&format!("{seed}:expected-action")));
    let mut envelope = UntrustedBindingLabelEnvelopeV1 {
        schema: BINDING_LABEL_ENVELOPE_SCHEMA_V1.to_owned(),
        envelope_sha256: String::new(),
        row_id_sha256: digest(&format!("{seed}:row")),
        evidence_ref_sha256: digest(&format!("{seed}:evidence")),
        frozen_graph_root_sha256: digest("frozen-graph"),
        capture_receipt_root_sha256: digest(&format!("{seed}:capture")),
        parity_receipt_root_sha256: digest(&format!("{seed}:parity")),
        verifier_root_sha256: digest(&format!("{seed}:verifier")),
        external_manifest_root_sha256: digest("external-manifest"),
        pre_action_wire_root_sha256: digest(&format!("{seed}:pre-action-wire")),
        observed_relation_root_sha256: Some(digest(&format!("{seed}:observed-relation"))),
        observation_source: BindingLabelObservationSourceV1::PreActionWire,
        intervention_id: intervention_id.to_owned(),
        session_lineage_sha256: digest(&format!("{seed}:session-lineage")),
        partition,
        captured_post_freeze: partition == BindingEvidencePartitionV1::Future,
        label,
        expected_action_equivalence_sha256: expected,
        baseline_outcome: BindingBaselineOutcomeV1::Exact,
    };
    envelope
        .refresh_integrity_checksum()
        .expect("fixture checksum");
    envelope
}

fn manifest() -> UntrustedBindingLabelManifestV1 {
    let mut envelopes = Vec::new();
    for partition in [
        BindingEvidencePartitionV1::Support,
        BindingEvidencePartitionV1::Future,
    ] {
        for intervention_id in BINDING_CAUSAL_INTERVENTION_IDS_V1 {
            for label in [
                BindingEvaluationLabelV1::Positive,
                BindingEvaluationLabelV1::ApplicabilityNegative,
            ] {
                let seed = format!("{partition:?}:{intervention_id}:{label:?}");
                envelopes.push(envelope(&seed, intervention_id, partition, label));
            }
        }
    }
    UntrustedBindingLabelManifestV1::new(
        digest("external-manifest"),
        digest("freeze-watermark"),
        envelopes,
    )
    .expect("fixture manifest")
}

#[test]
fn preregistration_keeps_h1_and_null_unproven() {
    let report = binding_evidence_preregistration_v1();
    assert!(report.missing_discriminator_exists);
    assert!(!report.resolving_relation_known);
    assert_eq!(report.candidate_hypotheses.len(), 2);
    assert!(report.candidate_hypotheses.iter().all(|hypothesis| {
        hypothesis.status == BindingCausalHypothesisStatusV1::Unproven
            && hypothesis.observation_source == BindingLabelObservationSourceV1::PreActionWire
            && !hypothesis.teacher_action_allowed
    }));
    assert_eq!(
        report.candidate_hypotheses[0].kind,
        BindingCausalHypothesisKindV1::RelationNotObservable
    );
    assert_eq!(
        report.candidate_hypotheses[1].candidate_relation.as_deref(),
        Some("parent_action_to_capability_instance")
    );
    assert!(!report.acquisition_run);
    assert!(!report.f4_started);
    assert!(!report.execution_authority);
}

#[test]
fn stop_b1b0_preregistration_matches_checked_in_golden_json() {
    let report = binding_evidence_preregistration_v1();
    let generated = format!(
        "{}\n",
        serde_json::to_string_pretty(&report).expect("STOP-B1B0 preregistration JSON")
    );
    let checked_in =
        include_str!("../../../plans/effect-law-unification-v1/STOP_B1B0_PREREGISTRATION.json");

    assert_eq!(generated, checked_in);
}

#[test]
fn all_six_causal_interventions_are_frozen() {
    let report = binding_evidence_preregistration_v1();
    assert_eq!(report.interventions.len(), 6);
    assert_eq!(
        report
            .interventions
            .iter()
            .map(|intervention| intervention.manipulated_factor.as_str())
            .collect::<Vec<_>>(),
        vec![
            "candidate_order",
            "parent_linkage",
            "same_type_decoy_presence",
            "parent_completion_state",
            "active_parent_cardinality",
            "matching_parent_presence",
        ]
    );
    assert!(report.interventions.iter().all(|intervention| {
        intervention.null_prediction == BindingInterventionPredictionV1::InsufficientEvidence
    }));
}

#[test]
fn external_root_rejects_recomputed_expected_digest_forgery() {
    let manifest = manifest();
    let original_bytes = manifest.canonical_bytes().expect("canonical manifest");
    let pinned =
        pin_trusted_binding_label_manifest_root(&original_bytes, &digest("external-manifest"))
            .expect("external pin");
    let trusted =
        resolve_trusted_binding_label_set_v1(&original_bytes, &pinned).expect("trusted label set");
    assert_eq!(trusted.positive_rows(), 12);
    assert_eq!(trusted.applicability_negative_rows(), 12);

    let mut forged = manifest;
    let positive = forged
        .envelopes
        .iter_mut()
        .find(|envelope| envelope.label == BindingEvaluationLabelV1::Positive)
        .expect("positive envelope");
    positive.expected_action_equivalence_sha256 = Some(digest("forged-action"));
    positive
        .refresh_integrity_checksum()
        .expect("attacker recomputes checksum");
    let forged_bytes = forged.canonical_bytes().expect("forged canonical bytes");
    assert_eq!(
        resolve_trusted_binding_label_set_v1(&forged_bytes, &pinned),
        Err(BindingPreregistrationErrorV1::InvalidTrustRoot)
    );
}

#[test]
fn support_and_future_session_lineages_must_be_disjoint() {
    let mut manifest = manifest();
    let support_lineage = manifest
        .envelopes
        .iter()
        .find(|envelope| envelope.partition == BindingEvidencePartitionV1::Support)
        .expect("support")
        .session_lineage_sha256
        .clone();
    let future = manifest
        .envelopes
        .iter_mut()
        .find(|envelope| envelope.partition == BindingEvidencePartitionV1::Future)
        .expect("future");
    future.session_lineage_sha256 = support_lineage;
    future.refresh_integrity_checksum().expect("checksum");
    manifest.future_lineage_root_sha256 = sha256_json(&lineage_set(
        &manifest.envelopes,
        BindingEvidencePartitionV1::Future,
    ))
    .expect("future root");
    let bytes = manifest.canonical_bytes().expect("canonical bytes");
    let pinned =
        pin_trusted_binding_label_manifest_root(&bytes, &digest("external-manifest")).expect("pin");
    assert_eq!(
        resolve_trusted_binding_label_set_v1(&bytes, &pinned),
        Err(BindingPreregistrationErrorV1::LineageOverlap)
    );
}

#[test]
fn both_partitions_require_real_applicability_negative_rows() {
    let mut manifest = manifest();
    let future_negative = manifest
        .envelopes
        .iter()
        .position(|envelope| {
            envelope.partition == BindingEvidencePartitionV1::Future
                && envelope.label == BindingEvaluationLabelV1::ApplicabilityNegative
        })
        .expect("future applicability negative");
    manifest.envelopes.remove(future_negative);
    manifest.future_lineage_root_sha256 = sha256_json(&lineage_set(
        &manifest.envelopes,
        BindingEvidencePartitionV1::Future,
    ))
    .expect("future root");
    let bytes = manifest.canonical_bytes().expect("canonical bytes");
    let pinned =
        pin_trusted_binding_label_manifest_root(&bytes, &digest("external-manifest")).expect("pin");
    assert_eq!(
        resolve_trusted_binding_label_set_v1(&bytes, &pinned),
        Err(BindingPreregistrationErrorV1::MissingApplicabilityNegativeDenominator)
    );
}

#[test]
fn both_partitions_require_the_preregistered_positive_denominator() {
    let mut manifest = manifest();
    let support_positive = manifest
        .envelopes
        .iter()
        .position(|envelope| {
            envelope.partition == BindingEvidencePartitionV1::Support
                && envelope.label == BindingEvaluationLabelV1::Positive
        })
        .expect("support positive");
    manifest.envelopes.remove(support_positive);
    manifest.support_lineage_root_sha256 = sha256_json(&lineage_set(
        &manifest.envelopes,
        BindingEvidencePartitionV1::Support,
    ))
    .expect("support root");
    let bytes = manifest.canonical_bytes().expect("canonical bytes");
    let pinned =
        pin_trusted_binding_label_manifest_root(&bytes, &digest("external-manifest")).expect("pin");
    assert_eq!(
        resolve_trusted_binding_label_set_v1(&bytes, &pinned),
        Err(BindingPreregistrationErrorV1::MissingPositiveDenominator)
    );
}

#[test]
fn every_intervention_requires_rows_in_support_and_future() {
    let mut manifest = manifest();
    for envelope in &mut manifest.envelopes {
        if envelope.partition == BindingEvidencePartitionV1::Future
            && envelope.intervention_id == "I6"
        {
            envelope.intervention_id = "I5".to_owned();
            envelope.refresh_integrity_checksum().expect("checksum");
        }
    }
    let bytes = manifest.canonical_bytes().expect("canonical bytes");
    let pinned =
        pin_trusted_binding_label_manifest_root(&bytes, &digest("external-manifest")).expect("pin");
    assert_eq!(
        resolve_trusted_binding_label_set_v1(&bytes, &pinned),
        Err(BindingPreregistrationErrorV1::MissingInterventionDenominator)
    );
}

#[test]
fn post_action_or_teacher_observability_is_rejected() {
    for source in [
        BindingLabelObservationSourceV1::TeacherAction,
        BindingLabelObservationSourceV1::PostActionState,
    ] {
        let mut manifest = manifest();
        let envelope = manifest.envelopes.first_mut().expect("envelope");
        envelope.observation_source = source;
        envelope.refresh_integrity_checksum().expect("checksum");
        let bytes = manifest.canonical_bytes().expect("canonical bytes");
        let pinned = pin_trusted_binding_label_manifest_root(&bytes, &digest("external-manifest"))
            .expect("pin");
        assert_eq!(
            resolve_trusted_binding_label_set_v1(&bytes, &pinned),
            Err(BindingPreregistrationErrorV1::InvalidEnvelope)
        );
    }
}

#[test]
fn future_rows_must_be_captured_post_freeze() {
    let mut manifest = manifest();
    let future = manifest
        .envelopes
        .iter_mut()
        .find(|envelope| envelope.partition == BindingEvidencePartitionV1::Future)
        .expect("future envelope");
    future.captured_post_freeze = false;
    future.refresh_integrity_checksum().expect("checksum");
    let bytes = manifest.canonical_bytes().expect("canonical bytes");
    let pinned =
        pin_trusted_binding_label_manifest_root(&bytes, &digest("external-manifest")).expect("pin");
    assert_eq!(
        resolve_trusted_binding_label_set_v1(&bytes, &pinned),
        Err(BindingPreregistrationErrorV1::InvalidEnvelope)
    );
}
