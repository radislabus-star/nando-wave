use std::collections::BTreeMap;
use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

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
    K2UncertaintyR8BControlWrapperV3, K2UncertaintyR8BEvidenceKindV2,
    K2UncertaintyR8BLedgerWriterV3, K2UncertaintyR8BMeasuredReceiptV2,
    K2UncertaintyR8BOracleWrapperV3, K2UncertaintyR8BValidatedFactV3,
    K2UncertaintyResultProcessRequestV1, K2UncertaintyTerminalEvaluationReceiptV1,
    K2UncertaintyTerminalProcessRequestV1, census_self_formed_cleanup_artifacts_v1,
    composition_sha256_file_v1, load_development_rehearsal_owner_metadata_v1,
    publish_immutable_file_v1, publish_self_formed_cleanup_manifest_v1,
    seal_self_formed_r8b_control_wrapper_v3, seal_self_formed_r8b_oracle_wrapper_v3,
    uncertainty_bytes_v1,
};

use nando_operator_learning::{
    K2_UNCERTAINTY_ORACLE_DESCRIPTOR_SCHEMA_V1, K2_UNCERTAINTY_RAW_PROBES_V1,
    K2CompositionTreeManifestV1, K2InquiryObservationReceiptV1, K2InquiryWorkerOutcomeV1,
    K2UncertaintyCaseJournalFaultV2, K2UncertaintyCaseJournalV2, K2UncertaintyConfirmDataMountV1,
    K2UncertaintyConfirmFinalVerifierReceiptV1, K2UncertaintyConfirmFinalVerifierRequestV1,
    K2UncertaintyConfirmGuestExecutableV1, K2UncertaintyConfirmMountTargetV1,
    K2UncertaintyConfirmPlanSafetyBindingV1, K2UncertaintyConfirmSafetyReceiptV1,
    K2UncertaintyConfirmSafetyRequestV1, K2UncertaintyControlEvaluationReceiptV1,
    K2UncertaintyControlEvaluationRequestV1, K2UncertaintyControlProcessOutcomeV1,
    K2UncertaintyControlScopeV1, K2UncertaintyControlStdoutV1,
    K2UncertaintyDevelopmentRehearsalMetadataV1,
    K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1,
    K2UncertaintyEvaluationResourceMeasurementsV1, K2UncertaintyEvaluationRouteReceiptV1,
    K2UncertaintyObservationVectorV2, K2UncertaintyOracleBaselineBatchReceiptV1,
    K2UncertaintyOracleBaselineCaseDescriptorV1, K2UncertaintyOracleBaselineCaseReceiptV1,
    K2UncertaintyOraclePublicBindingsV1, K2UncertaintyPrivateResolverReceiptV1,
    K2UncertaintyPrivateResolverRequestV1, K2UncertaintyProbeExecutionEvidenceV2,
    K2UncertaintyWorkspaceIdentityV2, composition_root_v1, composition_sha256_bytes_v1,
    expected_self_formed_control_v1, load_self_formed_public_case_v1,
    materialize_self_formed_probe_files_v1, new_self_formed_r7k_control_case_request_v1,
    prepare_self_formed_confirm_plan_dispatch_v1, publish_self_formed_final_verifier_material_v2,
    reopen_self_formed_probe_output_v1, run_self_formed_r7k_control_sandbox_v1,
    uncertainty_decode_v1, uncertainty_root_v1,
};

#[rustfmt::skip]
#[path = "k2_self_formed_uncertainty_confirm_r8b_support/mod.rs"]
mod support;

use support::*;

const CHILD_SELECTOR_V2: &str = "r8b_v8_m24_linked_child";

#[path = "k2_self_formed_uncertainty_confirm_r8b_linked_v1/child_cleanup.rs"]
mod child_cleanup;

use child_cleanup::*;

