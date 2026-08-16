use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use nando_operator_learning::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1,
    K2InquiryBaselineRequestV1, K2InquiryBaselinesV1, K2InquiryObservationReceiptV1,
    K2InquiryVerifierCommandV1, K2InquiryVerifierReceiptV1, K2InquiryWorkerOutcomeV1,
    K2UncertaintyBatchJournalEventKindV1, K2UncertaintyBatchJournalV1,
    K2UncertaintyBatchPrecommitV2, K2UncertaintyCaseJournalFaultV2,
    K2UncertaintyCaseJournalPhaseV2, K2UncertaintyCaseJournalV2,
    K2UncertaintyCasePreverificationV1, K2UncertaintyCasePreverificationV2,
    K2UncertaintyCaseVerificationReceiptV2, K2UncertaintyClosureCensusV1,
    K2UncertaintyClosurePlanV1, K2UncertaintyClosurePlannerRequestV1,
    K2UncertaintyClosureVerificationReceiptV1, K2UncertaintyClosureVerificationRequestV1,
    K2UncertaintyFinalVerifierRequestV2, K2UncertaintyGeneratorRequestV1,
    K2UncertaintyGeneratorResponseV1, K2UncertaintyLearnerRequestV1,
    K2UncertaintyLearnerResponseV1, K2UncertaintyObservationVectorV2,
    K2UncertaintyPlanSafetyBindingV2, K2UncertaintyPrivateSafetyDispositionV1,
    K2UncertaintyProbeArtifactsV1, K2UncertaintyProbeExecutionEvidenceV2,
    K2UncertaintyProbeRequestV1, K2UncertaintySafetyReceiptV1, K2UncertaintySafetyRequestV1,
    K2UncertaintyTournamentArtifactsV1, K2UncertaintyWorkspaceIdentityV2, composition_root_v1,
    composition_sha256_file_v1, enumerate_self_formed_probe_frontier_v1,
    materialize_self_formed_probe_files_v1, prepare_self_formed_plan_dispatch_v2,
    preverify_self_formed_case_with_owner_v1, publish_self_formed_final_verifier_material_v2,
    publish_self_formed_probe_output_v1, reopen_self_formed_probe_output_v1,
    run_self_formed_tournament_with_owners_v1, self_formed_grammar_root_v1, uncertainty_bytes_v1,
    uncertainty_decode_v1,
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn r7_v4_real_owners_close_all_cases_with_precommitted_probe_plans() {
    let environment = TestEnvironment::new();
    let binaries = ProcessBinaries::from_cargo();
    binaries.assert_pairwise_distinct();
    let seed_path = std::env::var_os("NANDO_K2_DEVELOPMENT_SEED_PATH")
        .map(PathBuf::from)
        .expect("NANDO_K2_DEVELOPMENT_SEED_PATH is required for R7 V4 process test");
    let generated: K2UncertaintyGeneratorResponseV1 = run_process(
        &binaries.generator,
        &K2UncertaintyGeneratorRequestV1::development(
            fs::read(seed_path).expect("read development seed"),
            binaries.generator_sha256.clone(),
        )
        .expect("generator request"),
    );

    let mut prepared = Vec::new();
    for (case_sequence, public_case) in generated.public.cases.iter().enumerate() {
        eprintln!("R7 V4 preverify case {}/16", case_sequence + 1);
        let learned: K2UncertaintyLearnerResponseV1 = run_process(
            &binaries.learner,
            &K2UncertaintyLearnerRequestV1::seal(
                public_case.vocabulary.clone(),
                public_case.support.clone(),
                binaries.learner_sha256.clone(),
            )
            .expect("learner request"),
        );
        let probe_request = K2UncertaintyProbeRequestV1::seal(
            public_case.clone(),
            learned.clone(),
            generated.public.split_commitment_root_sha256.clone(),
            binaries.probe_sha256.clone(),
        )
        .expect("probe request");
        let artifact_root = environment
            .root
            .join(format!("frontier-{case_sequence:02}"));
        let (probe_artifacts, probe_output) = if case_sequence == 0 {
            fs::create_dir_all(&artifact_root).expect("create probe output root");
            let receipt: K2UncertaintyProbeArtifactsV1 = run_isolated(
                &binaries.probe,
                &probe_request,
                &[Mount::ReadWrite(&artifact_root, "/out")],
                120,
            );
            let output = reopen_self_formed_probe_output_v1(&artifact_root, &receipt)
                .expect("reopen process-owned frontier");
            (receipt, output)
        } else {
            let output =
                enumerate_self_formed_probe_frontier_v1(&probe_request).expect("complete frontier");
            let receipt = publish_self_formed_probe_output_v1(&artifact_root, &output)
                .expect("publish complete frontier");
            (receipt, output)
        };

        let tournament = run_self_formed_tournament_with_owners_v1(
            public_case,
            &learned,
            &probe_output,
            &generated.public.split_commitment_root_sha256,
            K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1,
            &binaries.selector_sha256,
            &binaries.baseline_sha256,
            &mut |request| Ok(run_process(&binaries.selector, request)),
            &mut |request: &K2InquiryBaselineRequestV1| {
                Ok(run_process::<_, K2InquiryBaselinesV1>(
                    &binaries.baseline,
                    request,
                ))
            },
        )
        .expect("process-owned tournament");
        let preverification = preverify_self_formed_case_with_owner_v1(
            &tournament,
            &probe_artifacts,
            &binaries.baseline_sha256,
            &binaries.preverifier_sha256,
            &mut |command: &K2InquiryVerifierCommandV1| {
                Ok(run_process::<_, K2InquiryVerifierReceiptV1>(
                    &binaries.preverifier,
                    command,
                ))
            },
        )
        .expect("process-owned preverification");
        let representative_roots = probe_output
            .frontier
            .representative_probe_roots_sha256
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let representatives = probe_output
            .pages
            .iter()
            .flat_map(|page| &page.dispositions)
            .filter(|disposition| {
                representative_roots.contains(&disposition.probe.probe_root_sha256)
            })
            .cloned()
            .collect::<Vec<_>>();
        let closure_planner_request = K2UncertaintyClosurePlannerRequestV1::seal(
            public_case.vocabulary.case_id_sha256.clone(),
            probe_output.frontier.frontier_root_sha256.clone(),
            preverification.tournament.tournament_root_sha256.clone(),
            preverification
                .tournament
                .tournament_winner_probe_root_sha256
                .clone(),
            representatives,
            binaries.closure_planner_sha256.clone(),
        )
        .expect("closure planner request");
        let closure_census: K2UncertaintyClosureCensusV1 =
            run_isolated(&binaries.closure_planner, &closure_planner_request, &[], 60);
        let closure_verification_request = K2UncertaintyClosureVerificationRequestV1::seal(
            binaries.closure_verifier_sha256.clone(),
            closure_planner_request,
            closure_census.clone(),
        )
        .expect("closure verification request");
        assert!(
            uncertainty_bytes_v1(&closure_verification_request)
                .expect("closure request bytes")
                .len()
                <= K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1
        );
        let closure_verification_receipt: K2UncertaintyClosureVerificationReceiptV1 = run_isolated(
            &binaries.closure_verifier,
            &closure_verification_request,
            &[],
            60,
        );
        let closure_plan = K2UncertaintyClosurePlanV1::seal(
            &closure_verification_request.planner_request,
            &closure_census,
            &closure_verification_receipt,
        )
        .expect("closure plan");
        let preverification_v2 = K2UncertaintyCasePreverificationV2::seal(
            preverification.clone(),
            closure_verification_request,
            closure_verification_receipt,
            Some(closure_plan),
        )
        .expect("V4 case preverification");
        prepared.push(PreparedCase {
            probe_request,
            probe_artifacts,
            tournament,
            selection_preverification: preverification,
            preverification: preverification_v2,
        });
    }
    eprintln!("R7 V4 all-case batch precommit complete");

    let execution_order = generated
        .public
        .cases
        .iter()
        .map(|case| case.vocabulary.case_id_sha256.clone())
        .collect::<Vec<_>>();
    let case_preverifications = prepared
        .iter()
        .map(|case| case.preverification.clone())
        .collect::<Vec<_>>();
    let batch = K2UncertaintyBatchPrecommitV2::seal(
        generated.public.experiment_id_sha256.clone(),
        generated
            .private
            .expected_denominator_commitment_sha256
            .clone(),
        &case_preverifications,
        execution_order.clone(),
    )
    .expect("all-case batch precommit");
    assert!(batch.dispatch_permitted);
    let one_probe = case_preverifications
        .iter()
        .filter(|case| {
            case.closure_plan
                .as_ref()
                .is_some_and(|plan| plan.plan_length == 1)
        })
        .count();
    let two_probe = case_preverifications
        .iter()
        .filter(|case| {
            case.closure_plan
                .as_ref()
                .is_some_and(|plan| plan.plan_length == 2)
        })
        .count();
    assert_eq!((one_probe, two_probe), (8, 8));
    let journal_root = environment.root.join("journal");
    let mut journal = K2UncertaintyBatchJournalV1::create(
        &journal_root,
        generated.public.experiment_id_sha256.clone(),
        execution_order,
    )
    .expect("create batch journal");
    let model_sets_root = composition_root_v1(
        &prepared
            .iter()
            .map(|case| {
                case.selection_preverification
                    .tournament
                    .case_id_sha256
                    .as_str()
            })
            .collect::<Vec<_>>(),
    )
    .expect("model sets payload root");
    let probe_sets_root = composition_root_v1(
        &prepared
            .iter()
            .map(|case| case.probe_artifacts.artifacts_root_sha256.as_str())
            .collect::<Vec<_>>(),
    )
    .expect("probe sets payload root");
    let selections_root = composition_root_v1(
        &prepared
            .iter()
            .map(|case| case.tournament.tournament.tournament_root_sha256.as_str())
            .collect::<Vec<_>>(),
    )
    .expect("selections payload root");
    for (kind, owner, request, payload) in [
        (
            K2UncertaintyBatchJournalEventKindV1::BatchFrozen,
            binaries.coordinator_sha256.clone(),
            root("batch-freeze-request"),
            root("batch-freeze-payload"),
        ),
        (
            K2UncertaintyBatchJournalEventKindV1::CasesGenerated,
            binaries.generator_sha256.clone(),
            generated.generator_request_root_sha256.clone(),
            generated.response_root_sha256.clone(),
        ),
        (
            K2UncertaintyBatchJournalEventKindV1::ModelSetsFrozen,
            binaries.learner_sha256.clone(),
            root("model-sets-request"),
            model_sets_root,
        ),
        (
            K2UncertaintyBatchJournalEventKindV1::ProbeSetsFrozen,
            binaries.probe_sha256.clone(),
            root("probe-sets-request"),
            probe_sets_root,
        ),
        (
            K2UncertaintyBatchJournalEventKindV1::SelectionsFrozen,
            binaries.selector_sha256.clone(),
            root("selections-request"),
            selections_root,
        ),
        (
            K2UncertaintyBatchJournalEventKindV1::AllCasesPrecommitted,
            binaries.preverifier_sha256.clone(),
            root("all-cases-precommit-request"),
            batch.batch_root_sha256.clone(),
        ),
    ] {
        journal
            .append(kind, None, owner, request, payload)
            .expect("append batch barrier event");
    }
    let reopened = K2UncertaintyBatchJournalV1::open_existing(
        &journal_root,
        generated.public.experiment_id_sha256.clone(),
    )
    .expect("reopen all-case barrier");
    assert_eq!(reopened.projection(), journal.projection());

    let mut final_receipts = Vec::new();
    for (case_sequence, prepared_case) in prepared.iter().enumerate() {
        eprintln!("R7 V4 execute case {}/16", case_sequence + 1);
        final_receipts.push(execute_precommitted_case(
            &environment,
            &binaries,
            &generated,
            &batch,
            prepared_case,
            &mut journal,
            case_sequence,
        ));
    }
    assert_eq!(final_receipts.len(), 16);
    assert_eq!(journal.projection().completed_cases, 16);
    assert_eq!(
        K2UncertaintyBatchJournalV1::open_existing(
            &journal_root,
            generated.public.experiment_id_sha256.clone(),
        )
        .expect("restart after all observations")
        .projection(),
        journal.projection()
    );
}

