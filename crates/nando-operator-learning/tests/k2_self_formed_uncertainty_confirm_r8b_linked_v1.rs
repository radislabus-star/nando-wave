use std::collections::BTreeMap;
use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use nando_operator_learning::{
    K2UncertaintyCleanupArtifactKindV1, K2UncertaintyCleanupAuthorizationReceiptV1,
    K2UncertaintyCleanupAuthorizationRequestV1, K2UncertaintyCleanupOwnerReceiptV1,
    K2UncertaintyCleanupOwnerRequestV1, K2UncertaintyCleanupReceiptV1,
    K2UncertaintyCleanupRegistryEntryV1, K2UncertaintyCleanupVerifyRequestV1,
    K2UncertaintyDevelopmentRehearsalOwnerReceiptV1,
    K2UncertaintyDevelopmentRehearsalTerminalRequestV1, K2UncertaintyDevelopmentResultReceiptV1,
    K2UncertaintyDevelopmentResultRequestV1, K2UncertaintyImmutablePublicationFaultV1,
    K2UncertaintyPublicCoordinatorRequestV1, K2UncertaintyPublicOwnerRoleV1,
    K2UncertaintyPublicOwnerSetV1, K2UncertaintyPublicOwnerV1,
    K2UncertaintyPublicPrecommitReceiptV1, K2UncertaintyR7kCleanupGuestV1,
    K2UncertaintyR8BEvidenceKindV2, K2UncertaintyR8BMeasuredReceiptV2,
    K2UncertaintyResultProcessRequestV1, K2UncertaintyTerminalEvaluationReceiptV1,
    K2UncertaintyTerminalProcessRequestV1, census_self_formed_cleanup_artifacts_v1,
    composition_sha256_file_v1, load_development_rehearsal_owner_metadata_v1,
    publish_immutable_file_v1, publish_self_formed_cleanup_manifest_v1, uncertainty_bytes_v1,
};

#[rustfmt::skip]
#[path = "k2_self_formed_uncertainty_confirm_r8b_support/mod.rs"]
mod support;

use support::*;

const CHILD_SELECTOR_V2: &str = "r8b_v7_m24_linked_child";

