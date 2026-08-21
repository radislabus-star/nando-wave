use std::collections::BTreeSet;
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};

use nando_operator_learning::{
    K2UncertaintyCleanupArtifactKindV1, K2UncertaintyCleanupAuthorizationReceiptV1,
    K2UncertaintyCleanupAuthorizationRequestV1, K2UncertaintyCleanupFaultV1,
    K2UncertaintyCleanupOwnerReceiptV1, K2UncertaintyCleanupOwnerRequestV1,
    K2UncertaintyCleanupReceiptV1, K2UncertaintyCleanupRegistryEntryV1,
    K2UncertaintyCleanupVerifyRequestV1, K2UncertaintyControlScopeV1,
    K2UncertaintyDevelopmentRehearsalOwnerReceiptV1,
    K2UncertaintyDevelopmentRehearsalTerminalRequestV1, K2UncertaintyDevelopmentResultReceiptV1,
    K2UncertaintyDevelopmentResultRequestV1, K2UncertaintyR7kCleanupGuestV1,
    K2UncertaintyResultProcessRequestV1, K2UncertaintyTerminalEvaluationReceiptV1,
    K2UncertaintyTerminalProcessRequestV1, census_self_formed_cleanup_artifacts_v1,
    composition_sha256_file_v1, execute_self_formed_cleanup_with_fault_v1,
    load_development_rehearsal_owner_metadata_v1, publish_self_formed_cleanup_manifest_v1,
    run_self_formed_r7k_cleanup_sandbox_measured_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
};

#[rustfmt::skip]
#[path = "k2_self_formed_uncertainty_confirm_r8b_support/mod.rs"]
mod support;

use support::*;

const S04_SELECTOR_V2: &str = "r8b_v7_s04_cleanup_negative_aggregate";