#[test]
#[ignore = "requires explicit R8B V8 execution authorization"]
fn r8b_v8_m24_linked_child() {
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
    let (producer_v3, _) = active_producer_request_v3().expect("active M24 V3 request");
    freeze_downstream_contract_v3(producer_v3, &output_root);

    let freeze = owner_request
        .descriptor
        .successor_freeze_root_sha256
        .clone();
    let downstream = run_downstream_v2(
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
        assert_eq!(
            fs::read(candidate_root.join(relative)).expect("persisted M17 receipt"),
            uncertainty_bytes_v1(receipt).expect("canonical M17 receipt")
        );
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
    let summary = K2UncertaintyR8BLedgerWriterV3::attach_request(producer_v3)
        .expect("reattach M24 V3 ledger")
        .summary()
        .expect("M24 V3 root summary");
    let m16_events = summary
        .m16_event_roots_sha256
        .into_iter()
        .collect::<Vec<_>>();
    let m16_receipts = summary
        .m16_receipt_roots_sha256
        .into_iter()
        .collect::<Vec<_>>();
    let m17_events = summary
        .m17_event_roots_sha256
        .into_iter()
        .collect::<Vec<_>>();
    let m17_receipts = summary
        .m17_receipt_roots_sha256
        .into_iter()
        .collect::<Vec<_>>();
    let oracle_wrapper =
        seal_self_formed_r8b_oracle_wrapper_v3(oracle.clone(), m16_events, m16_receipts)
            .expect("seal exact M16 wrapper");
    let control_census = K2UncertaintyR8BMeasuredReceiptV2::seal(
        K2UncertaintyR8BEvidenceKindV2::FrozenControlScopes,
        producer.route_id_sha256.clone(),
        m17_receipts.clone(),
        4,
        BTreeMap::new(),
        m24.sha256.clone(),
    )
    .expect("four-scope census");
    let control_wrapper =
        seal_self_formed_r8b_control_wrapper_v3(control_census, m17_events, m17_receipts)
            .expect("seal exact M17 wrapper");
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
    publish_child_owned_v2(
        &output_root,
        &oracle_wrapper,
        &control_wrapper,
        &linked_route,
    );
    freeze_directory_tree_v2(&candidate_root);
}

struct DownstreamOutputV2 {
    oracle: K2UncertaintyOracleBaselineBatchReceiptV1,
    controls: Vec<K2UncertaintyControlEvaluationReceiptV1>,
    routes: Vec<K2UncertaintyEvaluationRouteReceiptV1>,
    resources: K2UncertaintyEvaluationResourceMeasurementsV1,
}

#[allow(clippy::too_many_arguments)]
fn run_downstream_v2(
    ledger: &mut DurableProcessLedgerV2,
    binaries: &LinkedBinariesV2,
    route_root: &Path,
    attempt_root: &Path,
    public_root: &Path,
    metadata: &K2UncertaintyDevelopmentRehearsalMetadataV1,
    public_receipt: &K2UncertaintyPublicPrecommitReceiptV1,
    freeze_root_sha256: &str,
) -> DownstreamOutputV2 {
    let batch_started = Instant::now();
    let generated_root = attempt_root.join("generated");
    let execution_root = route_root.join("execution");
    create_private_directory_v1(&execution_root);
    let oracle_root = route_root.join("oracle");
    create_private_directory_v1(&oracle_root);
    let mut measurements = MeasurementsV2::default();
    let mut maximum_case_wall_ms = 0_u64;
    let mut plan_lengths = Vec::new();
    let mut oracle_receipts = Vec::new();

    for artifact in &public_receipt.case_artifacts {
        let case_started = Instant::now();
        let prepared = load_self_formed_public_case_v1(public_root, artifact)
            .expect("reopen M24 prepared public case");
        let case_id = prepared
            .probe_request
            .public_case
            .vocabulary
            .case_id_sha256
            .clone();
        let resolver_artifact = private_artifact_v2(
            metadata,
            &case_id,
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::ResolverTable,
        );
        let truth_artifact = private_artifact_v2(
            metadata,
            &case_id,
            K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::FinalTruth,
        );
        let plan = prepared
            .preverification
            .closure_plan
            .as_ref()
            .expect("publicly frozen M24 closure plan");
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
                .expect("M24 planned representative");
            let resolver = binaries.get("M11_PRIVATE_RESOLVER");
            let resolver_request = K2UncertaintyPrivateResolverRequestV1::seal(
                metadata.public_batch.experiment_id_sha256.clone(),
                metadata.public_batch.public_batch_root_sha256.clone(),
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
                resolver.sha256.clone(),
            )
            .expect("M24 private resolver request");
            let resolver_descriptor = PrivateDescriptorV2::open(
                &generated_root.join(&resolver_artifact.relative_path),
                resolver_artifact.byte_len,
                resolver_artifact.unix_mode,
            );
            let resolver_proc = resolver_descriptor.proc_path();
            let resolver_mounts = [K2UncertaintyConfirmDataMountV1 {
                host_path: &resolver_proc,
                target: K2UncertaintyConfirmMountTargetV1::ResolverTable,
                writable: false,
            }];
            let resolver_receipt: K2UncertaintyPrivateResolverReceiptV1 = run_recorded_sandbox_v2(
                ledger,
                resolver,
                K2UncertaintyConfirmGuestExecutableV1::PrivateResolver,
                "C09",
                &case_id,
                Some(ordinal as u64),
                &resolver_request.request_root_sha256,
                &resolver_mounts,
                &resolver_request,
                &mut measurements,
                |receipt: &K2UncertaintyPrivateResolverReceiptV1| {
                    (&receipt.schema, &receipt.receipt_root_sha256)
                },
            );
            resolver_receipt
                .validate()
                .expect("validate M11 private resolver receipt");
            assert_eq!(resolver_receipt.exposed_effect_count, 1);

            let workspace_identity = K2UncertaintyWorkspaceIdentityV2::seal(
                case_id.clone(),
                plan.plan_root_sha256.clone(),
                ordinal as u64,
            )
            .expect("M24 workspace identity");
            let safety = binaries.get("M12_SAFETY");
            let safety_request = K2UncertaintyConfirmSafetyRequestV1::seal(
                resolver_request.clone(),
                resolver_receipt.clone(),
                prepared.probe_request.public_case.vocabulary.clone(),
                workspace_identity,
                safety.sha256.clone(),
            )
            .expect("M24 safety request");
            let safety_receipt: K2UncertaintyConfirmSafetyReceiptV1 = run_recorded_sandbox_v2(
                ledger,
                safety,
                K2UncertaintyConfirmGuestExecutableV1::Safety,
                "C09",
                &case_id,
                Some(ordinal as u64),
                &safety_request.request_root_sha256,
                &[],
                &safety_request,
                &mut measurements,
                |receipt: &K2UncertaintyConfirmSafetyReceiptV1| {
                    (&receipt.schema, &receipt.receipt_root_sha256)
                },
            );
            safety_receipt
                .validate()
                .expect("validate M12 safety receipt");
            bindings.push(K2UncertaintyConfirmPlanSafetyBindingV1 {
                resolver_request,
                resolver_receipt,
                safety_request,
                safety_receipt,
            });
        }

        let dispatch = prepare_self_formed_confirm_plan_dispatch_v1(
            &metadata.public_batch.public_batch_root_sha256,
            &public_receipt.batch_precommit,
            &prepared,
            bindings,
            &binaries.get("M13_WORKER").sha256,
            &binaries.get("M14_OBSERVER").sha256,
        )
        .expect("M24 bounded Confirm dispatch");
        let journal_root = execution_root.join(format!("journal-{:02}", artifact.case_sequence));
        let mut journal = K2UncertaintyCaseJournalV2::create(&journal_root, dispatch.clone())
            .expect("create M24 case journal");
        journal
            .record_plan_dispatch(
                binaries.get("M10_PUBLIC_COORDINATOR").sha256.clone(),
                K2UncertaintyCaseJournalFaultV2::None,
            )
            .expect("freeze M24 dispatch before worker");
        let mut executions = Vec::with_capacity(dispatch.items.len());

        for item in &dispatch.items {
            let source = execution_root.join(format!(
                "source-{}",
                item.workspace_identity.identity_root_sha256
            ));
            let workspace = execution_root.join(format!(
                "work-{}",
                item.workspace_identity.identity_root_sha256
            ));
            let files = materialize_self_formed_probe_files_v1(
                &prepared.probe_request.public_case,
                &item.selected_probe,
            )
            .expect("materialize M24 selected public state");
            materialize_v2(&source, &files);
            materialize_v2(&workspace, &files);
            let source_before =
                K2CompositionTreeManifestV1::scan(&source).expect("M24 source before");
            let permit = journal
                .begin_probe_execution(item.probe_ordinal, K2UncertaintyCaseJournalFaultV2::None)
                .expect("begin M24 exact ordinal");
            let worker = binaries.get("M13_WORKER");
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
            let worker_outcome: K2InquiryWorkerOutcomeV1 = run_recorded_sandbox_v2(
                ledger,
                worker,
                K2UncertaintyConfirmGuestExecutableV1::Worker,
                "C09",
                &case_id,
                Some(item.probe_ordinal),
                &item.worker_request.request_root_sha256,
                &worker_mounts,
                &item.worker_request,
                &mut measurements,
                |receipt: &K2InquiryWorkerOutcomeV1| {
                    (&receipt.schema, &receipt.outcome_root_sha256)
                },
            );
            let observer = binaries.get("M14_OBSERVER");
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
            let observation: K2InquiryObservationReceiptV1 = run_recorded_sandbox_v2(
                ledger,
                observer,
                K2UncertaintyConfirmGuestExecutableV1::Observer,
                "C09",
                &case_id,
                Some(item.probe_ordinal),
                &item.observer_request.request_root_sha256,
                &observer_mounts,
                &item.observer_request,
                &mut measurements,
                |receipt: &K2InquiryObservationReceiptV1| {
                    (&receipt.schema, &receipt.receipt_root_sha256)
                },
            );
            assert_eq!(worker_outcome.post_manifest, observation.post_manifest);
            assert_eq!(
                source_before,
                K2CompositionTreeManifestV1::scan(&source).expect("M24 source after")
            );
            let evidence = K2UncertaintyProbeExecutionEvidenceV2::seal(
                dispatch.dispatch_root_sha256.clone(),
                item,
                worker_outcome,
                observation,
            )
            .expect("seal M24 execution evidence");
            journal
                .record_probe_observation(
                    permit,
                    evidence.observation.receipt_root_sha256.clone(),
                    K2UncertaintyCaseJournalFaultV2::None,
                )
                .expect("freeze M24 observation");
            executions.push(evidence);
        }

        let observation_vector = K2UncertaintyObservationVectorV2::seal(&dispatch, executions)
            .expect("seal M24 ordered observation vector");
        journal
            .freeze_observation_vector(
                binaries.get("M10_PUBLIC_COORDINATOR").sha256.clone(),
                observation_vector.vector_root_sha256.clone(),
                observation_vector.vector_root_sha256.clone(),
                K2UncertaintyCaseJournalFaultV2::None,
            )
            .expect("freeze M24 complete observation vector");
        let evidence_root = public_root.join(format!("probes/case-{:02}", artifact.case_sequence));
        let material = publish_self_formed_final_verifier_material_v2(
            &evidence_root,
            &public_receipt.batch_precommit,
            &prepared.preverification,
        )
        .expect("publish M24 final verifier material");
        let final_verifier = binaries.get("M15_FINAL_VERIFIER");
        let final_request = K2UncertaintyConfirmFinalVerifierRequestV1::seal(
            final_verifier.sha256.clone(),
            material,
            prepared.probe_request.clone(),
            prepared.probe_artifacts.clone(),
            dispatch,
            observation_vector.clone(),
            journal.state().clone(),
            truth_artifact.semantic_root_sha256.clone(),
        )
        .expect("M24 final verifier request");
        let truth_descriptor = PrivateDescriptorV2::open(
            &generated_root.join(&truth_artifact.relative_path),
            truth_artifact.byte_len,
            truth_artifact.unix_mode,
        );
        let truth_proc = truth_descriptor.proc_path();
        let final_mounts = [
            K2UncertaintyConfirmDataMountV1 {
                host_path: &truth_proc,
                target: K2UncertaintyConfirmMountTargetV1::FinalTruth,
                writable: false,
            },
            K2UncertaintyConfirmDataMountV1 {
                host_path: &evidence_root,
                target: K2UncertaintyConfirmMountTargetV1::Evidence,
                writable: false,
            },
        ];
        let final_receipt: K2UncertaintyConfirmFinalVerifierReceiptV1 = run_recorded_sandbox_v2(
            ledger,
            final_verifier,
            K2UncertaintyConfirmGuestExecutableV1::FinalVerifier,
            "C09",
            &case_id,
            None,
            &final_request.request_root_sha256,
            &final_mounts,
            &final_request,
            &mut measurements,
            |receipt: &K2UncertaintyConfirmFinalVerifierReceiptV1| {
                (&receipt.schema, &receipt.receipt_root_sha256)
            },
        );
        final_receipt
            .validate()
            .expect("validate M15 final verifier receipt");
        assert_eq!(
            final_receipt.final_truth_root_sha256,
            truth_artifact.semantic_root_sha256
        );
        assert!(final_receipt.verification.private_true_class_match);
        assert_eq!(final_receipt.verification.false_accepts, 0);

        let probe_output =
            reopen_self_formed_probe_output_v1(&evidence_root, &prepared.probe_artifacts)
                .expect("reopen M24 public probe output");
        let public_bindings =
            K2UncertaintyOraclePublicBindingsV1::seal(public_receipt.clone(), prepared.clone())
                .expect("seal M24 Oracle public bindings");
        let oracle_evidence_root = oracle_root.join(format!("case-{:02}", artifact.case_sequence));
        let manifest = publish_oracle_case_evidence_v2(
            &oracle_evidence_root,
            &public_bindings,
            &prepared,
            &probe_output,
            &observation_vector,
            &final_receipt,
            truth_artifact,
        );
        let oracle = binaries.get("M16_ORACLE");
        let descriptor = K2UncertaintyOracleBaselineCaseDescriptorV1 {
            schema: K2_UNCERTAINTY_ORACLE_DESCRIPTOR_SCHEMA_V1.to_owned(),
            experiment_id_sha256: metadata.public_batch.experiment_id_sha256.clone(),
            public_batch_root_sha256: metadata.public_batch.public_batch_root_sha256.clone(),
            batch_precommit_root_sha256: public_receipt.batch_precommit.batch_root_sha256.clone(),
            all_cases_precommitted_root_sha256: public_receipt.receipt_root_sha256.clone(),
            case_id_sha256: case_id.clone(),
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
            private_truth_artifact_root_sha256: truth_artifact.semantic_root_sha256.clone(),
            case_evidence_manifest_root_sha256: manifest.manifest_root_sha256.clone(),
            oracle_evaluator_executable_sha256: oracle.sha256.clone(),
        };
        descriptor
            .validate()
            .expect("validate M24 Oracle descriptor");
        let oracle_mounts = [
            K2UncertaintyConfirmDataMountV1 {
                host_path: &oracle_evidence_root,
                target: K2UncertaintyConfirmMountTargetV1::OracleEvidence,
                writable: false,
            },
            K2UncertaintyConfirmDataMountV1 {
                host_path: &truth_proc,
                target: K2UncertaintyConfirmMountTargetV1::OraclePrivateTruth,
                writable: false,
            },
        ];
        let oracle_receipt: K2UncertaintyOracleBaselineCaseReceiptV1 = run_recorded_sandbox_v2(
            ledger,
            oracle,
            K2UncertaintyConfirmGuestExecutableV1::Oracle,
            "C10",
            &case_id,
            None,
            &descriptor.descriptor_root().expect("M16 descriptor root"),
            &oracle_mounts,
            &descriptor,
            &mut measurements,
            |receipt: &K2UncertaintyOracleBaselineCaseReceiptV1| {
                (&receipt.schema, &receipt.receipt_root_sha256)
            },
        );
        oracle_receipt
            .validate()
            .expect("validate M16 Oracle receipt");
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
        maximum_case_wall_ms = maximum_case_wall_ms.max(case_started.elapsed().as_millis() as u64);
    }

    assert_eq!(
        plan_lengths.iter().filter(|length| **length == 1).count(),
        8
    );
    assert_eq!(
        plan_lengths.iter().filter(|length| **length == 2).count(),
        8
    );
    let oracle = K2UncertaintyOracleBaselineBatchReceiptV1::seal(
        metadata.public_batch.experiment_id_sha256.clone(),
        oracle_receipts,
    )
    .expect("seal M24 independent Oracle batch");
    assert_eq!(oracle.oracle_equal_cases, 16);
    assert_eq!(oracle.true_class_retained_cases, 16);
    assert_eq!(oracle.false_accepts, 0);
    assert!(
        oracle
            .aggregates
            .iter()
            .all(|aggregate| aggregate.aggregate_superiority && aggregate.threshold_pass)
    );
    measurements.observe_bytes(&uncertainty_bytes_v1(&oracle).expect("M24 Oracle batch bytes"));
    let controls = run_controls_v2(
        ledger,
        binaries,
        route_root,
        &metadata.public_batch.experiment_id_sha256,
        freeze_root_sha256,
        &mut measurements,
    );
    let routes = evaluation_routes_v2(binaries, plan_lengths.iter().sum());
    let resources = K2UncertaintyEvaluationResourceMeasurementsV1::seal(
        0,
        maximum_case_wall_ms,
        batch_started.elapsed().as_millis() as u64,
        measurements.maximum_protocol_bytes,
        0,
        0,
        0,
        0,
        0,
        0,
    )
    .expect("seal M24 Development resource measurements");
    DownstreamOutputV2 {
        oracle,
        controls,
        routes,
        resources,
    }
}