#[test]
#[ignore = "requires explicit R8B V7 execution authorization"]
fn r8b_v7_m24_linked_child() {
    begin_suite_request_from_stdin_v2("M24_LINKED_RUNNER", CHILD_SELECTOR_V2);
    let producer = active_producer_request_v2();
    let binaries = LinkedBinariesV2::from_cargo();
    let linked_manifest = binaries.manifest();
    let suite_manifest = load_suite_manifest_v2();
    assert_eq!(linked_manifest.identities.len(), 26);
    assert_eq!(suite_manifest.identities.len(), 5);

    let output_root = PathBuf::from(&producer.exclusive_output_directory);
    let route_root = output_root
        .parent()
        .expect("M24 output parent")
        .join("child-route");
    create_private_directory_v1(&route_root);
    let candidate_root = route_root.join("candidate");
    create_private_directory_v1(&candidate_root);
    let lab = route_root.join("lab");
    create_private_directory_v1(&lab);
    let public_root = route_root.join("public");
    create_private_directory_v1(&public_root);
    let control = route_root.join("control");
    create_private_directory_v1(&control);

    let ledger_root = PathBuf::from(
        std::env::var_os(nando_operator_learning::K2_UNCERTAINTY_R8B_LEDGER_ROOT_ENV_V2)
            .expect("M24 child ledger root"),
    );
    let m24 = binaries.get("M24_LINKED_RUNNER");
    let allowed = [
        "M01_DEVELOPMENT_OWNER",
        "M10_PUBLIC_COORDINATOR",
        "M11_PRIVATE_RESOLVER",
        "M12_SAFETY",
        "M13_WORKER",
        "M14_OBSERVER",
        "M15_FINAL_VERIFIER",
        "M16_ORACLE",
        "M17_CONTROL_EVALUATOR",
        "M18_TERMINAL_EVALUATOR",
        "M19_FRESH_CONTROL_CASE",
        "M20_CLEANUP_AUTHORIZER",
        "M21_CLEANUP_OWNER",
        "M22_CLEANUP_VERIFIER",
        "M23_DEVELOPMENT_RESULT_PUBLISHER",
    ]
    .map(|role| binaries.get(role));
    let mut ledger = DurableProcessLedgerV2::open(
        &ledger_root,
        producer.route_id_sha256.clone(),
        m24,
        &allowed,
    );

    let owner = binaries.get("M01_DEVELOPMENT_OWNER");
    let generator = binaries.get("M02_GENERATOR");
    let owner_request = development_owner_request_v1(&lab, "attempt", &owner.path, &generator.path);
    let development =
        run_recorded_process_v2::<_, K2UncertaintyDevelopmentRehearsalOwnerReceiptV1, _>(
            &mut ledger,
            owner,
            "C01",
            None,
            None,
            owner_request.request_root_sha256.clone(),
            &owner_request,
            |receipt| (&receipt.schema, &receipt.receipt_root_sha256),
        );
    let attempt = lab.join("attempt");
    let metadata = load_development_rehearsal_owner_metadata_v1(&attempt, &owner_request)
        .expect("load M24 Development metadata");
    assert_eq!(metadata.owner, development.value);
    assert_eq!(metadata.split.artifacts.len(), 34);

    let coordinator = binaries.get("M10_PUBLIC_COORDINATOR");
    let coordinator_request = K2UncertaintyPublicCoordinatorRequestV1::seal(
        metadata.public_batch.clone(),
        metadata.public_denominator.clone(),
        public_owner_set_v2(&binaries),
        public_root.to_string_lossy().into_owned(),
        coordinator.sha256.clone(),
    )
    .expect("M10 request");
    let public = run_recorded_process_v2::<_, K2UncertaintyPublicPrecommitReceiptV1, _>(
        &mut ledger,
        coordinator,
        "C06",
        None,
        None,
        coordinator_request.request_root_sha256.clone(),
        &coordinator_request,
        |receipt| (&receipt.schema, &receipt.receipt_root_sha256),
    );
    assert!(public.value.all_cases_precommitted);
    assert_eq!(public.value.case_artifacts.len(), 16);

    let freeze = owner_request
        .descriptor
        .successor_freeze_root_sha256
        .clone();
    let downstream = r7j_bridge::run_downstream_v2(
        &mut ledger,
        &binaries,
        &route_root,
        &attempt,
        &public_root,
        &metadata,
        &public.value,
        &freeze,
    );
    let oracle = downstream.oracle;
    let controls = downstream.controls;
    let routes = downstream.routes;
    let resources = downstream.resources;
    let experiment = owner_request.descriptor.experiment_id_sha256.clone();
    assert_eq!(controls.len(), 4);
    for (kind, receipt, relative) in [
        (
            K2UncertaintyR8BEvidenceKindV2::LegacyControls,
            &controls[0],
            "linked/legacy-controls.json",
        ),
        (
            K2UncertaintyR8BEvidenceKindV2::V3Controls,
            &controls[1],
            "linked/v3-controls.json",
        ),
        (
            K2UncertaintyR8BEvidenceKindV2::V4Controls,
            &controls[2],
            "linked/v4-controls.json",
        ),
        (
            K2UncertaintyR8BEvidenceKindV2::FreshControlCases,
            &controls[3],
            "linked/fresh-controls.json",
        ),
    ] {
        persist_candidate_v2(&candidate_root, relative, receipt);
        assert_eq!(
            receipt.passed,
            kind.required().expect("control denominator")
        );
    }

    let terminal = binaries.get("M18_TERMINAL_EVALUATOR");
    let terminal_request = K2UncertaintyDevelopmentRehearsalTerminalRequestV1::seal(
        experiment.clone(),
        oracle.clone(),
        controls.clone(),
        routes,
        resources,
        terminal.sha256.clone(),
    )
    .expect("M18 request");
    let terminal_process = K2UncertaintyTerminalProcessRequestV1::Development {
        request: terminal_request,
    };
    let terminal_receipt = run_recorded_process_v2::<_, K2UncertaintyTerminalEvaluationReceiptV1, _>(
        &mut ledger,
        terminal,
        "C14",
        None,
        None,
        nando_operator_learning::uncertainty_root_v1(&terminal_process)
            .expect("M18 process request root"),
        &terminal_process,
        |receipt| (&receipt.schema, &receipt.receipt_root_sha256),
    );

    let cleanup = complete_cleanup_v2(
        &mut ledger,
        &binaries,
        &attempt,
        &control,
        &owner_request,
        &development.value,
        &terminal_receipt.value,
        &candidate_root,
    );
    assert!(cleanup.cleanup.cleanup_frozen);
    assert_eq!(cleanup.result.disposition, "DEVELOPMENT_REHEARSAL_COMPLETE");

    persist_candidate_v2(&candidate_root, "linked/oracle-batch.json", &oracle);
    let control_census = K2UncertaintyR8BMeasuredReceiptV2::seal(
        K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes,
        producer.route_id_sha256.clone(),
        controls
            .iter()
            .map(|receipt| receipt.receipt_root_sha256.clone())
            .collect(),
        4,
        BTreeMap::new(),
        m24.sha256.clone(),
    )
    .expect("four-scope census");
    let linked_route = K2UncertaintyR8BMeasuredReceiptV2::seal(
        K2UncertaintyR8BEvidenceKindV2::LinkedRoute,
        producer.route_id_sha256.clone(),
        vec![
            development.value.receipt_root_sha256,
            public.value.receipt_root_sha256,
            terminal_receipt.value.receipt_root_sha256,
            cleanup.cleanup.receipt_root_sha256,
            cleanup.result.receipt_root_sha256,
        ],
        1,
        BTreeMap::from([
            ("m01_dispatches".to_owned(), 1),
            ("public_cases".to_owned(), 16),
            ("control_scopes".to_owned(), 4),
        ]),
        m24.sha256.clone(),
    )
    .expect("linked route receipt");
    publish_child_owned_v2(&output_root, &oracle, &control_census, &linked_route);
    freeze_directory_tree_v2(&candidate_root);
}

