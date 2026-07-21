use super::*;
use crate::capture_provenance::CaptureRecordCommitment;

fn digest(seed: &str) -> String {
    sha256_bytes(seed.as_bytes())
}

struct Fixture {
    manifest: UntrustedBindingLabelManifestV1,
    watermark: UntrustedBindingCaptureWatermarkV1,
}

fn envelope(
    seed: &str,
    intervention_id: &str,
    partition: BindingEvidencePartitionV1,
    label: BindingEvaluationLabelV1,
    record: &CaptureRecordCommitment,
    receipt: &CaptureEvidenceReceipt,
) -> UntrustedBindingLabelEnvelopeV1 {
    let expected = (label == BindingEvaluationLabelV1::Positive)
        .then(|| digest(&format!("{seed}:expected-action")));
    let mut envelope = UntrustedBindingLabelEnvelopeV1 {
        schema: BINDING_LABEL_ENVELOPE_SCHEMA_V1.to_owned(),
        envelope_sha256: String::new(),
        row_id_sha256: digest(&format!("{seed}:row")),
        evidence_ref_sha256: digest(&format!("{seed}:evidence")),
        frozen_graph_root_sha256: digest("frozen-graph"),
        capture_receipt_root_sha256: receipt.records_root_sha256.clone(),
        capture_sequence: record.sequence,
        capture_record_sha256: record.record_sha256.clone(),
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

fn fixture() -> Fixture {
    let mut envelopes = Vec::new();
    let mut capture_receipts = Vec::new();
    let mut records = Vec::new();
    let mut sequence = 0_u64;
    let mut support_record_count = 0;
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
                let record = CaptureRecordCommitment {
                    sequence,
                    record_sha256: digest(&format!("{seed}:capture-record")),
                };
                let receipt =
                    CaptureEvidenceReceipt::new(vec![record.clone()]).expect("capture receipt");
                let row = envelope(&seed, intervention_id, partition, label, &record, &receipt);
                capture_receipts.push(BindingCaptureReceiptEntryV1 {
                    evidence_ref_sha256: row.evidence_ref_sha256.clone(),
                    receipt,
                });
                records.push(record);
                envelopes.push(row);
                sequence += 1;
            }
        }
        if partition == BindingEvidencePartitionV1::Support {
            support_record_count = records.len();
        }
    }

    let watermark_index = CaptureCommitmentIndex::new(records[..support_record_count].to_vec())
        .expect("watermark index");
    let watermark = UntrustedBindingCaptureWatermarkV1::new(watermark_index).expect("watermark");
    let watermark_bytes = watermark.canonical_bytes().expect("watermark bytes");
    let capture_index = CaptureCommitmentIndex::new(records).expect("capture index");
    let manifest = UntrustedBindingLabelManifestV1::new(
        digest("external-manifest"),
        sha256_bytes(&watermark_bytes),
        capture_index,
        capture_receipts,
        envelopes,
    )
    .expect("fixture manifest");
    Fixture {
        manifest,
        watermark,
    }
}

fn manifest_bytes(fixture: &Fixture) -> Vec<u8> {
    fixture
        .manifest
        .canonical_bytes()
        .expect("canonical manifest")
}

fn watermark_bytes(fixture: &Fixture) -> Vec<u8> {
    fixture
        .watermark
        .canonical_bytes()
        .expect("canonical watermark")
}

fn resolve_fixture(
    fixture: &Fixture,
) -> Result<TrustedBindingLabelSetV1, BindingPreregistrationErrorV1> {
    let manifest_bytes = manifest_bytes(fixture);
    let watermark_bytes = watermark_bytes(fixture);
    let manifest_root =
        pin_trusted_binding_label_manifest_root(&manifest_bytes, &digest("external-manifest"))?;
    let watermark_root = pin_trusted_binding_capture_watermark_root(&watermark_bytes)?;
    resolve_trusted_binding_label_set_v1(
        &manifest_bytes,
        &manifest_root,
        &watermark_bytes,
        &watermark_root,
    )
}