#[allow(clippy::too_many_arguments)]
fn execute_precommitted_case(
    environment: &TestEnvironment,
    binaries: &ProcessBinaries,
    generated: &K2UncertaintyGeneratorResponseV1,
    batch: &K2UncertaintyBatchPrecommitV2,
    prepared_case: &PreparedCase,
    journal: &mut K2UncertaintyBatchJournalV1,
    case_sequence: usize,
) -> K2UncertaintyCaseVerificationReceiptV2 {
    let public_case = &generated.public.cases[case_sequence];
    let private_case = generated
        .private
        .cases
        .iter()
        .find(|case| case.case_id_sha256 == public_case.vocabulary.case_id_sha256)
        .expect("private case");
    let plan = prepared_case
        .preverification
        .closure_plan
        .as_ref()
        .expect("dispatchable V4 plan");
    let planner = &prepared_case
        .preverification
        .closure_verification_request
        .planner_request;
    let mut workspace_paths = Vec::with_capacity(plan.plan_length as usize);
    let mut safety_bindings = Vec::with_capacity(plan.plan_length as usize);
    for (ordinal, probe_root) in plan.ordered_probe_roots_sha256.iter().enumerate() {
        let selected_probe = planner
            .representatives
            .iter()
            .find(|disposition| &disposition.probe.probe_root_sha256 == probe_root)
            .map(|disposition| disposition.probe.clone())
            .expect("planned complete-frontier probe");
        let resolved_effect = private_case
            .mapping
            .iter()
            .find(|entry| entry.opaque_action_root_sha256 == selected_probe.action_id_sha256)
            .map(|entry| entry.effect.clone())
            .expect("private planned effect");
        let workspace_identity = K2UncertaintyWorkspaceIdentityV2::seal(
            plan.case_id_sha256.clone(),
            plan.plan_root_sha256.clone(),
            ordinal as u64,
        )
        .expect("derived workspace identity");
        let workspace = environment.root.join(format!(
            "sandbox-work-{}",
            workspace_identity.identity_root_sha256
        ));
        assert!(!workspace.exists(), "workspace identity must be fresh");
        let safety_request = K2UncertaintySafetyRequestV1::seal(
            plan.plan_root_sha256.clone(),
            selected_probe,
            resolved_effect,
            public_case.vocabulary.clone(),
            self_formed_grammar_root_v1(&public_case.vocabulary).expect("grammar root"),
            composition_root_v1(&workspace.to_string_lossy().as_ref()).expect("sandbox root"),
            binaries.safety_sha256.clone(),
        )
        .expect("private plan safety request");
        let safety_receipt: K2UncertaintySafetyReceiptV1 =
            run_isolated(&binaries.safety, &safety_request, &[], 20);
        assert_eq!(
            safety_receipt.disposition,
            K2UncertaintyPrivateSafetyDispositionV1::Pass
        );
        workspace_paths.push(workspace);
        safety_bindings.push(K2UncertaintyPlanSafetyBindingV2 {
            request: safety_request,
            receipt: safety_receipt,
        });
    }
    assert_eq!(
        workspace_paths.iter().collect::<BTreeSet<_>>().len(),
        workspace_paths.len()
    );
    let dispatch = prepare_self_formed_plan_dispatch_v2(
        batch,
        &prepared_case.preverification,
        public_case,
        private_case,
        safety_bindings,
        &binaries.worker_sha256,
        &binaries.observer_sha256,
    )
    .expect("fail-closed plan dispatch preparation");
    for (item, workspace) in dispatch.items.iter().zip(&workspace_paths) {
        let expected_identity = K2UncertaintyWorkspaceIdentityV2::seal(
            plan.case_id_sha256.clone(),
            plan.plan_root_sha256.clone(),
            item.probe_ordinal,
        )
        .expect("workspace identity parity");
        assert_eq!(item.workspace_identity, expected_identity);
        assert_eq!(
            workspace,
            &environment.root.join(format!(
                "sandbox-work-{}",
                item.workspace_identity.identity_root_sha256
            ))
        );
    }
    let case_journal_root = environment
        .root
        .join(format!("case-journal-{case_sequence:02}"));
    let mut case_journal = K2UncertaintyCaseJournalV2::create(&case_journal_root, dispatch.clone())
        .expect("create V4 case journal");
    case_journal
        .record_plan_dispatch(
            binaries.coordinator_sha256.clone(),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("durably record whole plan dispatch");
    journal
        .append(
            K2UncertaintyBatchJournalEventKindV1::ProbeDispatched,
            Some(private_case.case_id_sha256.clone()),
            binaries.coordinator_sha256.clone(),
            batch.batch_root_sha256.clone(),
            dispatch.dispatch_root_sha256.clone(),
        )
        .expect("durable case-level plan dispatch");

    let mut executions = Vec::with_capacity(dispatch.items.len());
    for (item, workspace) in dispatch.items.iter().zip(&workspace_paths) {
        fs::create_dir_all(workspace).expect("create fresh sandbox workspace");
        for (relative_path, bytes) in
            materialize_self_formed_probe_files_v1(public_case, &item.selected_probe)
                .expect("materialize planned initial state")
        {
            let path = workspace.join(relative_path);
            fs::create_dir_all(path.parent().expect("workspace file parent"))
                .expect("create workspace parent");
            fs::write(path, bytes).expect("write workspace input");
        }
        let permit = case_journal
            .begin_probe_execution(item.probe_ordinal, K2UncertaintyCaseJournalFaultV2::None)
            .expect("begin precommitted probe execution");
        let worker_outcome: K2InquiryWorkerOutcomeV1 = run_isolated(
            &binaries.worker,
            &item.worker_request,
            &[Mount::ReadWrite(workspace, "/work")],
            20,
        );
        let observation: K2InquiryObservationReceiptV1 = run_isolated(
            &binaries.observer,
            &item.observer_request,
            &[Mount::ReadOnly(workspace, "/work")],
            20,
        );
        let mut resealed_worker = worker_outcome.clone();
        resealed_worker.reseal().expect("reseal worker outcome");
        assert_eq!(resealed_worker, worker_outcome);
        let mut resealed_observation = observation.clone();
        resealed_observation.reseal().expect("reseal observation");
        assert_eq!(resealed_observation, observation);
        assert_eq!(worker_outcome.post_manifest, observation.post_manifest);
        let evidence = K2UncertaintyProbeExecutionEvidenceV2::seal(
            dispatch.dispatch_root_sha256.clone(),
            item,
            worker_outcome,
            observation,
        )
        .expect("bind probe execution evidence");
        case_journal
            .record_probe_observation(
                permit,
                evidence.observation.receipt_root_sha256.clone(),
                K2UncertaintyCaseJournalFaultV2::None,
            )
            .expect("durably freeze planned observation");
        executions.push(evidence);
    }
    let observation_vector = K2UncertaintyObservationVectorV2::seal(&dispatch, executions)
        .expect("ordered observation vector");
    let vector_request_root = composition_root_v1(&(
        "nando.k2-self-formed-vector-freeze-request.v2",
        &dispatch.dispatch_root_sha256,
    ))
    .expect("vector request root");
    case_journal
        .freeze_observation_vector(
            binaries.coordinator_sha256.clone(),
            vector_request_root.clone(),
            observation_vector.vector_root_sha256.clone(),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("durably freeze observation vector");
    journal
        .append(
            K2UncertaintyBatchJournalEventKindV1::ProbeObserved,
            Some(private_case.case_id_sha256.clone()),
            binaries.observer_sha256.clone(),
            vector_request_root,
            observation_vector.vector_root_sha256.clone(),
        )
        .expect("durable case-level observation vector");
    let artifact_root = environment
        .root
        .join(format!("frontier-{case_sequence:02}"));
    let final_material = publish_self_formed_final_verifier_material_v2(
        &artifact_root,
        batch,
        &prepared_case.preverification,
    )
    .expect("publish root-addressed V4 verifier material");
    let final_request = K2UncertaintyFinalVerifierRequestV2::seal(
        binaries.final_verifier_v2_sha256.clone(),
        final_material,
        prepared_case.probe_request.clone(),
        prepared_case.probe_artifacts.clone(),
        private_case.clone(),
        dispatch.clone(),
        observation_vector.clone(),
        case_journal.state().clone(),
    )
    .expect("V4 final verifier request");
    let final_request_bytes = uncertainty_bytes_v1(&final_request).expect("final request bytes");
    assert!(final_request_bytes.len() <= K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1);
    eprintln!(
        "R7 V4 final verifier request bytes case {}: {}",
        case_sequence + 1,
        final_request_bytes.len()
    );
    let final_receipt: K2UncertaintyCaseVerificationReceiptV2 = run_isolated(
        &binaries.final_verifier_v2,
        &final_request,
        &[Mount::ReadOnly(&artifact_root, "/evidence")],
        120,
    );
    final_receipt.validate().expect("V4 final receipt");
    assert_eq!(
        final_receipt.selected_probe_executions,
        dispatch.closure_plan.plan_length
    );
    case_journal
        .record_case_terminal(
            binaries.final_verifier_v2_sha256.clone(),
            final_request.request_root_sha256.clone(),
            final_receipt.receipt_root_sha256.clone(),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("durably freeze case terminal");
    let outer_model_event_root = journal
        .append(
            K2UncertaintyBatchJournalEventKindV1::ModelsUpdated,
            Some(private_case.case_id_sha256.clone()),
            binaries.final_verifier_v2_sha256.clone(),
            final_request.request_root_sha256.clone(),
            final_receipt.receipt_root_sha256.clone(),
        )
        .expect("durable model update");
    case_journal
        .record_models_updated(
            binaries.coordinator_sha256.clone(),
            outer_model_event_root,
            final_receipt.receipt_root_sha256.clone(),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("bind outer model update");
    for workspace in &workspace_paths {
        fs::remove_dir_all(workspace).expect("remove observed disposable workspace");
        assert!(!workspace.exists());
    }
    let workspace_roots = dispatch
        .items
        .iter()
        .map(|item| item.workspace_identity.identity_root_sha256.as_str())
        .collect::<Vec<_>>();
    let cleanup_request_root = composition_root_v1(&(
        "nando.k2-self-formed-cleanup-request.v2",
        &dispatch.dispatch_root_sha256,
    ))
    .expect("cleanup request root");
    let cleanup_receipt_root = composition_root_v1(&(
        "nando.k2-self-formed-cleanup-receipt.v2",
        &dispatch.dispatch_root_sha256,
        workspace_roots,
        true,
    ))
    .expect("cleanup receipt root");
    case_journal
        .freeze_cleanup(
            binaries.coordinator_sha256.clone(),
            cleanup_request_root,
            cleanup_receipt_root,
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("freeze post-terminal cleanup");
    assert_eq!(
        K2UncertaintyCaseJournalV2::reopen(&case_journal_root)
            .expect("reopen completed case journal")
            .projection()
            .expect("completed case projection")
            .phase,
        K2UncertaintyCaseJournalPhaseV2::CleanupFrozen
    );
    final_receipt
}

struct PreparedCase {
    probe_request: K2UncertaintyProbeRequestV1,
    probe_artifacts: K2UncertaintyProbeArtifactsV1,
    tournament: K2UncertaintyTournamentArtifactsV1,
    selection_preverification: K2UncertaintyCasePreverificationV1,
    preverification: K2UncertaintyCasePreverificationV2,
}

struct ProcessBinaries {
    generator: PathBuf,
    learner: PathBuf,
    probe: PathBuf,
    selector: PathBuf,
    baseline: PathBuf,
    preverifier: PathBuf,
    closure_planner: PathBuf,
    closure_verifier: PathBuf,
    safety: PathBuf,
    worker: PathBuf,
    observer: PathBuf,
    final_verifier_v2: PathBuf,
    generator_sha256: String,
    learner_sha256: String,
    probe_sha256: String,
    selector_sha256: String,
    baseline_sha256: String,
    preverifier_sha256: String,
    closure_planner_sha256: String,
    closure_verifier_sha256: String,
    safety_sha256: String,
    worker_sha256: String,
    observer_sha256: String,
    final_verifier_v2_sha256: String,
    coordinator_sha256: String,
}

impl ProcessBinaries {
    fn from_cargo() -> Self {
        let generator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
        let learner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-learner"));
        let probe = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-probe"));
        let selector = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-selector"));
        let baseline = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-baseline"));
        let preverifier = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-verifier"));
        let closure_planner =
            PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-closure-planner"));
        let closure_verifier =
            PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-closure-verifier"));
        let safety = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-safety"));
        let worker = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-worker"));
        let observer = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-observer"));
        let final_verifier_v2 =
            PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-final-verifier-v2"));
        let coordinator = std::env::current_exe().expect("coordinator executable");
        Self {
            generator_sha256: sha(&generator),
            learner_sha256: sha(&learner),
            probe_sha256: sha(&probe),
            selector_sha256: sha(&selector),
            baseline_sha256: sha(&baseline),
            preverifier_sha256: sha(&preverifier),
            closure_planner_sha256: sha(&closure_planner),
            closure_verifier_sha256: sha(&closure_verifier),
            safety_sha256: sha(&safety),
            worker_sha256: sha(&worker),
            observer_sha256: sha(&observer),
            final_verifier_v2_sha256: sha(&final_verifier_v2),
            coordinator_sha256: sha(&coordinator),
            generator,
            learner,
            probe,
            selector,
            baseline,
            preverifier,
            closure_planner,
            closure_verifier,
            safety,
            worker,
            observer,
            final_verifier_v2,
        }
    }

    fn assert_pairwise_distinct(&self) {
        let roots = [
            &self.generator_sha256,
            &self.learner_sha256,
            &self.probe_sha256,
            &self.selector_sha256,
            &self.baseline_sha256,
            &self.preverifier_sha256,
            &self.closure_planner_sha256,
            &self.closure_verifier_sha256,
            &self.safety_sha256,
            &self.worker_sha256,
            &self.observer_sha256,
            &self.final_verifier_v2_sha256,
            &self.coordinator_sha256,
        ];
        assert_eq!(
            roots.iter().copied().collect::<BTreeSet<_>>().len(),
            roots.len()
        );
    }
}

enum Mount<'a> {
    ReadOnly(&'a Path, &'static str),
    ReadWrite(&'a Path, &'static str),
}

fn run_isolated<I, O>(executable: &Path, input: &I, mounts: &[Mount<'_>], cpu_seconds: u64) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let guest = "/nando/bin/process";
    let mut command = Command::new("/usr/bin/bwrap");
    command.args([
        "--unshare-all",
        "--die-with-parent",
        "--new-session",
        "--cap-drop",
        "ALL",
        "--clearenv",
    ]);
    for path in ["/usr", "/lib", "/lib64"] {
        if Path::new(path).exists() {
            command.args(["--ro-bind", path, path]);
        }
    }
    command
        .args(["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"])
        .args(["--dir", "/nando", "--dir", "/nando/bin"])
        .arg("--ro-bind")
        .arg(executable)
        .arg(guest);
    for mount in mounts {
        match mount {
            Mount::ReadOnly(host, guest) => {
                command.arg("--ro-bind").arg(host).arg(guest);
            }
            Mount::ReadWrite(host, guest) => {
                command.arg("--bind").arg(host).arg(guest);
            }
        }
    }
    command
        .args(["--setenv", "HOME", "/tmp", "--setenv", "LANG", "C"])
        .args(["--", "/usr/bin/prlimit"])
        .arg(format!("--cpu={cpu_seconds}:{cpu_seconds}"))
        .args(["--as=536870912:536870912", "--nproc=32:32"])
        .args(["--fsize=33554432:33554432", "--", guest])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    run_child(command, input, Duration::from_secs(cpu_seconds + 10))
}

fn run_process<I, O>(executable: &Path, input: &I) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let mut command = Command::new(executable);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    run_child(command, input, Duration::from_secs(120))
}

fn run_child<I, O>(mut command: Command, input: &I, timeout: Duration) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let mut child = command.spawn().expect("spawn owner process");
    child
        .stdin
        .take()
        .expect("owner stdin")
        .write_all(&uncertainty_bytes_v1(input).expect("owner request bytes"))
        .expect("write owner request");
    let mut stdout = child.stdout.take().expect("owner stdout");
    let mut stderr = child.stderr.take().expect("owner stderr");
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).expect("read owner stdout");
        bytes
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).expect("read owner stderr");
        bytes
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll owner process") {
            break status;
        }
        assert!(Instant::now() < deadline, "owner process timed out");
        thread::sleep(Duration::from_millis(5));
    };
    let stdout = stdout_reader.join().expect("join owner stdout");
    let stderr = stderr_reader.join().expect("join owner stderr");
    assert!(
        status.success(),
        "owner process failed: {}",
        String::from_utf8_lossy(&stderr)
    );
    uncertainty_decode_v1(&stdout).expect("canonical owner response")
}

struct TestEnvironment {
    root: PathBuf,
}

impl TestEnvironment {
    fn new() -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nando-k2-self-formed-r6-process-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create R6 process root");
        Self { root }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn sha(path: &Path) -> String {
    composition_sha256_file_v1(path).expect("executable hash")
}

fn root(label: &str) -> String {
    composition_root_v1(&("nando.k2-self-formed-r6-process-test.v1", label)).expect("test root")
}