struct CleanupOutputV2 {
    cleanup: K2UncertaintyCleanupReceiptV1,
    result: K2UncertaintyDevelopmentResultReceiptV1,
}

#[allow(clippy::too_many_arguments)]
fn complete_cleanup_v2(
    ledger: &mut DurableProcessLedgerV2,
    binaries: &LinkedBinariesV2,
    governed: &Path,
    control: &Path,
    owner_request: &nando_operator_learning::K2UncertaintyConfirmOwnerRequestV1,
    development: &K2UncertaintyDevelopmentRehearsalOwnerReceiptV1,
    terminal: &K2UncertaintyTerminalEvaluationReceiptV1,
    candidate: &Path,
) -> CleanupOutputV2 {
    let registry = actual_tree_registry_v2(governed, development);
    let m24_sha = composition_sha256_file_v1(
        &std::env::current_exe().expect("M24 cleanup census executable"),
    )
    .expect("M24 cleanup census SHA-256");
    let (manifest, pages) = census_self_formed_cleanup_artifacts_v1(
        governed,
        owner_request.descriptor.experiment_id_sha256.clone(),
        registry,
        m24_sha,
    )
    .expect("linked cleanup census");
    publish_self_formed_cleanup_manifest_v1(governed, control, &manifest, &pages)
        .expect("publish linked cleanup census");

    let m20 = binaries.get("M20_CLEANUP_AUTHORIZER");
    let auth_request = K2UncertaintyCleanupAuthorizationRequestV1::seal(
        "/control".to_owned(),
        owner_request.descriptor.experiment_id_sha256.clone(),
        terminal.clone(),
        manifest.clone(),
        root_v1("linked-cleanup-journal"),
        root_v1("linked-observer-event"),
        root_v1("linked-terminal-event"),
        m20.sha256.clone(),
    )
    .expect("M20 request");
    let authorization = run_cleanup_recorded_v2::<_, K2UncertaintyCleanupAuthorizationReceiptV1>(
        ledger,
        m20,
        "C16",
        K2UncertaintyR7kCleanupGuestV1::Authorizer,
        None,
        control,
        &auth_request,
    );
    let m21 = binaries.get("M21_CLEANUP_OWNER");
    let cleanup_owner_request = K2UncertaintyCleanupOwnerRequestV1::seal(
        "/governed".to_owned(),
        "/control".to_owned(),
        authorization,
        m21.sha256.clone(),
    )
    .expect("M21 request");
    let cleanup_owner = run_cleanup_recorded_v2::<_, K2UncertaintyCleanupOwnerReceiptV1>(
        ledger,
        m21,
        "C17",
        K2UncertaintyR7kCleanupGuestV1::Owner,
        Some(governed),
        control,
        &cleanup_owner_request,
    );
    let m22 = binaries.get("M22_CLEANUP_VERIFIER");
    let verify_request = K2UncertaintyCleanupVerifyRequestV1::seal(
        "/governed".to_owned(),
        "/control".to_owned(),
        manifest,
        cleanup_owner,
        m22.sha256.clone(),
    )
    .expect("M22 request");
    let cleanup = run_cleanup_recorded_v2::<_, K2UncertaintyCleanupReceiptV1>(
        ledger,
        m22,
        "C18",
        K2UncertaintyR7kCleanupGuestV1::Verifier,
        Some(governed),
        control,
        &verify_request,
    );
    persist_candidate_v2(candidate, "linked/cleanup.json", &cleanup);

    let m23 = binaries.get("M23_DEVELOPMENT_RESULT_PUBLISHER");
    let result_request = K2UncertaintyResultProcessRequestV1::Development {
        request: K2UncertaintyDevelopmentResultRequestV1::seal(
            "/control".to_owned(),
            terminal.clone(),
            cleanup.clone(),
            m23.sha256.clone(),
        )
        .expect("M23 request"),
    };
    let result = run_cleanup_recorded_v2::<_, K2UncertaintyDevelopmentResultReceiptV1>(
        ledger,
        m23,
        "C20",
        K2UncertaintyR7kCleanupGuestV1::ResultPublisher,
        None,
        control,
        &result_request,
    );
    persist_candidate_v2(candidate, "linked/development-result.json", &result);
    CleanupOutputV2 { cleanup, result }
}