fn run_controls_v2(
    ledger: &mut DurableProcessLedgerV2,
    binaries: &LinkedBinariesV2,
    route_root: &Path,
    experiment: &str,
    freeze: &str,
    measurements: &mut MeasurementsV2,
) -> Vec<K2UncertaintyControlEvaluationReceiptV1> {
    let evaluator = binaries.get("M17_CONTROL_EVALUATOR");
    let runner = binaries.get("M19_FRESH_CONTROL_CASE");
    let owner = binaries.get("M24_LINKED_RUNNER");
    let mut requests = [
        K2UncertaintyControlScopeV1::SuccessorStaticLegacy,
        K2UncertaintyControlScopeV1::SuccessorStaticV3,
        K2UncertaintyControlScopeV1::SuccessorStaticV4,
    ]
    .into_iter()
    .map(|scope| {
        let outcomes = (0..scope.expected_count())
            .map(|ordinal| {
                let (control_id, disposition) =
                    expected_self_formed_control_v1(scope, ordinal).expect("static control");
                let stdout = uncertainty_bytes_v1(&K2UncertaintyControlStdoutV1 {
                    control_id: control_id.clone(),
                    disposition: disposition.clone(),
                })
                .expect("static control stdout");
                K2UncertaintyControlProcessOutcomeV1::seal(
                    scope,
                    control_id,
                    experiment.to_owned(),
                    None,
                    None,
                    runner.sha256.clone(),
                    owner.sha256.clone(),
                    root_v1(&format!("static-request-{scope:?}-{ordinal}")),
                    true,
                    0,
                    stdout,
                    composition_sha256_bytes_v1(&[]),
                    false,
                    false,
                    disposition,
                    root_v1(&format!("static-source-{scope:?}-{ordinal}")),
                    root_v1(&format!("static-log-{scope:?}-{ordinal}")),
                )
                .expect("static predecessor outcome")
            })
            .collect();
        K2UncertaintyControlEvaluationRequestV1::seal(
            scope,
            experiment.to_owned(),
            None,
            None,
            outcomes,
            evaluator.sha256.clone(),
        )
        .expect("static M17 request")
    })
    .collect::<Vec<_>>();
    requests.push(
        K2UncertaintyControlEvaluationRequestV1::seal(
            K2UncertaintyControlScopeV1::DevelopmentRehearsalV5,
            experiment.to_owned(),
            Some(freeze.to_owned()),
            None,
            fresh_control_outcomes_v2(
                ledger,
                binaries,
                route_root,
                experiment,
                freeze,
                measurements,
            ),
            evaluator.sha256.clone(),
        )
        .expect("fresh M17 request"),
    );
    requests
        .into_iter()
        .map(|request| {
            let input = uncertainty_bytes_v1(&request).expect("M17 request bytes");
            let started = ledger.start_bound(
                "C12",
                None,
                None,
                evaluator,
                request.request_root_sha256.clone(),
                composition_sha256_bytes_v1(&input),
            );
            let output = ledger.run_started_process_v3(&started, &evaluator.path, &input, 60);
            let receipt: K2UncertaintyControlEvaluationReceiptV1 =
                uncertainty_decode_v1(&output.stdout).unwrap_or_else(|error| {
                    ledger.fail_unexpected_bound(
                        &started,
                        &output,
                        format!("decode M17 receipt: {error}"),
                    )
                });
            receipt.validate().expect("validate M17 receipt");
            assert_eq!(
                output.stdout,
                uncertainty_bytes_v1(&receipt).expect("canonical M17 bytes")
            );
            let relative = match request.scope {
                K2UncertaintyControlScopeV1::SuccessorStaticLegacy => "linked/legacy-controls.json",
                K2UncertaintyControlScopeV1::SuccessorStaticV3 => "linked/v3-controls.json",
                K2UncertaintyControlScopeV1::SuccessorStaticV4 => "linked/v4-controls.json",
                K2UncertaintyControlScopeV1::DevelopmentRehearsalV5 => "linked/fresh-controls.json",
                K2UncertaintyControlScopeV1::SealedAttemptV5 => {
                    panic!("sealed control forbidden in R8B")
                }
            };
            persist_candidate_v2(&route_root.join("candidate"), relative, &receipt);
            let descriptor = attested_evidence_output_v3(
                evaluator,
                relative,
                match request.scope {
                    K2UncertaintyControlScopeV1::SuccessorStaticLegacy => {
                        K2UncertaintyR8BEvidenceKindV2::LegacyControls
                    }
                    K2UncertaintyControlScopeV1::SuccessorStaticV3 => {
                        K2UncertaintyR8BEvidenceKindV2::V3Controls
                    }
                    K2UncertaintyControlScopeV1::SuccessorStaticV4 => {
                        K2UncertaintyR8BEvidenceKindV2::V4Controls
                    }
                    K2UncertaintyControlScopeV1::DevelopmentRehearsalV5 => {
                        K2UncertaintyR8BEvidenceKindV2::FreshControlCases
                    }
                    K2UncertaintyControlScopeV1::SealedAttemptV5 => {
                        panic!("sealed control forbidden in R8B")
                    }
                },
                &output.stdout,
                &receipt.schema,
                receipt.receipt_root_sha256.clone(),
            );
            ledger.finish_bound(
                &started,
                &output,
                receipt.schema.clone(),
                receipt.receipt_root_sha256.clone(),
                K2UncertaintyR8BValidatedFactV3::None,
                vec![descriptor],
            );
            measurements.observe_bytes(&input);
            measurements.observe_bytes(&output.stdout);
            assert!(receipt.all_pass);
            assert_eq!(receipt.passed, request.scope.expected_count() as u64);
            receipt
        })
        .collect()
}