#[test]
#[ignore = "requires explicit R8B V7 execution authorization"]
fn r8b_v7_s04_cleanup_negative_aggregate() {
    begin_suite_request_from_stdin_v2("S04_CLEANUP_NEGATIVE", S04_SELECTOR_V2);
    let environment = TestEnvironmentV1::new("cleanup-interruption");
    let owner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-confirm-owner"));
    let generator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
    let request = development_owner_request_v1(&environment.root, "governed", &owner, &generator);
    let development: K2UncertaintyDevelopmentRehearsalOwnerReceiptV1 =
        run_process_success_v1(&owner, &request);
    let governed = environment.root.join("governed");
    let control = environment.private_child("control");
    let terminal = development_terminal_v1(&request.descriptor.experiment_id_sha256);
    let registry = actual_tree_registry_v1(&governed, &development);
    let suite_sha =
        composition_sha256_file_v1(&std::env::current_exe().expect("S04 suite executable"))
            .expect("S04 suite SHA-256");
    let (manifest, pages) = census_self_formed_cleanup_artifacts_v1(
        &governed,
        request.descriptor.experiment_id_sha256.clone(),
        registry,
        suite_sha,
    )
    .expect("S04 before-cleanup census");
    publish_self_formed_cleanup_manifest_v1(&governed, &control, &manifest, &pages)
        .expect("publish S04 cleanup census");

    let authorizer = PathBuf::from(env!(
        "CARGO_BIN_EXE_nando-k2-self-formed-cleanup-authorizer"
    ));
    let cleanup_owner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-cleanup-owner"));
    let verifier = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-cleanup-verifier"));
    let authorizer_sha = composition_sha256_file_v1(&authorizer).expect("S04 M20 SHA-256");
    let owner_sha = composition_sha256_file_v1(&cleanup_owner).expect("S04 M21 SHA-256");
    let verifier_sha = composition_sha256_file_v1(&verifier).expect("S04 M22 SHA-256");
    let authorization_request = K2UncertaintyCleanupAuthorizationRequestV1::seal(
        "/control".to_owned(),
        request.descriptor.experiment_id_sha256,
        terminal,
        manifest.clone(),
        root_v1("s04-cleanup-journal-projection"),
        root_v1("s04-observer-durable-event"),
        root_v1("s04-terminal-durable-event"),
        authorizer_sha.clone(),
    )
    .expect("S04 M20 request");
    let authorization: K2UncertaintyCleanupAuthorizationReceiptV1 = run_cleanup_v1(
        K2UncertaintyR7kCleanupGuestV1::Authorizer,
        &authorizer,
        &authorizer_sha,
        None,
        &control,
        &authorization_request,
    );
    let host_request = K2UncertaintyCleanupOwnerRequestV1::seal(
        governed.to_string_lossy().into_owned(),
        control.to_string_lossy().into_owned(),
        authorization.clone(),
        owner_sha.clone(),
    )
    .expect("S04 host cleanup request");
    let interrupted = execute_self_formed_cleanup_with_fault_v1(
        &host_request,
        K2UncertaintyCleanupFaultV1::AfterIntent { target: 0 },
    )
    .expect_err("S04 must interrupt after durable intent");
    assert_eq!(
        interrupted.to_string(),
        "k2_composition_invalid:self_formed_cleanup_injected_fault"
    );
    assert!(
        governed
            .join(&authorization.disposable_entries[0].relative_path)
            .exists(),
        "AfterIntent must not mutate the first target"
    );
    assert!(!control.join("cleanup-owner-receipt.json").exists());

    let guest_request = K2UncertaintyCleanupOwnerRequestV1::seal(
        "/governed".to_owned(),
        "/control".to_owned(),
        authorization,
        owner_sha.clone(),
    )
    .expect("S04 resumed M21 request");
    let owner_receipt: K2UncertaintyCleanupOwnerReceiptV1 = run_cleanup_v1(
        K2UncertaintyR7kCleanupGuestV1::Owner,
        &cleanup_owner,
        &owner_sha,
        Some(&governed),
        &control,
        &guest_request,
    );
    let verify_request = K2UncertaintyCleanupVerifyRequestV1::seal(
        "/governed".to_owned(),
        "/control".to_owned(),
        manifest,
        owner_receipt.clone(),
        verifier_sha.clone(),
    )
    .expect("S04 M22 request");
    let cleanup: K2UncertaintyCleanupReceiptV1 = run_cleanup_v1(
        K2UncertaintyR7kCleanupGuestV1::Verifier,
        &verifier,
        &verifier_sha,
        Some(&governed),
        &control,
        &verify_request,
    );
    assert!(cleanup.cleanup_frozen);
    assert_eq!(cleanup.unexpected_residue, 0);
    assert_eq!(
        owner_receipt.events.len(),
        owner_receipt.deleted_paths as usize * 2
    );
    publish_suite_measurements_v2(vec![SuiteMeasurementV2 {
        relative_path: "suites/s04/cleanup-interruption.json",
        kind: nando_operator_learning::K2UncertaintyR8BEvidenceKindV2::CleanupInterruption,
        source_roots_sha256: vec![
            owner_receipt.events[0].event_root_sha256.clone(),
            cleanup.receipt_root_sha256,
        ],
        observed: 1,
        metrics: [(
            "recovered_deleted_paths".to_owned(),
            owner_receipt.deleted_paths,
        )]
        .into_iter()
        .collect(),
    }]);
}

