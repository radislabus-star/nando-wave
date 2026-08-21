use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use nando_operator_learning::{
    K2_UNCERTAINTY_ORACLE_DESCRIPTOR_SCHEMA_V1, K2_UNCERTAINTY_ORACLE_MANIFEST_FILE_V1, K2_UNCERTAINTY_RAW_PROBES_V1, K2CompositionTreeManifestV1,
    K2InquiryObservationReceiptV1, K2InquiryWorkerOutcomeV1, K2UncertaintyCaseJournalFaultV2, K2UncertaintyCaseJournalV2, K2UncertaintyConfirmDataMountV1,
    K2UncertaintyConfirmFinalVerifierReceiptV1, K2UncertaintyConfirmFinalVerifierRequestV1, K2UncertaintyConfirmGuestExecutableV1,
    K2UncertaintyConfirmMountTargetV1, K2UncertaintyConfirmPlanSafetyBindingV1, K2UncertaintyConfirmSafetyReceiptV1, K2UncertaintyConfirmSafetyRequestV1,
    K2UncertaintyControlEvaluationReceiptV1, K2UncertaintyControlEvaluationRequestV1, K2UncertaintyControlProcessOutcomeV1, K2UncertaintyControlScopeV1,
    K2UncertaintyControlStdoutV1, K2UncertaintyDevelopmentRehearsalMetadataV1, K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1,
    K2UncertaintyDevelopmentRehearsalStoredArtifactV1, K2UncertaintyEvaluationResourceMeasurementsV1, K2UncertaintyEvaluationRouteReceiptV1,
    K2UncertaintyObservationVectorV2, K2UncertaintyOracleBaselineBatchReceiptV1, K2UncertaintyOracleBaselineCaseDescriptorV1,
    K2UncertaintyOracleBaselineCaseReceiptV1, K2UncertaintyOracleCaseEvidenceManifestV1, K2UncertaintyOracleEvidenceEntryV1,
    K2UncertaintyOracleEvidenceKindV1, K2UncertaintyOraclePublicBindingsV1, K2UncertaintyPrivateResolverReceiptV1, K2UncertaintyPrivateResolverRequestV1,
    K2UncertaintyProbeExecutionEvidenceV2, K2UncertaintyPublicPrecommitReceiptV1, K2UncertaintyPublicPreparedCaseV1, K2UncertaintyWorkspaceIdentityV2,
    composition_root_v1, composition_sha256_bytes_v1, composition_sha256_file_v1, expected_self_formed_control_v1, load_self_formed_public_case_v1,
    materialize_self_formed_probe_files_v1, new_self_formed_r7k_control_case_request_v1, prepare_self_formed_confirm_plan_dispatch_v1,
    publish_self_formed_final_verifier_material_v2, reopen_self_formed_probe_output_v1, run_self_formed_confirm_sandbox_measured_v1,
    run_self_formed_r7k_control_sandbox_v1, uncertainty_bytes_v1, uncertainty_decode_v1, uncertainty_root_v1,
};

use super::{
    BinaryV2, DurableProcessLedgerV2, LinkedBinariesV2, PrivateDescriptorV2, create_private_directory_v1, freeze_directory_tree_v2, root_v1,
    run_recorded_process_v2, sandbox_output_v2, write_new_read_only_v2,
};

pub(super) struct DownstreamOutputV2 {
    pub(super) oracle: K2UncertaintyOracleBaselineBatchReceiptV1,
    pub(super) controls: Vec<K2UncertaintyControlEvaluationReceiptV1>,
    pub(super) routes: Vec<K2UncertaintyEvaluationRouteReceiptV1>,
    pub(super) resources: K2UncertaintyEvaluationResourceMeasurementsV1,
}

#[derive(Default)]
struct MeasurementsV2 {
    maximum_protocol_bytes: u64,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_downstream_v2(
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
        let prepared = load_self_formed_public_case_v1(public_root, artifact).expect("reopen M24 prepared public case");
        let case_id = prepared.probe_request.public_case.vocabulary.case_id_sha256.clone();
        let resolver_artifact = private_artifact_v2(metadata, &case_id, K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::ResolverTable);
        let truth_artifact = private_artifact_v2(metadata, &case_id, K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1::FinalTruth);
        let plan = prepared.preverification.closure_plan.as_ref().expect("publicly frozen M24 closure plan");
        plan_lengths.push(plan.plan_length);
        let planner = &prepared.preverification.closure_verification_request.planner_request;
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
                prepared.probe_request.public_case.public_case_root_sha256.clone(),
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
                |receipt: &K2UncertaintyPrivateResolverReceiptV1| (&receipt.schema, &receipt.receipt_root_sha256),
            );
            resolver_receipt.validate().expect("validate M11 private resolver receipt");
            assert_eq!(resolver_receipt.exposed_effect_count, 1);