#[allow(clippy::too_many_arguments)]
fn run_cleanup_recorded_v2<I, O>(
    ledger: &mut DurableProcessLedgerV2,
    binary: &BinaryV2,
    stage: &str,
    role: K2UncertaintyR7kCleanupGuestV1,
    governed: Option<&Path>,
    control: &Path,
    request: &I,
) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let input = uncertainty_bytes_v1(request).expect("cleanup process input");
    let started = ledger.start(
        stage,
        None,
        None,
        binary,
        nando_operator_learning::uncertainty_root_v1(request)
            .expect("cleanup request semantic root"),
        nando_operator_learning::composition_sha256_bytes_v1(&input),
    );
    let outcome = nando_operator_learning::run_self_formed_r7k_cleanup_sandbox_measured_v1(
        role,
        &binary.path,
        &binary.sha256,
        governed,
        control,
        &input,
        60,
    )
    .expect("run linked cleanup sandbox");
    let output = sandbox_output_v2(outcome);
    let value: O = nando_operator_learning::uncertainty_decode_v1(&output.stdout)
        .expect("decode linked cleanup receipt");
    let (schema, root) =
        typed_json_identity_v2(&output.stdout).expect("typed linked cleanup receipt");
    ledger.finish(&started, &output, schema, root);
    value
}

