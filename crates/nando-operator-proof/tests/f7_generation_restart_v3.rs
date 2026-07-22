#[allow(dead_code)]
mod f6_support;

use f6_support::handoff_v3;
use nando_operator_kernel::{
    OperatorGenerationComponentRootsV3, canonical_json_sha256, executable_artifact_set_sha256_v3,
    seal_operator_generation_manifest_v3,
};
use nando_operator_proof::independent_verifier_v3::IndependentVerifierArtifactSetV3;
use nando_operator_runtime::{
    OPERATOR_GENERATION_RESTART_MAX_BYTES_V3, OperatorGenerationRestartErrorV3,
    compile_structural_dispatch_index_v3, decode_operator_generation_restart_bundle_v3,
    encode_operator_generation_restart_bundle_v3,
};

fn root(label: &str) -> String {
    canonical_json_sha256(&label).expect("root")
}

#[test]
fn generation_restart_is_order_invariant_and_converges_with_f6() {
    let handoff = handoff_v3("continue_session", "handle", "CellA17", &[10, 11, 12]);
    let mut reversed = handoff.artifacts.clone();
    reversed.reverse();
    let index = compile_structural_dispatch_index_v3(&handoff.artifacts).expect("index");
    let verifier_set =
        IndependentVerifierArtifactSetV3::new(&handoff.artifacts).expect("F6 artifact set");
    let manifest = seal_operator_generation_manifest_v3(
        1,
        None,
        OperatorGenerationComponentRootsV3 {
            artifact_set_sha256: executable_artifact_set_sha256_v3(&handoff.artifacts)
                .expect("artifact set"),
            dispatch_index_sha256: index.index_sha256().to_owned(),
            actor_program_sha256: root("F7 actor program"),
            renderer_program_sha256: root("F7 renderer program"),
            verifier_contract_sha256: root("F7 verifier contract"),
            capability_contract_sha256: root("F7 capability contract"),
            resource_budget_sha256: root("F7 resource budget"),
        },
    )
    .expect("manifest");

    assert_eq!(
        manifest.components().artifact_set_sha256,
        verifier_set.artifact_set_sha256()
    );
    let bytes = encode_operator_generation_restart_bundle_v3(&manifest, &handoff.artifacts)
        .expect("bundle");
    let reversed_bytes = encode_operator_generation_restart_bundle_v3(&manifest, &reversed)
        .expect("reversed bundle");
    assert_eq!(bytes, reversed_bytes);

    let restored = decode_operator_generation_restart_bundle_v3(&bytes).expect("restore");
    assert_eq!(restored.manifest(), &manifest);
    assert_eq!(restored.index().index_sha256(), index.index_sha256());
    assert_eq!(restored.artifacts().len(), handoff.artifacts.len());
    assert!(!restored.execution_authority());
    assert_eq!(
        encode_operator_generation_restart_bundle_v3(restored.manifest(), restored.artifacts()),
        Ok(bytes)
    );
}

#[test]
fn new_generation_preserves_old_bytes_and_tampering_fails_closed() {
    let handoff = handoff_v3("continue_session", "handle", "CellA17", &[20, 21]);
    let index = compile_structural_dispatch_index_v3(&handoff.artifacts).expect("index");
    let components = OperatorGenerationComponentRootsV3 {
        artifact_set_sha256: executable_artifact_set_sha256_v3(&handoff.artifacts)
            .expect("artifact set"),
        dispatch_index_sha256: index.index_sha256().to_owned(),
        actor_program_sha256: root("actor"),
        renderer_program_sha256: root("renderer"),
        verifier_contract_sha256: root("verifier"),
        capability_contract_sha256: root("capability"),
        resource_budget_sha256: root("budget"),
    };
    let first = seal_operator_generation_manifest_v3(1, None, components.clone()).expect("first");
    let first_bytes = encode_operator_generation_restart_bundle_v3(&first, &handoff.artifacts)
        .expect("first bytes");
    let second = seal_operator_generation_manifest_v3(
        2,
        Some(first.generation_id_sha256().to_owned()),
        components,
    )
    .expect("second");
    let second_bytes = encode_operator_generation_restart_bundle_v3(&second, &handoff.artifacts)
        .expect("second bytes");

    assert_ne!(first.generation_id_sha256(), second.generation_id_sha256());
    assert_ne!(first_bytes, second_bytes);
    assert_eq!(
        encode_operator_generation_restart_bundle_v3(&first, &handoff.artifacts),
        Ok(first_bytes.clone())
    );

    let mut tampered = first_bytes;
    let offset = tampered.len() / 2;
    tampered[offset] ^= 1;
    assert!(decode_operator_generation_restart_bundle_v3(&tampered).is_err());
    assert!(decode_operator_generation_restart_bundle_v3(&tampered[..offset]).is_err());
    assert_eq!(
        decode_operator_generation_restart_bundle_v3(&vec![
            0;
            OPERATOR_GENERATION_RESTART_MAX_BYTES_V3
                + 1
        ])
        .err(),
        Some(OperatorGenerationRestartErrorV3::BudgetExhausted)
    );
    assert_eq!(
        encode_operator_generation_restart_bundle_v3(
            &first,
            &[handoff.artifacts[0].clone(), handoff.artifacts[0].clone()],
        ),
        Err(OperatorGenerationRestartErrorV3::DuplicateArtifact)
    );

    let misbound = seal_operator_generation_manifest_v3(
        1,
        None,
        OperatorGenerationComponentRootsV3 {
            dispatch_index_sha256: root("wrong index"),
            ..first.components().clone()
        },
    )
    .expect("misbound manifest");
    assert_eq!(
        encode_operator_generation_restart_bundle_v3(&misbound, &handoff.artifacts),
        Err(OperatorGenerationRestartErrorV3::ManifestMismatch)
    );
}