            let workspace_identity =
                K2UncertaintyWorkspaceIdentityV2::seal(case_id.clone(), plan.plan_root_sha256.clone(), ordinal as u64).expect("M24 workspace identity");
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
                |receipt: &K2UncertaintyConfirmSafetyReceiptV1| (&receipt.schema, &receipt.receipt_root_sha256),
            );
            safety_receipt.validate().expect("validate M12 safety receipt");
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
        let mut journal = K2UncertaintyCaseJournalV2::create(&journal_root, dispatch.clone()).expect("create M24 case journal");
        journal
            .record_plan_dispatch(binaries.get("M10_PUBLIC_COORDINATOR").sha256.clone(), K2UncertaintyCaseJournalFaultV2::None)
            .expect("freeze M24 dispatch before worker");
        let mut executions = Vec::with_capacity(dispatch.items.len());

        for item in &dispatch.items {
            let source = execution_root.join(format!("source-{}", item.workspace_identity.identity_root_sha256));
            let workspace = execution_root.join(format!("work-{}", item.workspace_identity.identity_root_sha256));
            let files = materialize_self_formed_probe_files_v1(&prepared.probe_request.public_case, &item.selected_probe)
                .expect("materialize M24 selected public state");
            materialize_v2(&source, &files);
            materialize_v2(&workspace, &files);
            let source_before = K2CompositionTreeManifestV1::scan(&source).expect("M24 source before");
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
                |receipt: &K2InquiryWorkerOutcomeV1| (&receipt.schema, &receipt.outcome_root_sha256),
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
                |receipt: &K2InquiryObservationReceiptV1| (&receipt.schema, &receipt.receipt_root_sha256),
            );
            assert_eq!(worker_outcome.post_manifest, observation.post_manifest);
            assert_eq!(source_before, K2CompositionTreeManifestV1::scan(&source).expect("M24 source after"));
            let evidence = K2UncertaintyProbeExecutionEvidenceV2::seal(dispatch.dispatch_root_sha256.clone(), item, worker_outcome, observation)
                .expect("seal M24 execution evidence");
            journal
                .record_probe_observation(permit, evidence.observation.receipt_root_sha256.clone(), K2UncertaintyCaseJournalFaultV2::None)
                .expect("freeze M24 observation");
            executions.push(evidence);
        }

        let observation_vector = K2UncertaintyObservationVectorV2::seal(&dispatch, executions).expect("seal M24 ordered observation vector");
        journal
            .freeze_observation_vector(
                binaries.get("M10_PUBLIC_COORDINATOR").sha256.clone(),
                observation_vector.vector_root_sha256.clone(),
                observation_vector.vector_root_sha256.clone(),
                K2UncertaintyCaseJournalFaultV2::None,
            )
            .expect("freeze M24 complete observation vector");
        let evidence_root = public_root.join(format!("probes/case-{:02}", artifact.case_sequence));
        let material = publish_self_formed_final_verifier_material_v2(&evidence_root, &public_receipt.batch_precommit, &prepared.preverification)
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
            |receipt: &K2UncertaintyConfirmFinalVerifierReceiptV1| (&receipt.schema, &receipt.receipt_root_sha256),
        );
        final_receipt.validate().expect("validate M15 final verifier receipt");
        assert_eq!(final_receipt.final_truth_root_sha256, truth_artifact.semantic_root_sha256);
        assert!(final_receipt.verification.private_true_class_match);
        assert_eq!(final_receipt.verification.false_accepts, 0);

        let probe_output = reopen_self_formed_probe_output_v1(&evidence_root, &prepared.probe_artifacts).expect("reopen M24 public probe output");
        let public_bindings = K2UncertaintyOraclePublicBindingsV1::seal(public_receipt.clone(), prepared.clone()).expect("seal M24 Oracle public bindings");
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
            public_case_root_sha256: prepared.probe_request.public_case.public_case_root_sha256.clone(),
            prepared_case_root_sha256: prepared.prepared_case_root_sha256.clone(),
            closure_plan_root_sha256: plan.plan_root_sha256.clone(),
            baseline_summary_root_sha256: prepared.selection_preverification.baseline_summary.summary_root_sha256.clone(),
            observation_vector_root_sha256: observation_vector.vector_root_sha256.clone(),
            final_verifier_receipt_root_sha256: final_receipt.receipt_root_sha256.clone(),
            private_truth_artifact_root_sha256: truth_artifact.semantic_root_sha256.clone(),
            case_evidence_manifest_root_sha256: manifest.manifest_root_sha256.clone(),
            oracle_evaluator_executable_sha256: oracle.sha256.clone(),
        };
        descriptor.validate().expect("validate M24 Oracle descriptor");
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
            |receipt: &K2UncertaintyOracleBaselineCaseReceiptV1| (&receipt.schema, &receipt.receipt_root_sha256),
        );
        oracle_receipt.validate().expect("validate M16 Oracle receipt");
        assert_eq!(oracle_receipt.reconstructed_frontier.raw_probe_count, K2_UNCERTAINTY_RAW_PROBES_V1 as u64);
        assert!(oracle_receipt.oracle_equality);
        assert!(oracle_receipt.model_guided_observation_parity);
        assert!(oracle_receipt.model_guided.true_class_retained);
        assert!(oracle_receipt.oracle.true_class_retained);
        assert_eq!(oracle_receipt.exact_plan_denominator, oracle_receipt.reconstructed_frontier.class_count.pow(2));
        oracle_receipts.push(oracle_receipt);
        maximum_case_wall_ms = maximum_case_wall_ms.max(case_started.elapsed().as_millis() as u64);
    }

    assert_eq!(plan_lengths.iter().filter(|length| **length == 1).count(), 8);
    assert_eq!(plan_lengths.iter().filter(|length| **length == 2).count(), 8);
    let oracle = K2UncertaintyOracleBaselineBatchReceiptV1::seal(metadata.public_batch.experiment_id_sha256.clone(), oracle_receipts)
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
                let (control_id, disposition) = expected_self_formed_control_v1(scope, ordinal).expect("static control");
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
        K2UncertaintyControlEvaluationRequestV1::seal(scope, experiment.to_owned(), None, None, outcomes, evaluator.sha256.clone())
            .expect("static M17 request")
    })
    .collect::<Vec<_>>();
    requests.push(
        K2UncertaintyControlEvaluationRequestV1::seal(
            K2UncertaintyControlScopeV1::DevelopmentRehearsalV5,
            experiment.to_owned(),
            Some(freeze.to_owned()),
            None,
            fresh_control_outcomes_v2(ledger, binaries, route_root, experiment, freeze, measurements),
            evaluator.sha256.clone(),
        )
        .expect("fresh M17 request"),
    );
    requests
        .into_iter()
        .map(|request| {
            let input = uncertainty_bytes_v1(&request).expect("M17 request bytes");
            let process = run_recorded_process_v2::<_, K2UncertaintyControlEvaluationReceiptV1, _>(
                ledger,
                evaluator,
                "C12",
                None,
                None,
                request.request_root_sha256.clone(),
                &request,
                |receipt| (&receipt.schema, &receipt.receipt_root_sha256),
            );
            measurements.observe_bytes(&input);
            measurements.observe_bytes(&process.output.stdout);
            assert!(process.value.all_pass);
            assert_eq!(process.value.passed, request.scope.expected_count() as u64);
            process.value
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
    let fixture = PathBuf::from(std::env::var_os("NANDO_K2_R7K_FIXTURE_ROOT").expect("NANDO_K2_R7K_FIXTURE_ROOT is required"));
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
        let guest = PathBuf::from("/scratch").join(scratch.strip_prefix(&mount).expect("fresh control scratch under mount"));
        let target = control_source_root_v2(control_id);
        let request = new_self_formed_r7k_control_case_request_v1(
            control_id.to_owned(),
            experiment.to_owned(),
            freeze.to_owned(),
            guest.to_string_lossy().into_owned(),
            (0..subcases).map(|ordinal| root_v1(&format!("{control_id}-subcase-{ordinal}"))).collect(),
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
                .map(|(index, value)| (format!("NANDO_K2_R7K_CANARY_{index}"), String::from_utf8(value).expect("UTF-8 K3 env")))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let input = uncertainty_bytes_v1(&request).expect("M19 request bytes");
        let started = ledger.start(
            "C11",
            Some(root_v1(&format!("fresh-control-{control_id}"))),
            None,
            runner,
            request.request_root_sha256.clone(),
            composition_sha256_bytes_v1(&input),
        );
        let outcome =
            run_self_formed_r7k_control_sandbox_v1(&runner.path, &runner.sha256, &mount, &input, &argv, &environment, 60).expect("run actual M19 control");
        let output = sandbox_output_v2(outcome.clone());
        assert!(output.status.success(), "M19 {control_id} failed");
        let decoded: K2UncertaintyControlStdoutV1 = uncertainty_decode_v1(&output.stdout).expect("decode M19 stdout");
        assert_eq!(decoded.control_id, control_id);
        assert_eq!(decoded.disposition, disposition);
        assert_eq!(output.stdout, uncertainty_bytes_v1(&decoded).expect("canonical M19 stdout"));
        ledger.finish(
            &started,
            &output,
            "nando.k2-self-formed-control-stdout.v1".to_owned(),
            uncertainty_root_v1(&decoded).expect("M19 stdout root"),
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
            outcome.stderr.windows(10).any(|value| value == b"panicked at"),
            decoded.disposition,
            composition_root_v1(&(target, adapter.clone())).expect("M19 source root"),
            composition_sha256_bytes_v1(&log),
        )
        .expect("seal measured M19 outcome")
    })
    .collect()
}