fn sandbox_output_v2(
    outcome: nando_operator_learning::K2UncertaintySandboxProcessOutcomeV1,
) -> std::process::Output {
    use std::os::unix::process::ExitStatusExt;
    std::process::Output {
        status: std::process::ExitStatus::from_raw(outcome.exit_code << 8),
        stdout: outcome.stdout,
        stderr: outcome.stderr,
    }
}

fn actual_tree_registry_v2(
    governed: &Path,
    owner: &K2UncertaintyDevelopmentRehearsalOwnerReceiptV1,
) -> Vec<K2UncertaintyCleanupRegistryEntryV1> {
    let mut pending = vec![governed.to_path_buf()];
    let mut registry = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("read linked governed tree") {
            let path = entry.expect("linked governed entry").path();
            let relative = path
                .strip_prefix(governed)
                .expect("linked governed relative path")
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

fn public_owner_set_v2(binaries: &LinkedBinariesV2) -> K2UncertaintyPublicOwnerSetV1 {
    let roles = [
        (K2UncertaintyPublicOwnerRoleV1::Learner, "M03_LEARNER"),
        (K2UncertaintyPublicOwnerRoleV1::Probe, "M04_PROBE"),
        (K2UncertaintyPublicOwnerRoleV1::Selector, "M05_SELECTOR"),
        (K2UncertaintyPublicOwnerRoleV1::Baseline, "M06_BASELINE"),
        (
            K2UncertaintyPublicOwnerRoleV1::SelectionPreverifier,
            "M07_SELECTION_PREVERIFIER",
        ),
        (
            K2UncertaintyPublicOwnerRoleV1::ClosurePlanner,
            "M08_CLOSURE_PLANNER",
        ),
        (
            K2UncertaintyPublicOwnerRoleV1::ClosureVerifier,
            "M09_CLOSURE_VERIFIER",
        ),
    ];
    K2UncertaintyPublicOwnerSetV1::seal(
        roles
            .into_iter()
            .map(|(role, name)| {
                let binary = binaries.get(name);
                K2UncertaintyPublicOwnerV1 {
                    role,
                    executable_path: binary.path.to_string_lossy().into_owned(),
                    executable_sha256: binary.sha256.clone(),
                }
            })
            .collect(),
    )
    .expect("linked public owner set")
}

fn persist_candidate_v2<T: serde::Serialize>(root: &Path, relative: &str, value: &T) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create candidate parent");
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .expect("chmod candidate parent");
    }
    write_new_read_only_v2(
        &path,
        &uncertainty_bytes_v1(value).expect("candidate canonical bytes"),
    );
}

fn publish_child_owned_v2(
    root: &Path,
    oracle: &nando_operator_learning::K2UncertaintyOracleBaselineBatchReceiptV1,
    controls: &K2UncertaintyR8BMeasuredReceiptV2,
    linked: &K2UncertaintyR8BMeasuredReceiptV2,
) {
    for (sequence, relative, bytes) in [
        (
            0_u64,
            "linked/oracle-batch.json",
            uncertainty_bytes_v1(oracle).expect("oracle batch bytes"),
        ),
        (
            1,
            "linked/control-scopes.json",
            uncertainty_bytes_v1(controls).expect("control census bytes"),
        ),
        (
            2,
            "linked/route.json",
            uncertainty_bytes_v1(linked).expect("linked route bytes"),
        ),
    ] {
        let parent = root
            .join(relative)
            .parent()
            .expect("child output parent")
            .to_path_buf();
        fs::create_dir_all(&parent).expect("create child output parent");
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700))
            .expect("chmod child output parent");
        publish_immutable_file_v1(
            root,
            relative,
            &bytes,
            0o400,
            sequence,
            K2UncertaintyImmutablePublicationFaultV1::None,
        )
        .expect("publish child-owned receipt");
    }
    File::open(root)
        .expect("open child output root")
        .sync_all()
        .expect("fsync child output root");
}

#[rustfmt::skip]
#[path = "k2_self_formed_uncertainty_confirm_r8b_linked_v1/parent.rs"]
mod r7j_bridge;