fn refresh_lineage_roots(manifest: &mut UntrustedBindingLabelManifestV1) {
    manifest.support_lineage_root_sha256 = sha256_json(&lineage_set(
        &manifest.envelopes,
        BindingEvidencePartitionV1::Support,
    ))
    .expect("support lineage root");
    manifest.future_lineage_root_sha256 = sha256_json(&lineage_set(
        &manifest.envelopes,
        BindingEvidencePartitionV1::Future,
    ))
    .expect("future lineage root");
    manifest
        .envelopes
        .sort_by(|left, right| left.row_id_sha256.cmp(&right.row_id_sha256));
}

fn remove_envelope(fixture: &mut Fixture, index: usize) {
    let evidence_ref = fixture.manifest.envelopes[index]
        .evidence_ref_sha256
        .clone();
    fixture.manifest.envelopes.remove(index);
    fixture
        .manifest
        .capture_receipts
        .retain(|entry| entry.evidence_ref_sha256 != evidence_ref);
    refresh_lineage_roots(&mut fixture.manifest);
}

#[test]
fn preregistration_keeps_h1_and_null_unproven() {
    let report = binding_evidence_preregistration_v1();
    assert!(report.missing_discriminator_exists);
    assert!(!report.resolving_relation_known);
    assert_eq!(report.stop_id, "STOP-B1B0R");
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
fn stop_b1b0r_preregistration_matches_checked_in_golden_json() {
    let report = binding_evidence_preregistration_v1();
    let generated = format!(
        "{}\n",
        serde_json::to_string_pretty(&report).expect("STOP-B1B0R preregistration JSON")
    );
    let checked_in =
        include_str!("../../../plans/effect-law-unification-v1/STOP_B1B0R_PREREGISTRATION.json");

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
fn valid_post_freeze_extension_resolves_with_independent_lineages() {
    let trusted = resolve_fixture(&fixture()).expect("trusted label set");
    assert_eq!(trusted.positive_rows(), 12);
    assert_eq!(trusted.applicability_negative_rows(), 12);
    assert_eq!(trusted.support_session_lineages(), 12);
    assert_eq!(trusted.future_session_lineages(), 12);
    assert!(is_sha256(trusted.capture_index_sha256()));
    assert!(is_sha256(trusted.capture_watermark_sha256()));
}

#[test]
fn external_root_rejects_recomputed_expected_digest_forgery() {
    let mut fixture = fixture();
    let original_bytes = manifest_bytes(&fixture);
    let watermark_bytes = watermark_bytes(&fixture);
    let manifest_root =
        pin_trusted_binding_label_manifest_root(&original_bytes, &digest("external-manifest"))
            .expect("external pin");
    let watermark_root =
        pin_trusted_binding_capture_watermark_root(&watermark_bytes).expect("watermark pin");

    let positive = fixture
        .manifest
        .envelopes
        .iter_mut()
        .find(|envelope| envelope.label == BindingEvaluationLabelV1::Positive)
        .expect("positive envelope");
    positive.expected_action_equivalence_sha256 = Some(digest("forged-action"));
    positive
        .refresh_integrity_checksum()
        .expect("attacker recomputes checksum");
    let forged_bytes = manifest_bytes(&fixture);
    assert_eq!(
        resolve_trusted_binding_label_set_v1(
            &forged_bytes,
            &manifest_root,
            &watermark_bytes,
            &watermark_root,
        ),
        Err(BindingPreregistrationErrorV1::InvalidTrustRoot)
    );
}

#[test]
fn external_watermark_rejects_recomputed_chronology_forgery() {
    let fixture = fixture();
    let mut forged_watermark = fixture.watermark.clone();
    let original_watermark_bytes = watermark_bytes(&fixture);
    let watermark_root = pin_trusted_binding_capture_watermark_root(&original_watermark_bytes)
        .expect("watermark pin");
    forged_watermark.next_sequence += 1;
    let forged_watermark_bytes = forged_watermark
        .canonical_bytes()
        .expect("forged watermark bytes");
    let manifest_bytes = manifest_bytes(&fixture);
    let manifest_root =
        pin_trusted_binding_label_manifest_root(&manifest_bytes, &digest("external-manifest"))
            .expect("manifest pin");
    assert_eq!(
        resolve_trusted_binding_label_set_v1(
            &manifest_bytes,
            &manifest_root,
            &forged_watermark_bytes,
            &watermark_root,
        ),
        Err(BindingPreregistrationErrorV1::InvalidWatermark)
    );
}

#[test]
fn support_and_future_session_lineages_must_be_disjoint() {
    let mut fixture = fixture();
    let support_lineage = fixture
        .manifest
        .envelopes
        .iter()
        .find(|envelope| envelope.partition == BindingEvidencePartitionV1::Support)
        .expect("support")
        .session_lineage_sha256
        .clone();
    let future = fixture
        .manifest
        .envelopes
        .iter_mut()
        .find(|envelope| envelope.partition == BindingEvidencePartitionV1::Future)
        .expect("future");
    future.session_lineage_sha256 = support_lineage;
    future.refresh_integrity_checksum().expect("checksum");
    refresh_lineage_roots(&mut fixture.manifest);
    assert_eq!(
        resolve_fixture(&fixture),
        Err(BindingPreregistrationErrorV1::LineageOverlap)
    );
}

#[test]
fn both_partitions_require_three_independent_sessions() {
    let mut fixture = fixture();
    let support_lineages = [digest("support-lineage-a"), digest("support-lineage-b")];
    let mut support_index = 0;
    for envelope in &mut fixture.manifest.envelopes {
        if envelope.partition == BindingEvidencePartitionV1::Support {
            envelope.session_lineage_sha256 = support_lineages[support_index % 2].clone();
            support_index += 1;
            envelope.refresh_integrity_checksum().expect("checksum");
        }
    }
    refresh_lineage_roots(&mut fixture.manifest);
    assert_eq!(
        resolve_fixture(&fixture),
        Err(BindingPreregistrationErrorV1::MissingSessionLineageDenominator)
    );
}

#[test]
fn one_session_cannot_repeat_the_same_intervention_vote() {
    let mut fixture = fixture();
    let first_index = fixture
        .manifest
        .envelopes
        .iter()
        .position(|envelope| {
            envelope.partition == BindingEvidencePartitionV1::Support
                && envelope.label == BindingEvaluationLabelV1::Positive
                && envelope.intervention_id == "I1"
        })
        .expect("first vote");
    let second_index = fixture
        .manifest
        .envelopes
        .iter()
        .position(|envelope| {
            envelope.partition == BindingEvidencePartitionV1::Support
                && envelope.label == BindingEvaluationLabelV1::Positive
                && envelope.intervention_id == "I2"
        })
        .expect("second vote");
    let first_lineage = fixture.manifest.envelopes[first_index]
        .session_lineage_sha256
        .clone();
    let second = &mut fixture.manifest.envelopes[second_index];
    second.intervention_id = "I1".to_owned();
    second.session_lineage_sha256 = first_lineage;
    second.refresh_integrity_checksum().expect("checksum");
    refresh_lineage_roots(&mut fixture.manifest);
    assert_eq!(
        resolve_fixture(&fixture),
        Err(BindingPreregistrationErrorV1::DuplicateLineageVote)
    );
}

#[test]
fn historical_capture_cannot_be_relabelled_as_future() {
    let mut fixture = fixture();
    let support = fixture
        .manifest
        .envelopes
        .iter_mut()
        .find(|envelope| envelope.partition == BindingEvidencePartitionV1::Support)
        .expect("support");
    support.partition = BindingEvidencePartitionV1::Future;
    support.captured_post_freeze = true;
    support.refresh_integrity_checksum().expect("checksum");
    refresh_lineage_roots(&mut fixture.manifest);
    assert_eq!(
        resolve_fixture(&fixture),
        Err(BindingPreregistrationErrorV1::InvalidCaptureChronology)
    );
}

#[test]
fn capture_receipt_must_join_the_committed_index_record() {
    let mut fixture = fixture();
    let entry = fixture
        .manifest
        .capture_receipts
        .first_mut()
        .expect("capture receipt");
    let sequence = entry.receipt.records[0].sequence;
    entry.receipt = CaptureEvidenceReceipt::new(vec![CaptureRecordCommitment {
        sequence,
        record_sha256: digest("forged-capture-record"),
    }])
    .expect("forged receipt");
    assert_eq!(
        resolve_fixture(&fixture),
        Err(BindingPreregistrationErrorV1::InvalidCaptureReceipt)
    );
}

#[test]
fn current_capture_index_must_extend_the_frozen_prefix() {
    let mut fixture = fixture();
    let records = fixture.manifest.capture_index.records[1..].to_vec();
    fixture.manifest.capture_index =
        CaptureCommitmentIndex::new(records).expect("shortened capture index");
    assert_eq!(
        resolve_fixture(&fixture),
        Err(BindingPreregistrationErrorV1::CaptureIndexNotExtension)
    );
}

#[test]
fn both_partitions_require_real_applicability_negative_rows() {
    let mut fixture = fixture();
    let future_negative = fixture
        .manifest
        .envelopes
        .iter()
        .position(|envelope| {
            envelope.partition == BindingEvidencePartitionV1::Future
                && envelope.label == BindingEvaluationLabelV1::ApplicabilityNegative
        })
        .expect("future applicability negative");
    remove_envelope(&mut fixture, future_negative);
    assert_eq!(
        resolve_fixture(&fixture),
        Err(BindingPreregistrationErrorV1::MissingApplicabilityNegativeDenominator)
    );
}

#[test]
fn both_partitions_require_the_preregistered_positive_denominator() {
    let mut fixture = fixture();
    let support_positive = fixture
        .manifest
        .envelopes
        .iter()
        .position(|envelope| {
            envelope.partition == BindingEvidencePartitionV1::Support
                && envelope.label == BindingEvaluationLabelV1::Positive
        })
        .expect("support positive");
    remove_envelope(&mut fixture, support_positive);
    assert_eq!(
        resolve_fixture(&fixture),
        Err(BindingPreregistrationErrorV1::MissingPositiveDenominator)
    );
}

#[test]
fn every_intervention_requires_rows_in_support_and_future() {
    let mut fixture = fixture();
    for envelope in &mut fixture.manifest.envelopes {
        if envelope.partition == BindingEvidencePartitionV1::Future
            && envelope.intervention_id == "I6"
        {
            envelope.intervention_id = "I5".to_owned();
            envelope.refresh_integrity_checksum().expect("checksum");
        }
    }
    assert_eq!(
        resolve_fixture(&fixture),
        Err(BindingPreregistrationErrorV1::MissingInterventionDenominator)
    );
}

#[test]
fn post_action_or_teacher_observability_is_rejected() {
    for source in [
        BindingLabelObservationSourceV1::TeacherAction,
        BindingLabelObservationSourceV1::PostActionState,
    ] {
        let mut fixture = fixture();
        let envelope = fixture.manifest.envelopes.first_mut().expect("envelope");
        envelope.observation_source = source;
        envelope.refresh_integrity_checksum().expect("checksum");
        assert_eq!(
            resolve_fixture(&fixture),
            Err(BindingPreregistrationErrorV1::InvalidEnvelope)
        );
    }
}

#[test]
fn future_rows_must_be_declared_post_freeze() {
    let mut fixture = fixture();
    let future = fixture
        .manifest
        .envelopes
        .iter_mut()
        .find(|envelope| envelope.partition == BindingEvidencePartitionV1::Future)
        .expect("future envelope");
    future.captured_post_freeze = false;
    future.refresh_integrity_checksum().expect("checksum");
    assert_eq!(
        resolve_fixture(&fixture),
        Err(BindingPreregistrationErrorV1::InvalidEnvelope)
    );
}
