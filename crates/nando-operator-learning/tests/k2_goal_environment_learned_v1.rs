use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::*;

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
#[ignore = "requires Linux bwrap and learner, Law Lab worker, and exact oracle binaries"]
fn learned_goal_capability_is_isolated_durable_and_authority_free() {
    let fixture = LearnedFixtureV1::new();
    let learner_path = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-effect-learner"));
    let worker_path = PathBuf::from(env!("CARGO_BIN_EXE_nando-law-lab-sandbox-worker"));
    let oracle_path = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-exact-goal-oracle"));
    let selector_path = std::env::current_exe().expect("selector path");
    let learner_runner = K2EffectLearnerRunnerV1::new(learner_path);
    let learner_manifest = learner_runner
        .learner_manifest_v1()
        .expect("learner manifest");
    let worker_sha256 = law_lab_sha256_file_v1(&worker_path).expect("worker sha");
    let oracle_sha256 = law_lab_sha256_file_v1(&oracle_path).expect("oracle sha");
    let selector_sha256 = law_lab_sha256_file_v1(&selector_path).expect("selector sha");
    let identities = [
        learner_manifest.executable_sha256.as_str(),
        worker_sha256.as_str(),
        oracle_sha256.as_str(),
        selector_sha256.as_str(),
    ];
    assert_eq!(
        identities
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );

    let adapter =
        LawLabSandboxAdapterV1::new(LawLabSandboxConfigV1::generated_capability_self_test_v1(
            worker_path,
            worker_sha256.clone(),
            fixture.source_store.clone(),
            fixture.workspace_store.clone(),
        ));
    let executor = adapter.executor_manifest().expect("executor manifest");
    let oracle_manifest =
        K2ExactOracleManifestV1::seal(oracle_sha256.clone()).expect("oracle manifest");
    let budget = K2LearnedCapabilityBudgetV1::preregistered_v1();
    let harness_commitment_sha256 = root("learned-harness-commitment");
    let experiment_id_sha256 = root("learned-experiment-id");
    let deterministic_seed_sha256 = root("learned-deterministic-seed");
    let catalog = K2OpaqueActionCatalogV1::from_harness_commitment_v1(&harness_commitment_sha256)
        .expect("opaque catalog");
    let mapping =
        K2HiddenActionMappingV1::seal_fixture_v1(&catalog, catalog.action_ids_sha256[0].clone())
            .expect("hidden mapping");
    let plan = K2SupportProbePlanV1::seal(
        experiment_id_sha256.clone(),
        &catalog,
        &fixture.support,
        &mapping,
        &deterministic_seed_sha256,
    )
    .expect("support plan");
    let public_context = K2LearnerPublicContextV1::seal(
        &catalog,
        &fixture.support,
        &plan,
        &learner_manifest,
        &budget,
    )
    .expect("public context");
    let private_contract = K2PrivateExperimentContractV1::seal(
        experiment_id_sha256.clone(),
        harness_commitment_sha256,
        &public_context,
        &catalog,
        &fixture.support,
        mapping.clone(),
        fixture.target_pre.clone(),
        fixture.target_expected.clone(),
        fixture.target_goal_store_snapshot_root_sha256.clone(),
    )
    .expect("private contract");
    let private_path = fixture.private_store.join("experiment.json");
    let private_receipt = publish_private_experiment_contract_v1(&private_path, &private_contract)
        .expect("private artifact publication");
    let reopened_private = reopen_private_experiment_contract_v1(
        &private_path,
        &private_receipt,
        &public_context,
        &catalog,
        &fixture.support,
    )
    .expect("private artifact restart");
    assert_eq!(reopened_private, private_contract);

    let freeze = K2LearnedCapabilityFreezeV1::seal(K2LearnedCapabilityFreezeInputV1 {
        private_contract: &reopened_private,
        public_context: &public_context,
        catalog: &catalog,
        support: &fixture.support,
        plan: &plan,
        learner: &learner_manifest,
        budget: &budget,
        independent_verifier_contract_root_sha256: canonical_json_sha256(
            &K2_INDEPENDENT_EFFECT_VERIFIER_CONTRACT_V1,
        )
        .expect("verifier contract root"),
        selector_executable_sha256: selector_sha256.clone(),
        sandbox_executor_manifest_root_sha256: executor.manifest_root_sha256.clone(),
        sandbox_worker_sha256: worker_sha256.clone(),
        exact_oracle_manifest_root_sha256: oracle_manifest.manifest_root_sha256.clone(),
        exact_oracle_executable_sha256: oracle_sha256,
        deterministic_seed_sha256: deterministic_seed_sha256.clone(),
        frozen_at_unix_ms: 1_786_550_000_000,
    })
    .expect("learned freeze");
    assert_eq!(
        freeze.private_contract_artifact_root_sha256,
        private_receipt.artifact_root_sha256
    );
    let mut journal = K2LearnedCapabilityJournalV1::create(
        &fixture.learned_journal_store,
        experiment_id_sha256.clone(),
    )
    .expect("learned journal");
    journal.append_freeze(&freeze, 1).expect("freeze durable");

    let mut dispatches = Vec::new();
    let mut observations = Vec::new();
    let mut support_requests = Vec::new();
    let mut support_executions = Vec::new();
    for probe in &plan.ordered_probes {
        let world = fixture
            .support
            .world(&probe.support_world_root_sha256)
            .expect("support world");
        let hidden = mapping
            .entry(&probe.action_id_sha256)
            .expect("hidden action");
        let request = LawLabSandboxRequestV1::seal(LawLabSandboxRequestInputV1 {
            executor_manifest_root_sha256: executor.manifest_root_sha256.clone(),
            worker_sha256: worker_sha256.clone(),
            candidate_root_sha256: experiment_id_sha256.clone(),
            version_space_root_sha256: plan.plan_root_sha256.clone(),
            durable_prediction_ledger_root_sha256: freeze.freeze_root_sha256.clone(),
            probe_root_sha256: probe.probe_root_sha256.clone(),
            source_tree_root_sha256: world.source_manifest.tree_root_sha256.clone(),
            deterministic_seed_sha256: probe.deterministic_seed_sha256.clone(),
            domain: LawLabProbeDomainV1::Filesystem,
            purpose: LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest,
            surviving_hypothesis_count: 1,
            precommitted_prediction_count: 1,
            operations: vec![hidden.effect.operation_v1()],
        })
        .expect("support request");
        let dispatch = K2SupportDispatchV1::seal(&freeze, &plan, probe, world, &mapping, &request)
            .expect("support dispatch");
        journal
            .append_support_dispatch(&dispatch, 2 + probe.probe_ordinal * 2)
            .expect("support dispatch durable");
        let execution = adapter.execute(&request).expect("support bwrap execution");
        let observation =
            K2SupportObservationV1::seal(&public_context, world, &dispatch, &request, &execution)
                .expect("support redaction");
        journal
            .append_support_observation(&observation, 3 + probe.probe_ordinal * 2)
            .expect("support observation durable");
        dispatches.push(dispatch);
        observations.push(observation);
        support_requests.push(request);
        support_executions.push(execution);
    }
    assert_eq!(dispatches.len(), 6);
    assert_eq!(support_executions.len(), 6);
    assert!(support_executions.iter().all(|execution| {
        execution.receipt.cleanup.verified_absent
            && execution
                .worker_outcome
                .isolation
                .ipv4_non_loopback_route_entries
                == 0
            && execution
                .worker_outcome
                .isolation
                .ipv6_non_loopback_route_entries
                == 0
    }));
    let observation_set =
        K2SupportObservationSetV1::seal(&public_context, &plan, observations).expect("support set");
    journal
        .append_support_evidence(&observation_set, 14)
        .expect("support set durable");

    let learning_request = K2EffectLearningRequestV1::seal(
        public_context.clone(),
        catalog.clone(),
        observation_set.clone(),
    )
    .expect("learning request");
    assert_learner_request_has_no_private_fields(&learning_request);
    let (learning_outcome, learning_process) = learner_runner
        .run_v1(
            &learner_manifest,
            &K2EffectLearnerProtocolRequestV1::LearnEffects(learning_request.clone()),
        )
        .expect("external learner effects");
    let laws = match learning_outcome {
        K2EffectLearnerProtocolOutcomeV1::LearnedEffects(value) => value,
        K2EffectLearnerProtocolOutcomeV1::TargetPredictions(_) => {
            panic!("wrong learner outcome")
        }
        K2EffectLearnerProtocolOutcomeV1::GeneratedAblation(_) => {
            panic!("wrong ablation outcome")
        }
    };
    learning_process
        .validate_persisted_v1()
        .expect("learning process receipt");
    assert_eq!(laws.laws.len(), 2);
    journal.append_laws(&laws, 15).expect("laws durable");

    let independence = K2TargetIndependenceReceiptV1::verify(
        &fixture.support,
        &fixture.target_pre,
        &learning_request,
    )
    .expect("target holdout");
    journal
        .append_independence(&independence, 16)
        .expect("holdout durable");
    let prediction_request = K2TargetPredictionRequestV1::seal(
        &public_context,
        catalog.clone(),
        laws.clone(),
        fixture.target_pre.clone(),
    )
    .expect("target prediction request");
    let (prediction_outcome, prediction_process) = learner_runner
        .run_v1(
            &learner_manifest,
            &K2EffectLearnerProtocolRequestV1::PredictTarget(prediction_request.clone()),
        )
        .expect("external learner predictions");
    let learned_predictions = match prediction_outcome {
        K2EffectLearnerProtocolOutcomeV1::TargetPredictions(value) => value,
        K2EffectLearnerProtocolOutcomeV1::LearnedEffects(_) => {
            panic!("wrong prediction outcome")
        }
        K2EffectLearnerProtocolOutcomeV1::GeneratedAblation(_) => {
            panic!("wrong ablation outcome")
        }
    };
    prediction_process
        .validate_persisted_v1()
        .expect("prediction process receipt");
    journal
        .append_predictions(&learned_predictions, 17)
        .expect("target predictions durable");
    let verification = K2LearnedEffectVerificationReceiptV1::verify(
        &freeze,
        &learning_request,
        &laws,
        &prediction_request,
        &learned_predictions,
    )
    .expect("independent verification");
    journal
        .append_verification(&verification, 18)
        .expect("verification durable");
    let (learned_binding, v1_actions) = K2LearnedToV1BindingV1::build(
        &freeze,
        &catalog,
        &mapping,
        &laws,
        &learned_predictions,
        &verification,
    )
    .expect("learned to V1 binding");
    journal
        .append_v1_binding(&learned_binding, 19)
        .expect("V1 binding durable");

    let goal = K2GoalEnvelopeV1::seal(
        K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
        fixture.target_pre.tree_root_sha256.clone(),
        fixture.target_expected.tree_root_sha256.clone(),
        fixture.target_goal_store_snapshot_root_sha256.clone(),
        root("learned-target-constraints"),
        oracle_manifest.manifest_root_sha256.clone(),
        1_786_550_000_100,
    )
    .expect("target goal after prediction durability");
    let vocabulary = K2K1VocabularySnapshotV1::seal(
        K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
        None,
        None,
        v1_actions,
        1_786_550_000_101,
    )
    .expect("V1 vocabulary");
    let alternatives =
        K2AlternativeSetV1::seal(&vocabulary, fixture.target_pre.tree_root_sha256.clone())
            .expect("V1 alternatives");
    let v1_budget = K2GoalEnvironmentBudgetV1::preregistered_v1();
    let v1_freeze = K2DecisionFreezeV1::seal(K2DecisionFreezeInputV1 {
        episode_id_sha256: root("learned-v1-target-episode"),
        goal: &goal,
        vocabulary: &vocabulary,
        alternatives: &alternatives,
        budget: v1_budget,
        selector_contract_root_sha256: root("learned-v1-selector-contract"),
        selector_executable_sha256: selector_sha256.clone(),
        oracle_manifest: &oracle_manifest,
        sandbox_worker_sha256: worker_sha256.clone(),
        deterministic_seed_sha256: deterministic_seed_sha256.clone(),
        observed_registry_revision: None,
        observed_registry_root_sha256: None,
        frozen_at_unix_ms: 1_786_550_000_102,
    })
    .expect("V1 decision freeze");
    let v1_predictions = K2AlternativePredictionSetV1::prepared_capability_v1(
        &v1_freeze,
        &goal,
        &vocabulary,
        &alternatives,
        &v1_budget,
        &oracle_manifest,
    )
    .expect("V1 prediction precommit");
    let selection =
        K2PreparedSelectionReceiptV1::select(&v1_freeze, &v1_predictions).expect("selection");
    let selected_learned = learned_binding
        .entry_for_v1_action(&selection.selected_action_root_sha256)
        .expect("selected learned binding");
    let selected_hidden = mapping
        .entry(&selected_learned.opaque_action_id_sha256)
        .expect("selected hidden operation");
    let target_request = LawLabSandboxRequestV1::seal(LawLabSandboxRequestInputV1 {
        executor_manifest_root_sha256: executor.manifest_root_sha256.clone(),
        worker_sha256: executor.worker_sha256.clone(),
        candidate_root_sha256: v1_freeze.episode_id_sha256.clone(),
        version_space_root_sha256: alternatives.alternative_set_root_sha256.clone(),
        durable_prediction_ledger_root_sha256: v1_predictions.prediction_set_root_sha256.clone(),
        probe_root_sha256: selection.selection_root_sha256.clone(),
        source_tree_root_sha256: fixture.target_pre.tree_root_sha256.clone(),
        deterministic_seed_sha256: v1_freeze.deterministic_seed_sha256.clone(),
        domain: LawLabProbeDomainV1::Filesystem,
        purpose: LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest,
        surviving_hypothesis_count: alternatives.alternatives.len() as u64,
        precommitted_prediction_count: v1_predictions.predictions.len() as u64,
        operations: vec![selected_hidden.effect.operation_v1()],
    })
    .expect("target request");
    let v1_law_lab_binding = K2LawLabBindingV1::seal(K2LawLabBindingInputV1 {
        freeze: &v1_freeze,
        goal: &goal,
        vocabulary: &vocabulary,
        alternatives: &alternatives,
        predictions: &v1_predictions,
        selection: &selection,
        request: &target_request,
    })
    .expect("target Law Lab binding");
    let mut v1_journal = K2EpisodeJournalV1::create(
        &fixture.v1_journal_store,
        v1_freeze.episode_id_sha256.clone(),
    )
    .expect("V1 journal");
    v1_journal
        .append(K2EpisodeEventKindV1::ContractFrozen, &v1_freeze, 1)
        .expect("V1 contract durable");
    v1_journal
        .append(
            K2EpisodeEventKindV1::PredictionsPrecommitted,
            &v1_predictions,
            2,
        )
        .expect("V1 predictions durable");
    v1_journal
        .append(K2EpisodeEventKindV1::ProbePlanned, &v1_law_lab_binding, 3)
        .expect("V1 plan durable");
    v1_journal
        .append(
            K2EpisodeEventKindV1::ProbeDispatched,
            &v1_law_lab_binding.binding_root_sha256,
            4,
        )
        .expect("V1 dispatch durable");
    let target_execution = adapter
        .execute(&target_request)
        .expect("target bwrap execution");
    v1_journal
        .append(
            K2EpisodeEventKindV1::ProbeExecuted,
            &target_execution.receipt,
            5,
        )
        .expect("V1 execution durable");
    let oracle_request = K2ExactOracleRequestV1::seal(
        &goal,
        &v1_freeze,
        &v1_law_lab_binding,
        &target_execution,
        &oracle_manifest,
    )
    .expect("oracle request");
    let oracle_outcome = execute_exact_oracle(&oracle_path, &oracle_request);
    let exact_goal = K2ExactGoalReceiptV1::evaluate(K2ExactGoalEvaluationInputV1 {
        freeze: &v1_freeze,
        goal: &goal,
        vocabulary: &vocabulary,
        alternatives: &alternatives,
        predictions: &v1_predictions,
        selection: &selection,
        binding: &v1_law_lab_binding,
        request: &target_request,
        execution: &target_execution,
        oracle_manifest: &oracle_manifest,
        oracle_request: &oracle_request,
        oracle_outcome: &oracle_outcome,
    })
    .expect("exact target goal");
    assert!(exact_goal.goal_satisfied);
    v1_journal
        .append(K2EpisodeEventKindV1::OutcomeVerified, &exact_goal, 6)
        .expect("V1 outcome durable");
    let v1_outcome = K2DecisionOutcomeReceiptV1::capability_pass(
        &v1_freeze,
        &v1_predictions,
        &v1_law_lab_binding,
        &target_execution,
        &exact_goal,
    )
    .expect("V1 outcome");
    v1_journal
        .append(K2EpisodeEventKindV1::Terminal, &v1_outcome, 7)
        .expect("V1 terminal durable");
    let v1_seal = v1_journal
        .derive_terminal_seal(&v1_outcome)
        .expect("V1 seal");
    let reopened_v1 = K2EpisodeJournalV1::open_existing(
        &fixture.v1_journal_store,
        v1_freeze.episode_id_sha256.clone(),
    )
    .expect("V1 restart");
    assert_eq!(reopened_v1.projection(), v1_journal.projection());
    assert_eq!(
        reopened_v1
            .derive_terminal_seal(&v1_outcome)
            .expect("reopened V1 seal"),
        v1_seal
    );
    let v1_evidence = K2V1EpisodeEvidenceV1::seal(K2V1EpisodeEvidenceInputV1 {
        learned_binding: &learned_binding,
        decision_freeze: &v1_freeze,
        predictions: &v1_predictions,
        selection: &selection,
        law_lab_binding: &v1_law_lab_binding,
        execution: &target_execution,
        exact_goal: &exact_goal,
        outcome: &v1_outcome,
        episode_seal: &v1_seal,
    })
    .expect("V1 episode evidence");
    journal
        .append_v1_episode(&v1_evidence, 20)
        .expect("V1 episode evidence durable");

    let ablation_controls = run_preregistered_ablation_controls(AblationContextV1 {
        fixture: &fixture,
        learning_request: &learning_request,
        observations: &observation_set,
        catalog: &catalog,
        mapping: &mapping,
        learner_runner: &learner_runner,
        learner_manifest: &learner_manifest,
        freeze: &freeze,
        learned_predictions: &learned_predictions,
        learned_binding: &learned_binding,
        positive_selection: &selection,
        journal: &journal,
        adapter: &adapter,
        goal: &goal,
        oracle_path: &oracle_path,
        oracle_manifest: &oracle_manifest,
        selector_sha256: &selector_sha256,
        worker_sha256: &worker_sha256,
        deterministic_seed_sha256: &deterministic_seed_sha256,
    });
    let ablations =
        K2LearnedAblationReceiptV1::seal(&freeze, ablation_controls).expect("ablation receipt");
    journal
        .append_ablations(&ablations, 21)
        .expect("ablations durable");
    let learned_outcome =
        K2LearnedCapabilityOutcomeV1::capability_pass(K2LearnedCapabilityOutcomeInputV1 {
            freeze: &freeze,
            dispatches: &dispatches,
            observations: &observation_set,
            learning_request: &learning_request,
            laws: &laws,
            independence: &independence,
            prediction_request: &prediction_request,
            predictions: &learned_predictions,
            verification: &verification,
            v1_binding: &learned_binding,
            v1_episode: &v1_evidence,
            ablations: &ablations,
        })
        .expect("learned terminal outcome");
    let terminal_event = journal
        .append_terminal(&learned_outcome, 22)
        .expect("learned terminal durable");
    let learned_seal = journal
        .derive_terminal_seal(&learned_outcome)
        .expect("learned terminal seal");
    assert_eq!(journal.events().len(), 22);
    assert_eq!(
        journal.projection().state,
        K2LearnedCapabilityStateV1::Terminal
    );
    assert_ne!(
        learned_seal.seal_root_sha256,
        terminal_event.entry_root_sha256
    );
    let reopened = K2LearnedCapabilityJournalV1::open_existing(
        &fixture.learned_journal_store,
        experiment_id_sha256,
    )
    .expect("learned journal restart");
    assert_eq!(reopened.projection(), journal.projection());
    assert_eq!(
        reopened
            .derive_terminal_seal(&learned_outcome)
            .expect("reopened learned seal"),
        learned_seal
    );
    run_journal_contract_assertions(
        &fixture,
        &freeze,
        &dispatches[0],
        &observation_set.observations[0],
        &journal,
        &learned_outcome,
        &learned_seal,
    );
    assert!(workspace_is_empty(&fixture.workspace_store));
    eprintln!(
        "{} outcome={} seal={} laws={} predictions={} v1_outcome={} v1_seal={} ablations={} ablation_learner={} ablation_sandbox={} ablation_oracle={} events=22 authority=false",
        K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PASS_V1,
        learned_outcome.outcome_root_sha256,
        learned_seal.seal_root_sha256,
        laws.law_set_root_sha256,
        learned_predictions.prediction_set_root_sha256,
        v1_outcome.outcome_root_sha256,
        v1_seal.seal_root_sha256,
        ablations.receipt_root_sha256,
        ablations.learner_processes,
        ablations.sandbox_probes,
        ablations.oracle_invocations,
    );
}

