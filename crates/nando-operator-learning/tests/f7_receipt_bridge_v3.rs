#[allow(dead_code)]
#[path = "../../nando-operator-proof/tests/f6_support/mod.rs"]
mod f6_support;

use f6_support::{finish_handoff_v3, handoff_v3, request_payload_v3};
use nando_operator_kernel::{
    GenerationEvidencePartitionV3, OperatorGenerationComponentRootsV3, RuntimeProjectionV3,
    canonical_json_sha256, executable_artifact_set_sha256_v3, seal_operator_generation_manifest_v3,
};
use nando_operator_learning::{
    GenerationEvidenceErrorV3, GenerationEvidenceLedgerV3, GenerationLearningOutcomeV3,
};
use nando_operator_proof::{
    generation_receipt_v3::{
        GenerationVerifierReceiptInputV3, seal_generation_verifier_receipt_v3,
    },
    independent_verifier_v3::{
        IndependentVerifierArtifactSetV3, IndependentVerifierBudgetV3, IndependentVerifierInputV3,
        IndependentVerifierReceiptV3, verify_operator_result_v3,
    },
};
use nando_operator_runtime::compile_structural_dispatch_index_v3;

fn root(label: &str) -> String {
    canonical_json_sha256(&label).expect("root")
}

fn verify(handoff: &f6_support::F5HandoffV3, output: &str) -> IndependentVerifierReceiptV3 {
    let artifact_set =
        IndependentVerifierArtifactSetV3::new(&handoff.artifacts).expect("artifact set");
    let input = IndependentVerifierInputV3::new(
        &handoff.request_sha256,
        RuntimeProjectionV3::Responses,
        &handoff.payload_bytes,
        &artifact_set,
        &handoff.action,
        output,
    )
    .expect("input");
    verify_operator_result_v3(&input, IndependentVerifierBudgetV3::default()).expect("receipt")
}

fn generation_manifest(
    handoff: &f6_support::F5HandoffV3,
    actor_label: &str,
) -> nando_operator_kernel::OperatorGenerationManifestV3 {
    let index = compile_structural_dispatch_index_v3(&handoff.artifacts).expect("index");
    seal_operator_generation_manifest_v3(
        1,
        None,
        OperatorGenerationComponentRootsV3 {
            artifact_set_sha256: executable_artifact_set_sha256_v3(&handoff.artifacts)
                .expect("artifact root"),
            dispatch_index_sha256: index.index_sha256().to_owned(),
            actor_program_sha256: root(actor_label),
            renderer_program_sha256: root("renderer"),
            verifier_contract_sha256: root("verifier"),
            capability_contract_sha256: root("capability"),
            resource_budget_sha256: root("budget"),
        },
    )
    .expect("manifest")
}

#[test]
fn f6_receipts_cross_the_generation_ledger_without_authority_or_relabelling() {
    let support = handoff_v3("continue_session", "handle", "CellA17", &[41]);
    let future = finish_handoff_v3(
        support.artifacts.clone(),
        "continue TaskB22".to_owned(),
        request_payload_v3("resume_task", "ticket", "TaskB22"),
    );
    let rejected = finish_handoff_v3(
        support.artifacts.clone(),
        "continue JobC33".to_owned(),
        request_payload_v3("resume_job", "job", "JobC33"),
    );
    let manifest = generation_manifest(&support, "actor");
    let support_f6 = verify(&support, &support.actor_output);
    let support_envelope = seal_generation_verifier_receipt_v3(
        &manifest,
        GenerationVerifierReceiptInputV3 {
            partition: GenerationEvidencePartitionV3::Support,
            capture_sequence: 1,
            support_watermark_next_sequence: 10,
            support_freeze_sha256: None,
            lineage_root_sha256: root("support lineage"),
            event_root_sha256: root("support event"),
        },
        &support_f6,
    )
    .expect("support envelope");
    let mut ledger = GenerationEvidenceLedgerV3::new(&manifest);
    ledger
        .append_generation_verifier_receipt(
            &support_envelope,
            GenerationLearningOutcomeV3::VerifiedPass,
        )
        .expect("support append");
    let freeze_sha256 = ledger
        .freeze_support(10, root("watermark"))
        .expect("freeze")
        .freeze_sha256()
        .to_owned();

    let future_f6 = verify(&future, &future.actor_output);
    let future_envelope = seal_generation_verifier_receipt_v3(
        &manifest,
        GenerationVerifierReceiptInputV3 {
            partition: GenerationEvidencePartitionV3::Future,
            capture_sequence: 10,
            support_watermark_next_sequence: 10,
            support_freeze_sha256: Some(freeze_sha256.clone()),
            lineage_root_sha256: root("future lineage"),
            event_root_sha256: root("future event"),
        },
        &future_f6,
    )
    .expect("future envelope");
    ledger
        .append_generation_verifier_receipt(
            &future_envelope,
            GenerationLearningOutcomeV3::VerifiedPass,
        )
        .expect("future append");

    let rejected_f6 = verify(&rejected, "wrong-output");
    let rejected_envelope = seal_generation_verifier_receipt_v3(
        &manifest,
        GenerationVerifierReceiptInputV3 {
            partition: GenerationEvidencePartitionV3::Future,
            capture_sequence: 11,
            support_watermark_next_sequence: 10,
            support_freeze_sha256: Some(freeze_sha256),
            lineage_root_sha256: root("reject lineage"),
            event_root_sha256: root("reject event"),
        },
        &rejected_f6,
    )
    .expect("reject envelope");
    assert_eq!(
        ledger.append_generation_verifier_receipt(
            &rejected_envelope,
            GenerationLearningOutcomeV3::VerifiedPass,
        ),
        Err(GenerationEvidenceErrorV3::VerifierOutcomeMismatch)
    );
    ledger
        .append_generation_verifier_receipt(
            &rejected_envelope,
            GenerationLearningOutcomeV3::HardContradiction,
        )
        .expect("hard contradiction append");

    assert_eq!(ledger.accounting().support_rows, 1);
    assert_eq!(ledger.accounting().future_rows, 2);
    assert_eq!(ledger.accounting().positive_rows, 2);
    assert_eq!(ledger.accounting().hard_contradiction_rows, 1);
    assert!(!ledger.execution_authority());

    let foreign_manifest = generation_manifest(&support, "different actor");
    let foreign_envelope = seal_generation_verifier_receipt_v3(
        &foreign_manifest,
        GenerationVerifierReceiptInputV3 {
            partition: GenerationEvidencePartitionV3::Future,
            capture_sequence: 12,
            support_watermark_next_sequence: 10,
            support_freeze_sha256: Some(root("foreign freeze")),
            lineage_root_sha256: root("foreign lineage"),
            event_root_sha256: root("foreign event"),
        },
        &support_f6,
    )
    .expect("foreign envelope");
    assert_eq!(
        ledger.append_generation_verifier_receipt(
            &foreign_envelope,
            GenerationLearningOutcomeV3::VerifiedPass,
        ),
        Err(GenerationEvidenceErrorV3::InvalidGeneration)
    );
}
