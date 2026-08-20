use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use nando_operator_learning::{
    K2_UNCERTAINTY_SEALED_RESULT_REQUEST_SCHEMA_V1, K2UncertaintyCleanupArtifactKindV1,
    K2UncertaintyCleanupAuthorizationReceiptV1, K2UncertaintyCleanupAuthorizationRequestV1,
    K2UncertaintyCleanupOwnerReceiptV1, K2UncertaintyCleanupOwnerRequestV1,
    K2UncertaintyCleanupReceiptV1, K2UncertaintyCleanupRegistryEntryV1,
    K2UncertaintyCleanupVerifyRequestV1, K2UncertaintyConfirmGuestExecutableV1,
    K2UncertaintyControlEvaluationReceiptV1, K2UncertaintyControlEvaluationRequestV1,
    K2UncertaintyControlProcessOutcomeV1, K2UncertaintyControlScopeV1,
    K2UncertaintyControlStdoutV1, K2UncertaintyDevelopmentRehearsalTerminalRequestV1,
    K2UncertaintyDevelopmentResultReceiptV1, K2UncertaintyDevelopmentResultRequestV1,
    K2UncertaintyEvaluationResourceMeasurementsV1, K2UncertaintyEvaluationRouteReceiptV1,
    K2UncertaintyOracleBaselineBatchReceiptV1, K2UncertaintyR7kCleanupGuestV1,
    K2UncertaintyR7kControlCaseRequestV1, K2UncertaintyResultProcessRequestV1,
    K2UncertaintySandboxProcessOutcomeV1, K2UncertaintySealedResultRequestV1,
    K2UncertaintyTerminalDispositionV1, K2UncertaintyTerminalEvaluationReceiptV1,
    K2UncertaintyTerminalProcessRequestV1, census_self_formed_cleanup_artifacts_v1,
    composition_root_v1, composition_sha256_bytes_v1, composition_sha256_file_v1,
    expected_self_formed_control_v1, new_self_formed_r7k_control_case_request_v1,
    publish_self_formed_cleanup_manifest_v1, run_self_formed_confirm_sandbox_v1,
    run_self_formed_r7k_cleanup_sandbox_v1, run_self_formed_r7k_control_sandbox_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1,
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

const R7J_FIXTURE_PAYLOADS: [&str; 7] = [
    "final-request.json",
    "one-probe-descriptor.json",
    "oracle-batch.json",
    "resolver-request.json",
    "resources.json",
    "routes.json",
    "two-probe-descriptor.json",
];

#[test]
fn r7k_implemented_controls_execute_as_canonical_child_processes() {
    validate_r7j_fixture_packet().expect("validate closed R7J predecessor packet");
    let fixture = ProcessFixture::new();
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-r7k-control-case"));
    let evaluator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-control-evaluator"));
    let runner_sha256 = composition_sha256_file_v1(&executable).expect("R7K runner SHA");
    let evaluator_sha256 = composition_sha256_file_v1(&evaluator).expect("R7J evaluator SHA");
    let test_sha256 = composition_sha256_file_v1(
        &std::env::current_exe().expect("resolve R7K integration test executable"),
    )
    .expect("R7K integration test SHA");
    let adapter_source_root = adapter_source_root();
    let experiment_root = root("experiment");
    let freeze_root = root("freeze");
    let mut outcomes = Vec::new();
    for (control_id, disposition, subcases) in [
        ("K1", "reused_development_commitment_rejected", 1_usize),
        ("K2", "missing_or_foreign_authorization_rejected", 2),
        ("K3", "nonce_transport_rejected", 4),
        ("K4", "private_public_leakage_rejected", 1),
        ("K5", "early_private_resolver_rejected", 2),
        ("K6", "early_final_truth_rejected", 2),
        ("K7", "coordinator_manifest_mismatch_rejected", 2),
        ("K8", "duplicate_slot_attempt_or_nonce_rejected", 2),
        ("K9", "partial_terminal_denominator_rejected", 2),
        ("K10", "one_probe_oracle_substitution_rejected", 1),
        ("K11", "baseline_denominator_omission_rejected", 2),
        ("K12", "cleanup_retention_or_residue_violation_rejected", 2),
    ] {
        let mount_root = fixture.root.join(control_id.to_ascii_lowercase());
        let mut scratch = mount_root.clone();
        if control_id == "K3" {
            for representation in canary_representations() {
                scratch.push(String::from_utf8(representation).expect("UTF-8 K3 path canary"));
            }
        }
        create_private_directory(&scratch);
        if control_id == "K1" {
            let seed_path = std::env::var_os("NANDO_K2_DEVELOPMENT_SEED_PATH")
                .map(PathBuf::from)
                .expect("NANDO_K2_DEVELOPMENT_SEED_PATH is required for R7K controls");
            let seed = fs::read(seed_path).expect("read frozen development seed");
            fs::write(scratch.join("development-seed.bin"), seed)
                .expect("copy frozen development seed into child scratch");
        }
        if matches!(control_id, "K5" | "K6" | "K9" | "K10" | "K11") {
            let fixture_root = std::env::var_os("NANDO_K2_R7K_FIXTURE_ROOT")
                .map(PathBuf::from)
                .expect("NANDO_K2_R7K_FIXTURE_ROOT is required for R7K controls");
            copy_fixture_tree(
                &fixture_root.join("fixture-packet"),
                &scratch.join("fixture-packet"),
            );
            if control_id == "K10" {
                copy_fixture_tree(
                    &fixture_root.join("oracle-case-00"),
                    &scratch.join("oracle-case-00"),
                );
            }
        }
        if control_id == "K3" {
            create_private_directory(&scratch.join("public"));
            let persisted = canary_representations()
                .into_iter()
                .flat_map(|mut representation| {
                    representation.push(b'\n');
                    representation
                })
                .collect::<Vec<_>>();
            fs::write(scratch.join("persisted-generator-request.bin"), persisted)
                .expect("write K3 persisted request fixture");
        }
        if control_id == "K4" {
            let public = scratch.join("public");
            create_private_directory(&public);
            for (index, representation) in canary_representations().into_iter().enumerate() {
                fs::write(public.join(format!("artifact-{index}.bin")), representation)
                    .expect("write K4 public canary fixture");
            }
        }
        if control_id == "K7" {
            let coordinator = PathBuf::from(env!(
                "CARGO_BIN_EXE_nando-k2-self-formed-public-coordinator"
            ));
            fs::copy(coordinator, scratch.join("public-coordinator"))
                .expect("copy K7 public coordinator fixture");
            fs::set_permissions(
                scratch.join("public-coordinator"),
                fs::Permissions::from_mode(0o400),
            )
            .expect("chmod K7 public coordinator fixture");
        }
        let guest_scratch = PathBuf::from("/scratch").join(
            scratch
                .strip_prefix(&mount_root)
                .expect("R7K scratch under mount root"),
        );
        let target_source_root = control_target_source_root(control_id);
        let request = new_self_formed_r7k_control_case_request_v1(
            control_id.to_owned(),
            experiment_root.clone(),
            freeze_root.clone(),
            guest_scratch.to_string_lossy().into_owned(),
            (0..subcases)
                .map(|ordinal| root(&format!("{control_id}-subcase-{ordinal}")))
                .collect(),
            target_source_root.clone(),
            adapter_source_root.clone(),
        )
        .expect("seal R7K control request");
        let argv = if control_id == "K3" {
            canary_representations()
                .into_iter()
                .map(|value| String::from_utf8(value).expect("UTF-8 K3 argv canary"))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let environment = if control_id == "K3" {
            canary_representations()
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    (
                        format!("NANDO_K2_R7K_CANARY_{index}"),
                        String::from_utf8(value).expect("UTF-8 K3 environment canary"),
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let process = run_control_process(
            &executable,
            &runner_sha256,
            &mount_root,
            &request,
            &argv,
            &environment,
        );
        let decoded: K2UncertaintyControlStdoutV1 =
            uncertainty_decode_v1(&process.stdout).expect("decode canonical R7K stdout");
        assert_eq!(decoded.control_id, control_id);
        assert_eq!(decoded.disposition, disposition);
        assert_eq!(
            process.stdout,
            uncertainty_bytes_v1(&decoded).expect("re-encode canonical R7K stdout")
        );
        let log_bytes = serde_json::to_vec(&serde_json::json!({
            "control_id": control_id,
            "request_root_sha256": request.request_root_sha256,
            "request_bytes_sha256": composition_sha256_bytes_v1(
                &uncertainty_bytes_v1(&request).expect("R7K request bytes")
            ),
            "normal_exit": process.normal_exit,
            "exit_code": process.exit_code,
            "stdout_sha256": composition_sha256_bytes_v1(&process.stdout),
            "stderr_sha256": composition_sha256_bytes_v1(&process.stderr),
            "timed_out": process.timed_out,
            "runner_executable_sha256": runner_sha256,
            "test_executable_sha256": test_sha256,
            "target_source_root_sha256": target_source_root,
            "adapter_source_root_sha256": adapter_source_root,
        }))
        .expect("encode R7K process log");
        let log_root = composition_sha256_bytes_v1(&log_bytes);
        let log_dir = fixture.root.join("logs");
        create_private_directory(&log_dir);
        let log_path = log_dir.join(format!("{control_id}.json"));
        fs::write(&log_path, &log_bytes).expect("write immutable R7K process log");
        fs::set_permissions(&log_path, fs::Permissions::from_mode(0o400))
            .expect("chmod immutable R7K process log");
        outcomes.push(
            K2UncertaintyControlProcessOutcomeV1::seal(
                K2UncertaintyControlScopeV1::DevelopmentRehearsalV5,
                control_id.to_owned(),
                experiment_root.clone(),
                Some(freeze_root.clone()),
                None,
                runner_sha256.clone(),
                test_sha256.clone(),
                request.request_root_sha256,
                process.normal_exit,
                process.exit_code,
                process.stdout,
                composition_sha256_bytes_v1(&process.stderr),
                process.timed_out,
                contains_subslice(&process.stderr, b"panicked at"),
                decoded.disposition,
                composition_root_v1(&(target_source_root, adapter_source_root.clone()))
                    .expect("R7K source artifact root"),
                log_root,
            )
            .expect("seal measured R7K process outcome"),
        );
    }
    assert_eq!(outcomes.len(), 12);
    let measured_outcomes = outcomes.clone();
    let evaluation_request = K2UncertaintyControlEvaluationRequestV1::seal(
        K2UncertaintyControlScopeV1::DevelopmentRehearsalV5,
        experiment_root.clone(),
        Some(freeze_root.clone()),
        None,
        outcomes,
        evaluator_sha256.clone(),
    )
    .expect("seal real R7K control evaluation request");
    let evaluation_stdout = run_self_formed_confirm_sandbox_v1(
        K2UncertaintyConfirmGuestExecutableV1::Safety,
        &evaluator,
        &evaluator_sha256,
        &[],
        &uncertainty_bytes_v1(&evaluation_request).expect("encode R7K evaluation request"),
        60,
    )
    .expect("run independent R7J control evaluator");
    let evaluation: K2UncertaintyControlEvaluationReceiptV1 =
        uncertainty_decode_v1(&evaluation_stdout).expect("decode R7J control evaluation");
    assert!(evaluation.all_pass);
    assert_eq!(evaluation.passed, 12);
    assert_eq!(evaluation.expected, 12);

    let mut controls = static_predecessor_control_receipts(
        &evaluator,
        &evaluator_sha256,
        &experiment_root,
        &runner_sha256,
        &test_sha256,
    );
    controls.push(evaluation.clone());
    let oracle_batch: K2UncertaintyOracleBaselineBatchReceiptV1 =
        decode_r7j_fixture("fixture-packet/oracle-batch.json");
    let routes: Vec<K2UncertaintyEvaluationRouteReceiptV1> =
        decode_r7j_fixture("fixture-packet/routes.json");
    let resources: K2UncertaintyEvaluationResourceMeasurementsV1 =
        decode_r7j_fixture("fixture-packet/resources.json");
    let terminal = PathBuf::from(env!(
        "CARGO_BIN_EXE_nando-k2-self-formed-terminal-evaluator"
    ));
    let terminal_sha256 = composition_sha256_file_v1(&terminal).expect("R7J terminal SHA");
    let terminal_request = K2UncertaintyDevelopmentRehearsalTerminalRequestV1::seal(
        experiment_root.clone(),
        oracle_batch,
        controls,
        routes,
        resources,
        terminal_sha256.clone(),
    )
    .expect("seal R7K Development terminal request");
    let terminal_stdout = run_self_formed_confirm_sandbox_v1(
        K2UncertaintyConfirmGuestExecutableV1::Safety,
        &terminal,
        &terminal_sha256,
        &[],
        &uncertainty_bytes_v1(&K2UncertaintyTerminalProcessRequestV1::Development {
            request: terminal_request,
        })
        .expect("encode R7K Development terminal request"),
        60,
    )
    .expect("run independent R7J terminal evaluator");
    let terminal_receipt: K2UncertaintyTerminalEvaluationReceiptV1 =
        uncertainty_decode_v1(&terminal_stdout).expect("decode R7K Development terminal receipt");
    assert_eq!(
        terminal_receipt.disposition,
        K2UncertaintyTerminalDispositionV1::DevelopmentRehearsalPass
    );
    assert_eq!(
        terminal_receipt.reason,
        "development_component_routes_complete"
    );

    complete_r7k_cleanup_route(
        &fixture,
        &experiment_root,
        &test_sha256,
        &evaluation,
        &terminal_receipt,
        &measured_outcomes,
    );
}

#[test]
fn r7k_rejects_incomplete_or_substituted_predecessor_packet() {
    let fixture = ProcessFixture::new();
    let packet = fixture.root.join("fixture-packet");
    create_private_directory(&packet);
    let payloads = R7J_FIXTURE_PAYLOADS
        .iter()
        .map(|relative_path| {
            (
                (*relative_path).to_owned(),
                format!("R7J fixture payload: {relative_path}").into_bytes(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for (relative_path, bytes) in &payloads {
        write_durable_read_only(&packet.join(relative_path), bytes);
    }
    let manifest = payloads
        .iter()
        .map(|(relative_path, bytes)| (relative_path.clone(), composition_sha256_bytes_v1(bytes)))
        .collect::<BTreeMap<_, _>>();
    write_durable_read_only(
        &packet.join("fixture-manifest.json"),
        &uncertainty_bytes_v1(&manifest).expect("encode R7J fixture manifest"),
    );
    validate_r7j_fixture_packet_at(&fixture.root).expect("complete R7J packet");

    let omitted = packet.join(R7J_FIXTURE_PAYLOADS[0]);
    fs::remove_file(&omitted).expect("remove one R7J payload");
    assert_eq!(
        validate_r7j_fixture_packet_at(&fixture.root).expect_err("omission must fail"),
        "r7j_fixture_packet_path_set_mismatch"
    );
    write_durable_read_only(
        &omitted,
        payloads
            .get(R7J_FIXTURE_PAYLOADS[0])
            .expect("omitted payload bytes"),
    );

    let substituted = packet.join(R7J_FIXTURE_PAYLOADS[1]);
    fs::remove_file(&substituted).expect("remove substitutable R7J payload");
    write_durable_read_only(&substituted, b"substituted R7J payload");
    assert_eq!(
        validate_r7j_fixture_packet_at(&fixture.root).expect_err("substitution must fail"),
        "r7j_fixture_packet_hash_mismatch"
    );
}

fn complete_r7k_cleanup_route(
    fixture: &ProcessFixture,
    experiment_root: &str,
    test_sha256: &str,
    evaluation: &K2UncertaintyControlEvaluationReceiptV1,
    terminal: &K2UncertaintyTerminalEvaluationReceiptV1,
    outcomes: &[K2UncertaintyControlProcessOutcomeV1],
) {
    let authorizer = PathBuf::from(env!(
        "CARGO_BIN_EXE_nando-k2-self-formed-cleanup-authorizer"
    ));
    let owner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-cleanup-owner"));
    let verifier = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-cleanup-verifier"));
    let publisher = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-result-publisher"));
    let authorizer_sha256 = composition_sha256_file_v1(&authorizer).expect("authorizer SHA");
    let owner_sha256 = composition_sha256_file_v1(&owner).expect("owner SHA");
    let verifier_sha256 = composition_sha256_file_v1(&verifier).expect("verifier SHA");
    let publisher_sha256 = composition_sha256_file_v1(&publisher).expect("publisher SHA");
    let owner_hashes = BTreeSet::from([
        authorizer_sha256.clone(),
        owner_sha256.clone(),
        verifier_sha256.clone(),
        publisher_sha256.clone(),
    ]);
    assert_eq!(owner_hashes.len(), 4, "R7K owner binaries must be distinct");

    let governed_root = fixture.root.join("governed");
    let control_root = fixture.root.join("control");
    create_private_directory(&governed_root);
    create_private_directory(&control_root);
    let outcomes_root = governed_root.join("process-outcomes");
    let logs_root = governed_root.join("process-logs");
    let scratch_root = governed_root.join("scratch");
    create_private_directory(&outcomes_root);
    create_private_directory(&logs_root);
    create_private_directory(&scratch_root);

    write_durable_read_only(
        &governed_root.join("terminal-receipt.json"),
        &uncertainty_bytes_v1(terminal).expect("terminal receipt bytes"),
    );
    write_durable_read_only(
        &governed_root.join("control-evaluation-receipt.json"),
        &uncertainty_bytes_v1(evaluation).expect("control evaluation bytes"),
    );
    for outcome in outcomes {
        write_durable_read_only(
            &outcomes_root.join(format!("{}.json", outcome.control_id)),
            &uncertainty_bytes_v1(outcome).expect("process outcome bytes"),
        );
        let log = fs::read(
            fixture
                .root
                .join("logs")
                .join(format!("{}.json", outcome.control_id)),
        )
        .expect("read measured process log");
        write_durable_read_only(
            &logs_root.join(format!("{}.json", outcome.control_id)),
            &log,
        );
    }
    write_durable_private(
        &scratch_root.join("temporary-workspace.bin"),
        b"R7K disposable workspace",
    );

    let retained_before = retained_identity_snapshot(&governed_root);
    let registry = cleanup_registry(&governed_root, outcomes, terminal, evaluation, test_sha256);
    let (manifest, pages) = census_self_formed_cleanup_artifacts_v1(
        &governed_root,
        experiment_root.to_owned(),
        registry,
        test_sha256.to_owned(),
    )
    .expect("R7K before census");
    publish_self_formed_cleanup_manifest_v1(&governed_root, &control_root, &manifest, &pages)
        .expect("publish R7K before census");
    publish_self_formed_cleanup_manifest_v1(&governed_root, &control_root, &manifest, &pages)
        .expect("restart R7K before-census publication");

    let authorization_request = K2UncertaintyCleanupAuthorizationRequestV1::seal(
        "/control".to_owned(),
        experiment_root.to_owned(),
        terminal.clone(),
        manifest.clone(),
        root("zero-attempt-journal-projection"),
        root("development-observer-durable"),
        root("development-terminal-durable"),
        authorizer_sha256.clone(),
    )
    .expect("cleanup authorization request");
    let authorization: K2UncertaintyCleanupAuthorizationReceiptV1 = run_cleanup_process(
        K2UncertaintyR7kCleanupGuestV1::Authorizer,
        &authorizer,
        &authorizer_sha256,
        None,
        &control_root,
        &authorization_request,
    );
    let authorization_after_restart: K2UncertaintyCleanupAuthorizationReceiptV1 =
        run_cleanup_process(
            K2UncertaintyR7kCleanupGuestV1::Authorizer,
            &authorizer,
            &authorizer_sha256,
            None,
            &control_root,
            &authorization_request,
        );
    assert_eq!(authorization_after_restart, authorization);

    let owner_request = K2UncertaintyCleanupOwnerRequestV1::seal(
        "/governed".to_owned(),
        "/control".to_owned(),
        authorization.clone(),
        owner_sha256.clone(),
    )
    .expect("cleanup owner request");
    let owner_receipt: K2UncertaintyCleanupOwnerReceiptV1 = run_cleanup_process(
        K2UncertaintyR7kCleanupGuestV1::Owner,
        &owner,
        &owner_sha256,
        Some(&governed_root),
        &control_root,
        &owner_request,
    );
    let owner_after_restart: K2UncertaintyCleanupOwnerReceiptV1 = run_cleanup_process(
        K2UncertaintyR7kCleanupGuestV1::Owner,
        &owner,
        &owner_sha256,
        Some(&governed_root),
        &control_root,
        &owner_request,
    );
    assert_eq!(owner_after_restart, owner_receipt);

    let verify_request = K2UncertaintyCleanupVerifyRequestV1::seal(
        "/governed".to_owned(),
        "/control".to_owned(),
        manifest,
        owner_receipt.clone(),
        verifier_sha256.clone(),
    )
    .expect("cleanup verifier request");
    let cleanup_receipt: K2UncertaintyCleanupReceiptV1 = run_cleanup_process(
        K2UncertaintyR7kCleanupGuestV1::Verifier,
        &verifier,
        &verifier_sha256,
        Some(&governed_root),
        &control_root,
        &verify_request,
    );
    assert!(cleanup_receipt.cleanup_frozen);
    assert_eq!(cleanup_receipt.unexpected_residue, 0);
    assert_eq!(cleanup_receipt.deleted_paths, 2);
    assert_eq!(retained_identity_snapshot(&governed_root), retained_before);
    assert!(!scratch_root.exists());
    let cleanup_after_restart: K2UncertaintyCleanupReceiptV1 = run_cleanup_process(
        K2UncertaintyR7kCleanupGuestV1::Verifier,
        &verifier,
        &verifier_sha256,
        Some(&governed_root),
        &control_root,
        &verify_request,
    );
    assert_eq!(cleanup_after_restart, cleanup_receipt);

    let result_request = K2UncertaintyDevelopmentResultRequestV1::seal(
        "/control".to_owned(),
        terminal.clone(),
        cleanup_receipt.clone(),
        publisher_sha256.clone(),
    )
    .expect("Development result request");
    let result: K2UncertaintyDevelopmentResultReceiptV1 = run_cleanup_process(
        K2UncertaintyR7kCleanupGuestV1::ResultPublisher,
        &publisher,
        &publisher_sha256,
        None,
        &control_root,
        &K2UncertaintyResultProcessRequestV1::Development {
            request: result_request,
        },
    );
    assert_eq!(result.disposition, "DEVELOPMENT_REHEARSAL_COMPLETE");
    assert_eq!(
        result.terminal_receipt_root_sha256,
        terminal.receipt_root_sha256
    );
    assert_eq!(
        result.cleanup_receipt_root_sha256,
        cleanup_receipt.receipt_root_sha256
    );
    let result_after_restart: K2UncertaintyDevelopmentResultReceiptV1 = run_cleanup_process(
        K2UncertaintyR7kCleanupGuestV1::ResultPublisher,
        &publisher,
        &publisher_sha256,
        None,
        &control_root,
        &K2UncertaintyResultProcessRequestV1::Development {
            request: K2UncertaintyDevelopmentResultRequestV1::seal(
                "/control".to_owned(),
                terminal.clone(),
                cleanup_receipt.clone(),
                publisher_sha256.clone(),
            )
            .expect("restart Development result request"),
        },
    );
    assert_eq!(result_after_restart, result);
    assert_control_root_complete(
        &control_root,
        &pages,
        &owner_receipt,
        &cleanup_receipt,
        &result,
    );

    let sealed = K2UncertaintyResultProcessRequestV1::Sealed {
        request: K2UncertaintySealedResultRequestV1 {
            schema: K2_UNCERTAINTY_SEALED_RESULT_REQUEST_SCHEMA_V1.to_owned(),
            terminal_receipt: terminal.clone(),
            cleanup_receipt,
            publisher_executable_sha256: publisher_sha256.clone(),
        },
    };
    assert!(
        run_self_formed_r7k_cleanup_sandbox_v1(
            K2UncertaintyR7kCleanupGuestV1::ResultPublisher,
            &publisher,
            &publisher_sha256,
            None,
            &control_root,
            &uncertainty_bytes_v1(&sealed).expect("sealed substitution bytes"),
            60,
        )
        .is_err(),
        "cross-mode sealed substitution must fail"
    );
}

fn cleanup_registry(
    governed_root: &Path,
    outcomes: &[K2UncertaintyControlProcessOutcomeV1],
    terminal: &K2UncertaintyTerminalEvaluationReceiptV1,
    evaluation: &K2UncertaintyControlEvaluationReceiptV1,
    test_sha256: &str,
) -> Vec<K2UncertaintyCleanupRegistryEntryV1> {
    let mut producers = BTreeMap::from([
        (
            "terminal-receipt.json".to_owned(),
            terminal.evaluator_executable_sha256.clone(),
        ),
        (
            "control-evaluation-receipt.json".to_owned(),
            evaluation.evaluator_executable_sha256.clone(),
        ),
        ("process-outcomes".to_owned(), test_sha256.to_owned()),
        ("process-logs".to_owned(), test_sha256.to_owned()),
        ("scratch".to_owned(), test_sha256.to_owned()),
        (
            "scratch/temporary-workspace.bin".to_owned(),
            test_sha256.to_owned(),
        ),
    ]);
    for outcome in outcomes {
        producers.insert(
            format!("process-outcomes/{}.json", outcome.control_id),
            test_sha256.to_owned(),
        );
        producers.insert(
            format!("process-logs/{}.json", outcome.control_id),
            test_sha256.to_owned(),
        );
    }
    let observed = governed_identity_snapshot(governed_root, true);
    assert_eq!(
        producers.keys().cloned().collect::<BTreeSet<_>>(),
        observed.keys().cloned().collect::<BTreeSet<_>>(),
        "artifact registry must cover the complete governed tree"
    );
    producers
        .into_iter()
        .map(|(relative_path, producer_executable_sha256)| {
            let artifact_kind =
                if relative_path == "scratch" || relative_path.starts_with("scratch/") {
                    K2UncertaintyCleanupArtifactKindV1::DisposableWorkspace
                } else if relative_path.starts_with("process-") {
                    K2UncertaintyCleanupArtifactKindV1::SealedPrivateEvidence
                } else {
                    K2UncertaintyCleanupArtifactKindV1::RetainedEvidence
                };
            K2UncertaintyCleanupRegistryEntryV1 {
                producing_journal_event_root_sha256: root(&format!(
                    "durable-governed-artifact:{relative_path}"
                )),
                relative_path,
                artifact_kind,
                producer_executable_sha256,
            }
        })
        .collect()
}

fn retained_identity_snapshot(
    governed_root: &Path,
) -> BTreeMap<String, (Option<String>, u32, u64)> {
    governed_identity_snapshot(governed_root, false)
}

fn governed_identity_snapshot(
    governed_root: &Path,
    include_disposable: bool,
) -> BTreeMap<String, (Option<String>, u32, u64)> {
    let mut pending = vec![governed_root.to_path_buf()];
    let mut snapshot = BTreeMap::new();
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)
            .expect("read governed snapshot directory")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect governed snapshot directory");
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).expect("stat governed snapshot path");
            assert!(!metadata.file_type().is_symlink());
            let relative = path
                .strip_prefix(governed_root)
                .expect("governed relative path")
                .to_string_lossy()
                .into_owned();
            if !include_disposable && (relative == "scratch" || relative.starts_with("scratch/")) {
                if metadata.is_dir() {
                    pending.push(path);
                }
                continue;
            }
            let content = if metadata.is_file() {
                Some(composition_sha256_file_v1(&path).expect("hash governed retained path"))
            } else {
                assert!(metadata.is_dir(), "special governed path forbidden");
                pending.push(path);
                None
            };
            snapshot.insert(
                relative,
                (
                    content,
                    metadata.permissions().mode() & 0o7777,
                    metadata.len(),
                ),
            );
        }
    }
    snapshot
}

fn run_cleanup_process<Request, Receipt>(
    role: K2UncertaintyR7kCleanupGuestV1,
    executable: &Path,
    executable_sha256: &str,
    governed_root: Option<&Path>,
    control_root: &Path,
    request: &Request,
) -> Receipt
where
    Request: serde::Serialize,
    Receipt: serde::de::DeserializeOwned + serde::Serialize,
{
    let stdout = run_self_formed_r7k_cleanup_sandbox_v1(
        role,
        executable,
        executable_sha256,
        governed_root,
        control_root,
        &uncertainty_bytes_v1(request).expect("encode R7K owner request"),
        60,
    )
    .expect("run R7K owner wrapper");
    uncertainty_decode_v1(&stdout).expect("decode R7K owner receipt")
}

fn assert_control_root_complete(
    control_root: &Path,
    pages: &[nando_operator_learning::K2UncertaintyCleanupManifestPageV1],
    owner: &K2UncertaintyCleanupOwnerReceiptV1,
    cleanup: &K2UncertaintyCleanupReceiptV1,
    result: &K2UncertaintyDevelopmentResultReceiptV1,
) {
    let mut expected = BTreeSet::from([
        "cleanup-manifest.json".to_owned(),
        "cleanup-authorization.json".to_owned(),
        "cleanup-owner-receipt.json".to_owned(),
        "cleanup-frozen.json".to_owned(),
        "development-rehearsal-result.json".to_owned(),
    ]);
    for page in pages {
        expected.insert(format!(
            "cleanup-manifest-pages/{}.json",
            page.page_root_sha256
        ));
    }
    let after_pages = fs::read_dir(control_root.join("cleanup-after-census-pages"))
        .expect("read durable after-census pages")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect durable after-census pages");
    assert!(
        !after_pages.is_empty(),
        "after-census pages must be durable"
    );
    for page in after_pages {
        expected.insert(format!(
            "cleanup-after-census-pages/{}",
            page.file_name().to_string_lossy()
        ));
    }
    for event in &owner.events {
        expected.insert(format!("cleanup-events/{:020}.json", event.sequence));
    }
    let actual = control_file_set(control_root);
    assert_eq!(actual, expected, "control root must be complete and exact");
    assert_eq!(
        fs::read(control_root.join("cleanup-frozen.json")).expect("read CleanupFrozen"),
        uncertainty_bytes_v1(cleanup).expect("CleanupFrozen bytes")
    );
    assert_eq!(
        fs::read(control_root.join("development-rehearsal-result.json"))
            .expect("read Development result"),
        uncertainty_bytes_v1(result).expect("Development result bytes")
    );
}

fn control_file_set(root: &Path) -> BTreeSet<String> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = BTreeSet::new();
    while let Some(directory) = pending.pop() {
        let metadata = fs::symlink_metadata(&directory).expect("stat control directory");
        assert!(metadata.is_dir());
        assert_eq!(metadata.permissions().mode() & 0o7777, 0o700);
        for entry in fs::read_dir(&directory).expect("read control directory") {
            let path = entry.expect("control entry").path();
            let metadata = fs::symlink_metadata(&path).expect("stat control entry");
            assert!(!metadata.file_type().is_symlink());
            if metadata.is_dir() {
                pending.push(path);
            } else {
                assert!(metadata.is_file());
                assert_eq!(metadata.permissions().mode() & 0o7777, 0o400);
                files.insert(
                    path.strip_prefix(root)
                        .expect("control relative path")
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    files
}

fn write_durable_read_only(path: &Path, bytes: &[u8]) {
    write_durable(path, bytes, 0o400);
}

fn write_durable_private(path: &Path, bytes: &[u8]) {
    write_durable(path, bytes, 0o600);
}

fn write_durable(path: &Path, bytes: &[u8], mode: u32) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .expect("create durable R7K artifact");
    file.write_all(bytes).expect("write durable R7K artifact");
    file.sync_all().expect("fsync durable R7K artifact");
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .expect("chmod durable R7K artifact");
    File::open(path.parent().expect("durable artifact parent"))
        .and_then(|directory| directory.sync_all())
        .expect("fsync durable R7K parent");
}

fn run_control_process(
    executable: &Path,
    executable_sha256: &str,
    scratch_root: &Path,
    request: &K2UncertaintyR7kControlCaseRequestV1,
    argv: &[String],
    environment: &[(String, String)],
) -> K2UncertaintySandboxProcessOutcomeV1 {
    let outcome = run_self_formed_r7k_control_sandbox_v1(
        executable,
        executable_sha256,
        scratch_root,
        &uncertainty_bytes_v1(request).expect("encode R7K control request"),
        argv,
        environment,
        60,
    )
    .expect("run measured R7K control sandbox");
    assert!(outcome.normal_exit);
    assert_eq!(
        outcome.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&outcome.stderr)
    );
    assert!(!outcome.timed_out);
    assert!(outcome.stderr.is_empty());
    outcome
}

fn static_predecessor_control_receipts(
    evaluator: &Path,
    evaluator_sha256: &str,
    experiment_root: &str,
    runner_sha256: &str,
    test_sha256: &str,
) -> Vec<K2UncertaintyControlEvaluationReceiptV1> {
    [
        K2UncertaintyControlScopeV1::SuccessorStaticLegacy,
        K2UncertaintyControlScopeV1::SuccessorStaticV3,
        K2UncertaintyControlScopeV1::SuccessorStaticV4,
    ]
    .into_iter()
    .map(|scope| {
        let outcomes = (0..scope.expected_count())
            .map(|ordinal| {
                let (control_id, disposition) =
                    expected_self_formed_control_v1(scope, ordinal).expect("static control row");
                let stdout = uncertainty_bytes_v1(&K2UncertaintyControlStdoutV1 {
                    control_id: control_id.clone(),
                    disposition: disposition.clone(),
                })
                .expect("static control stdout");
                K2UncertaintyControlProcessOutcomeV1::seal(
                    scope,
                    control_id,
                    experiment_root.to_owned(),
                    None,
                    None,
                    runner_sha256.to_owned(),
                    test_sha256.to_owned(),
                    root(&format!("static-request-{scope:?}-{ordinal}")),
                    true,
                    0,
                    stdout,
                    composition_sha256_bytes_v1(&[]),
                    false,
                    false,
                    disposition,
                    root(&format!("static-source-{scope:?}-{ordinal}")),
                    root(&format!("static-log-{scope:?}-{ordinal}")),
                )
                .expect("static predecessor control outcome")
            })
            .collect();
        let request = K2UncertaintyControlEvaluationRequestV1::seal(
            scope,
            experiment_root.to_owned(),
            None,
            None,
            outcomes,
            evaluator_sha256.to_owned(),
        )
        .expect("static predecessor control request");
        let stdout = run_self_formed_confirm_sandbox_v1(
            K2UncertaintyConfirmGuestExecutableV1::Safety,
            evaluator,
            evaluator_sha256,
            &[],
            &uncertainty_bytes_v1(&request).expect("encode static control request"),
            60,
        )
        .expect("run static predecessor control evaluator");
        uncertainty_decode_v1(&stdout).expect("decode static predecessor control receipt")
    })
    .collect()
}

fn validate_r7j_fixture_packet() -> Result<(), &'static str> {
    let root = std::env::var_os("NANDO_K2_R7K_FIXTURE_ROOT")
        .map(PathBuf::from)
        .ok_or("r7j_fixture_packet_root_missing")?;
    validate_r7j_fixture_packet_at(&root)
}

fn validate_r7j_fixture_packet_at(root: &Path) -> Result<(), &'static str> {
    let root_metadata =
        fs::symlink_metadata(root).map_err(|_| "r7j_fixture_packet_root_missing")?;
    if !root.is_absolute()
        || !root_metadata.is_dir()
        || root_metadata.file_type().is_symlink()
        || root_metadata.permissions().mode() & 0o7777 != 0o700
    {
        return Err("r7j_fixture_packet_root_invalid");
    }
    let packet = root.join("fixture-packet");
    let packet_metadata =
        fs::symlink_metadata(&packet).map_err(|_| "r7j_fixture_packet_directory_missing")?;
    if !packet_metadata.is_dir()
        || packet_metadata.file_type().is_symlink()
        || packet_metadata.permissions().mode() & 0o7777 != 0o700
    {
        return Err("r7j_fixture_packet_directory_invalid");
    }

    let expected_paths = R7J_FIXTURE_PAYLOADS
        .iter()
        .copied()
        .chain(std::iter::once("fixture-manifest.json"))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let mut observed_paths = BTreeSet::new();
    for entry in fs::read_dir(&packet).map_err(|_| "r7j_fixture_packet_directory_unreadable")? {
        let entry = entry.map_err(|_| "r7j_fixture_packet_entry_unreadable")?;
        let relative_path = entry
            .file_name()
            .into_string()
            .map_err(|_| "r7j_fixture_packet_path_not_utf8")?;
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|_| "r7j_fixture_packet_entry_unreadable")?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.permissions().mode() & 0o7777 != 0o400
        {
            return Err("r7j_fixture_packet_entry_invalid");
        }
        observed_paths.insert(relative_path);
    }
    if observed_paths != expected_paths {
        return Err("r7j_fixture_packet_path_set_mismatch");
    }

    let manifest: BTreeMap<String, String> = uncertainty_decode_v1(
        &fs::read(packet.join("fixture-manifest.json"))
            .map_err(|_| "r7j_fixture_manifest_unreadable")?,
    )
    .map_err(|_| "r7j_fixture_manifest_invalid")?;
    let expected_payloads = R7J_FIXTURE_PAYLOADS
        .iter()
        .map(|relative_path| (*relative_path).to_owned())
        .collect::<BTreeSet<_>>();
    if manifest.keys().cloned().collect::<BTreeSet<_>>() != expected_payloads {
        return Err("r7j_fixture_manifest_path_set_mismatch");
    }
    for relative_path in R7J_FIXTURE_PAYLOADS {
        let bytes = fs::read(packet.join(relative_path))
            .map_err(|_| "r7j_fixture_packet_payload_unreadable")?;
        if manifest.get(relative_path) != Some(&composition_sha256_bytes_v1(&bytes)) {
            return Err("r7j_fixture_packet_hash_mismatch");
        }
    }
    Ok(())
}

fn decode_r7j_fixture<T>(relative_path: &str) -> T
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let root = std::env::var_os("NANDO_K2_R7K_FIXTURE_ROOT")
        .map(PathBuf::from)
        .expect("NANDO_K2_R7K_FIXTURE_ROOT is required for R7K terminal");
    uncertainty_decode_v1(
        &fs::read(root.join(relative_path)).expect("read persisted R7J fixture packet"),
    )
    .expect("decode persisted R7J fixture packet")
}

struct ProcessFixture {
    root: PathBuf,
}

impl ProcessFixture {
    fn new() -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nando-k2-self-formed-r7k-process-{}-{sequence}",
            std::process::id()
        ));
        create_private_directory(&root);
        Self { root }
    }
}