fn assert_learner_request_has_no_private_fields(request: &K2EffectLearningRequestV1) {
    let text = String::from_utf8(request.canonical_bytes_v1().expect("learning bytes"))
        .expect("UTF-8 learning request");
    for forbidden in [
        "hidden_action_mapping",
        "operation_plan_root_sha256",
        "target_expected_goal_manifest",
        "target_goal_store_snapshot_root_sha256",
        "goal_envelope_root_sha256",
        "selector_executable_sha256",
    ] {
        assert!(
            !text.contains(forbidden),
            "private field leaked: {forbidden}"
        );
    }
}

struct AblationContextV1<'a> {
    fixture: &'a LearnedFixtureV1,
    learning_request: &'a K2EffectLearningRequestV1,
    observations: &'a K2SupportObservationSetV1,
    catalog: &'a K2OpaqueActionCatalogV1,
    mapping: &'a K2HiddenActionMappingV1,
    learner_runner: &'a K2EffectLearnerRunnerV1,
    learner_manifest: &'a K2EffectLearnerManifestV1,
    freeze: &'a K2LearnedCapabilityFreezeV1,
    learned_predictions: &'a K2LearnedTargetPredictionSetV1,
    learned_binding: &'a K2LearnedToV1BindingV1,
    positive_selection: &'a K2PreparedSelectionReceiptV1,
    journal: &'a K2LearnedCapabilityJournalV1,
    adapter: &'a LawLabSandboxAdapterV1,
    goal: &'a K2GoalEnvelopeV1,
    oracle_path: &'a Path,
    oracle_manifest: &'a K2ExactOracleManifestV1,
    selector_sha256: &'a str,
    worker_sha256: &'a str,
    deterministic_seed_sha256: &'a str,
}

