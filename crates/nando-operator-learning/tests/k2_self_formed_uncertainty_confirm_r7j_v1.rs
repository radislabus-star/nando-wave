use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use nando_operator_learning::{
    K2_UNCERTAINTY_ORACLE_DESCRIPTOR_SCHEMA_V1, K2_UNCERTAINTY_ORACLE_MANIFEST_FILE_V1,
    K2_UNCERTAINTY_RAW_PROBES_V1, K2CompositionTreeManifestV1, K2InquiryObservationReceiptV1,
    K2InquiryWorkerOutcomeV1, K2UncertaintyCaseJournalFaultV2, K2UncertaintyCaseJournalV2,
    K2UncertaintyConfirmDataMountV1, K2UncertaintyConfirmFinalTruthCaseV1,
    K2UncertaintyConfirmFinalVerifierReceiptV1, K2UncertaintyConfirmFinalVerifierRequestV1,
    K2UncertaintyConfirmGeneratorRequestV1, K2UncertaintyConfirmGeneratorResponseV1,
    K2UncertaintyConfirmGuestExecutableV1, K2UncertaintyConfirmMountTargetV1,
    K2UncertaintyConfirmPlanSafetyBindingV1, K2UncertaintyConfirmPrivateSplitReceiptV1,
    K2UncertaintyConfirmPublicDenominatorReceiptV1, K2UncertaintyConfirmSafetyReceiptV1,
    K2UncertaintyConfirmSafetyRequestV1, K2UncertaintyConfirmStoredArtifactKindV1,
    K2UncertaintyControlEvaluationReceiptV1, K2UncertaintyControlEvaluationRequestV1,
    K2UncertaintyControlProcessOutcomeV1, K2UncertaintyControlScopeV1,
    K2UncertaintyControlStdoutV1, K2UncertaintyDevelopmentRehearsalTerminalRequestV1,
    K2UncertaintyEvaluationResourceMeasurementsV1, K2UncertaintyEvaluationRouteReceiptV1,
    K2UncertaintyObservationVectorV2, K2UncertaintyOracleBaselineBatchReceiptV1,
    K2UncertaintyOracleBaselineCaseDescriptorV1, K2UncertaintyOracleBaselineCaseReceiptV1,
    K2UncertaintyOracleCaseEvidenceManifestV1, K2UncertaintyOracleEvidenceEntryV1,
    K2UncertaintyOracleEvidenceKindV1, K2UncertaintyOraclePublicBindingsV1,
    K2UncertaintyPrivateResolverReceiptV1, K2UncertaintyPrivateResolverRequestV1,
    K2UncertaintyProbeExecutionEvidenceV2, K2UncertaintyProbeOutputV1, K2UncertaintyPublicBatchV1,
    K2UncertaintyPublicCoordinatorRequestV1, K2UncertaintyPublicOwnerRoleV1,
    K2UncertaintyPublicOwnerSetV1, K2UncertaintyPublicOwnerV1,
    K2UncertaintyPublicPrecommitReceiptV1, K2UncertaintyPublicPreparedCaseV1,
    K2UncertaintySealedProjectionV1, K2UncertaintySealedTerminalRequestV1,
    K2UncertaintyTerminalDispositionV1, K2UncertaintyTerminalEvaluationReceiptV1,
    K2UncertaintyTerminalProcessRequestV1, K2UncertaintyWorkspaceIdentityV2, composition_root_v1,
    composition_sha256_bytes_v1, composition_sha256_file_v1, expected_self_formed_control_v1,
    load_confirm_generator_split_receipt_v1, load_self_formed_public_case_v1,
    load_self_formed_public_precommit_v1, materialize_self_formed_probe_files_v1,
    prepare_self_formed_confirm_plan_dispatch_v1, publish_confirm_generator_split_v1,
    publish_self_formed_final_verifier_material_v2, reopen_self_formed_probe_output_v1,
    run_self_formed_confirm_sandbox_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn r7j_independently_reconstructs_oracle_baselines_controls_and_terminal() {
    let batch_started = Instant::now();
    let environment = TestEnvironment::new("route");
    let binaries = ProcessBinaries::from_cargo();
    binaries.assert_pairwise_distinct();

    let reuse = std::env::var_os("NANDO_K2_R7J_REUSE_FIXTURE_ROOT").map(PathBuf::from);
    let split_root = reuse
        .as_ref()
        .map_or_else(|| environment.root.join("split"), |root| root.join("split"));
    let public_root = reuse.as_ref().map_or_else(
        || environment.root.join("public-precommit"),
        |root| root.join("public-precommit"),
    );
    let (split, public_batch, public_receipt) = if reuse.is_some() {
        (
            load_confirm_generator_split_receipt_v1(&split_root).expect("reopen Confirm split"),
            uncertainty_decode_v1::<K2UncertaintyPublicBatchV1>(
                &fs::read(split_root.join("public/public-batch.json")).expect("read public batch"),
            )
            .expect("decode public batch"),
            load_self_formed_public_precommit_v1(&public_root).expect("reuse public precommit"),
        )
    } else {
        let generator_request = K2UncertaintyConfirmGeneratorRequestV1::seal(
            vec![0xa5; 32],
            root("successor-freeze"),
            root("rehearsal-authorization"),
            binaries.generator.sha256.clone(),
        )
        .expect("confirm rehearsal generator request");
        let generated: K2UncertaintyConfirmGeneratorResponseV1 =
            run_process(&binaries.generator.path, &generator_request);
        let split = publish_confirm_generator_split_v1(&split_root, &generator_request, &generated)
            .expect("publish Confirm rehearsal split");
        let denominator: K2UncertaintyConfirmPublicDenominatorReceiptV1 = uncertainty_decode_v1(
            &fs::read(split_root.join("public/denominator-receipt.json"))
                .expect("read public denominator"),
        )
        .expect("decode public denominator");
        let public_request = K2UncertaintyPublicCoordinatorRequestV1::seal(
            generated.public.clone(),
            denominator,
            binaries.public_owner_set(),
            public_root.to_string_lossy().into_owned(),
            binaries.public_coordinator.sha256.clone(),
        )
        .expect("public coordinator request");
        let receipt: K2UncertaintyPublicPrecommitReceiptV1 =
            run_process(&binaries.public_coordinator.path, &public_request);
        (split, generated.public, receipt)
    };
    let private: K2UncertaintyConfirmPrivateSplitReceiptV1 = uncertainty_decode_v1(
        &fs::read(split_root.join("private/private-split-receipt.json"))
            .expect("read private split receipt"),
    )
    .expect("decode private split receipt");

    assert_eq!(
        load_self_formed_public_precommit_v1(&public_root).expect("reopen public precommit"),
        public_receipt
    );
    assert_eq!(public_receipt.case_artifacts.len(), 16);
    assert_eq!(public_receipt.private_mount_count, 0);
    assert!(public_receipt.all_cases_precommitted);
    assert!(!public_root.join("private").exists());
    for artifact in &public_receipt.case_artifacts {
        let material_root = public_root.join(format!(
            "probes/case-{:02}/final-v2",
            artifact.case_sequence
        ));
        let material_count = fs::read_dir(&material_root)
            .expect("final verifier material precedes private execution")
            .count();
        assert_eq!(material_count, 2);
    }
    assert_eq!(
        split.private_split_root_sha256,
        private.private_split_root_sha256
    );

    let mut plan_lengths = Vec::new();
    let mut oracle_receipts = Vec::new();
    let mut maximum_case_wall_ms = 0_u64;
    let mut maximum_protocol_bytes = 0_u64;
    for artifact in &public_receipt.case_artifacts {
        let prepared = load_self_formed_public_case_v1(&public_root, artifact)
            .expect("reopen prepared public case");
        let case_id = prepared
            .probe_request
            .public_case
            .vocabulary
            .case_id_sha256
            .clone();
        let resolver_artifact = private
            .artifacts
            .iter()
            .find(|entry| {
                entry.kind == K2UncertaintyConfirmStoredArtifactKindV1::ResolverTable
                    && entry.case_id_sha256.as_deref() == Some(case_id.as_str())
            })
            .expect("case resolver artifact");
        let truth_artifact = private
            .artifacts
            .iter()
            .find(|entry| {
                entry.kind == K2UncertaintyConfirmStoredArtifactKindV1::FinalTruth
                    && entry.case_id_sha256.as_deref() == Some(case_id.as_str())
            })
            .expect("case truth artifact");
        let plan = prepared
            .preverification
            .closure_plan
            .as_ref()
            .expect("publicly frozen closure plan");
        plan_lengths.push(plan.plan_length);
        let planner = &prepared
            .preverification
            .closure_verification_request
            .planner_request;
        let mut bindings = Vec::with_capacity(plan.plan_length as usize);
        for (ordinal, probe_root) in plan.ordered_probe_roots_sha256.iter().enumerate() {
            let selected_probe = planner
                .representatives
                .iter()
                .find(|candidate| &candidate.probe.probe_root_sha256 == probe_root)
                .map(|candidate| candidate.probe.clone())
                .expect("planned representative");
            let resolver_request = K2UncertaintyPrivateResolverRequestV1::seal(
                public_batch.experiment_id_sha256.clone(),
                public_batch.public_batch_root_sha256.clone(),
                public_receipt.batch_precommit.batch_root_sha256.clone(),
                prepared.preverification.receipt_root_sha256.clone(),
                prepared
                    .probe_request
                    .public_case
                    .public_case_root_sha256
                    .clone(),
                plan.clone(),
                ordinal as u64,
                selected_probe,
                resolver_artifact.semantic_root_sha256.clone(),
                binaries.private_resolver.sha256.clone(),
            )
            .expect("private resolver request");
            let resolver_path = split_root.join(&resolver_artifact.relative_path);
            let resolver_mount = [K2UncertaintyConfirmDataMountV1 {
                host_path: &resolver_path,
                target: K2UncertaintyConfirmMountTargetV1::ResolverTable,
                writable: false,
            }];
            eprintln!(
                "R7J resolver case {} ordinal {}",
                artifact.case_sequence, ordinal
            );
            let resolver_receipt: K2UncertaintyPrivateResolverReceiptV1 = run_sandbox(
                K2UncertaintyConfirmGuestExecutableV1::PrivateResolver,
                &binaries.private_resolver,
                &resolver_mount,
                &resolver_request,
                20,
            );
            assert_eq!(resolver_receipt.exposed_effect_count, 1);
            let workspace_identity = K2UncertaintyWorkspaceIdentityV2::seal(
                case_id.clone(),
                plan.plan_root_sha256.clone(),
                ordinal as u64,
            )
            .expect("workspace identity");
            let safety_request = K2UncertaintyConfirmSafetyRequestV1::seal(
                resolver_request.clone(),
                resolver_receipt.clone(),
                prepared.probe_request.public_case.vocabulary.clone(),
                workspace_identity,
                binaries.safety.sha256.clone(),
            )
            .expect("Confirm safety request");
            eprintln!(
                "R7J safety case {} ordinal {}",
                artifact.case_sequence, ordinal
            );
            let safety_receipt: K2UncertaintyConfirmSafetyReceiptV1 = run_sandbox(
                K2UncertaintyConfirmGuestExecutableV1::Safety,
                &binaries.safety,
                &[],
                &safety_request,
                20,
            );
            bindings.push(K2UncertaintyConfirmPlanSafetyBindingV1 {
                resolver_request,
                resolver_receipt,
                safety_request,
                safety_receipt,
            });
        }
        let dispatch = prepare_self_formed_confirm_plan_dispatch_v1(
            &public_batch.public_batch_root_sha256,
            &public_receipt.batch_precommit,
            &prepared,
            bindings,
            &binaries.worker.sha256,
            &binaries.observer.sha256,
        )
        .expect("bounded Confirm dispatch");
        let case_journal_root = environment
            .root
            .join(format!("journal-{}", artifact.case_sequence));
        let mut journal = K2UncertaintyCaseJournalV2::create(&case_journal_root, dispatch.clone())
            .expect("create case journal");
        journal
            .record_plan_dispatch(
                binaries.public_coordinator.sha256.clone(),
                K2UncertaintyCaseJournalFaultV2::None,
            )
            .expect("freeze dispatch before worker");
        let mut executions = Vec::with_capacity(dispatch.items.len());
        for item in &dispatch.items {
            let source = environment.root.join(format!(
                "source-{}",
                item.workspace_identity.identity_root_sha256
            ));
            let workspace = environment.root.join(format!(
                "work-{}",
                item.workspace_identity.identity_root_sha256
            ));
            let files = materialize_self_formed_probe_files_v1(
                &prepared.probe_request.public_case,
                &item.selected_probe,
            )
            .expect("materialize selected public state");
            materialize(&source, &files);
            materialize(&workspace, &files);
            let source_before = K2CompositionTreeManifestV1::scan(&source).expect("source before");
            let permit = journal
                .begin_probe_execution(item.probe_ordinal, K2UncertaintyCaseJournalFaultV2::None)
                .expect("begin exact ordinal");
            let worker_mounts = [
                K2UncertaintyConfirmDataMountV1 {
                    host_path: &source,
                    target: K2UncertaintyConfirmMountTargetV1::Source,
                    writable: false,
                },
                K2UncertaintyConfirmDataMountV1 {
                    host_path: &workspace,
                    target: K2UncertaintyConfirmMountTargetV1::Workspace,
                    writable: true,
                },
            ];
            let worker: K2InquiryWorkerOutcomeV1 = run_sandbox(
                K2UncertaintyConfirmGuestExecutableV1::Worker,
                &binaries.worker,
                &worker_mounts,
                &item.worker_request,
                20,
            );
            let observer_mounts = [
                K2UncertaintyConfirmDataMountV1 {
                    host_path: &source,
                    target: K2UncertaintyConfirmMountTargetV1::Source,
                    writable: false,
                },
                K2UncertaintyConfirmDataMountV1 {
                    host_path: &workspace,
                    target: K2UncertaintyConfirmMountTargetV1::Workspace,
                    writable: false,
                },
            ];
            eprintln!(
                "R7J observer case {} ordinal {}",
                artifact.case_sequence, item.probe_ordinal
            );
            let observation: K2InquiryObservationReceiptV1 = run_sandbox(
                K2UncertaintyConfirmGuestExecutableV1::Observer,
                &binaries.observer,
                &observer_mounts,
                &item.observer_request,
                20,
            );
            assert_eq!(worker.post_manifest, observation.post_manifest);
            assert_eq!(
                source_before,
                K2CompositionTreeManifestV1::scan(&source).expect("source after")
            );
            let evidence = K2UncertaintyProbeExecutionEvidenceV2::seal(
                dispatch.dispatch_root_sha256.clone(),
                item,
                worker,
                observation,
            )
            .expect("execution evidence");
            journal
                .record_probe_observation(
                    permit,
                    evidence.observation.receipt_root_sha256.clone(),
                    K2UncertaintyCaseJournalFaultV2::None,
                )
                .expect("freeze observation");
            executions.push(evidence);
        }
        let observation_vector = K2UncertaintyObservationVectorV2::seal(&dispatch, executions)
            .expect("ordered observation vector");
        let vector_request_root = root(&format!("vector-{}", artifact.case_sequence));
        journal
            .freeze_observation_vector(
                binaries.public_coordinator.sha256.clone(),
                vector_request_root,
                observation_vector.vector_root_sha256.clone(),
                K2UncertaintyCaseJournalFaultV2::None,
            )
            .expect("freeze complete observation vector");
        let evidence_root = public_root.join(format!("probes/case-{:02}", artifact.case_sequence));
        let material = publish_self_formed_final_verifier_material_v2(
            &evidence_root,
            &public_receipt.batch_precommit,
            &prepared.preverification,
        )
        .expect("publish final verifier material");
        let final_request = K2UncertaintyConfirmFinalVerifierRequestV1::seal(
            binaries.final_verifier.sha256.clone(),
            material,
            prepared.probe_request.clone(),
            prepared.probe_artifacts.clone(),
            dispatch,
            observation_vector.clone(),
            journal.state().clone(),
            truth_artifact.semantic_root_sha256.clone(),
        )
        .expect("Confirm final verifier request");
        let truth_path = split_root.join(&truth_artifact.relative_path);
        let final_mounts = [
            K2UncertaintyConfirmDataMountV1 {
                host_path: &truth_path,
                target: K2UncertaintyConfirmMountTargetV1::FinalTruth,
                writable: false,
            },
            K2UncertaintyConfirmDataMountV1 {
                host_path: &evidence_root,
                target: K2UncertaintyConfirmMountTargetV1::Evidence,
                writable: false,
            },
        ];
        eprintln!("R7J final verifier case {}", artifact.case_sequence);
        let final_receipt: K2UncertaintyConfirmFinalVerifierReceiptV1 = run_sandbox(
            K2UncertaintyConfirmGuestExecutableV1::FinalVerifier,
            &binaries.final_verifier,
            &final_mounts,
            &final_request,
            120,
        );
        final_receipt.validate().expect("Confirm final receipt");
        assert_eq!(
            final_receipt.final_truth_root_sha256,
            truth_artifact.semantic_root_sha256
        );
        assert!(final_receipt.verification.private_true_class_match);
        assert_eq!(final_receipt.verification.false_accepts, 0);

        let truth: K2UncertaintyConfirmFinalTruthCaseV1 = uncertainty_decode_v1(
            &fs::read(&truth_path).expect("read private truth for manifested oracle evidence"),
        )
        .expect("decode private truth for manifested oracle evidence");
        let probe_output =
            reopen_self_formed_probe_output_v1(&evidence_root, &prepared.probe_artifacts)
                .expect("reopen complete public probe output");
        let public_bindings =
            K2UncertaintyOraclePublicBindingsV1::seal(public_receipt.clone(), prepared.clone())
                .expect("seal oracle public bindings");
        let oracle_evidence_root = environment
            .root
            .join(format!("oracle-case-{:02}", artifact.case_sequence));
        let manifest = publish_oracle_case_evidence(
            &oracle_evidence_root,
            &public_bindings,
            &prepared,
            &probe_output,
            &observation_vector,
            &final_receipt,
            &truth,
        );
        let descriptor = K2UncertaintyOracleBaselineCaseDescriptorV1 {
            schema: K2_UNCERTAINTY_ORACLE_DESCRIPTOR_SCHEMA_V1.to_owned(),
            experiment_id_sha256: public_batch.experiment_id_sha256.clone(),
            public_batch_root_sha256: public_batch.public_batch_root_sha256.clone(),
            batch_precommit_root_sha256: public_receipt.batch_precommit.batch_root_sha256.clone(),
            all_cases_precommitted_root_sha256: public_receipt.receipt_root_sha256.clone(),
            case_id_sha256: case_id,
            case_sequence: artifact.case_sequence,
            public_case_root_sha256: prepared
                .probe_request
                .public_case
                .public_case_root_sha256
                .clone(),
            prepared_case_root_sha256: prepared.prepared_case_root_sha256.clone(),
            closure_plan_root_sha256: plan.plan_root_sha256.clone(),
            baseline_summary_root_sha256: prepared
                .selection_preverification
                .baseline_summary
                .summary_root_sha256
                .clone(),
            observation_vector_root_sha256: observation_vector.vector_root_sha256.clone(),
            final_verifier_receipt_root_sha256: final_receipt.receipt_root_sha256.clone(),
            private_truth_artifact_root_sha256: truth.final_truth_root_sha256.clone(),
            case_evidence_manifest_root_sha256: manifest.manifest_root_sha256.clone(),
            oracle_evaluator_executable_sha256: binaries.oracle.sha256.clone(),
        };
        descriptor.validate().expect("oracle descriptor");
        maximum_protocol_bytes = maximum_protocol_bytes.max(
            uncertainty_bytes_v1(&descriptor)
                .expect("oracle descriptor bytes")
                .len() as u64,
        );
        let case_started = Instant::now();
        let oracle_receipt: K2UncertaintyOracleBaselineCaseReceiptV1 =
            run_process_at(&binaries.oracle.path, &oracle_evidence_root, &descriptor);
        maximum_case_wall_ms = maximum_case_wall_ms.max(case_started.elapsed().as_millis() as u64);
        oracle_receipt
            .validate()
            .expect("independent oracle receipt");
        assert_eq!(
            oracle_receipt.reconstructed_frontier.raw_probe_count,
            K2_UNCERTAINTY_RAW_PROBES_V1 as u64
        );
        assert!(oracle_receipt.oracle_equality);
        assert!(oracle_receipt.model_guided_observation_parity);
        assert!(oracle_receipt.model_guided.true_class_retained);
        assert!(oracle_receipt.oracle.true_class_retained);
        assert_eq!(
            oracle_receipt.exact_plan_denominator,
            oracle_receipt.reconstructed_frontier.class_count.pow(2)
        );
        oracle_receipts.push(oracle_receipt);
    }
    assert_eq!(
        plan_lengths.iter().filter(|length| **length == 1).count(),
        8
    );
    assert_eq!(
        plan_lengths.iter().filter(|length| **length == 2).count(),
        8
    );

    let oracle_batch = K2UncertaintyOracleBaselineBatchReceiptV1::seal(
        public_batch.experiment_id_sha256.clone(),
        oracle_receipts,
    )
    .expect("seal independent oracle batch");
    assert_eq!(oracle_batch.oracle_equal_cases, 16);
    assert_eq!(oracle_batch.true_class_retained_cases, 16);
    assert_eq!(oracle_batch.false_accepts, 0);
    assert!(
        oracle_batch
            .aggregates
            .iter()
            .all(|aggregate| { aggregate.aggregate_superiority && aggregate.threshold_pass })
    );

    let experiment_root = root("r7j-development-experiment");
    let freeze_root = root("r7j-development-freeze");
    let controls = development_control_receipts(&binaries, &experiment_root, &freeze_root);
    assert_eq!(
        controls.iter().map(|receipt| receipt.expected).sum::<u64>(),
        64
    );
    let routes = evaluation_routes(&binaries, plan_lengths.iter().sum::<u64>());
    maximum_protocol_bytes = maximum_protocol_bytes.max(
        uncertainty_bytes_v1(&oracle_batch)
            .expect("oracle batch bytes")
            .len() as u64,
    );
    let resources = K2UncertaintyEvaluationResourceMeasurementsV1::seal(
        0,
        maximum_case_wall_ms,
        batch_started.elapsed().as_millis() as u64,
        maximum_protocol_bytes,
        0,
        0,
        0,
        0,
        0,
        0,
    )
    .expect("development resource measurements");
    let terminal_request = K2UncertaintyDevelopmentRehearsalTerminalRequestV1::seal(
        experiment_root.clone(),
        oracle_batch.clone(),
        controls.clone(),
        routes.clone(),
        resources.clone(),
        binaries.terminal.sha256.clone(),
    )
    .expect("development terminal request");
    assert!(
        uncertainty_bytes_v1(&terminal_request)
            .expect("development terminal bytes")
            .len()
            < 1_048_576
    );
    let terminal_receipt: K2UncertaintyTerminalEvaluationReceiptV1 = run_process(
        &binaries.terminal.path,
        &K2UncertaintyTerminalProcessRequestV1::Development {
            request: terminal_request,
        },
    );
    assert_eq!(
        terminal_receipt.disposition,
        K2UncertaintyTerminalDispositionV1::DevelopmentRehearsalPass
    );

    let projection = K2UncertaintySealedProjectionV1::seal(
        experiment_root,
        freeze_root,
        root("r7j-negative-attempt"),
        root("r7j-negative-slot"),
        root("r7j-negative-nonce"),
    )
    .expect("sealed projection test fixture");
    let mut sealed_request = K2UncertaintySealedTerminalRequestV1::seal(
        projection,
        oracle_batch,
        controls,
        routes,
        resources,
        0,
        0,
        binaries.terminal.sha256.clone(),
    )
    .expect("cross-mode sealed request");
    let cross_mode: K2UncertaintyTerminalEvaluationReceiptV1 = run_process(
        &binaries.terminal.path,
        &K2UncertaintyTerminalProcessRequestV1::Sealed {
            request: sealed_request.clone(),
        },
    );
    assert_eq!(
        cross_mode.disposition,
        K2UncertaintyTerminalDispositionV1::InfrastructureFail
    );
    sealed_request.irreversible_dispatch_missing_results = 1;
    sealed_request.reseal().expect("indeterminate request");
    let indeterminate: K2UncertaintyTerminalEvaluationReceiptV1 = run_process(
        &binaries.terminal.path,
        &K2UncertaintyTerminalProcessRequestV1::Sealed {
            request: sealed_request,
        },
    );
    assert_eq!(
        indeterminate.disposition,
        K2UncertaintyTerminalDispositionV1::Indeterminate
    );
}

#[test]
fn r7j_control_evaluator_rejects_substituted_process_evidence() {
    let binaries = ProcessBinaries::from_cargo();
    let experiment_root = root("r7j-control-negative-experiment");
    let freeze_root = root("r7j-control-negative-freeze");
    let receipts = development_control_receipts(&binaries, &experiment_root, &freeze_root);
    let legacy = &receipts[0];

    let mut substituted = legacy.outcomes.clone();
    substituted[0].decoded_disposition = "substituted_pass".to_owned();
    substituted[0]
        .reseal()
        .expect("reseal adversarial process outcome");
    let request = K2UncertaintyControlEvaluationRequestV1::seal(
        K2UncertaintyControlScopeV1::SuccessorStaticLegacy,
        experiment_root.clone(),
        None,
        None,
        substituted,
        binaries.control.sha256.clone(),
    )
    .expect("structurally valid substituted request");
    let stderr = run_process_failure(&binaries.control.path, &request);
    assert!(stderr.contains("self_formed_control_process_predicate_failed"));

    let omitted = K2UncertaintyControlEvaluationRequestV1::seal(
        K2UncertaintyControlScopeV1::SuccessorStaticLegacy,
        experiment_root,
        None,
        None,
        legacy.outcomes[..legacy.outcomes.len() - 1].to_vec(),
        binaries.control.sha256.clone(),
    );
    assert!(omitted.is_err());

    let source = concat!(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_oracle_baseline.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_oracle_frontier.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_oracle_process.rs"
        )),
    );
    for forbidden in [
        "run_self_formed_closure",
        "select_self_formed",
        "evaluate_self_formed_controls_v1",
        "evaluate_self_formed_development_terminal_v1",
        "verify_self_formed_confirm_final_v2",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden oracle dependency: {forbidden}"
        );
    }
}