#[test]
fn r8b_cleanup_uses_four_distinct_process_owners_and_completes_once() {
    let environment = TestEnvironmentV1::new("cleanup-positive");
    let owner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-confirm-owner"));
    let generator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
    let request = development_owner_request_v1(&environment.root, "governed", &owner, &generator);
    let development: K2UncertaintyDevelopmentRehearsalOwnerReceiptV1 =
        run_process_success_v1(&owner, &request);
    let governed = environment.root.join("governed");
    let control = environment.private_child("control");
    let metadata = load_development_rehearsal_owner_metadata_v1(&governed, &request)
        .expect("Development metadata before cleanup");
    assert_eq!(metadata.owner, development);

    let terminal = development_terminal_v1(&request.descriptor.experiment_id_sha256);
    let census_sha256 =
        composition_sha256_file_v1(&std::env::current_exe().expect("cleanup suite executable"))
            .expect("cleanup suite SHA-256");
    let registry = actual_tree_registry_v1(&governed, &development);
    let (manifest, pages) = census_self_formed_cleanup_artifacts_v1(
        &governed,
        request.descriptor.experiment_id_sha256.clone(),
        registry,
        census_sha256,
    )
    .expect("complete before-cleanup census");
    assert_eq!(
        manifest.entry_count,
        pages
            .iter()
            .map(|page| page.entries.len() as u64)
            .sum::<u64>()
    );
    assert_eq!(
        pages
            .iter()
            .flat_map(|page| &page.entries)
            .filter(|entry| {
                entry.artifact_kind == K2UncertaintyCleanupArtifactKindV1::DisposableWorkspace
            })
            .count(),
        32
    );
    publish_self_formed_cleanup_manifest_v1(&governed, &control, &manifest, &pages)
        .expect("publish cleanup census");

    let authorizer = PathBuf::from(env!(
        "CARGO_BIN_EXE_nando-k2-self-formed-cleanup-authorizer"
    ));
    let cleanup_owner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-cleanup-owner"));
    let verifier = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-cleanup-verifier"));
    let result_publisher =
        PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-result-publisher"));
    let authorizer_sha = composition_sha256_file_v1(&authorizer).expect("M20 SHA-256");
    let owner_sha = composition_sha256_file_v1(&cleanup_owner).expect("M21 SHA-256");
    let verifier_sha = composition_sha256_file_v1(&verifier).expect("M22 SHA-256");
    let publisher_sha = composition_sha256_file_v1(&result_publisher).expect("M23 SHA-256");
    assert_eq!(
        BTreeSet::from([
            authorizer_sha.clone(),
            owner_sha.clone(),
            verifier_sha.clone(),
            publisher_sha.clone(),
        ])
        .len(),
        4
    );

    let authorization_request = K2UncertaintyCleanupAuthorizationRequestV1::seal(
        "/control".to_owned(),
        request.descriptor.experiment_id_sha256.clone(),
        terminal.clone(),
        manifest.clone(),
        root_v1("cleanup-journal-projection"),
        root_v1("observer-durable-event"),
        root_v1("terminal-durable-event"),
        authorizer_sha.clone(),
    )
    .expect("M20 request");
    let authorization: K2UncertaintyCleanupAuthorizationReceiptV1 = run_cleanup_v1(
        K2UncertaintyR7kCleanupGuestV1::Authorizer,
        &authorizer,
        &authorizer_sha,
        None,
        &control,
        &authorization_request,
    );

    let owner_request = K2UncertaintyCleanupOwnerRequestV1::seal(
        "/governed".to_owned(),
        "/control".to_owned(),
        authorization,
        owner_sha.clone(),
    )
    .expect("M21 request");
    let owner_receipt: K2UncertaintyCleanupOwnerReceiptV1 = run_cleanup_v1(
        K2UncertaintyR7kCleanupGuestV1::Owner,
        &cleanup_owner,
        &owner_sha,
        Some(&governed),
        &control,
        &owner_request,
    );
    assert_eq!(owner_receipt.deleted_paths, 32);
    assert_eq!(owner_receipt.events.len(), 64);

    let verify_request = K2UncertaintyCleanupVerifyRequestV1::seal(
        "/governed".to_owned(),
        "/control".to_owned(),
        manifest,
        owner_receipt,
        verifier_sha.clone(),
    )
    .expect("M22 request");
    let cleanup: K2UncertaintyCleanupReceiptV1 = run_cleanup_v1(
        K2UncertaintyR7kCleanupGuestV1::Verifier,
        &verifier,
        &verifier_sha,
        Some(&governed),
        &control,
        &verify_request,
    );
    assert!(cleanup.cleanup_frozen);
    assert_eq!(cleanup.deleted_paths, 32);
    assert_eq!(cleanup.unexpected_residue, 0);

    let result_request = K2UncertaintyResultProcessRequestV1::Development {
        request: K2UncertaintyDevelopmentResultRequestV1::seal(
            "/control".to_owned(),
            terminal.clone(),
            cleanup.clone(),
            publisher_sha.clone(),
        )
        .expect("M23 request"),
    };
    let result: K2UncertaintyDevelopmentResultReceiptV1 = run_cleanup_v1(
        K2UncertaintyR7kCleanupGuestV1::ResultPublisher,
        &result_publisher,
        &publisher_sha,
        None,
        &control,
        &result_request,
    );
    assert_eq!(result.disposition, "DEVELOPMENT_REHEARSAL_COMPLETE");
    assert_eq!(
        result.terminal_receipt_root_sha256,
        terminal.receipt_root_sha256
    );
    assert_eq!(
        result.cleanup_receipt_root_sha256,
        cleanup.receipt_root_sha256
    );
    assert!(!governed.join("R8B_RECEIPT_V3.json").exists());
}