fn prepare_control_scratch_v2(id: &str, scratch: &Path, fixture: &Path, binaries: &LinkedBinariesV2) {
    if id == "K1" {
        let seed = fs::read(std::env::var_os("NANDO_K2_DEVELOPMENT_SEED_PATH").expect("NANDO_K2_DEVELOPMENT_SEED_PATH is required"))
            .expect("read Development seed for K1");
        fs::write(scratch.join("development-seed.bin"), seed).expect("write K1 seed");
    }
    if matches!(id, "K5" | "K6" | "K9" | "K10" | "K11") {
        copy_tree_v2(&fixture.join("fixture-packet"), &scratch.join("fixture-packet"));
        if id == "K10" {
            copy_tree_v2(&fixture.join("oracle-case-00"), &scratch.join("oracle-case-00"));
        }
    }
    if id == "K3" {
        create_private_directory_v1(&scratch.join("public"));
        let persisted = canaries_v2()
            .into_iter()
            .flat_map(|mut value| {
                value.push(b'\n');
                value
            })
            .collect::<Vec<_>>();
        fs::write(scratch.join("persisted-generator-request.bin"), persisted).expect("write K3 persisted canary");
    }
    if id == "K4" {
        let public = scratch.join("public");
        create_private_directory_v1(&public);
        for (index, value) in canaries_v2().into_iter().enumerate() {
            fs::write(public.join(format!("artifact-{index}.bin")), value).expect("write K4 canary");
        }
    }
    if id == "K7" {
        let target = scratch.join("public-coordinator");
        fs::copy(&binaries.get("M10_PUBLIC_COORDINATOR").path, &target).expect("copy K7 coordinator");
        fs::set_permissions(target, fs::Permissions::from_mode(0o400)).expect("chmod K7 coordinator");
    }
}

