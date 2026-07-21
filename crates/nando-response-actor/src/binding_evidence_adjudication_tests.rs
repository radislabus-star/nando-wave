use super::*;

const PREREGISTRATION: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B0R_PREREGISTRATION.json");
const B1A_REPORT: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1A_BINDING_EVIDENCE.json");
const SUPPORT_FREEZE: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_S_FREEZE.json");
const SUPPORT_WATERMARK: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_S_WATERMARK.json");
const FUTURE_FREEZE: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_F_FREEZE.json");
const FUTURE_EXTERNAL_RECEIPT: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_F_EXTERNAL_RECEIPT.json");
const GOLDEN_PHYSICAL_RECEIPTS: &[u8] = include_bytes!(
    "../../../plans/effect-law-unification-v1/STOP_B1B_PHYSICAL_LABEL_RECEIPTS.json"
);
const GOLDEN_LABEL_MANIFEST: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_LABEL_MANIFEST.json");
const GOLDEN_EXTERNAL_TRUST: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_EXTERNAL_LABEL_TRUST.json");
const GOLDEN_ADJUDICATION: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_ADJUDICATION.json");

struct Artifacts {
    physical: BindingPhysicalLabelReceiptSetV1,
    physical_bytes: Vec<u8>,
    manifest: UntrustedBindingLabelManifestV1,
    manifest_bytes: Vec<u8>,
    trust: BindingExternalLabelTrustReceiptV1,
    trust_bytes: Vec<u8>,
}

fn artifacts() -> Artifacts {
    let physical = observe_frozen_binding_labels_v1(
        SUPPORT_FREEZE,
        SUPPORT_WATERMARK,
        FUTURE_FREEZE,
        FUTURE_EXTERNAL_RECEIPT,
    )
    .expect("physical labels");
    let physical_bytes = physical.canonical_bytes().expect("physical bytes");
    let manifest = build_binding_label_manifest_v1(
        SUPPORT_FREEZE,
        SUPPORT_WATERMARK,
        FUTURE_FREEZE,
        FUTURE_EXTERNAL_RECEIPT,
        &physical,
    )
    .expect("label manifest");
    let manifest_bytes = manifest.canonical_bytes().expect("manifest bytes");
    let trust = seal_binding_external_label_trust_v1(
        PREREGISTRATION,
        B1A_REPORT,
        SUPPORT_FREEZE,
        SUPPORT_WATERMARK,
        FUTURE_FREEZE,
        FUTURE_EXTERNAL_RECEIPT,
        &physical_bytes,
        &manifest_bytes,
    )
    .expect("external trust receipt");
    let trust_bytes = trust.canonical_bytes().expect("trust bytes");
    Artifacts {
        physical,
        physical_bytes,
        manifest,
        manifest_bytes,
        trust,
        trust_bytes,
    }
}

fn adjudicate(fixture: &Artifacts) -> BindingCausalAdjudicationReportV1 {
    adjudicate_binding_hypotheses_v1(
        PREREGISTRATION,
        B1A_REPORT,
        SUPPORT_FREEZE,
        SUPPORT_WATERMARK,
        FUTURE_FREEZE,
        FUTURE_EXTERNAL_RECEIPT,
        &fixture.physical_bytes,
        &fixture.manifest_bytes,
        &fixture.trust_bytes,
    )
    .expect("B1B adjudication")
}

#[test]
fn frozen_support_and_future_replay_exactly_before_labels_exist() {
    let fixture = artifacts();
    assert_eq!(fixture.physical.receipts.len(), 24);
    assert_eq!(
        count_partition_labels(
            &fixture.physical.receipts,
            BindingEvidencePartitionV1::Support,
            BindingEvaluationLabelV1::Positive,
        ),
        6
    );
    assert_eq!(
        count_partition_labels(
            &fixture.physical.receipts,
            BindingEvidencePartitionV1::Future,
            BindingEvaluationLabelV1::ApplicabilityNegative,
        ),
        6
    );
    assert!(fixture.physical.receipts.iter().all(|receipt| {
        receipt.trials.iter().all(|trial| trial.verifier_agrees)
            && receipt.baseline_outcome == BindingBaselineOutcomeV1::Abstain
    }));
}

#[test]
fn trusted_adjudication_supports_h1_and_rejects_h0_without_compiling_f4() {
    let report = adjudicate(&artifacts());
    assert_eq!(
        report.h1_status,
        BindingHypothesisAdjudicationStatusV1::Supported
    );
    assert_eq!(
        report.h0_status,
        BindingHypothesisAdjudicationStatusV1::Rejected
    );
    assert_eq!(report.wrong_bindings, 0);
    assert_eq!(report.applicability_negative_accepts, 0);
    assert_eq!(report.parity_failures, 0);
    assert_eq!(report.b1a_ties_evaluated_against_relation, 86);
    assert_eq!(report.f4_status, "UNLOCKED_NOT_STARTED");
    assert!(!report.selector_compiled);
    assert!(!report.protocol_mode_compiled);
    assert!(!report.execution_authority);
}

