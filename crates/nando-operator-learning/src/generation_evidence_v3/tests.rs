use nando_operator_kernel::{
    OperatorGenerationComponentRootsV3, canonical_json_sha256, seal_operator_generation_manifest_v3,
};

use super::*;

fn root(label: &str) -> String {
    canonical_json_sha256(&label).expect("root")
}

fn manifest(label: &str) -> nando_operator_kernel::OperatorGenerationManifestV3 {
    seal_operator_generation_manifest_v3(
        1,
        None,
        OperatorGenerationComponentRootsV3 {
            artifact_set_sha256: root(&format!("{label}:artifacts")),
            dispatch_index_sha256: root(&format!("{label}:index")),
            actor_program_sha256: root(&format!("{label}:actor")),
            renderer_program_sha256: root(&format!("{label}:renderer")),
            verifier_contract_sha256: root(&format!("{label}:verifier")),
            capability_contract_sha256: root(&format!("{label}:capability")),
            resource_budget_sha256: root(&format!("{label}:budget")),
        },
    )
    .expect("manifest")
}

fn observation(
    manifest: &nando_operator_kernel::OperatorGenerationManifestV3,
    sequence: u64,
    support_watermark_next_sequence: u64,
    support_freeze_sha256: Option<&str>,
    lineage: &str,
    outcome: GenerationLearningOutcomeV3,
) -> GenerationEvidenceObservationV3 {
    seal_generation_evidence_observation_v3(GenerationEvidenceObservationInputV3 {
        generation_id_sha256: manifest.generation_id_sha256().to_owned(),
        capture_sequence: sequence,
        support_watermark_next_sequence,
        support_freeze_sha256: support_freeze_sha256.map(str::to_owned),
        lineage_root_sha256: root(&format!("{lineage}:lineage")),
        event_root_sha256: root(&format!("{lineage}:{sequence}:event")),
        request_root_sha256: root(&format!("{lineage}:{sequence}:request")),
        verifier_receipt_root_sha256: root(&format!("{lineage}:{sequence}:receipt")),
        outcome,
    })
    .expect("observation")
}

#[test]
fn frozen_partitions_restart_byte_identically_without_changing_generation() {
    let manifest = manifest("generation-a");
    let mut ledger = GenerationEvidenceLedgerV3::new(&manifest);
    ledger
        .append_support(observation(
            &manifest,
            1,
            10,
            None,
            "support-a",
            GenerationLearningOutcomeV3::VerifiedPass,
        ))
        .expect("support a");
    ledger
        .append_support(observation(
            &manifest,
            2,
            10,
            None,
            "support-b",
            GenerationLearningOutcomeV3::ApplicabilityNegative,
        ))
        .expect("support b");
    let freeze_sha256 = ledger
        .freeze_support(10, root("support watermark"))
        .expect("freeze")
        .freeze_sha256()
        .to_owned();
    let root_before_future = ledger.evidence_root_sha256().expect("root before");
    ledger
        .append_future(observation(
            &manifest,
            10,
            10,
            Some(&freeze_sha256),
            "future-a",
            GenerationLearningOutcomeV3::VerifiedPass,
        ))
        .expect("future a");
    ledger
        .append_future(observation(
            &manifest,
            11,
            10,
            Some(&freeze_sha256),
            "future-b",
            GenerationLearningOutcomeV3::Censored(GenerationCensoredReasonV3::Timeout),
        ))
        .expect("future b");

    assert_ne!(
        ledger.evidence_root_sha256().expect("root"),
        root_before_future
    );
    assert_eq!(
        ledger.generation_id_sha256(),
        manifest.generation_id_sha256()
    );
    assert_eq!(ledger.accounting().support_rows, 2);
    assert_eq!(ledger.accounting().future_rows, 2);
    assert_eq!(ledger.accounting().censored_rows, 1);
    assert!(!ledger.execution_authority());

    let bytes = ledger.canonical_bytes().expect("bytes");
    let restored =
        GenerationEvidenceLedgerV3::from_canonical_bytes(&bytes, &manifest).expect("restore");
    assert_eq!(restored.canonical_bytes().expect("restored bytes"), bytes);
    assert_eq!(
        restored.evidence_root_sha256(),
        ledger.evidence_root_sha256()
    );
}

#[test]
fn support_cannot_be_relabelled_as_future_or_cross_the_watermark() {
    let manifest = manifest("generation-b");
    let mut ledger = GenerationEvidenceLedgerV3::new(&manifest);
    ledger
        .append_support(observation(
            &manifest,
            1,
            10,
            None,
            "same-lineage",
            GenerationLearningOutcomeV3::VerifiedPass,
        ))
        .expect("support");
    let freeze_sha256 = ledger
        .freeze_support(10, root("watermark"))
        .expect("freeze")
        .freeze_sha256()
        .to_owned();

    assert_eq!(
        ledger.append_support(observation(
            &manifest,
            2,
            10,
            None,
            "late-support",
            GenerationLearningOutcomeV3::VerifiedPass,
        )),
        Err(GenerationEvidenceErrorV3::SupportClosed)
    );
    assert_eq!(
        ledger.append_future(observation(
            &manifest,
            9,
            10,
            Some(&freeze_sha256),
            "future-before-watermark",
            GenerationLearningOutcomeV3::VerifiedPass,
        )),
        Err(GenerationEvidenceErrorV3::BeforeWatermark)
    );
    assert_eq!(
        ledger.append_future(observation(
            &manifest,
            10,
            10,
            Some(&freeze_sha256),
            "same-lineage",
            GenerationLearningOutcomeV3::VerifiedPass,
        )),
        Err(GenerationEvidenceErrorV3::CrossPartitionLineage)
    );
    assert_eq!(ledger.support().len(), 1);
    assert!(ledger.future().is_empty());
}

