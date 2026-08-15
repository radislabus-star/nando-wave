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