fn publish_oracle_case_evidence(
    root_path: &Path,
    public_bindings: &K2UncertaintyOraclePublicBindingsV1,
    prepared: &K2UncertaintyPublicPreparedCaseV1,
    probe_output: &K2UncertaintyProbeOutputV1,
    observation_vector: &K2UncertaintyObservationVectorV2,
    final_receipt: &K2UncertaintyConfirmFinalVerifierReceiptV1,
    truth: &K2UncertaintyConfirmFinalTruthCaseV1,
) -> K2UncertaintyOracleCaseEvidenceManifestV1 {
    fs::create_dir_all(root_path).expect("create oracle evidence root");
    let model_set = &public_bindings.probe_request.learner_response.model_set;
    let closure_plan = prepared
        .preverification
        .closure_plan
        .as_ref()
        .expect("frozen closure plan");
    let mut entries = vec![
        write_oracle_entry(
            root_path,
            K2UncertaintyOracleEvidenceKindV1::PublicBindings,
            "public-bindings.json",
            &public_bindings.bindings_root_sha256,
            public_bindings,
        ),
        write_oracle_entry(
            root_path,
            K2UncertaintyOracleEvidenceKindV1::ModelSet,
            "model-set.json",
            &model_set.model_set_root_sha256,
            model_set,
        ),
        write_oracle_entry(
            root_path,
            K2UncertaintyOracleEvidenceKindV1::FrontierCensus,
            "frontier-census.json",
            &probe_output.frontier.frontier_root_sha256,
            &probe_output.frontier,
        ),
        write_oracle_entry(
            root_path,
            K2UncertaintyOracleEvidenceKindV1::ClosurePlan,
            "closure-plan.json",
            &closure_plan.plan_root_sha256,
            closure_plan,
        ),
        write_oracle_entry(
            root_path,
            K2UncertaintyOracleEvidenceKindV1::ClosurePreverification,
            "closure-preverification.json",
            &prepared.preverification.receipt_root_sha256,
            &prepared.preverification,
        ),
        write_oracle_entry(
            root_path,
            K2UncertaintyOracleEvidenceKindV1::BaselineSummary,
            "baseline-summary.json",
            &prepared
                .selection_preverification
                .baseline_summary
                .summary_root_sha256,
            &prepared.selection_preverification.baseline_summary,
        ),
        write_oracle_entry(
            root_path,
            K2UncertaintyOracleEvidenceKindV1::ObservationVector,
            "observation-vector.json",
            &observation_vector.vector_root_sha256,
            observation_vector,
        ),
        write_oracle_entry(
            root_path,
            K2UncertaintyOracleEvidenceKindV1::FinalVerifierReceipt,
            "final-verifier-receipt.json",
            &final_receipt.receipt_root_sha256,
            final_receipt,
        ),
        write_oracle_entry(
            root_path,
            K2UncertaintyOracleEvidenceKindV1::PrivateTruth,
            "private-truth.json",
            &truth.final_truth_root_sha256,
            truth,
        ),
    ];
    for page in &probe_output.pages {
        entries.push(write_oracle_entry(
            root_path,
            K2UncertaintyOracleEvidenceKindV1::FrontierPage,
            &format!("frontier-pages/page-{:04}.json", page.page_sequence),
            &page.page_root_sha256,
            page,
        ));
    }
    let manifest = K2UncertaintyOracleCaseEvidenceManifestV1::seal(
        prepared
            .probe_request
            .public_case
            .vocabulary
            .case_id_sha256
            .clone(),
        entries,
    )
    .expect("seal oracle evidence manifest");
    write_read_only(
        &root_path.join(K2_UNCERTAINTY_ORACLE_MANIFEST_FILE_V1),
        &uncertainty_bytes_v1(&manifest).expect("oracle manifest bytes"),
    );
    manifest
}