#[test]
fn censored_outcomes_have_no_semantic_update() {
    assert_eq!(
        GenerationLearningOutcomeV3::VerifiedPass.semantic_update(),
        Some(GenerationSemanticUpdateV3::PositiveReinforcement)
    );
    assert_eq!(
        GenerationLearningOutcomeV3::ApplicabilityNegative.semantic_update(),
        Some(GenerationSemanticUpdateV3::ApplicabilityCounterWave)
    );
    assert_eq!(
        GenerationLearningOutcomeV3::HardContradiction.semantic_update(),
        Some(GenerationSemanticUpdateV3::StructuralRevision)
    );
    assert_eq!(
        GenerationLearningOutcomeV3::Censored(GenerationCensoredReasonV3::BudgetExhausted)
            .semantic_update(),
        None
    );
}

#[test]
fn restart_rejects_tamper_and_foreign_generation() {
    let generation_manifest = manifest("generation-c");
    let mut ledger = GenerationEvidenceLedgerV3::new(&generation_manifest);
    ledger
        .append_support(observation(
            &generation_manifest,
            1,
            2,
            None,
            "support",
            GenerationLearningOutcomeV3::VerifiedPass,
        ))
        .expect("support");
    ledger
        .freeze_support(2, root("watermark-c"))
        .expect("freeze");
    let bytes = ledger.canonical_bytes().expect("bytes");

    let mut tampered = bytes.clone();
    let offset = tampered.len() / 2;
    tampered[offset] ^= 1;
    assert!(
        GenerationEvidenceLedgerV3::from_canonical_bytes(&tampered, &generation_manifest).is_err()
    );
    assert_eq!(
        GenerationEvidenceLedgerV3::from_canonical_bytes(&bytes, &manifest("foreign")).err(),
        Some(GenerationEvidenceErrorV3::InvalidGeneration)
    );
}

#[test]
fn duplicate_event_request_and_receipt_roots_are_independently_rejected() {
    let generation_manifest = manifest("generation-d");
    let mut ledger = GenerationEvidenceLedgerV3::new(&generation_manifest);
    let first = observation(
        &generation_manifest,
        1,
        10,
        None,
        "first",
        GenerationLearningOutcomeV3::VerifiedPass,
    );
    ledger.append_support(first.clone()).expect("first");

    let mut duplicate_event = observation(
        &generation_manifest,
        2,
        10,
        None,
        "event-copy",
        GenerationLearningOutcomeV3::VerifiedPass,
    );
    duplicate_event.event_root_sha256 = first.event_root_sha256.clone();
    assert_eq!(
        ledger.append_support(duplicate_event),
        Err(GenerationEvidenceErrorV3::DuplicateEvent)
    );

    let mut duplicate_request = observation(
        &generation_manifest,
        2,
        10,
        None,
        "request-copy",
        GenerationLearningOutcomeV3::VerifiedPass,
    );
    duplicate_request.request_root_sha256 = first.request_root_sha256.clone();
    assert_eq!(
        ledger.append_support(duplicate_request),
        Err(GenerationEvidenceErrorV3::DuplicateRequest)
    );

    let mut duplicate_receipt = observation(
        &generation_manifest,
        2,
        10,
        None,
        "receipt-copy",
        GenerationLearningOutcomeV3::VerifiedPass,
    );
    duplicate_receipt.verifier_receipt_root_sha256 = first.verifier_receipt_root_sha256.clone();
    assert_eq!(
        ledger.append_support(duplicate_receipt),
        Err(GenerationEvidenceErrorV3::DuplicateReceipt)
    );
}

#[test]
fn future_requires_a_nonempty_freeze_and_restart_is_bounded() {
    let generation_manifest = manifest("generation-e");
    let mut ledger = GenerationEvidenceLedgerV3::new(&generation_manifest);
    assert_eq!(
        ledger.append_future(observation(
            &generation_manifest,
            1,
            2,
            Some(&root("not-yet-frozen")),
            "early-future",
            GenerationLearningOutcomeV3::VerifiedPass,
        )),
        Err(GenerationEvidenceErrorV3::SupportNotFrozen)
    );
    assert_eq!(
        ledger.freeze_support(2, root("empty-watermark")).err(),
        Some(GenerationEvidenceErrorV3::EmptySupport)
    );
    assert_eq!(
        GenerationEvidenceLedgerV3::from_canonical_bytes(
            &vec![0; GENERATION_EVIDENCE_MAX_BYTES_V3 + 1],
            &generation_manifest,
        )
        .err(),
        Some(GenerationEvidenceErrorV3::LedgerBudgetExhausted)
    );
}