impl Drop for ProcessFixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!(
                "R7K failed process fixture retained at {}",
                self.root.display()
            );
        } else {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn create_private_directory(path: &Path) {
    fs::create_dir_all(path).expect("create R7K private directory");
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .expect("chmod R7K private directory");
}

fn canary_representations() -> Vec<Vec<u8>> {
    [
        b"z~z~z~z~z~z~z~z~z~z~z~z~z~z~z~z~".as_slice(),
        b"7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e".as_slice(),
        b"7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E".as_slice(),
        b"en56fnp+en56fnp+en56fnp+en56fnp+en56fnp+en4".as_slice(),
        b"en56fnp+en56fnp+en56fnp+en56fnp+en56fnp+en4=".as_slice(),
        b"856a844c677c7623f8004621d1dcd5b584f03de2909f4686eb57594227851502".as_slice(),
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect()
}

fn adapter_source_root() -> String {
    source_manifest_root(&[
        "src/bin/nando-k2-self-formed-r7k-control-case.rs",
        "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_control_cases.rs",
    ])
}

fn control_target_source_root(control_id: &str) -> String {
    let paths: &[&str] = match control_id {
        "K1" => &[
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/generator_model.rs",
        ],
        "K2" => &[
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_owner_model.rs",
        ],
        "K3" | "K4" => &[
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_control_canary.rs",
        ],
        "K5" => &[
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_private_resolver.rs",
        ],
        "K6" => &[
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/observation_vector_v2.rs",
        ],
        "K7" => &[
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_public_coordinator.rs",
        ],
        "K8" => &[
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_authorization.rs",
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_attempt_model.rs",
        ],
        "K9" => &[
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_evaluation_model.rs",
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_terminal.rs",
        ],
        "K10" => &[
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_oracle_process.rs",
        ],
        "K11" => &[
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_evaluation_model.rs",
        ],
        "K12" => &[
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_cleanup_verifier.rs",
        ],
        _ => panic!("unknown R7K control source: {control_id}"),
    };
    source_manifest_root(paths)
}

fn source_manifest_root(relative_paths: &[&str]) -> String {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rows = relative_paths
        .iter()
        .map(|relative_path| {
            (
                *relative_path,
                composition_sha256_file_v1(&crate_root.join(relative_path))
                    .expect("hash R7K source artifact"),
            )
        })
        .collect::<Vec<_>>();
    composition_root_v1(&rows).expect("R7K source manifest root")
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn copy_fixture_tree(source: &Path, destination: &Path) {
    create_private_directory(destination);
    let mut entries = fs::read_dir(source)
        .expect("read R7K fixture directory")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect R7K fixture directory");
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path).expect("stat R7K fixture entry");
        assert!(
            !metadata.file_type().is_symlink(),
            "fixture symlink forbidden"
        );
        if metadata.is_dir() {
            copy_fixture_tree(&source_path, &destination_path);
        } else {
            assert!(metadata.is_file(), "fixture special file forbidden");
            fs::copy(&source_path, &destination_path).expect("copy R7K fixture file");
            fs::set_permissions(&destination_path, fs::Permissions::from_mode(0o400))
                .expect("chmod R7K fixture file");
        }
    }
}

fn root(label: &str) -> String {
    composition_root_v1(&("nando.k2-self-formed-r7k-process-test.v1", label))
        .expect("R7K test root")
}