fn write_oracle_entry<T: serde::Serialize>(
    root_path: &Path,
    kind: K2UncertaintyOracleEvidenceKindV1,
    relative_path: &str,
    semantic_root_sha256: &str,
    value: &T,
) -> K2UncertaintyOracleEvidenceEntryV1 {
    let bytes = uncertainty_bytes_v1(value)
        .unwrap_or_else(|error| panic!("oracle evidence bytes for {relative_path}: {error}"));
    write_read_only(&root_path.join(relative_path), &bytes);
    K2UncertaintyOracleEvidenceEntryV1::seal(
        kind,
        relative_path.to_owned(),
        bytes.len() as u64,
        0o400,
        composition_sha256_bytes_v1(&bytes),
        semantic_root_sha256.to_owned(),
    )
    .expect("oracle evidence entry")
}

fn write_read_only(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().expect("oracle evidence parent"))
        .expect("create oracle evidence parent");
    fs::write(path, bytes).expect("write oracle evidence");
    fs::set_permissions(path, fs::Permissions::from_mode(0o400)).expect("chmod oracle evidence");
}

fn development_control_receipts(
    binaries: &ProcessBinaries,
    experiment_root: &str,
    freeze_root: &str,
) -> Vec<K2UncertaintyControlEvaluationReceiptV1> {
    let scopes = [
        (K2UncertaintyControlScopeV1::SuccessorStaticLegacy, None),
        (K2UncertaintyControlScopeV1::SuccessorStaticV3, None),
        (K2UncertaintyControlScopeV1::SuccessorStaticV4, None),
        (
            K2UncertaintyControlScopeV1::DevelopmentRehearsalV5,
            Some(freeze_root.to_owned()),
        ),
    ];
    scopes
        .into_iter()
        .map(|(scope, freeze)| {
            let outcomes = (0..scope.expected_count())
                .map(|ordinal| {
                    let (control_id, disposition) =
                        expected_self_formed_control_v1(scope, ordinal).expect("expected control");
                    let stdout = uncertainty_bytes_v1(&K2UncertaintyControlStdoutV1 {
                        control_id: control_id.clone(),
                        disposition: disposition.clone(),
                    })
                    .expect("control stdout");
                    K2UncertaintyControlProcessOutcomeV1::seal(
                        scope,
                        control_id,
                        experiment_root.to_owned(),
                        freeze.clone(),
                        None,
                        root(&format!("control-runner-{scope:?}")),
                        root(&format!("control-test-{scope:?}-{ordinal}")),
                        root(&format!("control-request-{scope:?}-{ordinal}")),
                        true,
                        0,
                        stdout,
                        root(&format!("control-stderr-{scope:?}-{ordinal}")),
                        false,
                        false,
                        disposition,
                        root(&format!("control-source-{scope:?}-{ordinal}")),
                        root(&format!("control-log-{scope:?}-{ordinal}")),
                    )
                    .expect("control process outcome")
                })
                .collect();
            let request = K2UncertaintyControlEvaluationRequestV1::seal(
                scope,
                experiment_root.to_owned(),
                freeze,
                None,
                outcomes,
                binaries.control.sha256.clone(),
            )
            .expect("control evaluation request");
            let receipt: K2UncertaintyControlEvaluationReceiptV1 =
                run_process(&binaries.control.path, &request);
            assert!(receipt.all_pass);
            receipt
        })
        .collect()
}