fn run_preregistered_ablation_controls(
    context: AblationContextV1<'_>,
) -> Vec<K2LearnedAblationControlV1> {
    use K2LearnedAblationKindV1 as Kind;
    use K2LearnedAblationVerdictV1 as Verdict;

    let copy_action_id = context
        .mapping
        .entries
        .iter()
        .find(|entry| matches!(&entry.effect, K2LearnedEffectLawBodyV1::CopyFile { .. }))
        .expect("copy action")
        .action_id_sha256
        .clone();
    let last_world = &context.fixture.support.worlds[2].world_root_sha256;
    let support_count_observations = context
        .observations
        .observations
        .iter()
        .filter(|observation| observation.support_world_root_sha256 != *last_world)
        .map(|observation| {
            K2GeneratedAblationObservationV1::unchanged_from_support_v1(
                observation,
                observation.action_id_sha256.clone(),
            )
            .expect("support-count observation")
        })
        .collect();
    let support_count_request = K2GeneratedAblationRequestV1::seal(
        context.learning_request.clone(),
        context.catalog.clone(),
        support_count_observations,
    )
    .expect("support-count request");
    let mut controls = Vec::with_capacity(13);
    let (control, _) = execute_generated_ablation_control(
        &context,
        Kind::SupportCount,
        Verdict::InsufficientSupport,
        &support_count_request,
    );
    controls.push(control);

    let shuffled_observations = context
        .observations
        .observations
        .iter()
        .map(|observation| {
            let action_id = if observation.support_world_root_sha256 == *last_world {
                context
                    .catalog
                    .action_ids_sha256
                    .iter()
                    .find(|action_id| **action_id != observation.action_id_sha256)
                    .expect("opposite action")
                    .clone()
            } else {
                observation.action_id_sha256.clone()
            };
            K2GeneratedAblationObservationV1::unchanged_from_support_v1(observation, action_id)
                .expect("shuffled observation")
        })
        .collect();
    let shuffled_request = K2GeneratedAblationRequestV1::seal(
        context.learning_request.clone(),
        context.catalog.clone(),
        shuffled_observations,
    )
    .expect("shuffled request");
    let (control, _) = execute_generated_ablation_control(
        &context,
        Kind::ActionIdentityShuffle,
        Verdict::NonTransferableDelta,
        &shuffled_request,
    );
    controls.push(control);

    let ambiguous_observations = context
        .observations
        .observations
        .iter()
        .map(|observation| {
            if observation.action_id_sha256 == copy_action_id {
                K2GeneratedAblationObservationV1::ambiguous_copy_source_from_support_v1(
                    observation,
                    observation.action_id_sha256.clone(),
                )
                .expect("ambiguous observation")
            } else {
                K2GeneratedAblationObservationV1::unchanged_from_support_v1(
                    observation,
                    observation.action_id_sha256.clone(),
                )
                .expect("unchanged remove observation")
            }
        })
        .collect();
    let ambiguous_request = K2GeneratedAblationRequestV1::seal(
        context.learning_request.clone(),
        context.catalog.clone(),
        ambiguous_observations,
    )
    .expect("ambiguous request");
    let (control, _) = execute_generated_ablation_control(
        &context,
        Kind::AmbiguousCopySource,
        Verdict::AmbiguousSourceMatch,
        &ambiguous_request,
    );
    controls.push(control);

    let copy_observations = context
        .observations
        .observations
        .iter()
        .filter(|observation| observation.action_id_sha256 == copy_action_id)
        .collect::<Vec<_>>();
    let constant_source_root = &copy_observations[0].observation_root_sha256;
    let constant_observations = context
        .observations
        .observations
        .iter()
        .map(|observation| {
            if observation.observation_root_sha256 == *constant_source_root {
                K2GeneratedAblationObservationV1::constant_output_from_support_v1(
                    observation,
                    copy_observations[1],
                    observation.action_id_sha256.clone(),
                )
                .expect("constant-output observation")
            } else {
                K2GeneratedAblationObservationV1::unchanged_from_support_v1(
                    observation,
                    observation.action_id_sha256.clone(),
                )
                .expect("constant unchanged observation")
            }
        })
        .collect();
    let constant_request = K2GeneratedAblationRequestV1::seal(
        context.learning_request.clone(),
        context.catalog.clone(),
        constant_observations,
    )
    .expect("constant-output request");
    let (control, _) = execute_generated_ablation_control(
        &context,
        Kind::ConstantOutput,
        Verdict::NonTransferableDelta,
        &constant_request,
    );
    controls.push(control);

    let outcome_dependent_observations = context
        .observations
        .observations
        .iter()
        .map(|observation| {
            K2GeneratedAblationObservationV1::outcome_equals_pre_from_support_v1(
                observation,
                observation.action_id_sha256.clone(),
            )
            .expect("outcome-dependence observation")
        })
        .collect();
    let outcome_dependent_request = K2GeneratedAblationRequestV1::seal(
        context.learning_request.clone(),
        context.catalog.clone(),
        outcome_dependent_observations,
    )
    .expect("outcome-dependence request");
    let (control, _) = execute_generated_ablation_control(
        &context,
        Kind::OutcomeDependence,
        Verdict::NonTransferableDelta,
        &outcome_dependent_request,
    );
    controls.push(control);

    let second_catalog = K2OpaqueActionCatalogV1::from_harness_commitment_v1(&root(
        "learned-second-harness-commitment",
    ))
    .expect("dynamic catalog");
    assert!(
        second_catalog
            .action_ids_sha256
            .iter()
            .all(|action_id| !context.catalog.action_ids_sha256.contains(action_id))
    );
    let original_copy_index = context
        .catalog
        .action_ids_sha256
        .iter()
        .position(|action_id| action_id == &copy_action_id)
        .expect("copy index");
    let second_mapping = K2HiddenActionMappingV1::seal_fixture_v1(
        &second_catalog,
        second_catalog.action_ids_sha256[original_copy_index].clone(),
    )
    .expect("dynamic mapping");
    let dynamic_observations = context
        .observations
        .observations
        .iter()
        .map(|observation| {
            let original_effect = &context
                .mapping
                .entry(&observation.action_id_sha256)
                .expect("original effect")
                .effect;
            let dynamic_action_id = second_mapping
                .entries
                .iter()
                .find(|entry| &entry.effect == original_effect)
                .expect("dynamic effect")
                .action_id_sha256
                .clone();
            K2GeneratedAblationObservationV1::unchanged_from_support_v1(
                observation,
                dynamic_action_id,
            )
            .expect("dynamic observation")
        })
        .collect();
    let dynamic_request = K2GeneratedAblationRequestV1::seal(
        context.learning_request.clone(),
        second_catalog.clone(),
        dynamic_observations,
    )
    .expect("dynamic request");
    let (control, dynamic_outcome) = execute_generated_ablation_control(
        &context,
        Kind::DynamicId,
        Verdict::TransferableWithDynamicIds,
        &dynamic_request,
    );
    assert_eq!(
        dynamic_outcome
            .learned_effects
            .iter()
            .map(|value| value.action_id_sha256.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        second_catalog
            .action_ids_sha256
            .iter()
            .map(String::as_str)
            .collect()
    );
    assert_eq!(
        dynamic_outcome
            .learned_effects
            .iter()
            .map(|value| value.effect.clone())
            .collect::<std::collections::BTreeSet<_>>(),
        context
            .mapping
            .entries
            .iter()
            .map(|entry| entry.effect.clone())
            .collect()
    );
    controls.push(control);

    let alias_result = K2TargetIndependenceReceiptV1::verify(
        &context.fixture.support,
        &context.fixture.support.worlds[0].source_manifest,
        context.learning_request,
    );
    let alias_reason = expect_invalid_reason(alias_result, "k2_target_not_independent");
    controls.push(seal_local_control(
        Kind::HoldoutAlias,
        Verdict::TargetNotIndependent,
        context.fixture.support.worlds[0]
            .source_manifest
            .tree_root_sha256
            .clone(),
        &alias_reason,
    ));

    let mut provenance_mismatch = support_count_request.clone();
    provenance_mismatch.observations[0].provenance =
        K2GeneratedAblationProvenanceV1::GeneratedCapabilitySelfTest;
    let provenance_input_root =
        canonical_json_sha256(&provenance_mismatch).expect("provenance mismatch input root");
    let provenance_reason = expect_invalid_reason(
        provenance_mismatch.validate(),
        "k2_support_evidence_invalid",
    );
    controls.push(seal_local_control(
        Kind::SupportProvenanceMismatch,
        Verdict::SupportEvidenceInvalid,
        provenance_input_root,
        &provenance_reason,
    ));

    let mut leaked = serde_json::to_value(K2EffectLearnerProtocolRequestV1::LearnEffects(
        context.learning_request.clone(),
    ))
    .expect("protocol value");
    leaked.as_object_mut().expect("protocol object").insert(
        "target_goal_root_sha256".to_owned(),
        serde_json::json!(root("leak")),
    );
    let leaked_bytes = nando_operator_kernel::canonical_json_bytes(&leaked).expect("leaked bytes");
    let leakage_input_root = canonical_json_sha256(&leaked_bytes).expect("leakage input root");
    let leakage_reason = expect_invalid_reason(
        K2EffectLearnerProtocolRequestV1::from_canonical_bytes_v1(&leaked_bytes),
        "k2_effect_learner_protocol_request_invalid",
    );
    controls.push(seal_local_control(
        Kind::TargetGoalLeakage,
        Verdict::LearnerRequestPrivateFieldRejected,
        leakage_input_root,
        &leakage_reason,
    ));

    let mut tampered_predictions = context.learned_predictions.clone();
    tampered_predictions.predictions[0]
        .predicted_terminal_manifest
        .tree_root_sha256 = root("tampered-target");
    let prediction_input_root =
        canonical_json_sha256(&tampered_predictions).expect("prediction tamper input root");
    let prediction_reason = expect_invalid_reason(
        verify_target_prediction_replay_v1(context.learned_predictions, &tampered_predictions),
        "k2_target_prediction_root_mismatch",
    );
    controls.push(seal_local_control(
        Kind::PredictionTamper,
        Verdict::TargetPredictionRootMismatch,
        prediction_input_root,
        &prediction_reason,
    ));

    controls.push(run_wrong_action_control(&context));

    let replay_experiment_id = root("cross-experiment-replay-target");
    let replay_reason = expect_invalid_reason(
        K2LearnedCapabilityProjectionV1::project(
            &replay_experiment_id,
            &context.journal.events()[..1],
        ),
        "k2_cross_experiment_replay",
    );
    controls.push(seal_local_control(
        Kind::CrossExperimentReplay,
        Verdict::CrossExperimentReplay,
        context.journal.events()[0].entry_root_sha256.clone(),
        &replay_reason,
    ));

    let mut authority_tamper = context.freeze.clone();
    authority_tamper.authority.k2_claim_granted = true;
    let authority_input_root =
        canonical_json_sha256(&authority_tamper).expect("authority tamper input root");
    let authority_reason = expect_invalid_reason(
        authority_tamper.validate_persisted_v1(),
        "k2_authority_boundary_violated",
    );
    controls.push(seal_local_control(
        Kind::AuthorityTamper,
        Verdict::AuthorityBoundaryViolated,
        authority_input_root,
        &authority_reason,
    ));

    assert_eq!(controls.len(), 13);
    controls
}

fn execute_generated_ablation_control(
    context: &AblationContextV1<'_>,
    kind: K2LearnedAblationKindV1,
    expected: K2LearnedAblationVerdictV1,
    request: &K2GeneratedAblationRequestV1,
) -> (K2LearnedAblationControlV1, K2GeneratedAblationOutcomeV1) {
    let (protocol_outcome, process) = context
        .learner_runner
        .run_v1(
            context.learner_manifest,
            &K2EffectLearnerProtocolRequestV1::EvaluateGeneratedAblation(request.clone()),
        )
        .expect("external ablation learner");
    process
        .validate_persisted_v1()
        .expect("ablation process receipt");
    let outcome = match protocol_outcome {
        K2EffectLearnerProtocolOutcomeV1::GeneratedAblation(value) => value,
        _ => panic!("wrong external ablation outcome"),
    };
    assert_eq!(outcome.observed_verdict, expected);
    let control = K2LearnedAblationControlV1::seal(
        kind,
        request.request_root_sha256.clone(),
        expected,
        outcome.observed_verdict,
        1,
        0,
        0,
        outcome.outcome_root_sha256.clone(),
    )
    .expect("external ablation control");
    (control, outcome)
}

fn seal_local_control(
    kind: K2LearnedAblationKindV1,
    verdict: K2LearnedAblationVerdictV1,
    input_root_sha256: String,
    observed_reason: &str,
) -> K2LearnedAblationControlV1 {
    let outcome_root_sha256 = canonical_json_sha256(&(
        "nando.k2-local-ablation-evidence.v1",
        kind,
        input_root_sha256.as_str(),
        observed_reason,
    ))
    .expect("local ablation outcome root");
    K2LearnedAblationControlV1::seal(
        kind,
        input_root_sha256,
        verdict,
        verdict,
        0,
        0,
        0,
        outcome_root_sha256,
    )
    .expect("local ablation control")
}

fn expect_invalid_reason<T>(
    result: K2GoalEnvironmentResultV1<T>,
    expected: &'static str,
) -> String {
    match result {
        Err(K2GoalEnvironmentErrorV1::Invalid(reason)) => {
            assert_eq!(reason, expected);
            reason.to_owned()
        }
        Err(error) => panic!("expected {expected}, got {error}"),
        Ok(_) => panic!("expected {expected}, got success"),
    }
}

fn run_wrong_action_control(context: &AblationContextV1<'_>) -> K2LearnedAblationControlV1 {
    let selected_learned = context
        .learned_binding
        .entry_for_v1_action(&context.positive_selection.selected_action_root_sha256)
        .expect("selected learned action");
    let wrong_hidden = context
        .mapping
        .entries
        .iter()
        .find(|entry| entry.action_id_sha256 != selected_learned.opaque_action_id_sha256)
        .expect("non-selected hidden action");
    let decoy_hidden = context
        .mapping
        .entry(&selected_learned.opaque_action_id_sha256)
        .expect("selected hidden action");
    let environment_root = context.fixture.target_pre.tree_root_sha256.clone();
    let wrong_action = K2K1ActionRefV1::seal(K2K1ActionRefInputV1 {
        provenance: K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
        applicability_environment_root_sha256: environment_root.clone(),
        applicability_receipt_root_sha256: root("wrong-action-applicability"),
        operation_plan_root_sha256: wrong_hidden.operation_plan_root_sha256.clone(),
        predicted_consequence_root_sha256: context.goal.expected_terminal_tree_root_sha256.clone(),
        fixture_effect_root_sha256: Some(root("wrong-action-effect")),
        law_certificate_root_sha256: None,
        epistemic_registry_member_root_sha256: None,
        bundle_v4_root_sha256: None,
        execution_certificate_root_sha256: None,
        applicability_guard_root_sha256: None,
        effect_contract_root_sha256: None,
        semantic_class_root_sha256: None,
        role_topology_root_sha256: None,
    })
    .expect("wrong fixture action");
    let decoy_action = K2K1ActionRefV1::seal(K2K1ActionRefInputV1 {
        provenance: K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
        applicability_environment_root_sha256: environment_root.clone(),
        applicability_receipt_root_sha256: root("wrong-action-decoy-applicability"),
        operation_plan_root_sha256: decoy_hidden.operation_plan_root_sha256.clone(),
        predicted_consequence_root_sha256: context.fixture.target_pre.tree_root_sha256.clone(),
        fixture_effect_root_sha256: Some(root("wrong-action-decoy-effect")),
        law_certificate_root_sha256: None,
        epistemic_registry_member_root_sha256: None,
        bundle_v4_root_sha256: None,
        execution_certificate_root_sha256: None,
        applicability_guard_root_sha256: None,
        effect_contract_root_sha256: None,
        semantic_class_root_sha256: None,
        role_topology_root_sha256: None,
    })
    .expect("wrong fixture decoy");
    let vocabulary = K2K1VocabularySnapshotV1::seal(
        K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
        None,
        None,
        vec![wrong_action, decoy_action],
        1_786_550_000_200,
    )
    .expect("wrong-action vocabulary");
    let alternatives =
        K2AlternativeSetV1::seal(&vocabulary, environment_root).expect("wrong-action alternatives");
    let budget = K2GoalEnvironmentBudgetV1::preregistered_v1();
    let freeze = K2DecisionFreezeV1::seal(K2DecisionFreezeInputV1 {
        episode_id_sha256: root("wrong-action-episode"),
        goal: context.goal,
        vocabulary: &vocabulary,
        alternatives: &alternatives,
        budget,
        selector_contract_root_sha256: root("wrong-action-selector-contract"),
        selector_executable_sha256: context.selector_sha256.to_owned(),
        oracle_manifest: context.oracle_manifest,
        sandbox_worker_sha256: context.worker_sha256.to_owned(),
        deterministic_seed_sha256: context.deterministic_seed_sha256.to_owned(),
        observed_registry_revision: None,
        observed_registry_root_sha256: None,
        frozen_at_unix_ms: 1_786_550_000_201,
    })
    .expect("wrong-action freeze");
    let predictions = K2AlternativePredictionSetV1::prepared_capability_v1(
        &freeze,
        context.goal,
        &vocabulary,
        &alternatives,
        &budget,
        context.oracle_manifest,
    )
    .expect("wrong-action predictions");
    let selection = K2PreparedSelectionReceiptV1::select(&freeze, &predictions)
        .expect("wrong-action selection");
    let selected = vocabulary
        .action(&selection.selected_action_root_sha256)
        .expect("wrong selected action");
    assert_eq!(
        selected.operation_plan_root_sha256,
        wrong_hidden.operation_plan_root_sha256
    );
    let executor = context.adapter.executor_manifest().expect("wrong executor");
    let request = LawLabSandboxRequestV1::seal(LawLabSandboxRequestInputV1 {
        executor_manifest_root_sha256: executor.manifest_root_sha256.clone(),
        worker_sha256: executor.worker_sha256.clone(),
        candidate_root_sha256: freeze.episode_id_sha256.clone(),
        version_space_root_sha256: alternatives.alternative_set_root_sha256.clone(),
        durable_prediction_ledger_root_sha256: predictions.prediction_set_root_sha256.clone(),
        probe_root_sha256: selection.selection_root_sha256.clone(),
        source_tree_root_sha256: context.fixture.target_pre.tree_root_sha256.clone(),
        deterministic_seed_sha256: freeze.deterministic_seed_sha256.clone(),
        domain: LawLabProbeDomainV1::Filesystem,
        purpose: LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest,
        surviving_hypothesis_count: alternatives.alternatives.len() as u64,
        precommitted_prediction_count: predictions.predictions.len() as u64,
        operations: vec![wrong_hidden.effect.operation_v1()],
    })
    .expect("wrong-action request");
    let binding = K2LawLabBindingV1::seal(K2LawLabBindingInputV1 {
        freeze: &freeze,
        goal: context.goal,
        vocabulary: &vocabulary,
        alternatives: &alternatives,
        predictions: &predictions,
        selection: &selection,
        request: &request,
    })
    .expect("wrong-action binding");
    let execution = context
        .adapter
        .execute(&request)
        .expect("wrong-action bwrap execution");
    let oracle_request = K2ExactOracleRequestV1::seal(
        context.goal,
        &freeze,
        &binding,
        &execution,
        context.oracle_manifest,
    )
    .expect("wrong-action oracle request");
    let oracle_outcome = execute_exact_oracle(context.oracle_path, &oracle_request);
    assert!(!oracle_outcome.goal_satisfied);
    let exact_goal = K2ExactGoalReceiptV1::evaluate(K2ExactGoalEvaluationInputV1 {
        freeze: &freeze,
        goal: context.goal,
        vocabulary: &vocabulary,
        alternatives: &alternatives,
        predictions: &predictions,
        selection: &selection,
        binding: &binding,
        request: &request,
        execution: &execution,
        oracle_manifest: context.oracle_manifest,
        oracle_request: &oracle_request,
        oracle_outcome: &oracle_outcome,
    })
    .expect("wrong-action exact receipt");
    assert!(!exact_goal.goal_satisfied);
    let v1_reason = expect_invalid_reason(
        K2DecisionOutcomeReceiptV1::capability_pass(
            &freeze,
            &predictions,
            &binding,
            &execution,
            &exact_goal,
        ),
        "k2_capability_outcome_inputs_invalid",
    );
    let learned_reason = expect_invalid_reason(
        require_exact_goal_for_learned_capability_v1(&exact_goal),
        "k2_exact_goal_unsatisfied",
    );
    let outcome_root_sha256 = canonical_json_sha256(&(
        "nando.k2-wrong-action-ablation-evidence.v1",
        execution.receipt.receipt_root_sha256.as_str(),
        oracle_outcome.outcome_root_sha256.as_str(),
        exact_goal.receipt_root_sha256.as_str(),
        v1_reason.as_str(),
        learned_reason.as_str(),
    ))
    .expect("wrong-action outcome root");
    K2LearnedAblationControlV1::seal(
        K2LearnedAblationKindV1::WrongActionExactOracle,
        request.request_root_sha256,
        K2LearnedAblationVerdictV1::ExactGoalUnsatisfied,
        K2LearnedAblationVerdictV1::ExactGoalUnsatisfied,
        0,
        1,
        1,
        outcome_root_sha256,
    )
    .expect("wrong-action control")
}

fn run_journal_contract_assertions(
    fixture: &LearnedFixtureV1,
    freeze: &K2LearnedCapabilityFreezeV1,
    first_dispatch: &K2SupportDispatchV1,
    first_observation: &K2SupportObservationV1,
    journal: &K2LearnedCapabilityJournalV1,
    outcome: &K2LearnedCapabilityOutcomeV1,
    seal: &K2LearnedCapabilitySealV1,
) {
    let experiment_id = &journal.projection().experiment_id_sha256;
    let contract_root = fixture.root.join("journal-contract-tests");
    fs::create_dir(&contract_root).expect("journal contract root");
    for prefix_length in 0..=journal.events().len() {
        let store = contract_root.join(format!("prefix-{prefix_length:02}"));
        write_journal_prefix(&store, experiment_id, &journal.events()[..prefix_length]);
        let reopened = K2LearnedCapabilityJournalV1::open_existing(&store, experiment_id.clone())
            .expect("journal prefix restart");
        let projected = K2LearnedCapabilityProjectionV1::project(
            experiment_id,
            &journal.events()[..prefix_length],
        )
        .expect("journal prefix projection");
        assert_eq!(reopened.projection(), &projected);
    }
    let binding_prefix =
        K2LearnedCapabilityProjectionV1::project(experiment_id, &journal.events()[..19])
            .expect("binding prefix");
    assert_eq!(
        binding_prefix.state,
        K2LearnedCapabilityStateV1::LearnedToV1BindingFrozen
    );

    let dispatched_store = contract_root.join("dispatched-restart");
    write_journal_prefix(&dispatched_store, experiment_id, &journal.events()[..2]);
    let mut dispatched =
        K2LearnedCapabilityJournalV1::open_existing(&dispatched_store, experiment_id.clone())
            .expect("dispatch restart");
    expect_invalid_reason(
        dispatched.append_support_observation(first_observation, 3),
        "k2_indeterminate_after_support_dispatch",
    );

    let gap_store = contract_root.join("gap");
    let gap_directory = gap_store.join(experiment_id);
    fs::create_dir_all(&gap_directory).expect("gap directory");
    write_file(
        &gap_directory.join("00000000000000000001.json"),
        &journal.events()[1]
            .canonical_bytes_v1()
            .expect("gap event bytes"),
    );
    assert!(
        K2LearnedCapabilityJournalV1::open_existing(&gap_store, experiment_id.clone(),).is_err()
    );

    let duplicate_store = contract_root.join("duplicate");
    let duplicate_directory = duplicate_store.join(experiment_id);
    fs::create_dir_all(&duplicate_directory).expect("duplicate directory");
    let first_bytes = journal.events()[0]
        .canonical_bytes_v1()
        .expect("first event bytes");
    write_file(
        &duplicate_directory.join("00000000000000000000.json"),
        &first_bytes,
    );
    write_file(
        &duplicate_directory.join("00000000000000000001.json"),
        &first_bytes,
    );
    assert!(
        K2LearnedCapabilityJournalV1::open_existing(&duplicate_store, experiment_id.clone(),)
            .is_err()
    );

    let tamper_store = contract_root.join("tamper");
    let tamper_directory = tamper_store.join(experiment_id);
    fs::create_dir_all(&tamper_directory).expect("tamper directory");
    let mut tampered = serde_json::to_value(&journal.events()[0]).expect("tamper value");
    tampered
        .as_object_mut()
        .expect("tamper object")
        .insert("recorded_at_unix_ms".to_owned(), serde_json::json!(99));
    let tampered_bytes =
        nando_operator_kernel::canonical_json_bytes(&tampered).expect("tampered event bytes");
    write_file(
        &tamper_directory.join("00000000000000000000.json"),
        &tampered_bytes,
    );
    assert!(
        K2LearnedCapabilityJournalV1::open_existing(&tamper_store, experiment_id.clone(),).is_err()
    );

    let wrong_order_store = contract_root.join("wrong-order");
    let wrong_order_directory = wrong_order_store.join(experiment_id);
    fs::create_dir_all(&wrong_order_directory).expect("wrong-order directory");
    write_file(
        &wrong_order_directory.join("00000000000000000000.json"),
        &journal.events()[1]
            .canonical_bytes_v1()
            .expect("wrong-order event bytes"),
    );
    write_file(
        &wrong_order_directory.join("00000000000000000001.json"),
        &journal.events()[0]
            .canonical_bytes_v1()
            .expect("wrong-order second bytes"),
    );
    assert!(
        K2LearnedCapabilityJournalV1::open_existing(&wrong_order_store, experiment_id.clone(),)
            .is_err()
    );

    let before_publish_store = contract_root.join("fault-before-publish");
    let mut before_publish =
        K2LearnedCapabilityJournalV1::create(&before_publish_store, experiment_id.clone())
            .expect("before-publish journal");
    before_publish.set_next_fault_for_test_v1(K2LearnedJournalFaultPointV1::AfterTempSync);
    assert!(before_publish.append_freeze(freeze, 1).is_err());
    let reopened_before_publish =
        K2LearnedCapabilityJournalV1::open_existing(&before_publish_store, experiment_id.clone())
            .expect("before-publish restart");
    assert_eq!(reopened_before_publish.events().len(), 0);

    let after_publish_store = contract_root.join("fault-after-publish");
    let mut after_publish =
        K2LearnedCapabilityJournalV1::create(&after_publish_store, experiment_id.clone())
            .expect("after-publish journal");
    after_publish
        .set_next_fault_for_test_v1(K2LearnedJournalFaultPointV1::AfterPublishBeforeDirectorySync);
    assert!(after_publish.append_freeze(freeze, 1).is_err());
    let reopened_after_publish =
        K2LearnedCapabilityJournalV1::open_existing(&after_publish_store, experiment_id.clone())
            .expect("after-publish restart");
    assert_eq!(reopened_after_publish.events().len(), 1);
    assert_eq!(
        reopened_after_publish.projection().state,
        K2LearnedCapabilityStateV1::Frozen
    );

    let support_fault_store = contract_root.join("support-dispatch-fault");
    let mut support_fault =
        K2LearnedCapabilityJournalV1::create(&support_fault_store, experiment_id.clone())
            .expect("support fault journal");
    support_fault
        .append_freeze(freeze, 1)
        .expect("support fault freeze");
    support_fault
        .set_next_fault_for_test_v1(K2LearnedJournalFaultPointV1::AfterPublishBeforeDirectorySync);
    assert!(
        support_fault
            .append_support_dispatch(first_dispatch, 2)
            .is_err()
    );
    let mut reopened_support_fault =
        K2LearnedCapabilityJournalV1::open_existing(&support_fault_store, experiment_id.clone())
            .expect("support fault restart");
    assert!(
        reopened_support_fault
            .projection()
            .indeterminate_after_support_dispatch
    );
    expect_invalid_reason(
        reopened_support_fault.append_support_observation(first_observation, 3),
        "k2_indeterminate_after_support_dispatch",
    );

    assert_ne!(outcome.outcome_root_sha256, seal.seal_root_sha256);
    assert_ne!(
        journal.projection().projection_root_sha256,
        seal.seal_root_sha256
    );
    assert_ne!(
        journal
            .events()
            .last()
            .expect("terminal event")
            .entry_root_sha256,
        seal.seal_root_sha256
    );
    let outcome_bytes =
        nando_operator_kernel::canonical_json_bytes(outcome).expect("terminal outcome bytes");
    assert!(
        !outcome_bytes
            .windows(seal.seal_root_sha256.len())
            .any(|window| window == seal.seal_root_sha256.as_bytes())
    );
}

fn write_journal_prefix(store: &Path, experiment_id: &str, events: &[K2LearnedCapabilityEventV1]) {
    let directory = store.join(experiment_id);
    fs::create_dir_all(&directory).expect("journal prefix directory");
    for event in events {
        write_file(
            &directory.join(format!("{:020}.json", event.sequence)),
            &event.canonical_bytes_v1().expect("journal event bytes"),
        );
    }
}

fn execute_exact_oracle(
    oracle_path: &Path,
    request: &K2ExactOracleRequestV1,
) -> K2ExactOracleOutcomeV1 {
    let mut child = Command::new(oracle_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn exact oracle");
    child
        .stdin
        .take()
        .expect("oracle stdin")
        .write_all(&request.canonical_bytes().expect("oracle request bytes"))
        .expect("write oracle request");
    let output = child.wait_with_output().expect("wait exact oracle");
    assert!(output.status.success(), "oracle process failed");
    assert!(output.stderr.is_empty());
    K2ExactOracleOutcomeV1::from_canonical_bytes(&output.stdout, request)
        .expect("canonical oracle outcome")
}

fn root(label: &str) -> String {
    canonical_json_sha256(&label).expect("root")
}

fn workspace_is_empty(path: &Path) -> bool {
    fs::read_dir(path)
        .expect("workspace directory")
        .next()
        .is_none()
}

struct LearnedFixtureV1 {
    root: PathBuf,
    source_store: PathBuf,
    workspace_store: PathBuf,
    private_store: PathBuf,
    learned_journal_store: PathBuf,
    v1_journal_store: PathBuf,
    support: K2SupportWorldSetV1,
    target_pre: LawLabTreeManifestV1,
    target_expected: LawLabTreeManifestV1,
    target_goal_store_snapshot_root_sha256: String,
}

impl LearnedFixtureV1 {
    fn new() -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let parent = std::env::var_os("NANDO_K2_GOAL_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .expect("current directory")
                    .join("target/k2-goal-environment-learned-tests")
            });
        fs::create_dir_all(&parent).expect("test parent");
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).expect("parent mode");
        let fixture_root = parent.join(format!("{}-{sequence}", std::process::id()));
        fs::create_dir(&fixture_root).expect("fixture root");
        fs::set_permissions(&fixture_root, fs::Permissions::from_mode(0o700)).expect("root mode");
        let source_store = fixture_root.join("sources");
        let workspace_store = fixture_root.join("workspaces");
        let private_store = fixture_root.join("private");
        let learned_journal_store = fixture_root.join("learned-journals");
        let v1_journal_store = fixture_root.join("v1-journals");
        for path in [
            &source_store,
            &workspace_store,
            &private_store,
            &learned_journal_store,
            &v1_journal_store,
        ] {
            fs::create_dir(path).expect("fixture directory");
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).expect("directory mode");
        }
        let mut worlds = Vec::new();
        for ordinal in 0..3_u64 {
            let staging = fixture_root.join(format!("support-{ordinal}"));
            fs::create_dir(&staging).expect("support staging");
            fs::set_permissions(&staging, fs::Permissions::from_mode(0o700))
                .expect("support staging mode");
            write_file(
                &staging.join("input.bin"),
                &vec![b'a' + ordinal as u8; 11 + ordinal as usize * 6],
            );
            write_file(
                &staging.join("obsolete.bin"),
                &vec![b'k' + ordinal as u8; 7 + ordinal as usize * 8],
            );
            match ordinal {
                0 => write_file(&staging.join("distractor-a.txt"), b"a"),
                1 => {
                    fs::create_dir(staging.join("nested")).expect("nested");
                    fs::set_permissions(staging.join("nested"), fs::Permissions::from_mode(0o700))
                        .expect("nested mode");
                    write_file(&staging.join("nested/distractor-b.txt"), b"bb");
                }
                _ => {
                    write_file(&staging.join("distractor-c.txt"), b"ccc");
                    write_file(&staging.join("distractor-d.txt"), b"dddd");
                }
            }
            let manifest = LawLabTreeManifestV1::scan(&staging, K2_LEARNED_MAX_TREE_BYTES_V1)
                .expect("support manifest");
            fs::rename(&staging, source_store.join(&manifest.tree_root_sha256))
                .expect("seal support source");
            worlds.push(
                K2SupportWorldV1::seal(
                    ordinal,
                    manifest,
                    root(&format!("support-provenance-{ordinal}")),
                )
                .expect("support world"),
            );
        }
        let support = K2SupportWorldSetV1::seal(worlds).expect("support world set");

        let target_staging = fixture_root.join("target-staging");
        fs::create_dir(&target_staging).expect("target staging");
        fs::set_permissions(&target_staging, fs::Permissions::from_mode(0o700))
            .expect("target staging mode");
        let target_input = vec![b'z'; 37];
        write_file(&target_staging.join("input.bin"), &target_input);
        write_file(&target_staging.join("obsolete.bin"), &[b'y'; 41]);
        fs::create_dir(target_staging.join("target-nested")).expect("target nested");
        fs::set_permissions(
            target_staging.join("target-nested"),
            fs::Permissions::from_mode(0o700),
        )
        .expect("target nested mode");
        write_file(
            &target_staging.join("target-nested/distractor-e.txt"),
            b"eeeee",
        );
        write_file(&target_staging.join("distractor-f.txt"), b"ffffff");
        let target_pre = LawLabTreeManifestV1::scan(&target_staging, K2_LEARNED_MAX_TREE_BYTES_V1)
            .expect("target pre manifest");
        fs::rename(
            &target_staging,
            source_store.join(&target_pre.tree_root_sha256),
        )
        .expect("seal target source");

        let expected_store = fixture_root.join("target-expected");
        fs::create_dir(&expected_store).expect("expected store");
        write_file(&expected_store.join("input.bin"), &target_input);
        write_file(&expected_store.join("selected.bin"), &target_input);
        write_file(&expected_store.join("obsolete.bin"), &[b'y'; 41]);
        fs::create_dir(expected_store.join("target-nested")).expect("expected nested");
        write_file(
            &expected_store.join("target-nested/distractor-e.txt"),
            b"eeeee",
        );
        write_file(&expected_store.join("distractor-f.txt"), b"ffffff");
        let target_expected =
            LawLabTreeManifestV1::scan(&expected_store, K2_LEARNED_MAX_TREE_BYTES_V1)
                .expect("target expected manifest");
        let target_goal_store_snapshot_root_sha256 =
            canonical_json_sha256(&target_expected).expect("goal store snapshot");
        Self {
            root: fixture_root,
            source_store,
            workspace_store,
            private_store,
            learned_journal_store,
            v1_journal_store,
            support,
            target_pre,
            target_expected,
            target_goal_store_snapshot_root_sha256,
        }
    }
}

impl Drop for LearnedFixtureV1 {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_file(path: &Path, bytes: &[u8]) {
    let mut file = File::create(path).expect("fixture file");
    file.write_all(bytes).expect("fixture write");
    file.sync_all().expect("fixture sync");
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("fixture mode");
}

#[test]
fn frozen_roots_are_sha256_and_authority_is_false() {
    let authority = K2AuthorityBoundaryV1::authority_free_v1();
    authority.validate().expect("authority-free boundary");
    assert!(valid_nonzero_sha256(&root("learned-unit-root")));
}
