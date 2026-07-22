#[allow(dead_code)]
mod f6_support;

use f6_support::handoff_v3;
use nando_operator_kernel::{
    GenerationEvidencePartitionV3, OperatorGenerationComponentRootsV3, RuntimeProjectionV3,
    canonical_json_sha256, executable_artifact_set_sha256_v3, seal_operator_generation_manifest_v3,
};
use nando_operator_proof::{
    generation_receipt_v3::{
        GenerationVerifierReceiptErrorV3, GenerationVerifierReceiptInputV3,
        GenerationVerifierReceiptV3, seal_generation_verifier_receipt_v3,
    },
    independent_verifier_v3::{
        IndependentVerifierArtifactSetV3, IndependentVerifierBudgetV3, IndependentVerifierInputV3,
        IndependentVerifierReceiptV3, IndependentVerifierVerdictV3, verify_operator_result_v3,
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
) -> nando_operator_kernel::OperatorGenerationManifestV3 {
    let index = compile_structural_dispatch_index_v3(&handoff.artifacts).expect("index");
    seal_operator_generation_manifest_v3(
        1,
        None,
        OperatorGenerationComponentRootsV3 {
            artifact_set_sha256: executable_artifact_set_sha256_v3(&handoff.artifacts)
                .expect("artifact root"),
            dispatch_index_sha256: index.index_sha256().to_owned(),
            actor_program_sha256: root("actor"),
            renderer_program_sha256: root("renderer"),
            verifier_contract_sha256: root("verifier"),
            capability_contract_sha256: root("capability"),
            resource_budget_sha256: root("budget"),
        },
    )
    .expect("manifest")
}

#[test]
fn verified_f6_receipt_binds_to_generation_and_restarts_exactly() {
    let handoff = handoff_v3("continue_session", "handle", "CellA17", &[31]);
    let manifest = generation_manifest(&handoff);
    let f6 = verify(&handoff, &handoff.actor_output);
    let envelope = seal_generation_verifier_receipt_v3(
        &manifest,
        GenerationVerifierReceiptInputV3 {
            partition: GenerationEvidencePartitionV3::Support,
            capture_sequence: 1,
            support_watermark_next_sequence: 10,
            support_freeze_sha256: None,
            lineage_root_sha256: root("support lineage"),
            event_root_sha256: root("support event"),
        },
        &f6,
    )
    .expect("envelope");

    assert!(envelope.is_verified_pass());
    assert_eq!(
        envelope.f6_verdict(),
        IndependentVerifierVerdictV3::Verified
    );
    assert_eq!(envelope.f6_request_sha256(), handoff.request_sha256);
    assert_eq!(envelope.raw_payloads_persisted(), 0);
    assert!(!envelope.execution_authority());
    let bytes = envelope.canonical_bytes().expect("bytes");
    assert_eq!(
        GenerationVerifierReceiptV3::from_canonical_bytes(&bytes, &manifest, &f6).expect("restart"),
        envelope
    );
    let foreign_manifest = seal_operator_generation_manifest_v3(
        1,
        None,
        OperatorGenerationComponentRootsV3 {
            actor_program_sha256: root("foreign actor"),
            ..manifest.components().clone()
        },
    )
    .expect("foreign manifest");
    assert_eq!(
        GenerationVerifierReceiptV3::from_canonical_bytes(&bytes, &foreign_manifest, &f6).err(),
        Some(GenerationVerifierReceiptErrorV3::InvalidGeneration)
    );
}

#[test]
fn reject_receipt_remains_reject_and_cannot_be_replayed_as_verified() {
    let handoff = handoff_v3("continue_session", "handle", "CellA17", &[32]);
    let manifest = generation_manifest(&handoff);
    let rejected = verify(&handoff, "wrong-output");
    assert_eq!(
        rejected.verdict(),
        IndependentVerifierVerdictV3::RejectProtocolParity
    );
    let envelope = seal_generation_verifier_receipt_v3(
        &manifest,
        GenerationVerifierReceiptInputV3 {
            partition: GenerationEvidencePartitionV3::Future,
            capture_sequence: 10,
            support_watermark_next_sequence: 10,
            support_freeze_sha256: Some(root("support freeze")),
            lineage_root_sha256: root("future lineage"),
            event_root_sha256: root("future event"),
        },
        &rejected,
    )
    .expect("reject envelope");
    assert!(!envelope.is_verified_pass());
    assert_eq!(
        envelope.f6_verdict(),
        IndependentVerifierVerdictV3::RejectProtocolParity
    );

    let verified = verify(&handoff, &handoff.actor_output);
    assert!(
        GenerationVerifierReceiptV3::from_canonical_bytes(
            &envelope.canonical_bytes().expect("bytes"),
            &manifest,
            &verified,
        )
        .is_err()
    );
}

#[test]
fn partition_generation_and_artifact_mismatches_fail_closed() {
    let handoff = handoff_v3("continue_session", "handle", "CellA17", &[33]);
    let manifest = generation_manifest(&handoff);
    let f6 = verify(&handoff, &handoff.actor_output);
    assert_eq!(
        seal_generation_verifier_receipt_v3(
            &manifest,
            GenerationVerifierReceiptInputV3 {
                partition: GenerationEvidencePartitionV3::Future,
                capture_sequence: 9,
                support_watermark_next_sequence: 10,
                support_freeze_sha256: Some(root("freeze")),
                lineage_root_sha256: root("lineage"),
                event_root_sha256: root("event"),
            },
            &f6,
        ),
        Err(GenerationVerifierReceiptErrorV3::InvalidPartitionBinding)
    );
    assert_eq!(
        seal_generation_verifier_receipt_v3(
            &manifest,
            GenerationVerifierReceiptInputV3 {
                partition: GenerationEvidencePartitionV3::Support,
                capture_sequence: 1,
                support_watermark_next_sequence: 10,
                support_freeze_sha256: Some(root("unexpected freeze")),
                lineage_root_sha256: root("lineage"),
                event_root_sha256: root("event"),
            },
            &f6,
        ),
        Err(GenerationVerifierReceiptErrorV3::InvalidPartitionBinding)
    );

    let foreign = handoff_v3("continue_session", "handle", "CellA17", &[34]);
    assert_eq!(
        seal_generation_verifier_receipt_v3(
            &generation_manifest(&foreign),
            GenerationVerifierReceiptInputV3 {
                partition: GenerationEvidencePartitionV3::Support,
                capture_sequence: 1,
                support_watermark_next_sequence: 10,
                support_freeze_sha256: None,
                lineage_root_sha256: root("lineage"),
                event_root_sha256: root("event"),
            },
            &f6,
        ),
        Err(GenerationVerifierReceiptErrorV3::ArtifactSetMismatch)
    );
}

#[test]
fn generation_envelope_tampering_is_rejected() {
    let handoff = handoff_v3("continue_session", "handle", "CellA17", &[35]);
    let manifest = generation_manifest(&handoff);
    let f6 = verify(&handoff, &handoff.actor_output);
    let envelope = seal_generation_verifier_receipt_v3(
        &manifest,
        GenerationVerifierReceiptInputV3 {
            partition: GenerationEvidencePartitionV3::Support,
            capture_sequence: 1,
            support_watermark_next_sequence: 10,
            support_freeze_sha256: None,
            lineage_root_sha256: root("lineage"),
            event_root_sha256: root("event"),
        },
        &f6,
    )
    .expect("envelope");
    let mut bytes = envelope.canonical_bytes().expect("bytes");
    let offset = bytes.len() / 2;
    bytes[offset] ^= 1;
    assert!(GenerationVerifierReceiptV3::from_canonical_bytes(&bytes, &manifest, &f6).is_err());
}