fn evaluation_routes(
    binaries: &ProcessBinaries,
    case_execution_count: u64,
) -> Vec<K2UncertaintyEvaluationRouteReceiptV1> {
    [
        (
            "public_precommit",
            &binaries.public_coordinator.sha256,
            &binaries.oracle.sha256,
            16,
        ),
        (
            "case_execution",
            &binaries.worker.sha256,
            &binaries.final_verifier.sha256,
            case_execution_count,
        ),
        (
            "final_verification",
            &binaries.final_verifier.sha256,
            &binaries.oracle.sha256,
            16,
        ),
        (
            "oracle_evaluation",
            &binaries.oracle.sha256,
            &binaries.terminal.sha256,
            16,
        ),
        (
            "control_evaluation",
            &binaries.control.sha256,
            &binaries.terminal.sha256,
            64,
        ),
    ]
    .into_iter()
    .map(|(id, producer, consumer, events)| {
        K2UncertaintyEvaluationRouteReceiptV1::seal(
            id.to_owned(),
            producer.clone(),
            consumer.clone(),
            events,
            events,
        )
        .expect("evaluation route")
    })
    .collect()
}

fn run_sandbox<I, O>(
    role: K2UncertaintyConfirmGuestExecutableV1,
    binary: &ProcessBinary,
    mounts: &[K2UncertaintyConfirmDataMountV1<'_>],
    input: &I,
    cpu_seconds: u64,
) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let bytes = run_self_formed_confirm_sandbox_v1(
        role,
        &binary.path,
        &binary.sha256,
        mounts,
        &uncertainty_bytes_v1(input).expect("sandbox input"),
        cpu_seconds,
    )
    .expect("sandbox process");
    uncertainty_decode_v1(&bytes).expect("sandbox output")
}