fn fresh_control_outcomes_v2(
    ledger: &mut DurableProcessLedgerV2,
    binaries: &LinkedBinariesV2,
    route_root: &Path,
    experiment: &str,
    freeze: &str,
    measurements: &mut MeasurementsV2,
) -> Vec<K2UncertaintyControlProcessOutcomeV1> {
    let runner = binaries.get("M19_FRESH_CONTROL_CASE");
    let owner = binaries.get("M24_LINKED_RUNNER");
    let root = route_root.join("control-cases");
    create_private_directory_v1(&root);
    let fixture = PathBuf::from(
        std::env::var_os("NANDO_K2_R7K_FIXTURE_ROOT")
            .expect("NANDO_K2_R7K_FIXTURE_ROOT is required"),
    );
    let adapter = source_root_v2(&[
        "src/bin/nando-k2-self-formed-r7k-control-case.rs",
        "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_control_cases.rs",
    ]);
    [
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
    ]
    .into_iter()
    .map(|(control_id, disposition, subcases)| {
        let mount = root.join(control_id.to_ascii_lowercase());
        let mut scratch = mount.clone();
        if control_id == "K3" {
            for value in canaries_v2() {
                scratch.push(String::from_utf8(value).expect("UTF-8 K3 canary"));
            }
        }
        create_private_directory_v1(&scratch);
        prepare_control_scratch_v2(control_id, &scratch, &fixture, binaries);
        let guest = PathBuf::from("/scratch").join(
            scratch
                .strip_prefix(&mount)
                .expect("fresh control scratch under mount"),
        );
        let target = control_source_root_v2(control_id);
        let request = new_self_formed_r7k_control_case_request_v1(
            control_id.to_owned(),
            experiment.to_owned(),
            freeze.to_owned(),
            guest.to_string_lossy().into_owned(),
            (0..subcases)
                .map(|ordinal| root_v1(&format!("{control_id}-subcase-{ordinal}")))
                .collect(),
            target.clone(),
            adapter.clone(),
        )
        .expect("fresh M19 request");
        let argv = if control_id == "K3" {
            canaries_v2()
                .into_iter()
                .map(|value| String::from_utf8(value).expect("UTF-8 K3 argv"))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let environment = if control_id == "K3" {
            canaries_v2()
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    (
                        format!("NANDO_K2_R7K_CANARY_{index}"),
                        String::from_utf8(value).expect("UTF-8 K3 env"),
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let input = uncertainty_bytes_v1(&request).expect("M19 request bytes");
        let started = ledger.start_bound(
            "C11",
            Some(root_v1(&format!("fresh-control-{control_id}"))),
            None,
            runner,
            request.request_root_sha256.clone(),
            composition_sha256_bytes_v1(&input),
        );
        let outcome = run_self_formed_r7k_control_sandbox_v1(
            &runner.path,
            &runner.sha256,
            &mount,
            &input,
            &argv,
            &environment,
            60,
        )
        .unwrap_or_else(|error| ledger.fail_launch_bound(&started, error));
        let output = sandbox_output_v2(outcome.clone());
        if !output.status.success() {
            ledger.fail_unexpected_bound(&started, &output, format!("M19 {control_id} failed"));
        }
        let decoded: K2UncertaintyControlStdoutV1 = uncertainty_decode_v1(&output.stdout)
            .unwrap_or_else(|error| {
                ledger.fail_unexpected_bound(
                    &started,
                    &output,
                    format!("decode M19 stdout: {error}"),
                )
            });
        assert_eq!(decoded.control_id, control_id);
        assert_eq!(decoded.disposition, disposition);
        assert_eq!(
            output.stdout,
            uncertainty_bytes_v1(&decoded).expect("canonical M19 stdout")
        );
        ledger.finish_bound(
            &started,
            &output,
            "nando.k2-self-formed-control-stdout.v1".to_owned(),
            uncertainty_root_v1(&decoded).expect("M19 stdout root"),
            K2UncertaintyR8BValidatedFactV3::None,
            Vec::new(),
        );
        measurements.observe_bytes(&input);
        measurements.observe_bytes(&output.stdout);
        let log = uncertainty_bytes_v1(&(
            control_id,
            &request.request_root_sha256,
            outcome.normal_exit,
            outcome.exit_code,
            composition_sha256_bytes_v1(&outcome.stdout),
            composition_sha256_bytes_v1(&outcome.stderr),
            outcome.timed_out,
            &runner.sha256,
            &target,
            &adapter,
        ))
        .expect("M19 process log");
        let log_path = root.join("logs").join(format!("{control_id}.json"));
        create_private_directory_v1(log_path.parent().expect("M19 log parent"));
        write_new_read_only_v2(&log_path, &log);
        K2UncertaintyControlProcessOutcomeV1::seal(
            K2UncertaintyControlScopeV1::DevelopmentRehearsalV5,
            control_id.to_owned(),
            experiment.to_owned(),
            Some(freeze.to_owned()),
            None,
            runner.sha256.clone(),
            owner.sha256.clone(),
            request.request_root_sha256,
            outcome.normal_exit,
            outcome.exit_code,
            outcome.stdout,
            composition_sha256_bytes_v1(&outcome.stderr),
            outcome.timed_out,
            outcome
                .stderr
                .windows(10)
                .any(|value| value == b"panicked at"),
            decoded.disposition,
            composition_root_v1(&(target, adapter.clone())).expect("M19 source root"),
            composition_sha256_bytes_v1(&log),
        )
        .expect("seal measured M19 outcome")
    })
    .collect()
}

#[rustfmt::skip]
#[path = "k2_self_formed_uncertainty_confirm_r8b_linked_v1/parent.rs"]
mod parent;