fn canaries_v2() -> Vec<Vec<u8>> {
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

fn control_source_root_v2(id: &str) -> String {
    let paths: &[&str] = match id {
        "K1" => &["src/k2_goal_environment/learned_composition/self_formed_uncertainty/generator_model.rs"],
        "K2" => &["src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_owner_model.rs"],
        "K3" | "K4" => &["src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_control_canary.rs"],
        "K5" => &["src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_private_resolver.rs"],
        "K6" => &["src/k2_goal_environment/learned_composition/self_formed_uncertainty/observation_vector_v2.rs"],
        "K7" => &["src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_public_coordinator.rs"],
        "K8" => &[
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_authorization.rs",
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_attempt_model.rs",
        ],
        "K9" => &[
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_evaluation_model.rs",
            "src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_terminal.rs",
        ],
        "K10" => &["src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_oracle_process.rs"],
        "K11" => &["src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_evaluation_model.rs"],
        "K12" => &["src/k2_goal_environment/learned_composition/self_formed_uncertainty/confirm_cleanup_verifier.rs"],
        _ => panic!("unknown M19 control {id}"),
    };
    source_root_v2(paths)
}

fn source_root_v2(paths: &[&str]) -> String {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rows = paths
        .iter()
        .map(|path| (*path, composition_sha256_file_v1(&crate_root.join(path)).expect("M19 source SHA")))
        .collect::<Vec<_>>();
    composition_root_v1(&rows).expect("M19 source manifest root")
}

fn copy_tree_v2(source: &Path, destination: &Path) {
    create_private_directory_v1(destination);
    for entry in fs::read_dir(source).expect("read M19 fixture") {
        let entry = entry.expect("M19 fixture entry");
        let source = entry.path();
        let target = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source).expect("stat M19 fixture");
        assert!(!metadata.file_type().is_symlink());
        if metadata.is_dir() {
            copy_tree_v2(&source, &target);
        } else {
            assert!(metadata.is_file());
            fs::copy(source, &target).expect("copy M19 fixture");
            fs::set_permissions(target, fs::Permissions::from_mode(0o400)).expect("chmod M19 fixture");
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_recorded_sandbox_v2<I, O, F>(
    ledger: &mut DurableProcessLedgerV2,
    binary: &BinaryV2,
    role: K2UncertaintyConfirmGuestExecutableV1,
    stage: &str,
    case_id_sha256: &str,
    probe_ordinal: Option<u64>,
    request_root_sha256: &str,
    mounts: &[K2UncertaintyConfirmDataMountV1<'_>],
    request: &I,
    measurements: &mut MeasurementsV2,
    receipt_root: F,
) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
    F: FnOnce(&O) -> (&str, &str),
{
    let input = uncertainty_bytes_v1(request).expect("M24 sandbox request bytes");
    let started = ledger.start(
        stage,
        Some(case_id_sha256.to_owned()),
        probe_ordinal,
        binary,
        request_root_sha256.to_owned(),
        composition_sha256_bytes_v1(&input),
    );
    let outcome = run_self_formed_confirm_sandbox_measured_v1(role, &binary.path, &binary.sha256, mounts, &input, 60).expect("run M24 Confirm sandbox");
    let output = sandbox_output_v2(outcome);
    assert!(output.status.success(), "{} failed: {}", binary.role, String::from_utf8_lossy(&output.stderr));
    let value: O = uncertainty_decode_v1(&output.stdout).unwrap_or_else(|error| panic!("{} output invalid: {error}", binary.role));
    assert_eq!(output.stdout, uncertainty_bytes_v1(&value).expect("canonical M24 sandbox receipt"));
    let (schema, root) = receipt_root(&value);
    ledger.finish(&started, &output, schema.to_owned(), root.to_owned());
    measurements.observe_bytes(&input);
    measurements.observe_bytes(&output.stdout);
    value
}

fn private_artifact_v2<'a>(
    metadata: &'a K2UncertaintyDevelopmentRehearsalMetadataV1,
    case_id_sha256: &str,
    kind: K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1,
) -> &'a K2UncertaintyDevelopmentRehearsalStoredArtifactV1 {
    metadata
        .split
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == kind && artifact.case_id_sha256.as_deref() == Some(case_id_sha256))
        .expect("M24 private artifact metadata")
}

fn materialize_v2(root: &Path, files: &std::collections::BTreeMap<String, Vec<u8>>) {
    create_private_directory_v1(root);
    for (relative, bytes) in files {
        let path = root.join(relative);
        create_private_directory_v1(path.parent().expect("M24 materialized parent"));
        fs::write(path, bytes).expect("write M24 materialized file");
    }
}

fn publish_oracle_case_evidence_v2(
    root: &Path,
    public_bindings: &K2UncertaintyOraclePublicBindingsV1,
    prepared: &K2UncertaintyPublicPreparedCaseV1,
    probe_output: &nando_operator_learning::K2UncertaintyProbeOutputV1,
    observation_vector: &K2UncertaintyObservationVectorV2,
    final_receipt: &K2UncertaintyConfirmFinalVerifierReceiptV1,
    truth_artifact: &K2UncertaintyDevelopmentRehearsalStoredArtifactV1,
) -> K2UncertaintyOracleCaseEvidenceManifestV1 {
    create_private_directory_v1(root);
    let model_set = &public_bindings.probe_request.learner_response.model_set;
    let closure_plan = prepared.preverification.closure_plan.as_ref().expect("M24 frozen closure plan");
    let mut entries = vec![
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::PublicBindings,
            "public-bindings.json",
            &public_bindings.bindings_root_sha256,
            public_bindings,
        ),
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::ModelSet,
            "model-set.json",
            &model_set.model_set_root_sha256,
            model_set,
        ),
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::FrontierCensus,
            "frontier-census.json",
            &probe_output.frontier.frontier_root_sha256,
            &probe_output.frontier,
        ),
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::ClosurePlan,
            "closure-plan.json",
            &closure_plan.plan_root_sha256,
            closure_plan,
        ),
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::ClosurePreverification,
            "closure-preverification.json",
            &prepared.preverification.receipt_root_sha256,
            &prepared.preverification,
        ),
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::BaselineSummary,
            "baseline-summary.json",
            &prepared.selection_preverification.baseline_summary.summary_root_sha256,
            &prepared.selection_preverification.baseline_summary,
        ),
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::ObservationVector,
            "observation-vector.json",
            &observation_vector.vector_root_sha256,
            observation_vector,
        ),
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::FinalVerifierReceipt,
            "final-verifier-receipt.json",
            &final_receipt.receipt_root_sha256,
            final_receipt,
        ),
        K2UncertaintyOracleEvidenceEntryV1::seal(
            K2UncertaintyOracleEvidenceKindV1::PrivateTruth,
            "private-truth.json".to_owned(),
            truth_artifact.byte_len,
            truth_artifact.unix_mode,
            truth_artifact.content_sha256.clone(),
            truth_artifact.semantic_root_sha256.clone(),
        )
        .expect("seal descriptor-only M24 private truth entry"),
    ];
    for page in &probe_output.pages {
        entries.push(write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::FrontierPage,
            &format!("frontier-pages/page-{:04}.json", page.page_sequence),
            &page.page_root_sha256,
            page,
        ));
    }
    let manifest = K2UncertaintyOracleCaseEvidenceManifestV1::seal(prepared.probe_request.public_case.vocabulary.case_id_sha256.clone(), entries)
        .expect("seal M24 Oracle evidence manifest");
    write_new_read_only_v2(
        &root.join(K2_UNCERTAINTY_ORACLE_MANIFEST_FILE_V1),
        &uncertainty_bytes_v1(&manifest).expect("M24 Oracle manifest bytes"),
    );
    freeze_directory_tree_v2(root);
    assert!(!root.join("private-truth.json").exists());
    manifest
}