#[test]
fn r8b_cleanup_rejects_unclassified_residue_before_authority() {
    let environment = TestEnvironmentV1::new("cleanup-residue");
    let owner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-confirm-owner"));
    let generator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
    let request = development_owner_request_v1(&environment.root, "governed", &owner, &generator);
    let receipt: K2UncertaintyDevelopmentRehearsalOwnerReceiptV1 =
        run_process_success_v1(&owner, &request);
    let governed = environment.root.join("governed");
    let registry = actual_tree_registry_v1(&governed, &receipt);
    fs::write(governed.join("unclassified-residue.bin"), b"residue")
        .expect("inject unclassified residue");
    assert!(
        census_self_formed_cleanup_artifacts_v1(
            &governed,
            request.descriptor.experiment_id_sha256,
            registry,
            root_v1("cleanup-negative-census"),
        )
        .is_err()
    );
    assert!(governed.join("development-owner-receipt.json").exists());
    assert!(!governed.join("R8B_RECEIPT_V3.json").exists());
}

fn development_terminal_v1(experiment: &str) -> K2UncertaintyTerminalEvaluationReceiptV1 {
    let (oracle, routes, resources) = r7j_terminal_evidence_v1();
    let controls = [
        K2UncertaintyControlScopeV1::SuccessorStaticLegacy,
        K2UncertaintyControlScopeV1::SuccessorStaticV3,
        K2UncertaintyControlScopeV1::SuccessorStaticV4,
        K2UncertaintyControlScopeV1::DevelopmentRehearsalV5,
    ]
    .into_iter()
    .map(|scope| {
        let freeze = (scope == K2UncertaintyControlScopeV1::DevelopmentRehearsalV5)
            .then_some(root_v1("freeze"));
        control_receipt_v1(scope, experiment, freeze.as_deref(), None)
    })
    .collect();
    let terminal = PathBuf::from(env!(
        "CARGO_BIN_EXE_nando-k2-self-formed-terminal-evaluator"
    ));
    let terminal_sha = composition_sha256_file_v1(&terminal).expect("M18 SHA-256");
    let request = K2UncertaintyDevelopmentRehearsalTerminalRequestV1::seal(
        experiment.to_owned(),
        oracle,
        controls,
        routes,
        resources,
        terminal_sha,
    )
    .expect("Development terminal request");
    run_process_success_v1(
        &terminal,
        &K2UncertaintyTerminalProcessRequestV1::Development { request },
    )
}

fn actual_tree_registry_v1(
    governed: &Path,
    owner: &K2UncertaintyDevelopmentRehearsalOwnerReceiptV1,
) -> Vec<K2UncertaintyCleanupRegistryEntryV1> {
    let mut pending = vec![governed.to_path_buf()];
    let mut registry = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("read governed tree") {
            let path = entry.expect("governed entry").path();
            let relative = path
                .strip_prefix(governed)
                .expect("governed relative path")
                .to_string_lossy()
                .into_owned();
            let disposable = path.is_file()
                && (relative.starts_with("generated/private/resolver/")
                    || relative.starts_with("generated/private/final-truth/"));
            registry.push(K2UncertaintyCleanupRegistryEntryV1 {
                relative_path: relative,
                artifact_kind: if disposable {
                    K2UncertaintyCleanupArtifactKindV1::DisposableWorkspace
                } else {
                    K2UncertaintyCleanupArtifactKindV1::RetainedEvidence
                },
                producer_executable_sha256: owner.owner_executable_sha256.clone(),
                producing_journal_event_root_sha256: owner
                    .cases_generated_event_root_sha256
                    .clone(),
            });
            if path.is_dir() {
                pending.push(path);
            }
        }
    }
    registry.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    registry
}

fn run_cleanup_v1<I, O>(
    role: K2UncertaintyR7kCleanupGuestV1,
    executable: &Path,
    executable_sha256: &str,
    governed: Option<&Path>,
    control: &Path,
    request: &I,
) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let input = uncertainty_bytes_v1(request).expect("cleanup process input");
    let nested = begin_suite_nested_child_v2(executable, request, &input);
    let outcome = run_self_formed_r7k_cleanup_sandbox_measured_v1(
        role,
        executable,
        executable_sha256,
        governed,
        control,
        &input,
        60,
    )
    .expect("cleanup process success");
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(outcome.exit_code << 8),
        stdout: outcome.stdout,
        stderr: outcome.stderr,
    };
    finish_suite_nested_child_v2(nested, &output);
    uncertainty_decode_v1(&output.stdout).expect("cleanup process output")
}