fn run_process<I, O>(path: &Path, input: &I) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let mut child = Command::new(path)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn process");
    child
        .stdin
        .take()
        .expect("process stdin")
        .write_all(&uncertainty_bytes_v1(input).expect("process input"))
        .expect("write process input");
    let output = child.wait_with_output().expect("wait process");
    assert!(
        output.status.success(),
        "process failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    uncertainty_decode_v1(&output.stdout).expect("process output")
}

fn run_process_at<I, O>(path: &Path, current_dir: &Path, input: &I) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let mut child = Command::new(path)
        .env_clear()
        .current_dir(current_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn process with evidence root");
    child
        .stdin
        .take()
        .expect("process stdin")
        .write_all(&uncertainty_bytes_v1(input).expect("process input"))
        .expect("write process input");
    let output = child.wait_with_output().expect("wait process");
    assert!(
        output.status.success(),
        "process failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    uncertainty_decode_v1(&output.stdout).expect("process output")
}

fn run_process_failure<I: serde::Serialize>(path: &Path, input: &I) -> String {
    let mut child = Command::new(path)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn failing process");
    child
        .stdin
        .take()
        .expect("failing process stdin")
        .write_all(&uncertainty_bytes_v1(input).expect("failing process input"))
        .expect("write failing process input");
    let output = child.wait_with_output().expect("wait failing process");
    assert!(!output.status.success());
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn materialize(root: &Path, files: &BTreeMap<String, Vec<u8>>) {
    fs::create_dir_all(root).expect("create materialization root");
    for (relative, bytes) in files {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("materialized parent"))
            .expect("create materialized parent");
        fs::write(path, bytes).expect("write materialized file");
    }
}

fn root(label: &str) -> String {
    composition_root_v1(&("nando.k2-self-formed-r7i-test.v1", label)).expect("test root")
}

#[derive(Clone)]
struct ProcessBinary {
    path: PathBuf,
    sha256: String,
}

impl ProcessBinary {
    fn new(path: PathBuf) -> Self {
        let sha256 = composition_sha256_file_v1(&path).expect("binary SHA-256");
        Self { path, sha256 }
    }
}

struct ProcessBinaries {
    generator: ProcessBinary,
    learner: ProcessBinary,
    probe: ProcessBinary,
    selector: ProcessBinary,
    baseline: ProcessBinary,
    selection_preverifier: ProcessBinary,
    closure_planner: ProcessBinary,
    closure_verifier: ProcessBinary,
    public_coordinator: ProcessBinary,
    private_resolver: ProcessBinary,
    safety: ProcessBinary,
    worker: ProcessBinary,
    observer: ProcessBinary,
    final_verifier: ProcessBinary,
    oracle: ProcessBinary,
    control: ProcessBinary,
    terminal: ProcessBinary,
}

impl ProcessBinaries {
    fn from_cargo() -> Self {
        Self {
            generator: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator")),
            learner: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-learner")),
            probe: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-probe")),
            selector: binary(env!("CARGO_BIN_EXE_nando-k2-inquiry-selector")),
            baseline: binary(env!("CARGO_BIN_EXE_nando-k2-inquiry-baseline")),
            selection_preverifier: binary(env!("CARGO_BIN_EXE_nando-k2-inquiry-verifier")),
            closure_planner: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-closure-planner")),
            closure_verifier: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-closure-verifier")),
            public_coordinator: binary(env!(
                "CARGO_BIN_EXE_nando-k2-self-formed-public-coordinator"
            )),
            private_resolver: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-private-resolver")),
            safety: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-safety")),
            worker: binary(env!("CARGO_BIN_EXE_nando-k2-inquiry-worker")),
            observer: binary(env!("CARGO_BIN_EXE_nando-k2-inquiry-observer")),
            final_verifier: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-final-verifier-v2")),
            oracle: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-oracle-baseline")),
            control: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-control-evaluator")),
            terminal: binary(env!(
                "CARGO_BIN_EXE_nando-k2-self-formed-terminal-evaluator"
            )),
        }
    }

    fn public_owner_set(&self) -> K2UncertaintyPublicOwnerSetV1 {
        K2UncertaintyPublicOwnerSetV1::seal(vec![
            owner(K2UncertaintyPublicOwnerRoleV1::Learner, &self.learner),
            owner(K2UncertaintyPublicOwnerRoleV1::Probe, &self.probe),
            owner(K2UncertaintyPublicOwnerRoleV1::Selector, &self.selector),
            owner(K2UncertaintyPublicOwnerRoleV1::Baseline, &self.baseline),
            owner(
                K2UncertaintyPublicOwnerRoleV1::SelectionPreverifier,
                &self.selection_preverifier,
            ),
            owner(
                K2UncertaintyPublicOwnerRoleV1::ClosurePlanner,
                &self.closure_planner,
            ),
            owner(
                K2UncertaintyPublicOwnerRoleV1::ClosureVerifier,
                &self.closure_verifier,
            ),
        ])
        .expect("public owner set")
    }

    fn assert_pairwise_distinct(&self) {
        let roots = [
            &self.generator.sha256,
            &self.learner.sha256,
            &self.probe.sha256,
            &self.selector.sha256,
            &self.baseline.sha256,
            &self.selection_preverifier.sha256,
            &self.closure_planner.sha256,
            &self.closure_verifier.sha256,
            &self.public_coordinator.sha256,
            &self.private_resolver.sha256,
            &self.safety.sha256,
            &self.worker.sha256,
            &self.observer.sha256,
            &self.final_verifier.sha256,
            &self.oracle.sha256,
            &self.control.sha256,
            &self.terminal.sha256,
        ];
        assert_eq!(
            roots.iter().copied().collect::<BTreeSet<_>>().len(),
            roots.len()
        );
    }
}

fn binary(path: &str) -> ProcessBinary {
    ProcessBinary::new(PathBuf::from(path))
}

fn owner(
    role: K2UncertaintyPublicOwnerRoleV1,
    binary: &ProcessBinary,
) -> K2UncertaintyPublicOwnerV1 {
    K2UncertaintyPublicOwnerV1 {
        role,
        executable_path: binary.path.to_string_lossy().into_owned(),
        executable_sha256: binary.sha256.clone(),
    }
}

struct TestEnvironment {
    root: PathBuf,
}

impl TestEnvironment {
    fn new(label: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nando-k2-self-formed-r7i-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create test root");
        Self { root }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("R7I failed fixture retained at {}", self.root.display());
            return;
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}