fn write_oracle_entry_v2<T: serde::Serialize>(
    root: &Path,
    kind: K2UncertaintyOracleEvidenceKindV1,
    relative: &str,
    semantic_root_sha256: &str,
    value: &T,
) -> K2UncertaintyOracleEvidenceEntryV1 {
    let bytes = uncertainty_bytes_v1(value).expect("M24 Oracle evidence bytes");
    let path = root.join(relative);
    create_private_directory_v1(path.parent().expect("M24 Oracle evidence parent"));
    write_new_read_only_v2(&path, &bytes);
    K2UncertaintyOracleEvidenceEntryV1::seal(
        kind,
        relative.to_owned(),
        bytes.len() as u64,
        0o400,
        composition_sha256_bytes_v1(&bytes),
        semantic_root_sha256.to_owned(),
    )
    .expect("seal M24 Oracle evidence entry")
}

fn evaluation_routes_v2(binaries: &LinkedBinariesV2, case_execution_count: u64) -> Vec<K2UncertaintyEvaluationRouteReceiptV1> {
    [
        ("public_precommit", "M10_PUBLIC_COORDINATOR", "M16_ORACLE", 16),
        ("case_execution", "M13_WORKER", "M15_FINAL_VERIFIER", case_execution_count),
        ("final_verification", "M15_FINAL_VERIFIER", "M16_ORACLE", 16),
        ("oracle_evaluation", "M16_ORACLE", "M18_TERMINAL_EVALUATOR", 16),
        ("control_evaluation", "M17_CONTROL_EVALUATOR", "M18_TERMINAL_EVALUATOR", 64),
    ]
    .into_iter()
    .map(|(route, producer, consumer, events)| {
        K2UncertaintyEvaluationRouteReceiptV1::seal(
            route.to_owned(),
            binaries.get(producer).sha256.clone(),
            binaries.get(consumer).sha256.clone(),
            events,
            events,
        )
        .expect("seal M24 evaluation route")
    })
    .collect()
}

impl MeasurementsV2 {
    fn observe_bytes(&mut self, bytes: &[u8]) {
        self.maximum_protocol_bytes = self.maximum_protocol_bytes.max(bytes.len() as u64);
    }
}
