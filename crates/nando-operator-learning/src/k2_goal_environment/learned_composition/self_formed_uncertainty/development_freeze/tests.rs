use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicU64, Ordering};

use super::super::{K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1, uncertainty_root_v1};
use super::persistence::{
    K2UncertaintyDevelopmentFreezeFaultV1, publish_self_formed_development_freeze_with_fault_v1,
};
use super::*;
use crate::k2_goal_environment::learned_composition::{
    K2CompositionResultV1, composition_sha256_bytes_v1,
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn development_freeze_is_atomic_idempotent_and_fail_closed() {
    let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nando-k2-development-freeze-{}-{sequence}",
        std::process::id()
    ));
    let receipt =
        seal_self_formed_development_freeze_v1(&input(sequence)).expect("seal development freeze");

    let before = publish_self_formed_development_freeze_with_fault_v1(
        &root,
        &receipt,
        K2UncertaintyDevelopmentFreezeFaultV1::BeforeRename,
    );
    assert_error(before, "self_formed_development_freeze_fault_before_rename");
    assert!(!root.join(DEVELOPMENT_FREEZE_FILE_V1).exists());
    assert!(
        !root
            .join(format!(".{DEVELOPMENT_FREEZE_FILE_V1}.tmp"))
            .exists()
    );

    publish_self_formed_development_freeze_v1(&root, &receipt).expect("publish development freeze");
    assert_eq!(
        read_self_formed_development_freeze_v1(&root).expect("read development freeze"),
        receipt
    );
    assert_eq!(
        fs::metadata(root.join(DEVELOPMENT_FREEZE_FILE_V1))
            .expect("freeze metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    publish_self_formed_development_freeze_v1(&root, &receipt)
        .expect("idempotent development freeze");

    let other = seal_self_formed_development_freeze_v1(&input(sequence + 1))
        .expect("seal different development freeze");
    assert_error(
        publish_self_formed_development_freeze_v1(&root, &other),
        "self_formed_development_freeze_collision",
    );
    fs::remove_dir_all(root).expect("remove development freeze root");
}

#[test]
fn after_rename_failure_recovers_without_rewriting_receipt() {
    let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nando-k2-development-freeze-after-{}-{sequence}",
        std::process::id()
    ));
    let receipt =
        seal_self_formed_development_freeze_v1(&input(sequence)).expect("seal development freeze");
    assert_error(
        publish_self_formed_development_freeze_with_fault_v1(
            &root,
            &receipt,
            K2UncertaintyDevelopmentFreezeFaultV1::AfterRename,
        ),
        "self_formed_development_freeze_fault_after_rename",
    );
    publish_self_formed_development_freeze_v1(&root, &receipt).expect("recover after rename");
    assert_eq!(
        read_self_formed_development_freeze_v1(&root).expect("read recovered freeze"),
        receipt
    );
    fs::remove_dir_all(root).expect("remove development freeze root");
}

#[test]
fn manifest_drift_and_authority_promotion_are_rejected() {
    let mut value = seal_self_formed_development_freeze_v1(&input(99)).expect("seal fixture");
    value.manifests.swap(0, 1);
    assert_error(
        value.validate(),
        "self_formed_development_manifest_set_invalid",
    );

    let mut value = seal_self_formed_development_freeze_v1(&input(100)).expect("seal fixture");
    value.authority.natural_k2_authority = true;
    assert_error(value.validate(), "authority_boundary_violated");
}

fn input(sequence: u64) -> K2UncertaintyDevelopmentFreezeInputV1 {
    let root = |label: &str| composition_sha256_bytes_v1(format!("{label}-{sequence}").as_bytes());
    K2UncertaintyDevelopmentFreezeInputV1 {
        schema: K2_UNCERTAINTY_DEVELOPMENT_FREEZE_INPUT_SCHEMA_V1.to_owned(),
        frozen_commit_sha1: format!("{:040x}", sequence + 1),
        manifests: [
            K2UncertaintyFrozenManifestKindV1::Contract,
            K2UncertaintyFrozenManifestKindV1::Source,
            K2UncertaintyFrozenManifestKindV1::Executable,
            K2UncertaintyFrozenManifestKindV1::TestGate,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, kind)| K2UncertaintyFrozenManifestInputV1 {
            kind,
            entry_count: index as u64 + 1,
            byte_len: index as u64 + 100,
            content_sha256: root(&format!("manifest-{index}")),
        })
        .collect(),
        development_result: K2UncertaintyDevelopmentResultV1 {
            schema: K2_UNCERTAINTY_DEVELOPMENT_RESULT_SCHEMA_V1.to_owned(),
            package_tests_passed: 465,
            package_tests_failed: 0,
            package_tests_ignored: 8,
            legacy_controls_passed: 32,
            v3_controls_passed: 4,
            v4_controls_passed: 16,
            development_cases_passed: 16,
            one_probe_cases: 8,
            two_probe_cases: 8,
            independent_final_verifications_passed: 16,
            false_accepts: 0,
            maximum_final_request_bytes: 913_356,
            release_process_duration_ms: 164_260,
        },
        r8_receipt_sha256: root("r8-receipt"),
        selector_source_sha256: K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1.to_owned(),
        production_serving_source_sha256: PRODUCTION_SERVING_SOURCE_SHA256_V1.to_owned(),
        production_dashboard_source_sha256: PRODUCTION_DASHBOARD_SOURCE_SHA256_V1.to_owned(),
        generator_executable_sha256: root("generator"),
        freeze_owner_executable_sha256: root("freeze-owner"),
    }
}

fn assert_error<T>(result: K2CompositionResultV1<T>, code: &str) {
    let error = result.err().expect("fault control accepted");
    assert!(error.to_string().contains(code), "wrong error: {error}");
}
