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