#[test]
fn all_preregistered_interventions_match_on_support_and_unseen_future_layouts() {
    let report = adjudicate(&artifacts());
    assert_eq!(report.interventions.len(), 6);
    assert!(report.interventions.iter().all(|intervention| {
        intervention.support_rows == 2
            && intervention.future_rows == 2
            && intervention.prediction_matched
    }));
    assert_eq!(report.interventions[0].selected_parent_ordinals, vec![0]);
    assert_eq!(report.interventions[1].selected_parent_ordinals, vec![0, 1]);
    assert_eq!(
        report.interventions[4].observed_relation_states,
        vec![BindingPhysicalRelationStateV1::Ambiguous]
    );
}

#[test]
fn intervention_metadata_cannot_relabel_a_frozen_physical_row() {
    let mut fixture = artifacts();
    fixture.physical.receipts[0].intervention_id =
        if fixture.physical.receipts[0].intervention_id == "I6" {
            "I1".to_owned()
        } else {
            "I6".to_owned()
        };
    fixture.physical.receipts[0].receipt_sha256 =
        physical_label_receipt_digest(&fixture.physical.receipts[0]).expect("receipt digest");
    fixture.physical.receipt_sha256 =
        physical_receipt_set_digest(&fixture.physical).expect("set digest");
    assert!(matches!(
        build_binding_label_manifest_v1(
            SUPPORT_FREEZE,
            SUPPORT_WATERMARK,
            FUTURE_FREEZE,
            FUTURE_EXTERNAL_RECEIPT,
            &fixture.physical,
        ),
        Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt)
    ));
}

#[test]
fn recomputed_relation_and_receipt_are_rejected_by_original_external_root() {
    let fixture = artifacts();
    let mut forged = fixture.physical.clone();
    let first = &mut forged.receipts[0];
    first.observed_relation.requested_capability_action_sha256 = Some(
        first.observed_relation.candidates[1]
            .action_equivalence_sha256
            .clone(),
    );
    first.observed_relation.relation_root_sha256 =
        observed_relation_digest(&first.observed_relation).expect("relation digest");
    first.receipt_sha256 = physical_label_receipt_digest(first).expect("receipt digest");
    forged.receipt_sha256 = physical_receipt_set_digest(&forged).expect("set digest");
    let forged_bytes = forged.canonical_bytes().expect("forged physical bytes");
    assert_eq!(
        adjudicate_binding_hypotheses_v1(
            PREREGISTRATION,
            B1A_REPORT,
            SUPPORT_FREEZE,
            SUPPORT_WATERMARK,
            FUTURE_FREEZE,
            FUTURE_EXTERNAL_RECEIPT,
            &forged_bytes,
            &fixture.manifest_bytes,
            &fixture.trust_bytes,
        ),
        Err(BindingAdjudicationErrorV1::InvalidTrustReceipt)
    );
}

#[test]
fn recomputed_expected_action_manifest_is_rejected_by_original_external_root() {
    let fixture = artifacts();
    let mut forged = fixture.manifest.clone();
    let envelope = forged
        .envelopes
        .iter_mut()
        .find(|envelope| envelope.label == BindingEvaluationLabelV1::Positive)
        .expect("positive envelope");
    envelope.expected_action_equivalence_sha256 = Some(sha256_bytes(b"forged-action"));
    envelope
        .refresh_integrity_checksum()
        .expect("forged local checksum");
    let forged_bytes = forged.canonical_bytes().expect("forged manifest bytes");
    assert_eq!(
        adjudicate_binding_hypotheses_v1(
            PREREGISTRATION,
            B1A_REPORT,
            SUPPORT_FREEZE,
            SUPPORT_WATERMARK,
            FUTURE_FREEZE,
            FUTURE_EXTERNAL_RECEIPT,
            &fixture.physical_bytes,
            &forged_bytes,
            &fixture.trust_bytes,
        ),
        Err(BindingAdjudicationErrorV1::InvalidTrustReceipt)
    );
}

#[test]
fn trust_owner_is_bound_to_the_original_preregistration_and_b1a_denominator() {
    let fixture = artifacts();
    let mut changed = PREREGISTRATION.to_vec();
    changed.push(b' ');
    assert_eq!(
        adjudicate_binding_hypotheses_v1(
            &changed,
            B1A_REPORT,
            SUPPORT_FREEZE,
            SUPPORT_WATERMARK,
            FUTURE_FREEZE,
            FUTURE_EXTERNAL_RECEIPT,
            &fixture.physical_bytes,
            &fixture.manifest_bytes,
            &fixture.trust_bytes,
        ),
        Err(BindingAdjudicationErrorV1::InvalidTrustReceipt)
    );
}

#[test]
fn trust_receipt_has_no_authority_or_protocol_compiler_capability() {
    let fixture = artifacts();
    assert!(fixture.trust.expected_labels_joined);
    assert!(!fixture.trust.protocol_mode_compiled);
    assert!(!fixture.trust.execution_authority);
    assert_eq!(
        fixture.trust.external_manifest_root_sha256,
        fixture.physical.receipt_sha256
    );
}

#[test]
fn generated_b1b_artifacts_are_byte_identical_to_checked_in_goldens() {
    let fixture = artifacts();
    let report = adjudicate(&fixture);
    assert_eq!(fixture.physical_bytes, GOLDEN_PHYSICAL_RECEIPTS);
    assert_eq!(fixture.manifest_bytes, GOLDEN_LABEL_MANIFEST);
    assert_eq!(fixture.trust_bytes, GOLDEN_EXTERNAL_TRUST);
    assert_eq!(
        report.canonical_bytes().expect("adjudication bytes"),
        GOLDEN_ADJUDICATION
    );
}
